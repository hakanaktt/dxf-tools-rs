use crate::error::Result;

use super::idwg_stream_reader::{DwgObjectType, DwgStreamReader};

/// Raw object payload extracted from DWG objects section.
#[derive(Debug, Clone, Default)]
pub struct DwgRawObject {
    pub handle: u64,
    pub object_type: Option<DwgObjectType>,
    pub data: Vec<u8>,
}

/// Reads generic DWG objects from object map/data streams.
pub struct DwgObjectReader;

impl DwgObjectReader {
    pub fn read_one(reader: &mut dyn DwgStreamReader) -> Result<DwgRawObject> {
        let handle = reader.handle_reference()?;
        let object_type = Some(reader.read_object_type()?);
        let size = reader.read_bit_long()?.max(0) as usize;
        let data = reader.read_bytes(size)?;

        Ok(DwgRawObject {
            handle,
            object_type,
            data,
        })
    }
}
