use std::collections::{BTreeMap, HashSet, VecDeque};
use std::io::Cursor;

use crate::{
    error::Result,
    io::dxf::GroupCodeValueType,
    types::{Color, DxfVersion, Transparency, Vector2, Vector3},
};

use super::{
    dwg_stream_reader_base::DwgStreamReaderBase,
    idwg_stream_reader::{DwgObjectType, DwgStreamReader},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawObjectType {
    Text,
    Attrib,
    AttDef,
    Block,
    EndBlk,
    SeqEnd,
    Insert,
    MInsert,
    Vertex2D,
    Vertex3D,
    VertexPFace,
    VertexMesh,
    Polyline2D,
    Polyline3D,
    Arc,
    Circle,
    Line,
    Point,
    Face3D,
    PolylinePFace,
    PolylineMesh,
    Solid,
    Trace,
    Shape,
    Viewport,
    Ellipse,
    Spline,
    Region,
    Solid3D,
    Body,
    Ray,
    XLine,
    Dictionary,
    DictionaryWithDefault,
    MText,
    Leader,
    Hatch,
    ProxyEntity,
    ProxyObject,
    MultiLeader,
    Tolerance,
    MLine,
    OLEFrame,
    OLE2Frame,
    Dummy,
    LongTransaction,
    LwPolyline,
    XRecord,
    Layout,
    Unknown(u16),
}

impl RawObjectType {
    pub fn from_code(code: u16) -> Self {
        // Ported from ACadSharp Types/ObjectType.cs fixed object IDs.
        match code {
            0x01 => Self::Text,
            0x02 => Self::Attrib,
            0x03 => Self::AttDef,
            0x04 => Self::Block,
            0x05 => Self::EndBlk,
            0x06 => Self::SeqEnd,
            0x07 => Self::Insert,
            0x08 => Self::MInsert,
            0x0A => Self::Vertex2D,
            0x0B => Self::Vertex3D,
            0x0C => Self::VertexMesh,
            0x0D => Self::VertexPFace,
            0x0F => Self::Polyline2D,
            0x10 => Self::Polyline3D,
            0x11 => Self::Arc,
            0x12 => Self::Circle,
            0x13 => Self::Line,
            0x1B => Self::Point,
            0x1C => Self::Face3D,
            0x1D => Self::PolylinePFace,
            0x1E => Self::PolylineMesh,
            0x1F => Self::Solid,
            0x20 => Self::Trace,
            0x21 => Self::Shape,
            0x22 => Self::Viewport,
            0x23 => Self::Ellipse,
            0x24 => Self::Spline,
            0x25 => Self::Region,
            0x26 => Self::Solid3D,
            0x27 => Self::Body,
            0x28 => Self::Ray,
            0x29 => Self::XLine,
            0x2A => Self::Dictionary,
            0x2B => Self::OLEFrame,
            0x2C => Self::MText,
            0x2D => Self::Leader,
            0x2E => Self::Tolerance,
            0x2F => Self::MLine,
            0x4A => Self::OLE2Frame,
            0x4B => Self::Dummy,
            0x4C => Self::LongTransaction,
            0x4D => Self::LwPolyline,
            0x4E => Self::Hatch,
            0x4F => Self::XRecord,
            0x52 => Self::Layout,
            0x01F2 => Self::ProxyEntity,
            0x01F3 => Self::ProxyObject,
            other => Self::Unknown(other),
        }
    }

    pub fn from_code_for_version(version: DxfVersion, code: u16) -> Self {
        let resolved = Self::from_code(code);
        match (version, resolved) {
            // Proxy IDs and several object classes appear only on newer releases.
            (v, Self::ProxyEntity) if v < DxfVersion::AC1015 => Self::Unknown(code),
            (v, Self::ProxyObject) if v < DxfVersion::AC1015 => Self::Unknown(code),
            (v, Self::Hatch) if v < DxfVersion::AC1015 => Self::Unknown(code),
            _ => resolved,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct DwgExtendedDataRecord {
    pub code: i32,
    pub text: Option<String>,
    pub number: Option<f64>,
    pub integer: Option<i64>,
    pub bytes: Vec<u8>,
    pub point: Option<Vector3>,
}

#[derive(Debug, Clone, Default)]
pub struct DwgRawObject {
    pub handle: u64,
    pub object_type: Option<DwgObjectType>,
    pub raw_type: Option<RawObjectType>,
    pub data: Vec<u8>,
    pub owner_handle: Option<u64>,
    pub reactors: Vec<u64>,
    pub xdict_handle: Option<u64>,
    pub color: Option<Color>,
    pub transparency: Option<Transparency>,
    pub line_type_scale: Option<f64>,
    pub line_weight: Option<i16>,
    pub eed: BTreeMap<u64, Vec<DwgExtendedDataRecord>>,
    pub int_props: BTreeMap<String, i64>,
    pub float_props: BTreeMap<String, f64>,
    pub bool_props: BTreeMap<String, bool>,
    pub text_props: BTreeMap<String, String>,
    pub point2_props: BTreeMap<String, Vector2>,
    pub point3_props: BTreeMap<String, Vector3>,
    pub handle_props: BTreeMap<String, u64>,
    pub handle_list_props: BTreeMap<String, Vec<u64>>,
    pub binary_props: BTreeMap<String, Vec<u8>>,
}

pub struct DwgObjectReader {
    version: DxfVersion,
    buffer: Vec<u8>,
    handles: VecDeque<u64>,
    map: BTreeMap<u64, i64>,
    read_objects: HashSet<u64>,
    classes: BTreeMap<i16, String>,
}

impl DwgObjectReader {
    pub fn new(
        version: DxfVersion,
        buffer: Vec<u8>,
        handles: VecDeque<u64>,
        handle_map: BTreeMap<u64, i64>,
    ) -> Self {
        Self {
            version,
            buffer,
            handles,
            map: handle_map,
            read_objects: HashSet::new(),
            classes: BTreeMap::new(),
        }
    }

    pub fn with_classes(mut self, classes: BTreeMap<i16, String>) -> Self {
        self.classes = classes;
        self
    }

    /// Compatibility helper retained from phase 1.
    pub fn read_one(reader: &mut dyn DwgStreamReader) -> Result<DwgRawObject> {
        let handle = reader.handle_reference()?;
        let object_type = Some(reader.read_object_type()?);
        let size = reader.read_bit_long()?.max(0) as usize;
        let data = reader.read_bytes(size)?;

        Ok(DwgRawObject {
            handle,
            object_type,
            raw_type: object_type.map(|t| RawObjectType::from_code(t.0)),
            data,
            ..Default::default()
        })
    }

    /// Semantic port of ACadSharp object section traversal.
    pub fn read(&mut self) -> Result<Vec<DwgRawObject>> {
        let mut out = Vec::new();

        while let Some(handle) = self.handles.pop_front() {
            if self.read_objects.contains(&handle) {
                continue;
            }

            let Some(offset) = self.map.get(&handle).copied() else {
                continue;
            };

            let parsed = self.get_entity_type(offset)?;
            self.read_objects.insert(handle);

            if let Some(mut obj) = self.read_object(parsed, handle)? {
                obj.handle = handle;
                out.push(obj);
            }
        }

        Ok(out)
    }

    fn get_entity_type(&self, offset: i64) -> Result<ParsedObjectStreams> {
        let mut crc_reader = DwgStreamReaderBase::new(Box::new(Cursor::new(self.buffer.clone())));
        crc_reader.set_position(offset as u64)?;

        let size = crc_reader.read_modular_short()? as u32;
        if size == 0 {
            return Ok(ParsedObjectStreams::empty());
        }

        let size_in_bits = size << 3;

        let mut object_reader = DwgStreamReaderBase::new(Box::new(Cursor::new(self.buffer.clone())));
        object_reader.set_position_in_bits(crc_reader.position_in_bits()?)?;
        let object_initial_pos = object_reader.position_in_bits()?;
        let object_type = object_reader.read_object_type()?;

        if self.r2010_plus() {
            let handle_size = crc_reader.read_modular_char()? as u64;
            let handle_section_offset = crc_reader.position_in_bits()? + size_in_bits as u64 - handle_size;

            let mut handles_reader =
                DwgStreamReaderBase::new(Box::new(Cursor::new(self.buffer.clone())));
            handles_reader.set_position_in_bits(handle_section_offset)?;

            let mut text_reader = DwgStreamReaderBase::new(Box::new(Cursor::new(self.buffer.clone())));
            let _ = text_reader.set_position_by_flag(handle_section_offset.saturating_sub(1));

            Ok(ParsedObjectStreams {
                object_initial_pos,
                size,
                object_type,
                object_reader,
                handles_reader,
                text_reader,
            })
        } else {
            let handles_reader = DwgStreamReaderBase::new(Box::new(Cursor::new(self.buffer.clone())));
            let text_reader = DwgStreamReaderBase::new(Box::new(Cursor::new(self.buffer.clone())));

            Ok(ParsedObjectStreams {
                object_initial_pos,
                size,
                object_type,
                object_reader,
                handles_reader,
                text_reader,
            })
        }
    }

    fn read_object(
        &mut self,
        mut parsed: ParsedObjectStreams,
        current_handle: u64,
    ) -> Result<Option<DwgRawObject>> {
        let mut raw_type = RawObjectType::from_code_for_version(self.version, parsed.object_type.0);
        if let Some(name) = self.classes.get(&(parsed.object_type.0 as i16)) {
            if matches!(raw_type, RawObjectType::Unknown(_)) {
                if name.eq_ignore_ascii_case("MULTILEADER") {
                    raw_type = RawObjectType::MultiLeader;
                } else if name.eq_ignore_ascii_case("DICTIONARYWDFLT")
                    || name.eq_ignore_ascii_case("ACDBDICTIONARYWDFLT")
                {
                    raw_type = RawObjectType::DictionaryWithDefault;
                } else if name.eq_ignore_ascii_case("XRECORD") {
                    raw_type = RawObjectType::XRecord;
                }
            }
        }

        let mut template = DwgRawObject {
            handle: current_handle,
            object_type: Some(parsed.object_type),
            raw_type: Some(raw_type),
            ..Default::default()
        };
        match raw_type {
            RawObjectType::Text | RawObjectType::Attrib | RawObjectType::AttDef => {
                self.read_text_like(&mut parsed, &mut template, raw_type)?;
            }
            RawObjectType::Insert | RawObjectType::MInsert => {
                self.read_insert_like(&mut parsed, &mut template, raw_type)?;
            }
            RawObjectType::Hatch => {
                self.read_hatch(&mut parsed, &mut template)?;
            }
            RawObjectType::Dictionary => {
                self.read_dictionary(&mut parsed, &mut template)?;
            }
            RawObjectType::DictionaryWithDefault => {
                self.read_dictionary_with_default(&mut parsed, &mut template)?;
            }
            RawObjectType::ProxyEntity => {
                self.read_proxy_entity(&mut parsed, &mut template)?;
            }
            RawObjectType::ProxyObject => {
                self.read_proxy_object(&mut parsed, &mut template)?;
            }
            RawObjectType::MultiLeader => {
                self.read_multi_leader(&mut parsed, &mut template)?;
            }
            RawObjectType::MText => {
                self.read_mtext(&mut parsed, &mut template, true)?;
            }
            RawObjectType::XRecord => {
                self.read_xrecord(&mut parsed, &mut template)?;
            }
            RawObjectType::Leader => {
                self.read_leader(&mut parsed, &mut template)?;
            }
            RawObjectType::Block
            | RawObjectType::EndBlk
            | RawObjectType::SeqEnd
            | RawObjectType::Vertex2D
            | RawObjectType::Vertex3D
            | RawObjectType::VertexPFace
            | RawObjectType::VertexMesh
            | RawObjectType::Polyline2D
            | RawObjectType::Polyline3D
            | RawObjectType::Arc
            | RawObjectType::Circle
            | RawObjectType::Line
            | RawObjectType::Point
            | RawObjectType::Face3D
            | RawObjectType::PolylinePFace
            | RawObjectType::PolylineMesh
            | RawObjectType::Solid
            | RawObjectType::Trace
            | RawObjectType::Shape
            | RawObjectType::Viewport
            | RawObjectType::Ellipse
            | RawObjectType::Spline
            | RawObjectType::Region
            | RawObjectType::Solid3D
            | RawObjectType::Body
            | RawObjectType::Ray
            | RawObjectType::XLine
            | RawObjectType::Tolerance
            | RawObjectType::MLine
            | RawObjectType::OLEFrame
            | RawObjectType::OLE2Frame
            | RawObjectType::Dummy
            | RawObjectType::LongTransaction
            | RawObjectType::LwPolyline
            | RawObjectType::Layout
            | RawObjectType::Unknown(_) => {
                if matches!(raw_type, RawObjectType::Dictionary | RawObjectType::Unknown(_)) {
                    self.read_common_non_entity_data(&mut parsed, &mut template)?;
                } else {
                    self.read_common_entity_data(&mut parsed, &mut template)?;
                }
            }
        }

        // Best-effort raw payload extraction after semantic fields.
        let end_bits = parsed.object_initial_pos + (parsed.size as u64 * 8);
        let current_bits = parsed.object_reader.position_in_bits()?;
        if template.data.is_empty() && end_bits > current_bits {
            let remaining = ((end_bits - current_bits) / 8) as usize;
            template.data = parsed.object_reader.read_bytes(remaining)?;
        }

        Ok(Some(template))
    }

    fn read_text_like(
        &mut self,
        parsed: &mut ParsedObjectStreams,
        template: &mut DwgRawObject,
        kind: RawObjectType,
    ) -> Result<()> {
        self.read_common_text_data(parsed, template)?;

        if matches!(kind, RawObjectType::Attrib | RawObjectType::AttDef) {
            self.read_common_att_data(parsed, template)?;
        }
        if matches!(kind, RawObjectType::AttDef) {
            if self.r2010_plus() {
                template
                    .int_props
                    .insert("attdef_version".to_string(), parsed.object_reader.read_byte()? as i64);
            }
            template
                .text_props
                .insert("attdef_prompt".to_string(), parsed.text_reader.read_variable_text()?);
        }

        Ok(())
    }

    fn read_common_text_data(
        &mut self,
        parsed: &mut ParsedObjectStreams,
        template: &mut DwgRawObject,
    ) -> Result<()> {
        self.read_common_entity_data(parsed, template)?;

        let mut elevation = 0.0;

        if self.r13_14_only() {
            elevation = parsed.object_reader.read_bit_double()?;
            let ins = parsed.object_reader.read_2_raw_double()?;
            template
                .point3_props
                .insert("insert_point".to_string(), Vector3::new(ins.x, ins.y, elevation));

            let align = parsed.object_reader.read_2_raw_double()?;
            template
                .point3_props
                .insert("alignment_point".to_string(), Vector3::new(align.x, align.y, elevation));

            template
                .point3_props
                .insert("normal".to_string(), parsed.object_reader.read_3_bit_double()?);
            template
                .float_props
                .insert("thickness".to_string(), parsed.object_reader.read_bit_double()?);
            template
                .float_props
                .insert("oblique_angle".to_string(), parsed.object_reader.read_bit_double()?);
            template
                .float_props
                .insert("rotation".to_string(), parsed.object_reader.read_bit_double()?);
            template
                .float_props
                .insert("height".to_string(), parsed.object_reader.read_bit_double()?);
            template
                .float_props
                .insert("width_factor".to_string(), parsed.object_reader.read_bit_double()?);
            template
                .text_props
                .insert("value".to_string(), parsed.text_reader.read_variable_text()?);
            template
                .int_props
                .insert("mirror".to_string(), parsed.object_reader.read_bit_short()? as i64);
            template.int_props.insert(
                "horizontal_alignment".to_string(),
                parsed.object_reader.read_bit_short()? as i64,
            );
            template.int_props.insert(
                "vertical_alignment".to_string(),
                parsed.object_reader.read_bit_short()? as i64,
            );

            template
                .handle_props
                .insert("style_handle".to_string(), self.handle_reference(parsed, 0)?);
            return Ok(());
        }

        let data_flags = parsed.object_reader.read_byte()?;
        if (data_flags & 0x1) == 0 {
            elevation = parsed.object_reader.read_double()?;
        }

        let ins = parsed.object_reader.read_2_raw_double()?;
        template
            .point3_props
            .insert("insert_point".to_string(), Vector3::new(ins.x, ins.y, elevation));

        if (data_flags & 0x2) == 0 {
            let x = parsed.object_reader.read_bit_double_with_default(ins.x)?;
            let y = parsed.object_reader.read_bit_double_with_default(ins.y)?;
            template
                .point3_props
                .insert("alignment_point".to_string(), Vector3::new(x, y, elevation));
        }

        template
            .point3_props
            .insert("normal".to_string(), parsed.object_reader.read_bit_extrusion()?);
        template
            .float_props
            .insert("thickness".to_string(), parsed.object_reader.read_bit_thickness()?);

        if (data_flags & 0x4) == 0 {
            template
                .float_props
                .insert("oblique_angle".to_string(), parsed.object_reader.read_double()?);
        }
        if (data_flags & 0x8) == 0 {
            template
                .float_props
                .insert("rotation".to_string(), parsed.object_reader.read_double()?);
        }
        template
            .float_props
            .insert("height".to_string(), parsed.object_reader.read_double()?);
        if (data_flags & 0x10) == 0 {
            template
                .float_props
                .insert("width_factor".to_string(), parsed.object_reader.read_double()?);
        }

        template
            .text_props
            .insert("value".to_string(), parsed.text_reader.read_variable_text()?);

        if (data_flags & 0x20) == 0 {
            template
                .int_props
                .insert("mirror".to_string(), parsed.object_reader.read_bit_short()? as i64);
        }
        if (data_flags & 0x40) == 0 {
            template.int_props.insert(
                "horizontal_alignment".to_string(),
                parsed.object_reader.read_bit_short()? as i64,
            );
        }
        if (data_flags & 0x80) == 0 {
            template.int_props.insert(
                "vertical_alignment".to_string(),
                parsed.object_reader.read_bit_short()? as i64,
            );
        }

        template
            .handle_props
            .insert("style_handle".to_string(), self.handle_reference(parsed, 0)?);

        Ok(())
    }

    fn read_common_att_data(
        &mut self,
        parsed: &mut ParsedObjectStreams,
        template: &mut DwgRawObject,
    ) -> Result<()> {
        if self.r2010_plus() {
            template
                .int_props
                .insert("attribute_version".to_string(), parsed.object_reader.read_byte()? as i64);
        }

        if self.version >= DxfVersion::AC1032 {
            template
                .int_props
                .insert("attribute_type".to_string(), parsed.object_reader.read_byte()? as i64);
        }

        template
            .text_props
            .insert("attribute_tag".to_string(), parsed.text_reader.read_variable_text()?);
        template
            .int_props
            .insert("attribute_field_length".to_string(), parsed.object_reader.read_bit_short()? as i64);
        template
            .int_props
            .insert("attribute_flags".to_string(), parsed.object_reader.read_byte()? as i64);

        if self.r2007_plus() {
            template
                .bool_props
                .insert("attribute_lock_position".to_string(), parsed.object_reader.read_bit()?);
        }

        Ok(())
    }

    fn read_insert_like(
        &mut self,
        parsed: &mut ParsedObjectStreams,
        template: &mut DwgRawObject,
        kind: RawObjectType,
    ) -> Result<()> {
        self.read_insert_common_data(parsed, template)?;

        if matches!(kind, RawObjectType::MInsert) {
            template
                .int_props
                .insert("column_count".to_string(), parsed.object_reader.read_bit_short()? as i64);
            template
                .int_props
                .insert("row_count".to_string(), parsed.object_reader.read_bit_short()? as i64);
            template
                .float_props
                .insert("column_spacing".to_string(), parsed.object_reader.read_bit_double()?);
            template
                .float_props
                .insert("row_spacing".to_string(), parsed.object_reader.read_bit_double()?);
        }

        self.read_insert_common_handles(parsed, template)
    }

    fn read_insert_common_data(
        &mut self,
        parsed: &mut ParsedObjectStreams,
        template: &mut DwgRawObject,
    ) -> Result<()> {
        self.read_common_entity_data(parsed, template)?;

        template
            .point3_props
            .insert("insert_point".to_string(), parsed.object_reader.read_3_bit_double()?);

        if self.r13_14_only() {
            let scale = parsed.object_reader.read_3_bit_double()?;
            template.float_props.insert("x_scale".to_string(), scale.x);
            template.float_props.insert("y_scale".to_string(), scale.y);
            template.float_props.insert("z_scale".to_string(), scale.z);
        }

        if self.version >= DxfVersion::AC1015 {
            match parsed.object_reader.read_2_bits()? {
                0 => {
                    let x = parsed.object_reader.read_double()?;
                    let y = parsed.object_reader.read_bit_double_with_default(x)?;
                    let z = parsed.object_reader.read_bit_double_with_default(x)?;
                    template.float_props.insert("x_scale".to_string(), x);
                    template.float_props.insert("y_scale".to_string(), y);
                    template.float_props.insert("z_scale".to_string(), z);
                }
                1 => {
                    let x = 1.0;
                    let y = parsed.object_reader.read_bit_double_with_default(x)?;
                    let z = parsed.object_reader.read_bit_double_with_default(x)?;
                    template.float_props.insert("x_scale".to_string(), x);
                    template.float_props.insert("y_scale".to_string(), y);
                    template.float_props.insert("z_scale".to_string(), z);
                }
                2 => {
                    let x = parsed.object_reader.read_double()?;
                    template.float_props.insert("x_scale".to_string(), x);
                    template.float_props.insert("y_scale".to_string(), x);
                    template.float_props.insert("z_scale".to_string(), x);
                }
                _ => {
                    template.float_props.insert("x_scale".to_string(), 1.0);
                    template.float_props.insert("y_scale".to_string(), 1.0);
                    template.float_props.insert("z_scale".to_string(), 1.0);
                }
            }
        }

        template
            .float_props
            .insert("rotation".to_string(), parsed.object_reader.read_bit_double()?);
        template
            .point3_props
            .insert("normal".to_string(), parsed.object_reader.read_3_bit_double()?);

        let has_atts = parsed.object_reader.read_bit()?;
        template.bool_props.insert("has_attributes".to_string(), has_atts);
        if self.r2004_plus() && has_atts {
            template
                .int_props
                .insert("owned_object_count".to_string(), parsed.object_reader.read_bit_long()? as i64);
        }

        Ok(())
    }

    fn read_insert_common_handles(
        &mut self,
        parsed: &mut ParsedObjectStreams,
        template: &mut DwgRawObject,
    ) -> Result<()> {
        template
            .handle_props
            .insert("block_header_handle".to_string(), self.handle_reference(parsed, 0)?);

        let has_atts = *template.bool_props.get("has_attributes").unwrap_or(&false);
        if !has_atts {
            return Ok(());
        }

        if self.version >= DxfVersion::AC1012 && self.version <= DxfVersion::AC1015 {
            template
                .handle_props
                .insert("first_attribute_handle".to_string(), self.handle_reference(parsed, 0)?);
            template
                .handle_props
                .insert("last_attribute_handle".to_string(), self.handle_reference(parsed, 0)?);
        } else if self.r2004_plus() {
            let count = *template.int_props.get("owned_object_count").unwrap_or(&0) as usize;
            let mut handles = Vec::with_capacity(count);
            for _ in 0..count {
                handles.push(self.handle_reference(parsed, 0)?);
            }
            template
                .handle_list_props
                .insert("owned_object_handles".to_string(), handles);
        }

        template
            .handle_props
            .insert("seqend_handle".to_string(), self.handle_reference(parsed, 0)?);
        Ok(())
    }

    fn read_dictionary(
        &mut self,
        parsed: &mut ParsedObjectStreams,
        template: &mut DwgRawObject,
    ) -> Result<()> {
        self.read_common_non_entity_data(parsed, template)?;

        let nentries = parsed.object_reader.read_bit_long()?.max(0) as usize;
        if self.version == DxfVersion::AC1014 {
            let _ = parsed.object_reader.read_byte()?;
        }
        if self.version >= DxfVersion::AC1015 {
            template.int_props.insert(
                "dictionary_cloning_flags".to_string(),
                parsed.object_reader.read_bit_short()? as i64,
            );
            template.bool_props.insert(
                "dictionary_hard_owner_flag".to_string(),
                parsed.object_reader.read_byte()? > 0,
            );
        }

        let mut names = Vec::new();
        let mut handles = Vec::new();
        for _ in 0..nentries {
            let name = parsed.text_reader.read_variable_text()?;
            let handle = self.handle_reference(parsed, 0)?;
            if handle != 0 && !name.is_empty() {
                names.push(name);
                handles.push(handle);
            }
        }
        template
            .text_props
            .insert("dictionary_entry_names".to_string(), names.join("\u{1f}"));
        template
            .handle_list_props
            .insert("dictionary_entry_handles".to_string(), handles);

        Ok(())
    }

    fn read_dictionary_with_default(
        &mut self,
        parsed: &mut ParsedObjectStreams,
        template: &mut DwgRawObject,
    ) -> Result<()> {
        self.read_dictionary(parsed, template)?;
        template
            .handle_props
            .insert("dictionary_default_entry_handle".to_string(), self.handle_reference(parsed, 0)?);
        Ok(())
    }

    fn read_mtext(
        &mut self,
        parsed: &mut ParsedObjectStreams,
        template: &mut DwgRawObject,
        read_common_data: bool,
    ) -> Result<()> {
        if read_common_data {
            self.read_common_entity_data(parsed, template)?;
        }

        template
            .point3_props
            .insert("mtext_insert_point".to_string(), parsed.object_reader.read_3_bit_double()?);
        template
            .point3_props
            .insert("mtext_normal".to_string(), parsed.object_reader.read_3_bit_double()?);
        template
            .point3_props
            .insert("mtext_x_axis_dir".to_string(), parsed.object_reader.read_3_bit_double()?);
        template.float_props.insert(
            "mtext_rect_width".to_string(),
            parsed.object_reader.read_bit_double()?,
        );
        if self.r2007_plus() {
            template.float_props.insert(
                "mtext_rect_height".to_string(),
                parsed.object_reader.read_bit_double()?,
            );
        }
        template
            .float_props
            .insert("mtext_height".to_string(), parsed.object_reader.read_bit_double()?);
        template.int_props.insert(
            "mtext_attachment".to_string(),
            parsed.object_reader.read_bit_short()? as i64,
        );
        template.int_props.insert(
            "mtext_drawing_dir".to_string(),
            parsed.object_reader.read_bit_short()? as i64,
        );
        let _ = parsed.object_reader.read_bit_double()?;
        let _ = parsed.object_reader.read_bit_double()?;
        template
            .text_props
            .insert("mtext_value".to_string(), parsed.text_reader.read_variable_text()?);
        template
            .handle_props
            .insert("mtext_style_handle".to_string(), self.handle_reference(parsed, 0)?);

        if self.version >= DxfVersion::AC1015 {
            template.int_props.insert(
                "mtext_line_spacing_style".to_string(),
                parsed.object_reader.read_bit_short()? as i64,
            );
            template.float_props.insert(
                "mtext_line_spacing".to_string(),
                parsed.object_reader.read_bit_double()?,
            );
            let _ = parsed.object_reader.read_bit()?;
        }

        Ok(())
    }

    fn read_leader(
        &mut self,
        parsed: &mut ParsedObjectStreams,
        template: &mut DwgRawObject,
    ) -> Result<()> {
        self.read_common_entity_data(parsed, template)?;
        let _ = parsed.object_reader.read_bit()?;
        template
            .int_props
            .insert("leader_creation_type".to_string(), parsed.object_reader.read_bit_short()? as i64);
        template
            .int_props
            .insert("leader_path_type".to_string(), parsed.object_reader.read_bit_short()? as i64);

        let npts = parsed.object_reader.read_bit_long()?.max(0) as usize;
        let mut pts = Vec::with_capacity(npts);
        for _ in 0..npts {
            pts.push(parsed.object_reader.read_3_bit_double()?);
        }
        template
            .int_props
            .insert("leader_vertex_count".to_string(), npts as i64);
        if let Some(first) = pts.first().copied() {
            template
                .point3_props
                .insert("leader_first_vertex".to_string(), first);
        }

        let _ = parsed.object_reader.read_3_bit_double()?;
        template
            .point3_props
            .insert("leader_normal".to_string(), parsed.object_reader.read_3_bit_double()?);
        template.point3_props.insert(
            "leader_horizontal_dir".to_string(),
            parsed.object_reader.read_3_bit_double()?,
        );
        template
            .point3_props
            .insert("leader_block_offset".to_string(), parsed.object_reader.read_3_bit_double()?);

        if self.version >= DxfVersion::AC1014 {
            template.point3_props.insert(
                "leader_annotation_offset".to_string(),
                parsed.object_reader.read_3_bit_double()?,
            );
        }

        template.bool_props.insert(
            "leader_hook_line_same_dir".to_string(),
            parsed.object_reader.read_bit()?,
        );
        template.bool_props.insert(
            "leader_arrow_enabled".to_string(),
            parsed.object_reader.read_bit()?,
        );

        template
            .handle_props
            .insert("leader_annotation_handle".to_string(), self.handle_reference(parsed, 0)?);
        template
            .handle_props
            .insert("leader_dimstyle_handle".to_string(), self.handle_reference(parsed, 0)?);
        Ok(())
    }

    fn read_multi_leader(
        &mut self,
        parsed: &mut ParsedObjectStreams,
        template: &mut DwgRawObject,
    ) -> Result<()> {
        self.read_common_entity_data(parsed, template)?;

        if self.r2010_plus() {
            template
                .int_props
                .insert("mleader_version".to_string(), parsed.object_reader.read_bit_short()? as i64);
        }

        self.read_multi_leader_annot_context(parsed, template)?;

        template
            .handle_props
            .insert("mleader_style_handle".to_string(), self.handle_reference(parsed, 0)?);
        template
            .int_props
            .insert("mleader_prop_override".to_string(), parsed.object_reader.read_bit_long()? as i64);
        template
            .int_props
            .insert("mleader_line_type".to_string(), parsed.object_reader.read_bit_short()? as i64);

        template.color = Some(parsed.object_reader.read_cm_color(false)?);
        template
            .handle_props
            .insert("mleader_line_type_handle".to_string(), self.handle_reference(parsed, 0)?);
        template.int_props.insert(
            "mleader_line_weight".to_string(),
            parsed.object_reader.read_bit_long()? as i64,
        );
        template
            .bool_props
            .insert("mleader_enable_landing".to_string(), parsed.object_reader.read_bit()?);
        template
            .bool_props
            .insert("mleader_enable_dogleg".to_string(), parsed.object_reader.read_bit()?);
        template.float_props.insert(
            "mleader_landing_distance".to_string(),
            parsed.object_reader.read_bit_double()?,
        );

        template
            .handle_props
            .insert("mleader_arrowhead_handle".to_string(), self.handle_reference(parsed, 0)?);

        template.float_props.insert(
            "mleader_arrowhead_size".to_string(),
            parsed.object_reader.read_bit_double()?,
        );
        template
            .int_props
            .insert("mleader_content_type".to_string(), parsed.object_reader.read_bit_short()? as i64);

        template
            .handle_props
            .insert("mleader_mtext_style_handle".to_string(), self.handle_reference(parsed, 0)?);
        template.int_props.insert(
            "mleader_text_left_attachment".to_string(),
            parsed.object_reader.read_bit_short()? as i64,
        );
        template.int_props.insert(
            "mleader_text_right_attachment".to_string(),
            parsed.object_reader.read_bit_short()? as i64,
        );
        template.int_props.insert(
            "mleader_text_angle".to_string(),
            parsed.object_reader.read_bit_short()? as i64,
        );
        template.int_props.insert(
            "mleader_text_alignment".to_string(),
            parsed.object_reader.read_bit_short()? as i64,
        );
        let _ = parsed.object_reader.read_cm_color(false)?;
        template
            .bool_props
            .insert("mleader_text_frame".to_string(), parsed.object_reader.read_bit()?);
        template
            .handle_props
            .insert("mleader_block_content_handle".to_string(), self.handle_reference(parsed, 0)?);
        let _ = parsed.object_reader.read_cm_color(false)?;
        template
            .point3_props
            .insert("mleader_block_content_scale".to_string(), parsed.object_reader.read_3_bit_double()?);
        template.float_props.insert(
            "mleader_block_content_rotation".to_string(),
            parsed.object_reader.read_bit_double()?,
        );
        template.int_props.insert(
            "mleader_block_connection".to_string(),
            parsed.object_reader.read_bit_short()? as i64,
        );
        template
            .bool_props
            .insert("mleader_enable_annotation_scale".to_string(), parsed.object_reader.read_bit()?);

        if self.version < DxfVersion::AC1021 {
            let arrow_head_count = parsed.object_reader.read_bit_long()?.max(0) as usize;
            for idx in 0..arrow_head_count {
                let is_default = parsed.object_reader.read_bit()?;
                template
                    .bool_props
                    .insert(format!("mleader_arrowhead_default_{idx}"), is_default);
                let h = self.handle_reference(parsed, 0)?;
                template
                    .handle_list_props
                    .entry("mleader_arrowhead_handles".to_string())
                    .or_default()
                    .push(h);
            }
        }

        let block_label_count = parsed.object_reader.read_bit_long()?.max(0) as usize;
        for i in 0..block_label_count {
            let h = self.handle_reference(parsed, 0)?;
            template
                .handle_list_props
                .entry("mleader_block_attribute_handles".to_string())
                .or_default()
                .push(h);
            let txt = parsed.text_reader.read_variable_text()?;
            template
                .text_props
                .insert(format!("mleader_block_attribute_text_{i}"), txt);
            template.int_props.insert(
                format!("mleader_block_attribute_index_{i}"),
                parsed.object_reader.read_bit_short()? as i64,
            );
            template.float_props.insert(
                format!("mleader_block_attribute_width_{i}"),
                parsed.object_reader.read_bit_double()?,
            );
        }

        template
            .bool_props
            .insert("mleader_text_direction_negative".to_string(), parsed.object_reader.read_bit()?);
        template.int_props.insert(
            "mleader_text_align_in_ipe".to_string(),
            parsed.object_reader.read_bit_short()? as i64,
        );
        template.int_props.insert(
            "mleader_text_attachment_point".to_string(),
            parsed.object_reader.read_bit_short()? as i64,
        );
        template
            .float_props
            .insert("mleader_scale_factor".to_string(), parsed.object_reader.read_bit_double()?);

        if self.r2010_plus() {
            template.int_props.insert(
                "mleader_text_attachment_direction".to_string(),
                parsed.object_reader.read_bit_short()? as i64,
            );
            template.int_props.insert(
                "mleader_text_bottom_attachment".to_string(),
                parsed.object_reader.read_bit_short()? as i64,
            );
            template.int_props.insert(
                "mleader_text_top_attachment".to_string(),
                parsed.object_reader.read_bit_short()? as i64,
            );
        }
        if self.r2013_plus() {
            template
                .bool_props
                .insert("mleader_extended_to_text".to_string(), parsed.object_reader.read_bit()?);
        }

        Ok(())
    }

    fn read_multi_leader_annot_context(
        &mut self,
        parsed: &mut ParsedObjectStreams,
        template: &mut DwgRawObject,
    ) -> Result<()> {
        let mut leader_root_count = parsed.object_reader.read_bit_long()?;
        if leader_root_count == 0 {
            let _b0 = parsed.object_reader.read_bit()?;
            let _b1 = parsed.object_reader.read_bit()?;
            let _b2 = parsed.object_reader.read_bit()?;
            let _b3 = parsed.object_reader.read_bit()?;
            let _b4 = parsed.object_reader.read_bit()?;
            let b5 = parsed.object_reader.read_bit()?;
            let _b6 = parsed.object_reader.read_bit()?;
            leader_root_count = if b5 { 2 } else { 1 };
        }
        template
            .int_props
            .insert("mleader_context_root_count".to_string(), leader_root_count as i64);

        for idx in 0..leader_root_count.max(0) as usize {
            self.read_mleader_root(parsed, template, idx)?;
        }

        template.float_props.insert(
            "mleader_ctx_scale_factor".to_string(),
            parsed.object_reader.read_bit_double()?,
        );
        template
            .point3_props
            .insert("mleader_ctx_content_base_point".to_string(), parsed.object_reader.read_3_bit_double()?);
        template.float_props.insert(
            "mleader_ctx_text_height".to_string(),
            parsed.object_reader.read_bit_double()?,
        );
        template.float_props.insert(
            "mleader_ctx_arrowhead_size".to_string(),
            parsed.object_reader.read_bit_double()?,
        );
        template
            .float_props
            .insert("mleader_ctx_landing_gap".to_string(), parsed.object_reader.read_bit_double()?);
        template.int_props.insert(
            "mleader_ctx_text_left_attachment".to_string(),
            parsed.object_reader.read_bit_short()? as i64,
        );
        template.int_props.insert(
            "mleader_ctx_text_right_attachment".to_string(),
            parsed.object_reader.read_bit_short()? as i64,
        );
        template.int_props.insert(
            "mleader_ctx_text_alignment".to_string(),
            parsed.object_reader.read_bit_short()? as i64,
        );
        template.int_props.insert(
            "mleader_ctx_block_content_connection".to_string(),
            parsed.object_reader.read_bit_short()? as i64,
        );

        let has_text_contents = parsed.object_reader.read_bit()?;
        template
            .bool_props
            .insert("mleader_ctx_has_text_contents".to_string(), has_text_contents);
        if has_text_contents {
            template
                .text_props
                .insert("mleader_ctx_text_label".to_string(), parsed.text_reader.read_variable_text()?);
            template
                .point3_props
                .insert("mleader_ctx_text_normal".to_string(), parsed.object_reader.read_3_bit_double()?);
            template
                .handle_props
                .insert("mleader_ctx_text_style_handle".to_string(), self.handle_reference(parsed, 0)?);
            template
                .point3_props
                .insert("mleader_ctx_text_location".to_string(), parsed.object_reader.read_3_bit_double()?);
            template
                .point3_props
                .insert("mleader_ctx_direction".to_string(), parsed.object_reader.read_3_bit_double()?);
            template
                .float_props
                .insert("mleader_ctx_text_rotation".to_string(), parsed.object_reader.read_bit_double()?);
            template
                .float_props
                .insert("mleader_ctx_boundary_width".to_string(), parsed.object_reader.read_bit_double()?);
            template
                .float_props
                .insert("mleader_ctx_boundary_height".to_string(), parsed.object_reader.read_bit_double()?);
            template
                .float_props
                .insert("mleader_ctx_line_spacing_factor".to_string(), parsed.object_reader.read_bit_double()?);
            template.int_props.insert(
                "mleader_ctx_line_spacing_style".to_string(),
                parsed.object_reader.read_bit_short()? as i64,
            );
            let _ = parsed.object_reader.read_cm_color(false)?;
            template.int_props.insert(
                "mleader_ctx_text_attachment_point".to_string(),
                parsed.object_reader.read_bit_short()? as i64,
            );
            template.int_props.insert(
                "mleader_ctx_flow_direction".to_string(),
                parsed.object_reader.read_bit_short()? as i64,
            );
            let _ = parsed.object_reader.read_cm_color(false)?;
            template.float_props.insert(
                "mleader_ctx_background_scale_factor".to_string(),
                parsed.object_reader.read_bit_double()?,
            );
            template.int_props.insert(
                "mleader_ctx_background_transparency".to_string(),
                parsed.object_reader.read_bit_long()? as i64,
            );
            template
                .bool_props
                .insert("mleader_ctx_background_fill_enabled".to_string(), parsed.object_reader.read_bit()?);
            template
                .bool_props
                .insert("mleader_ctx_background_mask_fill_on".to_string(), parsed.object_reader.read_bit()?);
            template.int_props.insert(
                "mleader_ctx_column_type".to_string(),
                parsed.object_reader.read_bit_short()? as i64,
            );
            template
                .bool_props
                .insert("mleader_ctx_text_height_auto".to_string(), parsed.object_reader.read_bit()?);
            template.float_props.insert(
                "mleader_ctx_column_width".to_string(),
                parsed.object_reader.read_bit_double()?,
            );
            template.float_props.insert(
                "mleader_ctx_column_gutter".to_string(),
                parsed.object_reader.read_bit_double()?,
            );
            template
                .bool_props
                .insert("mleader_ctx_column_flow_reversed".to_string(), parsed.object_reader.read_bit()?);

            let col_count = parsed.object_reader.read_bit_long()?.max(0) as usize;
            template
                .int_props
                .insert("mleader_ctx_column_size_count".to_string(), col_count as i64);
            for i in 0..col_count {
                template.float_props.insert(
                    format!("mleader_ctx_column_size_{i}"),
                    parsed.object_reader.read_bit_double()?,
                );
            }
            template
                .bool_props
                .insert("mleader_ctx_word_break".to_string(), parsed.object_reader.read_bit()?);
            let _ = parsed.object_reader.read_bit()?;
        } else {
            let has_contents_block = parsed.object_reader.read_bit()?;
            template
                .bool_props
                .insert("mleader_ctx_has_contents_block".to_string(), has_contents_block);
            if has_contents_block {
                template
                    .handle_props
                    .insert("mleader_ctx_block_record_handle".to_string(), self.handle_reference(parsed, 0)?);
                template.point3_props.insert(
                    "mleader_ctx_block_normal".to_string(),
                    parsed.object_reader.read_3_bit_double()?,
                );
                template.point3_props.insert(
                    "mleader_ctx_block_location".to_string(),
                    parsed.object_reader.read_3_bit_double()?,
                );
                template.point3_props.insert(
                    "mleader_ctx_block_scale".to_string(),
                    parsed.object_reader.read_3_bit_double()?,
                );
                template.float_props.insert(
                    "mleader_ctx_block_rotation".to_string(),
                    parsed.object_reader.read_bit_double()?,
                );
                let _ = parsed.object_reader.read_cm_color(false)?;

                for i in 0..16 {
                    template.float_props.insert(
                        format!("mleader_ctx_transform_{i}"),
                        parsed.object_reader.read_bit_double()?,
                    );
                }
            }
        }

        template
            .point3_props
            .insert("mleader_ctx_base_point".to_string(), parsed.object_reader.read_3_bit_double()?);
        template.point3_props.insert(
            "mleader_ctx_base_direction".to_string(),
            parsed.object_reader.read_3_bit_double()?,
        );
        template.point3_props.insert(
            "mleader_ctx_base_vertical".to_string(),
            parsed.object_reader.read_3_bit_double()?,
        );
        template
            .bool_props
            .insert("mleader_ctx_normal_reversed".to_string(), parsed.object_reader.read_bit()?);

        if self.r2010_plus() {
            template.int_props.insert(
                "mleader_ctx_top_attachment".to_string(),
                parsed.object_reader.read_bit_short()? as i64,
            );
            template.int_props.insert(
                "mleader_ctx_bottom_attachment".to_string(),
                parsed.object_reader.read_bit_short()? as i64,
            );
        }

        Ok(())
    }

    fn read_mleader_root(
        &mut self,
        parsed: &mut ParsedObjectStreams,
        template: &mut DwgRawObject,
        root_index: usize,
    ) -> Result<()> {
        template.bool_props.insert(
            format!("mleader_root_{root_index}_content_valid"),
            parsed.object_reader.read_bit()?,
        );
        template.bool_props.insert(
            format!("mleader_root_{root_index}_unknown"),
            parsed.object_reader.read_bit()?,
        );
        template.point3_props.insert(
            format!("mleader_root_{root_index}_connection_point"),
            parsed.object_reader.read_3_bit_double()?,
        );
        template.point3_props.insert(
            format!("mleader_root_{root_index}_direction"),
            parsed.object_reader.read_3_bit_double()?,
        );

        let pair_count = parsed.object_reader.read_bit_long()?.max(0) as usize;
        template.int_props.insert(
            format!("mleader_root_{root_index}_break_pair_count"),
            pair_count as i64,
        );
        for i in 0..pair_count {
            template.point3_props.insert(
                format!("mleader_root_{root_index}_break_start_{i}"),
                parsed.object_reader.read_3_bit_double()?,
            );
            template.point3_props.insert(
                format!("mleader_root_{root_index}_break_end_{i}"),
                parsed.object_reader.read_3_bit_double()?,
            );
        }

        template.int_props.insert(
            format!("mleader_root_{root_index}_leader_index"),
            parsed.object_reader.read_bit_long()? as i64,
        );
        template.float_props.insert(
            format!("mleader_root_{root_index}_landing_distance"),
            parsed.object_reader.read_bit_double()?,
        );

        let line_count = parsed.object_reader.read_bit_long()?.max(0) as usize;
        template.int_props.insert(
            format!("mleader_root_{root_index}_line_count"),
            line_count as i64,
        );
        for i in 0..line_count {
            self.read_mleader_line(parsed, template, root_index, i)?;
        }

        if self.r2010_plus() {
            template.int_props.insert(
                format!("mleader_root_{root_index}_text_attachment_direction"),
                parsed.object_reader.read_bit_short()? as i64,
            );
        }

        Ok(())
    }

    fn read_mleader_line(
        &mut self,
        parsed: &mut ParsedObjectStreams,
        template: &mut DwgRawObject,
        root_index: usize,
        line_index: usize,
    ) -> Result<()> {
        let point_count = parsed.object_reader.read_bit_long()?.max(0) as usize;
        template.int_props.insert(
            format!("mleader_root_{root_index}_line_{line_index}_point_count"),
            point_count as i64,
        );
        for i in 0..point_count {
            template.point3_props.insert(
                format!("mleader_root_{root_index}_line_{line_index}_point_{i}"),
                parsed.object_reader.read_3_bit_double()?,
            );
        }

        let break_info_count = parsed.object_reader.read_bit_long()?;
        template.int_props.insert(
            format!("mleader_root_{root_index}_line_{line_index}_break_info_count"),
            break_info_count as i64,
        );
        if break_info_count > 0 {
            template.int_props.insert(
                format!("mleader_root_{root_index}_line_{line_index}_segment_index"),
                parsed.object_reader.read_bit_long()? as i64,
            );
            let sep_count = parsed.object_reader.read_bit_long()?.max(0) as usize;
            template.int_props.insert(
                format!("mleader_root_{root_index}_line_{line_index}_start_end_count"),
                sep_count as i64,
            );
            for i in 0..sep_count {
                template.point3_props.insert(
                    format!("mleader_root_{root_index}_line_{line_index}_start_{i}"),
                    parsed.object_reader.read_3_bit_double()?,
                );
                template.point3_props.insert(
                    format!("mleader_root_{root_index}_line_{line_index}_end_{i}"),
                    parsed.object_reader.read_3_bit_double()?,
                );
            }
        }

        template.int_props.insert(
            format!("mleader_root_{root_index}_line_{line_index}_index"),
            parsed.object_reader.read_bit_long()? as i64,
        );

        if self.r2010_plus() {
            template.int_props.insert(
                format!("mleader_root_{root_index}_line_{line_index}_path_type"),
                parsed.object_reader.read_bit_short()? as i64,
            );
            let _ = parsed.object_reader.read_cm_color(false)?;
            template.handle_props.insert(
                format!("mleader_root_{root_index}_line_{line_index}_line_type_handle"),
                self.handle_reference(parsed, 0)?,
            );
            template.int_props.insert(
                format!("mleader_root_{root_index}_line_{line_index}_line_weight"),
                parsed.object_reader.read_bit_long()? as i64,
            );
            template.float_props.insert(
                format!("mleader_root_{root_index}_line_{line_index}_arrow_size"),
                parsed.object_reader.read_bit_double()?,
            );
            template.handle_props.insert(
                format!("mleader_root_{root_index}_line_{line_index}_arrow_symbol_handle"),
                self.handle_reference(parsed, 0)?,
            );
            template.int_props.insert(
                format!("mleader_root_{root_index}_line_{line_index}_override_flags"),
                parsed.object_reader.read_bit_long()? as i64,
            );
        }

        Ok(())
    }

    fn read_xrecord(
        &mut self,
        parsed: &mut ParsedObjectStreams,
        template: &mut DwgRawObject,
    ) -> Result<()> {
        self.read_common_non_entity_data(parsed, template)?;

        let end = parsed.object_reader.read_bit_long()? as i64 + parsed.object_reader.position()? as i64;
        let mut item_index = 0usize;
        while (parsed.object_reader.position()? as i64) < end {
            let code = parsed.object_reader.read_short()? as i32;
            let value_type = GroupCodeValueType::from_raw_code(code);
            match value_type {
                GroupCodeValueType::String => {
                    template
                        .text_props
                        .insert(format!("xrecord_{code}_{item_index}"), parsed.object_reader.read_text_unicode()?);
                }
                GroupCodeValueType::Double => {
                    if code == 10 {
                        let p = Vector3::new(
                            parsed.object_reader.read_double()?,
                            parsed.object_reader.read_double()?,
                            parsed.object_reader.read_double()?,
                        );
                        template
                            .point3_props
                            .insert(format!("xrecord_{code}_{item_index}"), p);
                    } else {
                        template.float_props.insert(
                            format!("xrecord_{code}_{item_index}"),
                            parsed.object_reader.read_double()?,
                        );
                    }
                }
                GroupCodeValueType::Byte => {
                    template.int_props.insert(
                        format!("xrecord_{code}_{item_index}"),
                        parsed.object_reader.read_byte()? as i64,
                    );
                }
                GroupCodeValueType::Int16 => {
                    template.int_props.insert(
                        format!("xrecord_{code}_{item_index}"),
                        parsed.object_reader.read_short()? as i64,
                    );
                }
                GroupCodeValueType::Int32 => {
                    template.int_props.insert(
                        format!("xrecord_{code}_{item_index}"),
                        parsed.object_reader.read_raw_long()?,
                    );
                }
                GroupCodeValueType::Int64 => {
                    template.int_props.insert(
                        format!("xrecord_{code}_{item_index}"),
                        parsed.object_reader.read_raw_u_long()? as i64,
                    );
                }
                GroupCodeValueType::Handle => {
                    if code == 330 || code == 1005 {
                        template
                            .handle_list_props
                            .entry("xrecord_handle_refs".to_string())
                            .or_default()
                            .push(parsed.object_reader.read_raw_u_long()?);
                    } else {
                        let text = parsed.object_reader.read_text_unicode()?;
                        if let Ok(value) = u64::from_str_radix(text.trim(), 16) {
                            template
                                .handle_list_props
                                .entry("xrecord_handle_refs".to_string())
                                .or_default()
                                .push(value);
                        }
                    }
                }
                GroupCodeValueType::Bool => {
                    template.bool_props.insert(
                        format!("xrecord_{code}_{item_index}"),
                        parsed.object_reader.read_byte()? > 0,
                    );
                }
                GroupCodeValueType::BinaryData => {
                    let len = parsed.object_reader.read_byte()? as usize;
                    template.binary_props.insert(
                        format!("xrecord_{code}_{item_index}"),
                        parsed.object_reader.read_bytes(len)?,
                    );
                }
                GroupCodeValueType::Point3D | GroupCodeValueType::None => {
                    // Fallback for unsupported/unknown codes in XRECORD stream
                    template.int_props.insert(
                        format!("xrecord_unknown_code_{item_index}"),
                        code as i64,
                    );
                    break;
                }
            }
            item_index += 1;
        }

        template
            .int_props
            .insert("xrecord_item_count".to_string(), item_index as i64);

        if self.version >= DxfVersion::AC1015 {
            template
                .int_props
                .insert("xrecord_cloning_flags".to_string(), parsed.object_reader.read_bit_short()? as i64);
        }

        let size_bits = parsed.object_initial_pos + (parsed.size as u64 * 8) - 7;
        while parsed.handles_reader.position_in_bits()? < size_bits {
            let h = self.handle_reference(parsed, 0)?;
            if h == 0 {
                break;
            }
            template
                .handle_list_props
                .entry("xrecord_tail_handles".to_string())
                .or_default()
                .push(h);
        }

        Ok(())
    }

    fn read_hatch(
        &mut self,
        parsed: &mut ParsedObjectStreams,
        template: &mut DwgRawObject,
    ) -> Result<()> {
        self.read_common_entity_data(parsed, template)?;

        if self.r2004_plus() {
            template.bool_props.insert(
                "hatch_gradient_enabled".to_string(),
                parsed.object_reader.read_bit_long()? != 0,
            );
            template
                .int_props
                .insert("hatch_gradient_reserved".to_string(), parsed.object_reader.read_bit_long()? as i64);
            template.float_props.insert(
                "hatch_gradient_angle".to_string(),
                parsed.object_reader.read_bit_double()?,
            );
            template.float_props.insert(
                "hatch_gradient_shift".to_string(),
                parsed.object_reader.read_bit_double()?,
            );
            template.bool_props.insert(
                "hatch_gradient_single".to_string(),
                parsed.object_reader.read_bit_long()? > 0,
            );
            template.float_props.insert(
                "hatch_gradient_tint".to_string(),
                parsed.object_reader.read_bit_double()?,
            );

            let ncolors = parsed.object_reader.read_bit_long()?.max(0) as usize;
            for _ in 0..ncolors {
                let _ = parsed.object_reader.read_bit_double()?;
                let _ = parsed.object_reader.read_cm_color(false)?;
            }

            template
                .text_props
                .insert("hatch_gradient_name".to_string(), parsed.text_reader.read_variable_text()?);
        }

        template
            .float_props
            .insert("hatch_elevation".to_string(), parsed.object_reader.read_bit_double()?);
        template
            .point3_props
            .insert("hatch_normal".to_string(), parsed.object_reader.read_3_bit_double()?);
        template
            .text_props
            .insert("hatch_pattern_name".to_string(), parsed.text_reader.read_variable_text()?);
        template
            .bool_props
            .insert("hatch_is_solid".to_string(), parsed.object_reader.read_bit()?);
        template
            .bool_props
            .insert("hatch_is_associative".to_string(), parsed.object_reader.read_bit()?);

        let npaths = parsed.object_reader.read_bit_long()?.max(0) as usize;
        template
            .int_props
            .insert("hatch_path_count".to_string(), npaths as i64);
        let mut has_derived_boundary = false;

        for _ in 0..npaths {
            let path_flags = parsed.object_reader.read_bit_long()?;
            if (path_flags & 0b100) != 0 {
                has_derived_boundary = true;
            }

            let is_polyline = (path_flags & 0b10) != 0;
            if !is_polyline {
                let nsegments = parsed.object_reader.read_bit_long()?.max(0) as usize;
                for _ in 0..nsegments {
                    match parsed.object_reader.read_byte()? {
                        1 => {
                            let _ = parsed.object_reader.read_2_raw_double()?;
                            let _ = parsed.object_reader.read_2_raw_double()?;
                        }
                        2 => {
                            let _ = parsed.object_reader.read_2_raw_double()?;
                            let _ = parsed.object_reader.read_bit_double()?;
                            let _ = parsed.object_reader.read_bit_double()?;
                            let _ = parsed.object_reader.read_bit_double()?;
                            let _ = parsed.object_reader.read_bit()?;
                        }
                        3 => {
                            let _ = parsed.object_reader.read_2_raw_double()?;
                            let _ = parsed.object_reader.read_2_raw_double()?;
                            let _ = parsed.object_reader.read_bit_double()?;
                            let _ = parsed.object_reader.read_bit_double()?;
                            let _ = parsed.object_reader.read_bit_double()?;
                            let _ = parsed.object_reader.read_bit()?;
                        }
                        4 => {
                            let _ = parsed.object_reader.read_bit_long()?;
                            let is_rational = parsed.object_reader.read_bit()?;
                            let _ = parsed.object_reader.read_bit()?;
                            let num_knots = parsed.object_reader.read_bit_long()?.max(0) as usize;
                            let num_ctlpts = parsed.object_reader.read_bit_long()?.max(0) as usize;
                            for _ in 0..num_knots {
                                let _ = parsed.object_reader.read_bit_double()?;
                            }
                            for _ in 0..num_ctlpts {
                                let _ = parsed.object_reader.read_2_raw_double()?;
                                if is_rational {
                                    let _ = parsed.object_reader.read_bit_double()?;
                                }
                            }
                            if self.r2010_plus() {
                                let fit = parsed.object_reader.read_bit_long()?.max(0) as usize;
                                for _ in 0..fit {
                                    let _ = parsed.object_reader.read_2_raw_double()?;
                                }
                                if fit > 0 {
                                    let _ = parsed.object_reader.read_2_raw_double()?;
                                    let _ = parsed.object_reader.read_2_raw_double()?;
                                }
                            }
                        }
                        _ => {}
                    }
                }
            } else {
                let bulges_present = parsed.object_reader.read_bit()?;
                let _ = parsed.object_reader.read_bit()?;
                let num_path_segs = parsed.object_reader.read_bit_long()?.max(0) as usize;
                for _ in 0..num_path_segs {
                    let _ = parsed.object_reader.read_2_raw_double()?;
                    if bulges_present {
                        let _ = parsed.object_reader.read_bit_double()?;
                    }
                }
            }

            let nhandles = parsed.object_reader.read_bit_long()?.max(0) as usize;
            for _ in 0..nhandles {
                let _ = self.handle_reference(parsed, 0)?;
            }
        }

        template
            .int_props
            .insert("hatch_style".to_string(), parsed.object_reader.read_bit_short()? as i64);
        template.int_props.insert(
            "hatch_pattern_type".to_string(),
            parsed.object_reader.read_bit_short()? as i64,
        );

        if !*template.bool_props.get("hatch_is_solid").unwrap_or(&false) {
            template.float_props.insert(
                "hatch_pattern_angle".to_string(),
                parsed.object_reader.read_bit_double()?,
            );
            template.float_props.insert(
                "hatch_pattern_scale".to_string(),
                parsed.object_reader.read_bit_double()?,
            );
            template
                .bool_props
                .insert("hatch_is_double".to_string(), parsed.object_reader.read_bit()?);
            let ndef = parsed.object_reader.read_bit_short()?.max(0) as usize;
            for _ in 0..ndef {
                let _ = parsed.object_reader.read_bit_double()?;
                let _ = parsed.object_reader.read_2_bit_double()?;
                let _ = parsed.object_reader.read_2_bit_double()?;
                let ndashes = parsed.object_reader.read_bit_short()?.max(0) as usize;
                for _ in 0..ndashes {
                    let _ = parsed.object_reader.read_bit_double()?;
                }
            }
        }

        if has_derived_boundary {
            template
                .float_props
                .insert("hatch_pixel_size".to_string(), parsed.object_reader.read_bit_double()?);
        }

        let nseed = parsed.object_reader.read_bit_long()?.max(0) as usize;
        template
            .int_props
            .insert("hatch_seed_count".to_string(), nseed as i64);
        for _ in 0..nseed {
            let _ = parsed.object_reader.read_2_raw_double()?;
        }

        Ok(())
    }

    fn read_proxy_object(
        &mut self,
        parsed: &mut ParsedObjectStreams,
        template: &mut DwgRawObject,
    ) -> Result<()> {
        self.read_common_non_entity_data(parsed, template)?;
        self.read_common_proxy_data(parsed, template)
    }

    fn read_proxy_entity(
        &mut self,
        parsed: &mut ParsedObjectStreams,
        template: &mut DwgRawObject,
    ) -> Result<()> {
        self.read_common_entity_data(parsed, template)?;
        self.read_common_proxy_data(parsed, template)
    }

    fn read_common_proxy_data(
        &mut self,
        parsed: &mut ParsedObjectStreams,
        template: &mut DwgRawObject,
    ) -> Result<()> {
        let class_id = parsed.object_reader.read_bit_long()?;
        template
            .int_props
            .insert("proxy_class_id".to_string(), class_id as i64);

        if self.version >= DxfVersion::AC1015 {
            if self.version > DxfVersion::AC1015 {
                template
                    .text_props
                    .insert("proxy_subclass".to_string(), parsed.object_reader.read_variable_text()?);
            }

            if self.version < DxfVersion::AC1032 {
                let format = parsed.object_reader.read_bit_long()?;
                template
                    .int_props
                    .insert("proxy_drawing_format".to_string(), format as i64);
                template
                    .int_props
                    .insert("proxy_version".to_string(), (format & 0xFFFF) as i64);
                template
                    .int_props
                    .insert("proxy_maintenance".to_string(), ((format >> 16) & 0xFFFF) as i64);
            } else {
                template
                    .int_props
                    .insert("proxy_version".to_string(), parsed.object_reader.read_bit_long()? as i64);
                template.int_props.insert(
                    "proxy_maintenance".to_string(),
                    parsed.object_reader.read_bit_long()? as i64,
                );
            }

            template
                .bool_props
                .insert("proxy_original_data_is_dxf".to_string(), parsed.object_reader.read_bit()?);
        }

        // Databits block: store remaining proxy payload bytes until handles section.
        let start_bits = parsed.object_reader.position_in_bits()?;
        let mut end_bits = parsed.handles_reader.position_in_bits()?;
        if end_bits <= start_bits {
            end_bits = parsed.object_initial_pos + (parsed.size as u64 * 8);
        }

        if end_bits > start_bits {
            let payload_bytes = ((end_bits - start_bits) as usize + 7) / 8;
            let payload = parsed.object_reader.read_bytes(payload_bytes)?;
            template
                .binary_props
                .insert("proxy_data_bits".to_string(), payload.clone());
            template.data = payload;
        }

        Ok(())
    }

    fn read_common_data(
        &mut self,
        parsed: &mut ParsedObjectStreams,
        template: &mut DwgRawObject,
    ) -> Result<()> {
        if self.version >= DxfVersion::AC1015 && self.version < DxfVersion::AC1024 {
            self.update_handle_reader(parsed)?;
        }

        template.handle = parsed.object_reader.handle_reference()?;
        self.read_extended_data(parsed, template)?;
        Ok(())
    }

    fn read_common_entity_data(
        &mut self,
        parsed: &mut ParsedObjectStreams,
        template: &mut DwgRawObject,
    ) -> Result<()> {
        self.read_common_data(parsed, template)?;

        // Graphic present flag.
        if parsed.object_reader.read_bit()? {
            let graphic_size = if self.version >= DxfVersion::AC1024 {
                parsed.object_reader.read_bit_long_long()?
            } else {
                parsed.object_reader.read_raw_long()?
            };
            if graphic_size > 0 {
                parsed.object_reader.advance(graphic_size as usize)?;
            }
        }

        if self.r13_14_only() {
            self.update_handle_reader(parsed)?;
        }

        self.read_entity_mode(parsed, template)
    }

    fn read_entity_mode(
        &mut self,
        parsed: &mut ParsedObjectStreams,
        template: &mut DwgRawObject,
    ) -> Result<()> {
        let ent_mode = parsed.object_reader.read_2_bits()?;

        if ent_mode == 0 {
            template.owner_handle = Some(parsed.handles_reader.handle_reference_from(template.handle)?);
        }

        self.read_reactors_and_dictionary_handle(parsed, template)?;

        if self.r13_14_only() {
            let _layer = self.handle_reference(parsed, 0)?;
            if !parsed.object_reader.read_bit()? {
                let _line_type = self.handle_reference(parsed, 0)?;
            }
        }

        let (color, transparency, color_flag) = parsed.object_reader.read_en_color()?;
        template.color = Some(color);
        template.transparency = Some(transparency);

        if self.version >= DxfVersion::AC1018 && color_flag {
            let _ = self.handle_reference(parsed, 0)?;
        }

        template.line_type_scale = Some(parsed.object_reader.read_bit_double()?);

        if self.version >= DxfVersion::AC1015 {
            let _layer = self.handle_reference(parsed, 0)?;
            let ltype_flags = parsed.object_reader.read_2_bits()?;
            if ltype_flags == 3 {
                let _ = self.handle_reference(parsed, 0)?;
            }

            if self.r2007_plus() {
                let material_flags = parsed.object_reader.read_2_bits()?;
                if material_flags == 3 {
                    let _ = self.handle_reference(parsed, 0)?;
                }
                let _shadow_flags = parsed.object_reader.read_byte()?;
            }

            let plotstyle_flags = parsed.object_reader.read_2_bits()?;
            if plotstyle_flags == 3 {
                let _ = self.handle_reference(parsed, 0)?;
            }

            if self.r2010_plus() {
                if parsed.object_reader.read_bit()? {
                    let _ = self.handle_reference(parsed, 0)?;
                }
                if parsed.object_reader.read_bit()? {
                    let _ = self.handle_reference(parsed, 0)?;
                }
                if parsed.object_reader.read_bit()? {
                    let _ = self.handle_reference(parsed, 0)?;
                }
            }

            let invis = parsed.object_reader.read_bit_short()?;
            template.line_weight = Some(parsed.object_reader.read_byte()? as i16);
            if (invis & 1) != 0 {
                // semantic parity: invisibility parsed and consumed
            }
        }

        Ok(())
    }

    fn read_common_non_entity_data(
        &mut self,
        parsed: &mut ParsedObjectStreams,
        template: &mut DwgRawObject,
    ) -> Result<()> {
        self.read_common_data(parsed, template)?;

        if self.r13_14_only() {
            self.update_handle_reader(parsed)?;
        }

        template.owner_handle = Some(self.handle_reference(parsed, template.handle)?);
        self.read_reactors_and_dictionary_handle(parsed, template)
    }

    fn read_extended_data(
        &mut self,
        parsed: &mut ParsedObjectStreams,
        template: &mut DwgRawObject,
    ) -> Result<()> {
        let mut size = parsed.object_reader.read_bit_short()?;
        while size != 0 {
            let app_handle = parsed.object_reader.handle_reference()?;
            let end_pos = parsed.object_reader.position()? + size as u64;

            let records = self.read_extended_data_records(parsed, end_pos)?;
            template.eed.insert(app_handle, records);

            size = parsed.object_reader.read_bit_short()?;
        }
        Ok(())
    }

    fn read_extended_data_records(
        &mut self,
        parsed: &mut ParsedObjectStreams,
        end_pos: u64,
    ) -> Result<Vec<DwgExtendedDataRecord>> {
        let mut records = Vec::new();

        while parsed.object_reader.position()? < end_pos {
            let dxf_code = 1000 + parsed.object_reader.read_byte()? as i32;
            let mut record = DwgExtendedDataRecord {
                code: dxf_code,
                ..Default::default()
            };

            match dxf_code {
                1000 | 1001 => {
                    record.text = Some(parsed.object_reader.read_text_unicode()?);
                }
                1002 => {
                    record.integer = Some(parsed.object_reader.read_byte()? as i64);
                }
                1003 | 1005 => {
                    record.bytes = parsed.object_reader.read_bytes(8)?;
                }
                1004 => {
                    let len = parsed.object_reader.read_byte()? as usize;
                    record.bytes = parsed.object_reader.read_bytes(len)?;
                }
                1010..=1013 => {
                    record.point = Some(Vector3::new(
                        parsed.object_reader.read_double()?,
                        parsed.object_reader.read_double()?,
                        parsed.object_reader.read_double()?,
                    ));
                }
                1040..=1042 => {
                    record.number = Some(parsed.object_reader.read_double()?);
                }
                1070 => {
                    record.integer = Some(parsed.object_reader.read_short()? as i64);
                }
                1071 => {
                    record.integer = Some(parsed.object_reader.read_raw_long()?);
                }
                _ => {
                    let remaining = (end_pos.saturating_sub(parsed.object_reader.position()?)) as usize;
                    let _ = parsed.object_reader.read_bytes(remaining)?;
                    records.push(record);
                    break;
                }
            }

            records.push(record);
        }

        Ok(records)
    }

    fn read_reactors_and_dictionary_handle(
        &mut self,
        parsed: &mut ParsedObjectStreams,
        template: &mut DwgRawObject,
    ) -> Result<()> {
        let reactor_count = parsed.object_reader.read_bit_long()?.max(0) as usize;
        for _ in 0..reactor_count {
            template.reactors.push(self.handle_reference(parsed, 0)?);
        }

        let mut xdict_missing = false;
        if self.r2004_plus() {
            xdict_missing = parsed.object_reader.read_bit()?;
        }

        if !xdict_missing {
            template.xdict_handle = Some(self.handle_reference(parsed, 0)?);
        }

        if self.r2013_plus() {
            let _has_ds_binary_data = parsed.object_reader.read_bit()?;
        }

        Ok(())
    }

    fn update_handle_reader(&self, parsed: &mut ParsedObjectStreams) -> Result<()> {
        let size = parsed.object_reader.read_raw_long()?;
        parsed
            .handles_reader
            .set_position_in_bits(size as u64 + parsed.object_initial_pos)?;

        if self.version == DxfVersion::AC1021 {
            let _ = parsed
                .text_reader
                .set_position_by_flag(size as u64 + parsed.object_initial_pos - 1);
        }

        Ok(())
    }

    fn handle_reference(&mut self, parsed: &mut ParsedObjectStreams, base: u64) -> Result<u64> {
        let value = parsed.handles_reader.handle_reference_from(base)?;
        if value != 0 && !self.read_objects.contains(&value) {
            self.handles.push_back(value);
        }
        Ok(value)
    }

    #[inline]
    fn r13_14_only(&self) -> bool {
        matches!(self.version, DxfVersion::AC1012 | DxfVersion::AC1014)
    }
    #[inline]
    fn r2004_plus(&self) -> bool {
        self.version >= DxfVersion::AC1018
    }
    #[inline]
    fn r2007_plus(&self) -> bool {
        self.version >= DxfVersion::AC1021
    }
    #[inline]
    fn r2010_plus(&self) -> bool {
        self.version >= DxfVersion::AC1024
    }
    #[inline]
    fn r2013_plus(&self) -> bool {
        self.version >= DxfVersion::AC1027
    }
}

struct ParsedObjectStreams {
    object_initial_pos: u64,
    size: u32,
    object_type: DwgObjectType,
    object_reader: DwgStreamReaderBase,
    handles_reader: DwgStreamReaderBase,
    text_reader: DwgStreamReaderBase,
}

impl ParsedObjectStreams {
    fn empty() -> Self {
        let stream = || DwgStreamReaderBase::new(Box::new(Cursor::new(Vec::<u8>::new())));
        Self {
            object_initial_pos: 0,
            size: 0,
            object_type: DwgObjectType(0),
            object_reader: stream(),
            handles_reader: stream(),
            text_reader: stream(),
        }
    }
}
