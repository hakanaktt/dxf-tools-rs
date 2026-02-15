use std::collections::BTreeMap;

use crate::error::Result;

use super::idwg_stream_reader::DwgStreamReader;

/// Raw DWG header variable bag.
#[derive(Debug, Default, Clone)]
pub struct DwgHeaderData {
    pub vars: BTreeMap<String, String>,
}

/// Reads DWG HEADER section.
pub struct DwgHeaderReader;

impl DwgHeaderReader {
    pub fn read(reader: &mut dyn DwgStreamReader) -> Result<DwgHeaderData> {
        let count = reader.read_bit_long()?.max(0) as usize;
        let mut data = DwgHeaderData::default();
        for _ in 0..count {
            let name = reader.read_variable_text()?;
            let value = reader.read_variable_text()?;
            data.vars.insert(name, value);
        }
        Ok(data)
    }
}
