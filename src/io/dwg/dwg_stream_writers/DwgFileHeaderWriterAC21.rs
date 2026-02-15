//! AC21 (R2007) file header writer — extends AC18 with 0x480 header and AC21 compressor.
//!
//! Note: The C# original is incomplete (LZ77 AC21 compressor is not implemented).

use std::io::{Cursor, Write};

use crate::error::Result;
use crate::io::dwg::{DwgLocalSectionMap, DwgSectionDescriptor};
use crate::types::DxfVersion;

use super::dwg_file_header_writer_ac18::DwgFileHeaderWriterAc18;
use super::dwg_file_header_writer_base::write_magic_number;
use super::dwg_lz77_ac21_compressor::DwgLz77Ac21Compressor;
use super::idwg_stream_writer::{Compressor, DwgFileHeaderWriter};

const AC21_FILE_HEADER_SIZE: usize = 0x480;

pub struct DwgFileHeaderWriterAc21 {
    inner: DwgFileHeaderWriterAc18,
}

impl DwgFileHeaderWriterAc21 {
    pub fn new(
        version: DxfVersion,
        version_string: String,
        code_page: String,
        maintenance_version: i16,
    ) -> Self {
        Self {
            inner: DwgFileHeaderWriterAc18::new(
                version,
                version_string,
                code_page,
                maintenance_version,
            ),
        }
    }
}

impl DwgFileHeaderWriter for DwgFileHeaderWriterAc21 {
    fn handle_section_offset(&self) -> i32 {
        self.inner.handle_section_offset()
    }

    fn add_section(
        &mut self,
        name: &str,
        stream: Vec<u8>,
        is_compressed: bool,
        decomp_size: usize,
    ) {
        self.inner.add_section(name, stream, is_compressed, decomp_size);
    }

    fn write_file(&mut self) -> Result<()> {
        self.inner.write_file()
    }
}
