use crate::error::Result;
use crate::types::DxfVersion;

use super::idwg_stream_reader::DwgStreamReader;

/// Single class definition from DWG CLASSES section.
#[derive(Debug, Clone, Default)]
pub struct DwgClassDef {
    pub class_number: i16,
    pub proxy_cap_flags: i16,
    pub app_name: String,
    pub cplusplus_name: String,
    pub dxf_name: String,
    pub was_zombie: bool,
    pub item_class_id: i16,
    pub is_an_entity: bool,
    pub instance_count: i32,
    pub dwg_version: i32,
    pub maintenance_version: i32,
}

/// Reads DWG class records.
pub struct DwgClassesReader;

impl DwgClassesReader {
    pub fn read(reader: &mut dyn DwgStreamReader, version: DxfVersion) -> Result<Vec<DwgClassDef>> {
        // RL: size of class data area
        let size = reader.read_raw_long()? as u64;
        let end_section = reader.position()? + size;

        let mut classes = Vec::new();

        // Read until we exhaust the data (no class count field in the format)
        while Self::get_curr_pos(reader, version)? < end_section {
            let mut class_def = DwgClassDef {
                class_number: reader.read_bit_short()?,
                proxy_cap_flags: reader.read_bit_short()?,
                app_name: reader.read_variable_text()?,
                cplusplus_name: reader.read_variable_text()?,
                dxf_name: reader.read_variable_text()?,
                was_zombie: reader.read_bit()?,
                item_class_id: reader.read_bit_short()?,
                ..Default::default()
            };

            // Derive is_an_entity from item_class_id
            class_def.is_an_entity = class_def.item_class_id == 0x1F2;

            // R2004+ per-class fields
            if version >= DxfVersion::AC1018 {
                class_def.instance_count = reader.read_bit_long()?;
                class_def.dwg_version = reader.read_bit_long()?;
                class_def.maintenance_version = reader.read_bit_long()?;
                let _unknown1 = reader.read_bit_long()?;
                let _unknown2 = reader.read_bit_long()?;
            }

            classes.push(class_def);
        }

        // RS: CRC
        let _ = reader.reset_shift();

        Ok(classes)
    }

    fn get_curr_pos(reader: &mut dyn DwgStreamReader, version: DxfVersion) -> Result<u64> {
        if version >= DxfVersion::AC1021 {
            reader.position_in_bits()
        } else {
            reader.position()
        }
    }
}
