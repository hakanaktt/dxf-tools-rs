use std::collections::BTreeMap;

use crate::error::Result;
use crate::types::DxfVersion;

use super::idwg_stream_reader::DwgStreamReader;

/// Summary info data read from a DWG file.
#[derive(Debug, Clone, Default)]
pub struct CadSummaryInfo {
    pub title: String,
    pub subject: String,
    pub author: String,
    pub keywords: String,
    pub comments: String,
    pub last_saved_by: String,
    pub revision_number: String,
    pub hyperlink_base: String,
    /// (julian_date, milliseconds)
    pub created_date: (i32, i32),
    /// (julian_date, milliseconds)
    pub modified_date: (i32, i32),
    /// Custom key/value properties.
    pub properties: BTreeMap<String, String>,
}

/// Reads SUMMARYINFO section from a DWG file.
/// Matches the C# DwgSummaryInfoReader implementation.
pub struct DwgSummaryInfoReader;

impl DwgSummaryInfoReader {
    /// Read the complete summary info section.
    ///
    /// The `version` parameter determines the string reading method:
    /// - Pre-AC1021: uses `read_unicode_string` (short length + encoding-based string)
    /// - AC1021+: uses `read_text_unicode` (short length + unicode string)
    pub fn read(
        reader: &mut dyn DwgStreamReader,
        version: DxfVersion,
    ) -> Result<CadSummaryInfo> {
        let mut summary = CadSummaryInfo::default();

        let read_string = |r: &mut dyn DwgStreamReader| -> Result<String> {
            if version < DxfVersion::AC1021 {
                // Pre-R2007: short (length) + string bytes (Windows-1252)
                Self::read_pre2007_string(r)
            } else {
                r.read_text_unicode()
            }
        };

        // String fields in fixed order
        summary.title = read_string(reader)?;
        summary.subject = read_string(reader)?;
        summary.author = read_string(reader)?;
        summary.keywords = read_string(reader)?;
        summary.comments = read_string(reader)?;
        summary.last_saved_by = read_string(reader)?;
        summary.revision_number = read_string(reader)?;
        summary.hyperlink_base = read_string(reader)?;

        // Total editing time (ODA writes two zero Int32s)
        let _ = reader.read_int()?;
        let _ = reader.read_int()?;

        // Julian date: Create date time
        summary.created_date = reader.read_8_bit_julian_date()?;

        // Julian date: Modified date time
        summary.modified_date = reader.read_8_bit_julian_date()?;

        // Int16: Property count, followed by key/value string pairs
        let nproperties = reader.read_short()?.max(0) as usize;
        for _ in 0..nproperties {
            let prop_name = read_string(reader)?;
            let prop_value = read_string(reader)?;
            summary.properties.insert(prop_name, prop_value);
        }

        // Unknown Int32 x2 (ODA writes 0)
        let _ = reader.read_int();
        let _ = reader.read_int();

        Ok(summary)
    }

    /// Read a pre-R2007 unicode string: short (length) + string bytes.
    /// This matches the C# `readUnicodeString` private method.
    fn read_pre2007_string(reader: &mut dyn DwgStreamReader) -> Result<String> {
        let text_length = reader.read_short()?;
        if text_length <= 0 {
            return Ok(String::new());
        }
        let bytes = reader.read_bytes(text_length as usize)?;
        // Windows-1252 approximation: use lossy UTF-8
        Ok(String::from_utf8_lossy(&bytes).replace('\0', ""))
    }
}
