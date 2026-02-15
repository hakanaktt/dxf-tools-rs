//! DWG auxiliary header section writer.

use std::io::Cursor;

use crate::error::Result;
use crate::io::dwg::dwg_section_io::DwgSectionContext;
use crate::io::dwg::DwgSectionDefinition;
use crate::types::DxfVersion;

use super::dwg_stream_writer_base::DwgStreamWriterBase;
use super::idwg_stream_writer::DwgStreamWriter;

pub struct DwgAuxHeaderWriter;

impl DwgAuxHeaderWriter {
    pub fn write(
        version: DxfVersion,
        maintenance_version: i16,
        create_jdate: i32,
        create_msecs: i32,
        update_jdate: i32,
        update_msecs: i32,
        handle_seed: u64,
    ) -> Result<Vec<u8>> {
        let ctx = DwgSectionContext::new(version, DwgSectionDefinition::AUX_HEADER);
        let mut writer = DwgStreamWriterBase::get_stream_writer(version, Box::new(Cursor::new(Vec::new())), "windows-1252");

        // RC: 0xff 0x77 0x01
        writer.write_byte(0xFF)?;
        writer.write_byte(0x77)?;
        writer.write_byte(0x01)?;

        // RS: DWG version
        writer.write_raw_short(version as i16)?;
        // RS: Maintenance version
        writer.write_raw_short(maintenance_version)?;

        // RL: Number of saves (starts at 1)
        writer.write_raw_long(1)?;
        // RL: -1
        writer.write_raw_long(-1)?;

        // RS: Number of saves part 1
        writer.write_raw_short(1)?;
        // RS: Number of saves part 2
        writer.write_raw_short(0)?;

        // RL: 0
        writer.write_raw_long(0)?;
        // RS: DWG version string
        writer.write_raw_short(version as i16)?;
        // RS: Maintenance version
        writer.write_raw_short(maintenance_version)?;
        // RS: DWG version string
        writer.write_raw_short(version as i16)?;
        // RS: Maintenance version
        writer.write_raw_short(maintenance_version)?;

        // RS: 0x0005
        writer.write_raw_short(0x5)?;
        // RS: 0x0893
        writer.write_raw_short(2195)?;
        // RS: 0x0005
        writer.write_raw_short(5)?;
        // RS: 0x0893
        writer.write_raw_short(2195)?;
        // RS: 0x0000
        writer.write_raw_short(0)?;
        // RS: 0x0001
        writer.write_raw_short(1)?;
        // RL: 0x0000 (5 times)
        for _ in 0..5 {
            writer.write_raw_long(0)?;
        }

        // TD: TDCREATE
        writer.write_8_bit_julian_date(create_jdate, create_msecs)?;
        // TD: TDUPDATE
        writer.write_8_bit_julian_date(update_jdate, update_msecs)?;

        let handseed: i32 = if handle_seed <= 0x7FFF_FFFF {
            handle_seed as i32
        } else {
            -1
        };

        // RL: HANDSEED
        writer.write_raw_long(handseed as i64)?;
        // RL: Educational plot stamp (default 0)
        writer.write_raw_long(0)?;
        // RS: 0
        writer.write_raw_short(0)?;
        // RS: Number of saves part 1 - number of saves part 2
        writer.write_raw_short(1)?;
        // RL: 0 (4 times)
        for _ in 0..4 {
            writer.write_raw_long(0)?;
        }
        // RL: Number of saves
        writer.write_raw_long(1)?;
        // RL: 0 (4 times)
        for _ in 0..4 {
            writer.write_raw_long(0)?;
        }

        // R2018+
        if ctx.r2018_plus {
            writer.write_raw_short(0)?;
            writer.write_raw_short(0)?;
            writer.write_raw_short(0)?;
        }

        // Extract buffer
        let ws = writer.stream();
        ws.seek(std::io::SeekFrom::Start(0))?;
        let mut buf = Vec::new();
        std::io::Read::read_to_end(ws, &mut buf)?;
        Ok(buf)
    }
}
