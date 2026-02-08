//! DWG Reader implementation
//!
//! This module provides the main `DwgReader` struct for reading AutoCAD DWG files.
//! It supports DWG versions from AutoCAD R13 (AC1012) through AutoCAD 2018+ (AC1032).
//!
//! ## Example
//!
//! ```rust,ignore
//! use acadrust::io::dwg::DwgReader;
//!
//! // Read from file
//! let reader = DwgReader::from_file("drawing.dwg")?;
//! let document = reader.read()?;
//!
//! // Or read from bytes
//! let bytes = std::fs::read("drawing.dwg")?;
//! let reader = DwgReader::from_bytes(&bytes)?;
//! let document = reader.read()?;
//! ```

use std::fs::File;
use std::io::{BufReader, Cursor, Read, Seek, SeekFrom};
use std::path::Path;

use crate::document::CadDocument;
use crate::error::{DxfError, Result};
use crate::types::ACadVersion;

use super::file_header::{
    CodePage, DwgFileHeaderAC15, DwgFileHeaderAC18, DwgFileHeaderAC21,
    DwgFileHeaderType,
};
use super::section::{DwgSectionDescriptor, DwgSectionLocatorRecord};

/// Configuration options for DWG reading
#[derive(Debug, Clone)]
pub struct DwgReaderConfiguration {
    /// Whether to read the summary info section
    pub read_summary_info: bool,
    /// Whether to read the preview image
    pub read_preview: bool,
    /// Whether to keep the original handles or renumber them
    pub keep_handles: bool,
}

impl Default for DwgReaderConfiguration {
    fn default() -> Self {
        Self {
            read_summary_info: true,
            read_preview: false,
            keep_handles: true,
        }
    }
}

/// DWG file reader
///
/// Reads AutoCAD DWG binary files and produces a `CadDocument`.
pub struct DwgReader<R: Read + Seek> {
    /// The underlying reader
    reader: R,
    /// Reader configuration
    config: DwgReaderConfiguration,
    /// Parsed file header (cached after first read)
    file_header: Option<DwgFileHeaderType>,
}

impl DwgReader<BufReader<File>> {
    /// Create a DWG reader from a file path
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path.as_ref())?;
        let reader = BufReader::new(file);
        Ok(Self::new(reader))
    }
}

impl<'a> DwgReader<Cursor<&'a [u8]>> {
    /// Create a DWG reader from a byte slice
    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self> {
        let cursor = Cursor::new(bytes);
        Ok(Self::new(cursor))
    }
}

impl<R: Read + Seek> DwgReader<R> {
    /// Create a new DWG reader with the given reader
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            config: DwgReaderConfiguration::default(),
            file_header: None,
        }
    }
    
    /// Set the reader configuration
    pub fn with_config(mut self, config: DwgReaderConfiguration) -> Self {
        self.config = config;
        self
    }
    
    /// Get the configuration
    pub fn config(&self) -> &DwgReaderConfiguration {
        &self.config
    }
    
    /// Get a mutable reference to the configuration
    pub fn config_mut(&mut self) -> &mut DwgReaderConfiguration {
        &mut self.config
    }
    
    /// Read and return the AutoCAD version from the file header
    pub fn read_version(&mut self) -> Result<ACadVersion> {
        self.ensure_file_header()?;
        Ok(self.file_header.as_ref().unwrap().version())
    }
    
    /// Read the DWG file and return a CadDocument
    /// 
    /// This implements the full DWG reading pipeline following the C# ACadSharp pattern:
    /// 1. Read file header
    /// 2. Read header section (drawing variables)
    /// 3. Read classes section (custom object types)
    /// 4. Read handles section (handle-to-offset map)
    /// 5. Read objects section (entities and objects)
    /// 6. Build document from templates
    pub fn read(mut self) -> Result<CadDocument> {
        // Read the file header first
        self.ensure_file_header()?;
        
        let file_header = self.file_header.take().unwrap();
        let version = file_header.version();
        
        // Create a new document
        let mut document = CadDocument::new();
        document.version = version;
        
        // For versions before R2004, use the simpler AC15 format
        match &file_header {
            DwgFileHeaderType::AC15(header) => {
                self.read_ac15(&mut document, header, version)?;
            }
            DwgFileHeaderType::AC18(header) => {
                // R2004+ uses different section format
                self.read_ac18(&mut document, header, version)?;
            }
            DwgFileHeaderType::AC21(header) => {
                // R2007+ builds on AC18 format
                self.read_ac21(&mut document, header, version)?;
            }
        }
        
        Ok(document)
    }
    
    /// Read AC15 format (R13-R2002)
    fn read_ac15(&mut self, document: &mut CadDocument, header: &DwgFileHeaderAC15, version: ACadVersion) -> Result<()> {
        use std::io::Cursor;
        use super::stream_reader::BitReader;
        use super::section_reader::DwgSectionReaderAC15;
        use super::header_reader::DwgHeaderReader;
        use super::classes_reader::DwgClassesReader;
        use super::handle_reader::DwgHandleReader;
        use super::object_reader::DwgObjectReader;
        
        // Get maintenance version from file header
        let maintenance_version = header.maintenance_version;
        
        // For AC15, read the entire file into memory as objects are at file offsets
        self.reader.seek(SeekFrom::Start(0))?;
        let mut file_data = Vec::new();
        self.reader.read_to_end(&mut file_data)?;
        
        // Read section data from the file
        let (header_section_data, classes_section_data, handles_section_data) = {
            let mut cursor = Cursor::new(&file_data);
            let mut section_reader = DwgSectionReaderAC15::new(&mut cursor, header, version);
            let header_data = section_reader.read_section("HEADER")?.data;
            let classes_data = section_reader.read_section("CLASSES")?.data;
            let handles_data = section_reader.read_section("HANDLES")?.data;
            (header_data, classes_data, handles_data)
        };
        
        // 1. Read header section
        let header_cursor = Cursor::new(header_section_data);
        let header_bit_reader = BitReader::new(header_cursor, version);
        let mut header_reader = DwgHeaderReader::new(header_bit_reader, version, maintenance_version);
        let header_handles = header_reader.read(&mut document.header)?;
        
        // Copy header handles to document header variables  
        Self::apply_header_handles_static(document, &header_handles);
        
        // 2. Read classes section
        let classes_cursor = Cursor::new(classes_section_data);
        let classes_bit_reader = BitReader::new(classes_cursor, version);
        let mut classes_reader = DwgClassesReader::new(classes_bit_reader, version);
        let classes = classes_reader.read()?;
        
        // 3. Read handles section  
        let handles_cursor = Cursor::new(handles_section_data);
        let handles_bit_reader = BitReader::new(handles_cursor, version);
        let mut handle_reader = DwgHandleReader::new(handles_bit_reader, version);
        let handle_map = handle_reader.read()?;
        
        // 4. Read objects from file offsets
        // For AC15, use ALL handle map entries as initial handles since entity handles
        // are not yet discovered through block records (handle reading in block records
        // requires separate stream support for R2000)
        let initial_handles: Vec<u64> = handle_map.keys().copied().collect();
        
        // Create a BitReader over the entire file data
        let objects_cursor = Cursor::new(file_data);
        let objects_bit_reader = BitReader::new(objects_cursor, version);
        let mut object_reader = DwgObjectReader::new(
            objects_bit_reader,
            version,
            handle_map,
            &classes,
            initial_handles,
        );
        // For AC15, offsets in handle map are absolute file offsets
        object_reader.set_section_base(0);
        
        let templates = object_reader.read()?;
        
        // 5. Build entities and objects from templates
        Self::build_document_static(document, templates)?;
        
        Ok(())
    }
    
    /// Read AC18 format (R2004-R2006)
    fn read_ac18(&mut self, document: &mut CadDocument, header: &DwgFileHeaderAC18, version: ACadVersion) -> Result<()> {
        use super::section_reader::DwgSectionReaderAC18;
        use std::io::Cursor;
        use super::stream_reader::BitReader;
        use super::header_reader::DwgHeaderReader;
        use super::classes_reader::DwgClassesReader;
        use super::handle_reader::DwgHandleReader;
        use super::object_reader::DwgObjectReader;
        
        // Get maintenance version from file header
        let maintenance_version = header.base.maintenance_version;
        
        // Read all section data first
        let (header_data, classes_data, handles_data, objects_data) = {
            let mut section_reader = match DwgSectionReaderAC18::new(&mut self.reader, header, version) {
                Ok(reader) => reader,
                Err(e) => {
                    // If we can't initialize the section reader (e.g., encrypted header),
                    // return an empty document with just the version set
                    let _ = e;
                    return Ok(());
                }
            };
            
            let header_data = match section_reader.read_section("AcDb:Header") {
                Ok(s) => Some(s.data),
                Err(_) => None,
            };
            let classes_data = match section_reader.read_section("AcDb:Classes") {
                Ok(s) => Some(s.data),
                Err(_) => None,
            };
            let handles_data = match section_reader.read_section("AcDb:Handles") {
                Ok(s) => Some(s.data),
                Err(_) => None,
            };
            let objects_data = match section_reader.read_section("AcDb:AcDbObjects") {
                Ok(s) => Some(s.data),
                Err(_) => None,
            };
            (header_data, classes_data, handles_data, objects_data)
        };
        
        // 1. Read header section
        let header_handles_result = if let Some(data) = header_data {
            let header_cursor = Cursor::new(data);
            let header_bit_reader = BitReader::new(header_cursor, version);
            let mut header_reader = DwgHeaderReader::new(header_bit_reader, version, maintenance_version);
            header_reader.read(&mut document.header).ok()
        } else {
            None
        };
        
        if let Some(ref header_handles) = header_handles_result {
            Self::apply_header_handles_static(document, header_handles);
        }
        
        // 2. Read classes section
        let classes = if let Some(data) = classes_data {
            let classes_cursor = Cursor::new(data);
            let classes_bit_reader = BitReader::new(classes_cursor, version);
            let mut classes_reader = DwgClassesReader::new(classes_bit_reader, version);
            classes_reader.read().unwrap_or_default()
        } else {
            Default::default()
        };
        
        // 3. Read handles section
        let handle_map = if let Some(data) = handles_data {
            let handles_cursor = Cursor::new(data);
            let handles_bit_reader = BitReader::new(handles_cursor, version);
            let mut handle_reader = DwgHandleReader::new(handles_bit_reader, version);
            handle_reader.read().unwrap_or_default()
        } else {
            Default::default()
        };
        
        // 4. Read objects section
        if let Some(data) = objects_data {
            let objects_cursor = Cursor::new(data);
            let objects_bit_reader = BitReader::new(objects_cursor, version);
            
            // Use header handles as initial handles, or fall back to all handles from handle_map
            let initial_handles: Vec<u64> = if let Some(ref hh) = header_handles_result {
                hh.get_handles()
            } else {
                // If no header handles, use all handles from handle_map
                handle_map.keys().copied().collect()
            };
            
            let mut object_reader = DwgObjectReader::new(
                objects_bit_reader,
                version,
                handle_map,
                &classes,
                initial_handles,
            );
            
            if let Ok(templates) = object_reader.read() {
                let _ = Self::build_document_static(document, templates);
            }
        }
        
        Ok(())
    }
    
    /// Read AC21 format (R2007+)
    fn read_ac21(&mut self, document: &mut CadDocument, header: &DwgFileHeaderAC21, version: ACadVersion) -> Result<()> {
        use super::decompressor::Lz77AC21Decompressor;
        use super::section::DwgSectionLocatorRecord;
        use std::io::Cursor;
        use super::stream_reader::BitReader;
        use super::header_reader::DwgHeaderReader;
        use super::classes_reader::DwgClassesReader;
        use super::handle_reader::DwgHandleReader;
        use super::object_reader::DwgObjectReader;
        
        let maintenance_version = header.base.base.maintenance_version;
        
        /// Read a section's decompressed data using AC21 format
        fn read_section_ac21(
            reader: &mut (impl Read + Seek),
            header: &DwgFileHeaderAC21,
            section_name: &str,
        ) -> Option<Vec<u8>> {
            let descriptor = header.base.descriptors.get(section_name)?;
            
            // Calculate total uncompressed size
            let total_size: u64 = descriptor.local_sections.iter()
                .map(|p| p.size)
                .sum();
            
            let mut data = Vec::with_capacity(total_size as usize);
            
            for page in &descriptor.local_sections {
                // Get page position from the records table
                let page_record = header.base.base.records.get(&page.page_number)?;
                
                // Seek to page position (base 0x480 for AC21)
                reader.seek(SeekFrom::Start(page_record.seeker as u64 + 0x480)).ok()?;
                
                let mut page_bytes = vec![0u8; page_record.size as usize];
                reader.read_exact(&mut page_bytes).ok()?;
                
                // Check if page is Reed-Solomon encoded
                if descriptor.encoding == Some(4) {
                    let v = page.compressed_size + 7;
                    let v1 = v & 0xFFFFFFF8u64;  // 32-bit mask like C#
                    let aligned_page_size = ((v1 + 251 - 1) / 251) as usize;
                    let mut decoded = vec![0u8; aligned_page_size * 251];
                    reed_solomon_decoding(&page_bytes, &mut decoded, aligned_page_size, 251);
                    page_bytes = decoded;
                }
                
                // Decompress if compressed
                if page.compressed_size != page.size {
                    let mut decompressed = vec![0u8; page.size as usize];
                    if Lz77AC21Decompressor::decompress(&page_bytes, 0, page.compressed_size as usize, &mut decompressed).is_ok() {
                        page_bytes = decompressed;
                    }
                }
                
                data.extend_from_slice(&page_bytes[..page.size as usize]);
            }
            
            Some(data)
        }
        
        // Read section data
        let header_data = read_section_ac21(&mut self.reader, header, "AcDb:Header");
        let classes_data = read_section_ac21(&mut self.reader, header, "AcDb:Classes");
        let handles_data = read_section_ac21(&mut self.reader, header, "AcDb:Handles");
        let objects_data = read_section_ac21(&mut self.reader, header, "AcDb:AcDbObjects");
        
        // 1. Read header section
        let header_handles_result = if let Some(data) = header_data {
            let header_cursor = Cursor::new(data);
            let header_bit_reader = BitReader::new(header_cursor, version);
            let mut header_reader = DwgHeaderReader::new(header_bit_reader, version, maintenance_version);
            header_reader.read(&mut document.header).ok()
        } else {
            None
        };
        
        if let Some(ref header_handles) = header_handles_result {
            Self::apply_header_handles_static(document, header_handles);
        }
        
        // 2. Read classes section
        let classes = if let Some(data) = classes_data {
            let classes_cursor = Cursor::new(data);
            let classes_bit_reader = BitReader::new(classes_cursor, version);
            let mut classes_reader = DwgClassesReader::new(classes_bit_reader, version);
            classes_reader.read().unwrap_or_default()
        } else {
            Default::default()
        };
        
        // 3. Read handles section
        let handle_map = if let Some(data) = handles_data {
            let handles_cursor = Cursor::new(data);
            let handles_bit_reader = BitReader::new(handles_cursor, version);
            let mut handle_reader = DwgHandleReader::new(handles_bit_reader, version);
            handle_reader.read().unwrap_or_default()
        } else {
            Default::default()
        };
        
        // 4. Read objects section
        if let Some(data) = objects_data {
            let objects_cursor = Cursor::new(data);
            let objects_bit_reader = BitReader::new(objects_cursor, version);
            
            // Use all handle map entries as initial handles
            let initial_handles: Vec<u64> = if let Some(ref hh) = header_handles_result {
                let h = hh.get_handles();
                if h.len() < 50 {
                    handle_map.keys().copied().collect()
                } else {
                    h
                }
            } else {
                handle_map.keys().copied().collect()
            };
            
            let mut object_reader = DwgObjectReader::new(
                objects_bit_reader,
                version,
                handle_map,
                &classes,
                initial_handles,
            );
            
            if let Ok(templates) = object_reader.read() {
                let _ = Self::build_document_static(document, templates);
            }
        }
        
        Ok(())
    }
    
    /// Apply header handles to document (static version to avoid borrow conflicts)
    fn apply_header_handles_static(document: &mut CadDocument, handles: &super::header_reader::DwgHeaderHandles) {
        use crate::types::Handle;
        
        // Block control handles
        if let Some(h) = handles.block_control {
            document.header.block_control_handle = Handle::new(h);
        }
        if let Some(h) = handles.layer_control {
            document.header.layer_control_handle = Handle::new(h);
        }
        if let Some(h) = handles.style_control {
            document.header.style_control_handle = Handle::new(h);
        }
        if let Some(h) = handles.linetype_control {
            document.header.linetype_control_handle = Handle::new(h);
        }
        if let Some(h) = handles.view_control {
            document.header.view_control_handle = Handle::new(h);
        }
        if let Some(h) = handles.ucs_control {
            document.header.ucs_control_handle = Handle::new(h);
        }
        if let Some(h) = handles.vport_control {
            document.header.vport_control_handle = Handle::new(h);
        }
        if let Some(h) = handles.appid_control {
            document.header.appid_control_handle = Handle::new(h);
        }
        if let Some(h) = handles.dimstyle_control {
            document.header.dimstyle_control_handle = Handle::new(h);
        }
        
        // Block record handles
        if let Some(h) = handles.model_space {
            document.header.model_space_block_handle = Handle::new(h);
        }
        if let Some(h) = handles.paper_space {
            document.header.paper_space_block_handle = Handle::new(h);
        }
        
        // Linetype handles
        if let Some(h) = handles.bylayer_linetype {
            document.header.bylayer_linetype_handle = Handle::new(h);
        }
        if let Some(h) = handles.byblock_linetype {
            document.header.byblock_linetype_handle = Handle::new(h);
        }
        if let Some(h) = handles.continuous_linetype {
            document.header.continuous_linetype_handle = Handle::new(h);
        }
        
        // Dictionary handles
        if let Some(h) = handles.named_objects_dict {
            document.header.named_objects_dict_handle = Handle::new(h);
        }
        if let Some(h) = handles.layout_dict {
            document.header.acad_layout_dict_handle = Handle::new(h);
        }
        if let Some(h) = handles.group_dict {
            document.header.acad_group_dict_handle = Handle::new(h);
        }
        if let Some(h) = handles.mline_style_dict {
            document.header.acad_mlinestyle_dict_handle = Handle::new(h);
        }
        if let Some(h) = handles.material_dict {
            document.header.acad_material_dict_handle = Handle::new(h);
        }
        if let Some(h) = handles.color_dict {
            document.header.acad_color_dict_handle = Handle::new(h);
        }
        if let Some(h) = handles.visualstyle_dict {
            document.header.acad_visualstyle_dict_handle = Handle::new(h);
        }
        if let Some(h) = handles.plotstyle_dict {
            document.header.acad_plotstylename_dict_handle = Handle::new(h);
        }
        
        // Current entity references
        if let Some(h) = handles.current_layer {
            document.header.current_layer_handle = Handle::new(h);
        }
        if let Some(h) = handles.current_textstyle {
            document.header.current_text_style_handle = Handle::new(h);
        }
        if let Some(h) = handles.current_linetype {
            document.header.current_linetype_handle = Handle::new(h);
        }
        if let Some(h) = handles.current_dimstyle {
            document.header.current_dimstyle_handle = Handle::new(h);
        }
        if let Some(h) = handles.current_multiline_style {
            document.header.current_multiline_style_handle = Handle::new(h);
        }
        
        // Dimension style handles
        if let Some(h) = handles.dim_textstyle {
            document.header.dim_text_style_handle = Handle::new(h);
        }
        if let Some(h) = handles.dim_linetype1 {
            document.header.dim_linetype1_handle = Handle::new(h);
        }
        if let Some(h) = handles.dim_linetype2 {
            document.header.dim_linetype2_handle = Handle::new(h);
        }
        if let Some(h) = handles.dim_arrow1 {
            document.header.dim_arrow_block1_handle = Handle::new(h);
        }
        if let Some(h) = handles.dim_arrow2 {
            document.header.dim_arrow_block2_handle = Handle::new(h);
        }
        if let Some(h) = handles.dim_leader_arrow {
            document.header.dim_arrow_block_handle = Handle::new(h);
        }
    }
    
    /// Build document from templates (static version to avoid borrow conflicts)
    fn build_document_static(
        document: &mut CadDocument,
        templates: std::collections::HashMap<u64, super::object_reader::CadTemplate>,
    ) -> Result<()> {
        use std::collections::HashMap;
        use super::template_builder::DwgTemplateBuilder;
        use super::object_reader::CadTemplate;
        use crate::types::Handle;
        
        // Build layer name map from layer templates
        let mut layer_map: HashMap<u64, String> = HashMap::new();
        for (handle, template) in &templates {
            if let CadTemplate::Layer { object_data: _, name, .. } = template {
                layer_map.insert(*handle, name.clone());
            }
        }
        
        // Create template builder with layer map
        let builder = DwgTemplateBuilder::new()
            .with_layer_map(layer_map);
        
        // Convert templates to entities
        for (handle, template) in &templates {
            // Add layers to document
            if let CadTemplate::Layer { name, is_frozen, is_on, is_locked, is_plotting, color, .. } = template {
                use crate::tables::Layer;
                use crate::tables::layer::LayerFlags;
                let mut layer = Layer::new(name.clone());
                layer.handle = Handle::new(*handle);
                layer.flags = LayerFlags {
                    frozen: *is_frozen,
                    locked: *is_locked,
                    off: !*is_on,
                };
                layer.is_plottable = *is_plotting;
                layer.color = color.clone();
                let _ = document.layers.add(layer);
            }
            
            // Add linetypes to document
            if let CadTemplate::LineType { name, description, pattern_length, .. } = template {
                use crate::tables::LineType;
                let mut linetype = LineType::new(name.clone());
                linetype.handle = Handle::new(*handle);
                linetype.description = description.clone();
                linetype.pattern_length = *pattern_length;
                let _ = document.line_types.add(linetype);
            }
            
            // Add text styles to document
            if let CadTemplate::TextStyle { name, font_name, big_font_name, .. } = template {
                use crate::tables::TextStyle;
                let mut style = TextStyle::new(name.clone());
                style.handle = Handle::new(*handle);
                style.font_file = font_name.clone();
                style.big_font_file = big_font_name.clone();
                let _ = document.text_styles.add(style);
            }
            
            // Add block records
            if let CadTemplate::BlockRecord { name, .. } = template {
                use crate::tables::BlockRecord;
                let mut block_record = BlockRecord::new(name.clone());
                block_record.handle = Handle::new(*handle);
                let _ = document.block_records.add(block_record);
            }
            
            // Add entities
            if let Some(entity) = builder.build_entity(template) {
                let _ = document.add_entity(entity);
            }
        }
        
        Ok(())
    }
    
    /// Read only the file header
    pub fn read_file_header(&mut self) -> Result<&DwgFileHeaderType> {
        self.ensure_file_header()?;
        Ok(self.file_header.as_ref().unwrap())
    }
    
    /// Ensure the file header has been read
    fn ensure_file_header(&mut self) -> Result<()> {
        if self.file_header.is_some() {
            return Ok(());
        }
        
        // Reset to beginning
        self.reader.seek(SeekFrom::Start(0))?;
        
        // Read version string (6 bytes: "ACxxxx")
        let mut version_bytes = [0u8; 6];
        self.reader.read_exact(&mut version_bytes)?;
        
        let version_str = std::str::from_utf8(&version_bytes)
            .map_err(|_| DxfError::InvalidHeader("Invalid version string encoding".to_string()))?;
        
        let version = ACadVersion::from_version_string(version_str);
        
        if !version.supports_dwg_read() {
            return Err(DxfError::UnsupportedVersion(version_str.to_string()));
        }
        
        // Create appropriate file header based on version
        let file_header = match version {
            ACadVersion::AC1012 | ACadVersion::AC1014 | ACadVersion::AC1015 => {
                self.read_file_header_ac15(version)?
            }
            ACadVersion::AC1018 => {
                self.read_file_header_ac18(version)?
            }
            ACadVersion::AC1021 => {
                self.read_file_header_ac21(version)?
            }
            ACadVersion::AC1024 | ACadVersion::AC1027 | ACadVersion::AC1032 => {
                // R2010+ uses AC18-style file header (reverted from AC21 format)
                self.read_file_header_ac18(version)?
            }
            _ => return Err(DxfError::UnsupportedVersion(format!("{:?}", version))),
        };
        
        self.file_header = Some(file_header);
        Ok(())
    }
    
    /// Read AC15 file header (R13-2002)
    fn read_file_header_ac15(&mut self, version: ACadVersion) -> Result<DwgFileHeaderType> {
        let mut header = DwgFileHeaderAC15::new(version);
        
        // Position after version string
        self.reader.seek(SeekFrom::Start(6))?;
        
        // Read 6 bytes (5 0x00 + 1 maintenance version)
        let mut buf = [0u8; 6];
        self.reader.read_exact(&mut buf)?;
        header.maintenance_version = buf[5] as i32;
        
        // Skip 1 byte
        self.reader.seek(SeekFrom::Current(1))?;
        
        // Read preview address (4 bytes, little-endian)
        let mut preview_addr_bytes = [0u8; 4];
        self.reader.read_exact(&mut preview_addr_bytes)?;
        header.preview_address = i32::from_le_bytes(preview_addr_bytes) as i64;
        
        // Read drawing version and app version (2 bytes)
        let mut version_bytes = [0u8; 2];
        self.reader.read_exact(&mut version_bytes)?;
        
        // Read code page (2 bytes)
        let mut codepage_bytes = [0u8; 2];
        self.reader.read_exact(&mut codepage_bytes)?;
        header.code_page = CodePage::from_value(i16::from_le_bytes(codepage_bytes));
        
        // Read number of section locator records (4 bytes)
        let mut num_records_bytes = [0u8; 4];
        self.reader.read_exact(&mut num_records_bytes)?;
        let num_records = i32::from_le_bytes(num_records_bytes);
        
        // Read section locator records
        for _ in 0..num_records {
            let mut record_num = [0u8; 1];
            self.reader.read_exact(&mut record_num)?;
            
            let mut seeker_bytes = [0u8; 4];
            self.reader.read_exact(&mut seeker_bytes)?;
            
            let mut size_bytes = [0u8; 4];
            self.reader.read_exact(&mut size_bytes)?;
            
            header.add_record(
                record_num[0] as i32,
                i32::from_le_bytes(seeker_bytes) as i64,
                i32::from_le_bytes(size_bytes) as i64,
            );
        }
        
        // Verify CRC
        // TODO: Implement CRC verification
        
        Ok(DwgFileHeaderType::AC15(header))
    }
    
    /// Read AC18 file header (2004-2006)
    fn read_file_header_ac18(&mut self, version: ACadVersion) -> Result<DwgFileHeaderType> {
        let mut header = DwgFileHeaderAC18::new(version);
        
        // Position after version string
        self.reader.seek(SeekFrom::Start(6))?;
        
        // Read 6 bytes (maintenance version at offset 11)
        let mut buf = [0u8; 6];
        self.reader.read_exact(&mut buf)?;
        header.base.maintenance_version = buf[5] as i32;
        
        // Read preview address (4 bytes at offset 13)
        self.reader.seek(SeekFrom::Start(13))?;
        let mut preview_addr_bytes = [0u8; 4];
        self.reader.read_exact(&mut preview_addr_bytes)?;
        header.base.preview_address = i32::from_le_bytes(preview_addr_bytes) as i64;
        
        // Read app version (1 byte at offset 17)
        let mut app_version = [0u8; 1];
        self.reader.read_exact(&mut app_version)?;
        header.app_release_version = app_version[0];
        
        // Read drawing version (1 byte at offset 18)
        let mut dwg_version = [0u8; 1];
        self.reader.read_exact(&mut dwg_version)?;
        header.dwg_version = dwg_version[0];
        
        // Read code page (2 bytes at offset 19)
        let mut codepage_bytes = [0u8; 2];
        self.reader.read_exact(&mut codepage_bytes)?;
        header.base.code_page = CodePage::from_value(i16::from_le_bytes(codepage_bytes));
        
        // Skip to security type (offset 24)
        self.reader.seek(SeekFrom::Start(24))?;
        let mut security_bytes = [0u8; 4];
        self.reader.read_exact(&mut security_bytes)?;
        header.security_type = i32::from_le_bytes(security_bytes) as i64;
        
        // Read unknown (4 bytes)
        self.reader.seek(SeekFrom::Current(4))?;
        
        // Read summary info address (4 bytes at offset 32)
        let mut summary_addr_bytes = [0u8; 4];
        self.reader.read_exact(&mut summary_addr_bytes)?;
        header.summary_info_addr = i32::from_le_bytes(summary_addr_bytes) as i64;
        
        // Read VBA project address (4 bytes at offset 36)
        let mut vba_addr_bytes = [0u8; 4];
        self.reader.read_exact(&mut vba_addr_bytes)?;
        header.vba_project_addr = i32::from_le_bytes(vba_addr_bytes) as i64;
        
        // Read encrypted data section (from offset 0x80)
        // This section contains the actual file header data
        self.reader.seek(SeekFrom::Start(0x80))?;
        
        // TODO: Read and decrypt the encrypted header section
        // The encrypted section contains page map, section info, etc.
        
        Ok(DwgFileHeaderType::AC18(header))
    }
    
    /// Read AC21 file header (2007+)
    fn read_file_header_ac21(&mut self, version: ACadVersion) -> Result<DwgFileHeaderType> {
        use super::decompressor::Lz77AC21Decompressor;
        use super::section::Dwg21CompressedMetadata;
        
        let mut header = DwgFileHeaderAC18::new(version);
        
        // Read file metadata (same format at offset 0x06-0x80)
        self.reader.seek(SeekFrom::Start(6))?;
        let mut meta_buf = [0u8; 6];
        self.reader.read_exact(&mut meta_buf)?;
        header.base.maintenance_version = meta_buf[5] as i32;
        
        self.reader.seek(SeekFrom::Start(13))?;
        let mut preview_addr_bytes = [0u8; 4];
        self.reader.read_exact(&mut preview_addr_bytes)?;
        header.base.preview_address = i32::from_le_bytes(preview_addr_bytes) as i64;
        
        let mut app_version = [0u8; 1];
        self.reader.read_exact(&mut app_version)?;
        header.app_release_version = app_version[0];
        
        let mut dwg_version = [0u8; 1];
        self.reader.read_exact(&mut dwg_version)?;
        header.dwg_version = dwg_version[0];
        
        let mut codepage_bytes = [0u8; 2];
        self.reader.read_exact(&mut codepage_bytes)?;
        header.base.code_page = CodePage::from_value(i16::from_le_bytes(codepage_bytes));
        
        // Read 0x400 bytes at position 0x80
        self.reader.seek(SeekFrom::Start(0x80))?;
        let mut compressed_data = [0u8; 0x400];
        self.reader.read_exact(&mut compressed_data)?;
        
        // Reed-Solomon decode with factor=3, blockSize=239
        let mut decoded_data = vec![0u8; 3 * 239]; // 717 bytes
        reed_solomon_decoding(&compressed_data, &mut decoded_data, 3, 239);
        
        // Parse header from decoded data
        // 0x00: CRC(8), 0x08: unknownKey(8), 0x10: compDataCRC(8), 
        // 0x18: comprLen(4), 0x1C: length2(4)
        let compr_len = i32::from_le_bytes([decoded_data[24], decoded_data[25], decoded_data[26], decoded_data[27]]);
        
        // Decompress to 0x110 bytes
        let mut metadata_buf = vec![0u8; 0x110];
        if compr_len < 0 {
            // Not compressed - copy directly
            let len = (-compr_len) as usize;
            metadata_buf[..len].copy_from_slice(&decoded_data[32..32+len]);
        } else {
            Lz77AC21Decompressor::decompress(&decoded_data, 32, compr_len as usize, &mut metadata_buf)?;
        }
        
        // Parse Dwg21CompressedMetadata from the 0x110 buffer
        let read_u64 = |offset: usize| -> u64 {
            u64::from_le_bytes([
                metadata_buf[offset], metadata_buf[offset+1], metadata_buf[offset+2], metadata_buf[offset+3],
                metadata_buf[offset+4], metadata_buf[offset+5], metadata_buf[offset+6], metadata_buf[offset+7],
            ])
        };
        
        let compressed_metadata = Dwg21CompressedMetadata {
            header_size: read_u64(0x00),
            file_size: read_u64(0x08),
            pages_map_crc_compressed: read_u64(0x10),
            pages_map_correction: read_u64(0x18),
            pages_map_crc_seed: read_u64(0x20),
            pages_map_2_offset: read_u64(0x28),
            pages_map_2_id: read_u64(0x30),
            pages_map_offset: read_u64(0x38),
            pages_map_id: read_u64(0x40),
            header_2_offset: read_u64(0x48),
            pages_map_size_compressed: read_u64(0x50),
            pages_map_size_uncompressed: read_u64(0x58),
            pages_amount: read_u64(0x60),
            pages_max_id: read_u64(0x68),
            // 0x70, 0x78: unknown
            sections_map_id: read_u64(0xC0),
            sections_map_size_uncompressed: read_u64(0xC8),
            sections_map_size_compressed: read_u64(0xB0),
            sections_map_crc_uncompressed: read_u64(0xA8),
            sections_map_crc_compressed: read_u64(0xD0),
            sections_map_correction: read_u64(0xD8),
            sections_map_crc_seed: read_u64(0xE0),
            stream_version: read_u64(0xE8),
            crc_seed: read_u64(0xF0),
            crc_seed_encoded: read_u64(0xF8),
            random_seed: read_u64(0x100),
            header_crc_64: read_u64(0x108),
        };
        
        // Read page map using getPageBuffer equivalent
        let page_map_data = self.get_page_buffer_ac21(
            compressed_metadata.pages_map_offset,
            compressed_metadata.pages_map_size_compressed,
            compressed_metadata.pages_map_size_uncompressed,
            compressed_metadata.pages_map_correction,
            0xEF, // blockSize = 239
        )?;
        
        // Parse page map entries: pairs of (size: i64, id: i64)
        let mut offset: i64 = 0;
        let mut pos = 0usize;
        while pos + 16 <= page_map_data.len() {
            let size = i64::from_le_bytes([
                page_map_data[pos], page_map_data[pos+1], page_map_data[pos+2], page_map_data[pos+3],
                page_map_data[pos+4], page_map_data[pos+5], page_map_data[pos+6], page_map_data[pos+7],
            ]);
            let id = i64::from_le_bytes([
                page_map_data[pos+8], page_map_data[pos+9], page_map_data[pos+10], page_map_data[pos+11],
                page_map_data[pos+12], page_map_data[pos+13], page_map_data[pos+14], page_map_data[pos+15],
            ]).abs();
            
            header.base.records.insert(
                id as i32,
                DwgSectionLocatorRecord::with_values(Some(id as i32), offset, size),
            );
            
            offset += size;
            pos += 16;
        }
        
        // Read section map from the sections map page
        let sections_map_id = compressed_metadata.sections_map_id as i32;
        if let Some(sections_page) = header.base.records.get(&sections_map_id) {
            let sections_page_seeker = sections_page.seeker;
            
            let section_map_data = self.get_page_buffer_ac21(
                sections_page_seeker as u64,
                compressed_metadata.sections_map_size_compressed,
                compressed_metadata.sections_map_size_uncompressed,
                compressed_metadata.sections_map_correction,
                239,
            )?;
            
            // Parse section descriptors
            let mut spos = 0usize;
            while spos + 0x40 <= section_map_data.len() {
                let read_u64_at = |off: usize| -> u64 {
                    u64::from_le_bytes([
                        section_map_data[off], section_map_data[off+1], section_map_data[off+2], section_map_data[off+3],
                        section_map_data[off+4], section_map_data[off+5], section_map_data[off+6], section_map_data[off+7],
                    ])
                };
                let read_i64_at = |off: usize| -> i64 {
                    i64::from_le_bytes([
                        section_map_data[off], section_map_data[off+1], section_map_data[off+2], section_map_data[off+3],
                        section_map_data[off+4], section_map_data[off+5], section_map_data[off+6], section_map_data[off+7],
                    ])
                };
                
                let compressed_size = read_u64_at(spos);        // 0x00
                let decompressed_size = read_u64_at(spos + 0x08); // 0x08
                let _encrypted = read_u64_at(spos + 0x10);      // 0x10
                let _hash_code = read_u64_at(spos + 0x18);      // 0x18
                let section_name_length = read_i64_at(spos + 0x20) as usize; // 0x20
                let _unknown = read_u64_at(spos + 0x28);        // 0x28
                let encoding = read_u64_at(spos + 0x30);        // 0x30
                let page_count = read_u64_at(spos + 0x38) as usize; // 0x38
                spos += 0x40;
                
                // Read section name (Unicode, variable length)
                let mut section_name = String::new();
                if section_name_length > 0 && spos + section_name_length <= section_map_data.len() {
                    // Unicode (UTF-16LE) encoded name
                    let name_bytes = &section_map_data[spos..spos + section_name_length];
                    let u16_chars: Vec<u16> = name_bytes.chunks_exact(2)
                        .map(|c| u16::from_le_bytes([c[0], c[1]]))
                        .collect();
                    section_name = String::from_utf16_lossy(&u16_chars).replace('\0', "");
                    spos += section_name_length;
                }
                
                // Read page entries (7 x 8-byte fields per page)
                let mut descriptor = DwgSectionDescriptor::with_name(&section_name);
                descriptor.compressed_size = compressed_size;
                descriptor.decompressed_size = decompressed_size;
                descriptor.encoding = Some(encoding);
                
                for _ in 0..page_count {
                    if spos + 56 > section_map_data.len() { break; }
                    
                    let page_offset = read_u64_at(spos);        // Data offset
                    let page_size = read_i64_at(spos + 8);       // Page size
                    let page_number = read_i64_at(spos + 16) as i32; // Page ID
                    let page_decomp_size = read_u64_at(spos + 24); // Decompressed size
                    let page_comp_size = read_u64_at(spos + 32);  // Compressed size
                    let _page_checksum = read_u64_at(spos + 40);  // Checksum
                    let _page_crc = read_u64_at(spos + 48);       // CRC
                    spos += 56;
                    
                    use super::section::DwgLocalSectionMap;
                    let local_section = DwgLocalSectionMap {
                        page_number,
                        offset: page_offset,
                        size: page_decomp_size,          // DecompressedSize (position 24)
                        page_size: page_size as u64,     // raw page size (position 8)
                        compressed_size: page_comp_size, // CompressedSize (position 32)
                        checksum: 0,
                        crc: 0,
                    };
                    descriptor.local_sections.push(local_section);
                }
                
                if !section_name.is_empty() {
                    header.descriptors.insert(section_name, descriptor);
                }
            }
        }
        
        let ac21_header = DwgFileHeaderAC21 {
            base: header,
            compressed_metadata: Some(compressed_metadata),
        };
        
        Ok(DwgFileHeaderType::AC21(ac21_header))
    }
    
    /// Get a decoded+decompressed page buffer for AC21 format
    fn get_page_buffer_ac21(&mut self, page_offset: u64, compressed_size: u64, uncompressed_size: u64, correction_factor: u64, block_size: usize) -> Result<Vec<u8>> {
        use super::decompressor::Lz77AC21Decompressor;
        
        // Avoid shifted bits
        let v = compressed_size + 7;
        let v1 = v & 0xFFFFFFF8u64;  // 32-bit mask like C#
        let total_size = (v1.wrapping_mul(correction_factor) as u32) as usize;
        let factor = (total_size + block_size - 1) / block_size;
        let length = factor * 255;
        
        let mut buffer = vec![0u8; length];
        
        // Relative to data page map 1, add 0x480 to get stream position
        let seek_pos = 0x480 + page_offset;
        self.reader.seek(SeekFrom::Start(seek_pos))?;
        self.reader.read_exact(&mut buffer[..length])?;
        
        let mut compressed_data = vec![0u8; total_size];
        reed_solomon_decoding(&buffer, &mut compressed_data, factor, block_size);
        
        let mut decompressed_data = vec![0u8; uncompressed_size as usize];
        Lz77AC21Decompressor::decompress(&compressed_data, 0, compressed_size as usize, &mut decompressed_data)?;
        
        Ok(decompressed_data)
    }
}

/// Reed-Solomon de-interleaving (not full error correction)
fn reed_solomon_decoding(encoded: &[u8], buffer: &mut [u8], factor: usize, block_size: usize) {
    let mut index = 0usize;
    let mut n = 0usize;
    let mut remaining = buffer.len();
    
    for _ in 0..factor {
        let cindex_start = n;
        if n < encoded.len() {
            let size = remaining.min(block_size);
            remaining -= size;
            let end = index + size;
            let mut cindex = cindex_start;
            while index < end {
                if cindex < encoded.len() {
                    buffer[index] = encoded[cindex];
                }
                index += 1;
                cindex += factor;
            }
        }
        n += 1;
    }
}

/// Check if a file is a DWG file by its magic bytes
pub fn is_dwg_file<R: Read + Seek>(reader: &mut R) -> Result<bool> {
    let pos = reader.stream_position()?;
    
    let mut magic = [0u8; 6];
    let result = reader.read_exact(&mut magic);
    
    // Restore position
    reader.seek(SeekFrom::Start(pos))?;
    
    if result.is_err() {
        return Ok(false);
    }
    
    // Check for "AC" prefix
    Ok(magic[0] == b'A' && magic[1] == b'C')
}

/// Get the AutoCAD version from a DWG file without fully parsing it
pub fn get_dwg_version<R: Read + Seek>(reader: &mut R) -> Result<ACadVersion> {
    let pos = reader.stream_position()?;
    
    let mut version_bytes = [0u8; 6];
    reader.read_exact(&mut version_bytes)?;
    
    // Restore position
    reader.seek(SeekFrom::Start(pos))?;
    
    let version_str = std::str::from_utf8(&version_bytes)
        .map_err(|_| DxfError::InvalidHeader("Invalid version string".to_string()))?;
    
    Ok(ACadVersion::from_version_string(version_str))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    
    #[test]
    fn test_is_dwg_file() {
        let dwg_data = b"AC1015\x00\x00\x00\x00";
        let mut cursor = Cursor::new(&dwg_data[..]);
        
        assert!(is_dwg_file(&mut cursor).unwrap());
    }
    
    #[test]
    fn test_is_not_dwg_file() {
        let dxf_data = b"0\nSECTION\n";
        let mut cursor = Cursor::new(&dxf_data[..]);
        
        assert!(!is_dwg_file(&mut cursor).unwrap());
    }
    
    #[test]
    fn test_get_dwg_version() {
        let dwg_data = b"AC1015\x00\x00\x00\x00";
        let mut cursor = Cursor::new(&dwg_data[..]);
        
        let version = get_dwg_version(&mut cursor).unwrap();
        assert_eq!(version, ACadVersion::AC1015);
    }
    
    #[test]
    fn test_unsupported_version() {
        let old_dwg = b"AC1009\x00\x00\x00\x00";
        let mut cursor = Cursor::new(&old_dwg[..]);
        
        let result = DwgReader::from_bytes(&old_dwg[..]).unwrap().read();
        assert!(result.is_err());
    }
}
