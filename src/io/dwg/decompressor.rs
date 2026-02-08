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
        let mut source_offset: usize = 0;
        let mut lit_length: usize = 0;
        let mut source_index = initial_offset;
        let mut op_code = source[source_index] as usize;
        let mut dest_index: usize = 0;
        let end_index = source_index + length;
        
        source_index += 1;
        
        if source_index >= end_index {
            return Ok(());
        }
        
        if (op_code & 0xF0) == 0x20 {
            source_index += 3;
            lit_length = (source[source_index - 1] & 7) as usize;
        }
        
        while source_index < end_index {
            // nextIndex - copy literal bytes with word-reordering
            if lit_length == 0 {
                lit_length = op_code + 8;
                if lit_length == 0x17 {
                    let mut n = source[source_index] as usize;
                    source_index += 1;
                    lit_length += n;
                    if n == 0xFF {
                        loop {
                            n = source[source_index] as usize;
                            source_index += 1;
                            n |= (source[source_index] as usize) << 8;
                            source_index += 1;
                            lit_length += n;
                            if n != 0xFFFF { break; }
                        }
                    }
                }
            }
            
            // Copy literal bytes with word-reordering (AC21 copy function)
            ac21_copy(source, source_index, buffer, dest_index, lit_length);
            source_index += lit_length;
            dest_index += lit_length;
            
            if source_index >= end_index {
                break;
            }
            
            // copyDecompressedChunks
            lit_length = 0;
            op_code = source[source_index] as usize;
            source_index += 1;
            
            // readInstructions + copy loop
            loop {
                // readInstructions
                match op_code >> 4 {
                    0 => {
                        lit_length = (op_code & 0x0F) + 0x13;
                        source_offset = source[source_index] as usize;
                        source_index += 1;
                        op_code = source[source_index] as usize;
                        source_index += 1;
                        lit_length = ((op_code >> 3) & 0x10) + lit_length;
                        source_offset = ((op_code & 0x78) << 5) + 1 + source_offset;
                    }
                    1 => {
                        lit_length = (op_code & 0x0F) + 3;
                        source_offset = source[source_index] as usize;
                        source_index += 1;
                        op_code = source[source_index] as usize;
                        source_index += 1;
                        source_offset = ((op_code & 0xF8) << 5) + 1 + source_offset;
                    }
                    2 => {
                        source_offset = source[source_index] as usize;
                        source_index += 1;
                        source_offset = ((source[source_index] as usize) << 8 & 0xFF00) | source_offset;
                        source_index += 1;
                        lit_length = op_code & 7;
                        if (op_code & 8) == 0 {
                            op_code = source[source_index] as usize;
                            source_index += 1;
                            lit_length = (op_code & 0xF8) + lit_length;
                        } else {
                            source_offset += 1;
                            lit_length = ((source[source_index] as usize) << 3) + lit_length;
                            source_index += 1;
                            op_code = source[source_index] as usize;
                            source_index += 1;
                            lit_length = ((op_code & 0xF8) << 8) + lit_length + 0x100;
                        }
                    }
                    _ => {
                        lit_length = op_code >> 4;
                        source_offset = op_code & 0x0F;
                        op_code = source[source_index] as usize;
                        source_index += 1;
                        source_offset = ((op_code & 0xF8) << 1) + source_offset + 1;
                    }
                }
                
                // copyBytes - copy from earlier position in dest buffer
                {
                    if source_offset > dest_index {
                        return Err(DxfError::Parse(format!(
                            "AC21 LZ77: source_offset ({}) > dest_index ({})", source_offset, dest_index
                        )));
                    }
                    let initial_index = dest_index - source_offset;
                    for i in 0..lit_length {
                        buffer[dest_index + i] = buffer[initial_index + i];
                    }
                }
                dest_index += lit_length;
                
                lit_length = op_code & 0x07;
                
                if lit_length != 0 || source_index >= end_index {
                    break;
                }
                
                op_code = source[source_index] as usize;
                source_index += 1;
                
                if (op_code >> 4) == 0 {
                    break;
                }
                
                if (op_code >> 4) == 15 {
                    op_code &= 15;
                }
            }
        }
        
        Ok(())
    }
}

/// AC21 literal copy with word-reordering
/// Copies bytes from src to dst with 32-byte block qword-reversal
fn ac21_copy(src: &[u8], mut src_idx: usize, dst: &mut [u8], mut dst_idx: usize, mut length: usize) {
    // Copy in 32-byte chunks with qword reordering
    while length >= 32 {
        // Reverse order of 8-byte groups within each 32-byte block
        dst[dst_idx]     = src[src_idx + 24];
        dst[dst_idx + 1] = src[src_idx + 25];
        dst[dst_idx + 2] = src[src_idx + 26];
        dst[dst_idx + 3] = src[src_idx + 27];
        
        dst[dst_idx + 4] = src[src_idx + 28];
        dst[dst_idx + 5] = src[src_idx + 29];
        dst[dst_idx + 6] = src[src_idx + 30];
        dst[dst_idx + 7] = src[src_idx + 31];
        
        dst[dst_idx + 8]  = src[src_idx + 16];
        dst[dst_idx + 9]  = src[src_idx + 17];
        dst[dst_idx + 10] = src[src_idx + 18];
        dst[dst_idx + 11] = src[src_idx + 19];
        
        dst[dst_idx + 12] = src[src_idx + 20];
        dst[dst_idx + 13] = src[src_idx + 21];
        dst[dst_idx + 14] = src[src_idx + 22];
        dst[dst_idx + 15] = src[src_idx + 23];
        
        dst[dst_idx + 16] = src[src_idx + 8];
        dst[dst_idx + 17] = src[src_idx + 9];
        dst[dst_idx + 18] = src[src_idx + 10];
        dst[dst_idx + 19] = src[src_idx + 11];
        
        dst[dst_idx + 20] = src[src_idx + 12];
        dst[dst_idx + 21] = src[src_idx + 13];
        dst[dst_idx + 22] = src[src_idx + 14];
        dst[dst_idx + 23] = src[src_idx + 15];
        
        dst[dst_idx + 24] = src[src_idx];
        dst[dst_idx + 25] = src[src_idx + 1];
        dst[dst_idx + 26] = src[src_idx + 2];
        dst[dst_idx + 27] = src[src_idx + 3];
        
        dst[dst_idx + 28] = src[src_idx + 4];
        dst[dst_idx + 29] = src[src_idx + 5];
        dst[dst_idx + 30] = src[src_idx + 6];
        dst[dst_idx + 31] = src[src_idx + 7];
        
        src_idx += 32;
        dst_idx += 32;
        length -= 32;
    }
    
    if length == 0 { return; }
    
    // For remaining bytes, use the C# m_copyMethods pattern
    ac21_copy_remainder(src, src_idx, dst, dst_idx, length);
}

/// Copy remaining 1-31 bytes with the AC21 byte reordering  
fn ac21_copy_remainder(src: &[u8], si: usize, dst: &mut [u8], di: usize, len: usize) {
    match len {
        1 => { dst[di] = src[si]; }
        2 => { dst[di] = src[si+1]; dst[di+1] = src[si]; }
        3 => { dst[di] = src[si+2]; dst[di+1] = src[si+1]; dst[di+2] = src[si]; }
        4 => { copy4(src, si, dst, di); }
        5 => { dst[di] = src[si+4]; copy4(src, si, dst, di+1); }
        6 => { dst[di] = src[si+5]; copy4(src, si+1, dst, di+1); dst[di+5] = src[si]; }
        7 => { dst[di] = src[si+6]; dst[di+1] = src[si+5]; copy4(src, si+1, dst, di+2); dst[di+6] = src[si]; }
        8 => { copy8(src, si, dst, di); }
        9 => { dst[di] = src[si+8]; copy8(src, si, dst, di+1); }
        10 => { dst[di] = src[si+9]; copy8(src, si+1, dst, di+1); dst[di+9] = src[si]; }
        11 => { dst[di] = src[si+10]; dst[di+1] = src[si+9]; copy8(src, si+1, dst, di+2); dst[di+10] = src[si]; }
        12 => { copy4(src, si+8, dst, di); copy8(src, si, dst, di+4); }
        13 => { dst[di] = src[si+12]; copy4(src, si+8, dst, di+1); copy8(src, si, dst, di+5); }
        14 => { dst[di] = src[si+13]; copy4(src, si+9, dst, di+1); copy8(src, si+1, dst, di+5); dst[di+13] = src[si]; }
        15 => { dst[di] = src[si+14]; dst[di+1] = src[si+13]; copy4(src, si+9, dst, di+2); copy8(src, si+1, dst, di+6); dst[di+14] = src[si]; }
        16 => { copy16(src, si, dst, di); }
        17 => { copy8(src, si+9, dst, di); dst[di+8] = src[si+8]; copy8(src, si, dst, di+9); }
        18 => { dst[di] = src[si+17]; copy16(src, si+1, dst, di+1); dst[di+17] = src[si]; }
        19 => { dst[di] = src[si+18]; dst[di+1] = src[si+17]; dst[di+2] = src[si+16]; copy16(src, si, dst, di+3); }
        20 => { copy4(src, si+16, dst, di); copy8(src, si+8, dst, di+4); copy8(src, si, dst, di+12); }
        21 => { dst[di] = src[si+20]; copy4(src, si+16, dst, di+1); copy8(src, si+8, dst, di+5); copy8(src, si, dst, di+13); }
        22 => { dst[di] = src[si+21]; dst[di+1] = src[si+20]; copy4(src, si+16, dst, di+2); copy8(src, si+8, dst, di+6); copy8(src, si, dst, di+14); }
        23 => { dst[di] = src[si+22]; dst[di+1] = src[si+21]; dst[di+2] = src[si+20]; copy4(src, si+16, dst, di+3); copy8(src, si+8, dst, di+7); copy8(src, si, dst, di+15); }
        24 => { copy8(src, si+16, dst, di); copy16(src, si, dst, di+8); }
        25 => { copy8(src, si+17, dst, di); dst[di+8] = src[si+16]; copy16(src, si, dst, di+9); }
        26 => { dst[di] = src[si+25]; copy8(src, si+17, dst, di+1); dst[di+9] = src[si+16]; copy16(src, si, dst, di+10); }
        27 => { dst[di] = src[si+26]; dst[di+1] = src[si+25]; copy8(src, si+17, dst, di+2); dst[di+10] = src[si+16]; copy16(src, si, dst, di+11); }
        28 => { copy4(src, si+24, dst, di); copy8(src, si+16, dst, di+4); copy8(src, si+8, dst, di+12); copy8(src, si, dst, di+20); }
        29 => { dst[di] = src[si+28]; copy4(src, si+24, dst, di+1); copy8(src, si+16, dst, di+5); copy8(src, si+8, dst, di+13); copy8(src, si, dst, di+21); }
        30 => { dst[di] = src[si+29]; dst[di+1] = src[si+28]; copy4(src, si+24, dst, di+2); copy8(src, si+16, dst, di+6); copy8(src, si+8, dst, di+14); copy8(src, si, dst, di+22); }
        31 => { dst[di] = src[si+30]; copy4(src, si+26, dst, di+1); copy8(src, si+18, dst, di+5); copy8(src, si+10, dst, di+13); copy8(src, si+2, dst, di+21); dst[di+29] = src[si+1]; dst[di+30] = src[si]; }
        _ => {}
    }
}

#[inline(always)]
fn copy4(src: &[u8], si: usize, dst: &mut [u8], di: usize) {
    dst[di]   = src[si];
    dst[di+1] = src[si+1];
    dst[di+2] = src[si+2];
    dst[di+3] = src[si+3];
}

#[inline(always)]
fn copy8(src: &[u8], si: usize, dst: &mut [u8], di: usize) {
    // copy8b in C# = two straight copy4b calls (no byte swapping!)
    dst[di]   = src[si];
    dst[di+1] = src[si+1];
    dst[di+2] = src[si+2];
    dst[di+3] = src[si+3];
    dst[di+4] = src[si+4];
    dst[di+5] = src[si+5];
    dst[di+6] = src[si+6];
    dst[di+7] = src[si+7];
}

#[inline(always)]
fn copy16(src: &[u8], si: usize, dst: &mut [u8], di: usize) {
    // copy16b in C# = copy8b(src+8, dst) then copy8b(src, dst+8)
    // Swaps two 8-byte halves
    copy8(src, si+8, dst, di);
    copy8(src, si, dst, di+8);
}
