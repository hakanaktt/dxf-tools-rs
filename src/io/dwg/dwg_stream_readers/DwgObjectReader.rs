use std::collections::{BTreeMap, HashSet, VecDeque};
use std::io::Cursor;

use crate::{
    error::Result,
    types::{Color, DxfVersion, Transparency, Vector3},
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
    MText,
    Leader,
    Tolerance,
    MLine,
    Unknown(u16),
}

impl RawObjectType {
    pub fn from_code(code: u16) -> Self {
        // Numeric values differ by DWG release; keep this map intentionally partial and
        // preserve unknown values to avoid data loss.
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
            0x0C => Self::VertexPFace,
            0x0D => Self::VertexMesh,
            0x0E => Self::Polyline2D,
            0x0F => Self::Polyline3D,
            0x11 => Self::Arc,
            0x12 => Self::Circle,
            0x13 => Self::Line,
            0x1A => Self::Point,
            0x1B => Self::Face3D,
            0x1C => Self::PolylinePFace,
            0x1D => Self::PolylineMesh,
            0x1E => Self::Solid,
            0x1F => Self::Trace,
            0x20 => Self::Shape,
            0x21 => Self::Viewport,
            0x22 => Self::Ellipse,
            0x23 => Self::Spline,
            0x24 => Self::Region,
            0x25 => Self::Solid3D,
            0x26 => Self::Body,
            0x27 => Self::Ray,
            0x28 => Self::XLine,
            0x29 => Self::Dictionary,
            0x2A => Self::MText,
            0x2B => Self::Leader,
            0x2C => Self::Tolerance,
            0x2D => Self::MLine,
            other => Self::Unknown(other),
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
}

pub struct DwgObjectReader {
    version: DxfVersion,
    buffer: Vec<u8>,
    handles: VecDeque<u64>,
    map: BTreeMap<u64, i64>,
    read_objects: HashSet<u64>,
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
        }
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
        let raw_type = RawObjectType::from_code(parsed.object_type.0);

        let mut template = DwgRawObject {
            handle: current_handle,
            object_type: Some(parsed.object_type),
            raw_type: Some(raw_type),
            ..Default::default()
        };

        self.read_common_data(&mut parsed, &mut template)?;

        match raw_type {
            RawObjectType::Text
            | RawObjectType::Attrib
            | RawObjectType::AttDef
            | RawObjectType::Block
            | RawObjectType::EndBlk
            | RawObjectType::SeqEnd
            | RawObjectType::Insert
            | RawObjectType::MInsert
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
            | RawObjectType::MText
            | RawObjectType::Leader
            | RawObjectType::Tolerance
            | RawObjectType::MLine => {
                self.read_common_entity_data(&mut parsed, &mut template)?;
            }
            RawObjectType::Dictionary | RawObjectType::Unknown(_) => {
                self.read_common_non_entity_data(&mut parsed, &mut template)?;
            }
        }

        // Best-effort raw payload extraction after semantic fields.
        let end_bits = parsed.object_initial_pos + (parsed.size as u64 * 8);
        let current_bits = parsed.object_reader.position_in_bits()?;
        if end_bits > current_bits {
            let remaining = ((end_bits - current_bits) / 8) as usize;
            template.data = parsed.object_reader.read_bytes(remaining)?;
        }

        Ok(Some(template))
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
