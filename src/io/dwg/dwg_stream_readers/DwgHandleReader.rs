use std::collections::HashMap;

use crate::error::Result;

use super::idwg_stream_reader::DwgStreamReader;

/// Reads the handle map section from a DWG file.
///
/// The handle map maps object handles to their file offsets.
/// Matches the C# DwgHandleReader which reads BigEndian section sizes,
/// modular chars for handle offsets, and signed modular chars for location offsets.
pub struct DwgHandleReader;

impl DwgHandleReader {
    /// Read the complete handle-to-location map.
    ///
    /// C# logic: Repeat until section size==2 (the last empty section except CRC).
    /// Each section has BigEndian short for size, then handle/offset pairs.
    pub fn read(reader: &mut dyn DwgStreamReader) -> Result<HashMap<u64, i64>> {
        let mut object_map: HashMap<u64, i64> = HashMap::new();

        loop {
            // Set the "last handle" to 0 and "last loc" to 0
            let mut last_handle: u64 = 0;
            let mut last_loc: i64 = 0;

            // Short: size of this section (BIGENDIAN order, MSB first)
            let hi = reader.read_byte()? as i16;
            let lo = reader.read_byte()? as i16;
            let size = (hi << 8) | lo;

            if size == 2 {
                break;
            }

            let start_pos = reader.position()?;
            let mut max_section_offset = (size - 2) as i64;
            if max_section_offset > 2032 {
                max_section_offset = 2032;
            }

            let last_position = start_pos as i64 + max_section_offset;

            // Repeat until out of data for this section
            while (reader.position()? as i64) < last_position {
                // offset of this handle from last handle as modular char
                let offset = reader.read_modular_char()?;
                last_handle = last_handle.wrapping_add(offset);

                // offset of location in file from last loc as signed modular char
                last_loc = last_loc.wrapping_add(reader.read_signed_modular_char()?);

                if offset > 0 {
                    object_map.insert(last_handle, last_loc);
                }
            }

            // CRC (most significant byte followed by least significant byte)
            let _crc_hi = reader.read_byte()?;
            let _crc_lo = reader.read_byte()?;
        }

        Ok(object_map)
    }

    /// Simple helper to read a single handle reference.
    pub fn read_handle(reader: &mut dyn DwgStreamReader, owner: u64) -> Result<u64> {
        reader.handle_reference_from(owner)
    }

    /// Read multiple handle references.
    pub fn read_handles(reader: &mut dyn DwgStreamReader, owner: u64, count: usize) -> Result<Vec<u64>> {
        let mut handles = Vec::with_capacity(count);
        for _ in 0..count {
            handles.push(reader.handle_reference_from(owner)?);
        }
        Ok(handles)
    }
}
