use std::io::{Read, Seek};

use crate::error::Result;

use super::dwg_stream_reader_base::DwgStreamReaderBase;

/// Merges object data streams into one bit-reader.
pub struct DwgMergedReader;

impl DwgMergedReader {
    pub fn create<R: Read + Seek + 'static>(stream: R) -> Result<DwgStreamReaderBase> {
        Ok(DwgStreamReaderBase::new(Box::new(stream)))
    }
}
