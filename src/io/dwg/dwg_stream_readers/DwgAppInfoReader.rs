use crate::error::Result;

use super::idwg_stream_reader::DwgStreamReader;

/// Reads DWG application information block.
pub struct DwgAppInfoReader;

impl DwgAppInfoReader {
    pub fn read(reader: &mut dyn DwgStreamReader) -> Result<Vec<(String, String)>> {
        let count = reader.read_bit_long()?.max(0) as usize;
        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            let key = reader.read_variable_text()?;
            let value = reader.read_variable_text()?;
            entries.push((key, value));
        }
        Ok(entries)
    }

    pub fn read_r18(reader: &mut dyn DwgStreamReader) -> Result<Vec<(String, String)>> {
        Self::read(reader)
    }
}
