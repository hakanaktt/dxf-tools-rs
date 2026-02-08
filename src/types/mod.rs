//! Core types used throughout acadrust

pub mod bounds;
pub mod color;
pub mod handle;
pub mod line_weight;
pub mod transform;
pub mod transparency;
pub mod vector;

pub use bounds::{BoundingBox2D, BoundingBox3D};
pub use color::Color;
pub use handle::Handle;
pub use line_weight::LineWeight;
pub use transform::{Matrix3, Matrix4, Transform, rotate_point_2d, is_zero_angle};
pub use transparency::Transparency;
pub use vector::{Vector2, Vector3};

/// AutoCAD version enumeration (applies to both DWG and DXF formats)
///
/// The version numbers represent the internal AutoCAD format version codes.
/// These are used in file headers to identify the format version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i16)]
pub enum ACadVersion {
    /// Version not identified
    Unknown = -1,
    /// Release 1.1
    MC0_0 = 0,
    /// Release 1.2
    AC1_2 = 1,
    /// Release 1.4
    AC1_4 = 2,
    /// Release 2.0
    AC1_50 = 3,
    /// Release 2.10
    AC2_10 = 4,
    /// Release 2.5
    AC1002 = 5,
    /// Release 2.6
    AC1003 = 6,
    /// Release 9
    AC1004 = 7,
    /// Release 10
    AC1006 = 8,
    /// Release 11/12 (LT R1/R2)
    AC1009 = 9,
    /// Release 13 (LT95)
    AC1012 = 19,
    /// Release 14, 14.01 (LT97/LT98)
    AC1014 = 21,
    /// Release 2000/2000i/2002
    AC1015 = 23,
    /// Release 2004/2005/2006
    AC1018 = 25,
    /// Release 2007/2008/2009
    AC1021 = 27,
    /// Release 2010/2011/2012
    AC1024 = 29,
    /// Release 2013/2014/2015/2016/2017
    AC1027 = 31,
    /// Release 2018/2019/2020
    AC1032 = 33,
}

impl ACadVersion {
    /// Get the version string (e.g., "AC1015")
    pub fn as_str(&self) -> &'static str {
        match self {
            ACadVersion::Unknown => "UNKNOWN",
            ACadVersion::MC0_0 => "MC0.0",
            ACadVersion::AC1_2 => "AC1.2",
            ACadVersion::AC1_4 => "AC1.4",
            ACadVersion::AC1_50 => "AC1.50",
            ACadVersion::AC2_10 => "AC2.10",
            ACadVersion::AC1002 => "AC1002",
            ACadVersion::AC1003 => "AC1003",
            ACadVersion::AC1004 => "AC1004",
            ACadVersion::AC1006 => "AC1006",
            ACadVersion::AC1009 => "AC1009",
            ACadVersion::AC1012 => "AC1012",
            ACadVersion::AC1014 => "AC1014",
            ACadVersion::AC1015 => "AC1015",
            ACadVersion::AC1018 => "AC1018",
            ACadVersion::AC1021 => "AC1021",
            ACadVersion::AC1024 => "AC1024",
            ACadVersion::AC1027 => "AC1027",
            ACadVersion::AC1032 => "AC1032",
        }
    }

    /// Get the DXF/DWG version string for file headers
    pub fn to_dxf_string(&self) -> &'static str {
        self.as_str()
    }

    /// Parse version from string (e.g., "AC1015")
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "MC0.0" => Some(ACadVersion::MC0_0),
            "AC1.2" => Some(ACadVersion::AC1_2),
            "AC1.4" => Some(ACadVersion::AC1_4),
            "AC1.50" => Some(ACadVersion::AC1_50),
            "AC2.10" => Some(ACadVersion::AC2_10),
            "AC1002" => Some(ACadVersion::AC1002),
            "AC1003" => Some(ACadVersion::AC1003),
            "AC1004" => Some(ACadVersion::AC1004),
            "AC1006" => Some(ACadVersion::AC1006),
            "AC1009" => Some(ACadVersion::AC1009),
            "AC1012" => Some(ACadVersion::AC1012),
            "AC1014" => Some(ACadVersion::AC1014),
            "AC1015" => Some(ACadVersion::AC1015),
            "AC1018" => Some(ACadVersion::AC1018),
            "AC1021" => Some(ACadVersion::AC1021),
            "AC1024" => Some(ACadVersion::AC1024),
            "AC1027" => Some(ACadVersion::AC1027),
            "AC1032" => Some(ACadVersion::AC1032),
            _ => None,
        }
    }

    /// Parse version from version string (e.g., "AC1015")
    pub fn from_version_string(s: &str) -> Self {
        Self::parse(s).unwrap_or(ACadVersion::Unknown)
    }

    /// Get the numeric version code
    pub fn version_code(&self) -> u16 {
        match self {
            ACadVersion::Unknown => 0,
            ACadVersion::MC0_0 => 0,
            ACadVersion::AC1_2 => 102,
            ACadVersion::AC1_4 => 104,
            ACadVersion::AC1_50 => 150,
            ACadVersion::AC2_10 => 210,
            ACadVersion::AC1002 => 1002,
            ACadVersion::AC1003 => 1003,
            ACadVersion::AC1004 => 1004,
            ACadVersion::AC1006 => 1006,
            ACadVersion::AC1009 => 1009,
            ACadVersion::AC1012 => 1012,
            ACadVersion::AC1014 => 1014,
            ACadVersion::AC1015 => 1015,
            ACadVersion::AC1018 => 1018,
            ACadVersion::AC1021 => 1021,
            ACadVersion::AC1024 => 1024,
            ACadVersion::AC1027 => 1027,
            ACadVersion::AC1032 => 1032,
        }
    }

    /// Create version from numeric code
    pub fn from_version_code(code: u16) -> Self {
        match code {
            0 => ACadVersion::MC0_0,
            102 => ACadVersion::AC1_2,
            104 => ACadVersion::AC1_4,
            150 => ACadVersion::AC1_50,
            210 => ACadVersion::AC2_10,
            1002 => ACadVersion::AC1002,
            1003 => ACadVersion::AC1003,
            1004 => ACadVersion::AC1004,
            1006 => ACadVersion::AC1006,
            1009 => ACadVersion::AC1009,
            1012 => ACadVersion::AC1012,
            1014 => ACadVersion::AC1014,
            1015 => ACadVersion::AC1015,
            1018 => ACadVersion::AC1018,
            1021 => ACadVersion::AC1021,
            1024 => ACadVersion::AC1024,
            1027 => ACadVersion::AC1027,
            1032 => ACadVersion::AC1032,
            _ => ACadVersion::Unknown,
        }
    }

    /// Check if this version supports DWG reading (R13+)
    pub fn supports_dwg_read(&self) -> bool {
        matches!(
            self,
            ACadVersion::AC1012
                | ACadVersion::AC1014
                | ACadVersion::AC1015
                | ACadVersion::AC1018
                | ACadVersion::AC1021
                | ACadVersion::AC1024
                | ACadVersion::AC1027
                | ACadVersion::AC1032
        )
    }

    /// Check if this is a legacy version (before R13)
    pub fn is_legacy(&self) -> bool {
        (*self as i16) < (ACadVersion::AC1012 as i16) && *self != ACadVersion::Unknown
    }

    /// Get the minimum supported DWG version
    pub fn min_dwg_version() -> Self {
        ACadVersion::AC1012
    }
}

impl std::fmt::Display for ACadVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Default for ACadVersion {
    fn default() -> Self {
        ACadVersion::AC1032
    }
}

/// Type alias for backward compatibility
pub type DxfVersion = ACadVersion;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_string() {
        assert_eq!(ACadVersion::AC1015.as_str(), "AC1015");
        assert_eq!(ACadVersion::AC1032.to_string(), "AC1032");
    }

    #[test]
    fn test_version_parse() {
        assert_eq!(
            ACadVersion::parse("AC1018"),
            Some(ACadVersion::AC1018)
        );
        assert_eq!(ACadVersion::parse("INVALID"), None);
    }

    #[test]
    fn test_version_code() {
        assert_eq!(ACadVersion::AC1021.version_code(), 1021);
    }

    #[test]
    fn test_dwg_support() {
        assert!(ACadVersion::AC1015.supports_dwg_read());
        assert!(!ACadVersion::AC1009.supports_dwg_read());
    }

    #[test]
    fn test_legacy() {
        assert!(ACadVersion::AC1009.is_legacy());
        assert!(!ACadVersion::AC1015.is_legacy());
    }

    // Backward compatibility test with DxfVersion alias
    #[test]
    fn test_dxf_version_alias() {
        let v: DxfVersion = ACadVersion::AC1015;
        assert_eq!(v.as_str(), "AC1015");
    }
}


