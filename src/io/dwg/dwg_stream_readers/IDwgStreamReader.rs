use std::io::{Read, Seek};

use crate::error::Result;
use crate::types::{Color, Transparency, Vector2, Vector3};

/// Handle reference addressing mode in DWG streams.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DwgReferenceType {
    Absolute,
    Relative,
    SoftPointer,
    HardPointer,
    SoftOwnership,
    HardOwnership,
    Unknown(u8),
}

/// Generic DWG object type code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DwgObjectType(pub u16);

/// Reader contract for DWG bit streams.
pub trait DwgStreamReader {
    fn bit_shift(&self) -> u8;
    fn set_bit_shift(&mut self, value: u8);

    fn is_empty(&self) -> bool;

    fn position(&mut self) -> Result<u64>;
    fn set_position(&mut self, value: u64) -> Result<()>;

    fn position_in_bits(&mut self) -> Result<u64>;
    fn set_position_in_bits(&mut self, value: u64) -> Result<()>;

    fn stream(&mut self) -> &mut (dyn ReadSeek + '_);

    fn advance(&mut self, offset: usize) -> Result<()>;
    fn advance_byte(&mut self) -> Result<()>;

    fn handle_reference(&mut self) -> Result<u64>;
    fn handle_reference_from(&mut self, reference_handle: u64) -> Result<u64>;
    fn handle_reference_with_type(
        &mut self,
        reference_handle: u64,
    ) -> Result<(u64, DwgReferenceType)>;

    fn read_2_bit_double(&mut self) -> Result<Vector2>;
    fn read_2_bit_double_with_default(&mut self, default_values: Vector2) -> Result<Vector2>;
    fn read_2_bits(&mut self) -> Result<u8>;
    fn read_2_raw_double(&mut self) -> Result<Vector2>;

    fn read_3_bit_double(&mut self) -> Result<Vector3>;
    fn read_3_bit_double_with_default(&mut self, default_values: Vector3) -> Result<Vector3>;
    fn read_3_raw_double(&mut self) -> Result<Vector3>;

    fn read_8_bit_julian_date(&mut self) -> Result<(i32, i32)>;

    fn read_bit(&mut self) -> Result<bool>;
    fn read_bit_as_short(&mut self) -> Result<i16>;
    fn read_bit_double(&mut self) -> Result<f64>;
    fn read_bit_double_with_default(&mut self, default_value: f64) -> Result<f64>;
    fn read_bit_extrusion(&mut self) -> Result<Vector3>;
    fn read_bit_long(&mut self) -> Result<i32>;
    fn read_bit_long_long(&mut self) -> Result<i64>;
    fn read_bit_short(&mut self) -> Result<i16>;
    fn read_bit_short_as_bool(&mut self) -> Result<bool>;
    fn read_bit_thickness(&mut self) -> Result<f64>;

    fn read_byte(&mut self) -> Result<u8>;
    fn read_bytes(&mut self, length: usize) -> Result<Vec<u8>>;

    fn read_cm_color(&mut self, use_text_stream: bool) -> Result<Color>;
    fn read_color_by_index(&mut self) -> Result<Color>;

    fn read_date_time(&mut self) -> Result<(i32, i32)>;
    fn read_double(&mut self) -> Result<f64>;

    fn read_en_color(&mut self) -> Result<(Color, Transparency, bool)>;

    fn read_int(&mut self) -> Result<i32>;
    fn read_modular_char(&mut self) -> Result<u64>;
    fn read_modular_short(&mut self) -> Result<i32>;

    fn read_object_type(&mut self) -> Result<DwgObjectType>;

    fn read_raw_char(&mut self) -> Result<u8>;
    fn read_raw_long(&mut self) -> Result<i64>;
    fn read_raw_u_long(&mut self) -> Result<u64>;
    fn read_sentinel(&mut self) -> Result<[u8; 16]>;
    fn read_short(&mut self) -> Result<i16>;
    fn read_signed_modular_char(&mut self) -> Result<i64>;

    fn read_text_unicode(&mut self) -> Result<String>;
    fn read_time_span(&mut self) -> Result<(i32, i32)>;
    fn read_uint(&mut self) -> Result<u32>;
    fn read_variable_text(&mut self) -> Result<String>;

    fn reset_shift(&mut self) -> u16;
    fn set_position_by_flag(&mut self, position: u64) -> Result<u64>;
}

/// Helper trait alias for `Read + Seek` trait objects.
pub trait ReadSeek: Read + Seek {}

impl<T: Read + Seek> ReadSeek for T {}
