//! DWG local section map for page-based sections in AC18+ files.

/// Describes a local section (page) within a DWG file.
///
/// Used for page-based section storage in AC18 (2004) and later versions.
#[derive(Debug, Clone)]
pub struct DwgLocalSectionMap {
    /// Compression type: 1 = none, 2 = compressed.
    pub compression: i32,
    /// Whether this section map entry is empty.
    pub is_empty: bool,
    /// Offset of this page within the section data.
    pub offset: u64,
    /// Compressed size in bytes.
    pub compressed_size: u64,
    /// Page number.
    pub page_number: i32,
    /// Decompressed size in bytes.
    pub decompressed_size: u64,
    /// Seeker (absolute file position).
    pub seeker: i64,
    /// Size of the page in the file.
    pub size: i64,
    /// Checksum of this page.
    pub checksum: u64,
    /// CRC of this page.
    pub crc: u64,
    /// Page size.
    pub page_size: i64,
    /// ODA flag.
    pub oda: u32,
    /// Section map identifier.
    pub section_map: i32,
}

impl Default for DwgLocalSectionMap {
    fn default() -> Self {
        Self {
            compression: 2,
            is_empty: false,
            offset: 0,
            compressed_size: 0,
            page_number: 0,
            decompressed_size: 0,
            seeker: 0,
            size: 0,
            checksum: 0,
            crc: 0,
            page_size: 0,
            oda: 0,
            section_map: 0,
        }
    }
}

impl DwgLocalSectionMap {
    /// Create a new local section map with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a local section map with a given section map value.
    pub fn with_section_map(value: i32) -> Self {
        Self {
            section_map: value,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let map = DwgLocalSectionMap::new();
        assert_eq!(map.compression, 2);
        assert!(!map.is_empty);
        assert_eq!(map.section_map, 0);
    }

    #[test]
    fn test_with_section_map() {
        let map = DwgLocalSectionMap::with_section_map(42);
        assert_eq!(map.section_map, 42);
        assert_eq!(map.compression, 2);
    }
}
