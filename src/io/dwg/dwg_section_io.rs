//! Base functionality for DWG section I/O.
//!
//! Ported from ACadSharp `DwgSectionIO.cs`.

use crate::notification::{Notification, NotificationType};
use crate::types::DxfVersion;

use super::dwg_stream_readers::DwgStreamReader;

/// Version-flag helper for DWG section readers/writers.
///
/// Each section reader typically inherits from this in ACadSharp.
/// In Rust we use composition: embed a `DwgSectionContext` in each section reader.
pub struct DwgSectionContext {
    /// The AutoCAD version being processed.
    pub version: DxfVersion,
    /// Section name (for diagnostics).
    pub section_name: String,
    /// Collected notifications.
    pub notifications: Vec<Notification>,

    // ── Version convenience flags ──
    /// R13–R14 only (`AC1012` or `AC1014`).
    pub r13_14_only: bool,
    /// R13–R15 only (`AC1012`..=`AC1015`).
    pub r13_15_only: bool,
    /// R2000+ (`>= AC1015`).
    pub r2000_plus: bool,
    /// Pre-2004 (`< AC1018`).
    pub r2004_pre: bool,
    /// Pre-2007 (`<= AC1021`).
    pub r2007_pre: bool,
    /// R2004+ (`>= AC1018`).
    pub r2004_plus: bool,
    /// R2007+ (`>= AC1021`).
    pub r2007_plus: bool,
    /// R2010+ (`>= AC1024`).
    pub r2010_plus: bool,
    /// R2013+ (`>= AC1027`).
    pub r2013_plus: bool,
    /// R2018+ (`>= AC1032`).
    pub r2018_plus: bool,
}

impl DwgSectionContext {
    /// Create a new section context for the given version and section name.
    pub fn new(version: DxfVersion, section_name: impl Into<String>) -> Self {
        Self {
            section_name: section_name.into(),
            notifications: Vec::new(),

            r13_14_only: version == DxfVersion::AC1014 || version == DxfVersion::AC1012,
            r13_15_only: version >= DxfVersion::AC1012 && version <= DxfVersion::AC1015,
            r2000_plus: version >= DxfVersion::AC1015,
            r2004_pre: version < DxfVersion::AC1018,
            r2007_pre: version <= DxfVersion::AC1021,
            r2004_plus: version >= DxfVersion::AC1018,
            r2007_plus: version >= DxfVersion::AC1021,
            r2010_plus: version >= DxfVersion::AC1024,
            r2013_plus: version >= DxfVersion::AC1027,
            r2018_plus: version >= DxfVersion::AC1032,

            version,
        }
    }

    /// Record a notification.
    pub fn notify(&mut self, message: impl Into<String>, notification_type: NotificationType) {
        self.notifications.push(Notification::new(notification_type, message));
    }
}

/// Check whether two sentinel byte arrays are identical.
pub fn check_sentinel(actual: &[u8], expected: &[u8]) -> bool {
    if actual.len() != expected.len() {
        return false;
    }
    actual.iter().zip(expected.iter()).all(|(a, b)| a == b)
}

/// Read and validate a 16-byte sentinel from a DWG stream reader.
///
/// Returns `true` if the sentinel matches, `false` otherwise.
/// A warning notification is recorded on mismatch.
pub fn check_sentinel_from_reader(
    reader: &mut dyn DwgStreamReader,
    expected: &[u8; 16],
    ctx: &mut DwgSectionContext,
) -> bool {
    match reader.read_sentinel() {
        Ok(actual) => {
            if !check_sentinel(&actual, expected) {
                ctx.notify(
                    format!("Invalid section sentinel found in {}", ctx.section_name),
                    NotificationType::Warning,
                );
                false
            } else {
                true
            }
        }
        Err(_) => {
            ctx.notify(
                format!("Failed to read sentinel in {}", ctx.section_name),
                NotificationType::Warning,
            );
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_sentinel_match() {
        let a = [1, 2, 3, 4];
        let b = [1, 2, 3, 4];
        assert!(check_sentinel(&a, &b));
    }

    #[test]
    fn test_check_sentinel_mismatch() {
        let a = [1, 2, 3, 4];
        let b = [1, 2, 3, 5];
        assert!(!check_sentinel(&a, &b));
    }

    #[test]
    fn test_check_sentinel_length_mismatch() {
        let a = [1, 2, 3];
        let b = [1, 2, 3, 4];
        assert!(!check_sentinel(&a, &b));
    }

    #[test]
    fn test_version_flags() {
        let ctx = DwgSectionContext::new(DxfVersion::AC1015, "Test");
        assert!(!ctx.r13_14_only);
        assert!(ctx.r13_15_only);
        assert!(ctx.r2000_plus);
        assert!(ctx.r2004_pre);
        assert!(!ctx.r2004_plus);

        let ctx2 = DwgSectionContext::new(DxfVersion::AC1032, "Test2");
        assert!(ctx2.r2018_plus);
        assert!(ctx2.r2013_plus);
        assert!(ctx2.r2010_plus);
        assert!(ctx2.r2007_plus);
        assert!(ctx2.r2004_plus);
        assert!(ctx2.r2000_plus);
        assert!(!ctx2.r2004_pre);
    }
}
