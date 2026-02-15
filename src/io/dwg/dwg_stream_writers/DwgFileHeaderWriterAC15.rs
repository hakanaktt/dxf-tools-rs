//! AC15 (R14/R15/R2000) file header writer — sequential record layout.

use std::collections::HashMap;
use std::io::{Cursor, Seek, SeekFrom, Write};

use crate::error::Result;
use crate::io::dwg::{crc8_value, DwgSectionDefinition, DwgSectionLocatorRecord};
use crate::types::DxfVersion;

use super::dwg_file_header_writer_base::{apply_mask, get_file_code_page};
use super::dwg_stream_writer_base::DwgStreamWriterBase;
use super::idwg_stream_writer::{DwgFileHeaderWriter, DwgStreamWriter};

const FILE_HEADER_SIZE: i64 = 0x61;

struct SectionEntry {
    record: DwgSectionLocatorRecord,
    data: Vec<u8>,
}

pub struct DwgFileHeaderWriterAc15 {
    stream: Box<dyn Write + Send>,
    version: DxfVersion,
    code_page: String,
    version_string: String,
    maintenance_version: i16,
    preview_address: i64,
    sections: Vec<(String, SectionEntry)>,
}

impl DwgFileHeaderWriterAc15 {
    pub fn new(
        stream: Box<dyn Write + Send>,
        version: DxfVersion,
        version_string: String,
        code_page: String,
        maintenance_version: i16,
    ) -> Self {
        // Pre-populate the standard section order
        let section_names = vec![
            (DwgSectionDefinition::HEADER.to_string(),       0),
            (DwgSectionDefinition::CLASSES.to_string(),      1),
            (DwgSectionDefinition::OBJ_FREE_SPACE.to_string(),3),
            (DwgSectionDefinition::TEMPLATE.to_string(),     4),
            (DwgSectionDefinition::AUX_HEADER.to_string(),   5),
            (DwgSectionDefinition::ACDB_OBJECTS.to_string(), -1), // no number
            (DwgSectionDefinition::HANDLES.to_string(),      2),
            (DwgSectionDefinition::PREVIEW.to_string(),      -1), // no number
        ];

        let sections = section_names
            .into_iter()
            .map(|(name, num)| {
                let rec = if num >= 0 {
                    DwgSectionLocatorRecord::with_number(Some(num))
                } else {
                    DwgSectionLocatorRecord::new()
                };
                (
                    name,
                    SectionEntry {
                        record: rec,
                        data: Vec::new(),
                    },
                )
            })
            .collect();

        Self {
            stream,
            version,
            code_page,
            version_string,
            maintenance_version,
            preview_address: 0,
            sections,
        }
    }

    fn find_section_mut(&mut self, name: &str) -> Option<&mut SectionEntry> {
        self.sections
            .iter_mut()
            .find(|(n, _)| n == name)
            .map(|(_, e)| e)
    }

    fn find_section(&self, name: &str) -> Option<&SectionEntry> {
        self.sections
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, e)| e)
    }

    fn set_record_seekers(&mut self) {
        let mut curr_offset = FILE_HEADER_SIZE;
        for (_, entry) in &mut self.sections {
            entry.record.seeker = curr_offset;
            curr_offset += entry.data.len() as i64;
        }
    }

    fn build_file_header(&self) -> Vec<u8> {
        let mut buf = Cursor::new(Vec::<u8>::new());
        let mut w = DwgStreamWriterBase::get_stream_writer(self.version, Box::new(Cursor::new(Vec::new())), "windows-1252");

        // 0x00  6  "ACXXXX" version string
        let ver_bytes = self.version_string.as_bytes();
        let _ = w.write_bytes(&ver_bytes[..6.min(ver_bytes.len())]);
        // pad to 6
        for _ in ver_bytes.len()..6 {
            let _ = w.write_byte(0);
        }

        // 0x06  7  5 zeros + maintenance + 0x01
        let _ = w.write_bytes(&[0, 0, 0, 0, 0, self.maintenance_version as u8, 1]);

        // 0x0D  4  Preview seeker
        let preview_seeker = self
            .find_section(DwgSectionDefinition::PREVIEW)
            .map(|e| e.record.seeker)
            .unwrap_or(0);
        let _ = w.write_raw_long(preview_seeker);

        let _ = w.write_byte(0x1B);
        let _ = w.write_byte(0x19);

        // 0x13  2  Code page
        let cp = get_file_code_page(&self.code_page);
        let _ = w.write_bytes(&cp.to_le_bytes());

        // Number of records
        let _ = w.write_bytes(&6i32.to_le_bytes());

        // Write each numbered record
        for (_, entry) in &self.sections {
            if let Some(num) = entry.record.number {
                let _ = w.write_byte(num as u8);
                let _ = w.write_raw_long(entry.record.seeker);
                let _ = w.write_raw_long(entry.record.size);
            }
        }

        // CRC
        let _ = w.write_spear_shift();
        let ws = w.stream();
        let _ = ws.seek(SeekFrom::Start(0));
        let mut hdr_data = Vec::new();
        let _ = std::io::Read::read_to_end(ws, &mut hdr_data);

        let crc = crc8_value(0xC0C1, &hdr_data, 0, hdr_data.len());
        hdr_data.extend_from_slice(&(crc as i16).to_le_bytes());

        // End sentinel
        let end_sentinel: [u8; 16] = [
            0x95, 0xA0, 0x4E, 0x28, 0x99, 0x82, 0x1A, 0xE5, 0x5E, 0x41, 0xE0, 0x5F, 0x9D, 0x3A,
            0x4D, 0x00,
        ];
        hdr_data.extend_from_slice(&end_sentinel);

        hdr_data
    }
}

impl DwgFileHeaderWriter for DwgFileHeaderWriterAc15 {
    fn handle_section_offset(&self) -> i32 {
        let mut offset = FILE_HEADER_SIZE;
        for (name, entry) in &self.sections {
            if name == DwgSectionDefinition::ACDB_OBJECTS {
                break;
            }
            offset += entry.data.len() as i64;
        }
        offset as i32
    }

    fn add_section(
        &mut self,
        name: &str,
        stream: Vec<u8>,
        _is_compressed: bool,
        _decomp_size: usize,
    ) {
        if let Some(entry) = self.find_section_mut(name) {
            entry.record.size = stream.len() as i64;
            entry.data = stream;
        }
    }

    fn write_file(&mut self) -> Result<()> {
        self.set_record_seekers();

        let header = self.build_file_header();
        self.stream.write_all(&header)?;

        for (_, entry) in &self.sections {
            self.stream.write_all(&entry.data)?;
        }

        Ok(())
    }
}
