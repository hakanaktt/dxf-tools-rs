//! LZ77 Compressor for DWG AC18 format
//!
//! Counterpart to the decompressor in `decompressor.rs`.
//! Uses a hash table for match finding with DWG-specific opcode encoding.

/// LZ77 compressor for AC18 (R2004) DWG section data
pub struct Lz77AC18Compressor {
    block: Vec<i32>, // hash table, 32768 entries
}

impl Lz77AC18Compressor {
    pub fn new() -> Self {
        Self {
            block: vec![-1; 0x8000],
        }
    }

    /// Compress source data into DWG LZ77 AC18 format
    pub fn compress(&mut self, source: &[u8]) -> Vec<u8> {
        self.restart_block();

        let total_offset = source.len();
        let mut dest = Vec::with_capacity(source.len());
        let mut curr_offset: usize = 0; // start of unwritten literal data
        let mut curr_position: usize = 4; // current scan position (skip first 4 bytes)

        let mut compression_offset: usize = 0; // last match length
        let mut match_pos: usize = 0; // last match distance
        let mut last_match_pos: usize;
        let mut curr_match_offset: usize;

        while curr_position < total_offset.saturating_sub(0x13) {
            let result = self.compress_chunk(source, curr_position, 0, total_offset);
            curr_match_offset = result.0;
            last_match_pos = result.1;

            if curr_match_offset < 3 {
                curr_position += 1;
                continue;
            }

            let mask = curr_position - curr_offset;

            if compression_offset != 0 {
                self.apply_mask(&mut dest, source, match_pos, compression_offset, mask);
            }

            self.write_literal_length(&mut dest, source, curr_offset, mask);
            curr_position += curr_match_offset;
            curr_offset = curr_position;
            compression_offset = curr_match_offset;
            match_pos = last_match_pos;
        }

        let literal_length = total_offset - curr_offset;

        if compression_offset != 0 {
            self.apply_mask(&mut dest, source, match_pos, compression_offset, literal_length);
        }

        self.write_literal_length(&mut dest, source, curr_offset, literal_length);

        // Terminator: 0x11, 0x00, 0x00
        dest.push(0x11);
        dest.push(0x00);
        dest.push(0x00);

        dest
    }

    fn restart_block(&mut self) {
        for entry in self.block.iter_mut() {
            *entry = -1;
        }
    }

    /// Run-length encoding for excess lengths
    fn write_len(dest: &mut Vec<u8>, mut len: usize) {
        while len > 0xFF {
            len -= 0xFF;
            dest.push(0);
        }
        dest.push(len as u8);
    }

    /// Write opcode byte with optional run-length extension
    fn write_opcode(dest: &mut Vec<u8>, opcode: u8, compression_offset: usize, max_inline: usize) {
        if compression_offset <= max_inline {
            dest.push(opcode | (compression_offset - 2) as u8);
        } else {
            dest.push(opcode);
            Self::write_len(dest, compression_offset - max_inline);
        }
    }

    /// Write literal run (raw bytes preceded by length encoding)
    fn write_literal_length(&self, dest: &mut Vec<u8>, source: &[u8], offset: usize, length: usize) {
        if length == 0 {
            return;
        }

        if length > 3 {
            Self::write_opcode(dest, 0, length - 1, 0x11);
        }

        for i in 0..length {
            dest.push(source[offset + i]);
        }
    }

    /// Encode a match (back-reference) into opcode bytes
    fn apply_mask(
        &self,
        dest: &mut Vec<u8>,
        _source: &[u8],
        mut match_position: usize,
        compression_offset: usize,
        mask: usize,
    ) {
        let curr: u8;
        let next: u8;

        if compression_offset >= 0x0F || match_position > 0x400 {
            if match_position <= 0x4000 {
                match_position -= 1;
                // Long compression offset + 0x21
                Self::write_opcode(dest, 0x20, compression_offset, 0x21);
            } else {
                match_position -= 0x4000;
                // Extra-long offset with bits packed into opcode
                Self::write_opcode(
                    dest,
                    0x10 | ((match_position >> 11) & 8) as u8,
                    compression_offset,
                    0x09,
                );
            }

            let mut c = ((match_position & 0xFF) << 2) as u8;
            next = (match_position >> 6) as u8;

            if mask < 4 {
                c |= mask as u8;
            }
            curr = c;
        } else {
            match_position -= 1;
            let mut c = (((compression_offset + 1) << 4) | ((match_position & 0b11) << 2)) as u8;
            next = (match_position >> 2) as u8;

            if mask < 4 {
                c |= mask as u8;
            }
            curr = c;
        }

        dest.push(curr);
        dest.push(next);
    }

    /// Find a match at the current position using hash table lookup
    fn compress_chunk(
        &mut self,
        source: &[u8],
        curr_position: usize,
        _initial_offset: usize,
        total_offset: usize,
    ) -> (usize, usize) {
        let mut offset: usize = 0;

        // Hash computation from 4 bytes at current position
        let v1 = (source[curr_position + 3] as i32) << 6;
        let v2 = v1 ^ source[curr_position + 2] as i32;
        let v3 = (v2 << 5) ^ source[curr_position + 1] as i32;
        let v4 = (v3 << 5) ^ source[curr_position] as i32;
        let value_index = ((v4.wrapping_add(v4 >> 5)) & 0x7FFF) as usize;

        let value = self.block[value_index];
        let mut match_pos = curr_position as i64 - value as i64;

        if value >= 0 && match_pos <= 0xBFFF {
            let value_u = value as usize;
            // Secondary hash probe if first match fails on byte 3
            if match_pos > 0x400
                && source[curr_position + 3] != source[value_u + 3]
            {
                let value_index2 = (value_index & 0x7FF) ^ 0b100000000011111;
                let value2 = self.block[value_index2];
                let match_pos2 = curr_position as i64 - value2 as i64;

                if value2 < 0
                    || match_pos2 > 0xBFFF
                    || (match_pos2 > 0x400
                        && source[curr_position + 3] != source[value2 as usize + 3])
                {
                    self.block[value_index] = curr_position as i32;
                    return (0, 0);
                }

                match_pos = match_pos2;
                // Update secondary hash
                self.block[value_index2] = curr_position as i32;
                let v = value2 as usize;
                // Verify first 3 bytes match
                if source[curr_position] == source[v]
                    && source[curr_position + 1] == source[v + 1]
                    && source[curr_position + 2] == source[v + 2]
                {
                    offset = 3;
                    let mut index = v + 3;
                    let mut curr_off = curr_position + 3;
                    while curr_off < total_offset && source[index] == source[curr_off] {
                        index += 1;
                        curr_off += 1;
                        offset += 1;
                    }
                }
            } else {
                let v = value as usize;
                // Verify first 3 bytes match, then extend
                if source[curr_position] == source[v]
                    && source[curr_position + 1] == source[v + 1]
                    && source[curr_position + 2] == source[v + 2]
                {
                    offset = 3;
                    let mut index = v + 3;
                    let mut curr_off = curr_position + 3;
                    while curr_off < total_offset && source[index] == source[curr_off] {
                        index += 1;
                        curr_off += 1;
                        offset += 1;
                    }
                }
            }
        }

        self.block[value_index] = curr_position as i32;
        (offset, match_pos as usize)
    }
}

/// Calculate padding needed to align to 0x20-byte boundary
pub fn compression_padding(length: usize) -> usize {
    0x1F - (length + 0x20 - 1) % 0x20
}

/// Generate the magic sequence used for padding compressed sections
pub fn magic_sequence() -> [u8; 256] {
    let mut seq = [0u8; 256];
    let mut rand_seed: i32 = 1;
    for i in 0..256 {
        rand_seed = rand_seed.wrapping_mul(0x343FD).wrapping_add(0x269EC3);
        seq[i] = (rand_seed >> 0x10) as u8;
    }
    seq
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::dwg::decompressor::Lz77AC18Decompressor;

    #[test]
    fn test_roundtrip_simple() {
        // Create test data with some repetition (LZ77 needs patterns)
        let mut data = Vec::new();
        for i in 0..256 {
            data.push((i & 0xFF) as u8);
        }
        // Add repetitive pattern
        for _ in 0..10 {
            data.extend_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD, 0x11, 0x22, 0x33, 0x44]);
        }

        let mut compressor = Lz77AC18Compressor::new();
        let compressed = compressor.compress(&data);

        // Decompress and verify roundtrip
        let mut decompressed = vec![0u8; data.len()];
        Lz77AC18Decompressor::decompress(&compressed, &mut decompressed);

        assert_eq!(data, decompressed);
    }

    #[test]
    fn test_roundtrip_large() {
        let mut data = vec![0u8; 4096];
        // Fill with pseudo-random repeating patterns
        for i in 0..data.len() {
            data[i] = ((i * 7 + 13) % 256) as u8;
        }
        // Insert some exact repeated blocks
        let block: Vec<u8> = (0..64).collect();
        for offset in (512..3000).step_by(128) {
            data[offset..offset + 64].copy_from_slice(&block);
        }

        let mut compressor = Lz77AC18Compressor::new();
        let compressed = compressor.compress(&data);

        let mut decompressed = vec![0u8; data.len()];
        Lz77AC18Decompressor::decompress(&compressed, &mut decompressed);

        assert_eq!(data, decompressed);
    }

    #[test]
    fn test_magic_sequence() {
        let seq = magic_sequence();
        // First few values should be deterministic
        assert_eq!(seq[0], 0x29);
        assert_eq!(seq[1], 0x23);
        assert_eq!(seq[2], 0xbe);
    }
}
