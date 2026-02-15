//! DWG section locator record for AC15 and earlier file headers.

use std::fmt;

/// A record describing the location and size of a section in the DWG file.
///
/// Used in AC15 (R2000) and earlier file header versions.
#[derive(Debug, Clone)]
pub struct DwgSectionLocatorRecord {
    /// Number of the record or id.
    pub number: Option<i32>,
    /// Offset where the record is in the file.
    pub seeker: i64,
    /// Size in bytes of this record.
    pub size: i64,
}

impl Default for DwgSectionLocatorRecord {
    fn default() -> Self {
        Self {
            number: None,
            seeker: 0,
            size: 0,
        }
    }
}

impl DwgSectionLocatorRecord {
    /// Create a new empty record.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a record with only a number.
    pub fn with_number(number: Option<i32>) -> Self {
        Self {
            number,
            ..Default::default()
        }
    }

    /// Create a record with number, seeker, and size.
    pub fn with_values(number: Option<i32>, seeker: i32, size: i32) -> Self {
        Self {
            number,
            seeker: seeker as i64,
            size: size as i64,
        }
    }

    /// Check if a position falls within this record.
    pub fn is_in_the_record(&self, position: i32) -> bool {
        let pos = position as i64;
        pos >= self.seeker && pos < self.seeker + self.size
    }
}

impl fmt::Display for DwgSectionLocatorRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Number : {:?} | Seeker : {} | Size : {}",
            self.number, self.seeker, self.size
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let rec = DwgSectionLocatorRecord::new();
        assert_eq!(rec.number, None);
        assert_eq!(rec.seeker, 0);
        assert_eq!(rec.size, 0);
    }

    #[test]
    fn test_with_number() {
        let rec = DwgSectionLocatorRecord::with_number(Some(3));
        assert_eq!(rec.number, Some(3));
    }

    #[test]
    fn test_with_values() {
        let rec = DwgSectionLocatorRecord::with_values(Some(1), 100, 200);
        assert_eq!(rec.number, Some(1));
        assert_eq!(rec.seeker, 100);
        assert_eq!(rec.size, 200);
    }

    #[test]
    fn test_is_in_the_record() {
        let rec = DwgSectionLocatorRecord::with_values(Some(0), 100, 50);
        assert!(!rec.is_in_the_record(99));
        assert!(rec.is_in_the_record(100));
        assert!(rec.is_in_the_record(125));
        assert!(rec.is_in_the_record(149));
        assert!(!rec.is_in_the_record(150));
    }

    #[test]
    fn test_display() {
        let rec = DwgSectionLocatorRecord::with_values(Some(2), 500, 100);
        let s = format!("{}", rec);
        assert!(s.contains("500"));
        assert!(s.contains("100"));
    }
}
