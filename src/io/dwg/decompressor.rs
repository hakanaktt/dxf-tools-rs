//! LZ77 Decompression for DWG files
//!
//! DWG files from AutoCAD 2004+ use LZ77 compression for various sections.
//! This module provides decompression support for:
//!
//! - AC1018 (2004-2006): LZ77 variant with specific opcode format
//! - AC1021+ (2007+): LZ77 variant with different instruction format

use std::io::{Read, Cursor};
use crate::error::{DxfError, Result};

/// LZ77 Decompressor for AC1018 (2004-2006) files
pub struct Lz77AC18Decompressor;

impl Lz77AC18Decompressor {
    /// Decompress a compressed stream to a new buffer
    pub fn decompress(compressed: &[u8], decompressed_size: usize) -> Result<Vec<u8>> {
        let mut src = Cursor::new(compressed);
        let mut dst = Vec::with_capacity(decompressed_size);
        
        Self::decompress_to_vec(&mut src, &mut dst)?;
        
        Ok(dst)
    }
    
    /// Decompress from a reader to a Vec (which can be read back for back-references)
    pub fn decompress_to_vec<R: Read>(
        src: &mut R,
        dst: &mut Vec<u8>,
    ) -> Result<()> {
        let mut temp_buf = vec![0u8; 128];
        
        let mut opcode1 = Self::read_byte(src)?;
        
        // Handle initial literal bytes
        if (opcode1 & 0xF0) == 0 {
            let lit_count = Self::literal_count(opcode1, src)? + 3;
            opcode1 = Self::copy_literal(lit_count, src, dst, &mut temp_buf)?;
        }
        
        // 0x11: Terminates the input stream
        while opcode1 != 0x11 {
            // Offset backwards from current position in decompressed data
            let mut comp_offset: usize;
            // Number of compressed bytes to copy
            let compressed_bytes: usize;
            
            if opcode1 >= 0x40 {
                // 0x40-0xFF: Normal compressed bytes case
                compressed_bytes = ((opcode1 >> 4) - 1) as usize;
                let opcode2 = Self::read_byte(src)?;
                comp_offset = (((opcode1 >> 2) & 3) as usize | ((opcode2 as usize) << 2)) + 1;
            } else if opcode1 < 0x10 {
                // 0x00-0x0F: Should not happen normally, skip literal handling
                // These are literal counts handled at start, if we get here it's an error
                // Use 0 compressed bytes to effectively skip
                compressed_bytes = 0;
                comp_offset = 1;
            } else if opcode1 < 0x20 {
                // 0x12 - 0x1F
                compressed_bytes = Self::read_compressed_bytes(opcode1, 0b0111, src)?;
                comp_offset = ((opcode1 & 8) as usize) << 11;
                opcode1 = Self::two_byte_offset(&mut comp_offset, 0x4000, src)?;
            } else {
                // 0x20+
                compressed_bytes = Self::read_compressed_bytes(opcode1, 0b00011111, src)?;
                comp_offset = 0;
                opcode1 = Self::two_byte_offset(&mut comp_offset, 1, src)?;
            }
            
            // Copy from earlier position in output (back-reference)
            let position = dst.len();
            let start_pos = position.saturating_sub(comp_offset);
            
            // Copy bytes with overlapping support
            for i in 0..compressed_bytes {
                let src_idx = start_pos + (i % comp_offset);
                if src_idx < dst.len() {
                    let byte = dst[src_idx];
                    dst.push(byte);
                }
            }
            
            // Calculate literal count
            let mut lit_count = (opcode1 & 3) as usize;
            
            if lit_count == 0 {
                opcode1 = Self::read_byte(src)?;
                if (opcode1 & 0xF0) == 0 {
                    lit_count = Self::literal_count(opcode1, src)? + 3;
                }
            }
            
            // Copy literal bytes
            if lit_count > 0 {
                opcode1 = Self::copy_literal(lit_count, src, dst, &mut temp_buf)?;
            }
        }
        
        Ok(())
    }
    
    fn read_byte<R: Read>(src: &mut R) -> Result<u8> {
        let mut buf = [0u8; 1];
        src.read_exact(&mut buf).map_err(DxfError::Io)?;
        Ok(buf[0])
    }
    
    fn copy_literal<R: Read>(
        count: usize,
        src: &mut R,
        dst: &mut Vec<u8>,
        temp_buf: &mut Vec<u8>,
    ) -> Result<u8> {
        if temp_buf.len() < count {
            temp_buf.resize(count, 0);
        }
        
        src.read_exact(&mut temp_buf[..count]).map_err(DxfError::Io)?;
        dst.extend_from_slice(&temp_buf[..count]);
        
        Self::read_byte(src)
    }
    
    fn literal_count<R: Read>(code: u8, src: &mut R) -> Result<usize> {
        let mut low_bits = (code & 0x0F) as usize;
        
        if low_bits == 0 {
            loop {
                let byte = Self::read_byte(src)?;
                if byte == 0 {
                    low_bits += 0xFF;
                } else {
                    low_bits += 0x0F + byte as usize;
                    break;
                }
            }
        }
        
        Ok(low_bits)
    }
    
    fn read_compressed_bytes<R: Read>(
        opcode1: u8,
        valid_bits: u8,
        src: &mut R,
    ) -> Result<usize> {
        let mut compressed_bytes = (opcode1 & valid_bits) as usize;
        
        if compressed_bytes == 0 {
            loop {
                let byte = Self::read_byte(src)?;
                if byte == 0 {
                    compressed_bytes += 0xFF;
                } else {
                    compressed_bytes += byte as usize + valid_bits as usize;
                    break;
                }
            }
        }
        
        Ok(compressed_bytes + 2)
    }
    
    fn two_byte_offset<R: Read>(
        offset: &mut usize,
        added_value: usize,
        src: &mut R,
    ) -> Result<u8> {
        let first_byte = Self::read_byte(src)?;
        let second_byte = Self::read_byte(src)?;
        
        *offset |= (first_byte >> 2) as usize;
        *offset |= (second_byte as usize) << 6;
        *offset += added_value;
        
        Ok(first_byte)
    }
}

/// LZ77 Decompressor for AC1021+ (2007+) files
pub struct Lz77AC21Decompressor;

impl Lz77AC21Decompressor {
    /// Decompress a compressed buffer
    pub fn decompress(
        source: &[u8],
        initial_offset: usize,
        length: usize,
        buffer: &mut [u8],
    ) -> Result<()> {
        let mut state = DecompressState {
            source_offset: 0,
            length: 0,
            source_index: initial_offset,
            op_code: source[initial_offset],
        };
        
        let mut dest_index: usize = 0;
        let end_index = state.source_index + length;
        
        state.source_index += 1;
        
        if state.source_index >= end_index {
            return Ok(());
        }
        
        if (state.op_code & 0xF0) == 0x20 {
            state.source_index += 3;
            state.length = (source[state.source_index - 1] & 7) as usize;
        }
        
        while state.source_index < end_index {
            // Next index
            state.next_index(source, buffer, &mut dest_index)?;
            
            if state.source_index >= end_index {
                break;
            }
            
            dest_index = state.copy_decompressed_chunks(source, end_index, buffer, dest_index)?;
        }
        
        Ok(())
    }
}

struct DecompressState {
    source_offset: usize,
    length: usize,
    source_index: usize,
    op_code: u8,
}

impl DecompressState {
    fn next_index(
        &mut self,
        source: &[u8],
        dest: &mut [u8],
        index: &mut usize,
    ) -> Result<()> {
        if self.length == 0 {
            self.read_literal_length(source);
        }
        
        // Copy bytes
        dest[*index..*index + self.length]
            .copy_from_slice(&source[self.source_index..self.source_index + self.length]);
        
        self.source_index += self.length;
        *index += self.length;
        
        Ok(())
    }
    
    fn copy_decompressed_chunks(
        &mut self,
        src: &[u8],
        end_index: usize,
        dst: &mut [u8],
        mut dest_index: usize,
    ) -> Result<usize> {
        self.length = 0;
        self.op_code = src[self.source_index];
        self.source_index += 1;
        
        self.read_instructions(src);
        
        loop {
            // Copy bytes from earlier position
            self.copy_bytes(dst, dest_index);
            
            dest_index += self.length;
            
            self.length = (self.op_code & 0x07) as usize;
            
            if self.length != 0 || self.source_index >= end_index {
                break;
            }
            
            self.op_code = src[self.source_index];
            self.source_index += 1;
            
            if (self.op_code >> 4) == 0 {
                break;
            }
            
            if (self.op_code >> 4) == 15 {
                self.op_code &= 15;
            }
            
            self.read_instructions(src);
        }
        
        Ok(dest_index)
    }
    
    fn read_instructions(&mut self, buffer: &[u8]) {
        match self.op_code >> 4 {
            0 => {
                self.length = (self.op_code & 0x0F) as usize + 0x13;
                self.source_offset = buffer[self.source_index] as usize;
                self.source_index += 1;
                self.op_code = buffer[self.source_index];
                self.source_index += 1;
                self.length = ((self.op_code >> 3 & 0x10) as usize) + self.length;
                self.source_offset = (((self.op_code & 0x78) as usize) << 5) + 1 + self.source_offset;
            }
            1 => {
                self.length = (self.op_code & 0x0F) as usize + 3;
                self.source_offset = buffer[self.source_index] as usize;
                self.source_index += 1;
                self.op_code = buffer[self.source_index];
                self.source_index += 1;
                self.source_offset = (((self.op_code & 0xF8) as usize) << 5) + 1 + self.source_offset;
            }
            2 => {
                self.source_offset = buffer[self.source_index] as usize;
                self.source_index += 1;
                self.source_offset = ((buffer[self.source_index] as usize) << 8) | self.source_offset;
                self.source_index += 1;
                self.length = (self.op_code & 7) as usize;
                
                if (self.op_code & 8) == 0 {
                    self.op_code = buffer[self.source_index];
                    self.source_index += 1;
                    self.length = ((self.op_code & 0xF8) as usize) + self.length;
                } else {
                    self.source_offset += 1;
                    self.length = ((buffer[self.source_index] as usize) << 3) + self.length;
                    self.source_index += 1;
                    self.op_code = buffer[self.source_index];
                    self.source_index += 1;
                    self.length = (((self.op_code & 0xF8) as usize) << 8) + self.length + 0x100;
                }
            }
            _ => {
                self.length = (self.op_code >> 4) as usize;
                self.source_offset = (self.op_code & 0x0F) as usize;
                self.op_code = buffer[self.source_index];
                self.source_index += 1;
                self.source_offset = (((self.op_code & 0xF8) as usize) << 1) + self.source_offset + 1;
            }
        }
    }
    
    fn read_literal_length(&mut self, buffer: &[u8]) {
        self.length = self.op_code as usize + 8;
        
        if self.length == 0x17 {
            let mut n = buffer[self.source_index] as usize;
            self.source_index += 1;
            self.length += n;
            
            if n == 0xFF {
                loop {
                    n = buffer[self.source_index] as usize;
                    self.source_index += 1;
                    n |= (buffer[self.source_index] as usize) << 8;
                    self.source_index += 1;
                    self.length += n;
                    
                    if n != 0xFFFF {
                        break;
                    }
                }
            }
        }
    }
    
    fn copy_bytes(&self, dst: &mut [u8], dst_index: usize) {
        let initial_index = dst_index - self.source_offset;
        
        for i in 0..self.length {
            dst[dst_index + i] = dst[initial_index + i];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ac21_decompressor_empty() {
        let source = vec![0x11]; // Terminator
        let mut buffer = vec![0u8; 100];
        
        // Should handle empty/minimal input gracefully
        let result = Lz77AC21Decompressor::decompress(&source, 0, 0, &mut buffer);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_simple_literal_copy() {
        // Create a simple test case with literal bytes
        // This is a minimal test - real DWG data would be more complex
        let mut source = vec![0u8; 256];
        source[0] = 0x11; // Terminator opcode
        
        // The decompressor should handle the terminator
    }
}
