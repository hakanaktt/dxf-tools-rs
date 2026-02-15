//! Base file header writer — shared helpers for AC15 and AC18+ writers.

use std::io::{Cursor, Seek, SeekFrom, Write};

use crate::error::Result;
use crate::io::dwg::{calculate, compression_calculator, MAGIC_SEQUENCE};
use crate::types::DxfVersion;

use super::idwg_stream_writer::{Compressor, DwgFileHeaderWriter};

/// Apply XOR mask using stream position as key.
pub fn apply_mask(buffer: &mut [u8], offset: usize, length: usize, stream_position: i64) {
    let key = (0x4164536Bu32 ^ (stream_position as u32)).to_le_bytes();
    let mut idx = offset;
    let end = offset + length;
    while idx < end {
        for i in 0..4 {
            if idx + i < end {
                buffer[idx + i] ^= key[i];
            }
        }
        idx += 4;
    }
}

/// Check if a range of bytes are all zeroes.
pub fn check_empty_bytes(buffer: &[u8], offset: usize, count: usize) -> bool {
    for i in 0..count {
        if buffer[offset + i] != 0 {
            return false;
        }
    }
    true
}

/// Write magic number padding to align to 0x20 boundary.
pub fn write_magic_number(stream: &mut dyn Write, position: u64) {
    let magic = &*MAGIC_SEQUENCE;
    let count = (position % 0x20) as usize;
    for i in 0..count {
        let _ = stream.write_all(&[magic[i]]);
    }
}

/// Apply magic sequence XOR to a buffer in-place.
pub fn apply_magic_sequence(buffer: &mut [u8]) {
    let magic = &*MAGIC_SEQUENCE;
    for i in 0..buffer.len() {
        buffer[i] ^= magic[i % magic.len()];
    }
}

/// Get file code page index.
pub fn get_file_code_page(code_page: &str) -> u16 {
    // Simple mapping; default to 30 (ANSI_1252)
    match code_page {
        "ANSI_1252" => 30,
        "ANSI_1251" => 29,
        "ANSI_1250" => 28,
        "ANSI_932" => 31,
        "ANSI_949" => 33,
        "ANSI_950" => 34,
        "ANSI_936" => 35,
        _ => 30,
    }
}
