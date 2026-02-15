use std::io::{Read, Seek};

use crate::{
    error::Result,
    types::{Color, Transparency, Vector2, Vector3},
};

use super::{
    dwg_stream_reader_base::DwgStreamReaderBase,
    idwg_stream_reader::{DwgObjectType, DwgReferenceType, DwgStreamReader, ReadSeek},
};

/// Merges object data streams into one reader.
///
/// - Main/object stream: numeric and structural values
/// - Text stream: variable/unicode text payloads
/// - Handle stream: handle references
pub struct DwgMergedReader {
    main_reader: Box<dyn DwgStreamReader>,
    text_reader: Box<dyn DwgStreamReader>,
    handle_reader: Box<dyn DwgStreamReader>,
}

impl DwgMergedReader {
    pub fn new(
        main_reader: Box<dyn DwgStreamReader>,
        text_reader: Box<dyn DwgStreamReader>,
        handle_reader: Box<dyn DwgStreamReader>,
    ) -> Self {
        Self {
            main_reader,
            text_reader,
            handle_reader,
        }
    }

    pub fn create<R: Read + Seek + 'static>(stream: R) -> Result<DwgStreamReaderBase> {
        Ok(DwgStreamReaderBase::new(Box::new(stream)))
    }
}

impl DwgStreamReader for DwgMergedReader {
    fn bit_shift(&self) -> u8 {
        self.main_reader.bit_shift()
    }

    fn set_bit_shift(&mut self, value: u8) {
        self.main_reader.set_bit_shift(value);
    }

    fn is_empty(&self) -> bool {
        self.main_reader.is_empty()
    }

    fn position(&mut self) -> Result<u64> {
        self.main_reader.position()
    }

    fn set_position(&mut self, value: u64) -> Result<()> {
        self.main_reader.set_position(value)
    }

    fn position_in_bits(&mut self) -> Result<u64> {
        self.main_reader.position_in_bits()
    }

    fn set_position_in_bits(&mut self, value: u64) -> Result<()> {
        self.main_reader.set_position_in_bits(value)
    }

    fn stream(&mut self) -> &mut (dyn ReadSeek + '_) {
        self.main_reader.stream()
    }

    fn advance(&mut self, offset: usize) -> Result<()> {
        self.main_reader.advance(offset)
    }

    fn advance_byte(&mut self) -> Result<()> {
        self.main_reader.advance_byte()
    }

    fn handle_reference(&mut self) -> Result<u64> {
        self.handle_reader.handle_reference()
    }

    fn handle_reference_from(&mut self, reference_handle: u64) -> Result<u64> {
        self.handle_reader.handle_reference_from(reference_handle)
    }

    fn handle_reference_with_type(
        &mut self,
        reference_handle: u64,
    ) -> Result<(u64, DwgReferenceType)> {
        self.handle_reader
            .handle_reference_with_type(reference_handle)
    }

    fn read_2_bit_double(&mut self) -> Result<Vector2> {
        self.main_reader.read_2_bit_double()
    }

    fn read_2_bit_double_with_default(&mut self, default_values: Vector2) -> Result<Vector2> {
        self.main_reader
            .read_2_bit_double_with_default(default_values)
    }

    fn read_2_bits(&mut self) -> Result<u8> {
        self.main_reader.read_2_bits()
    }

    fn read_2_raw_double(&mut self) -> Result<Vector2> {
        self.main_reader.read_2_raw_double()
    }

    fn read_3_bit_double(&mut self) -> Result<Vector3> {
        self.main_reader.read_3_bit_double()
    }

    fn read_3_bit_double_with_default(&mut self, default_values: Vector3) -> Result<Vector3> {
        self.main_reader
            .read_3_bit_double_with_default(default_values)
    }

    fn read_3_raw_double(&mut self) -> Result<Vector3> {
        self.main_reader.read_3_raw_double()
    }

    fn read_8_bit_julian_date(&mut self) -> Result<(i32, i32)> {
        self.main_reader.read_8_bit_julian_date()
    }

    fn read_bit(&mut self) -> Result<bool> {
        self.main_reader.read_bit()
    }

    fn read_bit_as_short(&mut self) -> Result<i16> {
        self.main_reader.read_bit_as_short()
    }

    fn read_bit_double(&mut self) -> Result<f64> {
        self.main_reader.read_bit_double()
    }

    fn read_bit_double_with_default(&mut self, default_value: f64) -> Result<f64> {
        self.main_reader.read_bit_double_with_default(default_value)
    }

    fn read_bit_extrusion(&mut self) -> Result<Vector3> {
        self.main_reader.read_bit_extrusion()
    }

    fn read_bit_long(&mut self) -> Result<i32> {
        self.main_reader.read_bit_long()
    }

    fn read_bit_long_long(&mut self) -> Result<i64> {
        self.main_reader.read_bit_long_long()
    }

    fn read_bit_short(&mut self) -> Result<i16> {
        self.main_reader.read_bit_short()
    }

    fn read_bit_short_as_bool(&mut self) -> Result<bool> {
        self.main_reader.read_bit_short_as_bool()
    }

    fn read_bit_thickness(&mut self) -> Result<f64> {
        self.main_reader.read_bit_thickness()
    }

    fn read_byte(&mut self) -> Result<u8> {
        self.main_reader.read_byte()
    }

    fn read_bytes(&mut self, length: usize) -> Result<Vec<u8>> {
        self.main_reader.read_bytes(length)
    }

    fn read_cm_color(&mut self, use_text_stream: bool) -> Result<Color> {
        if !use_text_stream {
            return self.main_reader.read_cm_color(false);
        }

        let _color_index = self.main_reader.read_bit_short()?;
        let rgb = self.main_reader.read_bit_long()? as u32;
        let arr = rgb.to_le_bytes();

        let color = if rgb == 0xC000_0000 {
            Color::ByLayer
        } else if (rgb & 0x0100_0000) != 0 {
            Color::from_index(arr[0] as i16)
        } else {
            Color::from_rgb(arr[2], arr[1], arr[0])
        };

        let id = self.main_reader.read_byte()?;
        if (id & 1) == 1 {
            let _ = self.text_reader.read_variable_text()?;
        }
        if (id & 2) == 2 {
            let _ = self.text_reader.read_variable_text()?;
        }

        Ok(color)
    }

    fn read_color_by_index(&mut self) -> Result<Color> {
        self.main_reader.read_color_by_index()
    }

    fn read_date_time(&mut self) -> Result<(i32, i32)> {
        self.main_reader.read_date_time()
    }

    fn read_double(&mut self) -> Result<f64> {
        self.main_reader.read_double()
    }

    fn read_en_color(&mut self) -> Result<(Color, Transparency, bool)> {
        self.main_reader.read_en_color()
    }

    fn read_int(&mut self) -> Result<i32> {
        self.main_reader.read_int()
    }

    fn read_modular_char(&mut self) -> Result<u64> {
        self.main_reader.read_modular_char()
    }

    fn read_modular_short(&mut self) -> Result<i32> {
        self.main_reader.read_modular_short()
    }

    fn read_object_type(&mut self) -> Result<DwgObjectType> {
        self.main_reader.read_object_type()
    }

    fn read_raw_char(&mut self) -> Result<u8> {
        self.main_reader.read_raw_char()
    }

    fn read_raw_long(&mut self) -> Result<i64> {
        self.main_reader.read_raw_long()
    }

    fn read_raw_u_long(&mut self) -> Result<u64> {
        self.main_reader.read_raw_u_long()
    }

    fn read_sentinel(&mut self) -> Result<[u8; 16]> {
        self.main_reader.read_sentinel()
    }

    fn read_short(&mut self) -> Result<i16> {
        self.main_reader.read_short()
    }

    fn read_signed_modular_char(&mut self) -> Result<i64> {
        self.main_reader.read_signed_modular_char()
    }

    fn read_text_unicode(&mut self) -> Result<String> {
        if self.text_reader.is_empty() {
            return Ok(String::new());
        }
        self.text_reader.read_text_unicode()
    }

    fn read_time_span(&mut self) -> Result<(i32, i32)> {
        self.main_reader.read_time_span()
    }

    fn read_uint(&mut self) -> Result<u32> {
        self.main_reader.read_uint()
    }

    fn read_variable_text(&mut self) -> Result<String> {
        if self.text_reader.is_empty() {
            return Ok(String::new());
        }
        self.text_reader.read_variable_text()
    }

    fn reset_shift(&mut self) -> u16 {
        self.main_reader.reset_shift()
    }

    fn set_position_by_flag(&mut self, position: u64) -> Result<u64> {
        self.main_reader.set_position_by_flag(position)
    }
}
