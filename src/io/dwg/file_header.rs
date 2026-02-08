//! DWG File Header structures
//!
//! The DWG file format has different header structures for different AutoCAD versions.
//! This module provides types for all supported versions:
//!
//! - AC1012/AC1014/AC1015 (R13-2002): DwgFileHeaderAC15
//! - AC1018 (2004-2006): DwgFileHeaderAC18
//! - AC1021+ (2007+): DwgFileHeaderAC21

use std::collections::HashMap;
use crate::types::ACadVersion;
use crate::error::{DxfError, Result};
use super::section::{DwgSectionLocatorRecord, DwgSectionDescriptor, Dwg21CompressedMetadata};

/// Code page enumeration for drawing files
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CodePage {
    /// No code page specified
    #[default]
    None = 0,
    /// US ASCII
    UsAscii = 1,
    /// ISO 8859-1 (Latin 1)
    Iso8859_1 = 2,
    /// ISO 8859-2 (Latin 2)
    Iso8859_2 = 3,
    /// ISO 8859-3 (Latin 3)
    Iso8859_3 = 4,
    /// ISO 8859-4 (Latin 4)
    Iso8859_4 = 5,
    /// ISO 8859-5 (Cyrillic)
    Iso8859_5 = 6,
    /// ISO 8859-6 (Arabic)
    Iso8859_6 = 7,
    /// ISO 8859-7 (Greek)
    Iso8859_7 = 8,
    /// ISO 8859-8 (Hebrew)
    Iso8859_8 = 9,
    /// ISO 8859-9 (Turkish)
    Iso8859_9 = 10,
    /// CP437 (DOS United States)
    Cp437 = 11,
    /// CP850 (DOS Multilingual Latin 1)
    Cp850 = 12,
    /// CP852 (DOS Central Europe)
    Cp852 = 13,
    /// CP855 (DOS Cyrillic)
    Cp855 = 14,
    /// CP857 (DOS Turkish)
    Cp857 = 15,
    /// CP860 (DOS Portuguese)
    Cp860 = 16,
    /// CP861 (DOS Icelandic)
    Cp861 = 17,
    /// CP863 (DOS French Canada)
    Cp863 = 18,
    /// CP864 (DOS Arabic)
    Cp864 = 19,
    /// CP865 (DOS Nordic)
    Cp865 = 20,
    /// CP869 (DOS Greek)
    Cp869 = 21,
    /// CP932 (Japanese Shift-JIS)
    Cp932 = 22,
    /// CP936 (Simplified Chinese GBK)
    Cp936 = 23,
    /// CP949 (Korean)
    Cp949 = 24,
    /// CP950 (Traditional Chinese Big5)
    Cp950 = 25,
    /// CP1250 (Windows Central Europe)
    Cp1250 = 26,
    /// CP1251 (Windows Cyrillic)
    Cp1251 = 27,
    /// CP1252 (Windows Western Europe)
    Cp1252 = 28,
    /// CP1253 (Windows Greek)
    Cp1253 = 29,
    /// CP1254 (Windows Turkish)
    Cp1254 = 30,
    /// CP1255 (Windows Hebrew)
    Cp1255 = 31,
    /// CP1256 (Windows Arabic)
    Cp1256 = 32,
    /// CP1257 (Windows Baltic)
    Cp1257 = 33,
    /// CP1258 (Windows Vietnamese)
    Cp1258 = 34,
    /// UTF-8
    Utf8 = 35,
    /// ANSI (System default)
    Ansi = 100,
}

impl CodePage {
    /// Get CodePage from raw value
    pub fn from_value(value: i16) -> Self {
        match value {
            0 => CodePage::None,
            1 => CodePage::UsAscii,
            2 => CodePage::Iso8859_1,
            3 => CodePage::Iso8859_2,
            11 => CodePage::Cp437,
            12 => CodePage::Cp850,
            22 => CodePage::Cp932,
            23 => CodePage::Cp936,
            24 => CodePage::Cp949,
            25 => CodePage::Cp950,
            26 => CodePage::Cp1250,
            27 => CodePage::Cp1251,
            28 => CodePage::Cp1252,
            29 => CodePage::Cp1253,
            30 => CodePage::Cp1254,
            31 => CodePage::Cp1255,
            32 => CodePage::Cp1256,
            33 => CodePage::Cp1257,
            34 => CodePage::Cp1258,
            35 => CodePage::Utf8,
            _ => CodePage::Ansi,
        }
    }
}

/// Trait for DWG file headers
pub trait DwgFileHeader {
    /// Get the AutoCAD version
    fn version(&self) -> ACadVersion;
    
    /// Get the preview address (-1 if not present)
    fn preview_address(&self) -> i64;
    
    /// Set the preview address
    fn set_preview_address(&mut self, addr: i64);
    
    /// Get the maintenance version
    fn maintenance_version(&self) -> i32;
    
    /// Set the maintenance version
    fn set_maintenance_version(&mut self, version: i32);
    
    /// Get the drawing code page
    fn code_page(&self) -> CodePage;
    
    /// Set the drawing code page
    fn set_code_page(&mut self, code_page: CodePage);
    
    /// Add a section by name
    fn add_section(&mut self, name: &str);
    
    /// Get a section descriptor by name
    fn get_descriptor(&self, name: &str) -> Option<&DwgSectionDescriptor>;
    
    /// Get a mutable section descriptor by name
    fn get_descriptor_mut(&mut self, name: &str) -> Option<&mut DwgSectionDescriptor>;
}

/// File header for AC1012/AC1014/AC1015 (R13-2002)
#[derive(Debug, Clone)]
pub struct DwgFileHeaderAC15 {
    /// AutoCAD version
    pub acad_version: ACadVersion,
    /// Preview address (-1 if not present)
    pub preview_address: i64,
    /// Maintenance version
    pub maintenance_version: i32,
    /// Drawing code page
    pub code_page: CodePage,
    /// Section locator records
    pub records: HashMap<i32, DwgSectionLocatorRecord>,
}

impl DwgFileHeaderAC15 {
    /// End sentinel for AC15 file header
    pub const END_SENTINEL: [u8; 16] = [
        0x95, 0xA0, 0x4E, 0x28, 0x99, 0x82, 0x1A, 0xE5,
        0x5E, 0x41, 0xE0, 0x5F, 0x9D, 0x3A, 0x4D, 0x00,
    ];
    
    /// Create a new AC15 file header
    pub fn new(version: ACadVersion) -> Self {
        Self {
            acad_version: version,
            preview_address: -1,
            maintenance_version: 0,
            code_page: CodePage::default(),
            records: HashMap::new(),
        }
    }
    
    /// Add a section locator record
    pub fn add_record(&mut self, number: i32, seeker: i64, size: i64) {
        self.records.insert(number, DwgSectionLocatorRecord::with_values(Some(number), seeker, size));
    }
    
    /// Get a section locator record
    pub fn get_record(&self, number: i32) -> Option<&DwgSectionLocatorRecord> {
        self.records.get(&number)
    }
}

impl Default for DwgFileHeaderAC15 {
    fn default() -> Self {
        Self::new(ACadVersion::AC1015)
    }
}

impl DwgFileHeader for DwgFileHeaderAC15 {
    fn version(&self) -> ACadVersion {
        self.acad_version
    }
    
    fn preview_address(&self) -> i64 {
        self.preview_address
    }
    
    fn set_preview_address(&mut self, addr: i64) {
        self.preview_address = addr;
    }
    
    fn maintenance_version(&self) -> i32 {
        self.maintenance_version
    }
    
    fn set_maintenance_version(&mut self, version: i32) {
        self.maintenance_version = version;
    }
    
    fn code_page(&self) -> CodePage {
        self.code_page
    }
    
    fn set_code_page(&mut self, code_page: CodePage) {
        self.code_page = code_page;
    }
    
    fn add_section(&mut self, _name: &str) {
        // AC15 uses numeric section locators, not named sections
    }
    
    fn get_descriptor(&self, _name: &str) -> Option<&DwgSectionDescriptor> {
        None
    }
    
    fn get_descriptor_mut(&mut self, _name: &str) -> Option<&mut DwgSectionDescriptor> {
        None
    }
}

/// File header for AC1018 (2004-2006)
#[derive(Debug, Clone)]
pub struct DwgFileHeaderAC18 {
    /// Base AC15 header fields
    pub base: DwgFileHeaderAC15,
    /// DWG version byte
    pub dwg_version: u8,
    /// Application release version
    pub app_release_version: u8,
    /// Summary info address
    pub summary_info_addr: i64,
    /// Security type
    pub security_type: i64,
    /// VBA project address
    pub vba_project_addr: i64,
    /// Root tree node gap
    pub root_tree_node_gap: i32,
    /// Gap array size
    pub gap_array_size: u32,
    /// CRC seed
    pub crc_seed: u32,
    /// Last page ID
    pub last_page_id: i32,
    /// Last section address
    pub last_section_addr: u64,
    /// Second header address
    pub second_header_addr: u64,
    /// Gap amount
    pub gap_amount: u32,
    /// Section amount
    pub section_amount: u32,
    /// Section page map ID
    pub section_page_map_id: u32,
    /// Page map address
    pub page_map_address: u64,
    /// Section map ID
    pub section_map_id: u32,
    /// Section array page size
    pub section_array_page_size: u32,
    /// Right gap
    pub right_gap: i32,
    /// Left gap
    pub left_gap: i32,
    /// Section descriptors
    pub descriptors: HashMap<String, DwgSectionDescriptor>,
}

impl DwgFileHeaderAC18 {
    /// Create a new AC18 file header
    pub fn new(version: ACadVersion) -> Self {
        Self {
            base: DwgFileHeaderAC15::new(version),
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
    
    /// Add a section descriptor
    pub fn add_section_descriptor(&mut self, descriptor: DwgSectionDescriptor) {
        self.descriptors.insert(descriptor.name.clone(), descriptor);
    }
}

impl Default for DwgFileHeaderAC18 {
    fn default() -> Self {
        Self::new(ACadVersion::AC1018)
    }
}

impl DwgFileHeader for DwgFileHeaderAC18 {
    fn version(&self) -> ACadVersion {
        self.base.acad_version
    }
    
    fn preview_address(&self) -> i64 {
        self.base.preview_address
    }
    
    fn set_preview_address(&mut self, addr: i64) {
        self.base.preview_address = addr;
    }
    
    fn maintenance_version(&self) -> i32 {
        self.base.maintenance_version
    }
    
    fn set_maintenance_version(&mut self, version: i32) {
        self.base.maintenance_version = version;
    }
    
    fn code_page(&self) -> CodePage {
        self.base.code_page
    }
    
    fn set_code_page(&mut self, code_page: CodePage) {
        self.base.code_page = code_page;
    }
    
    fn add_section(&mut self, name: &str) {
        self.descriptors.insert(name.to_string(), DwgSectionDescriptor::with_name(name));
    }
    
    fn get_descriptor(&self, name: &str) -> Option<&DwgSectionDescriptor> {
        self.descriptors.get(name)
    }
    
    fn get_descriptor_mut(&mut self, name: &str) -> Option<&mut DwgSectionDescriptor> {
        self.descriptors.get_mut(name)
    }
}

/// File header for AC1021+ (2007+)
#[derive(Debug, Clone)]
pub struct DwgFileHeaderAC21 {
    /// Base AC18 header fields
    pub base: DwgFileHeaderAC18,
    /// Compressed metadata
    pub compressed_metadata: Option<Dwg21CompressedMetadata>,
}

impl DwgFileHeaderAC21 {
    /// Create a new AC21 file header
    pub fn new(version: ACadVersion) -> Self {
        Self {
            base: DwgFileHeaderAC18::new(version),
            compressed_metadata: None,
        }
    }
}

impl Default for DwgFileHeaderAC21 {
    fn default() -> Self {
        Self::new(ACadVersion::AC1021)
    }
}

impl DwgFileHeader for DwgFileHeaderAC21 {
    fn version(&self) -> ACadVersion {
        self.base.version()
    }
    
    fn preview_address(&self) -> i64 {
        self.base.preview_address()
    }
    
    fn set_preview_address(&mut self, addr: i64) {
        self.base.set_preview_address(addr);
    }
    
    fn maintenance_version(&self) -> i32 {
        self.base.maintenance_version()
    }
    
    fn set_maintenance_version(&mut self, version: i32) {
        self.base.set_maintenance_version(version);
    }
    
    fn code_page(&self) -> CodePage {
        self.base.code_page()
    }
    
    fn set_code_page(&mut self, code_page: CodePage) {
        self.base.set_code_page(code_page);
    }
    
    fn add_section(&mut self, name: &str) {
        self.base.add_section(name);
    }
    
    fn get_descriptor(&self, name: &str) -> Option<&DwgSectionDescriptor> {
        self.base.get_descriptor(name)
    }
    
    fn get_descriptor_mut(&mut self, name: &str) -> Option<&mut DwgSectionDescriptor> {
        self.base.get_descriptor_mut(name)
    }
}

/// Enum wrapper for all file header types
#[derive(Debug, Clone)]
pub enum DwgFileHeaderType {
    /// AC15 file header (R13-2002)
    AC15(DwgFileHeaderAC15),
    /// AC18 file header (2004-2006)
    AC18(DwgFileHeaderAC18),
    /// AC21 file header (2007+)
    AC21(DwgFileHeaderAC21),
}

impl DwgFileHeaderType {
    /// Create the appropriate file header for the given version
    pub fn create_for_version(version: ACadVersion) -> Result<Self> {
        match version {
            ACadVersion::AC1012 | ACadVersion::AC1014 | ACadVersion::AC1015 => {
                Ok(DwgFileHeaderType::AC15(DwgFileHeaderAC15::new(version)))
            }
            ACadVersion::AC1018 => {
                Ok(DwgFileHeaderType::AC18(DwgFileHeaderAC18::new(version)))
            }
            ACadVersion::AC1021 => {
                Ok(DwgFileHeaderType::AC21(DwgFileHeaderAC21::new(version)))
            }
            ACadVersion::AC1024 | ACadVersion::AC1027 | ACadVersion::AC1032 => {
                // 2010+ uses AC18 format with some extensions
                Ok(DwgFileHeaderType::AC18(DwgFileHeaderAC18::new(version)))
            }
            _ => Err(DxfError::UnsupportedVersion(format!("{:?}", version))),
        }
    }
    
    /// Get the AutoCAD version
    pub fn version(&self) -> ACadVersion {
        match self {
            DwgFileHeaderType::AC15(h) => h.version(),
            DwgFileHeaderType::AC18(h) => h.version(),
            DwgFileHeaderType::AC21(h) => h.version(),
        }
    }
    
    /// Get the preview address
    pub fn preview_address(&self) -> i64 {
        match self {
            DwgFileHeaderType::AC15(h) => h.preview_address(),
            DwgFileHeaderType::AC18(h) => h.preview_address(),
            DwgFileHeaderType::AC21(h) => h.preview_address(),
        }
    }
    
    /// Get the maintenance version
    pub fn maintenance_version(&self) -> i32 {
        match self {
            DwgFileHeaderType::AC15(h) => h.maintenance_version(),
            DwgFileHeaderType::AC18(h) => h.maintenance_version(),
            DwgFileHeaderType::AC21(h) => h.maintenance_version(),
        }
    }
    
    /// Get the code page
    pub fn code_page(&self) -> CodePage {
        match self {
            DwgFileHeaderType::AC15(h) => h.code_page(),
            DwgFileHeaderType::AC18(h) => h.code_page(),
            DwgFileHeaderType::AC21(h) => h.code_page(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_create_header_ac15() {
        let header = DwgFileHeaderType::create_for_version(ACadVersion::AC1015).unwrap();
        assert_eq!(header.version(), ACadVersion::AC1015);
        
        if let DwgFileHeaderType::AC15(_) = header {
            // OK
        } else {
            panic!("Expected AC15 header");
        }
    }
    
    #[test]
    fn test_create_header_ac18() {
        let header = DwgFileHeaderType::create_for_version(ACadVersion::AC1018).unwrap();
        assert_eq!(header.version(), ACadVersion::AC1018);
        
        if let DwgFileHeaderType::AC18(_) = header {
            // OK
        } else {
            panic!("Expected AC18 header");
        }
    }
    
    #[test]
    fn test_create_header_ac21() {
        let header = DwgFileHeaderType::create_for_version(ACadVersion::AC1021).unwrap();
        assert_eq!(header.version(), ACadVersion::AC1021);
        
        if let DwgFileHeaderType::AC21(_) = header {
            // OK
        } else {
            panic!("Expected AC21 header");
        }
    }
    
    #[test]
    fn test_unsupported_version() {
        let result = DwgFileHeaderType::create_for_version(ACadVersion::AC1009);
        assert!(result.is_err());
    }
}
