//! DWG section hash values used for AC21+ section identification.

/// Hash values for well-known DWG sections.
///
/// Used in AC21 (2007) and later to identify sections by hash.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum DwgSectionHash {
    AcDbUnknown = 0x0000_0000,
    AcDbSecurity = 0x4A02_04EA_u32 as i32,
    AcDbFileDepList = 0x6C42_05CA_u32 as i32,
    AcDbVbaProject = 0x586E_0544_u32 as i32,
    AcDbAppInfo = 0x3FA0_043E_u32 as i32,
    AcDbPreview = 0x40AA_0473_u32 as i32,
    AcDbSummaryInfo = 0x717A_060F_u32 as i32,
    AcDbRevHistory = 0x60A2_05B3_u32 as i32,
    AcDbAcDbObjects = 0x674C_05A9_u32 as i32,
    AcDbObjFreeSpace = 0x77E2_061F_u32 as i32,
    AcDbTemplate = 0x4A14_04CE_u32 as i32,
    AcDbHandles = 0x3F6E_0450_u32 as i32,
    AcDbClasses = 0x3F54_045F_u32 as i32,
    AcDbAuxHeader = 0x54F0_050A_u32 as i32,
    AcDbHeader = 0x32B8_03D9_u32 as i32,
    AcDbSignature = -1,
}

impl DwgSectionHash {
    /// Try to convert a raw `i32` value into a `DwgSectionHash`.
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0x0000_0000 => Some(Self::AcDbUnknown),
            x if x == 0x4A02_04EA_u32 as i32 => Some(Self::AcDbSecurity),
            x if x == 0x6C42_05CA_u32 as i32 => Some(Self::AcDbFileDepList),
            x if x == 0x586E_0544_u32 as i32 => Some(Self::AcDbVbaProject),
            x if x == 0x3FA0_043E_u32 as i32 => Some(Self::AcDbAppInfo),
            x if x == 0x40AA_0473_u32 as i32 => Some(Self::AcDbPreview),
            x if x == 0x717A_060F_u32 as i32 => Some(Self::AcDbSummaryInfo),
            x if x == 0x60A2_05B3_u32 as i32 => Some(Self::AcDbRevHistory),
            x if x == 0x674C_05A9_u32 as i32 => Some(Self::AcDbAcDbObjects),
            x if x == 0x77E2_061F_u32 as i32 => Some(Self::AcDbObjFreeSpace),
            x if x == 0x4A14_04CE_u32 as i32 => Some(Self::AcDbTemplate),
            x if x == 0x3F6E_0450_u32 as i32 => Some(Self::AcDbHandles),
            x if x == 0x3F54_045F_u32 as i32 => Some(Self::AcDbClasses),
            x if x == 0x54F0_050A_u32 as i32 => Some(Self::AcDbAuxHeader),
            x if x == 0x32B8_03D9_u32 as i32 => Some(Self::AcDbHeader),
            -1 => Some(Self::AcDbSignature),
            _ => None,
        }
    }

    /// Get the raw `i32` hash value.
    pub fn as_i32(self) -> i32 {
        self as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_hashes() {
        assert_eq!(DwgSectionHash::AcDbUnknown.as_i32(), 0);
        assert_eq!(DwgSectionHash::AcDbSignature.as_i32(), -1);
        assert_eq!(
            DwgSectionHash::AcDbHeader.as_i32(),
            0x32B8_03D9_u32 as i32
        );
    }

    #[test]
    fn test_from_i32_roundtrip() {
        let all = [
            DwgSectionHash::AcDbUnknown,
            DwgSectionHash::AcDbSecurity,
            DwgSectionHash::AcDbFileDepList,
            DwgSectionHash::AcDbVbaProject,
            DwgSectionHash::AcDbAppInfo,
            DwgSectionHash::AcDbPreview,
            DwgSectionHash::AcDbSummaryInfo,
            DwgSectionHash::AcDbRevHistory,
            DwgSectionHash::AcDbAcDbObjects,
            DwgSectionHash::AcDbObjFreeSpace,
            DwgSectionHash::AcDbTemplate,
            DwgSectionHash::AcDbHandles,
            DwgSectionHash::AcDbClasses,
            DwgSectionHash::AcDbAuxHeader,
            DwgSectionHash::AcDbHeader,
            DwgSectionHash::AcDbSignature,
        ];
        for hash in all {
            let raw = hash.as_i32();
            let back = DwgSectionHash::from_i32(raw).expect("roundtrip failed");
            assert_eq!(back, hash);
        }
    }

    #[test]
    fn test_from_i32_unknown_value() {
        assert!(DwgSectionHash::from_i32(0x12345678).is_none());
    }
}
