//! DWG classes section writer.

use std::io::{Cursor, Write};

use crate::classes::DxfClass;
use crate::error::Result;
use crate::io::dwg::dwg_section_io::DwgSectionContext;
use crate::io::dwg::{crc8_value, Crc8StreamHandler, DwgSectionDefinition, START_SENTINELS, END_SENTINELS};
use crate::types::DxfVersion;

use super::dwg_stream_writer_base::DwgStreamWriterBase;
use super::idwg_stream_writer::DwgStreamWriter;

pub struct DwgClassesWriter {
    ctx: DwgSectionContext,
    start_writer: Box<dyn DwgStreamWriter>,
    writer: Box<dyn DwgStreamWriter>,
    section_stream: Cursor<Vec<u8>>,
    classes: Vec<DxfClass>,
    version: DxfVersion,
    maintenance_version: i16,
}

impl DwgClassesWriter {
    pub fn new(
        version: DxfVersion,
        start_stream: &mut dyn Write,
        classes: Vec<DxfClass>,
        maintenance_version: i16,
    ) -> Self {
        // We'll produce section data into section_stream, then wrap it
        let section_stream = Cursor::new(Vec::new());
        Self {
            ctx: DwgSectionContext::new(version, DwgSectionDefinition::CLASSES),
            start_writer: DwgStreamWriterBase::get_stream_writer(version, Box::new(Cursor::new(Vec::new())), "windows-1252"),
            writer: DwgStreamWriterBase::get_merged_writer(version, Box::new(Cursor::new(Vec::new())), "windows-1252"),
            section_stream,
            classes,
            version,
            maintenance_version,
        }
    }

    pub fn write(
        version: DxfVersion,
        classes: &[DxfClass],
        maintenance_version: i16,
    ) -> Result<Vec<u8>> {
        let ctx = DwgSectionContext::new(version, DwgSectionDefinition::CLASSES);

        // Build section data
        let mut section_stream = Cursor::new(Vec::<u8>::new());
        let mut writer: Box<dyn DwgStreamWriter> =
            DwgStreamWriterBase::get_merged_writer(version, Box::new(Cursor::new(Vec::new())), "windows-1252");

        if ctx.r2007_plus {
            writer.save_position_for_size()?;
        }

        let max_class_number = classes.iter().map(|c| c.class_number).max().unwrap_or(0);

        if ctx.r2004_plus {
            writer.write_bit_short(max_class_number)?;
            writer.write_byte(0)?;
            writer.write_byte(0)?;
            writer.write_bit(true)?;
        }

        for c in classes {
            writer.write_bit_short(c.class_number)?;
            writer.write_bit_short(c.proxy_flags.0 as i16)?;
            writer.write_variable_text(&c.application_name)?;
            writer.write_variable_text(&c.cpp_class_name)?;
            writer.write_variable_text(&c.dxf_name)?;
            writer.write_bit(c.was_zombie)?;
            writer.write_bit_short(c.item_class_id)?;

            if ctx.r2004_plus {
                writer.write_bit_long(c.instance_count)?;
                writer.write_bit_long(0)?;
                writer.write_bit_long(0)?;
                writer.write_bit_long(0)?;
                writer.write_bit_long(0)?;
            }
        }

        writer.write_spear_shift()?;

        // Now build the final output with sentinels and CRC
        let mut output = Vec::new();
        let section_data = {
            // Get the written data from the merged writer's main stream
            let stream = writer.stream();
            let pos = stream.stream_position()?;
            stream.seek(std::io::SeekFrom::Start(0))?;
            let mut buf = Vec::new();
            std::io::Read::read_to_end(stream, &mut buf)?;
            buf
        };

        // Start sentinel
        let start_sentinel: [u8; 16] = [
            0x8D, 0xA1, 0xC4, 0xB8, 0xC4, 0xA9, 0xF8, 0xC5,
            0xC0, 0xDC, 0xF4, 0x5F, 0xE7, 0xCF, 0xB6, 0x8A,
        ];
        let end_sentinel: [u8; 16] = [
            0x72, 0x5E, 0x3B, 0x47, 0x3B, 0x56, 0x07, 0x3A,
            0x3F, 0x23, 0x0B, 0xA0, 0x18, 0x30, 0x49, 0x75,
        ];

        output.extend_from_slice(&start_sentinel);

        // CRC8 section: size + data
        let mut crc_data = Vec::new();
        crc_data.extend_from_slice(&(section_data.len() as i32).to_le_bytes());

        if (version >= DxfVersion::AC1024 && maintenance_version > 3)
            || version > DxfVersion::AC1027
        {
            crc_data.extend_from_slice(&0i32.to_le_bytes());
        }

        crc_data.extend_from_slice(&section_data);

        let crc = crc8_value(0xC0C1, &crc_data, 0, crc_data.len());
        output.extend_from_slice(&crc_data);
        output.extend_from_slice(&(crc as i16).to_le_bytes());

        output.extend_from_slice(&end_sentinel);

        if ctx.r2004_plus {
            output.extend_from_slice(&0i64.to_le_bytes());
        }

        Ok(output)
    }
}
