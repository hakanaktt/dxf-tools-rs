//! Base DWG bit-stream writer (mirrors `DwgStreamReaderBase`).
//!
//! All version-specific writers delegate to or override methods here.

use std::io::{Cursor, Read, Seek, SeekFrom, Write};

use crate::error::{DxfError, Result};
use crate::io::dwg::dwg_stream_readers::idwg_stream_reader::DwgReferenceType;
use crate::types::{Color, DxfVersion, Transparency, Vector2, Vector3};

use super::idwg_stream_writer::{DwgStreamWriter, WriteSeek};

/// Shared implementation for all DWG bit-stream writers.
pub struct DwgStreamWriterBase {
    stream: Box<dyn WriteSeek>,
    pub version: DxfVersion,
    pub encoding_name: String,
    bit_shift: i32,
    last_byte: u8,
}

impl DwgStreamWriterBase {
    pub fn new(stream: Box<dyn WriteSeek>, encoding_name: &str) -> Self {
        Self {
            stream,
            version: DxfVersion::Unknown,
            encoding_name: encoding_name.to_string(),
            bit_shift: 0,
            last_byte: 0,
        }
    }

    /// Factory: create the appropriate writer for the given version.
    pub fn get_stream_writer(
        version: DxfVersion,
        stream: Box<dyn WriteSeek>,
        encoding_name: &str,
    ) -> Box<dyn DwgStreamWriter> {
        match version {
            DxfVersion::AC1012 | DxfVersion::AC1014 => {
                let mut w = DwgStreamWriterBase::new(stream, encoding_name);
                w.version = version;
                Box::new(DwgStreamWriterAc12 { inner: w })
            }
            DxfVersion::AC1015 => {
                let mut w = DwgStreamWriterBase::new(stream, encoding_name);
                w.version = version;
                Box::new(DwgStreamWriterAc15 {
                    inner: DwgStreamWriterAc12 { inner: w },
                })
            }
            DxfVersion::AC1018 => {
                let mut w = DwgStreamWriterBase::new(stream, encoding_name);
                w.version = version;
                Box::new(DwgStreamWriterAc18 {
                    inner: DwgStreamWriterAc15 {
                        inner: DwgStreamWriterAc12 { inner: w },
                    },
                })
            }
            DxfVersion::AC1021 => {
                let mut w = DwgStreamWriterBase::new(stream, encoding_name);
                w.version = version;
                Box::new(DwgStreamWriterAc21 {
                    inner: DwgStreamWriterAc18 {
                        inner: DwgStreamWriterAc15 {
                            inner: DwgStreamWriterAc12 { inner: w },
                        },
                    },
                })
            }
            DxfVersion::AC1024 | DxfVersion::AC1027 | DxfVersion::AC1032 => {
                let mut w = DwgStreamWriterBase::new(stream, encoding_name);
                w.version = version;
                Box::new(DwgStreamWriterAc24 {
                    inner: DwgStreamWriterAc21 {
                        inner: DwgStreamWriterAc18 {
                            inner: DwgStreamWriterAc15 {
                                inner: DwgStreamWriterAc12 { inner: w },
                            },
                        },
                    },
                })
            }
            _ => panic!("DWG version not supported for writing: {:?}", version),
        }
    }

    /// Factory: create a merged writer for the given version.
    pub fn get_merged_writer(
        version: DxfVersion,
        stream: Box<dyn WriteSeek>,
        encoding_name: &str,
    ) -> Box<dyn DwgStreamWriter> {
        match version {
            DxfVersion::AC1012 | DxfVersion::AC1014 => {
                let main = Self::get_stream_writer(version, stream, encoding_name);
                let handle = Self::get_stream_writer(
                    version,
                    Box::new(Cursor::new(Vec::new())),
                    encoding_name,
                );
                Box::new(super::dwg_merged_stream_writer::DwgMergedStreamWriterAc14::new(
                    main, handle,
                ))
            }
            DxfVersion::AC1015 => {
                let main = Self::get_stream_writer(version, stream, encoding_name);
                let handle = Self::get_stream_writer(
                    version,
                    Box::new(Cursor::new(Vec::new())),
                    encoding_name,
                );
                Box::new(super::dwg_merged_stream_writer::DwgMergedStreamWriterAc14::new(
                    main, handle,
                ))
            }
            DxfVersion::AC1018 => {
                let main = Self::get_stream_writer(version, stream, encoding_name);
                let handle = Self::get_stream_writer(
                    version,
                    Box::new(Cursor::new(Vec::new())),
                    encoding_name,
                );
                Box::new(super::dwg_merged_stream_writer::DwgMergedStreamWriterAc14::new(
                    main, handle,
                ))
            }
            DxfVersion::AC1021 => {
                let main = Self::get_stream_writer(version, stream, encoding_name);
                let text = Self::get_stream_writer(
                    version,
                    Box::new(Cursor::new(Vec::new())),
                    encoding_name,
                );
                let handle = Self::get_stream_writer(
                    version,
                    Box::new(Cursor::new(Vec::new())),
                    encoding_name,
                );
                Box::new(super::dwg_merged_stream_writer::DwgMergedStreamWriter::new(
                    main, text, handle,
                ))
            }
            DxfVersion::AC1024 | DxfVersion::AC1027 | DxfVersion::AC1032 => {
                let main = Self::get_stream_writer(version, stream, encoding_name);
                let text = Self::get_stream_writer(
                    version,
                    Box::new(Cursor::new(Vec::new())),
                    encoding_name,
                );
                let handle = Self::get_stream_writer(
                    version,
                    Box::new(Cursor::new(Vec::new())),
                    encoding_name,
                );
                Box::new(super::dwg_merged_stream_writer::DwgMergedStreamWriter::new(
                    main, text, handle,
                ))
            }
            _ => panic!("DWG version not supported for merged writing: {:?}", version),
        }
    }

    fn reset_shift(&mut self) {
        self.bit_shift = 0;
        self.last_byte = 0;
    }

    fn write_3_bits(&mut self, value: u8) -> Result<()> {
        self.write_bit_impl((value & 4) != 0)?;
        self.write_bit_impl((value & 2) != 0)?;
        self.write_bit_impl((value & 1) != 0)?;
        Ok(())
    }

    // ---- Internal bit-level primitives ----

    fn write_bit_impl(&mut self, value: bool) -> Result<()> {
        if self.bit_shift < 7 {
            if value {
                self.last_byte |= 1 << (7 - self.bit_shift);
            }
            self.bit_shift += 1;
            return Ok(());
        }
        if value {
            self.last_byte |= 1;
        }
        self.stream.write_all(&[self.last_byte])?;
        self.reset_shift();
        Ok(())
    }

    fn write_2_bits_impl(&mut self, value: u8) -> Result<()> {
        if self.bit_shift < 6 {
            self.last_byte |= value << (6 - self.bit_shift);
            self.bit_shift += 2;
        } else if self.bit_shift == 6 {
            self.last_byte |= value;
            self.stream.write_all(&[self.last_byte])?;
            self.reset_shift();
        } else {
            // bit_shift == 7
            self.last_byte |= value >> 1;
            self.stream.write_all(&[self.last_byte])?;
            self.last_byte = value << 7;
            self.bit_shift = 1;
        }
        Ok(())
    }

    fn write_byte_impl(&mut self, value: u8) -> Result<()> {
        if self.bit_shift == 0 {
            self.stream.write_all(&[value])?;
            return Ok(());
        }
        let shift = 8 - self.bit_shift;
        self.stream
            .write_all(&[self.last_byte | (value >> self.bit_shift as u32)])?;
        self.last_byte = value << shift as u32;
        Ok(())
    }

    fn write_bytes_impl(&mut self, arr: &[u8]) -> Result<()> {
        if self.bit_shift == 0 {
            for &b in arr {
                self.stream.write_all(&[b])?;
            }
            return Ok(());
        }
        let num = 8 - self.bit_shift;
        for &b in arr {
            self.stream
                .write_all(&[self.last_byte | (b >> self.bit_shift as u32)])?;
            self.last_byte = b << num as u32;
        }
        Ok(())
    }

    fn write_bytes_offset_impl(
        &mut self,
        arr: &[u8],
        initial_index: usize,
        length: usize,
    ) -> Result<()> {
        if self.bit_shift == 0 {
            for i in 0..length {
                self.stream.write_all(&[arr[initial_index + i]])?;
            }
            return Ok(());
        }
        let num = 8 - self.bit_shift;
        for i in 0..length {
            let b = arr[initial_index + i];
            self.stream
                .write_all(&[self.last_byte | (b >> self.bit_shift as u32)])?;
            self.last_byte = b << num as u32;
        }
        Ok(())
    }

    fn write_bit_short_impl(&mut self, value: i16) -> Result<()> {
        if value == 0 {
            self.write_2_bits_impl(2)?;
        } else if value > 0 && value < 256 {
            self.write_2_bits_impl(1)?;
            self.write_byte_impl(value as u8)?;
        } else if value == 256 {
            self.write_2_bits_impl(3)?;
        } else {
            self.write_2_bits_impl(0)?;
            self.write_byte_impl(value as u8)?;
            self.write_byte_impl((value >> 8) as u8)?;
        }
        Ok(())
    }

    fn write_bit_double_impl(&mut self, value: f64) -> Result<()> {
        if value == 0.0 {
            self.write_2_bits_impl(2)?;
            return Ok(());
        }
        if value == 1.0 {
            self.write_2_bits_impl(1)?;
            return Ok(());
        }
        self.write_2_bits_impl(0)?;
        self.write_bytes_impl(&value.to_le_bytes())?;
        Ok(())
    }

    fn write_bit_long_impl(&mut self, value: i32) -> Result<()> {
        if value == 0 {
            self.write_2_bits_impl(2)?;
            return Ok(());
        }
        if value > 0 && value < 256 {
            self.write_2_bits_impl(1)?;
            self.write_byte_impl(value as u8)?;
            return Ok(());
        }
        self.write_2_bits_impl(0)?;
        self.write_byte_impl(value as u8)?;
        self.write_byte_impl((value >> 8) as u8)?;
        self.write_byte_impl((value >> 16) as u8)?;
        self.write_byte_impl((value >> 24) as u8)?;
        Ok(())
    }

    fn write_bit_long_long_impl(&mut self, value: i64) -> Result<()> {
        let mut size: u8 = 0;
        let unsigned_value = value as u64;
        let mut hold = unsigned_value;
        while hold != 0 {
            hold >>= 8;
            size += 1;
        }
        self.write_3_bits(size)?;
        hold = unsigned_value;
        for _ in 0..size {
            self.write_byte_impl((hold & 0xFF) as u8)?;
            hold >>= 8;
        }
        Ok(())
    }

    fn write_variable_text_impl(&mut self, value: &str) -> Result<()> {
        if value.is_empty() {
            self.write_bit_short_impl(0)?;
            return Ok(());
        }
        let bytes = value.as_bytes();
        self.write_bit_short_impl(bytes.len() as i16)?;
        self.write_bytes_impl(bytes)?;
        Ok(())
    }

    fn write_text_unicode_impl(&mut self, value: &str) -> Result<()> {
        let bytes = value.as_bytes();
        self.write_raw_short_unsigned_impl((bytes.len() as u16) + 1)?;
        self.stream.write_all(bytes)?;
        self.stream.write_all(&[0])?;
        Ok(())
    }

    fn write_raw_short_impl(&mut self, value: i16) -> Result<()> {
        self.write_bytes_impl(&value.to_le_bytes())?;
        Ok(())
    }

    fn write_raw_short_unsigned_impl(&mut self, value: u16) -> Result<()> {
        self.write_bytes_impl(&value.to_le_bytes())?;
        Ok(())
    }

    fn write_raw_double_impl(&mut self, value: f64) -> Result<()> {
        self.write_bytes_impl(&value.to_le_bytes())?;
        Ok(())
    }

    fn write_raw_long_impl(&mut self, value: i64) -> Result<()> {
        self.write_bytes_impl(&(value as i32).to_le_bytes())?;
        Ok(())
    }

    fn write_int_impl(&mut self, value: i32) -> Result<()> {
        self.write_bytes_impl(&value.to_le_bytes())?;
        Ok(())
    }

    fn handle_reference_impl(
        &mut self,
        ref_type: DwgReferenceType,
        handle: u64,
    ) -> Result<()> {
        let b = (ref_type as u8) << 4;

        if handle == 0 {
            self.write_byte_impl(b)?;
        } else if handle < 0x100 {
            self.write_byte_impl(b | 1)?;
            self.write_byte_impl(handle as u8)?;
        } else if handle < 0x10000 {
            self.write_byte_impl(b | 2)?;
            self.write_byte_impl((handle >> 8) as u8)?;
            self.write_byte_impl(handle as u8)?;
        } else if handle < 0x100_0000 {
            self.write_byte_impl(b | 3)?;
            self.write_byte_impl((handle >> 16) as u8)?;
            self.write_byte_impl((handle >> 8) as u8)?;
            self.write_byte_impl(handle as u8)?;
        } else if handle < 0x1_0000_0000 {
            self.write_byte_impl(b | 4)?;
            self.write_byte_impl((handle >> 24) as u8)?;
            self.write_byte_impl((handle >> 16) as u8)?;
            self.write_byte_impl((handle >> 8) as u8)?;
            self.write_byte_impl(handle as u8)?;
        } else if handle < 0x100_0000_0000 {
            self.write_byte_impl(b | 5)?;
            self.write_byte_impl((handle >> 32) as u8)?;
            self.write_byte_impl((handle >> 24) as u8)?;
            self.write_byte_impl((handle >> 16) as u8)?;
            self.write_byte_impl((handle >> 8) as u8)?;
            self.write_byte_impl(handle as u8)?;
        } else if handle < 0x1_0000_0000_0000 {
            self.write_byte_impl(b | 6)?;
            self.write_byte_impl((handle >> 40) as u8)?;
            self.write_byte_impl((handle >> 32) as u8)?;
            self.write_byte_impl((handle >> 24) as u8)?;
            self.write_byte_impl((handle >> 16) as u8)?;
            self.write_byte_impl((handle >> 8) as u8)?;
            self.write_byte_impl(handle as u8)?;
        } else if handle < 0x100_0000_0000_0000 {
            self.write_byte_impl(b | 7)?;
            self.write_byte_impl((handle >> 48) as u8)?;
            self.write_byte_impl((handle >> 40) as u8)?;
            self.write_byte_impl((handle >> 32) as u8)?;
            self.write_byte_impl((handle >> 24) as u8)?;
            self.write_byte_impl((handle >> 16) as u8)?;
            self.write_byte_impl((handle >> 8) as u8)?;
            self.write_byte_impl(handle as u8)?;
        } else {
            self.write_byte_impl(b | 8)?;
            self.write_byte_impl((handle >> 56) as u8)?;
            self.write_byte_impl((handle >> 48) as u8)?;
            self.write_byte_impl((handle >> 40) as u8)?;
            self.write_byte_impl((handle >> 32) as u8)?;
            self.write_byte_impl((handle >> 24) as u8)?;
            self.write_byte_impl((handle >> 16) as u8)?;
            self.write_byte_impl((handle >> 8) as u8)?;
            self.write_byte_impl(handle as u8)?;
        }
        Ok(())
    }

    fn write_spear_shift_impl(&mut self) -> Result<()> {
        if self.bit_shift > 0 {
            for _ in self.bit_shift..8 {
                self.write_bit_impl(false)?;
            }
        }
        Ok(())
    }

    fn write_cm_color_impl(&mut self, value: &Color) -> Result<()> {
        // R15 and earlier: BS color index
        let index = match value {
            Color::ByLayer => 256,
            Color::ByBlock => 0,
            Color::Index(i) => *i as i16,
            Color::Rgb { .. } => value.approximate_index(),
        };
        self.write_bit_short_impl(index)?;
        Ok(())
    }

    fn write_en_color_impl(&mut self, color: &Color, _transparency: &Transparency) -> Result<()> {
        self.write_cm_color_impl(color)
    }

    fn write_en_color_book_impl(
        &mut self,
        color: &Color,
        transparency: &Transparency,
        _is_book_color: bool,
    ) -> Result<()> {
        self.write_cm_color_impl(color)
    }

    fn write_bit_thickness_impl(&mut self, thickness: f64) -> Result<()> {
        // R13-R14: BD
        self.write_bit_double_impl(thickness)
    }

    fn write_bit_extrusion_impl(&mut self, normal: &Vector3) -> Result<()> {
        // R13-R14: 3BD
        self.write_bit_double_impl(normal.x)?;
        self.write_bit_double_impl(normal.y)?;
        self.write_bit_double_impl(normal.z)?;
        Ok(())
    }

    fn write_bit_double_with_default_impl(&mut self, def: f64, value: f64) -> Result<()> {
        if def == value {
            // 00 — use default
            self.write_2_bits_impl(0)?;
            return Ok(());
        }

        let def_bytes = def.to_le_bytes();
        let value_bytes = value.to_le_bytes();

        // Compare symmetrically from both ends
        let mut first = 0usize;
        let mut last = 7i32;
        while last >= 0 && def_bytes[last as usize] == value_bytes[last as usize] {
            first += 1;
            last -= 1;
        }

        if first >= 4 {
            // 01 — patch first 4 bytes
            self.write_2_bits_impl(1)?;
            self.write_bytes_offset_impl(&value_bytes, 0, 4)?;
        } else if first >= 2 {
            // 10 — patch bytes 4,5 + first 4
            self.write_2_bits_impl(2)?;
            self.write_byte_impl(value_bytes[4])?;
            self.write_byte_impl(value_bytes[5])?;
            self.write_byte_impl(value_bytes[0])?;
            self.write_byte_impl(value_bytes[1])?;
            self.write_byte_impl(value_bytes[2])?;
            self.write_byte_impl(value_bytes[3])?;
        } else {
            // 11 — full RD
            self.write_2_bits_impl(3)?;
            self.write_bytes_impl(&value_bytes)?;
        }
        Ok(())
    }

    fn position_in_bits_impl(&self) -> i64 {
        // Position depends on the stream position, which we need to query
        // but we don't have &mut self here. We'll track it differently.
        // Actually in the C# code: Position * 8 + BitShift
        // where Position = stream.Position
        // We'll need to store position externally — or just use stream_position() with &mut.
        // For now we return 0 — actual usage of this method gets the real value via the trait.
        0
    }

    /// Get byte position of the underlying stream.
    fn stream_position(&mut self) -> Result<u64> {
        Ok(self.stream.stream_position()?)
    }

    fn set_position_in_bits_impl(&mut self, pos_in_bits: i64) -> Result<()> {
        let byte_pos = pos_in_bits / 8;
        self.bit_shift = (pos_in_bits % 8) as i32;
        self.stream.seek(SeekFrom::Start(byte_pos as u64))?;

        if self.bit_shift > 0 {
            let mut buf = [0u8; 1];
            self.stream.read_exact(&mut buf)?;
            self.last_byte = buf[0];
            self.stream.seek(SeekFrom::Start(byte_pos as u64))?;
        } else {
            self.last_byte = 0;
        }
        Ok(())
    }

    fn write_shift_value_impl(&mut self) -> Result<()> {
        if self.bit_shift > 0 {
            let position = self.stream.stream_position()?;
            let mut buf = [0u8; 1];
            self.stream.read_exact(&mut buf)?;
            let last_value = buf[0];
            let mask = 0xFFu8 >> self.bit_shift as u32;
            let curr_value = self.last_byte | (last_value & mask);
            self.stream.seek(SeekFrom::Start(position))?;
            self.stream.write_all(&[curr_value])?;
        }
        Ok(())
    }

    fn set_position_by_flag_impl(&mut self, pos: i64) -> Result<()> {
        if pos >= 0x8000 {
            if pos >= 0x4000_0000 {
                let v = ((pos >> 30) & 0xFFFF) as u16;
                self.write_bytes_impl(&v.to_le_bytes())?;
                let v = (((pos >> 15) & 0x7FFF) | 0x8000) as u16;
                self.write_bytes_impl(&v.to_le_bytes())?;
            } else {
                let v = ((pos >> 15) & 0xFFFF) as u16;
                self.write_bytes_impl(&v.to_le_bytes())?;
            }
            let v = ((pos & 0x7FFF) | 0x8000) as u16;
            self.write_bytes_impl(&v.to_le_bytes())?;
        } else {
            let v = pos as u16;
            self.write_bytes_impl(&v.to_le_bytes())?;
        }
        Ok(())
    }
}

// ─────────────────────────────── AC12 ───────────────────────────────

/// AC1012/AC1014 writer — identical to base.
pub struct DwgStreamWriterAc12 {
    pub(crate) inner: DwgStreamWriterBase,
}

impl DwgStreamWriter for DwgStreamWriterAc12 {
    fn stream(&mut self) -> &mut dyn WriteSeek {
        &mut *self.inner.stream
    }

    fn position_in_bits(&self) -> i64 {
        // Cannot query stream pos without &mut — use cached approach
        // In practice, callers go through DwgMergedStreamWriter which tracks this.
        // For base writers, we approximate using bit_shift only.
        // Actual approach: the C# code uses stream.Position * 8 + bitShift
        // We'll return a sentinel; real position_in_bits is handled by the merged writer.
        // TODO: track position internally for non-merged usage
        0
    }

    fn saved_position_in_bits(&self) -> i64 {
        0
    }

    fn write_bytes(&mut self, bytes: &[u8]) -> Result<()> {
        self.inner.write_bytes_impl(bytes)
    }

    fn write_bytes_offset(&mut self, bytes: &[u8], offset: usize, length: usize) -> Result<()> {
        self.inner.write_bytes_offset_impl(bytes, offset, length)
    }

    fn write_int(&mut self, value: i32) -> Result<()> {
        self.inner.write_int_impl(value)
    }

    fn write_object_type(&mut self, value: i16) -> Result<()> {
        self.inner.write_bit_short_impl(value)
    }

    fn write_raw_long(&mut self, value: i64) -> Result<()> {
        self.inner.write_raw_long_impl(value)
    }

    fn write_bit_double(&mut self, value: f64) -> Result<()> {
        self.inner.write_bit_double_impl(value)
    }

    fn write_bit_long(&mut self, value: i32) -> Result<()> {
        self.inner.write_bit_long_impl(value)
    }

    fn write_bit_long_long(&mut self, value: i64) -> Result<()> {
        self.inner.write_bit_long_long_impl(value)
    }

    fn write_variable_text(&mut self, value: &str) -> Result<()> {
        self.inner.write_variable_text_impl(value)
    }

    fn write_text_unicode(&mut self, value: &str) -> Result<()> {
        self.inner.write_text_unicode_impl(value)
    }

    fn write_bit(&mut self, value: bool) -> Result<()> {
        self.inner.write_bit_impl(value)
    }

    fn write_2_bits(&mut self, value: u8) -> Result<()> {
        self.inner.write_2_bits_impl(value)
    }

    fn write_bit_short(&mut self, value: i16) -> Result<()> {
        self.inner.write_bit_short_impl(value)
    }

    fn write_date_time(&mut self, jdate: i32, msecs: i32) -> Result<()> {
        self.inner.write_bit_long_impl(jdate)?;
        self.inner.write_bit_long_impl(msecs)?;
        Ok(())
    }

    fn write_8_bit_julian_date(&mut self, jdate: i32, msecs: i32) -> Result<()> {
        self.inner.write_raw_long_impl(jdate as i64)?;
        self.inner.write_raw_long_impl(msecs as i64)?;
        Ok(())
    }

    fn write_time_span(&mut self, days: i32, msecs: i32) -> Result<()> {
        self.inner.write_bit_long_impl(days)?;
        self.inner.write_bit_long_impl(msecs)?;
        Ok(())
    }

    fn write_cm_color(&mut self, value: &Color) -> Result<()> {
        self.inner.write_cm_color_impl(value)
    }

    fn write_en_color(&mut self, color: &Color, transparency: &Transparency) -> Result<()> {
        self.inner.write_en_color_impl(color, transparency)
    }

    fn write_en_color_book(
        &mut self,
        color: &Color,
        transparency: &Transparency,
        is_book_color: bool,
    ) -> Result<()> {
        self.inner
            .write_en_color_book_impl(color, transparency, is_book_color)
    }

    fn write_2_bit_double(&mut self, value: &Vector2) -> Result<()> {
        self.inner.write_bit_double_impl(value.x)?;
        self.inner.write_bit_double_impl(value.y)?;
        Ok(())
    }

    fn write_3_bit_double(&mut self, value: &Vector3) -> Result<()> {
        self.inner.write_bit_double_impl(value.x)?;
        self.inner.write_bit_double_impl(value.y)?;
        self.inner.write_bit_double_impl(value.z)?;
        Ok(())
    }

    fn write_2_raw_double(&mut self, value: &Vector2) -> Result<()> {
        self.inner.write_raw_double_impl(value.x)?;
        self.inner.write_raw_double_impl(value.y)?;
        Ok(())
    }

    fn write_byte(&mut self, value: u8) -> Result<()> {
        self.inner.write_byte_impl(value)
    }

    fn handle_reference(&mut self, handle: u64) -> Result<()> {
        self.inner
            .handle_reference_impl(DwgReferenceType::Undefined, handle)
    }

    fn handle_reference_typed(
        &mut self,
        ref_type: DwgReferenceType,
        handle: u64,
    ) -> Result<()> {
        self.inner.handle_reference_impl(ref_type, handle)
    }

    fn write_spear_shift(&mut self) -> Result<()> {
        self.inner.write_spear_shift_impl()
    }

    fn write_raw_short(&mut self, value: i16) -> Result<()> {
        self.inner.write_raw_short_impl(value)
    }

    fn write_raw_short_unsigned(&mut self, value: u16) -> Result<()> {
        self.inner.write_raw_short_unsigned_impl(value)
    }

    fn write_raw_double(&mut self, value: f64) -> Result<()> {
        self.inner.write_raw_double_impl(value)
    }

    fn write_bit_thickness(&mut self, thickness: f64) -> Result<()> {
        self.inner.write_bit_thickness_impl(thickness)
    }

    fn write_bit_extrusion(&mut self, normal: &Vector3) -> Result<()> {
        self.inner.write_bit_extrusion_impl(normal)
    }

    fn write_bit_double_with_default(&mut self, def: f64, value: f64) -> Result<()> {
        self.inner.write_bit_double_with_default_impl(def, value)
    }

    fn write_2_bit_double_with_default(
        &mut self,
        def: &Vector2,
        value: &Vector2,
    ) -> Result<()> {
        self.inner
            .write_bit_double_with_default_impl(def.x, value.x)?;
        self.inner
            .write_bit_double_with_default_impl(def.y, value.y)?;
        Ok(())
    }

    fn write_3_bit_double_with_default(
        &mut self,
        def: &Vector3,
        value: &Vector3,
    ) -> Result<()> {
        self.inner
            .write_bit_double_with_default_impl(def.x, value.x)?;
        self.inner
            .write_bit_double_with_default_impl(def.y, value.y)?;
        self.inner
            .write_bit_double_with_default_impl(def.z, value.z)?;
        Ok(())
    }

    fn reset_stream(&mut self) -> Result<()> {
        self.inner.stream.seek(SeekFrom::Start(0))?;
        self.inner.reset_shift();
        // Truncate by writing nothing from position 0
        // WriteSeek doesn't expose set_len, so we reset shift and position only
        Ok(())
    }

    fn save_position_for_size(&mut self) -> Result<()> {
        self.inner.write_raw_long_impl(0)
    }

    fn set_position_in_bits(&mut self, pos_in_bits: i64) -> Result<()> {
        self.inner.set_position_in_bits_impl(pos_in_bits)
    }

    fn set_position_by_flag(&mut self, pos: i64) -> Result<()> {
        self.inner.set_position_by_flag_impl(pos)
    }

    fn write_shift_value(&mut self) -> Result<()> {
        self.inner.write_shift_value_impl()
    }
}

/// Helper to get stream position + bit shift as position in bits.
fn position_in_bits_of(base: &mut DwgStreamWriterBase) -> i64 {
    let pos = base.stream.stream_position().unwrap_or(0);
    pos as i64 * 8 + base.bit_shift as i64
}

// ─────────────────────────────── AC15 ───────────────────────────────

/// AC1015 (R2000) writer — overrides thickness / extrusion for optimized formats.
pub struct DwgStreamWriterAc15 {
    pub(crate) inner: DwgStreamWriterAc12,
}

impl DwgStreamWriter for DwgStreamWriterAc15 {
    fn stream(&mut self) -> &mut dyn WriteSeek {
        self.inner.stream()
    }

    fn position_in_bits(&self) -> i64 {
        self.inner.position_in_bits()
    }

    fn saved_position_in_bits(&self) -> i64 {
        self.inner.saved_position_in_bits()
    }

    fn write_bytes(&mut self, bytes: &[u8]) -> Result<()> {
        self.inner.write_bytes(bytes)
    }

    fn write_bytes_offset(&mut self, bytes: &[u8], offset: usize, length: usize) -> Result<()> {
        self.inner.write_bytes_offset(bytes, offset, length)
    }

    fn write_int(&mut self, value: i32) -> Result<()> {
        self.inner.write_int(value)
    }

    fn write_object_type(&mut self, value: i16) -> Result<()> {
        self.inner.write_object_type(value)
    }

    fn write_raw_long(&mut self, value: i64) -> Result<()> {
        self.inner.write_raw_long(value)
    }

    fn write_bit_double(&mut self, value: f64) -> Result<()> {
        self.inner.write_bit_double(value)
    }

    fn write_bit_long(&mut self, value: i32) -> Result<()> {
        self.inner.write_bit_long(value)
    }

    fn write_bit_long_long(&mut self, value: i64) -> Result<()> {
        self.inner.write_bit_long_long(value)
    }

    fn write_variable_text(&mut self, value: &str) -> Result<()> {
        self.inner.write_variable_text(value)
    }

    fn write_text_unicode(&mut self, value: &str) -> Result<()> {
        self.inner.write_text_unicode(value)
    }

    fn write_bit(&mut self, value: bool) -> Result<()> {
        self.inner.write_bit(value)
    }

    fn write_2_bits(&mut self, value: u8) -> Result<()> {
        self.inner.write_2_bits(value)
    }

    fn write_bit_short(&mut self, value: i16) -> Result<()> {
        self.inner.write_bit_short(value)
    }

    fn write_date_time(&mut self, jdate: i32, msecs: i32) -> Result<()> {
        self.inner.write_date_time(jdate, msecs)
    }

    fn write_8_bit_julian_date(&mut self, jdate: i32, msecs: i32) -> Result<()> {
        self.inner.write_8_bit_julian_date(jdate, msecs)
    }

    fn write_time_span(&mut self, days: i32, msecs: i32) -> Result<()> {
        self.inner.write_time_span(days, msecs)
    }

    fn write_cm_color(&mut self, value: &Color) -> Result<()> {
        self.inner.write_cm_color(value)
    }

    fn write_en_color(&mut self, color: &Color, transparency: &Transparency) -> Result<()> {
        self.inner.write_en_color(color, transparency)
    }

    fn write_en_color_book(
        &mut self,
        color: &Color,
        transparency: &Transparency,
        is_book_color: bool,
    ) -> Result<()> {
        self.inner
            .write_en_color_book(color, transparency, is_book_color)
    }

    fn write_2_bit_double(&mut self, value: &Vector2) -> Result<()> {
        self.inner.write_2_bit_double(value)
    }

    fn write_3_bit_double(&mut self, value: &Vector3) -> Result<()> {
        self.inner.write_3_bit_double(value)
    }

    fn write_2_raw_double(&mut self, value: &Vector2) -> Result<()> {
        self.inner.write_2_raw_double(value)
    }

    fn write_byte(&mut self, value: u8) -> Result<()> {
        self.inner.write_byte(value)
    }

    fn handle_reference(&mut self, handle: u64) -> Result<()> {
        self.inner.handle_reference(handle)
    }

    fn handle_reference_typed(
        &mut self,
        ref_type: DwgReferenceType,
        handle: u64,
    ) -> Result<()> {
        self.inner.handle_reference_typed(ref_type, handle)
    }

    fn write_spear_shift(&mut self) -> Result<()> {
        self.inner.write_spear_shift()
    }

    fn write_raw_short(&mut self, value: i16) -> Result<()> {
        self.inner.write_raw_short(value)
    }

    fn write_raw_short_unsigned(&mut self, value: u16) -> Result<()> {
        self.inner.write_raw_short_unsigned(value)
    }

    fn write_raw_double(&mut self, value: f64) -> Result<()> {
        self.inner.write_raw_double(value)
    }

    // ---- overrides ----

    fn write_bit_thickness(&mut self, thickness: f64) -> Result<()> {
        // R2000+: single bit flag, then optional BD
        if thickness == 0.0 {
            self.inner.inner.write_bit_impl(true)?;
            return Ok(());
        }
        self.inner.inner.write_bit_impl(false)?;
        self.inner.inner.write_bit_double_impl(thickness)?;
        Ok(())
    }

    fn write_bit_extrusion(&mut self, normal: &Vector3) -> Result<()> {
        // R2000+: if (0,0,1) write bit=1, otherwise bit=0 + 3BD
        if *normal == Vector3::UNIT_Z {
            self.inner.inner.write_bit_impl(true)?;
            return Ok(());
        }
        self.inner.inner.write_bit_impl(false)?;
        self.inner.inner.write_bit_double_impl(normal.x)?;
        self.inner.inner.write_bit_double_impl(normal.y)?;
        self.inner.inner.write_bit_double_impl(normal.z)?;
        Ok(())
    }

    fn write_bit_double_with_default(&mut self, def: f64, value: f64) -> Result<()> {
        self.inner.write_bit_double_with_default(def, value)
    }

    fn write_2_bit_double_with_default(
        &mut self,
        def: &Vector2,
        value: &Vector2,
    ) -> Result<()> {
        self.inner.write_2_bit_double_with_default(def, value)
    }

    fn write_3_bit_double_with_default(
        &mut self,
        def: &Vector3,
        value: &Vector3,
    ) -> Result<()> {
        self.inner.write_3_bit_double_with_default(def, value)
    }

    fn reset_stream(&mut self) -> Result<()> {
        self.inner.reset_stream()
    }

    fn save_position_for_size(&mut self) -> Result<()> {
        self.inner.save_position_for_size()
    }

    fn set_position_in_bits(&mut self, pos_in_bits: i64) -> Result<()> {
        self.inner.set_position_in_bits(pos_in_bits)
    }

    fn set_position_by_flag(&mut self, pos: i64) -> Result<()> {
        self.inner.set_position_by_flag(pos)
    }

    fn write_shift_value(&mut self) -> Result<()> {
        self.inner.write_shift_value()
    }
}

// ─────────────────────────────── AC18 ───────────────────────────────

/// AC1018 (R2004) writer — overrides CMC/ENC color writing.
pub struct DwgStreamWriterAc18 {
    pub(crate) inner: DwgStreamWriterAc15,
}

impl DwgStreamWriter for DwgStreamWriterAc18 {
    fn stream(&mut self) -> &mut dyn WriteSeek {
        self.inner.stream()
    }

    fn position_in_bits(&self) -> i64 {
        self.inner.position_in_bits()
    }

    fn saved_position_in_bits(&self) -> i64 {
        self.inner.saved_position_in_bits()
    }

    fn write_bytes(&mut self, bytes: &[u8]) -> Result<()> {
        self.inner.write_bytes(bytes)
    }

    fn write_bytes_offset(&mut self, bytes: &[u8], offset: usize, length: usize) -> Result<()> {
        self.inner.write_bytes_offset(bytes, offset, length)
    }

    fn write_int(&mut self, value: i32) -> Result<()> {
        self.inner.write_int(value)
    }

    fn write_object_type(&mut self, value: i16) -> Result<()> {
        self.inner.write_object_type(value)
    }

    fn write_raw_long(&mut self, value: i64) -> Result<()> {
        self.inner.write_raw_long(value)
    }

    fn write_bit_double(&mut self, value: f64) -> Result<()> {
        self.inner.write_bit_double(value)
    }

    fn write_bit_long(&mut self, value: i32) -> Result<()> {
        self.inner.write_bit_long(value)
    }

    fn write_bit_long_long(&mut self, value: i64) -> Result<()> {
        self.inner.write_bit_long_long(value)
    }

    fn write_variable_text(&mut self, value: &str) -> Result<()> {
        self.inner.write_variable_text(value)
    }

    fn write_text_unicode(&mut self, value: &str) -> Result<()> {
        self.inner.write_text_unicode(value)
    }

    fn write_bit(&mut self, value: bool) -> Result<()> {
        self.inner.write_bit(value)
    }

    fn write_2_bits(&mut self, value: u8) -> Result<()> {
        self.inner.write_2_bits(value)
    }

    fn write_bit_short(&mut self, value: i16) -> Result<()> {
        self.inner.write_bit_short(value)
    }

    fn write_date_time(&mut self, jdate: i32, msecs: i32) -> Result<()> {
        self.inner.write_date_time(jdate, msecs)
    }

    fn write_8_bit_julian_date(&mut self, jdate: i32, msecs: i32) -> Result<()> {
        self.inner.write_8_bit_julian_date(jdate, msecs)
    }

    fn write_time_span(&mut self, days: i32, msecs: i32) -> Result<()> {
        self.inner.write_time_span(days, msecs)
    }

    // ---- overrides for AC18 ----

    fn write_cm_color(&mut self, value: &Color) -> Result<()> {
        // BS: color index (always 0)
        let base = &mut self.inner.inner.inner;
        base.write_bit_short_impl(0)?;

        let mut arr = [0u8; 4];
        match value {
            Color::Rgb { r, g, b } => {
                arr[2] = *r;
                arr[1] = *g;
                arr[0] = *b;
                arr[3] = 0b1100_0010;
            }
            Color::ByLayer => {
                arr[3] = 0b1100_0000;
            }
            Color::Index(i) => {
                arr[3] = 0b1100_0011;
                arr[0] = *i;
            }
            Color::ByBlock => {
                arr[3] = 0b1100_0000;
            }
        }

        let rgb_val = i32::from_le_bytes(arr);
        base.write_bit_long_impl(rgb_val)?;
        base.write_byte_impl(0)?; // Color Byte — no color/book name
        Ok(())
    }

    fn write_en_color(&mut self, color: &Color, transparency: &Transparency) -> Result<()> {
        let base = &mut self.inner.inner.inner;

        // BS: flags + color index
        let mut size: u16 = 0;

        if matches!(color, Color::ByBlock) && transparency.is_opaque() {
            base.write_bit_short_impl(0)?;
            return Ok(());
        }

        // 0x2000: transparency follows
        if !transparency.is_opaque() {
            size |= 0x2000;
        }

        match color {
            Color::Rgb { .. } => {
                size |= 0x8000;
            }
            Color::Index(i) => {
                size |= *i as u16;
            }
            _ => {}
        }

        base.write_bit_short_impl(size as i16)?;

        if let Color::Rgb { r, g, b } = color {
            let arr = [*b, *g, *r, 0b1100_0010u8];
            let rgb = u32::from_le_bytes(arr);
            base.write_bit_long_impl(rgb as i32)?;
        }

        if !transparency.is_opaque() {
            base.write_bit_long_impl(transparency.to_alpha_value())?;
        }

        Ok(())
    }

    fn write_en_color_book(
        &mut self,
        color: &Color,
        transparency: &Transparency,
        is_book_color: bool,
    ) -> Result<()> {
        let base = &mut self.inner.inner.inner;

        let mut size: u16 = 0;

        if matches!(color, Color::ByBlock)
            && transparency.is_opaque()
            && !is_book_color
        {
            base.write_bit_short_impl(0)?;
            return Ok(());
        }

        if !transparency.is_opaque() {
            size |= 0x2000;
        }

        if is_book_color {
            size |= 0x4000;
            size |= 0x8000;
        } else {
            match color {
                Color::Rgb { .. } => {
                    size |= 0x8000;
                }
                Color::Index(i) => {
                    size |= *i as u16;
                }
                _ => {}
            }
        }

        base.write_bit_short_impl(size as i16)?;

        if let Color::Rgb { r, g, b } = color {
            let arr = [*b, *g, *r, 0b1100_0010u8];
            let rgb = u32::from_le_bytes(arr);
            base.write_bit_long_impl(rgb as i32)?;
        }

        if !transparency.is_opaque() {
            base.write_bit_long_impl(transparency.to_alpha_value())?;
        }

        Ok(())
    }

    fn write_2_bit_double(&mut self, value: &Vector2) -> Result<()> {
        self.inner.write_2_bit_double(value)
    }

    fn write_3_bit_double(&mut self, value: &Vector3) -> Result<()> {
        self.inner.write_3_bit_double(value)
    }

    fn write_2_raw_double(&mut self, value: &Vector2) -> Result<()> {
        self.inner.write_2_raw_double(value)
    }

    fn write_byte(&mut self, value: u8) -> Result<()> {
        self.inner.write_byte(value)
    }

    fn handle_reference(&mut self, handle: u64) -> Result<()> {
        self.inner.handle_reference(handle)
    }

    fn handle_reference_typed(
        &mut self,
        ref_type: DwgReferenceType,
        handle: u64,
    ) -> Result<()> {
        self.inner.handle_reference_typed(ref_type, handle)
    }

    fn write_spear_shift(&mut self) -> Result<()> {
        self.inner.write_spear_shift()
    }

    fn write_raw_short(&mut self, value: i16) -> Result<()> {
        self.inner.write_raw_short(value)
    }

    fn write_raw_short_unsigned(&mut self, value: u16) -> Result<()> {
        self.inner.write_raw_short_unsigned(value)
    }

    fn write_raw_double(&mut self, value: f64) -> Result<()> {
        self.inner.write_raw_double(value)
    }

    fn write_bit_thickness(&mut self, thickness: f64) -> Result<()> {
        self.inner.write_bit_thickness(thickness)
    }

    fn write_bit_extrusion(&mut self, normal: &Vector3) -> Result<()> {
        self.inner.write_bit_extrusion(normal)
    }

    fn write_bit_double_with_default(&mut self, def: f64, value: f64) -> Result<()> {
        self.inner.write_bit_double_with_default(def, value)
    }

    fn write_2_bit_double_with_default(
        &mut self,
        def: &Vector2,
        value: &Vector2,
    ) -> Result<()> {
        self.inner.write_2_bit_double_with_default(def, value)
    }

    fn write_3_bit_double_with_default(
        &mut self,
        def: &Vector3,
        value: &Vector3,
    ) -> Result<()> {
        self.inner.write_3_bit_double_with_default(def, value)
    }

    fn reset_stream(&mut self) -> Result<()> {
        self.inner.reset_stream()
    }

    fn save_position_for_size(&mut self) -> Result<()> {
        self.inner.save_position_for_size()
    }

    fn set_position_in_bits(&mut self, pos_in_bits: i64) -> Result<()> {
        self.inner.set_position_in_bits(pos_in_bits)
    }

    fn set_position_by_flag(&mut self, pos: i64) -> Result<()> {
        self.inner.set_position_by_flag(pos)
    }

    fn write_shift_value(&mut self) -> Result<()> {
        self.inner.write_shift_value()
    }
}

// ─────────────────────────────── AC21 ───────────────────────────────

/// AC1021 (R2007) writer — overrides text to Unicode (UTF-16LE).
pub struct DwgStreamWriterAc21 {
    pub(crate) inner: DwgStreamWriterAc18,
}

impl DwgStreamWriter for DwgStreamWriterAc21 {
    fn stream(&mut self) -> &mut dyn WriteSeek {
        self.inner.stream()
    }
    fn position_in_bits(&self) -> i64 {
        self.inner.position_in_bits()
    }
    fn saved_position_in_bits(&self) -> i64 {
        self.inner.saved_position_in_bits()
    }
    fn write_bytes(&mut self, bytes: &[u8]) -> Result<()> {
        self.inner.write_bytes(bytes)
    }
    fn write_bytes_offset(&mut self, bytes: &[u8], offset: usize, length: usize) -> Result<()> {
        self.inner.write_bytes_offset(bytes, offset, length)
    }
    fn write_int(&mut self, value: i32) -> Result<()> {
        self.inner.write_int(value)
    }
    fn write_object_type(&mut self, value: i16) -> Result<()> {
        self.inner.write_object_type(value)
    }
    fn write_raw_long(&mut self, value: i64) -> Result<()> {
        self.inner.write_raw_long(value)
    }
    fn write_bit_double(&mut self, value: f64) -> Result<()> {
        self.inner.write_bit_double(value)
    }
    fn write_bit_long(&mut self, value: i32) -> Result<()> {
        self.inner.write_bit_long(value)
    }
    fn write_bit_long_long(&mut self, value: i64) -> Result<()> {
        self.inner.write_bit_long_long(value)
    }

    // ---- overrides: Unicode text ----

    fn write_variable_text(&mut self, value: &str) -> Result<()> {
        if value.is_empty() {
            let base = &mut self.inner.inner.inner.inner;
            base.write_bit_short_impl(0)?;
            return Ok(());
        }
        let utf16: Vec<u16> = value.encode_utf16().collect();
        let base = &mut self.inner.inner.inner.inner;
        base.write_bit_short_impl(utf16.len() as i16)?;
        let bytes: Vec<u8> = utf16.iter().flat_map(|c| c.to_le_bytes()).collect();
        base.write_bytes_impl(&bytes)?;
        Ok(())
    }

    fn write_text_unicode(&mut self, value: &str) -> Result<()> {
        let utf16: Vec<u16> = value.encode_utf16().collect();
        let base = &mut self.inner.inner.inner.inner;
        base.write_raw_short_impl((utf16.len() as i16) + 1)?;
        let bytes: Vec<u8> = utf16.iter().flat_map(|c| c.to_le_bytes()).collect();
        base.write_bytes_impl(&bytes)?;
        // Null terminator (2 bytes for UTF-16)
        base.stream.write_all(&[0, 0])?;
        Ok(())
    }

    fn write_bit(&mut self, value: bool) -> Result<()> {
        self.inner.write_bit(value)
    }
    fn write_2_bits(&mut self, value: u8) -> Result<()> {
        self.inner.write_2_bits(value)
    }
    fn write_bit_short(&mut self, value: i16) -> Result<()> {
        self.inner.write_bit_short(value)
    }
    fn write_date_time(&mut self, jdate: i32, msecs: i32) -> Result<()> {
        self.inner.write_date_time(jdate, msecs)
    }
    fn write_8_bit_julian_date(&mut self, jdate: i32, msecs: i32) -> Result<()> {
        self.inner.write_8_bit_julian_date(jdate, msecs)
    }
    fn write_time_span(&mut self, days: i32, msecs: i32) -> Result<()> {
        self.inner.write_time_span(days, msecs)
    }
    fn write_cm_color(&mut self, value: &Color) -> Result<()> {
        self.inner.write_cm_color(value)
    }
    fn write_en_color(&mut self, color: &Color, transparency: &Transparency) -> Result<()> {
        self.inner.write_en_color(color, transparency)
    }
    fn write_en_color_book(
        &mut self,
        color: &Color,
        transparency: &Transparency,
        is_book_color: bool,
    ) -> Result<()> {
        self.inner
            .write_en_color_book(color, transparency, is_book_color)
    }
    fn write_2_bit_double(&mut self, value: &Vector2) -> Result<()> {
        self.inner.write_2_bit_double(value)
    }
    fn write_3_bit_double(&mut self, value: &Vector3) -> Result<()> {
        self.inner.write_3_bit_double(value)
    }
    fn write_2_raw_double(&mut self, value: &Vector2) -> Result<()> {
        self.inner.write_2_raw_double(value)
    }
    fn write_byte(&mut self, value: u8) -> Result<()> {
        self.inner.write_byte(value)
    }
    fn handle_reference(&mut self, handle: u64) -> Result<()> {
        self.inner.handle_reference(handle)
    }
    fn handle_reference_typed(
        &mut self,
        ref_type: DwgReferenceType,
        handle: u64,
    ) -> Result<()> {
        self.inner.handle_reference_typed(ref_type, handle)
    }
    fn write_spear_shift(&mut self) -> Result<()> {
        self.inner.write_spear_shift()
    }
    fn write_raw_short(&mut self, value: i16) -> Result<()> {
        self.inner.write_raw_short(value)
    }
    fn write_raw_short_unsigned(&mut self, value: u16) -> Result<()> {
        self.inner.write_raw_short_unsigned(value)
    }
    fn write_raw_double(&mut self, value: f64) -> Result<()> {
        self.inner.write_raw_double(value)
    }
    fn write_bit_thickness(&mut self, thickness: f64) -> Result<()> {
        self.inner.write_bit_thickness(thickness)
    }
    fn write_bit_extrusion(&mut self, normal: &Vector3) -> Result<()> {
        self.inner.write_bit_extrusion(normal)
    }
    fn write_bit_double_with_default(&mut self, def: f64, value: f64) -> Result<()> {
        self.inner.write_bit_double_with_default(def, value)
    }
    fn write_2_bit_double_with_default(
        &mut self,
        def: &Vector2,
        value: &Vector2,
    ) -> Result<()> {
        self.inner.write_2_bit_double_with_default(def, value)
    }
    fn write_3_bit_double_with_default(
        &mut self,
        def: &Vector3,
        value: &Vector3,
    ) -> Result<()> {
        self.inner.write_3_bit_double_with_default(def, value)
    }
    fn reset_stream(&mut self) -> Result<()> {
        self.inner.reset_stream()
    }
    fn save_position_for_size(&mut self) -> Result<()> {
        self.inner.save_position_for_size()
    }
    fn set_position_in_bits(&mut self, pos_in_bits: i64) -> Result<()> {
        self.inner.set_position_in_bits(pos_in_bits)
    }
    fn set_position_by_flag(&mut self, pos: i64) -> Result<()> {
        self.inner.set_position_by_flag(pos)
    }
    fn write_shift_value(&mut self) -> Result<()> {
        self.inner.write_shift_value()
    }
}

// ─────────────────────────────── AC24 ───────────────────────────────

/// AC1024 (R2010+) writer — overrides ObjectType encoding.
pub struct DwgStreamWriterAc24 {
    pub(crate) inner: DwgStreamWriterAc21,
}

impl DwgStreamWriter for DwgStreamWriterAc24 {
    fn stream(&mut self) -> &mut dyn WriteSeek {
        self.inner.stream()
    }
    fn position_in_bits(&self) -> i64 {
        self.inner.position_in_bits()
    }
    fn saved_position_in_bits(&self) -> i64 {
        self.inner.saved_position_in_bits()
    }
    fn write_bytes(&mut self, bytes: &[u8]) -> Result<()> {
        self.inner.write_bytes(bytes)
    }
    fn write_bytes_offset(&mut self, bytes: &[u8], offset: usize, length: usize) -> Result<()> {
        self.inner.write_bytes_offset(bytes, offset, length)
    }
    fn write_int(&mut self, value: i32) -> Result<()> {
        self.inner.write_int(value)
    }

    // ---- override: object type encoding ----
    fn write_object_type(&mut self, value: i16) -> Result<()> {
        let base = &mut self.inner.inner.inner.inner.inner;
        if value <= 255 {
            base.write_2_bits_impl(0)?;
            base.write_byte_impl(value as u8)?;
        } else if value >= 0x1F0 && value <= 0x2EF {
            base.write_2_bits_impl(1)?;
            base.write_byte_impl((value - 0x1F0) as u8)?;
        } else {
            base.write_2_bits_impl(2)?;
            let bytes = value.to_le_bytes();
            base.write_bytes_impl(&bytes)?;
        }
        Ok(())
    }

    fn write_raw_long(&mut self, value: i64) -> Result<()> {
        self.inner.write_raw_long(value)
    }
    fn write_bit_double(&mut self, value: f64) -> Result<()> {
        self.inner.write_bit_double(value)
    }
    fn write_bit_long(&mut self, value: i32) -> Result<()> {
        self.inner.write_bit_long(value)
    }
    fn write_bit_long_long(&mut self, value: i64) -> Result<()> {
        self.inner.write_bit_long_long(value)
    }
    fn write_variable_text(&mut self, value: &str) -> Result<()> {
        self.inner.write_variable_text(value)
    }
    fn write_text_unicode(&mut self, value: &str) -> Result<()> {
        self.inner.write_text_unicode(value)
    }
    fn write_bit(&mut self, value: bool) -> Result<()> {
        self.inner.write_bit(value)
    }
    fn write_2_bits(&mut self, value: u8) -> Result<()> {
        self.inner.write_2_bits(value)
    }
    fn write_bit_short(&mut self, value: i16) -> Result<()> {
        self.inner.write_bit_short(value)
    }
    fn write_date_time(&mut self, jdate: i32, msecs: i32) -> Result<()> {
        self.inner.write_date_time(jdate, msecs)
    }
    fn write_8_bit_julian_date(&mut self, jdate: i32, msecs: i32) -> Result<()> {
        self.inner.write_8_bit_julian_date(jdate, msecs)
    }
    fn write_time_span(&mut self, days: i32, msecs: i32) -> Result<()> {
        self.inner.write_time_span(days, msecs)
    }
    fn write_cm_color(&mut self, value: &Color) -> Result<()> {
        self.inner.write_cm_color(value)
    }
    fn write_en_color(&mut self, color: &Color, transparency: &Transparency) -> Result<()> {
        self.inner.write_en_color(color, transparency)
    }
    fn write_en_color_book(
        &mut self,
        color: &Color,
        transparency: &Transparency,
        is_book_color: bool,
    ) -> Result<()> {
        self.inner
            .write_en_color_book(color, transparency, is_book_color)
    }
    fn write_2_bit_double(&mut self, value: &Vector2) -> Result<()> {
        self.inner.write_2_bit_double(value)
    }
    fn write_3_bit_double(&mut self, value: &Vector3) -> Result<()> {
        self.inner.write_3_bit_double(value)
    }
    fn write_2_raw_double(&mut self, value: &Vector2) -> Result<()> {
        self.inner.write_2_raw_double(value)
    }
    fn write_byte(&mut self, value: u8) -> Result<()> {
        self.inner.write_byte(value)
    }
    fn handle_reference(&mut self, handle: u64) -> Result<()> {
        self.inner.handle_reference(handle)
    }
    fn handle_reference_typed(
        &mut self,
        ref_type: DwgReferenceType,
        handle: u64,
    ) -> Result<()> {
        self.inner.handle_reference_typed(ref_type, handle)
    }
    fn write_spear_shift(&mut self) -> Result<()> {
        self.inner.write_spear_shift()
    }
    fn write_raw_short(&mut self, value: i16) -> Result<()> {
        self.inner.write_raw_short(value)
    }
    fn write_raw_short_unsigned(&mut self, value: u16) -> Result<()> {
        self.inner.write_raw_short_unsigned(value)
    }
    fn write_raw_double(&mut self, value: f64) -> Result<()> {
        self.inner.write_raw_double(value)
    }
    fn write_bit_thickness(&mut self, thickness: f64) -> Result<()> {
        self.inner.write_bit_thickness(thickness)
    }
    fn write_bit_extrusion(&mut self, normal: &Vector3) -> Result<()> {
        self.inner.write_bit_extrusion(normal)
    }
    fn write_bit_double_with_default(&mut self, def: f64, value: f64) -> Result<()> {
        self.inner.write_bit_double_with_default(def, value)
    }
    fn write_2_bit_double_with_default(
        &mut self,
        def: &Vector2,
        value: &Vector2,
    ) -> Result<()> {
        self.inner.write_2_bit_double_with_default(def, value)
    }
    fn write_3_bit_double_with_default(
        &mut self,
        def: &Vector3,
        value: &Vector3,
    ) -> Result<()> {
        self.inner.write_3_bit_double_with_default(def, value)
    }
    fn reset_stream(&mut self) -> Result<()> {
        self.inner.reset_stream()
    }
    fn save_position_for_size(&mut self) -> Result<()> {
        self.inner.save_position_for_size()
    }
    fn set_position_in_bits(&mut self, pos_in_bits: i64) -> Result<()> {
        self.inner.set_position_in_bits(pos_in_bits)
    }
    fn set_position_by_flag(&mut self, pos: i64) -> Result<()> {
        self.inner.set_position_by_flag(pos)
    }
    fn write_shift_value(&mut self) -> Result<()> {
        self.inner.write_shift_value()
    }
}

/// Trait extension: get position in bits from a `DwgStreamWriterBase`.
impl DwgStreamWriterBase {
    pub fn position_in_bits_val(&mut self) -> i64 {
        position_in_bits_of(self)
    }

    /// Raw stream access for external consumers.
    pub fn stream_mut(&mut self) -> &mut Box<dyn WriteSeek> {
        &mut self.stream
    }

    /// Borrow stream bytes (only works with `Cursor<Vec<u8>>`).
    pub fn get_buffer(&self) -> Option<&Vec<u8>> {
        None // Cannot downcast trait object; use `into_inner` pattern instead.
    }

    pub fn bit_shift(&self) -> i32 {
        self.bit_shift
    }
}
