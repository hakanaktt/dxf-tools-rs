//! DWG section definitions — well-known section names. sentinels, and locator mappings.

use std::collections::HashMap;
use once_cell::sync::Lazy;

/// Well-known DWG section names (AC18+ string-based identifiers).
pub struct DwgSectionDefinition;

impl DwgSectionDefinition {
    pub const ACDB_OBJECTS: &'static str = "AcDb:AcDbObjects";
    pub const APP_INFO: &'static str = "AcDb:AppInfo";
    pub const AUX_HEADER: &'static str = "AcDb:AuxHeader";
    pub const HEADER: &'static str = "AcDb:Header";
    pub const CLASSES: &'static str = "AcDb:Classes";
    pub const HANDLES: &'static str = "AcDb:Handles";
    pub const OBJ_FREE_SPACE: &'static str = "AcDb:ObjFreeSpace";
    pub const TEMPLATE: &'static str = "AcDb:Template";
    pub const SUMMARY_INFO: &'static str = "AcDb:SummaryInfo";
    pub const FILE_DEP_LIST: &'static str = "AcDb:FileDepList";
    pub const PREVIEW: &'static str = "AcDb:Preview";
    pub const REV_HISTORY: &'static str = "AcDb:RevHistory";

    /// Map a section name to an AC15 record locator index.
    ///
    /// Returns `None` for sections that have no locator in AC15.
    pub fn get_section_locator_by_name(name: &str) -> Option<i32> {
        match name {
            Self::HEADER => Some(0),
            Self::CLASSES => Some(1),
            Self::HANDLES => Some(2),
            Self::OBJ_FREE_SPACE => Some(3),
            Self::TEMPLATE => Some(4),
            Self::AUX_HEADER => Some(5),
            _ => None,
        }
    }
}

/// Start sentinels keyed by section name.
pub static START_SENTINELS: Lazy<HashMap<&'static str, [u8; 16]>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert(
        DwgSectionDefinition::HEADER,
        [
            0xCF, 0x7B, 0x1F, 0x23, 0xFD, 0xDE, 0x38, 0xA9, 0x5F, 0x7C, 0x68, 0xB8, 0x4E, 0x6D,
            0x33, 0x5F,
        ],
    );
    m.insert(
        DwgSectionDefinition::CLASSES,
        [
            0x8D, 0xA1, 0xC4, 0xB8, 0xC4, 0xA9, 0xF8, 0xC5, 0xC0, 0xDC, 0xF4, 0x5F, 0xE7, 0xCF,
            0xB6, 0x8A,
        ],
    );
    m.insert(
        DwgSectionDefinition::PREVIEW,
        [
            0x1F, 0x25, 0x6D, 0x07, 0xD4, 0x36, 0x28, 0x28, 0x9D, 0x57, 0xCA, 0x3F, 0x9D, 0x44,
            0x10, 0x2B,
        ],
    );
    m
});

/// End sentinels keyed by section name.
pub static END_SENTINELS: Lazy<HashMap<&'static str, [u8; 16]>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert(
        DwgSectionDefinition::HEADER,
        [
            0x30, 0x84, 0xE0, 0xDC, 0x02, 0x21, 0xC7, 0x56, 0xA0, 0x83, 0x97, 0x47, 0xB1, 0x92,
            0xCC, 0xA0,
        ],
    );
    m.insert(
        DwgSectionDefinition::CLASSES,
        [
            0x72, 0x5E, 0x3B, 0x47, 0x3B, 0x56, 0x07, 0x3A, 0x3F, 0x23, 0x0B, 0xA0, 0x18, 0x30,
            0x49, 0x75,
        ],
    );
    m.insert(
        DwgSectionDefinition::PREVIEW,
        [
            0xE0, 0xDA, 0x92, 0xF8, 0x2B, 0xC9, 0xD7, 0xD7, 0x62, 0xA8, 0x35, 0xC0, 0x62, 0xBB,
            0xEF, 0xD4,
        ],
    );
    m
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_section_locator_by_name() {
        assert_eq!(
            DwgSectionDefinition::get_section_locator_by_name(DwgSectionDefinition::HEADER),
            Some(0)
        );
        assert_eq!(
            DwgSectionDefinition::get_section_locator_by_name(DwgSectionDefinition::CLASSES),
            Some(1)
        );
        assert_eq!(
            DwgSectionDefinition::get_section_locator_by_name(DwgSectionDefinition::HANDLES),
            Some(2)
        );
        assert_eq!(
            DwgSectionDefinition::get_section_locator_by_name(DwgSectionDefinition::OBJ_FREE_SPACE),
            Some(3)
        );
        assert_eq!(
            DwgSectionDefinition::get_section_locator_by_name(DwgSectionDefinition::TEMPLATE),
            Some(4)
        );
        assert_eq!(
            DwgSectionDefinition::get_section_locator_by_name(DwgSectionDefinition::AUX_HEADER),
            Some(5)
        );
        assert_eq!(
            DwgSectionDefinition::get_section_locator_by_name("UnknownSection"),
            None
        );
    }

    #[test]
    fn test_start_sentinels_exist() {
        assert!(START_SENTINELS.contains_key(DwgSectionDefinition::HEADER));
        assert!(START_SENTINELS.contains_key(DwgSectionDefinition::CLASSES));
        assert!(START_SENTINELS.contains_key(DwgSectionDefinition::PREVIEW));
        assert_eq!(START_SENTINELS.len(), 3);
    }

    #[test]
    fn test_end_sentinels_exist() {
        assert!(END_SENTINELS.contains_key(DwgSectionDefinition::HEADER));
        assert!(END_SENTINELS.contains_key(DwgSectionDefinition::CLASSES));
        assert!(END_SENTINELS.contains_key(DwgSectionDefinition::PREVIEW));
        assert_eq!(END_SENTINELS.len(), 3);
    }

    #[test]
    fn test_sentinel_values() {
        let header_start = START_SENTINELS.get(DwgSectionDefinition::HEADER).unwrap();
        assert_eq!(header_start[0], 0xCF);
        assert_eq!(header_start[15], 0x5F);

        let header_end = END_SENTINELS.get(DwgSectionDefinition::HEADER).unwrap();
        assert_eq!(header_end[0], 0x30);
        assert_eq!(header_end[15], 0xA0);
    }
}
