//! DWG preview (thumbnail) section writer.

use std::io::Write;

use crate::error::Result;
use crate::io::dwg::dwg_section_io::DwgSectionContext;
use crate::io::dwg::{DwgSectionDefinition, START_SENTINELS, END_SENTINELS};
use crate::types::DxfVersion;

use super::dwg_stream_writer_base::DwgStreamWriterBase;
use super::idwg_stream_writer::DwgStreamWriter;

/// Preview data for DWG files.
pub struct DwgPreview {
    pub code: u8,
    pub raw_header: Vec<u8>,
    pub raw_image: Vec<u8>,
}

pub struct DwgPreviewWriter;

impl DwgPreviewWriter {
    /// Write an empty preview section.
    pub fn write_empty(version: DxfVersion) -> Result<Vec<u8>> {
        let start_sentinel = START_SENTINELS
            .get(DwgSectionDefinition::PREVIEW)
            .copied()
            .unwrap_or([0u8; 16]);
        let end_sentinel = END_SENTINELS
            .get(DwgSectionDefinition::PREVIEW)
            .copied()
            .unwrap_or([0u8; 16]);

        let mut out = Vec::new();
        out.extend_from_slice(&start_sentinel);
        // overall size RL = 1
        out.extend_from_slice(&1i32.to_le_bytes());
        // images present RC = 0
        out.push(0);
        out.extend_from_slice(&end_sentinel);
        Ok(out)
    }

    /// Write a preview section with image data.
    pub fn write_with_preview(
        version: DxfVersion,
        preview: &DwgPreview,
        start_pos: i64,
    ) -> Result<Vec<u8>> {
        let start_sentinel = START_SENTINELS
            .get(DwgSectionDefinition::PREVIEW)
            .copied()
            .unwrap_or([0u8; 16]);
        let end_sentinel = END_SENTINELS
            .get(DwgSectionDefinition::PREVIEW)
            .copied()
            .unwrap_or([0u8; 16]);

        let size = preview.raw_header.len() + preview.raw_image.len() + 19;

        let mut out = Vec::new();
        out.extend_from_slice(&start_sentinel);

        // overall size RL
        out.extend_from_slice(&(size as i32).to_le_bytes());
        // images present RC = 2
        out.push(2);

        // Code RC = 1 (header)
        out.push(1);
        // header data start
        let header_offset = start_pos + out.len() as i64 + 12 + 5 + 32;
        out.extend_from_slice(&(header_offset as i32).to_le_bytes());
        // header data size
        out.extend_from_slice(&(preview.raw_header.len() as i32).to_le_bytes());

        // Code RC
        out.push(preview.code);
        // image data start
        let image_offset = header_offset + preview.raw_header.len() as i64;
        out.extend_from_slice(&(image_offset as i32).to_le_bytes());
        // image data size
        out.extend_from_slice(&(preview.raw_image.len() as i32).to_le_bytes());

        out.extend_from_slice(&preview.raw_header);
        out.extend_from_slice(&preview.raw_image);

        out.extend_from_slice(&end_sentinel);
        Ok(out)
    }
}
