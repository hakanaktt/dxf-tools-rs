use crate::error::Result;

use super::idwg_stream_reader::DwgStreamReader;

/// Preview image type in DWG file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewType {
    Unknown = 0,
    Bmp = 2,
    Wmf = 3,
    Png = 6,
}

impl From<u8> for PreviewType {
    fn from(code: u8) -> Self {
        match code {
            2 => PreviewType::Bmp,
            3 => PreviewType::Wmf,
            6 => PreviewType::Png,
            _ => PreviewType::Unknown,
        }
    }
}

/// Preview image data from a DWG file.
#[derive(Debug, Clone)]
pub struct DwgPreview {
    /// Type of the preview image.
    pub code: PreviewType,
    /// Raw header data (typically 80 zero bytes).
    pub raw_header: Vec<u8>,
    /// Raw image data.
    pub raw_image: Vec<u8>,
}

pub const PREVIEW_START_SENTINEL: [u8; 16] = [
    0x1F, 0x25, 0x6D, 0x07, 0xD4, 0x36, 0x28, 0x28,
    0x9D, 0x57, 0xCA, 0x3F, 0x9D, 0x44, 0x10, 0x2B,
];

pub const PREVIEW_END_SENTINEL: [u8; 16] = [
    0xE0, 0xDA, 0x92, 0xF8, 0x2B, 0xC9, 0xD7, 0xD7,
    0x62, 0xA8, 0x35, 0xC0, 0x62, 0xBB, 0xEF, 0xD4,
];

/// Reads preview image payload from DWG file.
/// Matches the C# DwgPreviewReader implementation.
pub struct DwgPreviewReader;

impl DwgPreviewReader {
    /// Read the complete preview section.
    ///
    /// Reads start sentinel, overall size, image entries,
    /// header data, body data, and end sentinel.
    pub fn read(reader: &mut dyn DwgStreamReader) -> Result<DwgPreview> {
        // Read and validate start sentinel
        let _start_sentinel = reader.read_sentinel()?;

        // RL: overall size of image area
        let _overall_size = reader.read_raw_long()?;

        // RC: counter indicating what is present here
        let images_present = reader.read_raw_char()?;

        let mut _header_data_start: Option<i64> = None;
        let mut header_data_size: Option<i64> = None;
        let mut _start_of_image: Option<i64> = None;
        let mut size_image: Option<i64> = None;
        let mut preview_code = PreviewType::Unknown;

        for _ in 0..images_present {
            // RC: code indicating what follows
            let code = reader.read_raw_char()?;
            match code {
                1 => {
                    // Header data: start RL + size RL
                    _header_data_start = Some(reader.read_raw_long()?);
                    header_data_size = Some(reader.read_raw_long()?);
                }
                _ => {
                    preview_code = PreviewType::from(code);
                    _start_of_image = Some(reader.read_raw_long()?);
                    size_image = Some(reader.read_raw_long()?);
                }
            }
        }

        // Read header bytes
        let header = if let Some(size) = header_data_size {
            reader.read_bytes(size.max(0) as usize)?
        } else {
            Vec::new()
        };

        // Read image bytes
        let body = if let Some(size) = size_image {
            reader.read_bytes(size.max(0) as usize)?
        } else {
            Vec::new()
        };

        // Read and validate end sentinel
        let _end_sentinel = reader.read_sentinel()?;

        Ok(DwgPreview {
            code: preview_code,
            raw_header: header,
            raw_image: body,
        })
    }
}
