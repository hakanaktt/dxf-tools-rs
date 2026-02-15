use crate::error::Result;

use super::{dwg_object_reader::DwgRawObject, idwg_stream_reader::DwgStreamReader};

/// Entity-specific object decoding helpers.
pub struct DwgObjectReaderEntities;

#[derive(Debug, Clone, Default)]
pub struct DwgTableCellContent {
    pub value_text: Option<String>,
    pub value_number: Option<f64>,
}

#[derive(Debug, Clone, Default)]
pub struct DwgTableCell {
    pub flags: i32,
    pub tooltip: Option<String>,
    pub custom_data_count: usize,
    pub contents: Vec<DwgTableCellContent>,
}

#[derive(Debug, Clone, Default)]
pub struct DwgTableEntityData {
    pub rows: i32,
    pub cols: i32,
    pub cells: Vec<DwgTableCell>,
}

impl DwgObjectReaderEntities {
    pub fn read_entity(reader: &mut dyn DwgStreamReader) -> Result<DwgRawObject> {
        super::dwg_object_reader::DwgObjectReader::read_one(reader)
    }

    pub fn read_table_entity(reader: &mut dyn DwgStreamReader) -> Result<DwgTableEntityData> {
        let rows = reader.read_bit_long()?;
        let cols = reader.read_bit_long()?;
        let count = (rows.max(0) * cols.max(0)) as usize;

        let mut cells = Vec::with_capacity(count);
        for _ in 0..count {
            cells.push(Self::read_table_cell(reader)?);
        }

        Ok(DwgTableEntityData { rows, cols, cells })
    }

    pub fn read_table_cell(reader: &mut dyn DwgStreamReader) -> Result<DwgTableCell> {
        let flags = reader.read_bit_long()?;
        let has_tooltip = reader.read_bit()?;
        let tooltip = if has_tooltip {
            Some(reader.read_variable_text()?)
        } else {
            None
        };

        let custom_data_count = reader.read_bit_long()?.max(0) as usize;
        let content_count = reader.read_bit_long()?.max(0) as usize;

        let mut contents = Vec::with_capacity(content_count);
        for _ in 0..content_count {
            contents.push(Self::read_table_cell_content(reader)?);
        }

        Ok(DwgTableCell {
            flags,
            tooltip,
            custom_data_count,
            contents,
        })
    }

    pub fn read_table_cell_content(reader: &mut dyn DwgStreamReader) -> Result<DwgTableCellContent> {
        let kind = reader.read_bit_short()?;
        let mut content = DwgTableCellContent::default();
        match kind {
            0 => {
                content.value_text = Some(reader.read_variable_text()?);
            }
            1 => {
                content.value_number = Some(reader.read_bit_double()?);
            }
            _ => {
                content.value_text = Some(reader.read_variable_text()?);
            }
        }
        Ok(content)
    }
}
