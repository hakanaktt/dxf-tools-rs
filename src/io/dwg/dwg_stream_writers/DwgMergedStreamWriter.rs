//! Merged DWG stream writers for object data (main + text + handle streams).

use std::io::Cursor;

use crate::error::Result;
use crate::io::dwg::dwg_stream_readers::idwg_stream_reader::DwgReferenceType;
use crate::types::{Color, Transparency, Vector2, Vector3};

use super::idwg_stream_writer::{DwgStreamWriter, WriteSeek};

/// Helper to extract bytes from a boxed DwgStreamWriter whose inner stream
/// is a `Cursor<Vec<u8>>`. Uses `write_spear_shift` to flush, then reads
/// stream contents via a temporary helper.
fn extract_stream_bytes(writer: &mut dyn DwgStreamWriter) -> Result<Vec<u8>> {
    // Flush residual bits
    writer.write_spear_shift()?;
    let stream = writer.stream();
    let pos = stream.stream_position()?;
    stream.seek(std::io::SeekFrom::Start(0))?;
    let mut buf = Vec::with_capacity(pos as usize);
    std::io::Read::read_to_end(stream, &mut buf)?;
    Ok(buf)
}

fn stream_length(writer: &mut dyn DwgStreamWriter) -> Result<u64> {
    let stream = writer.stream();
    let pos = stream.stream_position()?;
    let end = stream.seek(std::io::SeekFrom::End(0))?;
    stream.seek(std::io::SeekFrom::Start(pos))?;
    Ok(end)
}

// ─── AC21+ merged writer (main + text + handle) ────────────────────

/// For R2007+ (AC21, AC24, AC27, AC32): three separate sub-streams
/// are written (main data, text, handles) then concatenated with
/// position-by-flag encoding.
pub struct DwgMergedStreamWriter {
    pub main_writer: Box<dyn DwgStreamWriter>,
    pub text_writer: Box<dyn DwgStreamWriter>,
    pub handle_writer: Box<dyn DwgStreamWriter>,
    saved_position: i64,
    saved_flag: bool,
}

impl DwgMergedStreamWriter {
    pub fn new(
        main: Box<dyn DwgStreamWriter>,
        text: Box<dyn DwgStreamWriter>,
        handle: Box<dyn DwgStreamWriter>,
    ) -> Self {
        Self {
            main_writer: main,
            text_writer: text,
            handle_writer: handle,
            saved_position: 0,
            saved_flag: false,
        }
    }
}

impl DwgStreamWriter for DwgMergedStreamWriter {
    fn stream(&mut self) -> &mut dyn WriteSeek {
        self.main_writer.stream()
    }

    fn position_in_bits(&self) -> i64 {
        self.main_writer.position_in_bits()
    }

    fn saved_position_in_bits(&self) -> i64 {
        self.saved_position
    }

    fn write_bytes(&mut self, bytes: &[u8]) -> Result<()> {
        self.main_writer.write_bytes(bytes)
    }

    fn write_bytes_offset(&mut self, bytes: &[u8], offset: usize, length: usize) -> Result<()> {
        self.main_writer.write_bytes_offset(bytes, offset, length)
    }

    fn write_int(&mut self, value: i32) -> Result<()> {
        self.main_writer.write_int(value)
    }

    fn write_object_type(&mut self, value: i16) -> Result<()> {
        self.main_writer.write_object_type(value)
    }

    fn write_raw_long(&mut self, value: i64) -> Result<()> {
        self.main_writer.write_raw_long(value)
    }

    fn write_bit_double(&mut self, value: f64) -> Result<()> {
        self.main_writer.write_bit_double(value)
    }

    fn write_bit_long(&mut self, value: i32) -> Result<()> {
        self.main_writer.write_bit_long(value)
    }

    fn write_bit_long_long(&mut self, value: i64) -> Result<()> {
        self.main_writer.write_bit_long_long(value)
    }

    /// Variable text goes to the text sub-stream.
    fn write_variable_text(&mut self, value: &str) -> Result<()> {
        self.text_writer.write_variable_text(value)
    }

    /// Text Unicode goes to the text sub-stream.
    fn write_text_unicode(&mut self, value: &str) -> Result<()> {
        self.text_writer.write_text_unicode(value)
    }

    fn write_bit(&mut self, value: bool) -> Result<()> {
        self.main_writer.write_bit(value)
    }

    fn write_2_bits(&mut self, value: u8) -> Result<()> {
        self.main_writer.write_2_bits(value)
    }

    fn write_bit_short(&mut self, value: i16) -> Result<()> {
        self.main_writer.write_bit_short(value)
    }

    fn write_date_time(&mut self, jdate: i32, msecs: i32) -> Result<()> {
        self.main_writer.write_date_time(jdate, msecs)
    }

    fn write_8_bit_julian_date(&mut self, jdate: i32, msecs: i32) -> Result<()> {
        self.main_writer.write_8_bit_julian_date(jdate, msecs)
    }

    fn write_time_span(&mut self, days: i32, msecs: i32) -> Result<()> {
        self.main_writer.write_time_span(days, msecs)
    }

    fn write_cm_color(&mut self, value: &Color) -> Result<()> {
        self.main_writer.write_cm_color(value)
    }

    fn write_en_color(&mut self, color: &Color, transparency: &Transparency) -> Result<()> {
        self.main_writer.write_en_color(color, transparency)
    }

    fn write_en_color_book(
        &mut self,
        color: &Color,
        transparency: &Transparency,
        is_book_color: bool,
    ) -> Result<()> {
        self.main_writer
            .write_en_color_book(color, transparency, is_book_color)
    }

    fn write_2_bit_double(&mut self, value: &Vector2) -> Result<()> {
        self.main_writer.write_2_bit_double(value)
    }

    fn write_3_bit_double(&mut self, value: &Vector3) -> Result<()> {
        self.main_writer.write_3_bit_double(value)
    }

    fn write_2_raw_double(&mut self, value: &Vector2) -> Result<()> {
        self.main_writer.write_2_raw_double(value)
    }

    fn write_byte(&mut self, value: u8) -> Result<()> {
        self.main_writer.write_byte(value)
    }

    /// Handle references go to the handle sub-stream.
    fn handle_reference(&mut self, handle: u64) -> Result<()> {
        self.handle_writer.handle_reference(handle)
    }

    fn handle_reference_typed(
        &mut self,
        ref_type: DwgReferenceType,
        handle: u64,
    ) -> Result<()> {
        self.handle_writer.handle_reference_typed(ref_type, handle)
    }

    fn write_spear_shift(&mut self) -> Result<()> {
        let main_size_bits = self.main_writer.position_in_bits();
        let text_size_bits = self.text_writer.position_in_bits();

        self.main_writer.write_spear_shift()?;

        if self.saved_flag {
            let mut main_text_total_bits = (main_size_bits + text_size_bits + 1) as i32;
            if text_size_bits > 0 {
                main_text_total_bits += 16;
                if text_size_bits >= 0x8000 {
                    main_text_total_bits += 16;
                    if text_size_bits >= 0x4000_0000 {
                        main_text_total_bits += 16;
                    }
                }
            }

            self.main_writer
                .set_position_in_bits(self.saved_position)?;
            self.main_writer.write_raw_long(main_text_total_bits as i64)?;
            self.main_writer.write_shift_value()?;
        }

        self.main_writer.set_position_in_bits(main_size_bits)?;

        if text_size_bits > 0 {
            let text_buf = extract_stream_bytes(&mut *self.text_writer)?;
            self.main_writer.write_bytes(&text_buf)?;
            self.main_writer.write_spear_shift()?;
            self.main_writer
                .set_position_in_bits(main_size_bits + text_size_bits)?;
            self.main_writer.set_position_by_flag(text_size_bits)?;
            self.main_writer.write_bit(true)?;
        } else {
            self.main_writer.write_bit(false)?;
        }

        let handle_buf = extract_stream_bytes(&mut *self.handle_writer)?;
        self.saved_position = self.main_writer.position_in_bits();
        self.main_writer.write_bytes(&handle_buf)?;
        self.main_writer.write_spear_shift()?;

        Ok(())
    }

    fn write_raw_short(&mut self, value: i16) -> Result<()> {
        self.main_writer.write_raw_short(value)
    }

    fn write_raw_short_unsigned(&mut self, value: u16) -> Result<()> {
        self.main_writer.write_raw_short_unsigned(value)
    }

    fn write_raw_double(&mut self, value: f64) -> Result<()> {
        self.main_writer.write_raw_double(value)
    }

    fn write_bit_thickness(&mut self, thickness: f64) -> Result<()> {
        self.main_writer.write_bit_thickness(thickness)
    }

    fn write_bit_extrusion(&mut self, normal: &Vector3) -> Result<()> {
        self.main_writer.write_bit_extrusion(normal)
    }

    fn write_bit_double_with_default(&mut self, def: f64, value: f64) -> Result<()> {
        self.main_writer.write_bit_double_with_default(def, value)
    }

    fn write_2_bit_double_with_default(
        &mut self,
        def: &Vector2,
        value: &Vector2,
    ) -> Result<()> {
        self.main_writer
            .write_2_bit_double_with_default(def, value)
    }

    fn write_3_bit_double_with_default(
        &mut self,
        def: &Vector3,
        value: &Vector3,
    ) -> Result<()> {
        self.main_writer
            .write_3_bit_double_with_default(def, value)
    }

    fn reset_stream(&mut self) -> Result<()> {
        self.main_writer.reset_stream()?;
        self.text_writer.reset_stream()?;
        self.handle_writer.reset_stream()?;
        Ok(())
    }

    fn save_position_for_size(&mut self) -> Result<()> {
        self.saved_flag = true;
        self.saved_position = self.main_writer.position_in_bits();
        self.main_writer.write_raw_long(0)
    }

    fn set_position_in_bits(&mut self, pos_in_bits: i64) -> Result<()> {
        self.main_writer.set_position_in_bits(pos_in_bits)
    }

    fn set_position_by_flag(&mut self, pos: i64) -> Result<()> {
        self.main_writer.set_position_by_flag(pos)
    }

    fn write_shift_value(&mut self) -> Result<()> {
        self.main_writer.write_shift_value()
    }
}

// ─── AC14 merged writer (main + handle, no separate text stream) ───

/// For pre-R2007 (AC12..AC18): text goes into main, only handle is separate.
pub struct DwgMergedStreamWriterAc14 {
    pub main_writer: Box<dyn DwgStreamWriter>,
    pub handle_writer: Box<dyn DwgStreamWriter>,
    saved_position: i64,
    saved_flag: bool,
}

impl DwgMergedStreamWriterAc14 {
    pub fn new(
        main: Box<dyn DwgStreamWriter>,
        handle: Box<dyn DwgStreamWriter>,
    ) -> Self {
        Self {
            main_writer: main,
            handle_writer: handle,
            saved_position: 0,
            saved_flag: false,
        }
    }
}

impl DwgStreamWriter for DwgMergedStreamWriterAc14 {
    fn stream(&mut self) -> &mut dyn WriteSeek {
        self.main_writer.stream()
    }

    fn position_in_bits(&self) -> i64 {
        self.main_writer.position_in_bits()
    }

    fn saved_position_in_bits(&self) -> i64 {
        self.saved_position
    }

    fn write_bytes(&mut self, bytes: &[u8]) -> Result<()> {
        self.main_writer.write_bytes(bytes)
    }

    fn write_bytes_offset(&mut self, bytes: &[u8], offset: usize, length: usize) -> Result<()> {
        self.main_writer.write_bytes_offset(bytes, offset, length)
    }

    fn write_int(&mut self, value: i32) -> Result<()> {
        self.main_writer.write_int(value)
    }

    fn write_object_type(&mut self, value: i16) -> Result<()> {
        self.main_writer.write_object_type(value)
    }

    fn write_raw_long(&mut self, value: i64) -> Result<()> {
        self.main_writer.write_raw_long(value)
    }

    fn write_bit_double(&mut self, value: f64) -> Result<()> {
        self.main_writer.write_bit_double(value)
    }

    fn write_bit_long(&mut self, value: i32) -> Result<()> {
        self.main_writer.write_bit_long(value)
    }

    fn write_bit_long_long(&mut self, value: i64) -> Result<()> {
        self.main_writer.write_bit_long_long(value)
    }

    /// Pre-R2007: text goes in main stream.
    fn write_variable_text(&mut self, value: &str) -> Result<()> {
        self.main_writer.write_variable_text(value)
    }

    fn write_text_unicode(&mut self, value: &str) -> Result<()> {
        self.main_writer.write_text_unicode(value)
    }

    fn write_bit(&mut self, value: bool) -> Result<()> {
        self.main_writer.write_bit(value)
    }

    fn write_2_bits(&mut self, value: u8) -> Result<()> {
        self.main_writer.write_2_bits(value)
    }

    fn write_bit_short(&mut self, value: i16) -> Result<()> {
        self.main_writer.write_bit_short(value)
    }

    fn write_date_time(&mut self, jdate: i32, msecs: i32) -> Result<()> {
        self.main_writer.write_date_time(jdate, msecs)
    }

    fn write_8_bit_julian_date(&mut self, jdate: i32, msecs: i32) -> Result<()> {
        self.main_writer.write_8_bit_julian_date(jdate, msecs)
    }

    fn write_time_span(&mut self, days: i32, msecs: i32) -> Result<()> {
        self.main_writer.write_time_span(days, msecs)
    }

    fn write_cm_color(&mut self, value: &Color) -> Result<()> {
        self.main_writer.write_cm_color(value)
    }

    fn write_en_color(&mut self, color: &Color, transparency: &Transparency) -> Result<()> {
        self.main_writer.write_en_color(color, transparency)
    }

    fn write_en_color_book(
        &mut self,
        color: &Color,
        transparency: &Transparency,
        is_book_color: bool,
    ) -> Result<()> {
        self.main_writer
            .write_en_color_book(color, transparency, is_book_color)
    }

    fn write_2_bit_double(&mut self, value: &Vector2) -> Result<()> {
        self.main_writer.write_2_bit_double(value)
    }

    fn write_3_bit_double(&mut self, value: &Vector3) -> Result<()> {
        self.main_writer.write_3_bit_double(value)
    }

    fn write_2_raw_double(&mut self, value: &Vector2) -> Result<()> {
        self.main_writer.write_2_raw_double(value)
    }

    fn write_byte(&mut self, value: u8) -> Result<()> {
        self.main_writer.write_byte(value)
    }

    fn handle_reference(&mut self, handle: u64) -> Result<()> {
        self.handle_writer.handle_reference(handle)
    }

    fn handle_reference_typed(
        &mut self,
        ref_type: DwgReferenceType,
        handle: u64,
    ) -> Result<()> {
        self.handle_writer.handle_reference_typed(ref_type, handle)
    }

    fn write_spear_shift(&mut self) -> Result<()> {
        let pos = self.main_writer.position_in_bits();

        if self.saved_flag {
            self.main_writer.write_spear_shift()?;
            self.main_writer
                .set_position_in_bits(self.saved_position)?;
            self.main_writer.write_raw_long(pos)?;
            self.main_writer.write_shift_value()?;
            self.main_writer.set_position_in_bits(pos)?;
        }

        let handle_buf = extract_stream_bytes(&mut *self.handle_writer)?;
        self.main_writer.write_bytes(&handle_buf)?;
        self.main_writer.write_spear_shift()?;

        Ok(())
    }

    fn write_raw_short(&mut self, value: i16) -> Result<()> {
        self.main_writer.write_raw_short(value)
    }

    fn write_raw_short_unsigned(&mut self, value: u16) -> Result<()> {
        self.main_writer.write_raw_short_unsigned(value)
    }

    fn write_raw_double(&mut self, value: f64) -> Result<()> {
        self.main_writer.write_raw_double(value)
    }

    fn write_bit_thickness(&mut self, thickness: f64) -> Result<()> {
        self.main_writer.write_bit_thickness(thickness)
    }

    fn write_bit_extrusion(&mut self, normal: &Vector3) -> Result<()> {
        self.main_writer.write_bit_extrusion(normal)
    }

    fn write_bit_double_with_default(&mut self, def: f64, value: f64) -> Result<()> {
        self.main_writer.write_bit_double_with_default(def, value)
    }

    fn write_2_bit_double_with_default(
        &mut self,
        def: &Vector2,
        value: &Vector2,
    ) -> Result<()> {
        self.main_writer
            .write_2_bit_double_with_default(def, value)
    }

    fn write_3_bit_double_with_default(
        &mut self,
        def: &Vector3,
        value: &Vector3,
    ) -> Result<()> {
        self.main_writer
            .write_3_bit_double_with_default(def, value)
    }

    fn reset_stream(&mut self) -> Result<()> {
        self.main_writer.reset_stream()?;
        self.handle_writer.reset_stream()?;
        Ok(())
    }

    fn save_position_for_size(&mut self) -> Result<()> {
        self.saved_flag = true;
        self.saved_position = self.main_writer.position_in_bits();
        self.main_writer.write_raw_long(0)
    }

    fn set_position_in_bits(&mut self, pos_in_bits: i64) -> Result<()> {
        self.main_writer.set_position_in_bits(pos_in_bits)
    }

    fn set_position_by_flag(&mut self, pos: i64) -> Result<()> {
        self.main_writer.set_position_by_flag(pos)
    }

    fn write_shift_value(&mut self) -> Result<()> {
        self.main_writer.write_shift_value()
    }
}
