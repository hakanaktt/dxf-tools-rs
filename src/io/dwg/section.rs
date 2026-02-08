//! DWG Section definitions and descriptors
//!
//! DWG files are organized into sections, each containing specific types of data.
//! This module provides types for managing section definitions and locating
//! sections within the file.

/// Section name constants
pub mod section_names {
    pub const AC_DB_OBJECTS: &str = "AcDb:AcDbObjects";
    pub const APP_INFO: &str = "AcDb:AppInfo";
    pub const AUX_HEADER: &str = "AcDb:AuxHeader";
    pub const HEADER: &str = "AcDb:Header";
    pub const CLASSES: &str = "AcDb:Classes";
    pub const HANDLES: &str = "AcDb:Handles";
    pub const OBJ_FREE_SPACE: &str = "AcDb:ObjFreeSpace";
    pub const TEMPLATE: &str = "AcDb:Template";
    pub const SUMMARY_INFO: &str = "AcDb:SummaryInfo";
    pub const FILE_DEP_LIST: &str = "AcDb:FileDepList";
    pub const PREVIEW: &str = "AcDb:Preview";
    pub const REV_HISTORY: &str = "AcDb:RevHistory";
}

/// Start sentinels for various sections
pub struct DwgSectionDefinition;

impl DwgSectionDefinition {
    /// Get the section locator number by section name
    pub fn get_section_locator_by_name(name: &str) -> Option<i32> {
        match name {
            section_names::HEADER => Some(0),
            section_names::CLASSES => Some(1),
            section_names::HANDLES => Some(2),
            section_names::OBJ_FREE_SPACE => Some(3),
            section_names::TEMPLATE => Some(4),
            section_names::AUX_HEADER => Some(5),
            _ => None,
        }
    }
    
    /// Header section start sentinel
    pub const HEADER_START_SENTINEL: [u8; 16] = [
        0xCF, 0x7B, 0x1F, 0x23, 0xFD, 0xDE, 0x38, 0xA9,
        0x5F, 0x7C, 0x68, 0xB8, 0x4E, 0x6D, 0x33, 0x5F,
    ];
    
    /// Header section end sentinel
    pub const HEADER_END_SENTINEL: [u8; 16] = [
        0x30, 0x84, 0xE0, 0xDC, 0x02, 0x21, 0xC7, 0x56,
        0xA0, 0x83, 0x97, 0x47, 0xB1, 0x92, 0xCC, 0xA0,
    ];
    
    /// Classes section start sentinel
    pub const CLASSES_START_SENTINEL: [u8; 16] = [
        0x8D, 0xA1, 0xC4, 0xB8, 0xC4, 0xA9, 0xF8, 0xC5,
        0xC0, 0xDC, 0xF4, 0x5F, 0xE7, 0xCF, 0xB6, 0x8A,
    ];
    
    /// Classes section end sentinel
    pub const CLASSES_END_SENTINEL: [u8; 16] = [
        0x72, 0x5E, 0x3B, 0x47, 0x3B, 0x56, 0x07, 0x3A,
        0x3F, 0x23, 0x0B, 0xA0, 0x18, 0x30, 0x49, 0x75,
    ];
    
    /// Preview section start sentinel
    pub const PREVIEW_START_SENTINEL: [u8; 16] = [
        0x1F, 0x25, 0x6D, 0x07, 0xD4, 0x36, 0x28, 0x28,
        0x9D, 0x57, 0xCA, 0x3F, 0x9D, 0x44, 0x10, 0x2B,
    ];
    
    /// Preview section end sentinel
    pub const PREVIEW_END_SENTINEL: [u8; 16] = [
        0xE0, 0xDA, 0x92, 0xF8, 0x2B, 0xC9, 0xD7, 0xD7,
        0x62, 0xA8, 0x35, 0xC0, 0x62, 0xBB, 0xEF, 0xD4,
    ];
}

/// Record locating a section in the file
#[derive(Debug, Clone, Default)]
pub struct DwgSectionLocatorRecord {
    /// Number/ID of the record
    pub number: Option<i32>,
    /// Offset where the record is located
    pub seeker: i64,
    /// Size in bytes of this record
    pub size: i64,
}

impl DwgSectionLocatorRecord {
    /// Create a new empty record
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Create a new record with number
    pub fn with_number(number: i32) -> Self {
        Self {
            number: Some(number),
            ..Default::default()
        }
    }
    
    /// Create a new record with all fields
    pub fn with_values(number: Option<i32>, seeker: i64, size: i64) -> Self {
        Self { number, seeker, size }
    }
    
    /// Check if a position is within this record
    pub fn contains_position(&self, position: i64) -> bool {
        position >= self.seeker && position < self.seeker + self.size
    }
}

/// Descriptor for a section (used in AC1018+)
#[derive(Debug, Clone)]
pub struct DwgSectionDescriptor {
    /// Page type marker (0x4163043B)
    pub page_type: u64,
    /// Section name
    pub name: String,
    /// Compressed size
    pub compressed_size: u64,
    /// Number of pages
    pub page_count: i32,
    /// Decompressed size (default 0x7400)
    pub decompressed_size: u64,
    /// Compression code (1 = not compressed, 2 = compressed)
    pub compressed_code: i32,
    /// Section ID
    pub section_id: i32,
    /// Encryption flag
    pub encrypted: i32,
    /// Optional hash code
    pub hash_code: Option<u64>,
    /// Optional encoding
    pub encoding: Option<u64>,
    /// Local section maps
    pub local_sections: Vec<DwgLocalSectionMap>,
}

impl Default for DwgSectionDescriptor {
    fn default() -> Self {
        Self {
            page_type: 0x4163043B,
            name: String::new(),
            compressed_size: 0,
            page_count: 0,
            decompressed_size: 0x7400,
            compressed_code: 2,
            section_id: 0,
            encrypted: 0,
            hash_code: None,
            encoding: None,
            local_sections: Vec::new(),
        }
    }
}

impl DwgSectionDescriptor {
    /// Create a new section descriptor
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Create a new section descriptor with name
    pub fn with_name(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }
    
    /// Check if the section is compressed
    pub fn is_compressed(&self) -> bool {
        self.compressed_code == 2
    }
    
    /// Set compression code (validates value is 1 or 2)
    pub fn set_compressed_code(&mut self, code: i32) -> Result<(), String> {
        if code == 1 || code == 2 {
            self.compressed_code = code;
            Ok(())
        } else {
            Err(format!("Invalid compression code: {}", code))
        }
    }
}

/// Local section map entry (for paged sections)
#[derive(Debug, Clone, Default)]
pub struct DwgLocalSectionMap {
    /// Page number
    pub page_number: i32,
    /// Offset in the decompressed data
    pub offset: u64,
    /// Size of data in this page
    pub size: u64,
    /// Page size
    pub page_size: u64,
    /// Compressed page size
    pub compressed_size: u64,
    /// Checksum
    pub checksum: u32,
    /// CRC
    pub crc: u32,
}

impl DwgLocalSectionMap {
    /// Create a new local section map
    pub fn new() -> Self {
        Self::default()
    }
}

/// AC1021+ compressed metadata
#[derive(Debug, Clone, Default)]
pub struct Dwg21CompressedMetadata {
    /// Header size
    pub header_size: u64,
    /// File size
    pub file_size: u64,
    /// Pages map CRC compressed
    pub pages_map_crc_compressed: u64,
    /// Pages map correction
    pub pages_map_correction: u64,
    /// Pages map CRC seed
    pub pages_map_crc_seed: u64,
    /// Pages map 2 offset
    pub pages_map_2_offset: u64,
    /// Pages map 2 ID
    pub pages_map_2_id: u64,
    /// Pages map offset
    pub pages_map_offset: u64,
    /// Pages map ID
    pub pages_map_id: u64,
    /// Header 2 offset
    pub header_2_offset: u64,
    /// Pages map size compressed
    pub pages_map_size_compressed: u64,
    /// Pages map size uncompressed
    pub pages_map_size_uncompressed: u64,
    /// Pages amount
    pub pages_amount: u64,
    /// Pages max ID
    pub pages_max_id: u64,
    /// Sections map ID
    pub sections_map_id: u64,
    /// Sections map size uncompressed
    pub sections_map_size_uncompressed: u64,
    /// Sections map size compressed
    pub sections_map_size_compressed: u64,
    /// Sections map CRC uncompressed
    pub sections_map_crc_uncompressed: u64,
    /// Sections map CRC compressed
    pub sections_map_crc_compressed: u64,
    /// Sections map correction
    pub sections_map_correction: u64,
    /// Sections map CRC seed
    pub sections_map_crc_seed: u64,
    /// Stream version
    pub stream_version: u64,
    /// CRC seed
    pub crc_seed: u64,
    /// CRC seed encoded
    pub crc_seed_encoded: u64,
    /// Random seed
    pub random_seed: u64,
    /// Header CRC 64
    pub header_crc_64: u64,
}

impl Dwg21CompressedMetadata {
    /// Create a new compressed metadata
    pub fn new() -> Self {
        Self::default()
    }
}

/// Section hash for AC1021+
#[derive(Debug, Clone, Default)]
pub struct DwgSectionHash {
    /// Data section
    pub data_section: i64,
    /// Size
    pub size: u64,
    /// Page count
    pub page_count: u64,
    /// Max decompressed size
    pub max_decompressed_size: u64,
    /// Compressed
    pub compressed: u64,
    /// Section ID
    pub section_id: i64,
    /// Encrypted
    pub encrypted: u64,
    /// Section name
    pub name: String,
}

impl DwgSectionHash {
    /// Create a new section hash
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_section_locator() {
        assert_eq!(DwgSectionDefinition::get_section_locator_by_name(section_names::HEADER), Some(0));
        assert_eq!(DwgSectionDefinition::get_section_locator_by_name(section_names::CLASSES), Some(1));
        assert_eq!(DwgSectionDefinition::get_section_locator_by_name("invalid"), None);
    }
    
    #[test]
    fn test_section_locator_record() {
        let record = DwgSectionLocatorRecord::with_values(Some(0), 100, 50);
        
        assert!(record.contains_position(100));
        assert!(record.contains_position(149));
        assert!(!record.contains_position(150));
        assert!(!record.contains_position(99));
    }
    
    #[test]
    fn test_section_descriptor() {
        let mut desc = DwgSectionDescriptor::with_name("Test");
        
        assert_eq!(desc.name, "Test");
        assert!(desc.is_compressed());
        
        desc.set_compressed_code(1).unwrap();
        assert!(!desc.is_compressed());
        
        assert!(desc.set_compressed_code(3).is_err());
    }
}
