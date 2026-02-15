//! LZ77 compressor for DWG AC21 (R2007) format.
//!
//! The C# original throws `NotImplementedException` — this is preserved here.

use crate::error::DxfError;
use super::idwg_stream_writer::Compressor;

pub struct DwgLz77Ac21Compressor;

impl DwgLz77Ac21Compressor {
    pub fn new() -> Self {
        Self
    }
}

impl Compressor for DwgLz77Ac21Compressor {
    fn compress(
        &mut self,
        _source: &[u8],
        _offset: usize,
        _total_size: usize,
        _dest: &mut Vec<u8>,
    ) {
        // The original C# implementation throws NotImplementedException.
        // AC21 (R2007) compression is not yet implemented.
        panic!("DwgLZ77AC21Compressor::compress is not implemented");
    }
}
