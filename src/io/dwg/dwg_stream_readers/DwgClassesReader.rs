use crate::error::Result;

use super::idwg_stream_reader::DwgStreamReader;

/// Single class definition from DWG CLASSES section.
#[derive(Debug, Clone, Default)]
pub struct DwgClassDef {
    pub class_number: i16,
    pub proxy_cap_flags: i32,
    pub app_name: String,
    pub cplusplus_name: String,
    pub dxf_name: String,
    pub was_zombie: bool,
    pub item_class_id: i16,
}

/// Reads DWG class records.
pub struct DwgClassesReader;

impl DwgClassesReader {
    pub fn read(reader: &mut dyn DwgStreamReader) -> Result<Vec<DwgClassDef>> {
        let count = reader.read_bit_long()?.max(0) as usize;
        let mut classes = Vec::with_capacity(count);
        for _ in 0..count {
            classes.push(DwgClassDef {
                class_number: reader.read_bit_short()?,
                proxy_cap_flags: reader.read_bit_long()?,
                app_name: reader.read_variable_text()?,
                cplusplus_name: reader.read_variable_text()?,
                dxf_name: reader.read_variable_text()?,
                was_zombie: reader.read_bit()?,
                item_class_id: reader.read_bit_short()?,
            });
        }
        Ok(classes)
    }
}
