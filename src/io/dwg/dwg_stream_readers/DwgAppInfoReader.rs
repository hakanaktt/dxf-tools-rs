use crate::error::Result;
use crate::types::DxfVersion;

use super::idwg_stream_reader::DwgStreamReader;

/// Application info read from a DWG file.
#[derive(Debug, Clone, Default)]
pub struct DwgAppInfo {
    pub info_name: String,
    pub version: String,
    pub comment: String,
    pub product_xml: String,
    pub version_checksum: Vec<u8>,
    pub comment_checksum: Vec<u8>,
    pub product_checksum: Vec<u8>,
}

/// Reads DWG application information block.
/// Matches the C# DwgAppInfoReader implementation.
pub struct DwgAppInfoReader;

impl DwgAppInfoReader {
    /// Read the AppInfo section.
    ///
    /// - Pre-R2007: uses `readR18` path with variable text strings.
    /// - R2007+: uses `ReadTextUnicode` with checksums and optional product info.
    pub fn read(reader: &mut dyn DwgStreamReader, version: DxfVersion) -> Result<DwgAppInfo> {
        if version < DxfVersion::AC1021 {
            return Self::read_r18(reader);
        }

        let mut info = DwgAppInfo::default();

        // UInt32: Unknown (ODA writes 2)
        let _unknown1 = reader.read_int()?;

        // String: App info name, ODA writes "AppInfoDataList"
        info.info_name = reader.read_text_unicode()?;

        // UInt32: Unknown (ODA writes 3)
        let _unknown2 = reader.read_int()?;

        // Byte[16]: Version data (checksum, ODA writes zeroes)
        info.version_checksum = reader.read_bytes(16)?;

        // String: Version
        info.version = reader.read_text_unicode()?;

        // Byte[16]: Comment data (checksum, ODA writes zeroes)
        info.comment_checksum = reader.read_bytes(16)?;

        if version < DxfVersion::AC1024 {
            return Ok(info);
        }

        // R2010+ fields:
        // String: Comment
        info.comment = reader.read_text_unicode()?;

        // Byte[16]: Product data (checksum, ODA writes zeroes)
        info.product_checksum = reader.read_bytes(16)?;

        // String: Product XML
        info.product_xml = reader.read_text_unicode()?;

        Ok(info)
    }

    /// Read the R18 (pre-R2007) AppInfo format.
    /// For this version the field order differs from the documentation.
    fn read_r18(reader: &mut dyn DwgStreamReader) -> Result<DwgAppInfo> {
        let mut info = DwgAppInfo::default();

        // String: App info name
        info.info_name = reader.read_variable_text()?;

        // UInt32: Unknown (ODA writes 2)
        let _unknown2 = reader.read_int()?;

        // String: Version (ODA writes "4001")
        info.version = reader.read_variable_text()?;

        // String: Product XML element
        info.product_xml = reader.read_variable_text()?;

        // String: Comment / app info version (e.g. "2.7.2.0")
        info.comment = reader.read_variable_text()?;

        Ok(info)
    }
}
