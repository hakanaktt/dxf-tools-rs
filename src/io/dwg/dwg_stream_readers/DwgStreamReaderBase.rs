use std::io::{Cursor, Read, Seek, SeekFrom};

use byteorder::{LittleEndian, ReadBytesExt};

use crate::error::{DxfError, Result};
use crate::types::{Color, DxfVersion, Transparency, Vector2, Vector3};

use super::idwg_stream_reader::{DwgObjectType, DwgReferenceType, DwgStreamReader, ReadSeek};

/// Shared implementation for DWG bit-stream readers.
pub struct DwgStreamReaderBase {
    stream: Box<dyn ReadSeek>,
    bit_shift: u8,
    is_empty: bool,
    last_byte: u8,
    text_stream: Option<Cursor<Vec<u8>>>,
}

impl DwgStreamReaderBase {
    pub fn new(stream: Box<dyn ReadSeek>) -> Self {
        Self {
            stream,
            bit_shift: 0,
            is_empty: false,
            last_byte: 0,
            text_stream: None,
        }
    }

    pub fn get_stream_handler<R: Read + Seek + 'static>(
        _version: DxfVersion,
        stream: R,
    ) -> Self {
        Self::new(Box::new(stream))
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

    fn read_aligned_byte(&mut self) -> Result<u8> {
        if self.bit_shift == 0 {
            let value = self.stream.read_u8()?;
            self.last_byte = value;
            return Ok(value);
        }

        let high = self.last_byte << self.bit_shift;
        let next = self.stream.read_u8()?;
        self.last_byte = next;
        let low = next >> (8 - self.bit_shift);
        Ok(high | low)
    }

    fn read_handle(&mut self) -> Result<u64> {
        self.handle_reference()
    }

    fn read_3_bits(&mut self) -> Result<u8> {
        let mut value = 0u8;
        if self.read_bit()? {
            value |= 0b100;
        }
        if self.read_bit()? {
            value |= 0b010;
        }
        if self.read_bit()? {
            value |= 0b001;
        }
        Ok(value)
    }

    fn apply_flag_to_position(&mut self, position: u64) -> Result<u64> {
        self.set_position_by_flag(position)
    }

    fn apply_shift_to_las_byte(value: u8, shift: u8) -> u8 {
        value << (shift & 7)
    }

    fn apply_shift_to_arr(bytes: &mut [u8], shift: u8) {
        let shift = shift & 7;
        if shift == 0 || bytes.is_empty() {
            return;
        }

        let mut prev = 0u8;
        for b in bytes.iter_mut() {
            let current = *b;
            *b = (current << shift) | (prev >> (8 - shift));
            prev = current;
        }
    }

    fn julian_to_date(days: i32, millis: i32) -> (i32, i32) {
        (days, millis)
    }

    fn throw_exception(message: &str) -> DxfError {
        DxfError::Parse(message.to_string())
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

    fn position_in_bits(&mut self) -> Result<u64> {
        Ok(self.position()? * 8 + u64::from(self.bit_shift))
    }

    fn set_position_in_bits(&mut self, value: u64) -> Result<()> {
        let byte_pos = value / 8;
        let shift = (value % 8) as u8;
        self.stream.seek(SeekFrom::Start(byte_pos))?;
        self.bit_shift = shift;
        if shift > 0 {
            self.last_byte = self.stream.read_u8()?;
            self.stream.seek(SeekFrom::Current(-1))?;
        }
        Ok(())
    }

    fn stream(&mut self) -> &mut (dyn ReadSeek + '_) {
        &mut *self.stream
    }

    fn advance(&mut self, offset: usize) -> Result<()> {
        self.stream.seek(SeekFrom::Current(offset as i64))?;
        self.bit_shift = 0;
        Ok(())
    }

    fn advance_byte(&mut self) -> Result<()> {
        self.last_byte = self.stream.read_u8()?;
        Ok(())
    }

    fn handle_reference(&mut self) -> Result<u64> {
        self.read_modular_char()
    }

    fn handle_reference_from(&mut self, reference_handle: u64) -> Result<u64> {
        let (value, _kind) = self.handle_reference_with_type(reference_handle)?;
        Ok(value)
    }

    fn handle_reference_with_type(
        &mut self,
        reference_handle: u64,
    ) -> Result<(u64, DwgReferenceType)> {
        let code = self.read_2_bits()?;
        let offset = self.read_modular_char()?;
        let (resolved, kind) = match code {
            0 => (offset, DwgReferenceType::Absolute),
            1 => (reference_handle.wrapping_add(offset), DwgReferenceType::Relative),
            2 => (reference_handle.wrapping_sub(offset), DwgReferenceType::Relative),
            3 => (offset, DwgReferenceType::HardPointer),
            other => (offset, DwgReferenceType::Unknown(other)),
        };
        Ok((resolved, kind))
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
            self.bit_shift = 1;
            return Ok((self.last_byte & 0x80) == 0x80);
        }

        let value = ((self.last_byte << self.bit_shift) & 0x80) == 0x80;
        self.bit_shift = (self.bit_shift + 1) & 7;
        Ok(value)
    }

    fn read_bit_as_short(&mut self) -> Result<i16> {
        Ok(if self.read_bit()? { 1 } else { 0 })
    }

    fn read_bit_double(&mut self) -> Result<f64> {
        let code = self.read_2_bits()?;
        match code {
            0 => self.read_double(),
            1 => Ok(1.0),
            2 => Ok(0.0),
            _ => Err(DxfError::Parse("Invalid bitdouble code".to_string())),
        }
    }

    fn read_bit_double_with_default(&mut self, default_value: f64) -> Result<f64> {
        match self.read_2_bits()? {
            0 => self.read_double(),
            1 => Ok(default_value),
            2 => Ok(0.0),
            _ => Ok(default_value),
        }
    }

    fn read_bit_extrusion(&mut self) -> Result<Vector3> {
        if self.read_bit()? {
            Ok(Vector3::new(0.0, 0.0, 1.0))
        } else {
            self.read_3_bit_double()
        }
    }

    fn read_bit_long(&mut self) -> Result<i32> {
        match self.read_2_bits()? {
            0 => self.read_int(),
            1 => Ok(self.read_raw_char()? as i32),
            2 => Ok(0),
            _ => Err(DxfError::Parse("Invalid bitlong code".to_string())),
        }
    }

    fn read_bit_long_long(&mut self) -> Result<i64> {
        self.stream.read_i64::<LittleEndian>().map_err(Into::into)
    }

    fn read_bit_short(&mut self) -> Result<i16> {
        match self.read_2_bits()? {
            0 => self.read_short(),
            1 => Ok(self.read_raw_char()? as i16),
            2 => Ok(0),
            3 => Ok(256),
            _ => unreachable!(),
        }
    }

    fn read_bit_short_as_bool(&mut self) -> Result<bool> {
        Ok(self.read_bit_short()? != 0)
    }

    fn read_bit_thickness(&mut self) -> Result<f64> {
        if self.read_bit()? {
            Ok(0.0)
        } else {
            self.read_bit_double()
        }
    }

    fn read_byte(&mut self) -> Result<u8> {
        self.read_aligned_byte()
    }

    fn read_bytes(&mut self, length: usize) -> Result<Vec<u8>> {
        let mut data = vec![0u8; length];
        if self.bit_shift == 0 {
            self.stream.read_exact(&mut data)?;
            return Ok(data);
        }

        for value in &mut data {
            *value = self.read_aligned_byte()?;
        }
        Ok(data)
    }

    fn read_cm_color(&mut self, _use_text_stream: bool) -> Result<Color> {
        let index = self.read_bit_short()?;
        Ok(Color::from_index(index))
    }

    fn read_color_by_index(&mut self) -> Result<Color> {
        Ok(Color::from_index(self.read_short()?))
    }

    fn read_date_time(&mut self) -> Result<(i32, i32)> {
        Ok((self.read_bit_long()?, self.read_bit_long()?))
    }

    fn read_double(&mut self) -> Result<f64> {
        self.stream.read_f64::<LittleEndian>().map_err(Into::into)
    }

    fn read_en_color(&mut self) -> Result<(Color, Transparency, bool)> {
        let color = self.read_cm_color(false)?;
        Ok((color, Transparency::OPAQUE, false))
    }

    fn read_int(&mut self) -> Result<i32> {
        self.stream.read_i32::<LittleEndian>().map_err(Into::into)
    }

    fn read_modular_char(&mut self) -> Result<u64> {
        let mut value: u64 = 0;
        let mut shift = 0;
        loop {
            let byte = self.read_raw_char()?;
            value |= u64::from(byte & 0x7F) << shift;
            if (byte & 0x80) == 0 {
                break;
            }
            shift += 7;
            if shift > 63 {
                return Err(DxfError::Parse("Invalid modular char".to_string()));
            }
        }
        Ok(value)
    }

    fn read_modular_short(&mut self) -> Result<i32> {
        Ok(self.read_modular_char()? as i32)
    }

    fn read_object_type(&mut self) -> Result<DwgObjectType> {
        Ok(DwgObjectType(self.read_bit_short()? as u16))
    }

    fn read_raw_char(&mut self) -> Result<u8> {
        self.stream.read_u8().map_err(Into::into)
    }

    fn read_raw_long(&mut self) -> Result<i64> {
        self.stream.read_i32::<LittleEndian>().map(|v| v as i64).map_err(Into::into)
    }

    fn read_raw_u_long(&mut self) -> Result<u64> {
        self.stream.read_u32::<LittleEndian>().map(|v| v as u64).map_err(Into::into)
    }

    fn read_sentinel(&mut self) -> Result<[u8; 16]> {
        let mut sentinel = [0u8; 16];
        self.stream.read_exact(&mut sentinel)?;
        Ok(sentinel)
    }

    fn read_short(&mut self) -> Result<i16> {
        self.stream.read_i16::<LittleEndian>().map_err(Into::into)
    }

    fn read_signed_modular_char(&mut self) -> Result<i64> {
        let unsigned = self.read_modular_char()?;
        let sign = (unsigned & 1) != 0;
        let magnitude = (unsigned >> 1) as i64;
        Ok(if sign { -magnitude } else { magnitude })
    }

    fn read_text_unicode(&mut self) -> Result<String> {
        let length = self.read_bit_short()?;
        if length <= 0 {
            return Ok(String::new());
        }

        let char_count = length as usize;
        if let Some(ref mut text_stream) = self.text_stream {
            let mut bytes = vec![0u8; char_count * 2];
            text_stream.read_exact(&mut bytes)?;
            let utf16: Vec<u16> = bytes
                .chunks_exact(2)
                .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
                .collect();
            Ok(String::from_utf16_lossy(&utf16))
        } else {
            let mut bytes = vec![0u8; char_count];
            self.stream.read_exact(&mut bytes)?;
            Ok(String::from_utf8_lossy(&bytes).to_string())
        }
    }

    fn read_time_span(&mut self) -> Result<(i32, i32)> {
        Ok((self.read_bit_long()?, self.read_bit_long()?))
    }

    fn read_uint(&mut self) -> Result<u32> {
        self.stream.read_u32::<LittleEndian>().map_err(Into::into)
    }

    fn read_variable_text(&mut self) -> Result<String> {
        self.read_text_unicode()
    }

    fn reset_shift(&mut self) -> u16 {
        let previous = self.bit_shift as u16;
        self.bit_shift = 0;
        previous
    }

    fn set_position_by_flag(&mut self, position: u64) -> Result<u64> {
        self.set_position_in_bits(position)?;
        let has_stream = self.read_bit()?;
        if !has_stream {
            self.is_empty = true;
            let end = self.stream.seek(SeekFrom::End(0))?;
            self.stream.seek(SeekFrom::Start(end))?;
            return Ok(end * 8);
        }

        self.ensure_text_stream()?;
        Ok(position)
    }
}
