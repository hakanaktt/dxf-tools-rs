use crate::error::Result;

use super::idwg_stream_reader::DwgStreamReader;

/// Reads preview image payload from DWG file.
pub struct DwgPreviewReader;

impl DwgPreviewReader {
    pub fn read(reader: &mut dyn DwgStreamReader) -> Result<Vec<u8>> {
        let size = reader.read_bit_long()?.max(0) as usize;
        reader.read_bytes(size)
    }
}
