//! DWG writer — main orchestrator that writes a `CadDocument` to DWG format.
//!
//! This is the Rust equivalent of the C# `DwgWriter` class. It coordinates all
//! section writers and the file header writer to produce a valid DWG file.

use std::collections::HashMap;
use std::io::{Cursor, Seek, SeekFrom, Write};

use crate::document::{CadDocument, HeaderVariables};
use crate::error::{DxfError, Result};
use crate::types::DxfVersion;
use crate::io::dwg::DwgFileHeader;
use crate::io::dwg::DwgSectionDefinition;

use super::dwg_app_info_writer::DwgAppInfoWriter;
use super::dwg_aux_header_writer::DwgAuxHeaderWriter;
use super::dwg_classes_writer::DwgClassesWriter;
use super::dwg_file_header_writer_ac15::DwgFileHeaderWriterAc15;
use super::dwg_file_header_writer_ac18::DwgFileHeaderWriterAc18;
use super::dwg_handle_writer::DwgHandleWriter;
use super::dwg_header_writer::DwgHeaderWriter;
use super::dwg_preview_writer::{DwgPreview, DwgPreviewWriter};
use super::dwg_writer_configuration::DwgWriterConfiguration;
use super::idwg_stream_writer::DwgFileHeaderWriter;

/// DWG file writer.
pub struct DwgWriter<W: Write + Seek> {
    stream: W,
    document: CadDocument,
    config: DwgWriterConfiguration,
    preview: Option<DwgPreview>,
    handles_map: HashMap<u64, i64>,
}

impl DwgWriter<std::io::BufWriter<std::fs::File>> {
    /// Create a writer that writes to a file path.
    pub fn from_path(
        filename: &str,
        document: CadDocument,
    ) -> Result<Self> {
        let file = std::fs::File::create(filename)
            .map_err(DxfError::Io)?;
        Ok(Self::new(std::io::BufWriter::new(file), document))
    }
}

impl<W: Write + Seek> DwgWriter<W> {
    /// Create a writer that writes to any `Write + Seek` stream.
    pub fn new(stream: W, document: CadDocument) -> Self {
        Self {
            stream,
            document,
            config: DwgWriterConfiguration::default(),
            preview: None,
            handles_map: HashMap::new(),
        }
    }

    /// Set the writer configuration.
    pub fn with_config(mut self, config: DwgWriterConfiguration) -> Self {
        self.config = config;
        self
    }

    /// Set the preview image.
    pub fn with_preview(mut self, preview: DwgPreview) -> Self {
        self.preview = Some(preview);
        self
    }

    /// Write the DWG file.
    pub fn write(&mut self) -> Result<()> {
        let version = self.document.version;

        // Validate version
        if version < DxfVersion::AC1014 {
            return Err(DxfError::UnsupportedVersion(format!(
                "DWG writing not supported for version {:?}",
                version
            )));
        }
        if version == DxfVersion::AC1021 {
            return Err(DxfError::UnsupportedVersion(
                "AC1021 (2007) writing not currently supported".into(),
            ));
        }

        let maint_ver = version.maintenance_version();

        let mut file_header_writer: Box<dyn DwgFileHeaderWriter> = match version {
            DxfVersion::AC1014 | DxfVersion::AC1015 => {
                Box::new(DwgFileHeaderWriterAc15::new(
                    Box::new(Cursor::new(Vec::new())),
                    version,
                    version.to_string(),
                    "windows-1252".to_string(),
                    maint_ver,
                ))
            }
            DxfVersion::AC1018
            | DxfVersion::AC1024
            | DxfVersion::AC1027
            | DxfVersion::AC1032 => {
                Box::new(DwgFileHeaderWriterAc18::new(
                    version,
                    version.to_string(),
                    "windows-1252".to_string(),
                    maint_ver,
                ))
            }
            _ => {
                return Err(DxfError::UnsupportedVersion(format!(
                    "Unsupported DWG version {:?}",
                    version
                )));
            }
        };

        // Write all sections
        self.write_header(version, &mut *file_header_writer)?;
        self.write_classes(version, &mut *file_header_writer)?;
        self.write_summary_info(version, &mut *file_header_writer)?;
        self.write_preview(version, &mut *file_header_writer)?;
        self.write_app_info(version, &mut *file_header_writer)?;
        self.write_file_dep_list(version, &mut *file_header_writer)?;
        self.write_rev_history(version, &mut *file_header_writer)?;
        self.write_aux_header(version, &mut *file_header_writer)?;
        // Objects section: writeObjects produces handles_map — currently a placeholder
        self.write_objects(version, &mut *file_header_writer)?;
        self.write_obj_free_space(version, &mut *file_header_writer)?;
        self.write_template(version, &mut *file_header_writer)?;
        self.write_handles(version, &mut *file_header_writer)?;

        // Finalize: write file header + all section data to output stream
        file_header_writer.write_file()?;

        // Copy file header writer output to our stream
        // The file header writer already wrote everything to its internal stream.
        // We need to transfer that data to the output.
        // For now the file header writer writes to its own Cursor — we'd extract
        // the bytes and write them to self.stream.
        // This architecture detail depends on the file header writer design:
        // In the current implementation, the writers take the output stream reference.
        // TODO: integrate file header writer with the output stream directly.

        self.stream.flush().map_err(DxfError::Io)?;

        Ok(())
    }

    fn write_header(
        &self,
        version: DxfVersion,
        fhw: &mut dyn DwgFileHeaderWriter,
    ) -> Result<()> {
        let data = DwgHeaderWriter::write(version, &self.document.header)?;
        fhw.add_section(DwgSectionDefinition::HEADER, data, true, 0);
        Ok(())
    }

    fn write_classes(
        &self,
        version: DxfVersion,
        fhw: &mut dyn DwgFileHeaderWriter,
    ) -> Result<()> {
        let classes: Vec<_> = self.document.classes.iter().cloned().collect();
        let data = DwgClassesWriter::write(
            version,
            &classes,
            version.maintenance_version(),
        )?;
        fhw.add_section(DwgSectionDefinition::CLASSES, data, true, 0);
        Ok(())
    }

    fn write_summary_info(
        &self,
        version: DxfVersion,
        fhw: &mut dyn DwgFileHeaderWriter,
    ) -> Result<()> {
        if version < DxfVersion::AC1018 {
            return Ok(());
        }

        // Write summary info section: title, subject, author, etc.
        let mut buf = Vec::new();

        // Write empty summary for now (matching ODA minimal implementation)
        // Title, Subject, Author, Keywords, Comments, LastSavedBy, RevisionNumber, HyperlinkBase
        for _ in 0..8 {
            // Unicode string: u16 length + UTF-16LE data
            buf.extend_from_slice(&0u16.to_le_bytes());
        }

        // Total editing time (two zero Int32s)
        buf.extend_from_slice(&0i32.to_le_bytes());
        buf.extend_from_slice(&0i32.to_le_bytes());

        // Created date / Modified date (8 bytes each)
        buf.extend_from_slice(&0i32.to_le_bytes());
        buf.extend_from_slice(&0i32.to_le_bytes());
        buf.extend_from_slice(&0i32.to_le_bytes());
        buf.extend_from_slice(&0i32.to_le_bytes());

        // Property count = 0
        buf.extend_from_slice(&0u16.to_le_bytes());

        // Padding
        buf.extend_from_slice(&0i32.to_le_bytes());
        buf.extend_from_slice(&0i32.to_le_bytes());

        fhw.add_section(DwgSectionDefinition::SUMMARY_INFO, buf, false, 0x100);
        Ok(())
    }

    fn write_preview(
        &self,
        version: DxfVersion,
        fhw: &mut dyn DwgFileHeaderWriter,
    ) -> Result<()> {
        let data = DwgPreviewWriter::write_empty(version)?;
        fhw.add_section(DwgSectionDefinition::PREVIEW, data, false, 0x400);
        Ok(())
    }

    fn write_app_info(
        &self,
        version: DxfVersion,
        fhw: &mut dyn DwgFileHeaderWriter,
    ) -> Result<()> {
        if version < DxfVersion::AC1018 {
            return Ok(());
        }

        let data = DwgAppInfoWriter::write(version)?;
        fhw.add_section(DwgSectionDefinition::APP_INFO, data, false, 0x80);
        Ok(())
    }

    fn write_file_dep_list(
        &self,
        version: DxfVersion,
        fhw: &mut dyn DwgFileHeaderWriter,
    ) -> Result<()> {
        if version < DxfVersion::AC1018 {
            return Ok(());
        }

        let mut buf = Vec::new();
        // Feature count: 0
        buf.extend_from_slice(&0u32.to_le_bytes());
        // File count: 0
        buf.extend_from_slice(&0u32.to_le_bytes());

        fhw.add_section(DwgSectionDefinition::FILE_DEP_LIST, buf, false, 0x80);
        Ok(())
    }

    fn write_rev_history(
        &self,
        version: DxfVersion,
        fhw: &mut dyn DwgFileHeaderWriter,
    ) -> Result<()> {
        if version < DxfVersion::AC1018 {
            return Ok(());
        }

        let mut buf = Vec::new();
        buf.extend_from_slice(&0u32.to_le_bytes());
        buf.extend_from_slice(&0u32.to_le_bytes());
        buf.extend_from_slice(&0u32.to_le_bytes());

        fhw.add_section(DwgSectionDefinition::REV_HISTORY, buf, true, 0);
        Ok(())
    }

    fn write_aux_header(
        &self,
        version: DxfVersion,
        fhw: &mut dyn DwgFileHeaderWriter,
    ) -> Result<()> {
        let header = &self.document.header;
        let (c_jdate, c_ms) = julian_from_f64(header.create_date_julian);
        let (u_jdate, u_ms) = julian_from_f64(header.update_date_julian);

        let data = DwgAuxHeaderWriter::write(
            version,
            version.maintenance_version(),
            c_jdate,
            c_ms,
            u_jdate,
            u_ms,
            header.handle_seed,
        )?;

        fhw.add_section(DwgSectionDefinition::AUX_HEADER, data, true, 0);
        Ok(())
    }

    fn write_objects(
        &mut self,
        version: DxfVersion,
        fhw: &mut dyn DwgFileHeaderWriter,
    ) -> Result<()> {
        // The DwgObjectWriter is a large multi-file partial class (1400+ lines in C#).
        // It writes all table objects, block entities, and non-graphical objects.
        // For now, write a minimal objects section.
        let mut buf = Vec::new();

        if version >= DxfVersion::AC1018 {
            // R2004+ start marker
            buf.extend_from_slice(&0x0DCAi32.to_le_bytes());
        }

        // The handles_map would be populated by DwgObjectWriter.
        // Left empty for now — this means the HANDLES section will be empty too.
        self.handles_map.clear();

        fhw.add_section(DwgSectionDefinition::ACDB_OBJECTS, buf, true, 0);
        Ok(())
    }

    fn write_obj_free_space(
        &self,
        version: DxfVersion,
        fhw: &mut dyn DwgFileHeaderWriter,
    ) -> Result<()> {
        let mut buf = Vec::new();

        // Int32: 0
        buf.extend_from_slice(&0i32.to_le_bytes());
        // UInt32: approximate number of objects
        buf.extend_from_slice(&(self.handles_map.len() as u32).to_le_bytes());

        // Julian datetime
        let (jdate, ms) = julian_from_f64(self.document.header.update_date_julian);
        buf.extend_from_slice(&jdate.to_le_bytes());
        buf.extend_from_slice(&ms.to_le_bytes());

        // Offset of objects section
        buf.extend_from_slice(&0u32.to_le_bytes());

        // Number of 64-bit values: 4
        buf.push(4);
        buf.extend_from_slice(&0x00000032u32.to_le_bytes());
        buf.extend_from_slice(&0x00000000u32.to_le_bytes());
        buf.extend_from_slice(&0x00000064u32.to_le_bytes());
        buf.extend_from_slice(&0x00000000u32.to_le_bytes());
        buf.extend_from_slice(&0x00000200u32.to_le_bytes());
        buf.extend_from_slice(&0x00000000u32.to_le_bytes());
        buf.extend_from_slice(&0xffffffffu32.to_le_bytes());
        buf.extend_from_slice(&0x00000000u32.to_le_bytes());

        fhw.add_section(DwgSectionDefinition::OBJ_FREE_SPACE, buf, true, 0);
        Ok(())
    }

    fn write_template(
        &self,
        _version: DxfVersion,
        fhw: &mut dyn DwgFileHeaderWriter,
    ) -> Result<()> {
        let mut buf = Vec::new();

        // Int16: template description length = 0
        buf.extend_from_slice(&0i16.to_le_bytes());
        // UInt16: MEASUREMENT (1 = Metric)
        buf.extend_from_slice(&1u16.to_le_bytes());

        fhw.add_section(DwgSectionDefinition::TEMPLATE, buf, true, 0);
        Ok(())
    }

    fn write_handles(
        &self,
        version: DxfVersion,
        fhw: &mut dyn DwgFileHeaderWriter,
    ) -> Result<()> {
        let section_offset = fhw.handle_section_offset();

        let sorted_map: std::collections::BTreeMap<u64, i64> =
            self.handles_map.iter().map(|(&k, &v)| (k, v)).collect();
        let mut handle_writer = DwgHandleWriter::new(
            version,
            Cursor::new(Vec::new()),
            sorted_map,
        );
        handle_writer.write(section_offset)?;
        let data = handle_writer.into_inner();

        fhw.add_section(DwgSectionDefinition::HANDLES, data, true, 0);
        Ok(())
    }
}

/// Convenience function: write a document to a file.
pub fn write_dwg(filename: &str, document: CadDocument) -> Result<()> {
    let mut writer = DwgWriter::from_path(filename, document)?;
    writer.write()
}

/// Convenience function: write a document to a byte vector.
pub fn write_dwg_to_bytes(document: CadDocument) -> Result<Vec<u8>> {
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = DwgWriter::new(&mut cursor, document);
        writer.write()?;
    }
    Ok(cursor.into_inner())
}

/// Convert f64 julian date to (day, milliseconds) pair.
fn julian_from_f64(julian: f64) -> (i32, i32) {
    let day = julian as i32;
    let frac = julian - day as f64;
    let ms = (frac * 86_400_000.0) as i32;
    (day, ms)
}
