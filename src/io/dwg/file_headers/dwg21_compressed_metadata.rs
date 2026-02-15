//! Compressed metadata for AC21 (2007) DWG file headers.

/// Metadata block stored in the compressed portion of AC21 file headers.
///
/// All fields are 64-bit unsigned integers matching the on-disk layout.
#[derive(Debug, Clone)]
pub struct Dwg21CompressedMetadata {
    pub header_size: u64,
    pub file_size: u64,
    pub pages_map_crc_compressed: u64,
    pub pages_map_correction_factor: u64,
    pub pages_map_crc_seed: u64,
    pub map2_offset: u64,
    pub map2_id: u64,
    pub pages_map_offset: u64,
    pub header2_offset: u64,
    pub pages_map_size_compressed: u64,
    pub pages_map_size_uncompressed: u64,
    pub pages_amount: u64,
    pub pages_max_id: u64,
    pub sections_map2_id: u64,
    pub pages_map_id: u64,
    pub unknown_0x20: u64,
    pub unknown_0x40: u64,
    pub pages_map_crc_uncompressed: u64,
    pub unknown_0xf800: u64,
    pub unknown_4: u64,
    pub unknown_1: u64,
    pub sections_amount: u64,
    pub sections_map_crc_uncompressed: u64,
    pub sections_map_size_compressed: u64,
    pub sections_map_id: u64,
    pub sections_map_size_uncompressed: u64,
    pub sections_map_crc_compressed: u64,
    pub sections_map_correction_factor: u64,
    pub sections_map_crc_seed: u64,
    pub stream_version: u64,
    pub crc_seed: u64,
    pub crc_seed_encoded: u64,
    pub random_seed: u64,
    pub header_crc64: u64,
}

impl Default for Dwg21CompressedMetadata {
    fn default() -> Self {
        Self {
            header_size: 0x70,
            file_size: 0,
            pages_map_crc_compressed: 0,
            pages_map_correction_factor: 0,
            pages_map_crc_seed: 0,
            map2_offset: 0,
            map2_id: 0,
            pages_map_offset: 0,
            header2_offset: 0,
            pages_map_size_compressed: 0,
            pages_map_size_uncompressed: 0,
            pages_amount: 0,
            pages_max_id: 0,
            sections_map2_id: 0,
            pages_map_id: 0,
            unknown_0x20: 32,
            unknown_0x40: 64,
            pages_map_crc_uncompressed: 0,
            unknown_0xf800: 0xF800,
            unknown_4: 4,
            unknown_1: 1,
            sections_amount: 0,
            sections_map_crc_uncompressed: 0,
            sections_map_size_compressed: 0,
            sections_map_id: 0,
            sections_map_size_uncompressed: 0,
            sections_map_crc_compressed: 0,
            sections_map_correction_factor: 0,
            sections_map_crc_seed: 0,
            stream_version: 393472,
            crc_seed: 0,
            crc_seed_encoded: 0,
            random_seed: 0,
            header_crc64: 0,
        }
    }
}

impl Dwg21CompressedMetadata {
    /// Create a new metadata block with default constant values.
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_constants() {
        let meta = Dwg21CompressedMetadata::new();
        assert_eq!(meta.header_size, 0x70);
        assert_eq!(meta.unknown_0x20, 32);
        assert_eq!(meta.unknown_0x40, 64);
        assert_eq!(meta.unknown_0xf800, 0xF800);
        assert_eq!(meta.unknown_4, 4);
        assert_eq!(meta.unknown_1, 1);
        assert_eq!(meta.stream_version, 393472);
    }

    #[test]
    fn test_mutable_fields() {
        let mut meta = Dwg21CompressedMetadata::new();
        meta.file_size = 123456;
        meta.pages_amount = 10;
        meta.sections_amount = 5;
        assert_eq!(meta.file_size, 123456);
        assert_eq!(meta.pages_amount, 10);
        assert_eq!(meta.sections_amount, 5);
    }
}
