use crate::error::Result;

use super::{dwg_object_reader::DwgRawObject, idwg_stream_reader::DwgStreamReader};

/// Non-entity object decoding helpers.
pub struct DwgObjectReaderObjects;

impl DwgObjectReaderObjects {
    pub fn read_object(reader: &mut dyn DwgStreamReader) -> Result<DwgRawObject> {
        super::dwg_object_reader::DwgObjectReader::read_one(reader)
    }
}
