use crate::error::Result;

use super::{dwg_object_reader::DwgRawObject, idwg_stream_reader::DwgStreamReader};

/// Non-entity object decoding helpers.
pub struct DwgObjectReaderObjects;

#[derive(Debug, Clone, Default)]
pub struct DwgEvaluationExpression {
    pub value_code: i32,
    pub value_text: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct DwgBlockElementData {
    pub flags: i32,
    pub grip_count: i32,
}

impl DwgObjectReaderObjects {
    pub fn read_object(reader: &mut dyn DwgStreamReader) -> Result<DwgRawObject> {
        super::dwg_object_reader::DwgObjectReader::read_one(reader)
    }

    pub fn read_block_element(reader: &mut dyn DwgStreamReader) -> Result<DwgBlockElementData> {
        let flags = reader.read_bit_long()?;
        let grip_count = reader.read_bit_long()?;

        for _ in 0..grip_count.max(0) {
            let _grip = reader.read_3_bit_double()?;
        }

        Ok(DwgBlockElementData { flags, grip_count })
    }

    pub fn read_evaluation_expression(
        reader: &mut dyn DwgStreamReader,
    ) -> Result<DwgEvaluationExpression> {
        let value_code = reader.read_bit_long()?;
        let has_text = reader.read_bit()?;
        let value_text = if has_text {
            Some(reader.read_variable_text()?)
        } else {
            None
        };

        Ok(DwgEvaluationExpression {
            value_code,
            value_text,
        })
    }
}
