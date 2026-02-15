use std::collections::BTreeMap;

use crate::error::Result;

use super::idwg_stream_reader::DwgStreamReader;

/// Reads SUMMARYINFO dictionary.
pub struct DwgSummaryInfoReader;

impl DwgSummaryInfoReader {
    pub fn read(reader: &mut dyn DwgStreamReader) -> Result<BTreeMap<String, String>> {
        let count = reader.read_bit_long()?.max(0) as usize;
        let mut info = BTreeMap::new();
        for _ in 0..count {
            let key = reader.read_variable_text()?;
            let value = reader.read_variable_text()?;
            info.insert(key, value);
        }
        Ok(info)
    }
}
