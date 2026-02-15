//! DWG file header structures for all supported versions.
//!
//! The DWG binary format has different file header layouts depending on the
//! AutoCAD version:
//!
//! - **AC15** (R2000): record-based section locators
//! - **AC18** (R2004): page-based section descriptors
//! - **AC21** (R2007): page-based with compressed metadata

use std::collections::HashMap;

use crate::error::DxfError;
use crate::types::DxfVersion;

use super::{
    Dwg21CompressedMetadata, DwgSectionDescriptor, DwgSectionLocatorRecord,
};

// ── AC15 file header (R13 / R14 / R2000) ──────────────────────────────────

/// End sentinel for AC15 file headers.
pub const AC15_END_SENTINEL: [u8; 16] = [
    0x95, 0xA0, 0x4E, 0x28, 0x99, 0x82, 0x1A, 0xE5, 0x5E, 0x41, 0xE0, 0x5F, 0x9D, 0x3A, 0x4D,
    0x00,
];

/// File header data specific to AC15 (R13/R14/R2000).
#[derive(Debug, Clone)]
pub struct DwgFileHeaderAC15 {
    /// Record-based section locators keyed by record number.
    pub records: HashMap<i32, DwgSectionLocatorRecord>,
}

impl Default for DwgFileHeaderAC15 {
    fn default() -> Self {
        Self {
            records: HashMap::new(),
        }
    }
}

// ── AC18 file header (R2004 and above) ─────────────────────────────────────

/// Additional file header data for AC18 (R2004) and later.
///
/// Inherits from AC15 and adds page-based section descriptors.
#[derive(Debug, Clone)]
pub struct DwgFileHeaderAC18 {
    /// AC15 base data (records).
    pub ac15: DwgFileHeaderAC15,
    /// DWG internal version byte.
    pub dwg_version: u8,
    /// Application release version byte.
    pub app_release_version: u8,
    /// Address of the summary info section.
    pub summary_info_addr: i64,
    /// Security type flag.
    pub security_type: i64,
    /// Address of the VBA project section.
    pub vba_project_addr: i64,
    /// Root tree node gap.
    pub root_tree_node_gap: i32,
    /// Gap array size.
    pub gap_array_size: u32,
    /// CRC seed value.
    pub crc_seed: u32,
    /// Last page id.
    pub last_page_id: i32,
    /// Last section address.
    pub last_section_addr: u64,
    /// Second header address.
    pub second_header_addr: u64,
    /// Number of gaps.
    pub gap_amount: u32,
    /// Number of sections.
    pub section_amount: u32,
    /// Section page map id.
    pub section_page_map_id: u32,
    /// Page map address.
    pub page_map_address: u64,
    /// Section map id.
    pub section_map_id: u32,
    /// Section array page size.
    pub section_array_page_size: u32,
    /// Right gap.
    pub right_gap: i32,
    /// Left gap.
    pub left_gap: i32,
    /// Named section descriptors keyed by section name.
    pub descriptors: HashMap<String, DwgSectionDescriptor>,
}

impl Default for DwgFileHeaderAC18 {
    fn default() -> Self {
        Self {
            ac15: DwgFileHeaderAC15::default(),
            dwg_version: 0,
            app_release_version: 0,
            summary_info_addr: 0,
            security_type: 0,
            vba_project_addr: 0,
            root_tree_node_gap: 0,
            gap_array_size: 0,
            crc_seed: 0,
            last_page_id: 0,
            last_section_addr: 0,
            second_header_addr: 0,
            gap_amount: 0,
            section_amount: 0,
            section_page_map_id: 0,
            page_map_address: 0,
            section_map_id: 0,
            section_array_page_size: 0,
            right_gap: 0,
            left_gap: 0,
            descriptors: HashMap::new(),
        }
    }
}

// ── AC21 file header (R2007) ───────────────────────────────────────────────

/// Additional file header data for AC21 (R2007).
///
/// Extends AC18 with compressed metadata.
#[derive(Debug, Clone)]
pub struct DwgFileHeaderAC21 {
    /// AC18 base data.
    pub ac18: DwgFileHeaderAC18,
    /// Compressed metadata block.
    pub compressed_metadata: Dwg21CompressedMetadata,
}

impl Default for DwgFileHeaderAC21 {
    fn default() -> Self {
        Self {
            ac18: DwgFileHeaderAC18::default(),
            compressed_metadata: Dwg21CompressedMetadata::new(),
        }
    }
}

// ── Unified DWG file header ────────────────────────────────────────────────

/// Unified DWG file header that holds version-specific data.
#[derive(Debug, Clone)]
pub struct DwgFileHeader {
    /// The AutoCAD version of this file.
    pub version: DxfVersion,
    /// Address of the preview image (-1 if absent).
    pub preview_address: i64,
    /// AutoCAD maintenance version number.
    pub acad_maintenance_version: i32,
    /// Drawing code page string (e.g., `"ANSI_1252"`).
    pub drawing_code_page: String,
    /// Version-specific header data.
    pub data: DwgFileHeaderData,
}

/// Version-specific portion of a [`DwgFileHeader`].
#[derive(Debug, Clone)]
pub enum DwgFileHeaderData {
    /// AC15 (R13 / R14 / R2000) header data.
    AC15(DwgFileHeaderAC15),
    /// AC18 (R2004+) header data.
    AC18(DwgFileHeaderAC18),
    /// AC21 (R2007) header data.
    AC21(DwgFileHeaderAC21),
}

impl DwgFileHeader {
    /// Create a file header for the given version.
    ///
    /// # Errors
    ///
    /// Returns `DxfError::UnsupportedVersion` for versions older than AC1012
    /// or for `DxfVersion::Unknown`.
    pub fn create(version: DxfVersion) -> Result<Self, DxfError> {
        let data = match version {
            DxfVersion::Unknown => {
                return Err(DxfError::UnsupportedVersion("Unknown".into()));
            }
            DxfVersion::AC1012 | DxfVersion::AC1014 | DxfVersion::AC1015 => {
                DwgFileHeaderData::AC15(DwgFileHeaderAC15::default())
            }
            DxfVersion::AC1018 => {
                DwgFileHeaderData::AC18(DwgFileHeaderAC18::default())
            }
            DxfVersion::AC1021 => {
                DwgFileHeaderData::AC21(DwgFileHeaderAC21::default())
            }
            // AC1024, AC1027, AC1032 use the AC18 layout
            DxfVersion::AC1024 | DxfVersion::AC1027 | DxfVersion::AC1032 => {
                DwgFileHeaderData::AC18(DwgFileHeaderAC18::default())
            }
        };

        Ok(Self {
            version,
            preview_address: -1,
            acad_maintenance_version: 0,
            drawing_code_page: String::new(),
            data,
        })
    }

    /// Add a named section to this header.
    ///
    /// Only meaningful for AC18+ headers that use named descriptors.
    /// AC15 headers will return a `NotImplemented` error.
    pub fn add_section(&mut self, name: &str) -> Result<(), DxfError> {
        match &mut self.data {
            DwgFileHeaderData::AC15(_) => Err(DxfError::NotImplemented(
                "AddSection not supported for AC15 file headers".into(),
            )),
            DwgFileHeaderData::AC18(ref mut ac18) => {
                ac18.descriptors
                    .insert(name.to_string(), DwgSectionDescriptor::with_name(name));
                Ok(())
            }
            DwgFileHeaderData::AC21(ref mut ac21) => {
                ac21.ac18
                    .descriptors
                    .insert(name.to_string(), DwgSectionDescriptor::with_name(name));
                Ok(())
            }
        }
    }

    /// Add a pre-built section descriptor (AC18+ only).
    pub fn add_section_descriptor(
        &mut self,
        descriptor: DwgSectionDescriptor,
    ) -> Result<(), DxfError> {
        match &mut self.data {
            DwgFileHeaderData::AC15(_) => Err(DxfError::NotImplemented(
                "AddSection not supported for AC15 file headers".into(),
            )),
            DwgFileHeaderData::AC18(ref mut ac18) => {
                ac18.descriptors
                    .insert(descriptor.name.clone(), descriptor);
                Ok(())
            }
            DwgFileHeaderData::AC21(ref mut ac21) => {
                ac21.ac18
                    .descriptors
                    .insert(descriptor.name.clone(), descriptor);
                Ok(())
            }
        }
    }

    /// Get a section descriptor by name.
    ///
    /// Returns `None` for AC15 headers or if the section is not found.
    pub fn get_descriptor(&self, name: &str) -> Option<&DwgSectionDescriptor> {
        match &self.data {
            DwgFileHeaderData::AC15(_) => None,
            DwgFileHeaderData::AC18(ac18) => ac18.descriptors.get(name),
            DwgFileHeaderData::AC21(ac21) => ac21.ac18.descriptors.get(name),
        }
    }

    /// Get a mutable reference to a section descriptor by name.
    pub fn get_descriptor_mut(&mut self, name: &str) -> Option<&mut DwgSectionDescriptor> {
        match &mut self.data {
            DwgFileHeaderData::AC15(_) => None,
            DwgFileHeaderData::AC18(ac18) => ac18.descriptors.get_mut(name),
            DwgFileHeaderData::AC21(ac21) => ac21.ac18.descriptors.get_mut(name),
        }
    }

    /// Get a reference to the AC15 data, if this is an AC15-based header.
    pub fn as_ac15(&self) -> Option<&DwgFileHeaderAC15> {
        match &self.data {
            DwgFileHeaderData::AC15(ac15) => Some(ac15),
            DwgFileHeaderData::AC18(ac18) => Some(&ac18.ac15),
            DwgFileHeaderData::AC21(ac21) => Some(&ac21.ac18.ac15),
        }
    }

    /// Get a mutable reference to the AC15 data.
    pub fn as_ac15_mut(&mut self) -> Option<&mut DwgFileHeaderAC15> {
        match &mut self.data {
            DwgFileHeaderData::AC15(ac15) => Some(ac15),
            DwgFileHeaderData::AC18(ac18) => Some(&mut ac18.ac15),
            DwgFileHeaderData::AC21(ac21) => Some(&mut ac21.ac18.ac15),
        }
    }

    /// Get a reference to the AC18 data, if available.
    pub fn as_ac18(&self) -> Option<&DwgFileHeaderAC18> {
        match &self.data {
            DwgFileHeaderData::AC15(_) => None,
            DwgFileHeaderData::AC18(ac18) => Some(ac18),
            DwgFileHeaderData::AC21(ac21) => Some(&ac21.ac18),
        }
    }

    /// Get a mutable reference to the AC18 data, if available.
    pub fn as_ac18_mut(&mut self) -> Option<&mut DwgFileHeaderAC18> {
        match &mut self.data {
            DwgFileHeaderData::AC15(_) => None,
            DwgFileHeaderData::AC18(ac18) => Some(ac18),
            DwgFileHeaderData::AC21(ac21) => Some(&mut ac21.ac18),
        }
    }

    /// Get a reference to the AC21 data, if this is an AC21 header.
    pub fn as_ac21(&self) -> Option<&DwgFileHeaderAC21> {
        match &self.data {
            DwgFileHeaderData::AC21(ac21) => Some(ac21),
            _ => None,
        }
    }

    /// Get a mutable reference to the AC21 data.
    pub fn as_ac21_mut(&mut self) -> Option<&mut DwgFileHeaderAC21> {
        match &mut self.data {
            DwgFileHeaderData::AC21(ac21) => Some(ac21),
            _ => None,
        }
    }

    /// Check whether this header uses AC18+ page-based sections.
    pub fn is_page_based(&self) -> bool {
        matches!(
            self.data,
            DwgFileHeaderData::AC18(_) | DwgFileHeaderData::AC21(_)
        )
    }

    /// Check whether this header uses AC15 record-based sections.
    pub fn is_record_based(&self) -> bool {
        matches!(self.data, DwgFileHeaderData::AC15(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_ac15_versions() {
        for ver in [DxfVersion::AC1012, DxfVersion::AC1014, DxfVersion::AC1015] {
            let hdr = DwgFileHeader::create(ver).unwrap();
            assert_eq!(hdr.version, ver);
            assert!(hdr.is_record_based());
            assert!(!hdr.is_page_based());
            assert!(hdr.as_ac15().is_some());
            assert!(hdr.as_ac18().is_none());
            assert!(hdr.as_ac21().is_none());
        }
    }

    #[test]
    fn test_create_ac18() {
        let hdr = DwgFileHeader::create(DxfVersion::AC1018).unwrap();
        assert_eq!(hdr.version, DxfVersion::AC1018);
        assert!(hdr.is_page_based());
        assert!(hdr.as_ac18().is_some());
        assert!(hdr.as_ac21().is_none());
    }

    #[test]
    fn test_create_ac21() {
        let hdr = DwgFileHeader::create(DxfVersion::AC1021).unwrap();
        assert_eq!(hdr.version, DxfVersion::AC1021);
        assert!(hdr.is_page_based());
        assert!(hdr.as_ac18().is_some()); // AC21 includes AC18
        assert!(hdr.as_ac21().is_some());
    }

    #[test]
    fn test_create_ac1024_uses_ac18() {
        let hdr = DwgFileHeader::create(DxfVersion::AC1024).unwrap();
        assert!(hdr.is_page_based());
        assert!(matches!(hdr.data, DwgFileHeaderData::AC18(_)));
    }

    #[test]
    fn test_create_unknown_error() {
        let result = DwgFileHeader::create(DxfVersion::Unknown);
        assert!(result.is_err());
    }

    #[test]
    fn test_default_preview_address() {
        let hdr = DwgFileHeader::create(DxfVersion::AC1015).unwrap();
        assert_eq!(hdr.preview_address, -1);
    }

    #[test]
    fn test_add_section_ac18() {
        let mut hdr = DwgFileHeader::create(DxfVersion::AC1018).unwrap();
        hdr.add_section("AcDb:Header").unwrap();
        assert!(hdr.get_descriptor("AcDb:Header").is_some());
        assert_eq!(
            hdr.get_descriptor("AcDb:Header").unwrap().name,
            "AcDb:Header"
        );
    }

    #[test]
    fn test_add_section_ac15_errors() {
        let mut hdr = DwgFileHeader::create(DxfVersion::AC1015).unwrap();
        assert!(hdr.add_section("AcDb:Header").is_err());
    }

    #[test]
    fn test_add_section_descriptor() {
        let mut hdr = DwgFileHeader::create(DxfVersion::AC1021).unwrap();
        let mut desc = DwgSectionDescriptor::with_name("AcDb:Classes");
        desc.section_id = 42;
        hdr.add_section_descriptor(desc).unwrap();
        let retrieved = hdr.get_descriptor("AcDb:Classes").unwrap();
        assert_eq!(retrieved.section_id, 42);
    }

    #[test]
    fn test_get_descriptor_mut() {
        let mut hdr = DwgFileHeader::create(DxfVersion::AC1018).unwrap();
        hdr.add_section("AcDb:Handles").unwrap();
        let desc = hdr.get_descriptor_mut("AcDb:Handles").unwrap();
        desc.section_id = 99;
        assert_eq!(hdr.get_descriptor("AcDb:Handles").unwrap().section_id, 99);
    }

    #[test]
    fn test_ac15_sentinel() {
        assert_eq!(AC15_END_SENTINEL.len(), 16);
        assert_eq!(AC15_END_SENTINEL[0], 0x95);
        assert_eq!(AC15_END_SENTINEL[15], 0x00);
    }
}
