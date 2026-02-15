//! DWG AppInfo section writer.

use std::io::{Cursor, Write};

use crate::error::Result;
use crate::io::dwg::dwg_section_io::DwgSectionContext;
use crate::io::dwg::DwgSectionDefinition;
use crate::types::DxfVersion;

use super::dwg_stream_writer_base::DwgStreamWriterBase;
use super::idwg_stream_writer::DwgStreamWriter;

pub struct DwgAppInfoWriter;

impl DwgAppInfoWriter {
    pub fn write(version: DxfVersion) -> Result<Vec<u8>> {
        let mut stream = Cursor::new(Vec::<u8>::new());
        let mut writer = DwgStreamWriterBase::get_stream_writer(version, Box::new(Cursor::new(Vec::new())), "windows-1252");

        let version_str = env!("CARGO_PKG_VERSION");
        let empty_arr = [0u8; 16];

        // UInt32 4 class_version (default: 3)
        writer.write_int(3)?;
        // String: App info name
        writer.write_text_unicode("AppInfoDataList")?;
        // UInt32 4 num strings (default: 3)
        writer.write_int(3)?;
        // Byte[] 16 Version data checksum
        writer.write_bytes(&empty_arr)?;
        // String: Version
        writer.write_text_unicode(version_str)?;
        // Byte[] 16 Comment data checksum
        writer.write_bytes(&empty_arr)?;
        // String: Comment
        writer.write_text_unicode("This file was written by acadrust")?;
        // Byte[] 16 Product data checksum
        writer.write_bytes(&empty_arr)?;
        // String: Product
        let product = format!(
            "<ProductInformation name =\"acadrust\" build_version=\"{}\" registry_version=\"{}\" install_id_string=\"acadrust\" registry_localeID=\"1033\"/>",
            version_str, version_str
        );
        writer.write_text_unicode(&product)?;

        // Get the data out
        let ws = writer.stream();
        ws.seek(std::io::SeekFrom::Start(0))?;
        let mut buf = Vec::new();
        std::io::Read::read_to_end(ws, &mut buf)?;
        Ok(buf)
    }
}
