use std::io::{Cursor, Read, Seek, SeekFrom};

use crate::error::{DxfError, Result};

/// LZ77 variant used by AC1018 (DWG 2004).
pub struct DwgLz77Ac18Decompressor;

impl DwgLz77Ac18Decompressor {
    pub fn decompress<R: Read>(mut compressed: R, decompressed_size: usize) -> Result<Vec<u8>> {
        let mut output = Cursor::new(vec![0u8; decompressed_size]);
        Self::decompress_to_dest(&mut compressed, &mut output)?;

        let pos = output.stream_position()? as usize;
        let mut data = output.into_inner();
        data.truncate(pos);
        Ok(data)
    }

    pub fn decompress_to_dest<R: Read, W: Read + std::io::Write + Seek>(
        src: &mut R,
        dst: &mut W,
    ) -> Result<()> {
        let mut temp_buf = vec![0u8; 128];
        let mut opcode1 = Self::read_u8(src)?;

        if (opcode1 & 0xF0) == 0 {
            opcode1 = Self::copy(Self::literal_count(opcode1, src)? + 3, src, dst, &mut temp_buf)?;
        }

        while opcode1 != 0x11 {
            let mut comp_offset = 0usize;
            let compressed_bytes: usize;

            if opcode1 < 0x10 || opcode1 >= 0x40 {
                compressed_bytes = (opcode1 as usize >> 4).saturating_sub(1);
                let opcode2 = Self::read_u8(src)?;
                comp_offset = (((opcode1 as usize >> 2) & 0x3) | ((opcode2 as usize) << 2)) + 1;
            } else if opcode1 < 0x20 {
                compressed_bytes = Self::read_compressed_bytes(opcode1, 0b0111, src)?;
                comp_offset = ((opcode1 as usize & 0x8) << 11) as usize;
                opcode1 = Self::two_byte_offset(&mut comp_offset, 0x4000, src)?;
            } else {
                compressed_bytes = Self::read_compressed_bytes(opcode1, 0b0001_1111, src)?;
                opcode1 = Self::two_byte_offset(&mut comp_offset, 1, src)?;
            }

            let position = dst.stream_position()?;
            if comp_offset == 0 {
                return Err(DxfError::Decompression("Invalid compressed offset 0".to_string()));
            }

            if temp_buf.len() < compressed_bytes {
                temp_buf.resize(compressed_bytes, 0);
            }

            dst.seek(SeekFrom::Start(position.saturating_sub(comp_offset as u64)))?;
            let copy_len = compressed_bytes.min(comp_offset);
            dst.read_exact(&mut temp_buf[..copy_len])?;
            dst.seek(SeekFrom::Start(position))?;

            let mut remaining = compressed_bytes;
            while remaining > 0 {
                let chunk = remaining.min(comp_offset);
                dst.write_all(&temp_buf[..chunk])?;
                remaining -= chunk;
            }

            let mut lit_count = opcode1 as usize & 0x3;
            if lit_count == 0 {
                opcode1 = Self::read_u8(src)?;
                if (opcode1 & 0xF0) == 0 {
                    lit_count = Self::literal_count(opcode1, src)? + 3;
                }
            }

            if lit_count > 0 {
                opcode1 = Self::copy(lit_count, src, dst, &mut temp_buf)?;
            }
        }

        Ok(())
    }

    fn copy<R: Read, W: std::io::Write>(
        count: usize,
        src: &mut R,
        dst: &mut W,
        temp_buf: &mut Vec<u8>,
    ) -> Result<u8> {
        if temp_buf.len() < count {
            temp_buf.resize(count, 0);
        }
        src.read_exact(&mut temp_buf[..count])?;
        dst.write_all(&temp_buf[..count])?;
        Self::read_u8(src)
    }

    fn literal_count<R: Read>(code: u8, src: &mut R) -> Result<usize> {
        let mut lowbits = (code & 0x0F) as usize;
        if lowbits == 0 {
            loop {
                let b = Self::read_u8(src)?;
                if b == 0 {
                    lowbits += 0xFF;
                } else {
                    lowbits += 0x0F + b as usize;
                    break;
                }
            }
        }
        Ok(lowbits)
    }

    fn read_compressed_bytes<R: Read>(opcode1: u8, valid_bits: u8, src: &mut R) -> Result<usize> {
        let mut compressed_bytes = (opcode1 & valid_bits) as usize;

        if compressed_bytes == 0 {
            loop {
                let b = Self::read_u8(src)?;
                if b == 0 {
                    compressed_bytes += 0xFF;
                } else {
                    compressed_bytes += b as usize + valid_bits as usize;
                    break;
                }
            }
        }

        Ok(compressed_bytes + 2)
    }

    fn two_byte_offset<R: Read>(offset: &mut usize, added_value: usize, src: &mut R) -> Result<u8> {
        let first = Self::read_u8(src)?;
        let second = Self::read_u8(src)?;

        *offset |= (first as usize) >> 2;
        *offset |= (second as usize) << 6;
        *offset += added_value;

        Ok(first)
    }

    fn read_u8<R: Read>(reader: &mut R) -> Result<u8> {
        let mut b = [0u8; 1];
        reader.read_exact(&mut b)?;
        Ok(b[0])
    }
}
