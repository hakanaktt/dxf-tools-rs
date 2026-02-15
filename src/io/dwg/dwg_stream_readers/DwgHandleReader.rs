use crate::error::Result;

use super::idwg_stream_reader::DwgStreamReader;

/// Reads handle stream values.
pub struct DwgHandleReader;

impl DwgHandleReader {
    pub fn read_handle(reader: &mut dyn DwgStreamReader, owner: u64) -> Result<u64> {
        reader.handle_reference_from(owner)
    }

    pub fn read_handles(reader: &mut dyn DwgStreamReader, owner: u64, count: usize) -> Result<Vec<u64>> {
        let mut handles = Vec::with_capacity(count);
        for _ in 0..count {
            handles.push(reader.handle_reference_from(owner)?);
        }
        Ok(handles)
    }
}
