//! DWG Stream Reader - Bit-level binary reading for DWG format
//!
//! DWG files use bit-packed data structures that require special handling.
//! This module provides a `BitReader` for reading individual bits and
//! various bit-coded data types defined in the DWG specification.
//!
//! ## Data Type Codes (from DWG spec)
//!
//! - B: bit (1 or 0)
//! - BB: special 2 bit code
//! - 3B: bit triplet (1-3 bits)
//! - BS: bitshort (16 bits)
//! - BL: bitlong (32 bits)
//! - BLL: bitlonglong (64 bits)
//! - BD: bitdouble
//! - RC: raw char (not compressed)
//! - RS: raw short (not compressed)
//! - RD: raw double (not compressed)
//! - RL: raw long (not compressed)
//! - MC: modular char
//! - MS: modular short
//! - H: handle reference
//! - T: text (bitshort length, followed by string)
//! - TU: Unicode text
//! - TV: Variable text (T for 2004-, TU for 2007+)

use std::io::{Read, Seek, SeekFrom};
use crate::error::{DxfError, Result};
use crate::types::{ACadVersion, Color, Vector2, Vector3};

/// Reference type for handle references
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DwgReferenceType {
    /// No reference, use handle as-is
    None = 0,
    /// Soft ownership reference
    SoftOwnership = 2,
    /// Hard ownership reference  
    HardOwnership = 3,
    /// Soft pointer reference
    SoftPointer = 4,
    /// Hard pointer reference
    HardPointer = 5,
}

impl TryFrom<u8> for DwgReferenceType {
    type Error = DxfError;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            0 => Ok(DwgReferenceType::None),
            2 => Ok(DwgReferenceType::SoftOwnership),
            3 => Ok(DwgReferenceType::HardOwnership),
            4 => Ok(DwgReferenceType::SoftPointer),
            5 => Ok(DwgReferenceType::HardPointer),
            _ => Err(DxfError::Parse(format!("Invalid reference type: {}", value))),
        }
    }
}

/// Trait for reading DWG bit-coded data
pub trait DwgStreamReader {
    /// Get the current bit shift (0-7)
    fn bit_shift(&self) -> u8;
    
    /// Get the current byte position in the stream
    fn position(&mut self) -> u64;
    
    /// Set the byte position in the stream (resets bit shift)
    fn set_position(&mut self, pos: u64) -> Result<()>;
    
    /// Get the position in bits
    fn position_in_bits(&mut self) -> u64;
    
    /// Set the position in bits
    fn set_position_in_bits(&mut self, pos: u64) -> Result<()>;
    
    /// Check if the stream is empty/exhausted
    fn is_empty(&mut self) -> bool;
    
    /// Advance the stream position by the given number of bytes (like C# Advance)
    /// This resets bit alignment to byte boundary
    fn advance_bytes(&mut self, count: usize) -> Result<()> {
        let current_bits = self.position_in_bits();
        // Advance by count bytes (count * 8 bits) from current bit position
        // But first align to byte boundary, then advance
        let new_bit_pos = current_bits + (count as u64 * 8);
        self.set_position_in_bits(new_bit_pos)
    }
    
    /// Advance by the given number of bytes
    fn advance(&mut self, offset: u64) -> Result<()>;
    
    /// Advance by one byte
    fn advance_byte(&mut self) -> Result<()>;
    
    /// Reset the bit shift to 0
    fn reset_shift(&mut self) -> u16;
    
    // === Bit-level reading ===
    
    /// Read a single bit (B)
    fn read_bit(&mut self) -> Result<bool>;
    
    /// Read 2 bits (BB)
    fn read_2bits(&mut self) -> Result<u8>;
    
    /// Read 3 bits (3B) - R24+
    fn read_3bits(&mut self) -> Result<u8>;
    
    // === Raw (uncompressed) types ===
    
    /// Read a raw byte (RC)
    fn read_byte(&mut self) -> Result<u8>;
    
    /// Read multiple raw bytes
    fn read_bytes(&mut self, length: usize) -> Result<Vec<u8>>;
    
    /// Read a raw char (RC)
    fn read_raw_char(&mut self) -> Result<u8>;
    
    /// Read a raw short (RS) - 16 bits, little-endian
    fn read_raw_short(&mut self) -> Result<i16>;
    
    /// Read a raw unsigned short
    fn read_raw_ushort(&mut self) -> Result<u16>;
    
    /// Read a raw long (RL) - 32 bits, little-endian
    fn read_raw_long(&mut self) -> Result<i32>;
    
    /// Read a raw unsigned long
    fn read_raw_ulong(&mut self) -> Result<u32>;

    /// Read a raw long long - 64 bits
    fn read_raw_longlong(&mut self) -> Result<i64>;
    
    /// Read a raw double (RD) - 64-bit IEEE floating point
    fn read_raw_double(&mut self) -> Result<f64>;
    
    /// Read 2 raw doubles (2RD)
    fn read_2raw_double(&mut self) -> Result<Vector2>;
    
    /// Read 3 raw doubles (3RD)
    fn read_3raw_double(&mut self) -> Result<Vector3>;
    
    // === Bit-coded types ===
    
    /// Read a bitshort (BS) - 16 bits
    fn read_bitshort(&mut self) -> Result<i16>;
    
    /// Read a bitlong (BL) - 32 bits
    fn read_bitlong(&mut self) -> Result<i32>;
    
    /// Read a bitlonglong (BLL) - 64 bits (R24+)
    fn read_bitlonglong(&mut self) -> Result<i64>;
    
    /// Read a bitdouble (BD)
    fn read_bitdouble(&mut self) -> Result<f64>;

    /// Read a bitdouble with default (DD)
    fn read_bitdouble_with_default(&mut self, default: f64) -> Result<f64>;
    
    /// Read 2 bitdoubles (2BD)
    fn read_2bitdouble(&mut self) -> Result<Vector2>;
    
    /// Read 3 bitdoubles (3BD)
    fn read_3bitdouble(&mut self) -> Result<Vector3>;
    
    /// Read 2 bitdoubles with default (2DD)
    fn read_2bitdouble_with_default(&mut self, default: Vector2) -> Result<Vector2>;
    
    /// Read 3 bitdoubles with default (3DD)
    fn read_3bitdouble_with_default(&mut self, default: Vector3) -> Result<Vector3>;

    /// Read bit extrusion (BE)
    fn read_bit_extrusion(&mut self) -> Result<Vector3>;

    /// Read bit thickness (BT)
    fn read_bit_thickness(&mut self) -> Result<f64>;
    
    // === Modular types ===
    
    /// Read a modular char (MC)
    fn read_modular_char(&mut self) -> Result<u64>;
    
    /// Read a signed modular char
    fn read_signed_modular_char(&mut self) -> Result<i64>;
    
    /// Read a modular short (MS)
    fn read_modular_short(&mut self) -> Result<i32>;
    
    // === Handle references ===
    
    /// Read a handle reference (H)
    fn read_handle(&mut self) -> Result<u64>;
    
    /// Read a handle reference relative to a reference handle
    fn read_handle_reference(&mut self, reference_handle: u64) -> Result<u64>;
    
    /// Read a handle reference with type information
    fn read_handle_with_type(&mut self, reference_handle: u64) -> Result<(u64, DwgReferenceType)>;
    
    // === Text types ===
    
    /// Read text (T) - bitshort length followed by string
    fn read_text(&mut self) -> Result<String>;
    
    /// Read Unicode text (TU) - for 2007+ files
    fn read_text_unicode(&mut self) -> Result<String>;
    
    /// Read variable text (TV) - T for 2004-, TU for 2007+
    fn read_variable_text(&mut self, version: ACadVersion) -> Result<String>;
    
    // === Other types ===
    
    /// Read a 16-byte sentinel (SN)
    fn read_sentinel(&mut self) -> Result<[u8; 16]>;
    
    /// Read a color by index
    fn read_color_by_index(&mut self) -> Result<Color>;
    
    /// Read CMC color value
    fn read_cmc_color(&mut self) -> Result<Color>;
    
    /// Read a DateTime from Julian date
    fn read_julian_date(&mut self) -> Result<f64>;
}

/// Bit reader implementation for DWG streams
pub struct BitReader<R: Read + Seek> {
    inner: R,
    last_byte: u8,
    bit_shift: u8,
    version: ACadVersion,
}

impl<R: Read + Seek> BitReader<R> {
    /// Create a new BitReader
    pub fn new(inner: R, version: ACadVersion) -> Self {
        Self {
            inner,
            last_byte: 0,
            bit_shift: 0,
            version,
        }
    }
    
    /// Get a reference to the inner reader
    pub fn inner(&self) -> &R {
        &self.inner
    }
    
    /// Get a mutable reference to the inner reader
    pub fn inner_mut(&mut self) -> &mut R {
        &mut self.inner
    }
    
    /// Get the version
    pub fn version(&self) -> ACadVersion {
        self.version
    }

    /// Read a single byte from the underlying stream
    fn read_raw_byte(&mut self) -> Result<u8> {
        let mut buf = [0u8; 1];
        self.inner.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    /// Apply bit shift when reading bytes
    fn read_byte_with_shift(&mut self) -> Result<u8> {
        if self.bit_shift == 0 {
            self.last_byte = self.read_raw_byte()?;
            return Ok(self.last_byte);
        }

        // Get the remaining bits from the last byte
        let high_bits = self.last_byte << self.bit_shift;
        self.last_byte = self.read_raw_byte()?;
        let low_bits = self.last_byte >> (8 - self.bit_shift);

        Ok(high_bits | low_bits)
    }
}

impl<R: Read + Seek> DwgStreamReader for BitReader<R> {
    fn bit_shift(&self) -> u8 {
        self.bit_shift
    }
    
    fn position(&mut self) -> u64 {
        self.inner.stream_position().unwrap_or(0)
    }
    
    fn set_position(&mut self, pos: u64) -> Result<()> {
        self.inner.seek(SeekFrom::Start(pos))?;
        self.bit_shift = 0;
        self.last_byte = 0;
        Ok(())
    }
    
    fn position_in_bits(&mut self) -> u64 {
        self.position() * 8 + self.bit_shift as u64
    }
    
    fn set_position_in_bits(&mut self, pos: u64) -> Result<()> {
        let byte_pos = pos / 8;
        let bit_pos = (pos % 8) as u8;
        
        self.inner.seek(SeekFrom::Start(byte_pos))?;
        self.bit_shift = 0;
        self.last_byte = 0;
        
        if bit_pos > 0 {
            // Read the byte and set bit position
            self.last_byte = self.read_raw_byte()?;
            self.inner.seek(SeekFrom::Current(-1))?;
            self.bit_shift = bit_pos;
        }
        
        Ok(())
    }
    
    fn is_empty(&mut self) -> bool {
        // Check if we're at end of stream
        if let Ok(pos) = self.inner.stream_position() {
            if let Ok(len) = self.inner.seek(SeekFrom::End(0)) {
                let _ = self.inner.seek(SeekFrom::Start(pos));
                return pos >= len;
            }
        }
        true
    }
    
    fn advance(&mut self, offset: u64) -> Result<()> {
        self.inner.seek(SeekFrom::Current(offset as i64))?;
        Ok(())
    }
    
    fn advance_byte(&mut self) -> Result<()> {
        self.last_byte = self.read_raw_byte()?;
        Ok(())
    }
    
    fn reset_shift(&mut self) -> u16 {
        let old = self.last_byte as u16;
        self.bit_shift = 0;
        old
    }
    
    // === Bit-level reading ===
    
    fn read_bit(&mut self) -> Result<bool> {
        if self.bit_shift == 0 {
            self.advance_byte()?;
            let result = (self.last_byte & 0x80) != 0;
            self.bit_shift = 1;
            return Ok(result);
        }
        
        let mask = 0x80u8 >> self.bit_shift;
        let result = (self.last_byte & mask) != 0;
        
        self.bit_shift += 1;
        if self.bit_shift >= 8 {
            self.bit_shift = 0;
        }
        
        Ok(result)
    }
    
    fn read_2bits(&mut self) -> Result<u8> {
        let b1 = self.read_bit()? as u8;
        let b2 = self.read_bit()? as u8;
        Ok((b1 << 1) | b2)
    }
    
    fn read_3bits(&mut self) -> Result<u8> {
        let b1 = self.read_bit()? as u8;
        let b2 = self.read_bit()? as u8;
        let b3 = self.read_bit()? as u8;
        Ok((b1 << 2) | (b2 << 1) | b3)
    }
    
    // === Raw types ===
    
    fn read_byte(&mut self) -> Result<u8> {
        self.read_byte_with_shift()
    }
    
    fn read_bytes(&mut self, length: usize) -> Result<Vec<u8>> {
        // Bounds check to prevent huge allocations
        if length > 100_000_000 {
            return Err(DxfError::Parse(format!("Allocation size too large: {}", length)));
        }
        let mut result = vec![0u8; length];
        for byte in result.iter_mut() {
            *byte = self.read_byte_with_shift()?;
        }
        Ok(result)
    }
    
    fn read_raw_char(&mut self) -> Result<u8> {
        self.read_byte_with_shift()
    }
    
    fn read_raw_short(&mut self) -> Result<i16> {
        let bytes = self.read_bytes(2)?;
        Ok(i16::from_le_bytes([bytes[0], bytes[1]]))
    }
    
    fn read_raw_ushort(&mut self) -> Result<u16> {
        let bytes = self.read_bytes(2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }
    
    fn read_raw_long(&mut self) -> Result<i32> {
        let bytes = self.read_bytes(4)?;
        Ok(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }
    
    fn read_raw_ulong(&mut self) -> Result<u32> {
        let bytes = self.read_bytes(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn read_raw_longlong(&mut self) -> Result<i64> {
        let bytes = self.read_bytes(8)?;
        Ok(i64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3],
            bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }
    
    fn read_raw_double(&mut self) -> Result<f64> {
        let bytes = self.read_bytes(8)?;
        Ok(f64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3],
            bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }
    
    fn read_2raw_double(&mut self) -> Result<Vector2> {
        let x = self.read_raw_double()?;
        let y = self.read_raw_double()?;
        Ok(Vector2::new(x, y))
    }
    
    fn read_3raw_double(&mut self) -> Result<Vector3> {
        let x = self.read_raw_double()?;
        let y = self.read_raw_double()?;
        let z = self.read_raw_double()?;
        Ok(Vector3::new(x, y, z))
    }
    
    // === Bit-coded types ===
    
    fn read_bitshort(&mut self) -> Result<i16> {
        let code = self.read_2bits()?;
        match code {
            0 => self.read_raw_short(),
            1 => Ok(self.read_raw_char()? as i16),
            2 => Ok(0),
            3 => Ok(256),
            _ => unreachable!(),
        }
    }
    
    fn read_bitlong(&mut self) -> Result<i32> {
        let code = self.read_2bits()?;
        match code {
            0 => self.read_raw_long(),
            1 => Ok(self.read_raw_char()? as i32),
            2 => Ok(0),
            3 => Err(DxfError::Parse("Invalid bitlong code 3".to_string())),
            _ => unreachable!(),
        }
    }
    
    fn read_bitlonglong(&mut self) -> Result<i64> {
        // BLL uses 3B for length indicator
        let count = self.read_3bits()?;
        if count == 0 {
            return Ok(0);
        }
        
        let mut result: i64 = 0;
        for i in 0..count {
            let byte = self.read_raw_char()? as i64;
            result |= byte << (i * 8);
        }
        
        Ok(result)
    }
    
    fn read_bitdouble(&mut self) -> Result<f64> {
        let code = self.read_2bits()?;
        match code {
            0 => self.read_raw_double(),
            1 => Ok(1.0),
            2 => Ok(0.0),
            3 => Err(DxfError::Parse("Invalid bitdouble code 3".to_string())),
            _ => unreachable!(),
        }
    }

    fn read_bitdouble_with_default(&mut self, default: f64) -> Result<f64> {
        let code = self.read_2bits()?;
        match code {
            0 => Ok(default),
            1 => {
                // Read 4 bytes and use as bytes 1-4, keeping default bytes 5-8
                let bytes = self.read_bytes(4)?;
                let default_bytes = default.to_le_bytes();
                let mut result_bytes = default_bytes;
                result_bytes[0] = bytes[0];
                result_bytes[1] = bytes[1];
                result_bytes[2] = bytes[2];
                result_bytes[3] = bytes[3];
                Ok(f64::from_le_bytes(result_bytes))
            }
            2 => {
                // Read 6 bytes:
                // - first 2 bytes replace bytes 5-6 (arr[4], arr[5])
                // - next 4 bytes replace bytes 1-4 (arr[0], arr[1], arr[2], arr[3])
                // - bytes 7-8 (arr[6], arr[7]) come from default
                let bytes = self.read_bytes(6)?;
                let default_bytes = default.to_le_bytes();
                let mut result_bytes = default_bytes;
                result_bytes[4] = bytes[0];
                result_bytes[5] = bytes[1];
                result_bytes[0] = bytes[2];
                result_bytes[1] = bytes[3];
                result_bytes[2] = bytes[4];
                result_bytes[3] = bytes[5];
                // arr[6] and arr[7] remain from default
                Ok(f64::from_le_bytes(result_bytes))
            }
            3 => self.read_raw_double(),
            _ => unreachable!(),
        }
    }
    
    fn read_2bitdouble(&mut self) -> Result<Vector2> {
        let x = self.read_bitdouble()?;
        let y = self.read_bitdouble()?;
        Ok(Vector2::new(x, y))
    }
    
    fn read_3bitdouble(&mut self) -> Result<Vector3> {
        let x = self.read_bitdouble()?;
        let y = self.read_bitdouble()?;
        let z = self.read_bitdouble()?;
        Ok(Vector3::new(x, y, z))
    }

    fn read_2bitdouble_with_default(&mut self, default: Vector2) -> Result<Vector2> {
        let x = self.read_bitdouble_with_default(default.x)?;
        let y = self.read_bitdouble_with_default(default.y)?;
        Ok(Vector2::new(x, y))
    }

    fn read_3bitdouble_with_default(&mut self, default: Vector3) -> Result<Vector3> {
        let x = self.read_bitdouble_with_default(default.x)?;
        let y = self.read_bitdouble_with_default(default.y)?;
        let z = self.read_bitdouble_with_default(default.z)?;
        Ok(Vector3::new(x, y, z))
    }

    fn read_bit_extrusion(&mut self) -> Result<Vector3> {
        // If the bit is set, the extrusion is (0, 0, 1)
        if self.read_bit()? {
            Ok(Vector3::new(0.0, 0.0, 1.0))
        } else {
            self.read_3bitdouble()
        }
    }

    fn read_bit_thickness(&mut self) -> Result<f64> {
        // If the bit is set, thickness is 0
        if self.read_bit()? {
            Ok(0.0)
        } else {
            self.read_bitdouble()
        }
    }
    
    // === Modular types ===
    
    fn read_modular_char(&mut self) -> Result<u64> {
        let mut result: u64 = 0;
        let mut shift = 0;
        
        loop {
            let byte = self.read_raw_char()?;
            result |= ((byte & 0x7F) as u64) << shift;
            
            if (byte & 0x80) == 0 {
                break;
            }
            
            shift += 7;
            if shift > 63 {
                return Err(DxfError::Parse("Modular char overflow".to_string()));
            }
        }
        
        Ok(result)
    }
    
    fn read_signed_modular_char(&mut self) -> Result<i64> {
        // Modular chars: bytes with high bit (0x80) as continuation flag
        // For signed: bit 6 (0x40) of LAST byte is sign flag
        // Single byte: bits 0-5 are data (0x3F)
        // Multi-byte: intermediate bytes have bits 0-6 data, last byte bits 0-5 data
        
        let first_byte = self.read_raw_char()?;
        
        // Single byte (no continuation)
        if (first_byte & 0x80) == 0 {
            let value = (first_byte & 0x3F) as i64;
            // Sign bit is bit 6 of this byte
            if (first_byte & 0x40) != 0 {
                return Ok(-value);
            }
            return Ok(value);
        }
        
        // Multi-byte: first byte has 7 bits of data
        let mut result: i64 = (first_byte & 0x7F) as i64;
        let mut shift = 7;
        
        loop {
            let byte = self.read_raw_char()?;
            
            if (byte & 0x80) == 0 {
                // Last byte: bits 0-5 are data, bit 6 is sign
                result |= ((byte & 0x3F) as i64) << shift;
                
                // Check sign bit
                if (byte & 0x40) != 0 {
                    return Ok(-result);
                }
                return Ok(result);
            }
            
            // Intermediate byte: bits 0-6 are data
            result |= ((byte & 0x7F) as i64) << shift;
            shift += 7;
            
            if shift > 56 {
                return Err(DxfError::Parse("Signed modular char overflow".to_string()));
            }
        }
    }
    
    fn read_modular_short(&mut self) -> Result<i32> {
        let mut result: i32 = 0;
        let mut shift = 0;
        
        loop {
            let word = self.read_raw_short()? as u16;
            result |= ((word & 0x7FFF) as i32) << shift;
            
            if (word & 0x8000) == 0 {
                break;
            }
            
            shift += 15;
            if shift > 31 {
                return Err(DxfError::Parse("Modular short overflow".to_string()));
            }
        }
        
        Ok(result)
    }
    
    // === Handle references ===
    
    fn read_handle(&mut self) -> Result<u64> {
        let code = self.read_byte()?;
        let counter = code & 0x0F;
        
        let mut handle: u64 = 0;
        for _ in 0..counter {
            handle = (handle << 8) | (self.read_byte()? as u64);
        }
        
        Ok(handle)
    }
    
    fn read_handle_reference(&mut self, reference_handle: u64) -> Result<u64> {
        let code = self.read_byte()?;
        let counter = code & 0x0F;
        let code_type = (code & 0xF0) >> 4;
        
        let mut offset: u64 = 0;
        for _ in 0..counter {
            offset = (offset << 8) | (self.read_byte()? as u64);
        }
        
        // Calculate actual handle based on reference type
        let handle = match code_type {
            0 => offset,
            2 | 3 | 4 | 5 => offset,
            6 => reference_handle.wrapping_add(1),
            8 => reference_handle.wrapping_sub(1),
            10 => reference_handle.wrapping_add(offset),
            12 => reference_handle.wrapping_sub(offset),
            _ => offset,
        };
        
        Ok(handle)
    }

    fn read_handle_with_type(&mut self, reference_handle: u64) -> Result<(u64, DwgReferenceType)> {
        let code = self.read_byte()?;
        let counter = code & 0x0F;
        let code_type = (code & 0xF0) >> 4;
        
        let mut offset: u64 = 0;
        for _ in 0..counter {
            offset = (offset << 8) | (self.read_byte()? as u64);
        }
        
        // Calculate actual handle based on reference type
        let handle = match code_type {
            0 => offset,
            2 | 3 | 4 | 5 => offset,
            6 => reference_handle.wrapping_add(1),
            8 => reference_handle.wrapping_sub(1),
            10 => reference_handle.wrapping_add(offset),
            12 => reference_handle.wrapping_sub(offset),
            _ => offset,
        };

        let ref_type = match code_type {
            0 => DwgReferenceType::None,
            2 => DwgReferenceType::SoftOwnership,
            3 => DwgReferenceType::HardOwnership,
            4 => DwgReferenceType::SoftPointer,
            5 => DwgReferenceType::HardPointer,
            _ => DwgReferenceType::None,
        };
        
        Ok((handle, ref_type))
    }
    
    // === Text types ===
    
    fn read_text(&mut self) -> Result<String> {
        let length = self.read_bitshort()? as usize;
        if length == 0 {
            return Ok(String::new());
        }
        
        let bytes = self.read_bytes(length)?;
        // Try to decode as UTF-8, fall back to lossy
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }
    
    fn read_text_unicode(&mut self) -> Result<String> {
        let length = self.read_bitshort()? as usize;
        if length == 0 {
            return Ok(String::new());
        }
        
        // Bounds check to prevent overflow
        if length > 50_000_000 {
            return Err(DxfError::Parse(format!("Unicode string length too large: {}", length)));
        }
        
        // Unicode: 2 bytes per character
        let bytes = self.read_bytes(length * 2)?;
        
        // Convert from UTF-16 LE
        let u16_vec: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();
        
        String::from_utf16(&u16_vec)
            .map_err(|e| DxfError::Encoding(e.to_string()))
    }
    
    fn read_variable_text(&mut self, version: ACadVersion) -> Result<String> {
        if version >= ACadVersion::AC1021 {
            self.read_text_unicode()
        } else {
            self.read_text()
        }
    }
    
    // === Other types ===
    
    fn read_sentinel(&mut self) -> Result<[u8; 16]> {
        let bytes = self.read_bytes(16)?;
        let mut result = [0u8; 16];
        result.copy_from_slice(&bytes);
        Ok(result)
    }
    
    fn read_color_by_index(&mut self) -> Result<Color> {
        let index = self.read_bitshort()?;
        Ok(Color::from_index(index))
    }
    
    fn read_cmc_color(&mut self) -> Result<Color> {
        if self.version >= ACadVersion::AC1018 {
            // R2004+: EnColor format
            // BS: color number + flags
            let size = self.read_bitshort()? as u16;
            
            if size == 0 {
                return Ok(Color::ByBlock);
            }
            
            let flags = size & 0xFF00;
            
            // Determine the color value.
            // Check 0x4000 BEFORE 0x8000: when 0x4000 is set, 0x8000 is also set,
            // but no RGB BL follows — only a handle in the handle stream.
            let color = if (flags & 0x4000) != 0 {
                // 0x4000: has AcDbColor book reference (handle read by caller)
                Color::ByBlock
            } else if (flags & 0x8000) != 0 {
                // 0x8000: complex color (RGB) — next is BL with RGB value
                let rgb = self.read_bitlong()? as u32;
                let r = ((rgb >> 16) & 0xFF) as u8;
                let g = ((rgb >> 8) & 0xFF) as u8;
                let b = (rgb & 0xFF) as u8;
                Color::Rgb { r, g, b }
            } else {
                // No flags: use color index (low 12 bits)
                Color::from_index((size & 0x0FFF) as i16)
            };
            
            // 0x2000: color is followed by transparency BL — always consume it
            if (flags & 0x2000) != 0 {
                let _transparency = self.read_bitlong()?;
            }
            
            Ok(color)
        } else {
            // R13-R2000: Simple CMC format
            let index = self.read_bitshort()?;
            Ok(Color::from_index(index))
        }
    }
    
    fn read_julian_date(&mut self) -> Result<f64> {
        let days = self.read_bitlong()?;
        let ms = self.read_bitlong()?;
        
        // Convert to days with fractional part
        Ok(days as f64 + (ms as f64 / 86400000.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    
    #[test]
    fn test_read_bit() {
        let data = vec![0b10101010, 0b11110000];
        let mut reader = BitReader::new(Cursor::new(data), ACadVersion::AC1015);
        
        assert!(reader.read_bit().unwrap());
        assert!(!reader.read_bit().unwrap());
        assert!(reader.read_bit().unwrap());
        assert!(!reader.read_bit().unwrap());
    }
    
    #[test]
    fn test_read_2bits() {
        let data = vec![0b11010010];
        let mut reader = BitReader::new(Cursor::new(data), ACadVersion::AC1015);
        
        assert_eq!(reader.read_2bits().unwrap(), 0b11);
        assert_eq!(reader.read_2bits().unwrap(), 0b01);
    }
    
    #[test]
    fn test_read_raw_short() {
        let data = vec![0x34, 0x12];
        let mut reader = BitReader::new(Cursor::new(data), ACadVersion::AC1015);
        
        assert_eq!(reader.read_raw_short().unwrap(), 0x1234);
    }
    
    #[test]
    fn test_read_bitshort_zero() {
        // Code 2 = value is 0
        let data = vec![0b10000000]; // 2 in first 2 bits
        let mut reader = BitReader::new(Cursor::new(data), ACadVersion::AC1015);
        
        assert_eq!(reader.read_bitshort().unwrap(), 0);
    }
    
    #[test]
    fn test_read_bitshort_256() {
        // Code 3 = value is 256
        let data = vec![0b11000000]; // 3 in first 2 bits
        let mut reader = BitReader::new(Cursor::new(data), ACadVersion::AC1015);
        
        assert_eq!(reader.read_bitshort().unwrap(), 256);
    }
}
