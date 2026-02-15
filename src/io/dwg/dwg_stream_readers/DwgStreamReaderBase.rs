use std::io::{Cursor, Read, Seek, SeekFrom};

use byteorder::{LittleEndian, ReadBytesExt};

use crate::error::{DxfError, Result};
use crate::types::{Color, DxfVersion, Transparency, Vector2, Vector3};

use super::idwg_stream_reader::{DwgObjectType, DwgReferenceType, DwgStreamReader, ReadSeek};

/// Shared implementation for DWG bit-stream readers.
/// This is the R13/R14 base reader. Version-specific overrides are done
/// via the `version` field (matching the C# inheritance chain).
pub struct DwgStreamReaderBase {
    stream: Box<dyn ReadSeek>,
    pub version: DxfVersion,
    bit_shift: u8,
    is_empty: bool,
    last_byte: u8,
    text_stream: Option<Cursor<Vec<u8>>>,
}

impl DwgStreamReaderBase {
    pub fn new(stream: Box<dyn ReadSeek>) -> Self {
        Self {
            stream,
            version: DxfVersion::Unknown,
            bit_shift: 0,
            is_empty: false,
            last_byte: 0,
            text_stream: None,
        }
    }

    pub fn get_stream_handler<R: Read + Seek + 'static>(
        version: DxfVersion,
        stream: R,
    ) -> Self {
        let mut reader = Self::new(Box::new(stream));
        reader.version = version;
        reader
    }

    pub fn with_version(mut self, version: DxfVersion) -> Self {
        self.version = version;
        self
    }

    pub fn with_text_stream(mut self, text_stream: Vec<u8>) -> Self {
        self.text_stream = Some(Cursor::new(text_stream));
        self
    }

    fn ensure_text_stream(&self) -> Result<()> {
        if self.text_stream.is_some() {
            Ok(())
        } else {
            Err(DxfError::NotImplemented(
                "DWG string stream is not initialized".to_string(),
            ))
        }
    }

    /// Apply bit-shift to read a full byte, combining bits from last_byte and the next byte.
    fn apply_shift_to_last_byte(&mut self) -> Result<u8> {
        let value = self.last_byte << self.bit_shift;
        self.advance_byte()?;
        Ok(value | (self.last_byte >> (8 - self.bit_shift)))
    }

    /// Read 3 bits (used by ReadBitLongLong).
    fn read_3_bits(&mut self) -> Result<u8> {
        let mut b: u8 = 0;
        if self.read_bit()? {
            b = 1;
        }
        b <<= 1;
        if self.read_bit()? {
            b |= 1;
        }
        b <<= 1;
        if self.read_bit()? {
            b |= 1;
        }
        Ok(b)
    }

    /// Apply bit-shift to an array of bytes read from stream.
    fn apply_shift_to_arr(&mut self, arr: &mut [u8]) -> Result<()> {
        let length = arr.len();
        self.stream.read_exact(arr)?;

        if self.bit_shift > 0 {
            let shift = 8 - self.bit_shift;
            for i in 0..length {
                let last_byte_value = self.last_byte << self.bit_shift;
                self.last_byte = arr[i];
                arr[i] = last_byte_value | (self.last_byte >> shift);
            }
        }
        Ok(())
    }

    /// Read a handle value (big-endian, variable-length).
    fn read_handle_bytes(&mut self, length: usize) -> Result<u64> {
        let mut raw = vec![0u8; length];
        let mut arr = [0u8; 8];

        self.stream.read_exact(&mut raw)?;

        if self.bit_shift == 0 {
            for i in 0..length {
                arr[length - 1 - i] = raw[i];
            }
        } else {
            let shift = 8 - self.bit_shift;
            for i in 0..length {
                let last_byte_value = self.last_byte << self.bit_shift;
                self.last_byte = raw[i];
                let value = last_byte_value | (self.last_byte >> shift);
                arr[length - 1 - i] = value;
            }
        }

        Ok(u64::from_le_bytes(arr))
    }

    /// Apply flag to position (for string stream detection).
    fn apply_flag_to_position(&mut self, last_pos: u64) -> Result<(u64, u64)> {
        // Decrement by 16 bytes (128 bits)
        let mut length = if last_pos >= 16 { last_pos - 16 } else { 0 };
        self.set_position_in_bits(length)?;

        // Read short at location endbit - 128 (bits)
        let mut str_data_size = self.read_u_short()? as u64;

        // If this short has the 0x8000 bit set
        if (str_data_size & 0x8000) != 0 {
            length -= 16;
            self.set_position_in_bits(length)?;
            str_data_size &= 0x7FFF;
            let hi_size = self.read_u_short()? as u64;
            str_data_size += (hi_size & 0xFFFF) << 15;
        }

        Ok((length, str_data_size))
    }

    /// Read unsigned short (little-endian, applying bit-shift).
    fn read_u_short(&mut self) -> Result<u16> {
        let lo = self.read_byte()? as u16;
        let hi = self.read_byte()? as u16;
        Ok(lo | (hi << 8))
    }

    pub fn explore(&mut self) -> Result<u64> {
        self.position_in_bits()
    }
}

impl DwgStreamReader for DwgStreamReaderBase {
    fn bit_shift(&self) -> u8 {
        self.bit_shift
    }

    fn set_bit_shift(&mut self, value: u8) {
        self.bit_shift = value & 7;
    }

    fn is_empty(&self) -> bool {
        self.is_empty
    }

    fn position(&mut self) -> Result<u64> {
        self.stream.stream_position().map_err(Into::into)
    }

    fn set_position(&mut self, value: u64) -> Result<()> {
        self.stream.seek(SeekFrom::Start(value))?;
        self.bit_shift = 0;
        Ok(())
    }

    /// C#: stream.Position * 8 + (bitShift > 0 ? bitShift - 8 : 0)
    /// When bitShift > 0, the stream position is already PAST the byte being read
    /// (because advance_byte was called), so we subtract 8.
    fn position_in_bits(&mut self) -> Result<u64> {
        let byte_pos = self.position()?;
        let bit_pos = byte_pos * 8;
        if self.bit_shift > 0 {
            Ok(bit_pos + (self.bit_shift as u64) - 8)
        } else {
            Ok(bit_pos)
        }
    }

    fn set_position_in_bits(&mut self, value: u64) -> Result<()> {
        let byte_pos = value >> 3;
        let shift = (value & 7) as u8;
        self.stream.seek(SeekFrom::Start(byte_pos))?;
        self.bit_shift = shift;
        if shift > 0 {
            self.advance_byte()?;
        }
        Ok(())
    }

    fn stream(&mut self) -> &mut (dyn ReadSeek + '_) {
        &mut *self.stream
    }

    /// C# Advance: if offset > 1, seek forward offset-1, then ReadByte()
    fn advance(&mut self, offset: usize) -> Result<()> {
        if offset > 1 {
            self.stream.seek(SeekFrom::Current((offset - 1) as i64))?;
        }
        self.read_byte()?;
        Ok(())
    }

    fn advance_byte(&mut self) -> Result<()> {
        self.last_byte = self.stream.read_u8()?;
        Ok(())
    }

    // ---- Handle references ----
    // C# format: |CODE (4 bits)|COUNTER (4 bits)|HANDLE or OFFSET|
    fn handle_reference(&mut self) -> Result<u64> {
        let (value, _) = self.handle_reference_with_type(0)?;
        Ok(value)
    }

    fn handle_reference_from(&mut self, reference_handle: u64) -> Result<u64> {
        let (value, _) = self.handle_reference_with_type(reference_handle)?;
        Ok(value)
    }

    fn handle_reference_with_type(
        &mut self,
        reference_handle: u64,
    ) -> Result<(u64, DwgReferenceType)> {
        // Read the form byte: CODE in high nibble, COUNTER in low nibble
        let form = self.read_byte()?;
        let code = form >> 4;
        let counter = (form & 0x0F) as usize;

        // Get the reference type from the low 2 bits of code
        // C#: reference = (DwgReferenceType)((uint)code & 0b0011);
        let ref_code = code & 0x03;
        let reference = match ref_code {
            0 => DwgReferenceType::Undefined,
            1 => DwgReferenceType::Unknown1,
            2 => DwgReferenceType::SoftOwnership,
            3 => DwgReferenceType::HardOwnership,
            _ => unreachable!(),
        };

        let result = if code <= 0x05 {
            // 0x2..0x5: just read offset and use it as the result
            self.read_handle_bytes(counter)?
        } else if code == 0x06 {
            // result is reference_handle + 1 (length is 0 in this case)
            reference_handle.wrapping_add(1)
        } else if code == 0x08 {
            // result is reference_handle - 1 (length is 0 in this case)
            reference_handle.wrapping_sub(1)
        } else if code == 0x0A {
            // result is reference_handle plus offset
            let offset = self.read_handle_bytes(counter)?;
            reference_handle.wrapping_add(offset)
        } else if code == 0x0C {
            // result is reference_handle minus offset
            let offset = self.read_handle_bytes(counter)?;
            reference_handle.wrapping_sub(offset)
        } else {
            return Err(DxfError::Parse(format!(
                "[HandleReference] invalid reference code with value: {}",
                code
            )));
        };

        Ok((result, reference))
    }

    fn read_2_bit_double(&mut self) -> Result<Vector2> {
        Ok(Vector2::new(self.read_bit_double()?, self.read_bit_double()?))
    }

    fn read_2_bit_double_with_default(&mut self, default_values: Vector2) -> Result<Vector2> {
        Ok(Vector2::new(
            self.read_bit_double_with_default(default_values.x)?,
            self.read_bit_double_with_default(default_values.y)?,
        ))
    }

    fn read_2_bits(&mut self) -> Result<u8> {
        let value = if self.bit_shift == 0 {
            self.advance_byte()?;
            self.bit_shift = 2;
            self.last_byte >> 6
        } else if self.bit_shift == 7 {
            let carry = (self.last_byte << 1) & 0b10;
            self.advance_byte()?;
            self.bit_shift = 1;
            carry | (self.last_byte >> 7)
        } else {
            let val = (self.last_byte >> (6 - self.bit_shift)) & 0b11;
            self.bit_shift = (self.bit_shift + 2) & 7;
            val
        };
        Ok(value)
    }

    fn read_2_raw_double(&mut self) -> Result<Vector2> {
        Ok(Vector2::new(self.read_double()?, self.read_double()?))
    }

    fn read_3_bit_double(&mut self) -> Result<Vector3> {
        Ok(Vector3::new(
            self.read_bit_double()?,
            self.read_bit_double()?,
            self.read_bit_double()?,
        ))
    }

    fn read_3_bit_double_with_default(&mut self, default_values: Vector3) -> Result<Vector3> {
        Ok(Vector3::new(
            self.read_bit_double_with_default(default_values.x)?,
            self.read_bit_double_with_default(default_values.y)?,
            self.read_bit_double_with_default(default_values.z)?,
        ))
    }

    fn read_3_raw_double(&mut self) -> Result<Vector3> {
        Ok(Vector3::new(
            self.read_double()?,
            self.read_double()?,
            self.read_double()?,
        ))
    }

    fn read_8_bit_julian_date(&mut self) -> Result<(i32, i32)> {
        Ok((self.read_int()?, self.read_int()?))
    }

    fn read_bit(&mut self) -> Result<bool> {
        if self.bit_shift == 0 {
            self.advance_byte()?;
            let result = (self.last_byte & 128) == 128;
            self.bit_shift = 1;
            return Ok(result);
        }

        let value = ((self.last_byte << self.bit_shift) & 128) == 128;
        self.bit_shift = (self.bit_shift + 1) & 7;
        Ok(value)
    }

    fn read_bit_as_short(&mut self) -> Result<i16> {
        Ok(if self.read_bit()? { 1 } else { 0 })
    }

    fn read_bit_double(&mut self) -> Result<f64> {
        match self.read_2_bits()? {
            0 => self.read_double(),
            1 => Ok(1.0),
            2 => Ok(0.0),
            _ => Err(DxfError::Parse("Invalid bitdouble code".to_string())),
        }
    }

    /// DD : BitDouble With Default
    /// C# does byte-level patching of the default value based on the 2-bit code.
    fn read_bit_double_with_default(&mut self, def: f64) -> Result<f64> {
        let mut arr = def.to_le_bytes();

        match self.read_2_bits()? {
            // 00: No more data present, use default
            0 => Ok(def),
            // 01: 4 bytes patch first 4 bytes of the default double
            1 => {
                if self.bit_shift == 0 {
                    self.advance_byte()?;
                    arr[0] = self.last_byte;
                    self.advance_byte()?;
                    arr[1] = self.last_byte;
                    self.advance_byte()?;
                    arr[2] = self.last_byte;
                    self.advance_byte()?;
                    arr[3] = self.last_byte;
                } else {
                    let shift = 8 - self.bit_shift;
                    arr[0] = self.last_byte << self.bit_shift;
                    self.advance_byte()?;
                    arr[0] |= self.last_byte >> shift;
                    arr[1] = self.last_byte << self.bit_shift;
                    self.advance_byte()?;
                    arr[1] |= self.last_byte >> shift;
                    arr[2] = self.last_byte << self.bit_shift;
                    self.advance_byte()?;
                    arr[2] |= self.last_byte >> shift;
                    arr[3] = self.last_byte << self.bit_shift;
                    self.advance_byte()?;
                    arr[3] |= self.last_byte >> shift;
                }
                Ok(f64::from_le_bytes(arr))
            }
            // 10: 6 bytes - first 2 patch bytes 4,5; last 4 patch bytes 0..3
            2 => {
                if self.bit_shift == 0 {
                    self.advance_byte()?;
                    arr[4] = self.last_byte;
                    self.advance_byte()?;
                    arr[5] = self.last_byte;
                    self.advance_byte()?;
                    arr[0] = self.last_byte;
                    self.advance_byte()?;
                    arr[1] = self.last_byte;
                    self.advance_byte()?;
                    arr[2] = self.last_byte;
                    self.advance_byte()?;
                    arr[3] = self.last_byte;
                } else {
                    let shift = 8 - self.bit_shift;
                    arr[4] = self.last_byte << self.bit_shift;
                    self.advance_byte()?;
                    arr[4] |= self.last_byte >> shift;
                    arr[5] = self.last_byte << self.bit_shift;
                    self.advance_byte()?;
                    arr[5] |= self.last_byte >> shift;
                    arr[0] = self.last_byte << self.bit_shift;
                    self.advance_byte()?;
                    arr[0] |= self.last_byte >> shift;
                    arr[1] = self.last_byte << self.bit_shift;
                    self.advance_byte()?;
                    arr[1] |= self.last_byte >> shift;
                    arr[2] = self.last_byte << self.bit_shift;
                    self.advance_byte()?;
                    arr[2] |= self.last_byte >> shift;
                    arr[3] = self.last_byte << self.bit_shift;
                    self.advance_byte()?;
                    arr[3] |= self.last_byte >> shift;
                }
                Ok(f64::from_le_bytes(arr))
            }
            // 11: A full RD follows
            3 => self.read_double(),
            _ => unreachable!(),
        }
    }

    /// BitExtrusion - R13/R14 base: just 3BD (3 bitdoubles).
    /// AC15+ overrides to use bit-flag optimization.
    fn read_bit_extrusion(&mut self) -> Result<Vector3> {
        if self.version >= DxfVersion::AC1015 {
            // R2000+: single bit, if 1 => (0,0,1), else 3BD
            if self.read_bit()? {
                Ok(Vector3::new(0.0, 0.0, 1.0))
            } else {
                self.read_3_bit_double()
            }
        } else {
            // R13-R14: just 3BD
            self.read_3_bit_double()
        }
    }

    fn read_bit_long(&mut self) -> Result<i32> {
        match self.read_2_bits()? {
            0 => self.read_int(),
            1 => {
                if self.bit_shift == 0 {
                    self.advance_byte()?;
                    Ok(self.last_byte as i32)
                } else {
                    Ok(self.apply_shift_to_last_byte()? as i32)
                }
            }
            2 => Ok(0),
            _ => Err(DxfError::Parse("Failed to read ReadBitLong".to_string())),
        }
    }

    /// BLL : bitlonglong (64 bits) - reads 3 bits for size, then that many bytes
    fn read_bit_long_long(&mut self) -> Result<i64> {
        let size = self.read_3_bits()? as usize;
        let mut value: u64 = 0;
        for i in 0..size {
            let b = self.read_byte()? as u64;
            value += b << (i << 3);
        }
        Ok(value as i64)
    }

    fn read_bit_short(&mut self) -> Result<i16> {
        match self.read_2_bits()? {
            0 => {
                // 00: A short (2 bytes) follows, little-endian
                let lo = self.read_byte()? as u16;
                let hi = self.read_byte()? as u16;
                Ok((lo | (hi << 8)) as i16)
            }
            1 => {
                // 01: An unsigned char (1 byte) follows
                if self.bit_shift == 0 {
                    self.advance_byte()?;
                    Ok(self.last_byte as i16)
                } else {
                    Ok(self.apply_shift_to_last_byte()? as i16)
                }
            }
            2 => Ok(0),   // 10: 0
            3 => Ok(256),  // 11: 256
            _ => unreachable!(),
        }
    }

    fn read_bit_short_as_bool(&mut self) -> Result<bool> {
        Ok(self.read_bit_short()? != 0)
    }

    /// BitThickness - R13/R14 base: just BD.
    /// AC15+ overrides to use bit-flag optimization.
    fn read_bit_thickness(&mut self) -> Result<f64> {
        if self.version >= DxfVersion::AC1015 {
            // R2000+: single bit, if 1 => 0.0, else BD
            if self.read_bit()? {
                Ok(0.0)
            } else {
                self.read_bit_double()
            }
        } else {
            // R13-R14: just BD
            self.read_bit_double()
        }
    }

    /// Read a byte, applying bit-shift if necessary.
    fn read_byte(&mut self) -> Result<u8> {
        if self.bit_shift == 0 {
            self.last_byte = self.stream.read_u8()?;
            return Ok(self.last_byte);
        }

        let last_values = self.last_byte << self.bit_shift;
        self.last_byte = self.stream.read_u8()?;
        Ok(last_values | (self.last_byte >> (8 - self.bit_shift)))
    }

    fn read_bytes(&mut self, length: usize) -> Result<Vec<u8>> {
        let mut data = vec![0u8; length];
        self.apply_shift_to_arr(&mut data)?;
        Ok(data)
    }

    /// CMC : CmColor value
    /// R15 and earlier: BS color index
    /// AC18+: complex color with RGB, color name, book name
    fn read_cm_color(&mut self, _use_text_stream: bool) -> Result<Color> {
        if self.version >= DxfVersion::AC1018 {
            let _color_index = self.read_bit_short()?;
            let rgb = self.read_bit_long()? as u32;
            let arr = rgb.to_le_bytes();

            let color = if rgb == 0xC000_0000 {
                Color::ByLayer
            } else if (rgb & 0x0100_0000) != 0 {
                Color::from_index(arr[0] as i16)
            } else {
                Color::from_rgb(arr[2], arr[1], arr[0])
            };

            let id = self.read_byte()?;
            if (id & 1) == 1 {
                let _ = self.read_variable_text()?;
            }
            if (id & 2) == 2 {
                let _ = self.read_variable_text()?;
            }

            return Ok(color);
        }

        // R15 and earlier: just BS color index
        Ok(Color::from_index(self.read_bit_short()?))
    }

    fn read_color_by_index(&mut self) -> Result<Color> {
        Ok(Color::from_index(self.read_bit_short()?))
    }

    fn read_date_time(&mut self) -> Result<(i32, i32)> {
        Ok((self.read_bit_long()?, self.read_bit_long()?))
    }

    fn read_double(&mut self) -> Result<f64> {
        if self.bit_shift == 0 {
            return self.stream.read_f64::<LittleEndian>().map_err(Into::into);
        }
        // When bit-shifted, read 8 bytes through the byte reader
        let bytes = self.read_bytes(8)?;
        Ok(f64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3],
            bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    /// ENC: Entity color with optional transparency.
    /// R15 and earlier: just BS color index.
    /// AC18+: complex color with flags, RGB, transparency.
    fn read_en_color(&mut self) -> Result<(Color, Transparency, bool)> {
        if self.version >= DxfVersion::AC1018 {
            let size = self.read_bit_short()?;
            if size == 0 {
                return Ok((Color::ByBlock, Transparency::OPAQUE, false));
            }

            let flags = (size as u16) & 0xFF00;
            let mut is_book_color = false;

            let color = if (flags & 0x4000) != 0 {
                // 0x4000: has AcDbColor reference (0x8000 is also set)
                is_book_color = true;
                Color::ByBlock
            } else if (flags & 0x8000) != 0 {
                // 0x8000: complex color (rgb)
                let rgb = self.read_bit_long()? as u32;
                let arr = rgb.to_le_bytes();
                Color::from_rgb(arr[2], arr[1], arr[0])
            } else {
                // Color index
                Color::from_index((size & 0x0FFF) as i16)
            };

            let transparency = if (flags & 0x2000) != 0 {
                let value = self.read_bit_long()? as u32;
                Transparency::from_alpha_value(value)
            } else {
                Transparency::BY_LAYER
            };

            return Ok((color, transparency, is_book_color));
        }

        // R15 and earlier
        let color_number = self.read_bit_short()?;
        Ok((Color::from_index(color_number), Transparency::BY_LAYER, false))
    }

    fn read_int(&mut self) -> Result<i32> {
        if self.bit_shift == 0 {
            return self.stream.read_i32::<LittleEndian>().map_err(Into::into);
        }
        let bytes = self.read_bytes(4)?;
        Ok(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    /// MC : modular char
    /// Stream of bytes, high bit is continuation flag.
    fn read_modular_char(&mut self) -> Result<u64> {
        let mut shift = 0;
        let last_byte = self.read_byte()?;
        let mut value = (last_byte & 0x7F) as u64;

        if (last_byte & 0x80) != 0 {
            loop {
                shift += 7;
                let last = self.read_byte()?;
                value |= ((last & 0x7F) as u64) << shift;
                if (last & 0x80) == 0 {
                    break;
                }
            }
        }

        Ok(value)
    }

    /// MC : signed modular char
    /// The 4th bit (bit 6, 0x40) of the final byte is the sign bit.
    fn read_signed_modular_char(&mut self) -> Result<i64> {
        if self.bit_shift == 0 {
            // No shift, read normal
            self.advance_byte()?;

            if (self.last_byte & 0x80) == 0 {
                // Single byte
                let value = (self.last_byte & 0x3F) as i64;
                if (self.last_byte & 0x40) != 0 {
                    return Ok(-value);
                }
                return Ok(value);
            }

            let mut total_shift = 0;
            let mut sum = (self.last_byte & 0x7F) as i64;
            loop {
                total_shift += 7;
                self.advance_byte()?;
                if (self.last_byte & 0x80) != 0 {
                    sum |= ((self.last_byte & 0x7F) as i64) << total_shift;
                } else {
                    break;
                }
            }

            let mut value = sum | (((self.last_byte & 0x3F) as i64) << total_shift);
            if (self.last_byte & 0x40) != 0 {
                value = -value;
            }
            Ok(value)
        } else {
            // Apply the shift to each byte
            let last_byte = self.apply_shift_to_last_byte()?;
            if (last_byte & 0x80) == 0 {
                let value = (last_byte & 0x3F) as i64;
                if (last_byte & 0x40) != 0 {
                    return Ok(-value);
                }
                return Ok(value);
            }

            let mut total_shift = 0;
            let mut sum = (last_byte & 0x7F) as i64;
            let mut curr_byte;
            loop {
                total_shift += 7;
                curr_byte = self.apply_shift_to_last_byte()?;
                if (curr_byte & 0x80) != 0 {
                    sum |= ((curr_byte & 0x7F) as i64) << total_shift;
                } else {
                    break;
                }
            }

            let mut value = sum | (((curr_byte & 0x3F) as i64) << total_shift);
            if (curr_byte & 0x40) != 0 {
                value = -value;
            }
            Ok(value)
        }
    }

    /// MS : modular short
    /// Reads pairs of bytes: b1 (full), b2 (high bit = continuation flag, 7 bits data).
    fn read_modular_short(&mut self) -> Result<i32> {
        let mut shift = 0x0F; // 15

        let b1 = self.read_byte()?;
        let b2 = self.read_byte()?;

        let mut flag = (b2 & 0x80) == 0;
        let mut value = (b1 as i32) | (((b2 & 0x7F) as i32) << 8);

        while !flag {
            let b1 = self.read_byte()?;
            let b2 = self.read_byte()?;
            flag = (b2 & 0x80) == 0;
            value |= (b1 as i32) << shift;
            shift += 8;
            value |= ((b2 & 0x7F) as i32) << shift;
            shift += 7;
        }

        Ok(value)
    }

    /// OT : Object type
    /// Until R2007: bit short.
    /// R2010+: bit pair + 1 or 2 bytes.
    fn read_object_type(&mut self) -> Result<DwgObjectType> {
        if self.version >= DxfVersion::AC1024 {
            let pair = self.read_2_bits()?;
            let value = match pair {
                0 => self.read_byte()? as u16,
                1 => 0x01F0 + self.read_byte()? as u16,
                2 | 3 => self.read_short()? as u16,
                _ => unreachable!(),
            };
            return Ok(DwgObjectType(value));
        }

        Ok(DwgObjectType(self.read_bit_short()? as u16))
    }

    /// RC : raw char (not compressed) - goes through ReadByte (with bit-shift)
    fn read_raw_char(&mut self) -> Result<u8> {
        self.read_byte()
    }

    /// RL : raw long (not compressed) - 4 bytes LE = i32, widened to i64
    fn read_raw_long(&mut self) -> Result<i64> {
        Ok(self.read_int()? as i64)
    }

    fn read_raw_u_long(&mut self) -> Result<u64> {
        Ok(self.read_uint()? as u64)
    }

    /// SN : 16 byte sentinel - read through ReadBytes (applies bit-shift)
    fn read_sentinel(&mut self) -> Result<[u8; 16]> {
        let bytes = self.read_bytes(16)?;
        let mut sentinel = [0u8; 16];
        sentinel.copy_from_slice(&bytes);
        Ok(sentinel)
    }

    fn read_short(&mut self) -> Result<i16> {
        if self.bit_shift == 0 {
            return self.stream.read_i16::<LittleEndian>().map_err(Into::into);
        }
        let bytes = self.read_bytes(2)?;
        Ok(i16::from_le_bytes([bytes[0], bytes[1]]))
    }

    /// R12-R15 text: short (length), byte (encoding key), then string bytes.
    /// R2007+: short (char count), then char_count*2 unicode bytes.
    fn read_text_unicode(&mut self) -> Result<String> {
        if self.version >= DxfVersion::AC1021 {
            // AC21: LE short for length, then length*2 bytes of unicode
            let text_length = self.read_short()?;
            if text_length <= 0 {
                return Ok(String::new());
            }
            let byte_len = (text_length as usize) * 2;
            let bytes = self.read_bytes(byte_len)?;
            let utf16: Vec<u16> = bytes
                .chunks_exact(2)
                .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
                .collect();
            return Ok(String::from_utf16_lossy(&utf16).replace('\0', ""));
        }

        // Pre-R2007: short (length), byte (encoding), then string
        let text_length = self.read_short()?;
        let _encoding_key = self.read_byte()?;
        if text_length <= 0 {
            return Ok(String::new());
        }

        let bytes = self.read_bytes(text_length as usize)?;
        Ok(String::from_utf8_lossy(&bytes).to_string())
    }

    fn read_time_span(&mut self) -> Result<(i32, i32)> {
        Ok((self.read_bit_long()?, self.read_bit_long()?))
    }

    fn read_uint(&mut self) -> Result<u32> {
        if self.bit_shift == 0 {
            return self.stream.read_u32::<LittleEndian>().map_err(Into::into);
        }
        let bytes = self.read_bytes(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    /// TV : Variable text
    /// Pre-R2007: bitshort length, then string bytes using encoding.
    /// R2007+: bitshort length, then length*2 unicode bytes.
    fn read_variable_text(&mut self) -> Result<String> {
        if self.version >= DxfVersion::AC1021 {
            let text_length = self.read_bit_short()?;
            if text_length <= 0 {
                return Ok(String::new());
            }
            let byte_len = (text_length as usize) * 2;
            let bytes = self.read_bytes(byte_len)?;
            let utf16: Vec<u16> = bytes
                .chunks_exact(2)
                .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
                .collect();
            return Ok(String::from_utf16_lossy(&utf16).replace('\0', ""));
        }

        // Pre-R2007: bitshort length, then string using encoding
        let length = self.read_bit_short()?;
        if length <= 0 {
            return Ok(String::new());
        }
        let bytes = self.read_bytes(length as usize)?;
        Ok(String::from_utf8_lossy(&bytes).replace('\0', ""))
    }

    /// ResetShift: resets bit_shift to 0, then reads 2 bytes and returns them as u16.
    fn reset_shift(&mut self) -> u16 {
        if self.bit_shift > 0 {
            self.bit_shift = 0;
        }

        let _ = self.advance_byte();
        let lo = self.last_byte as u16;
        let _ = self.advance_byte();
        lo | ((self.last_byte as u16) << 8)
    }

    /// Find the position of the string stream.
    fn set_position_by_flag(&mut self, position: u64) -> Result<u64> {
        self.set_position_in_bits(position)?;

        // String stream present bit (last bit in pre-handles section)
        let flag = self.read_bit()?;

        let start_position = position;
        if flag {
            // String stream present
            let (length, size) = self.apply_flag_to_position(position)?;
            let start_position = length - size;
            self.set_position_in_bits(start_position)?;
            Ok(start_position)
        } else {
            // Mark as empty
            self.is_empty = true;
            // Set position to end of stream
            let end = self.stream.seek(SeekFrom::End(0))?;
            self.stream.seek(SeekFrom::Start(end))?;
            Ok(start_position)
        }
    }
}
