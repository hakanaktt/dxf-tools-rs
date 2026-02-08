//! DWG Stream Writer - Bit-level binary writing for DWG format
//!
//! This is the writing counterpart to `stream_reader.rs`.
//! DWG files use bit-packed data structures that require special handling.

use std::io::{Write, Seek, SeekFrom, Cursor};
use crate::error::{DxfError, Result};
use crate::types::{ACadVersion, Color, Vector2, Vector3};
use super::stream_reader::DwgReferenceType;

/// Bit-level stream writer for DWG format
pub struct DwgStreamWriter {
    buffer: Vec<u8>,
    /// Current byte position in the buffer (where next full byte write goes)
    position: usize,
    /// Bit shift within the current byte (0-7)
    bit_shift: u8,
    /// Accumulator for partial byte being assembled
    last_byte: u8,
    /// Target DWG version
    version: ACadVersion,
}

impl DwgStreamWriter {
    /// Create a new stream writer for the given version
    pub fn new(version: ACadVersion) -> Self {
        Self {
            buffer: Vec::new(),
            position: 0,
            bit_shift: 0,
            last_byte: 0,
            version,
        }
    }

    /// Create a writer with pre-allocated capacity
    pub fn with_capacity(version: ACadVersion, capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
            position: 0,
            bit_shift: 0,
            last_byte: 0,
            version,
        }
    }

    /// Get the current position in bits
    pub fn position_in_bits(&self) -> u64 {
        (self.position as u64) * 8 + self.bit_shift as u64
    }

    /// Get the current byte position
    pub fn position(&self) -> usize {
        self.position
    }

    /// Get the current bit shift
    pub fn bit_shift(&self) -> u8 {
        self.bit_shift
    }

    /// Get the version
    pub fn version(&self) -> ACadVersion {
        self.version
    }

    /// Set position in bits (for patching previously written data)
    pub fn set_position_in_bits(&mut self, pos_in_bits: u64) {
        let byte_pos = (pos_in_bits / 8) as usize;
        self.bit_shift = (pos_in_bits % 8) as u8;
        self.position = byte_pos;

        if self.bit_shift > 0 && byte_pos < self.buffer.len() {
            self.last_byte = self.buffer[byte_pos];
        } else {
            self.last_byte = 0;
        }
    }

    /// Flush the partial byte currently being assembled
    /// Pads remaining bits with zeros
    pub fn flush_bits(&mut self) {
        if self.bit_shift > 0 {
            self.ensure_capacity(self.position + 1);
            self.buffer[self.position] = self.last_byte;
            self.position += 1;
            self.bit_shift = 0;
            self.last_byte = 0;
        }
    }

    /// Merge the partial byte with existing data in the buffer
    /// Used when the remaining bits should be OR'd with existing content
    pub fn merge_partial_byte(&mut self) {
        if self.bit_shift > 0 && self.position < self.buffer.len() {
            let existing = self.buffer[self.position];
            let mask = 0xFF >> self.bit_shift;
            self.buffer[self.position] = self.last_byte | (existing & mask);
        }
    }

    /// Get the written data as a byte slice
    pub fn as_bytes(&self) -> &[u8] {
        &self.buffer[..self.position]
    }

    /// Consume the writer and return the buffer
    pub fn into_bytes(mut self) -> Vec<u8> {
        self.flush_bits();
        self.buffer.truncate(self.position);
        self.buffer
    }

    /// Get the total length in bytes (including partial byte if any)
    pub fn len(&self) -> usize {
        if self.bit_shift > 0 {
            self.position + 1
        } else {
            self.position
        }
    }

    /// Ensure the buffer has room for at least `needed` bytes
    fn ensure_capacity(&mut self, needed: usize) {
        if needed > self.buffer.len() {
            self.buffer.resize(needed, 0);
        }
    }

    fn reset_shift(&mut self) {
        self.bit_shift = 0;
        self.last_byte = 0;
    }

    // =========================================================================
    // Bit-level writing
    // =========================================================================

    /// Write a single bit (B)
    pub fn write_bit(&mut self, value: bool) {
        if self.bit_shift < 7 {
            if value {
                self.last_byte |= 1 << (7 - self.bit_shift);
            }
            self.bit_shift += 1;
        } else {
            // bit_shift == 7, this completes a byte
            if value {
                self.last_byte |= 1;
            }
            self.ensure_capacity(self.position + 1);
            self.buffer[self.position] = self.last_byte;
            self.position += 1;
            self.reset_shift();
        }
    }

    /// Write 2 bits (BB)
    pub fn write_2bits(&mut self, value: u8) {
        let v = value & 0x03;
        if self.bit_shift < 6 {
            self.last_byte |= v << (6 - self.bit_shift);
            self.bit_shift += 2;
        } else if self.bit_shift == 6 {
            self.last_byte |= v;
            self.ensure_capacity(self.position + 1);
            self.buffer[self.position] = self.last_byte;
            self.position += 1;
            self.reset_shift();
        } else {
            // bit_shift == 7: straddles byte boundary
            self.last_byte |= v >> 1;
            self.ensure_capacity(self.position + 1);
            self.buffer[self.position] = self.last_byte;
            self.position += 1;
            self.last_byte = v << 7;
            self.bit_shift = 1;
        }
    }

    /// Write 3 bits (3B) — used for BLL size prefix
    pub fn write_3bits(&mut self, value: u8) {
        self.write_bit((value & 4) != 0);
        self.write_bit((value & 2) != 0);
        self.write_bit((value & 1) != 0);
    }

    // =========================================================================
    // Raw (uncompressed) types
    // =========================================================================

    /// Write a raw byte (RC)
    pub fn write_byte(&mut self, value: u8) {
        if self.bit_shift == 0 {
            self.ensure_capacity(self.position + 1);
            self.buffer[self.position] = value;
            self.position += 1;
        } else {
            let shift = 8 - self.bit_shift;
            let combined = self.last_byte | (value >> self.bit_shift);
            self.ensure_capacity(self.position + 1);
            self.buffer[self.position] = combined;
            self.position += 1;
            self.last_byte = value << shift;
        }
    }

    /// Write raw bytes
    pub fn write_bytes(&mut self, arr: &[u8]) {
        if self.bit_shift == 0 {
            self.ensure_capacity(self.position + arr.len());
            self.buffer[self.position..self.position + arr.len()].copy_from_slice(arr);
            self.position += arr.len();
        } else {
            let shift = 8 - self.bit_shift;
            for &b in arr {
                let combined = self.last_byte | (b >> self.bit_shift);
                self.ensure_capacity(self.position + 1);
                self.buffer[self.position] = combined;
                self.position += 1;
                self.last_byte = b << shift;
            }
        }
    }

    /// Write a raw short (RS) - 16-bit little-endian
    pub fn write_raw_short(&mut self, value: i16) {
        let bytes = value.to_le_bytes();
        self.write_byte(bytes[0]);
        self.write_byte(bytes[1]);
    }

    /// Write a raw unsigned short
    pub fn write_raw_ushort(&mut self, value: u16) {
        let bytes = value.to_le_bytes();
        self.write_byte(bytes[0]);
        self.write_byte(bytes[1]);
    }

    /// Write a raw long (RL) - 32-bit little-endian
    pub fn write_raw_long(&mut self, value: i32) {
        let bytes = value.to_le_bytes();
        for &b in &bytes {
            self.write_byte(b);
        }
    }

    /// Write a raw unsigned long
    pub fn write_raw_ulong(&mut self, value: u32) {
        let bytes = value.to_le_bytes();
        for &b in &bytes {
            self.write_byte(b);
        }
    }

    /// Write a raw long long (RLL) - 64-bit little-endian
    pub fn write_raw_longlong(&mut self, value: i64) {
        let bytes = value.to_le_bytes();
        for &b in &bytes {
            self.write_byte(b);
        }
    }

    /// Write a raw double (RD) - 64-bit IEEE 754 little-endian
    pub fn write_raw_double(&mut self, value: f64) {
        let bytes = value.to_le_bytes();
        self.write_bytes(&bytes);
    }

    /// Write 2 raw doubles (2RD)
    pub fn write_2raw_double(&mut self, value: Vector2) {
        self.write_raw_double(value.x);
        self.write_raw_double(value.y);
    }

    /// Write 3 raw doubles (3RD)
    pub fn write_3raw_double(&mut self, value: Vector3) {
        self.write_raw_double(value.x);
        self.write_raw_double(value.y);
        self.write_raw_double(value.z);
    }

    // =========================================================================
    // Bit-coded types
    // =========================================================================

    /// Write a bit-coded short (BS)
    pub fn write_bitshort(&mut self, value: i16) {
        if value == 0 {
            self.write_2bits(2); // prefix 10 → value is 0
        } else if value > 0 && value < 256 {
            self.write_2bits(1); // prefix 01 → unsigned char follows
            self.write_byte(value as u8);
        } else if value == 256 {
            self.write_2bits(3); // prefix 11 → value is 256
        } else {
            self.write_2bits(0); // prefix 00 → full raw short
            self.write_byte(value as u8);
            self.write_byte((value >> 8) as u8);
        }
    }

    /// Write a bit-coded long (BL)
    pub fn write_bitlong(&mut self, value: i32) {
        if value == 0 {
            self.write_2bits(2); // prefix 10 → value is 0
        } else if value > 0 && value < 256 {
            self.write_2bits(1); // prefix 01 → unsigned char follows
            self.write_byte(value as u8);
        } else {
            self.write_2bits(0); // prefix 00 → full raw long
            self.write_byte(value as u8);
            self.write_byte((value >> 8) as u8);
            self.write_byte((value >> 16) as u8);
            self.write_byte((value >> 24) as u8);
        }
    }

    /// Write a bit-coded long long (BLL) - R24+
    pub fn write_bitlonglong(&mut self, value: i64) {
        let unsigned = value as u64;
        let mut size = 0u8;
        let mut hold = unsigned;
        while hold != 0 {
            hold >>= 8;
            size += 1;
        }
        self.write_3bits(size);
        hold = unsigned;
        for _ in 0..size {
            self.write_byte((hold & 0xFF) as u8);
            hold >>= 8;
        }
    }

    /// Write a bit-coded double (BD)
    pub fn write_bitdouble(&mut self, value: f64) {
        if value == 0.0 {
            self.write_2bits(2); // prefix 10 → value is 0.0
        } else if value == 1.0 {
            self.write_2bits(1); // prefix 01 → value is 1.0
        } else {
            self.write_2bits(0); // prefix 00 → full RD follows
            self.write_raw_double(value);
        }
    }

    /// Write a bit-coded double with default (DD)
    pub fn write_bitdouble_with_default(&mut self, def: f64, value: f64) {
        if def == value {
            self.write_2bits(0); // 00 → use default
            return;
        }

        let def_bytes = def.to_le_bytes();
        let val_bytes = value.to_le_bytes();

        // Compare bytes symmetrically from edges inward
        let mut first = 0;
        let mut last = 7i32;
        while last >= 0 && def_bytes[last as usize] == val_bytes[last as usize] {
            first += 1;
            last -= 1;
        }

        if first >= 4 {
            // 01 → 4 bytes patch (first 4 bytes of value)
            self.write_2bits(1);
            self.write_bytes(&val_bytes[0..4]);
        } else if first >= 2 {
            // 10 → 6 bytes: [4],[5] then [0],[1],[2],[3]
            self.write_2bits(2);
            self.write_byte(val_bytes[4]);
            self.write_byte(val_bytes[5]);
            self.write_byte(val_bytes[0]);
            self.write_byte(val_bytes[1]);
            self.write_byte(val_bytes[2]);
            self.write_byte(val_bytes[3]);
        } else {
            // 11 → full RD
            self.write_2bits(3);
            self.write_raw_double(value);
        }
    }

    /// Write 2 bit-coded doubles (2BD)
    pub fn write_2bitdouble(&mut self, value: Vector2) {
        self.write_bitdouble(value.x);
        self.write_bitdouble(value.y);
    }

    /// Write 3 bit-coded doubles (3BD)
    pub fn write_3bitdouble(&mut self, value: Vector3) {
        self.write_bitdouble(value.x);
        self.write_bitdouble(value.y);
        self.write_bitdouble(value.z);
    }

    /// Write 2 bit-coded doubles with defaults (2DD)
    pub fn write_2bitdouble_with_default(&mut self, def: Vector2, value: Vector2) {
        self.write_bitdouble_with_default(def.x, value.x);
        self.write_bitdouble_with_default(def.y, value.y);
    }

    /// Write 3 bit-coded doubles with defaults (3DD)
    pub fn write_3bitdouble_with_default(&mut self, def: Vector3, value: Vector3) {
        self.write_bitdouble_with_default(def.x, value.x);
        self.write_bitdouble_with_default(def.y, value.y);
        self.write_bitdouble_with_default(def.z, value.z);
    }

    /// Write bit-coded extrusion (BE)
    /// R2000+: single bit (1 = default 0,0,1), or bit(0) + 3BD
    pub fn write_bit_extrusion(&mut self, normal: Vector3) {
        if self.version >= ACadVersion::AC1015 {
            // R2000+
            if normal.x == 0.0 && normal.y == 0.0 && normal.z == 1.0 {
                self.write_bit(true);
            } else {
                self.write_bit(false);
                self.write_3bitdouble(normal);
            }
        } else {
            // R13-R14: full 3BD
            self.write_3bitdouble(normal);
        }
    }

    /// Write bit-coded thickness (BT)
    /// R2000+: single bit (1 = 0.0), or bit(0) + BD
    pub fn write_bit_thickness(&mut self, thickness: f64) {
        if self.version >= ACadVersion::AC1015 {
            // R2000+
            if thickness == 0.0 {
                self.write_bit(true);
            } else {
                self.write_bit(false);
                self.write_bitdouble(thickness);
            }
        } else {
            // R13-R14: full BD
            self.write_bitdouble(thickness);
        }
    }

    // =========================================================================
    // Modular types
    // =========================================================================

    /// Write unsigned modular char (MC)
    /// Each byte: 7 data bits + bit7 continuation flag
    pub fn write_modular_char(&mut self, value: u64) {
        if value == 0 {
            self.write_byte(0);
            return;
        }
        let mut remaining = value;
        while remaining >= 0x80 {
            self.write_byte(((remaining & 0x7F) | 0x80) as u8);
            remaining >>= 7;
        }
        self.write_byte(remaining as u8);
    }

    /// Write signed modular char (SMC)
    /// Final byte: 6 data bits + bit6 sign flag + bit7=0
    pub fn write_signed_modular_char(&mut self, value: i64) {
        let negative = value < 0;
        let mut v = if negative { -value } else { value } as u64;

        // Write continuation bytes (7 data bits each)
        while v >= 64 {
            self.write_byte(((v & 0x7F) | 0x80) as u8);
            v >>= 7;
        }

        // Write final byte with sign in bit 6
        let mut final_byte = (v & 0x3F) as u8;
        if negative {
            final_byte |= 0x40;
        }
        self.write_byte(final_byte);
    }

    /// Write modular short (MS)
    /// Pairs of bytes: byte1 = 8 data bits, byte2 = 7 data bits + bit7 continuation
    pub fn write_modular_short(&mut self, value: i32) {
        let size = value as u32;
        if size >= 0x8000 {
            // Large value: 4 bytes
            self.write_byte((size & 0xFF) as u8);
            self.write_byte((((size >> 8) & 0x7F) | 0x80) as u8);
            self.write_byte(((size >> 15) & 0xFF) as u8);
            self.write_byte(((size >> 23) & 0xFF) as u8);
        } else {
            // Small value: 2 bytes
            self.write_byte((size & 0xFF) as u8);
            self.write_byte(((size >> 8) & 0xFF) as u8);
        }
    }

    // =========================================================================
    // Handle references
    // =========================================================================

    /// Write a handle reference with type code
    /// Format: first byte = [code:4bits][counter:4bits], then counter bytes big-endian
    pub fn write_handle_reference(&mut self, ref_type: DwgReferenceType, handle: u64) {
        let code = (ref_type as u8) << 4;

        if handle == 0 {
            self.write_byte(code);
        } else if handle < 0x100 {
            self.write_byte(code | 1);
            self.write_byte(handle as u8);
        } else if handle < 0x10000 {
            self.write_byte(code | 2);
            self.write_byte((handle >> 8) as u8);
            self.write_byte(handle as u8);
        } else if handle < 0x100_0000 {
            self.write_byte(code | 3);
            self.write_byte((handle >> 16) as u8);
            self.write_byte((handle >> 8) as u8);
            self.write_byte(handle as u8);
        } else if handle < 0x1_0000_0000 {
            self.write_byte(code | 4);
            self.write_byte((handle >> 24) as u8);
            self.write_byte((handle >> 16) as u8);
            self.write_byte((handle >> 8) as u8);
            self.write_byte(handle as u8);
        } else {
            // Handles > 32 bits (rare)
            let mut bytes = Vec::new();
            let mut h = handle;
            while h > 0 {
                bytes.push((h & 0xFF) as u8);
                h >>= 8;
            }
            let count = bytes.len() as u8;
            self.write_byte(code | count);
            for &b in bytes.iter().rev() {
                self.write_byte(b);
            }
        }
    }

    /// Write a handle with no reference type (type = 0)
    pub fn write_handle(&mut self, handle: u64) {
        self.write_handle_reference(DwgReferenceType::None, handle);
    }

    // =========================================================================
    // Text types
    // =========================================================================

    /// Write text (T) - BS length + ASCII bytes
    pub fn write_text(&mut self, value: &str) {
        if value.is_empty() {
            self.write_bitshort(0);
            return;
        }
        let bytes = value.as_bytes();
        self.write_bitshort(bytes.len() as i16);
        self.write_bytes(bytes);
    }

    /// Write Unicode text (TU) - RS length + UTF-16LE + null terminator
    pub fn write_text_unicode(&mut self, value: &str) {
        if value.is_empty() {
            self.write_raw_short(0);
            return;
        }
        let utf16: Vec<u16> = value.encode_utf16().collect();
        self.write_raw_short((utf16.len() + 1) as i16); // +1 for null terminator
        for &ch in &utf16 {
            let bytes = ch.to_le_bytes();
            self.write_byte(bytes[0]);
            self.write_byte(bytes[1]);
        }
        // Null terminator (2 bytes for UTF-16)
        self.write_byte(0);
        self.write_byte(0);
    }

    /// Write variable text (TV) - T for pre-R2007, TU for R2007+
    pub fn write_variable_text(&mut self, value: &str) {
        if self.version >= ACadVersion::AC1021 {
            // R2007+: Unicode as TU via bitshort length
            if value.is_empty() {
                self.write_bitshort(0);
                return;
            }
            let utf16: Vec<u16> = value.encode_utf16().collect();
            self.write_bitshort(utf16.len() as i16);
            for &ch in &utf16 {
                let bytes = ch.to_le_bytes();
                self.write_byte(bytes[0]);
                self.write_byte(bytes[1]);
            }
        } else {
            self.write_text(value);
        }
    }

    // =========================================================================
    // Sentinel
    // =========================================================================

    /// Write a 16-byte sentinel
    pub fn write_sentinel(&mut self, sentinel: &[u8; 16]) {
        self.flush_bits();
        self.write_bytes(sentinel);
    }

    // =========================================================================
    // Color types
    // =========================================================================

    /// Write color by index (BS)
    pub fn write_color_by_index(&mut self, color: &Color) {
        let index = match color {
            Color::ByLayer => 256i16,
            Color::ByBlock => 0,
            Color::Index(i) => *i as i16,
            Color::Rgb { .. } => 7, // Default to white for RGB
        };
        self.write_bitshort(index);
    }

    /// Write CMC color (version-aware)
    pub fn write_cmc_color(&mut self, color: &Color) {
        if self.version >= ACadVersion::AC1018 {
            // R2004+: BS(0) + BL(rgb) + RC(0)
            self.write_bitshort(0); // color index always 0

            let bl_value = match color {
                Color::ByLayer => 0xC0_00_00_00u32 as i32,
                Color::ByBlock => 0xC1_00_00_00u32 as i32,
                Color::Index(i) => {
                    let mut arr = [0u8; 4];
                    arr[0] = *i;
                    arr[3] = 0xC3;
                    i32::from_le_bytes(arr)
                }
                Color::Rgb { r, g, b } => {
                    let mut arr = [0u8; 4];
                    arr[0] = *b;
                    arr[1] = *g;
                    arr[2] = *r;
                    arr[3] = 0xC2;
                    i32::from_le_bytes(arr)
                }
            };
            self.write_bitlong(bl_value);
            self.write_byte(0); // color byte
        } else {
            // R15 and earlier: BS color index
            self.write_color_by_index(color);
        }
    }

    /// Write entity color (EnColor) - used in entity common data for R2004+
    pub fn write_en_color(&mut self, color: &Color, transparency: u8) {
        let has_transparency = transparency != 0xFF; // 0xFF = ByLayer
        match color {
            Color::ByBlock if !has_transparency => {
                self.write_bitshort(0);
            }
            Color::Rgb { r, g, b } => {
                let mut size: u16 = 0x8000; // true color flag
                if has_transparency {
                    size |= 0x2000;
                }
                self.write_bitshort(size as i16);
                let mut arr = [0u8; 4];
                arr[0] = *b;
                arr[1] = *g;
                arr[2] = *r;
                arr[3] = 0xC2;
                self.write_bitlong(i32::from_le_bytes(arr));
                if has_transparency {
                    self.write_bitlong((transparency as i32) | 0x02000000);
                }
            }
            _ => {
                let index = match color {
                    Color::ByLayer => 256u16,
                    Color::ByBlock => 0,
                    Color::Index(i) => *i as u16,
                    _ => 7,
                };
                let mut size = index;
                if has_transparency {
                    size |= 0x2000;
                }
                self.write_bitshort(size as i16);
                if has_transparency {
                    self.write_bitlong((transparency as i32) | 0x02000000);
                }
            }
        }
    }

    /// Write object type (BS for pre-R2010, special encoding for R2010+)
    pub fn write_object_type(&mut self, value: i16) {
        if self.version >= ACadVersion::AC1024 {
            // R2010+: special encoding
            if value <= 255 {
                self.write_2bits(0);
                self.write_byte(value as u8);
            } else if value >= 0x1F0 && value <= 0x2EF {
                self.write_2bits(1);
                self.write_byte((value - 0x1F0) as u8);
            } else {
                self.write_2bits(2);
                let bytes = value.to_le_bytes();
                self.write_byte(bytes[0]);
                self.write_byte(bytes[1]);
            }
        } else {
            self.write_bitshort(value);
        }
    }

    /// Write a julian date (2 raw longs: day number and fraction)
    pub fn write_julian_date(&mut self, value: f64) {
        let day = value as i32;
        let msec = ((value - day as f64) * 86400000.0) as i32;
        self.write_raw_long(day);
        self.write_raw_long(msec);
    }

    /// Write position by flag (for section position encoding in handles)
    pub fn write_position_by_flag(&mut self, pos: i64) {
        let pos = pos as u64;
        if pos >= 0x8000 {
            if pos >= 0x40000000 {
                let hi = ((pos >> 30) & 0xFFFF) as u16;
                self.write_raw_ushort(hi);
                let mid = (((pos >> 15) & 0x7FFF) | 0x8000) as u16;
                self.write_raw_ushort(mid);
            } else {
                let mid = ((pos >> 15) & 0xFFFF) as u16;
                self.write_raw_ushort(mid);
            }
            let lo = ((pos & 0x7FFF) | 0x8000) as u16;
            self.write_raw_ushort(lo);
        } else {
            self.write_raw_ushort(pos as u16);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ACadVersion;

    #[test]
    fn test_write_bit() {
        let mut w = DwgStreamWriter::new(ACadVersion::AC1018);
        w.write_bit(true);
        w.write_bit(false);
        w.write_bit(true);
        w.write_bit(true);
        w.write_bit(false);
        w.write_bit(false);
        w.write_bit(true);
        w.write_bit(false);
        assert_eq!(w.position, 1);
        assert_eq!(w.buffer[0], 0b10110010);
    }

    #[test]
    fn test_write_bitshort_zero() {
        let mut w = DwgStreamWriter::new(ACadVersion::AC1018);
        w.write_bitshort(0);
        // Should write 2 bits = 10 (prefix for 0)
        assert_eq!(w.bit_shift, 2);
        assert_eq!(w.last_byte, 0b10000000);
    }

    #[test]
    fn test_write_bitshort_small() {
        let mut w = DwgStreamWriter::new(ACadVersion::AC1018);
        w.write_bitshort(42);
        // prefix 01 + byte 42
        w.flush_bits();
        assert_eq!(w.buffer[0], 0b01_101010); // 01 prefix + first 6 bits of 42
        assert_eq!(w.buffer[1], 0b00_000000); // remaining 2 bits of 42
    }

    #[test]
    fn test_write_bitshort_256() {
        let mut w = DwgStreamWriter::new(ACadVersion::AC1018);
        w.write_bitshort(256);
        // Should write 2 bits = 11 (prefix for 256)
        assert_eq!(w.bit_shift, 2);
        assert_eq!(w.last_byte, 0b11000000);
    }

    #[test]
    fn test_write_handle() {
        let mut w = DwgStreamWriter::new(ACadVersion::AC1018);
        w.write_handle(0x1A);
        // code=0, counter=1 → first byte = 0x01, second = 0x1A
        w.flush_bits();
        assert_eq!(w.buffer[0..2], [0x01, 0x1A]);
    }

    #[test]
    fn test_write_modular_char() {
        let mut w = DwgStreamWriter::new(ACadVersion::AC1018);
        w.write_modular_char(0);
        assert_eq!(w.buffer[0], 0);

        let mut w = DwgStreamWriter::new(ACadVersion::AC1018);
        w.write_modular_char(127);
        assert_eq!(w.buffer[0], 127);

        let mut w = DwgStreamWriter::new(ACadVersion::AC1018);
        w.write_modular_char(128);
        assert_eq!(w.buffer[0..2], [0x80, 0x01]); // 128 = 0|0000000 1|0000000
    }

    #[test]
    fn test_write_text() {
        let mut w = DwgStreamWriter::new(ACadVersion::AC1018);
        w.write_text("AB");
        w.flush_bits();
        // BS(2) = prefix 01 + byte 2, then 'A' 'B'
        // 01_00000010 01000001 01000010
        assert_eq!(w.buffer.len(), 4); // 2bits + 8bits + 8bits + 8bits = 26 bits = 4 bytes
    }
}
