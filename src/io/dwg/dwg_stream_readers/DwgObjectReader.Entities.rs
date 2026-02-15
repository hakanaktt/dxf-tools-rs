use crate::error::Result;

use super::{dwg_object_reader::DwgRawObject, idwg_stream_reader::DwgStreamReader};

/// Entity-specific object decoding helpers.
pub struct DwgObjectReaderEntities;

impl DwgObjectReaderEntities {
    pub fn read_entity(reader: &mut dyn DwgStreamReader) -> Result<DwgRawObject> {
        super::dwg_object_reader::DwgObjectReader::read_one(reader)
    }
}
