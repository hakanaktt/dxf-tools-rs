//! DWG section descriptor for AC18+ page-based sections.

use super::DwgLocalSectionMap;

/// Describes a named section in an AC18+ DWG file.
///
/// Each descriptor tracks a section's compression settings,
/// sizes, page count, and the list of local section pages that
/// compose it.
#[derive(Debug, Clone)]
pub struct DwgSectionDescriptor {
    /// Magic page-type marker (constant `0x4163043B`).
    pub page_type: i64,
    /// Section name (e.g., `"AcDb:Header"`).
    pub name: String,
    /// Total compressed size across all pages.
    pub compressed_size: u64,
    /// Number of pages for this section.
    pub page_count: i32,
    /// Total decompressed size (default `0x7400`).
    pub decompressed_size: u64,
    /// Compression code: 1 = uncompressed, 2 = compressed.
    ///
    /// Only used for AC1018 and AC1024 or above.
    compressed_code: i32,
    /// Section id.
    pub section_id: i32,
    /// Encryption flag.
    pub encrypted: i32,
    /// Optional hash code.
    pub hash_code: Option<u64>,
    /// Optional encoding value.
    pub encoding: Option<u64>,
    /// Local section pages that belong to this section.
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
    /// Create a new section descriptor with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new section descriptor with a given name.
    pub fn with_name(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Get the compression code (1 = uncompressed, 2 = compressed).
    pub fn compressed_code(&self) -> i32 {
        self.compressed_code
    }

    /// Set the compression code.
    ///
    /// # Panics
    ///
    /// Panics if `value` is not 1 or 2.
    pub fn set_compressed_code(&mut self, value: i32) {
        assert!(
            value == 1 || value == 2,
            "CompressedCode must be 1 (uncompressed) or 2 (compressed), got {}",
            value
        );
        self.compressed_code = value;
    }

    /// Returns `true` if this section uses compression (code == 2).
    pub fn is_compressed(&self) -> bool {
        self.compressed_code == 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let desc = DwgSectionDescriptor::new();
        assert_eq!(desc.page_type, 0x4163043B);
        assert!(desc.name.is_empty());
        assert_eq!(desc.decompressed_size, 0x7400);
        assert_eq!(desc.compressed_code(), 2);
        assert!(desc.is_compressed());
    }

    #[test]
    fn test_with_name() {
        let desc = DwgSectionDescriptor::with_name("AcDb:Header");
        assert_eq!(desc.name, "AcDb:Header");
    }

    #[test]
    fn test_set_compressed_code_valid() {
        let mut desc = DwgSectionDescriptor::new();
        desc.set_compressed_code(1);
        assert_eq!(desc.compressed_code(), 1);
        assert!(!desc.is_compressed());

        desc.set_compressed_code(2);
        assert_eq!(desc.compressed_code(), 2);
        assert!(desc.is_compressed());
    }

    #[test]
    #[should_panic(expected = "CompressedCode must be 1 (uncompressed) or 2 (compressed)")]
    fn test_set_compressed_code_invalid() {
        let mut desc = DwgSectionDescriptor::new();
        desc.set_compressed_code(3);
    }

    #[test]
    fn test_local_sections() {
        let mut desc = DwgSectionDescriptor::with_name("AcDb:Classes");
        desc.local_sections.push(DwgLocalSectionMap::new());
        assert_eq!(desc.local_sections.len(), 1);
    }
}
