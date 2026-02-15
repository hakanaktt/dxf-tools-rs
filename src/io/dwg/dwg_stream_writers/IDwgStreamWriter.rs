//! DWG stream writer trait — write-side equivalent of `IDwgStreamReader`.

use std::io::{Read, Seek, Write};

use crate::error::Result;
use crate::io::dwg::dwg_stream_readers::idwg_stream_reader::DwgReferenceType;
use crate::types::{Color, Transparency, Vector2, Vector3};

/// Trait object helper for `Write + Seek + Read`.
pub trait WriteSeek: Write + Seek + Read {}
impl<T: Write + Seek + Read> WriteSeek for T {}

/// Writer contract for DWG bit streams (mirror of `DwgStreamReader`).
pub trait DwgStreamWriter {
    // ---- position / stream management ----
    fn stream(&mut self) -> &mut dyn WriteSeek;
    fn position_in_bits(&self) -> i64;
    fn saved_position_in_bits(&self) -> i64;

    fn write_bytes(&mut self, bytes: &[u8]) -> Result<()>;
    fn write_bytes_offset(&mut self, bytes: &[u8], offset: usize, length: usize) -> Result<()>;

    fn write_int(&mut self, value: i32) -> Result<()>;
    fn write_object_type(&mut self, value: i16) -> Result<()>;
    fn write_raw_long(&mut self, value: i64) -> Result<()>;
    fn write_bit_double(&mut self, value: f64) -> Result<()>;
    fn write_bit_long(&mut self, value: i32) -> Result<()>;
    fn write_bit_long_long(&mut self, value: i64) -> Result<()>;
    fn write_variable_text(&mut self, value: &str) -> Result<()>;
    fn write_text_unicode(&mut self, value: &str) -> Result<()>;
    fn write_bit(&mut self, value: bool) -> Result<()>;
    fn write_2_bits(&mut self, value: u8) -> Result<()>;
    fn write_bit_short(&mut self, value: i16) -> Result<()>;

    fn write_date_time(&mut self, jdate: i32, msecs: i32) -> Result<()>;
    fn write_8_bit_julian_date(&mut self, jdate: i32, msecs: i32) -> Result<()>;
    fn write_time_span(&mut self, days: i32, msecs: i32) -> Result<()>;

    fn write_cm_color(&mut self, value: &Color) -> Result<()>;
    fn write_en_color(&mut self, color: &Color, transparency: &Transparency) -> Result<()>;
    fn write_en_color_book(
        &mut self,
        color: &Color,
        transparency: &Transparency,
        is_book_color: bool,
    ) -> Result<()>;

    fn write_2_bit_double(&mut self, value: &Vector2) -> Result<()>;
    fn write_3_bit_double(&mut self, value: &Vector3) -> Result<()>;
    fn write_2_raw_double(&mut self, value: &Vector2) -> Result<()>;

    fn write_byte(&mut self, value: u8) -> Result<()>;

    fn handle_reference(&mut self, handle: u64) -> Result<()>;
    fn handle_reference_typed(
        &mut self,
        ref_type: DwgReferenceType,
        handle: u64,
    ) -> Result<()>;

    fn write_spear_shift(&mut self) -> Result<()>;

    fn write_raw_short(&mut self, value: i16) -> Result<()>;
    fn write_raw_short_unsigned(&mut self, value: u16) -> Result<()>;
    fn write_raw_double(&mut self, value: f64) -> Result<()>;

    fn write_bit_thickness(&mut self, thickness: f64) -> Result<()>;
    fn write_bit_extrusion(&mut self, normal: &Vector3) -> Result<()>;

    fn write_bit_double_with_default(&mut self, def: f64, value: f64) -> Result<()>;
    fn write_2_bit_double_with_default(
        &mut self,
        def: &Vector2,
        value: &Vector2,
    ) -> Result<()>;
    fn write_3_bit_double_with_default(
        &mut self,
        def: &Vector3,
        value: &Vector3,
    ) -> Result<()>;

    fn reset_stream(&mut self) -> Result<()>;
    fn save_position_for_size(&mut self) -> Result<()>;
    fn set_position_in_bits(&mut self, pos_in_bits: i64) -> Result<()>;
    fn set_position_by_flag(&mut self, pos: i64) -> Result<()>;
    fn write_shift_value(&mut self) -> Result<()>;
}

/// File header writer trait.
pub trait DwgFileHeaderWriter {
    /// Offset that the handle section data starts at, relative to
    /// the objects section for versions < AC1018.
    fn handle_section_offset(&self) -> i32;

    /// Register a section with its stream data.
    fn add_section(
        &mut self,
        name: &str,
        stream: Vec<u8>,
        is_compressed: bool,
        decomp_size: usize,
    );

    /// Finalize: write all section data and file header to the output stream.
    fn write_file(&mut self) -> Result<()>;
}

/// Compressor trait (LZ77 variants).
pub trait Compressor {
    fn compress(
        &mut self,
        source: &[u8],
        offset: usize,
        total_size: usize,
        dest: &mut Vec<u8>,
    );
}
