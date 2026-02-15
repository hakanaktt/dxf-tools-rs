//! Configuration for reading DWG files.
//!
//! Ported from ACadSharp `DwgReaderConfiguration.cs`.

/// Configuration options for the DWG reader.
#[derive(Debug, Clone)]
pub struct DwgReaderConfiguration {
    /// Use the Standard Cyclic Redundancy Check to verify the integrity of the
    /// file. Default: `false`.
    ///
    /// The DWG file format uses a modification of a standard CRC as an
    /// error-detecting mechanism. Enabling this flag causes the reader to
    /// perform this verification, but it will greatly increase the reading time.
    pub crc_check: bool,

    /// If `false`, the reader will skip the summary info section.
    /// Default: `true`.
    pub read_summary_info: bool,

    /// When `true`, unknown/unrecognized entities are kept in the document
    /// as opaque blobs rather than being dropped.
    /// Default: `false`.
    pub keep_unknown_entities: bool,

    /// When `true`, unknown/unrecognized non-graphical objects are kept in the
    /// document as opaque blobs rather than being dropped.
    /// Default: `false`.
    pub keep_unknown_non_graphical_objects: bool,

    /// When `true`, parse errors within individual entities/objects/sections
    /// are caught and reported as notifications instead of aborting the read.
    /// Default: `false`.
    pub failsafe: bool,
}

impl Default for DwgReaderConfiguration {
    fn default() -> Self {
        Self {
            crc_check: false,
            read_summary_info: true,
            keep_unknown_entities: false,
            keep_unknown_non_graphical_objects: false,
            failsafe: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults() {
        let cfg = DwgReaderConfiguration::default();
        assert!(!cfg.crc_check);
        assert!(cfg.read_summary_info);
        assert!(!cfg.keep_unknown_entities);
        assert!(!cfg.keep_unknown_non_graphical_objects);
        assert!(!cfg.failsafe);
    }
}
