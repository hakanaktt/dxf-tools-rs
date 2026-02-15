//! Handle section writer — reverse of `DwgReader::read_handles()`.
//!
//! Writes the sorted handle → offset map with modular short encoding
//! and CRC8 per 2032-byte block.

use std::collections::BTreeMap;
use std::io::{Cursor, Write};

use crate::error::Result;
use crate::io::dwg::{crc8_value, DwgSectionDefinition};
use crate::io::dwg::dwg_section_io::DwgSectionContext;
use crate::types::DxfVersion;

pub struct DwgHandleWriter {
    ctx: DwgSectionContext,
    stream: Cursor<Vec<u8>>,
    handle_map: BTreeMap<u64, i64>, // sorted by key
}

impl DwgHandleWriter {
    pub fn new(version: DxfVersion, stream: Cursor<Vec<u8>>, map: BTreeMap<u64, i64>) -> Self {
        Self {
            ctx: DwgSectionContext::new(version, DwgSectionDefinition::HANDLES),
            stream,
            handle_map: map,
        }
    }

    /// `section_offset`: For R18+ the offset is relative, for earlier it is absolute.
    pub fn write(&mut self, section_offset: i32) -> Result<()> {
        let mut arr = [0u8; 10];
        let mut arr2 = [0u8; 5];

        let mut offset: u64 = 0;
        let mut initial_loc: i64 = 0;

        let last_position = self.stream.position();

        self.stream.write_all(&[0, 0])?;

        // Clone keys to avoid borrow conflict
        let entries: Vec<(u64, i64)> = self.handle_map.iter().map(|(&k, &v)| (k, v)).collect();

        for (handle, loc_value) in &entries {
            let mut handle_off = handle - offset;
            let last_loc = loc_value + section_offset as i64;
            let mut loc_diff = last_loc - initial_loc;

            let mut offset_size = Self::modular_short_to_value(handle_off, &mut arr);
            let mut loc_size = Self::signed_modular_short_to_value(loc_diff as i32, &mut arr2);

            if self.stream.position() - last_position + (offset_size + loc_size) as u64 > 2032 {
                self.process_position(last_position)?;
                offset = 0;
                initial_loc = 0;
                let last_position_new = self.stream.position();
                self.stream.write_all(&[0, 0])?;
                handle_off = handle - offset;

                if handle_off == 0 {
                    return Err(crate::error::DxfError::Custom(
                        "Handle offset is 0 in handle writer".into(),
                    ));
                }

                loc_diff = last_loc - initial_loc;
                offset_size = Self::modular_short_to_value(handle_off, &mut arr);
                loc_size = Self::signed_modular_short_to_value(loc_diff as i32, &mut arr2);

                // process from the new position next time
                self.write_chunk(&arr, offset_size, &arr2, loc_size)?;
                offset = *handle;
                initial_loc = last_loc;
                continue;
            }

            self.write_chunk(&arr, offset_size, &arr2, loc_size)?;
            offset = *handle;
            initial_loc = last_loc;
        }

        self.process_position(last_position)?;
        let last_position = self.stream.position();
        self.stream.write_all(&[0, 0])?;
        self.process_position(last_position)?;

        Ok(())
    }

    fn write_chunk(&mut self, arr: &[u8], offset_size: usize, arr2: &[u8], loc_size: usize) -> Result<()> {
        self.stream.write_all(&arr[..offset_size])?;
        self.stream.write_all(&arr2[..loc_size])?;
        Ok(())
    }

    /// Unsigned modular short encoding.
    fn modular_short_to_value(mut value: u64, arr: &mut [u8]) -> usize {
        let mut i = 0;
        while value >= 0b1000_0000 {
            arr[i] = ((value & 0b111_1111) | 0b1000_0000) as u8;
            i += 1;
            value >>= 7;
        }
        arr[i] = value as u8;
        i + 1
    }

    /// Signed modular short encoding.
    fn signed_modular_short_to_value(mut value: i32, arr: &mut [u8]) -> usize {
        let mut i = 0;
        if value < 0 {
            value = -value;
            while value >= 64 {
                arr[i] = ((value as u32 & 0x7F) | 0x80) as u8;
                i += 1;
                value >>= 7;
            }
            arr[i] = (value as u32 | 0x40) as u8;
            return i + 1;
        }

        while value >= 0b100_0000 {
            arr[i] = ((value as u32 & 0x7F) | 0x80) as u8;
            i += 1;
            value >>= 7;
        }
        arr[i] = value as u8;
        i + 1
    }

    fn process_position(&mut self, pos: u64) -> Result<()> {
        let diff = (self.stream.position() - pos) as u16;
        let stream_pos = self.stream.position();

        // go back and write the size
        self.stream.set_position(pos);
        self.stream.write_all(&[(diff >> 8) as u8, (diff & 0xFF) as u8])?;
        self.stream.set_position(stream_pos);

        // CRC
        let buf = self.stream.get_ref();
        let crc = crc8_value(0xC0C1, buf, pos as usize, (buf.len() - pos as usize));
        self.stream.write_all(&[(crc >> 8) as u8, (crc & 0xFF) as u8])?;

        Ok(())
    }

    /// Consume and return the underlying buffer.
    pub fn into_inner(self) -> Vec<u8> {
        self.stream.into_inner()
    }
}
