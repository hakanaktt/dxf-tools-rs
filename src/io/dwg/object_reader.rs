//! DWG Object Reader - Reads entities and objects from DWG files
//!
//! This module contains the main object reader that parses entities (Line, Arc, etc.),
//! table entries (Layer, LineType, etc.), and non-graphical objects (Dictionary, etc.)
//! from the AcDbObjects section of a DWG file.

use std::collections::{HashMap, VecDeque};
use std::io::{Cursor, Read, Seek, SeekFrom};
use crate::error::{DxfError, Result};
use crate::types::{ACadVersion, Color, Handle, Vector2, Vector3};
use crate::entities::*;
use crate::tables::*;
use super::stream_reader::{BitReader, DwgStreamReader, DwgReferenceType};
use super::classes_reader::{DxfClass, DxfClassCollection, ObjectType};
use super::crc::Crc8;

/// Common entity data read from DWG
#[derive(Debug, Clone, Default)]
pub struct DwgEntityData {
    /// Entity handle
    pub handle: u64,
    /// Extended data (xdata)
    pub xdata: Vec<u8>,
    /// Graphic data flag
    pub has_graphics: bool,
    /// Graphics data if present
    pub graphics_data: Vec<u8>,
    /// Entity mode (0=normal, 1=BYBLOCK, 2=BYLAYER, 3=custom)
    pub entity_mode: u8,
    /// Number of reactors
    pub num_reactors: u32,
    /// XDic missing flag (R2004+)
    pub xdic_missing: bool,
    /// Binary data missing flag (R2013+)
    pub binary_data_missing: bool,
    /// Has DS binary data (R2013+)  
    pub has_ds_data: bool,
    /// Linetype scale
    pub linetype_scale: f64,
    /// Linetype flags (R2000+)
    pub linetype_flags: u8,
    /// Plotstyle flags (R2000+)
    pub plotstyle_flags: u8,
    /// Material flags (R2007+)
    pub material_flags: u8,
    /// Shadow flags (R2007+)
    pub shadow_flags: u8,
    /// Has full visualstyle (R2010+)
    pub has_full_visualstyle: bool,
    /// Has face visualstyle (R2010+)
    pub has_face_visualstyle: bool,
    /// Has edge visualstyle (R2010+)
    pub has_edge_visualstyle: bool,
    /// Invisibility flag
    pub invisible: bool,
    /// Lineweight
    pub lineweight: i16,
    /// Color
    pub color: Color,
    
    // Handle references
    /// Owner handle (usually block record)
    pub owner_handle: Option<u64>,
    /// Reactor handles
    pub reactor_handles: Vec<u64>,
    /// XDictionary handle
    pub xdic_handle: Option<u64>,
    /// Layer handle
    pub layer_handle: Option<u64>,
    /// Linetype handle
    pub linetype_handle: Option<u64>,
    /// Previous entity handle (for linked list)
    pub prev_entity: Option<u64>,
    /// Next entity handle (for linked list)
    pub next_entity: Option<u64>,
    /// Plotstyle handle (R2000+)
    pub plotstyle_handle: Option<u64>,
    /// Material handle (R2007+)
    pub material_handle: Option<u64>,
    /// Visualstyle handle (R2010+)
    pub visualstyle_handle: Option<u64>,
}

/// Common non-entity object data
#[derive(Debug, Clone, Default)]
pub struct DwgObjectData {
    /// Object handle
    pub handle: u64,
    /// Extended data
    pub xdata: Vec<u8>,
    /// Number of reactors
    pub num_reactors: u32,
    /// XDic missing flag (R2004+)
    pub xdic_missing: bool,
    /// Has DS binary data (R2013+)
    pub has_ds_data: bool,
    
    // Handle references
    /// Owner handle
    pub owner_handle: Option<u64>,
    /// Reactor handles
    pub reactor_handles: Vec<u64>,
    /// XDictionary handle
    pub xdic_handle: Option<u64>,
}

/// Template for building entities/objects after reading
#[derive(Debug)]
pub enum CadTemplate {
    /// Line entity
    Line {
        entity_data: DwgEntityData,
        start: Vector3,
        end: Vector3,
        thickness: f64,
        extrusion: Vector3,
    },
    /// Circle entity
    Circle {
        entity_data: DwgEntityData,
        center: Vector3,
        radius: f64,
        thickness: f64,
        extrusion: Vector3,
    },
    /// Arc entity
    Arc {
        entity_data: DwgEntityData,
        center: Vector3,
        radius: f64,
        thickness: f64,
        extrusion: Vector3,
        start_angle: f64,
        end_angle: f64,
    },
    /// Point entity
    Point {
        entity_data: DwgEntityData,
        location: Vector3,
        thickness: f64,
        extrusion: Vector3,
        x_axis_angle: f64,
    },
    /// LwPolyline entity
    LwPolyline {
        entity_data: DwgEntityData,
        flag: i16,
        const_width: f64,
        elevation: f64,
        thickness: f64,
        extrusion: Vector3,
        vertices: Vec<LwPolylineVertex>,
    },
    /// Text entity
    Text {
        entity_data: DwgEntityData,
        insertion: Vector3,
        alignment: Vector3,
        extrusion: Vector3,
        thickness: f64,
        oblique_angle: f64,
        rotation: f64,
        height: f64,
        width_factor: f64,
        value: String,
        generation_flags: i16,
        horizontal_alignment: i16,
        vertical_alignment: i16,
        style_handle: Option<u64>,
    },
    /// Layer table entry
    Layer {
        object_data: DwgObjectData,
        name: String,
        flags: i16,
        color: Color,
        linetype_handle: Option<u64>,
        plotstyle_handle: Option<u64>,
        material_handle: Option<u64>,
        is_on: bool,
        is_frozen: bool,
        is_locked: bool,
        is_plotting: bool,
        lineweight: i16,
    },
    /// LineType table entry
    LineType {
        object_data: DwgObjectData,
        name: String,
        description: String,
        flags: i16,
        pattern_length: f64,
        alignment: u8,
        elements: Vec<LineTypeElement>,
    },
    /// Block header/record
    BlockRecord {
        object_data: DwgObjectData,
        name: String,
        flags: i16,
        is_xref: bool,
        is_overlaid: bool,
        is_anonymous: bool,
        has_attributes: bool,
        is_xref_resolved: bool,
        xref_path: String,
        block_entity: Option<u64>,
        endblk_entity: Option<u64>,
        first_entity: Option<u64>,
        last_entity: Option<u64>,
        layout_handle: Option<u64>,
        insert_units: i16,
        is_explodable: bool,
        can_scale: bool,
    },
    /// Dictionary object
    Dictionary {
        object_data: DwgObjectData,
        num_entries: u32,
        cloning_flag: u8,
        hard_owner_flag: bool,
        entries: Vec<(String, u64)>,
    },
    /// MText entity
    MText {
        entity_data: DwgEntityData,
        insertion: Vector3,
        extrusion: Vector3,
        x_direction: Vector3,
        rect_width: f64,
        rect_height: f64,
        text_height: f64,
        attachment: u8,
        drawing_direction: u8,
        line_spacing_style: u8,
        line_spacing_factor: f64,
        contents: String,
        style_handle: Option<u64>,
    },
    /// Ellipse entity
    Ellipse {
        entity_data: DwgEntityData,
        center: Vector3,
        major_axis: Vector3,
        extrusion: Vector3,
        axis_ratio: f64,
        start_angle: f64,
        end_angle: f64,
    },
    /// Spline entity
    Spline {
        entity_data: DwgEntityData,
        scenario: i16,
        degree: i16,
        flags: i16,
        closed: bool,
        periodic: bool,
        rational: bool,
        knot_tolerance: f64,
        control_tolerance: f64,
        fit_tolerance: f64,
        start_tangent: Option<Vector3>,
        end_tangent: Option<Vector3>,
        knots: Vec<f64>,
        weights: Vec<f64>,
        control_points: Vec<Vector3>,
        fit_points: Vec<Vector3>,
    },
    /// Insert entity
    Insert {
        entity_data: DwgEntityData,
        insertion_point: Vector3,
        scale: Vector3,
        rotation: f64,
        extrusion: Vector3,
        has_attribs: bool,
        owned_obj_count: u32,
        block_header_handle: Option<u64>,
        first_attrib_handle: Option<u64>,
        last_attrib_handle: Option<u64>,
        seqend_handle: Option<u64>,
        attrib_handles: Vec<u64>,
    },
    /// Polyline 2D entity
    Polyline2D {
        entity_data: DwgEntityData,
        flags: i16,
        curve_type: i16,
        start_width: f64,
        end_width: f64,
        thickness: f64,
        elevation: f64,
        extrusion: Vector3,
        owned_obj_count: u32,
        first_vertex_handle: Option<u64>,
        last_vertex_handle: Option<u64>,
        seqend_handle: Option<u64>,
        vertex_handles: Vec<u64>,
    },
    /// Polyline 3D entity
    Polyline3D {
        entity_data: DwgEntityData,
        flags: u8,
        curve_type: u8,
        owned_obj_count: u32,
        first_vertex_handle: Option<u64>,
        last_vertex_handle: Option<u64>,
        seqend_handle: Option<u64>,
        vertex_handles: Vec<u64>,
    },
    /// Vertex 2D
    Vertex2D {
        entity_data: DwgEntityData,
        flags: u8,
        point: Vector3,
        start_width: f64,
        end_width: f64,
        bulge: f64,
        tangent_dir: f64,
    },
    /// Vertex 3D
    Vertex3D {
        entity_data: DwgEntityData,
        flags: u8,
        point: Vector3,
    },
    /// Block entity (inside block definition)
    Block {
        entity_data: DwgEntityData,
        name: String,
    },
    /// Block end entity
    BlockEnd {
        entity_data: DwgEntityData,
    },
    /// Seqend entity
    Seqend {
        entity_data: DwgEntityData,
    },
    /// Aligned Dimension
    DimAligned {
        entity_data: DwgEntityData,
        dim_common: DimCommonData,
        def_point: Vector3,
        xline1_pt: Vector3,
        xline2_pt: Vector3,
    },
    /// Linear Dimension
    DimLinear {
        entity_data: DwgEntityData,
        dim_common: DimCommonData,
        def_point: Vector3,
        xline1_pt: Vector3,
        xline2_pt: Vector3,
        rotation: f64,
        oblique_angle: f64,
    },
    /// Radius Dimension
    DimRadius {
        entity_data: DwgEntityData,
        dim_common: DimCommonData,
        def_point: Vector3,
        leader_len: f64,
    },
    /// Diameter Dimension
    DimDiameter {
        entity_data: DwgEntityData,
        dim_common: DimCommonData,
        def_point: Vector3,
        leader_len: f64,
    },
    /// Angular 3-Point Dimension
    DimAngular3Pt {
        entity_data: DwgEntityData,
        dim_common: DimCommonData,
        def_point: Vector3,
        xline1_pt: Vector3,
        xline2_pt: Vector3,
        center_pt: Vector3,
    },
    /// Ordinate Dimension
    DimOrdinate {
        entity_data: DwgEntityData,
        dim_common: DimCommonData,
        def_point: Vector3,
        feature_pt: Vector3,
        leader_pt: Vector3,
        ordinate_type: u8,
    },
    /// Hatch entity
    Hatch {
        entity_data: DwgEntityData,
        elevation: f64,
        extrusion: Vector3,
        pattern_name: String,
        is_solid_fill: bool,
        is_associative: bool,
        pattern_type: i16,
        pattern_angle: f64,
        pattern_scale: f64,
        pattern_double: bool,
        num_seed_points: i16,
        seed_points: Vec<Vector2>,
        boundary_paths: Vec<HatchBoundaryPath>,
        pattern_def_lines: Vec<HatchPatternDefLine>,
    },
    /// Solid (2D solid fill)
    Solid {
        entity_data: DwgEntityData,
        thickness: f64,
        elevation: f64,
        extrusion: Vector3,
        corner1: Vector2,
        corner2: Vector2,
        corner3: Vector2,
        corner4: Vector2,
    },
    /// Trace (thick line)
    Trace {
        entity_data: DwgEntityData,
        thickness: f64,
        elevation: f64,
        extrusion: Vector3,
        corner1: Vector2,
        corner2: Vector2,
        corner3: Vector2,
        corner4: Vector2,
    },
    /// 3DFace entity
    Face3D {
        entity_data: DwgEntityData,
        has_no_flags: bool,
        z_is_zero: bool,
        corners: [Vector3; 4],
        invisible_edge: u16,
    },
    /// Viewport entity
    Viewport {
        entity_data: DwgEntityData,
        center: Vector3,
        width: f64,
        height: f64,
        view_target: Vector3,
        view_direction: Vector3,
        view_twist_angle: f64,
        view_height: f64,
        lens_length: f64,
        front_clip: f64,
        back_clip: f64,
        snap_angle: f64,
        view_center: Vector2,
        snap_base: Vector2,
        snap_spacing: Vector2,
        grid_spacing: Vector2,
        circle_sides: u16,
        frozen_layer_handles: Vec<u64>,
    },
    /// Ray entity
    Ray {
        entity_data: DwgEntityData,
        point: Vector3,
        vector: Vector3,
    },
    /// XLine (construction line)
    XLine {
        entity_data: DwgEntityData,
        point: Vector3,
        vector: Vector3,
    },
    /// Attribute Definition
    AttDef {
        entity_data: DwgEntityData,
        text_data: TextTemplateData,
        version: u8,
        prompt: String,
        tag: String,
        flags: u8,
        field_length: u8,
        lock_position: bool,
    },
    /// Attribute Entity
    Attrib {
        entity_data: DwgEntityData,
        text_data: TextTemplateData,
        version: u8,
        tag: String,
        flags: u8,
        field_length: u8,
        lock_position: bool,
    },
    /// TextStyle table entry
    TextStyle {
        object_data: DwgObjectData,
        name: String,
        flags: i16,
        fixed_height: f64,
        width_factor: f64,
        oblique_angle: f64,
        generation_flags: u8,
        last_height: f64,
        font_name: String,
        big_font_name: String,
    },
    /// DimStyle table entry
    DimStyle {
        object_data: DwgObjectData,
        name: String,
        flags: i16,
        dimpost: String,
        dimapost: String,
        dimscale: f64,
        dimasz: f64,
        dimexo: f64,
        dimdli: f64,
        dimexe: f64,
        dimrnd: f64,
        dimdle: f64,
        dimtp: f64,
        dimtm: f64,
        dimtxt: f64,
        dimcen: f64,
        dimtsz: f64,
        dimaltf: f64,
        dimlfac: f64,
        dimtvp: f64,
        dimtfac: f64,
        dimgap: f64,
        dimtol: bool,
        dimlim: bool,
        dimtih: bool,
        dimtoh: bool,
        dimse1: bool,
        dimse2: bool,
        dimtad: u8,
        dimzin: u8,
        dimalt: bool,
        dimaltd: u8,
        dimtofl: bool,
        dimsah: bool,
        dimtix: bool,
        dimsoxd: bool,
        dimclrd: i16,
        dimclre: i16,
        dimclrt: i16,
        dimadec: u8,
        dimdec: u8,
        dimtdec: u8,
        dimaltu: u8,
        dimalttd: u8,
        dimaunit: u8,
        dimfrac: u8,
        dimlunit: u8,
        dimdsep: u8,
        dimtmove: u8,
        dimjust: u8,
        dimsd1: bool,
        dimsd2: bool,
        dimtolj: u8,
        dimtzin: u8,
        dimaltz: u8,
        dimalttz: u8,
        dimfit: u8,
        dimupt: bool,
        dimatfit: u8,
        dimtxsty_handle: Option<u64>,
        dimldrblk_handle: Option<u64>,
        dimblk_handle: Option<u64>,
        dimblk1_handle: Option<u64>,
        dimblk2_handle: Option<u64>,
        dimltype_handle: Option<u64>,
        dimltex1_handle: Option<u64>,
        dimltex2_handle: Option<u64>,
    },
    /// Unknown/unsupported type
    Unknown {
        object_type: u16,
        object_data: DwgObjectData,
    },
}

/// Common text template data shared by Text, AttDef, Attrib
#[derive(Debug, Clone, Default)]
pub struct TextTemplateData {
    pub insertion: Vector3,
    pub alignment: Vector3,
    pub extrusion: Vector3,
    pub thickness: f64,
    pub oblique_angle: f64,
    pub rotation: f64,
    pub height: f64,
    pub width_factor: f64,
    pub value: String,
    pub generation_flags: i16,
    pub horizontal_alignment: i16,
    pub vertical_alignment: i16,
    pub style_handle: Option<u64>,
}

/// Common dimension data shared by all dimension types
#[derive(Debug, Clone, Default)]
pub struct DimCommonData {
    pub version: u8,
    pub extrusion: Vector3,
    pub text_midpoint: Vector2,
    pub elevation: f64,
    pub flags: u8,
    pub user_text: String,
    pub text_rotation: f64,
    pub horiz_dir: f64,
    pub ins_scale: Vector3,
    pub ins_rotation: f64,
    pub attachment_point: u8,
    pub linespacing_style: u8,
    pub linespacing_factor: f64,
    pub actual_measurement: f64,
    pub clone_ins_pt: Vector2,
    pub dimstyle_handle: Option<u64>,
    pub block_handle: Option<u64>,
}

/// Hatch boundary path
#[derive(Debug, Clone, Default)]
pub struct HatchBoundaryPath {
    pub flags: u32,
    pub path_type: u8,
    pub edges: Vec<HatchEdge>,
    pub polyline_has_bulge: bool,
    pub polyline_closed: bool,
    pub polyline_vertices: Vec<(Vector2, f64)>,
    pub boundary_handles: Vec<u64>,
}

/// Hatch edge type
#[derive(Debug, Clone)]
pub enum HatchEdge {
    Line { start: Vector2, end: Vector2 },
    CircularArc { center: Vector2, radius: f64, start_angle: f64, end_angle: f64, is_ccw: bool },
    EllipticArc { center: Vector2, major_axis: Vector2, minor_ratio: f64, start_angle: f64, end_angle: f64, is_ccw: bool },
    Spline { degree: i32, rational: bool, periodic: bool, knots: Vec<f64>, control_points: Vec<Vector2>, weights: Vec<f64>, fit_data: Option<(Vector2, Vector2, Vec<Vector2>)> },
}

/// Hatch pattern definition line
#[derive(Debug, Clone, Default)]
pub struct HatchPatternDefLine {
    pub angle: f64,
    pub base_point: Vector2,
    pub offset: Vector2,
    pub dash_lengths: Vec<f64>,
}

/// Vertex data for LwPolyline
#[derive(Debug, Clone, Default)]
pub struct LwPolylineVertex {
    pub point: Vector2,
    pub start_width: f64,
    pub end_width: f64,
    pub bulge: f64,
    pub vertex_id: i32,
}

/// Line type element
#[derive(Debug, Clone, Default)]
pub struct LineTypeElement {
    pub dash_length: f64,
    pub complex_flags: i16,
    pub shape_flag: i16,
    pub shape_number: i16,
    pub offset: Vector2,
    pub scale: f64,
    pub rotation: f64,
    pub style_handle: Option<u64>,
    pub text: String,
}

/// Reader for DWG objects section
pub struct DwgObjectReader<R: Read + Seek> {
    /// The bit reader over the section data
    reader: BitReader<R>,
    /// DWG version
    version: ACadVersion,
    /// Handle-to-offset map
    handle_map: HashMap<u64, i64>,
    /// DXF class definitions
    classes: HashMap<i16, DxfClass>,
    /// Queue of handles to read
    handle_queue: VecDeque<u64>,
    /// Already-read handles
    read_handles: HashMap<u64, ObjectType>,
    /// Resulting templates
    templates: HashMap<u64, CadTemplate>,
    /// Current section start for offset calculations
    section_base: i64,
}

impl<R: Read + Seek> DwgObjectReader<R> {
    /// Create a new object reader
    pub fn new(
        reader: BitReader<R>,
        version: ACadVersion,
        handle_map: HashMap<u64, i64>,
        classes: &DxfClassCollection,
        initial_handles: Vec<u64>,
    ) -> Self {
        Self {
            reader,
            version,
            handle_map,
            classes: classes.to_map().into_iter().map(|(k, v)| (k, v.clone())).collect(),
            handle_queue: initial_handles.into(),
            read_handles: HashMap::new(),
            templates: HashMap::new(),
            section_base: 0,
        }
    }
    
    /// Set the section base address for R13-R15 files
    pub fn set_section_base(&mut self, base: i64) {
        self.section_base = base;
    }
    
    /// Read all queued objects and return templates
    pub fn read(&mut self) -> Result<HashMap<u64, CadTemplate>> {
        while let Some(handle) = self.handle_queue.pop_front() {
            // Skip if already read
            if self.read_handles.contains_key(&handle) {
                continue;
            }
            
            // Get offset for this handle
            let offset = match self.handle_map.get(&handle) {
                Some(&off) => off,
                None => continue, // Handle not in map
            };
            
            // Try to read the object
            match self.read_object_at(handle, offset) {
                Ok(Some(template)) => {
                    self.templates.insert(handle, template);
                }
                Ok(None) => {
                    // Object type not supported, skip
                }
                Err(e) => {
                    // Already logged in read_object_at
                }
            }
        }
        
        Ok(std::mem::take(&mut self.templates))
    }
    
    /// Read a single object at the given offset
    fn read_object_at(&mut self, handle: u64, offset: i64) -> Result<Option<CadTemplate>> {
        // Seek to object position
        let abs_pos = (self.section_base + offset) as u64;
        self.reader.set_position(abs_pos)?;
        
        // Debug first few objects
        if self.templates.len() < 5 || offset > 1200000 {
            // Note: reading object at position
        }
        
        // Read object size (modular short)
        let size = self.reader.read_modular_short()? as usize;
        if size == 0 {
            return Ok(None);
        }
        
        // R2010+ ONLY: Read size in bits and handle size BEFORE object type
        // For R2004-R2007, these are read in readCommonData instead
        let (_obj_size_bits, _handle_size) = if self.version >= ACadVersion::AC1024 {
            let size_bits = self.reader.read_raw_long()? as u64;
            let handle_size = self.reader.read_modular_char()? as u64;
            (Some(size_bits), Some(handle_size))
        } else {
            (None, None)
        };
        
        // Read object type
        let type_code = self.read_object_type()?;
        let object_type = ObjectType::try_from(type_code).unwrap_or(ObjectType::Invalid);
        

        
        // Track that we've read this handle
        self.read_handles.insert(handle, object_type);
        
        // Read based on type
        let result = match object_type {
            ObjectType::Line => self.read_line().map(Some),
            ObjectType::Circle => self.read_circle().map(Some),
            ObjectType::Arc => self.read_arc().map(Some),
            ObjectType::Point => self.read_point().map(Some),
            ObjectType::LwPolyline => self.read_lwpolyline().map(Some),
            ObjectType::Text => self.read_text().map(Some),
            ObjectType::MText => self.read_mtext().map(Some),
            ObjectType::Ellipse => self.read_ellipse().map(Some),
            ObjectType::Spline => self.read_spline().map(Some),
            ObjectType::Insert => self.read_insert().map(Some),
            ObjectType::Polyline2D => self.read_polyline2d().map(Some),
            ObjectType::Polyline3D => self.read_polyline3d().map(Some),
            ObjectType::Vertex2D => self.read_vertex2d().map(Some),
            ObjectType::Vertex3D => self.read_vertex3d().map(Some),
            ObjectType::Block => self.read_block().map(Some),
            ObjectType::Endblk => self.read_block_end().map(Some),
            ObjectType::Seqend => self.read_seqend().map(Some),
            ObjectType::Solid => self.read_solid().map(Some),
            ObjectType::Trace => self.read_trace().map(Some),
            ObjectType::Face3D => self.read_face3d().map(Some),
            ObjectType::Viewport => self.read_viewport().map(Some),
            ObjectType::Ray => self.read_ray().map(Some),
            ObjectType::XLine => self.read_xline().map(Some),
            ObjectType::Attrib => self.read_attrib().map(Some),
            ObjectType::AttDef => self.read_attdef().map(Some),
            ObjectType::DimensionOrdinate => self.read_dim_ordinate().map(Some),
            ObjectType::DimensionLinear => self.read_dim_linear().map(Some),
            ObjectType::DimensionAligned => self.read_dim_aligned().map(Some),
            ObjectType::DimensionAng3Pt => self.read_dim_angular3pt().map(Some),
            ObjectType::DimensionRadius => self.read_dim_radius().map(Some),
            ObjectType::DimensionDiameter => self.read_dim_diameter().map(Some),
            ObjectType::Hatch => self.read_hatch().map(Some),
            ObjectType::Layer => self.read_layer().map(Some),
            ObjectType::Linetype => self.read_linetype().map(Some),
            ObjectType::ShapeFile => self.read_textstyle().map(Some),
            ObjectType::DimStyle => self.read_dimstyle().map(Some),
            ObjectType::BlockHeader => self.read_block_record().map(Some),
            ObjectType::Dictionary => self.read_dictionary().map(Some),
            // Control objects - read and queue entries
            ObjectType::BlockControlObj => { self.read_control_object()?; Ok(None) },
            ObjectType::LayerControlObj => { self.read_control_object()?; Ok(None) },
            ObjectType::ShapeFileControlObj => { self.read_control_object()?; Ok(None) },
            ObjectType::LinetypeControlObj => { self.read_control_object()?; Ok(None) },
            ObjectType::ViewControlObj => { self.read_control_object()?; Ok(None) },
            ObjectType::UcsControlObj => { self.read_control_object()?; Ok(None) },
            ObjectType::VPortControlObj => { self.read_control_object()?; Ok(None) },
            ObjectType::AppIdControlObj => { self.read_control_object()?; Ok(None) },
            ObjectType::DimStyleControlObj => { self.read_control_object()?; Ok(None) },
            _ => {
                // Unknown or unsupported type
                Ok(None)
            }
        };
        
        // Log type on error for debugging
        if let Err(ref e) = result {
            eprintln!("Error reading type_code={} ({:?}) handle={}: {:?}", type_code, object_type, handle, e);
        }
        
        result
    }
    
    /// Read the object type code
    fn read_object_type(&mut self) -> Result<u16> {
        if self.version >= ACadVersion::AC1018 {
            // R2004+: Object type is BS
            Ok(self.reader.read_bitshort()? as u16)
        } else {
            // R13-R15: Object type is 2 bytes
            Ok(self.reader.read_raw_ushort()?)
        }
    }
    
    /// Read xref-dependant bit from table entries (readXrefDependantBit in C#)
    /// Returns whether the entry is xref-dependent
    fn read_xref_dependant_bit(&mut self) -> Result<bool> {
        if self.version >= ACadVersion::AC1021 {
            // R2007+: BS xrefindex (bit 8 = xdep)
            let xrefindex = self.reader.read_bitshort()? as u16;
            Ok((xrefindex & 0x100) != 0)
        } else if self.version >= ACadVersion::AC1015 {
            // R2000-R2004: B 64-flag + BS xrefindex + B Xdep
            let _flag_64 = self.reader.read_bit()?;
            let _xrefindex = self.reader.read_bitshort()?;
            let xdep = self.reader.read_bit()?;
            Ok(xdep)
        } else {
            // R13-R14: B 64-flag + BS xrefindex
            let _flag_64 = self.reader.read_bit()?;
            let _xrefindex = self.reader.read_bitshort()?;
            // R13-R14: The Xdep bit is separate
            let xdep = self.reader.read_bit()?;
            Ok(xdep)
        }
    }
    
    /// Read extended entity data (EED)
    /// Format: BS size, while size != 0: Handle (app), size bytes of data, BS next_size
    fn read_extended_data(&mut self, data: &mut DwgEntityData) -> Result<()> {
        let mut size = self.reader.read_bitshort()?;
        while size != 0 {
            let _app_handle = self.reader.read_handle()?;
            let eed_size = size as usize;
            if eed_size > 100000 {
                return Err(DxfError::Parse(format!("EED size too large: {}", eed_size)));
            }
            let eed_bytes = self.reader.read_bytes(eed_size)?;
            data.xdata.extend_from_slice(&eed_bytes);
            size = self.reader.read_bitshort()?;
        }
        Ok(())
    }
    
    /// Read extended data for non-entity objects (same format)
    fn read_extended_data_obj(&mut self, data: &mut DwgObjectData) -> Result<()> {
        let mut size = self.reader.read_bitshort()?;
        while size != 0 {
            let _app_handle = self.reader.read_handle()?;
            let eed_size = size as usize;
            if eed_size > 100000 {
                return Err(DxfError::Parse(format!("EED size too large: {}", eed_size)));
            }
            let eed_bytes = self.reader.read_bytes(eed_size)?;
            data.xdata.extend_from_slice(&eed_bytes);
            size = self.reader.read_bitshort()?;
        }
        Ok(())
    }
    
    /// Read common entity data
    fn read_entity_data(&mut self) -> Result<DwgEntityData> {
        let mut data = DwgEntityData::default();
        
        // R2000-R2007: First read RL - size of object data in bits (before handles)
        // This is used to position the handle reader at the end of object data
        // For R2010+, this was already read before the object type
        if self.version >= ACadVersion::AC1015 && self.version < ACadVersion::AC1024 {
            let _size_bits = self.reader.read_raw_long()? as u64;
        }
        
        // Handle 
        data.handle = self.reader.read_handle()?;
        
        // Extended data (EED)
        // Format: BS size, then while size != 0: Handle (app), size bytes of data, BS next_size
        self.read_extended_data(&mut data)?;
        
        // Graphic data
        data.has_graphics = self.reader.read_bit()?;
        if data.has_graphics {
            // R13-R2007: RL (raw long) size in bytes
            // R2010+: BLL (bit long long) size in bytes
            let graphics_size = if self.version >= ACadVersion::AC1024 {
                self.reader.read_bitlonglong()? as usize
            } else {
                self.reader.read_raw_long()? as usize
            };
            if graphics_size > 0 && graphics_size < 100_000_000 {
                // Skip the graphic data (advance by graphics_size bytes)
                self.reader.advance_bytes(graphics_size)?;
            }
        }
        
        // Entity mode (BB)
        data.entity_mode = self.reader.read_2bits()?;
        
        // Reactor count
        data.num_reactors = self.reader.read_bitlong()? as u32;
        
        // R2004+: XDic missing flag
        if self.version >= ACadVersion::AC1018 {
            data.xdic_missing = self.reader.read_bit()?;
        }
        
        // R2013+: Binary data flag
        if self.version >= ACadVersion::AC1027 {
            data.binary_data_missing = self.reader.read_bit()?;
        }
        
        // R13-R14: Separate visibility flag
        if self.version <= ACadVersion::AC1014 {
            data.invisible = !self.reader.read_bit()?;
        }
        
        // R13-R14: Entity handle
        if self.version <= ACadVersion::AC1014 {
            let _entity_handle = self.reader.read_handle()?;
        }
        
        // R13-R2000+: No-links flag for prev/next entity handles
        // For R2004+, this is always 1 (links not used)
        let no_links = if self.version < ACadVersion::AC1018 {
            self.reader.read_bit()?
        } else {
            true
        };
        
        // R13-R14 only: Previous/next entity handles if no_links is false
        // R2000+: These handles are read from a different stream, so we skip here
        if self.version <= ACadVersion::AC1014 && !no_links {
            // Previous entity handle
            let _prev = self.reader.read_handle_reference(data.handle)?;
            // Next entity handle
            let _next = self.reader.read_handle_reference(data.handle)?;
        }
        
        // Color (CMC for R13-R14, EnColor for R2000+)
        data.color = self.reader.read_cmc_color()?;
        
        // R2004+: If color has book name flag, read color handle
        // (simplified - we just store the color)
        
        // Linetype scale (BD 48)
        // For R2000+: Direct BitDouble
        // For R13-R14: possibly different format
        data.linetype_scale = if self.version >= ACadVersion::AC1015 {
            self.reader.read_bitdouble()?
        } else {
            // R13-R14 may have a flag bit first
            let has_linetype_scale = self.reader.read_bit()?;
            if has_linetype_scale {
                self.reader.read_bitdouble()?
            } else {
                1.0
            }
        };
        
        // R2000+: Linetype flags (BB)
        if self.version >= ACadVersion::AC1015 {
            data.linetype_flags = self.reader.read_2bits()?;
        }
        
        // R2007+: Material flags + Shadow flags
        if self.version >= ACadVersion::AC1021 {
            data.material_flags = self.reader.read_2bits()?;
            data.shadow_flags = self.reader.read_raw_char()? as u8; // RC = raw byte, NOT 2 bits!
        }
        
        // R2000+: Plotstyle flags (BB)
        if self.version >= ACadVersion::AC1015 {
            data.plotstyle_flags = self.reader.read_2bits()?;
        }
        
        // R2010+: Visual style flags
        if self.version >= ACadVersion::AC1024 {
            data.has_full_visualstyle = self.reader.read_bit()?;
            data.has_face_visualstyle = self.reader.read_bit()?;
            data.has_edge_visualstyle = self.reader.read_bit()?;
        }
        
        // Invisibility (R2000+)
        if self.version >= ACadVersion::AC1015 {
            let invis = self.reader.read_bitshort()?;
            data.invisible = (invis & 0x01) != 0;
        }
        
        // R2000+: lineweight
        if self.version >= ACadVersion::AC1015 {
            data.lineweight = self.reader.read_raw_char()? as i16;
        }
        
        Ok(data)
    }
    
    /// Read handle references at end of entity data
    fn read_entity_handles(&mut self, data: &mut DwgEntityData) -> Result<()> {
        // R2000-R2007: Handle references are at a separate position
        // For now, skip handle reading for these versions
        // TODO: Implement proper handle reader positioning
        if self.version >= ACadVersion::AC1015 && self.version < ACadVersion::AC1024 {
            return Ok(());
        }
        
        // Owner handle (usually block record)
        data.owner_handle = Some(self.reader.read_handle_reference(data.handle)?);
        
        // Reactor handles
        for _ in 0..data.num_reactors {
            data.reactor_handles.push(self.reader.read_handle_reference(data.handle)?);
        }
        
        // XDictionary
        if !data.xdic_missing {
            data.xdic_handle = Some(self.reader.read_handle_reference(data.handle)?);
        }
        
        // R13-R14: Previous/next entity in block
        if self.version <= ACadVersion::AC1014 {
            data.prev_entity = Some(self.reader.read_handle_reference(data.handle)?);
            data.next_entity = Some(self.reader.read_handle_reference(data.handle)?);
        }
        
        // Layer handle
        data.layer_handle = Some(self.reader.read_handle_reference(data.handle)?);
        
        // Linetype handle (depending on flags)
        if data.linetype_flags == 3 {
            data.linetype_handle = Some(self.reader.read_handle_reference(data.handle)?);
        }
        
        // R2000+: Plotstyle
        if self.version >= ACadVersion::AC1015 && data.plotstyle_flags == 3 {
            data.plotstyle_handle = Some(self.reader.read_handle_reference(data.handle)?);
        }
        
        // R2007+: Material
        if self.version >= ACadVersion::AC1021 && data.material_flags == 3 {
            data.material_handle = Some(self.reader.read_handle_reference(data.handle)?);
        }
        
        // R2010+: Visual style handles
        if self.version >= ACadVersion::AC1024 {
            if data.has_full_visualstyle {
                data.visualstyle_handle = Some(self.reader.read_handle_reference(data.handle)?);
            }
        }
        
        Ok(())
    }
    
    // === Entity readers ===
    
    /// Read a LINE entity
    fn read_line(&mut self) -> Result<CadTemplate> {
        let mut data = self.read_entity_data()?;
        
        let (start, end) = if self.version >= ACadVersion::AC1015 {
            // R2000+: Special format with interleaved X/Y/Z and bit-double-with-default
            // Z's are zero bit
            let z_is_zero = self.reader.read_bit()?;
            
            // Start Point X (RD - raw double)
            let start_x = self.reader.read_raw_double()?;
            // End Point X (DD - bit double with default = startX)
            let end_x = self.reader.read_bitdouble_with_default(start_x)?;
            // Start Point Y (RD)
            let start_y = self.reader.read_raw_double()?;
            // End Point Y (DD with default = startY)
            let end_y = self.reader.read_bitdouble_with_default(start_y)?;
            
            let (start_z, end_z) = if !z_is_zero {
                // Start Point Z (RD)
                let sz = self.reader.read_raw_double()?;
                // End Point Z (DD with default = startZ)
                let ez = self.reader.read_bitdouble_with_default(sz)?;
                (sz, ez)
            } else {
                (0.0, 0.0)
            };
            
            (Vector3::new(start_x, start_y, start_z), Vector3::new(end_x, end_y, end_z))
        } else {
            // R13-R14: Start/end are 3BD
            let start = self.reader.read_3bitdouble()?;
            let end = self.reader.read_3bitdouble()?;
            (start, end)
        };
        
        // Thickness
        let thickness = self.reader.read_bit_thickness()?;
        
        // Extrusion
        let extrusion = self.reader.read_bit_extrusion()?;
        
        // Read handle references
        self.read_entity_handles(&mut data)?;
        
        // Queue referenced handles
        self.queue_entity_handles(&data);
        
        Ok(CadTemplate::Line {
            entity_data: data,
            start,
            end,
            thickness,
            extrusion,
        })
    }
    
    /// Read a CIRCLE entity
    fn read_circle(&mut self) -> Result<CadTemplate> {
        let mut data = self.read_entity_data()?;
        
        let center = self.reader.read_3bitdouble()?;
        let radius = self.reader.read_bitdouble()?;
        let thickness = self.reader.read_bit_thickness()?;
        let extrusion = self.reader.read_bit_extrusion()?;
        
        self.read_entity_handles(&mut data)?;
        self.queue_entity_handles(&data);
        
        Ok(CadTemplate::Circle {
            entity_data: data,
            center,
            radius,
            thickness,
            extrusion,
        })
    }
    
    /// Read an ARC entity
    fn read_arc(&mut self) -> Result<CadTemplate> {
        let mut data = self.read_entity_data()?;
        
        let center = self.reader.read_3bitdouble()?;
        let radius = self.reader.read_bitdouble()?;
        let thickness = self.reader.read_bit_thickness()?;
        let extrusion = self.reader.read_bit_extrusion()?;
        let start_angle = self.reader.read_bitdouble()?;
        let end_angle = self.reader.read_bitdouble()?;
        
        self.read_entity_handles(&mut data)?;
        self.queue_entity_handles(&data);
        
        Ok(CadTemplate::Arc {
            entity_data: data,
            center,
            radius,
            thickness,
            extrusion,
            start_angle,
            end_angle,
        })
    }
    
    /// Read a POINT entity
    fn read_point(&mut self) -> Result<CadTemplate> {
        let mut data = self.read_entity_data()?;
        
        let location = self.reader.read_3bitdouble()?;
        let thickness = self.reader.read_bit_thickness()?;
        let extrusion = self.reader.read_bit_extrusion()?;
        let x_axis_angle = self.reader.read_bitdouble()?;
        
        self.read_entity_handles(&mut data)?;
        self.queue_entity_handles(&data);
        
        Ok(CadTemplate::Point {
            entity_data: data,
            location,
            thickness,
            extrusion,
            x_axis_angle,
        })
    }
    
    /// Read a LWPOLYLINE entity
    fn read_lwpolyline(&mut self) -> Result<CadTemplate> {
        let mut data = self.read_entity_data()?;
        
        let flag = self.reader.read_bitshort()?;
        
        let has_const_width = (flag & 0x4) != 0;
        let has_elevation = (flag & 0x8) != 0;
        let has_thickness = (flag & 0x2) != 0;
        let has_extrusion = (flag & 0x1) != 0;
        let has_bulges = (flag & 0x10) != 0;
        let has_vertex_ids = (flag & 0x400) != 0;
        let has_widths = (flag & 0x20) != 0;
        
        let const_width = if has_const_width {
            self.reader.read_bitdouble()?
        } else {
            0.0
        };
        
        let elevation = if has_elevation {
            self.reader.read_bitdouble()?
        } else {
            0.0
        };
        
        let thickness = if has_thickness {
            self.reader.read_bitdouble()?
        } else {
            0.0
        };
        
        let extrusion = if has_extrusion {
            self.reader.read_3bitdouble()?
        } else {
            Vector3::new(0.0, 0.0, 1.0)
        };
        
        let num_vertices = self.reader.read_bitlong()? as usize;
        
        let num_bulges = if has_bulges {
            self.reader.read_bitlong()? as usize
        } else {
            0
        };
        
        let num_vertex_ids = if has_vertex_ids {
            self.reader.read_bitlong()? as usize
        } else {
            0
        };
        
        let num_widths = if has_widths {
            self.reader.read_bitlong()? as usize
        } else {
            0
        };
        
        // Read vertices
        // R13-R14: each vertex is 2RD
        // R2000+: first vertex is 2RD, rest are 2DD (with previous as default)
        let mut vertices = Vec::with_capacity(num_vertices);
        
        if num_vertices > 0 {
            // First vertex always read as 2RD (2 raw doubles)
            let first_pt = self.reader.read_2raw_double()?;
            vertices.push(LwPolylineVertex {
                point: first_pt,
                ..Default::default()
            });
            
            if self.version <= ACadVersion::AC1014 {
                // R13-R14: all remaining vertices are 2RD
                for _ in 1..num_vertices {
                    let pt = self.reader.read_2raw_double()?;
                    vertices.push(LwPolylineVertex {
                        point: pt,
                        ..Default::default()
                    });
                }
            } else {
                // R2000+: remaining vertices are 2DD with default = previous
                let mut last_point = first_pt;
                for _ in 1..num_vertices {
                    let pt = self.reader.read_2bitdouble_with_default(last_point)?;
                    last_point = pt;
                    vertices.push(LwPolylineVertex {
                        point: pt,
                        ..Default::default()
                    });
                }
            }
        }
        
        // Read bulges
        for i in 0..num_bulges.min(num_vertices) {
            vertices[i].bulge = self.reader.read_bitdouble()?;
        }
        
        // Read vertex IDs
        for i in 0..num_vertex_ids.min(num_vertices) {
            vertices[i].vertex_id = self.reader.read_bitlong()?;
        }
        
        // Read widths
        for i in 0..num_widths.min(num_vertices) {
            vertices[i].start_width = self.reader.read_bitdouble()?;
            vertices[i].end_width = self.reader.read_bitdouble()?;
        }
        
        self.read_entity_handles(&mut data)?;
        self.queue_entity_handles(&data);
        
        Ok(CadTemplate::LwPolyline {
            entity_data: data,
            flag,
            const_width,
            elevation,
            thickness,
            extrusion,
            vertices,
        })
    }
    
    /// Read a TEXT entity
    fn read_text(&mut self) -> Result<CadTemplate> {
        let mut data = self.read_entity_data()?;
        
        let mut elevation = 0.0;
        let insertion;
        let mut alignment;
        let extrusion;
        let thickness;
        let mut oblique_angle = 0.0;
        let mut rotation = 0.0;
        let height;
        let mut width_factor = 1.0;
        let value;
        let mut generation_flags: i16 = 0;
        let mut horizontal_alignment: i16 = 0;
        let mut vertical_alignment: i16 = 0;
        
        if self.version <= ACadVersion::AC1014 {
            // R13-R14
            elevation = self.reader.read_bitdouble()?;
            let pt = self.reader.read_2raw_double()?;
            insertion = Vector3::new(pt.x, pt.y, elevation);
            
            let apt = self.reader.read_2raw_double()?;
            alignment = Vector3::new(apt.x, apt.y, elevation);
            
            extrusion = self.reader.read_3bitdouble()?;
            thickness = self.reader.read_bitdouble()?;
            oblique_angle = self.reader.read_bitdouble()?;
            rotation = self.reader.read_bitdouble()?;
            height = self.reader.read_bitdouble()?;
            width_factor = self.reader.read_bitdouble()?;
            value = self.reader.read_variable_text(self.version)?;
            generation_flags = self.reader.read_bitshort()?;
            horizontal_alignment = self.reader.read_bitshort()?;
            vertical_alignment = self.reader.read_bitshort()?;
        } else {
            // R2000+: Compact format with dataFlags
            let data_flags = self.reader.read_raw_char()?;  // RC - raw byte
            
            // Elevation RD if !(flags & 0x01)
            if (data_flags & 0x01) == 0 {
                elevation = self.reader.read_raw_double()?;
            }
            
            // Insertion pt 2RD
            let pt = self.reader.read_2raw_double()?;
            insertion = Vector3::new(pt.x, pt.y, elevation);
            
            // Alignment pt 2DD if !(flags & 0x02), defaults from insertion
            if (data_flags & 0x02) == 0 {
                let ax = self.reader.read_bitdouble_with_default(insertion.x)?;
                let ay = self.reader.read_bitdouble_with_default(insertion.y)?;
                alignment = Vector3::new(ax, ay, elevation);
            } else {
                alignment = insertion;
            }
            
            // Extrusion BE
            extrusion = self.reader.read_bit_extrusion()?;
            // Thickness BT
            thickness = self.reader.read_bit_thickness()?;
            
            // Oblique angle RD if !(flags & 0x04)
            if (data_flags & 0x04) == 0 {
                oblique_angle = self.reader.read_raw_double()?;
            }
            // Rotation RD if !(flags & 0x08)
            if (data_flags & 0x08) == 0 {
                rotation = self.reader.read_raw_double()?;
            }
            // Height RD (always present)
            height = self.reader.read_raw_double()?;
            // Width factor RD if !(flags & 0x10)
            if (data_flags & 0x10) == 0 {
                width_factor = self.reader.read_raw_double()?;
            }
            
            // Text value TV
            value = self.reader.read_variable_text(self.version)?;
            
            // Generation BS if !(flags & 0x20)
            if (data_flags & 0x20) == 0 {
                generation_flags = self.reader.read_bitshort()?;
            }
            // Horiz align BS if !(flags & 0x40)
            if (data_flags & 0x40) == 0 {
                horizontal_alignment = self.reader.read_bitshort()?;
            }
            // Vert align BS if !(flags & 0x80)
            if (data_flags as u8 & 0x80) == 0 {
                vertical_alignment = self.reader.read_bitshort()?;
            }
        }
        
        self.read_entity_handles(&mut data)?;
        self.queue_entity_handles(&data);
        
        // Style handle is read from the handles stream (skipped for R2004)
        let style_handle = None;
        
        Ok(CadTemplate::Text {
            entity_data: data,
            insertion,
            alignment,
            extrusion,
            thickness,
            oblique_angle,
            rotation,
            height,
            width_factor,
            value,
            generation_flags,
            horizontal_alignment,
            vertical_alignment,
            style_handle,
        })
    }
    
    // === Table entry readers ===
    
    /// Read a LAYER table entry
    fn read_layer(&mut self) -> Result<CadTemplate> {
        let mut data = DwgObjectData::default();
        
        // R2000-R2007: First read RL - size of object data in bits (before handles)
        if self.version >= ACadVersion::AC1015 && self.version < ACadVersion::AC1024 {
            let _obj_size_bits = self.reader.read_raw_long()? as u64;
        }
        
        // Handle and xdata
        data.handle = self.reader.read_handle()?;
        self.read_extended_data_obj(&mut data)?;
        
        data.num_reactors = self.reader.read_bitlong()? as u32;
        
        if self.version >= ACadVersion::AC1018 {
            data.xdic_missing = self.reader.read_bit()?;
        }
        
        // Layer name
        let name = self.reader.read_variable_text(self.version)?;
        
        // readXrefDependantBit
        let xref_dep = self.read_xref_dependant_bit()?;
        
        // R13-R14: individual bits
        let (mut is_frozen, mut is_on, mut is_locked, mut is_plotting, lineweight) = if self.version <= ACadVersion::AC1014 {
            let frozen = self.reader.read_bit()?;
            let on = self.reader.read_bit()?;
            let _frozen_new_vp = self.reader.read_bit()?;
            let locked = self.reader.read_bit()?;
            (frozen, on, locked, true, -1i16)
        } else {
            // R2000+: packed BS with frozen/on/frozen_new/locked/plotting/lineweight
            let values = self.reader.read_bitshort()? as u16;
            let frozen = (values & 0x01) != 0;
            let on = (values & 0x02) == 0; // bit 1 = OFF, so on = !bit
            let locked = (values & 0x08) != 0;
            let plotting = (values & 0x10) != 0;
            let lw_byte = ((values & 0x03E0) >> 5) as i16;
            (frozen, on, locked, plotting, lw_byte)
        };
        
        // CMC color
        let color = self.reader.read_cmc_color()?;
        
        // Handle references
        data.owner_handle = Some(self.reader.read_handle_reference(data.handle)?);
        
        for _ in 0..data.num_reactors {
            data.reactor_handles.push(self.reader.read_handle_reference(data.handle)?);
        }
        
        if !data.xdic_missing {
            data.xdic_handle = Some(self.reader.read_handle_reference(data.handle)?);
        }
        
        // R2000+: External reference handle
        if self.version >= ACadVersion::AC1015 && xref_dep {
            let _xref_handle = self.reader.read_handle_reference(data.handle)?;
        }
        
        // Plotstyle handle
        let plotstyle_handle = if self.version >= ACadVersion::AC1015 {
            Some(self.reader.read_handle_reference(data.handle)?)
        } else {
            None
        };
        
        // Linetype handle  
        let linetype_handle = Some(self.reader.read_handle_reference(data.handle)?);
        
        // R2007+: Material handle
        let material_handle = if self.version >= ACadVersion::AC1021 {
            Some(self.reader.read_handle_reference(data.handle)?)
        } else {
            None
        };
        
        let flags = if is_frozen { 1 } else { 0 }
            | if !is_on { 2 } else { 0 }
            | if is_locked { 4 } else { 0 }
            | if xref_dep { 16 } else { 0 };
        
        Ok(CadTemplate::Layer {
            object_data: data,
            name,
            flags,
            color,
            linetype_handle,
            plotstyle_handle,
            material_handle,
            is_on,
            is_frozen,
            is_locked,
            is_plotting,
            lineweight,
        })
    }
    
    /// Read a LINETYPE table entry
    fn read_linetype(&mut self) -> Result<CadTemplate> {
        let mut data = DwgObjectData::default();
        
        // R2000-R2007: First read RL - size of object data in bits (before handles)
        if self.version >= ACadVersion::AC1015 && self.version < ACadVersion::AC1024 {
            let _obj_size_bits = self.reader.read_raw_long()? as u64;
        }
        
        data.handle = self.reader.read_handle()?;
        self.read_extended_data_obj(&mut data)?;
        
        data.num_reactors = self.reader.read_bitlong()? as u32;
        
        if self.version >= ACadVersion::AC1018 {
            data.xdic_missing = self.reader.read_bit()?;
        }
        
        let name = self.reader.read_variable_text(self.version)?;
        
        // readXrefDependantBit
        let xref_dep = self.read_xref_dependant_bit()?;
        
        let description = self.reader.read_variable_text(self.version)?;
        let pattern_length = self.reader.read_bitdouble()?;
        let alignment = self.reader.read_raw_char()?;
        let num_dashes = self.reader.read_raw_char()? as usize;
        
        let mut elements = Vec::with_capacity(num_dashes);
        let mut has_text = false;
        for _ in 0..num_dashes {
            let mut elem = LineTypeElement::default();
            // All 7 fields are always present (unconditional)
            elem.dash_length = self.reader.read_bitdouble()?;       // BD 49
            elem.shape_number = self.reader.read_bitshort()?;        // BS 75 (complex shapecode)
            elem.offset = self.reader.read_2raw_double()?;           // 2RD 44,45
            elem.scale = self.reader.read_bitdouble()?;              // BD 46
            elem.rotation = self.reader.read_bitdouble()?;           // BD 50
            elem.shape_flag = self.reader.read_bitshort()?;          // BS 74 (shapeflag)
            
            if (elem.shape_flag & 0x04) != 0 {
                has_text = true;
            }
            
            elements.push(elem);
        }
        
        // Text area: 256 bytes for R2004 and earlier, 512 bytes for R2007+ if text present
        if self.version <= ACadVersion::AC1018 {
            let _textarea = self.reader.read_bytes(256)?;
        } else if self.version >= ACadVersion::AC1021 && has_text {
            let _textarea = self.reader.read_bytes(512)?;
        }
        
        // Skip handle reading for R2004 (handles are in separate stream)
        if self.version >= ACadVersion::AC1024 {
            // Handle references
            data.owner_handle = Some(self.reader.read_handle_reference(data.handle)?);
            
            for _ in 0..data.num_reactors {
                data.reactor_handles.push(self.reader.read_handle_reference(data.handle)?);
            }
            
            if !data.xdic_missing {
                data.xdic_handle = Some(self.reader.read_handle_reference(data.handle)?);
            }
            
            // Ltype control handle
            let _ltype_control = self.reader.read_handle_reference(data.handle)?;
            
            // Style handles (one per dash)
            for elem in elements.iter_mut() {
                elem.style_handle = Some(self.reader.read_handle_reference(data.handle)?);
            }
        }
        
        let flags = if xref_dep { 16 } else { 0 };
        
        Ok(CadTemplate::LineType {
            object_data: data,
            name,
            description,
            flags,
            pattern_length,
            alignment,
            elements,
        })
    }
    
    /// Read a BLOCK_RECORD/BLOCK_HEADER
    fn read_block_record(&mut self) -> Result<CadTemplate> {
        let mut data = DwgObjectData::default();
        
        // R2000-R2007: First read RL - size of object data in bits (before handles)
        if self.version >= ACadVersion::AC1015 && self.version < ACadVersion::AC1024 {
            let _obj_size_bits = self.reader.read_raw_long()? as u64;
        }
        
        data.handle = self.reader.read_handle()?;
        self.read_extended_data_obj(&mut data)?;
        
        data.num_reactors = self.reader.read_bitlong()? as u32;
        
        if self.version >= ACadVersion::AC1018 {
            data.xdic_missing = self.reader.read_bit()?;
        }
        
        let name = self.reader.read_variable_text(self.version)?;
        
        // readXrefDependantBit
        let xref_dep = self.read_xref_dependant_bit()?;
        
        // Anonymous B
        let is_anonymous = self.reader.read_bit()?;
        // HasAtts B
        let has_attributes = self.reader.read_bit()?;
        // BlkIsXref B
        let is_xref = self.reader.read_bit()?;
        // XrefOverlaid B
        let is_overlaid = self.reader.read_bit()?;
        
        // R2000+: Loaded bit (unconditional)
        let loaded = if self.version >= ACadVersion::AC1015 {
            self.reader.read_bit()?
        } else {
            false
        };
        
        // R2004+: Owned Object Count BL (only if not xref/overlay)
        let n_owned_objects = if self.version >= ACadVersion::AC1018 && !is_xref && !is_overlaid {
            self.reader.read_bitlong()? as usize
        } else {
            0
        };
        
        // Base point 3BD
        let _base_point = self.reader.read_3bitdouble()?;
        // Xref path
        let xref_path = self.reader.read_variable_text(self.version)?;
        
        // R2000+: Insert count loop (series of non-zero RCs, followed by 0 RC)
        let mut insert_count = 0usize;
        if self.version >= ACadVersion::AC1015 {
            loop {
                let b = self.reader.read_raw_char()?;
                if b == 0 { break; }
                insert_count += 1;
            }
            
            // Block description TV
            let _description = self.reader.read_variable_text(self.version)?;
            
            // Preview data BL size + N bytes
            let preview_size = self.reader.read_bitlong()? as usize;
            if preview_size > 0 && preview_size < 10_000_000 {
                for _ in 0..preview_size {
                    let _ = self.reader.read_raw_char()?;
                }
            }
        }
        
        // R2007+: Insert units, Explodable, Block scaling
        let insert_units = if self.version >= ACadVersion::AC1021 {
            self.reader.read_bitshort()?
        } else {
            0
        };
        
        let is_explodable = if self.version >= ACadVersion::AC1021 {
            self.reader.read_bit()?
        } else {
            true
        };
        
        let can_scale = if self.version >= ACadVersion::AC1021 {
            self.reader.read_raw_char()? > 0  // RC (byte), not B (bit)
        } else {
            true
        };
        
        // Skip handle reading for R2004 (handles are in separate stream)
        if self.version >= ACadVersion::AC1024 {
            // Handle references
            data.owner_handle = Some(self.reader.read_handle_reference(data.handle)?);
            
            for _ in 0..data.num_reactors {
                data.reactor_handles.push(self.reader.read_handle_reference(data.handle)?);
            }
            
            if !data.xdic_missing {
                data.xdic_handle = Some(self.reader.read_handle_reference(data.handle)?);
            }
            
            // NULL handle
            let _ = self.reader.read_handle_reference(data.handle)?;
            // Block entity handle
            let _block_entity = self.reader.read_handle_reference(data.handle)?;
            
            // R2004+: Owned objects
            for _ in 0..n_owned_objects {
                let _ = self.reader.read_handle_reference(data.handle)?;
            }
            
            // EndBlk entity
            let _ = self.reader.read_handle_reference(data.handle)?;
            
            // Insert handles
            for _ in 0..insert_count {
                let _ = self.reader.read_handle_reference(data.handle)?;
            }
            
            // Layout handle
            let _ = self.reader.read_handle_reference(data.handle)?;
        }
        
        let flags: i16 = {
            let mut f: i16 = 0;
            if is_anonymous { f |= 1; }
            if has_attributes { f |= 2; }
            if is_xref { f |= 4; }
            if is_overlaid { f |= 8; }
            if xref_dep { f |= 16; }
            f
        };
        
        Ok(CadTemplate::BlockRecord {
            object_data: data,
            name,
            flags,
            is_xref,
            is_overlaid,
            is_anonymous,
            has_attributes,
            is_xref_resolved: false,
            xref_path,
            block_entity: None,
            endblk_entity: None,
            first_entity: None,
            last_entity: None,
            layout_handle: None,
            insert_units,
            is_explodable,
            can_scale,
        })
    }
    
    /// Read a DICTIONARY object
    fn read_dictionary(&mut self) -> Result<CadTemplate> {
        let mut data = DwgObjectData::default();
        
        // R2000-R2007: First read RL - size of object data in bits (before handles)
        if self.version >= ACadVersion::AC1015 && self.version < ACadVersion::AC1024 {
            let _obj_size_bits = self.reader.read_raw_long()? as u64;
        }
        
        data.handle = self.reader.read_handle()?;
        self.read_extended_data_obj(&mut data)?;
        
        data.num_reactors = self.reader.read_bitlong()? as u32;
        
        if self.version >= ACadVersion::AC1018 {
            data.xdic_missing = self.reader.read_bit()?;
        }
        
        let num_entries = self.reader.read_bitlong()? as u32;
        
        // Bounds check to avoid capacity overflow
        if num_entries > 10000 {
            return Err(DxfError::Parse("Dictionary has too many entries".to_string()));
        }
        
        // Cloning flag (R2000+)
        let cloning_flag = if self.version >= ACadVersion::AC1015 {
            self.reader.read_bitshort()? as u8
        } else {
            0
        };
        
        // Hard owner flag (R2000+)
        let hard_owner_flag = if self.version >= ACadVersion::AC1015 {
            self.reader.read_raw_char()? != 0
        } else {
            false
        };
        
        // R2000-R2007: Skip handle reading (handles are in separate stream)
        // R2010+: Read handles inline
        if self.version >= ACadVersion::AC1024 {
            // Handle references
            data.owner_handle = Some(self.reader.read_handle_reference(data.handle)?);
            
            for _ in 0..data.num_reactors {
                data.reactor_handles.push(self.reader.read_handle_reference(data.handle)?);
            }
            
            if !data.xdic_missing {
                data.xdic_handle = Some(self.reader.read_handle_reference(data.handle)?);
            }
            
            // Entry handles and names (for R2010+, handles are inline)
            let mut entries = Vec::with_capacity(num_entries as usize);
            for _ in 0..num_entries {
                let entry_handle = self.reader.read_handle_reference(data.handle)?;
                let entry_name = self.reader.read_variable_text(self.version)?;
                entries.push((entry_name, entry_handle));
                
                // Queue the entry handles
                self.handle_queue.push_back(entry_handle);
            }
            
            return Ok(CadTemplate::Dictionary {
                object_data: data,
                num_entries,
                cloning_flag,
                hard_owner_flag,
                entries,
            });
        }
        
        // R2000-R2007: Entry names only (handles are in separate stream)
        // For now, skip the names since we can't read entries properly without handles
        Ok(CadTemplate::Dictionary {
            object_data: data,
            num_entries,
            cloning_flag,
            hard_owner_flag,
            entries: Vec::new(),
        })
    }
    
    // === Additional entity readers ===
    
    /// Read an MTEXT entity
    fn read_mtext(&mut self) -> Result<CadTemplate> {
        let mut data = self.read_entity_data()?;
        
        let insertion = self.reader.read_3bitdouble()?;
        // MTEXT extrusion is 3BD, not BE
        let extrusion = self.reader.read_3bitdouble()?;
        let x_direction = self.reader.read_3bitdouble()?;
        let rect_width = self.reader.read_bitdouble()?;
        
        // R2007+: rect_height
        let rect_height = if self.version >= ACadVersion::AC1021 {
            self.reader.read_bitdouble()?
        } else {
            0.0
        };
        
        let text_height = self.reader.read_bitdouble()?;
        let attachment = self.reader.read_bitshort()? as u8;
        let drawing_direction = self.reader.read_bitshort()? as u8;
        
        // R2000+: extended text height, linespacing
        let _ext_height = if self.version >= ACadVersion::AC1015 {
            self.reader.read_bitdouble()?
        } else {
            0.0
        };
        
        let _ext_height_2 = if self.version >= ACadVersion::AC1015 {
            self.reader.read_bitdouble()?
        } else {
            0.0
        };
        
        let contents = self.reader.read_variable_text(self.version)?;
        
        let line_spacing_style = if self.version >= ACadVersion::AC1015 {
            self.reader.read_bitshort()? as u8
        } else {
            1
        };
        
        let line_spacing_factor = if self.version >= ACadVersion::AC1015 {
            self.reader.read_bitdouble()?
        } else {
            1.0
        };
        
        self.read_entity_handles(&mut data)?;
        let style_handle = Some(self.reader.read_handle_reference(data.handle)?);
        
        self.queue_entity_handles(&data);
        
        Ok(CadTemplate::MText {
            entity_data: data,
            insertion,
            extrusion,
            x_direction,
            rect_width,
            rect_height,
            text_height,
            attachment,
            drawing_direction,
            line_spacing_style,
            line_spacing_factor,
            contents,
            style_handle,
        })
    }
    
    /// Read an ELLIPSE entity
    fn read_ellipse(&mut self) -> Result<CadTemplate> {
        let mut data = self.read_entity_data()?;
        
        let center = self.reader.read_3bitdouble()?;
        let major_axis = self.reader.read_3bitdouble()?;
        // Ellipse extrusion is 3BD (not BE)
        let extrusion = self.reader.read_3bitdouble()?;
        let axis_ratio = self.reader.read_bitdouble()?;
        let start_angle = self.reader.read_bitdouble()?;
        let end_angle = self.reader.read_bitdouble()?;
        
        self.read_entity_handles(&mut data)?;
        self.queue_entity_handles(&data);
        
        Ok(CadTemplate::Ellipse {
            entity_data: data,
            center,
            major_axis,
            extrusion,
            axis_ratio,
            start_angle,
            end_angle,
        })
    }
    
    /// Read a SPLINE entity
    fn read_spline(&mut self) -> Result<CadTemplate> {
        let mut data = self.read_entity_data()?;
        
        // Scenario BL: 1 = ctrl pts/knots, 2 = fit pts only
        let scenario = self.reader.read_bitlong()?;
        
        // R2013+: spline flags1 + knot parametrization (BL + BL)
        let _spline_flags = if self.version >= ACadVersion::AC1027 {
            let flags1 = self.reader.read_bitlong()?;
            let _knot_param = self.reader.read_bitlong()?;
            flags1
        } else {
            0
        };
        
        // Degree BL
        let degree = self.reader.read_bitlong()? as i16;
        
        let mut num_fit_pts = 0usize;
        let mut num_knots = 0usize;
        let mut num_ctrl_pts = 0usize;
        let mut rational = false;
        let mut closed = false;
        let mut periodic = false;
        let mut weighted = false;
        let mut knot_tol = 0.0;
        let mut ctrl_tol = 0.0;
        let mut fit_tol = 0.0;
        let mut start_tangent: Option<Vector3> = None;
        let mut end_tangent: Option<Vector3> = None;
        
        match scenario {
            1 => {
                // Ctrl pts / knots path
                rational = self.reader.read_bit()?;
                closed = self.reader.read_bit()?;
                periodic = self.reader.read_bit()?;
                knot_tol = self.reader.read_bitdouble()?;
                ctrl_tol = self.reader.read_bitdouble()?;
                num_knots = self.reader.read_bitlong()? as usize;
                num_ctrl_pts = self.reader.read_bitlong()? as usize;
                weighted = self.reader.read_bit()?;
            }
            2 => {
                // Fit pts path
                fit_tol = self.reader.read_bitdouble()?;
                start_tangent = Some(self.reader.read_3bitdouble()?);
                end_tangent = Some(self.reader.read_3bitdouble()?);
                num_fit_pts = self.reader.read_bitlong()? as usize;
            }
            _ => {
                // Unknown scenario, try to read as scenario 1
                rational = self.reader.read_bit()?;
                closed = self.reader.read_bit()?;
                periodic = self.reader.read_bit()?;
                knot_tol = self.reader.read_bitdouble()?;
                ctrl_tol = self.reader.read_bitdouble()?;
                num_knots = self.reader.read_bitlong()? as usize;
                num_ctrl_pts = self.reader.read_bitlong()? as usize;
                weighted = self.reader.read_bit()?;
            }
        }
        
        let mut knots = Vec::with_capacity(num_knots);
        for _ in 0..num_knots {
            knots.push(self.reader.read_bitdouble()?);
        }
        
        let mut control_points = Vec::with_capacity(num_ctrl_pts);
        let mut weights = Vec::with_capacity(num_ctrl_pts);
        for _ in 0..num_ctrl_pts {
            control_points.push(self.reader.read_3bitdouble()?);
            if weighted {
                weights.push(self.reader.read_bitdouble()?);
            }
        }
        
        let mut fit_points = Vec::with_capacity(num_fit_pts);
        for _ in 0..num_fit_pts {
            fit_points.push(self.reader.read_3bitdouble()?);
        }
        
        self.read_entity_handles(&mut data)?;
        self.queue_entity_handles(&data);
        
        let flags = if closed { 1 } else { 0 }
            | if periodic { 2 } else { 0 }
            | if rational { 4 } else { 0 };
        
        Ok(CadTemplate::Spline {
            entity_data: data,
            scenario: scenario as i16,
            degree,
            flags,
            closed,
            periodic,
            rational,
            knot_tolerance: knot_tol,
            control_tolerance: ctrl_tol,
            fit_tolerance: fit_tol,
            start_tangent,
            end_tangent,
            knots,
            weights,
            control_points,
            fit_points,
        })
    }
    
    /// Read an INSERT entity
    fn read_insert(&mut self) -> Result<CadTemplate> {
        let mut data = self.read_entity_data()?;
        
        let insertion_point = self.reader.read_3bitdouble()?;
        
        // Scale values: R13-R14 vs R2000+
        let scale = if self.version >= ACadVersion::AC1015 {
            let scale_flag = self.reader.read_2bits()?;
            match scale_flag {
                // 00: XScale as RD, YScale as DD(default=XScale), ZScale as DD(default=XScale)
                0b00 => {
                    let x = self.reader.read_raw_double()?;
                    let y = self.reader.read_bitdouble_with_default(x)?;
                    let z = self.reader.read_bitdouble_with_default(x)?;
                    Vector3::new(x, y, z)
                },
                // 01: XScale=1.0, YScale as DD(default=1.0), ZScale as DD(default=1.0)
                0b01 => {
                    let y = self.reader.read_bitdouble_with_default(1.0)?;
                    let z = self.reader.read_bitdouble_with_default(1.0)?;
                    Vector3::new(1.0, y, z)
                },
                // 10: single RD, all three equal
                0b10 => {
                    let xyz = self.reader.read_raw_double()?;
                    Vector3::new(xyz, xyz, xyz)
                },
                // 11: all 1.0
                _ => Vector3::new(1.0, 1.0, 1.0),
            }
        } else {
            Vector3::new(
                self.reader.read_bitdouble()?,
                self.reader.read_bitdouble()?,
                self.reader.read_bitdouble()?,
            )
        };
        
        let rotation = self.reader.read_bitdouble()?;
        // Extrusion is 3BD (not BE) for Insert 
        let extrusion = self.reader.read_3bitdouble()?;
        let has_attribs = self.reader.read_bit()?;
        
        // R2004+: Object count
        let owned_obj_count = if self.version >= ACadVersion::AC1018 && has_attribs {
            self.reader.read_bitlong()? as u32
        } else {
            0
        };
        
        self.read_entity_handles(&mut data)?;
        
        let block_header_handle = Some(self.reader.read_handle_reference(data.handle)?);
        
        // R13-R2000: First and last attrib
        let (first_attrib_handle, last_attrib_handle, seqend_handle) = if has_attribs && self.version < ACadVersion::AC1018 {
            (
                Some(self.reader.read_handle_reference(data.handle)?),
                Some(self.reader.read_handle_reference(data.handle)?),
                Some(self.reader.read_handle_reference(data.handle)?),
            )
        } else {
            (None, None, None)
        };
        
        // R2004+: Attrib handles
        let mut attrib_handles = Vec::new();
        if self.version >= ACadVersion::AC1018 && has_attribs {
            for _ in 0..owned_obj_count {
                attrib_handles.push(self.reader.read_handle_reference(data.handle)?);
            }
        }
        
        self.queue_entity_handles(&data);
        
        Ok(CadTemplate::Insert {
            entity_data: data,
            insertion_point,
            scale,
            rotation,
            extrusion,
            has_attribs,
            owned_obj_count,
            block_header_handle,
            first_attrib_handle,
            last_attrib_handle,
            seqend_handle,
            attrib_handles,
        })
    }
    
    /// Read a 2D POLYLINE entity
    fn read_polyline2d(&mut self) -> Result<CadTemplate> {
        let mut data = self.read_entity_data()?;
        
        let flags = self.reader.read_bitshort()?;
        let curve_type = self.reader.read_bitshort()?;
        let start_width = self.reader.read_bitdouble()?;
        let end_width = self.reader.read_bitdouble()?;
        let thickness = self.reader.read_bit_thickness()?;
        let elevation = self.reader.read_bitdouble()?;
        let extrusion = self.reader.read_bit_extrusion()?;
        
        // R2004+: Owned object count
        let owned_obj_count = if self.version >= ACadVersion::AC1018 {
            self.reader.read_bitlong()? as u32
        } else {
            0
        };
        
        self.read_entity_handles(&mut data)?;
        
        // R13-R2000: First and last vertex, seqend
        let (first_vertex_handle, last_vertex_handle, seqend_handle) = if self.version < ACadVersion::AC1018 {
            (
                Some(self.reader.read_handle_reference(data.handle)?),
                Some(self.reader.read_handle_reference(data.handle)?),
                Some(self.reader.read_handle_reference(data.handle)?),
            )
        } else {
            (None, None, None)
        };
        
        // R2004+: Vertex handles
        let mut vertex_handles = Vec::new();
        if self.version >= ACadVersion::AC1018 {
            for _ in 0..owned_obj_count {
                vertex_handles.push(self.reader.read_handle_reference(data.handle)?);
            }
        }
        
        self.queue_entity_handles(&data);
        
        Ok(CadTemplate::Polyline2D {
            entity_data: data,
            flags,
            curve_type,
            start_width,
            end_width,
            thickness,
            elevation,
            extrusion,
            owned_obj_count,
            first_vertex_handle,
            last_vertex_handle,
            seqend_handle,
            vertex_handles,
        })
    }
    
    /// Read a 3D POLYLINE entity
    fn read_polyline3d(&mut self) -> Result<CadTemplate> {
        let mut data = self.read_entity_data()?;
        
        let flags = self.reader.read_raw_char()?;
        let curve_type = self.reader.read_raw_char()?;
        
        // R2004+: Owned object count
        let owned_obj_count = if self.version >= ACadVersion::AC1018 {
            self.reader.read_bitlong()? as u32
        } else {
            0
        };
        
        self.read_entity_handles(&mut data)?;
        
        // R13-R2000: First and last vertex, seqend
        let (first_vertex_handle, last_vertex_handle, seqend_handle) = if self.version < ACadVersion::AC1018 {
            (
                Some(self.reader.read_handle_reference(data.handle)?),
                Some(self.reader.read_handle_reference(data.handle)?),
                Some(self.reader.read_handle_reference(data.handle)?),
            )
        } else {
            (None, None, None)
        };
        
        // R2004+: Vertex handles
        let mut vertex_handles = Vec::new();
        if self.version >= ACadVersion::AC1018 {
            for _ in 0..owned_obj_count {
                vertex_handles.push(self.reader.read_handle_reference(data.handle)?);
            }
        }
        
        self.queue_entity_handles(&data);
        
        Ok(CadTemplate::Polyline3D {
            entity_data: data,
            flags,
            curve_type,
            owned_obj_count,
            first_vertex_handle,
            last_vertex_handle,
            seqend_handle,
            vertex_handles,
        })
    }
    
    /// Read a VERTEX_2D entity
    fn read_vertex2d(&mut self) -> Result<CadTemplate> {
        let mut data = self.read_entity_data()?;
        
        let flags = self.reader.read_raw_char()?;
        let point = self.reader.read_3bitdouble()?;
        let start_width = self.reader.read_bitdouble()?;
        
        let end_width = if start_width >= 0.0 {
            self.reader.read_bitdouble()?
        } else {
            start_width.abs()
        };
        
        let bulge = self.reader.read_bitdouble()?;
        
        // R2010+: vertex id
        let _vertex_id = if self.version >= ACadVersion::AC1024 {
            self.reader.read_bitlong()?
        } else {
            0
        };
        
        let tangent_dir = self.reader.read_bitdouble()?;
        
        self.read_entity_handles(&mut data)?;
        self.queue_entity_handles(&data);
        
        Ok(CadTemplate::Vertex2D {
            entity_data: data,
            flags,
            point,
            start_width: start_width.abs(),
            end_width,
            bulge,
            tangent_dir,
        })
    }
    
    /// Read a VERTEX_3D entity
    fn read_vertex3d(&mut self) -> Result<CadTemplate> {
        let mut data = self.read_entity_data()?;
        
        let flags = self.reader.read_raw_char()?;
        let point = self.reader.read_3bitdouble()?;
        
        self.read_entity_handles(&mut data)?;
        self.queue_entity_handles(&data);
        
        Ok(CadTemplate::Vertex3D {
            entity_data: data,
            flags,
            point,
        })
    }
    
    /// Read a BLOCK entity
    fn read_block(&mut self) -> Result<CadTemplate> {
        let mut data = self.read_entity_data()?;
        let name = self.reader.read_variable_text(self.version)?;
        
        self.read_entity_handles(&mut data)?;
        self.queue_entity_handles(&data);
        
        Ok(CadTemplate::Block {
            entity_data: data,
            name,
        })
    }
    
    /// Read a ENDBLK entity
    fn read_block_end(&mut self) -> Result<CadTemplate> {
        let mut data = self.read_entity_data()?;
        self.read_entity_handles(&mut data)?;
        self.queue_entity_handles(&data);
        
        Ok(CadTemplate::BlockEnd { entity_data: data })
    }
    
    /// Read a SEQEND entity
    fn read_seqend(&mut self) -> Result<CadTemplate> {
        let mut data = self.read_entity_data()?;
        self.read_entity_handles(&mut data)?;
        self.queue_entity_handles(&data);
        
        Ok(CadTemplate::Seqend { entity_data: data })
    }
    
    /// Read a SOLID entity (2D fill)
    fn read_solid(&mut self) -> Result<CadTemplate> {
        let mut data = self.read_entity_data()?;
        
        let thickness = self.reader.read_bit_thickness()?;
        let elevation = self.reader.read_bitdouble()?;
        
        // Corners are 2RD (two Raw Doubles), not 2BD
        let corner1 = self.reader.read_2raw_double()?;
        let corner2 = self.reader.read_2raw_double()?;
        let corner3 = self.reader.read_2raw_double()?;
        let corner4 = self.reader.read_2raw_double()?;
        
        let extrusion = self.reader.read_bit_extrusion()?;
        
        self.read_entity_handles(&mut data)?;
        self.queue_entity_handles(&data);
        
        Ok(CadTemplate::Solid {
            entity_data: data,
            thickness,
            elevation,
            extrusion,
            corner1,
            corner2,
            corner3,
            corner4,
        })
    }
    
    /// Read a TRACE entity
    fn read_trace(&mut self) -> Result<CadTemplate> {
        let mut data = self.read_entity_data()?;
        
        let thickness = self.reader.read_bit_thickness()?;
        let elevation = self.reader.read_bitdouble()?;
        
        // Corners are 2RD (two Raw Doubles), not 2BD
        let corner1 = self.reader.read_2raw_double()?;
        let corner2 = self.reader.read_2raw_double()?;
        let corner3 = self.reader.read_2raw_double()?;
        let corner4 = self.reader.read_2raw_double()?;
        
        let extrusion = self.reader.read_bit_extrusion()?;
        
        self.read_entity_handles(&mut data)?;
        self.queue_entity_handles(&data);
        
        Ok(CadTemplate::Trace {
            entity_data: data,
            thickness,
            elevation,
            extrusion,
            corner1,
            corner2,
            corner3,
            corner4,
        })
    }
    
    /// Read a 3DFACE entity
    fn read_face3d(&mut self) -> Result<CadTemplate> {
        let mut data = self.read_entity_data()?;
        
        // R13-R14: four 3BD corners + BS flags
        // R2000+: B(no_flags) + B(z_is_zero) + RD+RD+[RD] first corner + 3DD×3 + optional BS flags
        
        if self.version <= ACadVersion::AC1014 {
            // R13-R14
            let c0 = self.reader.read_3bitdouble()?;
            let c1 = self.reader.read_3bitdouble()?;
            let c2 = self.reader.read_3bitdouble()?;
            let c3 = self.reader.read_3bitdouble()?;
            let invisible_edge = self.reader.read_bitshort()? as u16;
            
            self.read_entity_handles(&mut data)?;
            self.queue_entity_handles(&data);
            
            Ok(CadTemplate::Face3D {
                entity_data: data,
                has_no_flags: false,
                z_is_zero: false,
                corners: [c0, c1, c2, c3],
                invisible_edge,
            })
        } else {
            // R2000+
            let has_no_flags = self.reader.read_bit()?;
            let z_is_zero = self.reader.read_bit()?;
            
            // 1st corner: RD x, RD y, conditional RD z
            let x = self.reader.read_raw_double()?;
            let y = self.reader.read_raw_double()?;
            let z = if !z_is_zero {
                self.reader.read_raw_double()?
            } else {
                0.0
            };
            let c0 = Vector3::new(x, y, z);
            
            // 2nd-4th corners: 3DD with previous as default
            let c1 = self.reader.read_3bitdouble_with_default(c0)?;
            let c2 = self.reader.read_3bitdouble_with_default(c1)?;
            let c3 = self.reader.read_3bitdouble_with_default(c2)?;
            
            let invisible_edge = if !has_no_flags {
                self.reader.read_bitshort()? as u16
            } else {
                0
            };
            
            self.read_entity_handles(&mut data)?;
            self.queue_entity_handles(&data);
            
            Ok(CadTemplate::Face3D {
                entity_data: data,
                has_no_flags,
                z_is_zero,
                corners: [c0, c1, c2, c3],
                invisible_edge,
            })
        }
    }
    
    /// Read a VIEWPORT entity
    fn read_viewport(&mut self) -> Result<CadTemplate> {
        let mut data = self.read_entity_data()?;
        
        let center = self.reader.read_3bitdouble()?;
        let width = self.reader.read_bitdouble()?;
        let height = self.reader.read_bitdouble()?;
        
        // R2000+: Full viewport data
        let view_target = if self.version >= ACadVersion::AC1015 {
            self.reader.read_3bitdouble()?
        } else {
            Vector3::new(0.0, 0.0, 0.0)
        };
        
        let view_direction = if self.version >= ACadVersion::AC1015 {
            self.reader.read_3bitdouble()?
        } else {
            Vector3::new(0.0, 0.0, 1.0)
        };
        
        let view_twist_angle = if self.version >= ACadVersion::AC1015 {
            self.reader.read_bitdouble()?
        } else {
            0.0
        };
        
        let view_height = if self.version >= ACadVersion::AC1015 {
            self.reader.read_bitdouble()?
        } else {
            height
        };
        
        let lens_length = if self.version >= ACadVersion::AC1015 {
            self.reader.read_bitdouble()?
        } else {
            50.0
        };
        
        let front_clip = if self.version >= ACadVersion::AC1015 {
            self.reader.read_bitdouble()?
        } else {
            0.0
        };
        
        let back_clip = if self.version >= ACadVersion::AC1015 {
            self.reader.read_bitdouble()?
        } else {
            0.0
        };
        
        let snap_angle = if self.version >= ACadVersion::AC1015 {
            self.reader.read_bitdouble()?
        } else {
            0.0
        };
        
        let view_center = if self.version >= ACadVersion::AC1015 {
            self.reader.read_2raw_double()?
        } else {
            Vector2::new(0.0, 0.0)
        };
        
        let snap_base = if self.version >= ACadVersion::AC1015 {
            self.reader.read_2raw_double()?
        } else {
            Vector2::new(0.0, 0.0)
        };
        
        let snap_spacing = if self.version >= ACadVersion::AC1015 {
            self.reader.read_2raw_double()?
        } else {
            Vector2::new(1.0, 1.0)
        };
        
        let grid_spacing = if self.version >= ACadVersion::AC1015 {
            self.reader.read_2raw_double()?
        } else {
            Vector2::new(1.0, 1.0)
        };
        
        let circle_sides = if self.version >= ACadVersion::AC1015 {
            self.reader.read_bitshort()? as u16
        } else {
            1000
        };
        
        // Skip frozen layer count for now
        let frozen_layer_count = if self.version >= ACadVersion::AC1015 {
            self.reader.read_bitlong()? as usize
        } else {
            0
        };
        
        self.read_entity_handles(&mut data)?;
        
        // Frozen layer handles
        let mut frozen_layer_handles = Vec::with_capacity(frozen_layer_count);
        for _ in 0..frozen_layer_count {
            frozen_layer_handles.push(self.reader.read_handle_reference(data.handle)?);
        }
        
        self.queue_entity_handles(&data);
        
        Ok(CadTemplate::Viewport {
            entity_data: data,
            center,
            width,
            height,
            view_target,
            view_direction,
            view_twist_angle,
            view_height,
            lens_length,
            front_clip,
            back_clip,
            snap_angle,
            view_center,
            snap_base,
            snap_spacing,
            grid_spacing,
            circle_sides,
            frozen_layer_handles,
        })
    }
    
    /// Read a RAY entity
    fn read_ray(&mut self) -> Result<CadTemplate> {
        let mut data = self.read_entity_data()?;
        
        let point = self.reader.read_3bitdouble()?;
        let vector = self.reader.read_3bitdouble()?;
        
        self.read_entity_handles(&mut data)?;
        self.queue_entity_handles(&data);
        
        Ok(CadTemplate::Ray {
            entity_data: data,
            point,
            vector,
        })
    }
    
    /// Read an XLINE entity
    fn read_xline(&mut self) -> Result<CadTemplate> {
        let mut data = self.read_entity_data()?;
        
        let point = self.reader.read_3bitdouble()?;
        let vector = self.reader.read_3bitdouble()?;
        
        self.read_entity_handles(&mut data)?;
        self.queue_entity_handles(&data);
        
        Ok(CadTemplate::XLine {
            entity_data: data,
            point,
            vector,
        })
    }
    
    /// Read common text data (shared by TEXT, ATTRIB, ATTDEF)
    fn read_text_data(&mut self) -> Result<TextTemplateData> {
        let mut elevation = 0.0;
        let insertion;
        let mut alignment;
        let extrusion;
        let thickness;
        let mut oblique_angle = 0.0;
        let mut rotation = 0.0;
        let height;
        let mut width_factor = 1.0;
        let value;
        let mut generation_flags: i16 = 0;
        let mut horizontal_alignment: i16 = 0;
        let mut vertical_alignment: i16 = 0;
        
        if self.version <= ACadVersion::AC1014 {
            // R13-R14
            elevation = self.reader.read_bitdouble()?;
            let pt = self.reader.read_2raw_double()?;
            insertion = Vector3::new(pt.x, pt.y, elevation);
            let apt = self.reader.read_2raw_double()?;
            alignment = Vector3::new(apt.x, apt.y, elevation);
            extrusion = self.reader.read_3bitdouble()?;
            thickness = self.reader.read_bitdouble()?;
            oblique_angle = self.reader.read_bitdouble()?;
            rotation = self.reader.read_bitdouble()?;
            height = self.reader.read_bitdouble()?;
            width_factor = self.reader.read_bitdouble()?;
            value = self.reader.read_variable_text(self.version)?;
            generation_flags = self.reader.read_bitshort()?;
            horizontal_alignment = self.reader.read_bitshort()?;
            vertical_alignment = self.reader.read_bitshort()?;
        } else {
            // R2000+: Compact format
            let data_flags = self.reader.read_raw_char()?;  // RC
            
            if (data_flags & 0x01) == 0 {
                elevation = self.reader.read_raw_double()?;
            }
            
            let pt = self.reader.read_2raw_double()?;
            insertion = Vector3::new(pt.x, pt.y, elevation);
            
            if (data_flags & 0x02) == 0 {
                let ax = self.reader.read_bitdouble_with_default(insertion.x)?;
                let ay = self.reader.read_bitdouble_with_default(insertion.y)?;
                alignment = Vector3::new(ax, ay, elevation);
            } else {
                alignment = insertion;
            }
            
            extrusion = self.reader.read_bit_extrusion()?;
            thickness = self.reader.read_bit_thickness()?;
            
            if (data_flags & 0x04) == 0 {
                oblique_angle = self.reader.read_raw_double()?;
            }
            if (data_flags & 0x08) == 0 {
                rotation = self.reader.read_raw_double()?;
            }
            height = self.reader.read_raw_double()?;
            if (data_flags & 0x10) == 0 {
                width_factor = self.reader.read_raw_double()?;
            }
            
            value = self.reader.read_variable_text(self.version)?;
            
            if (data_flags & 0x20) == 0 {
                generation_flags = self.reader.read_bitshort()?;
            }
            if (data_flags & 0x40) == 0 {
                horizontal_alignment = self.reader.read_bitshort()?;
            }
            if (data_flags as u8 & 0x80) == 0 {
                vertical_alignment = self.reader.read_bitshort()?;
            }
        }
        
        Ok(TextTemplateData {
            insertion,
            alignment,
            extrusion,
            thickness,
            oblique_angle,
            rotation,
            height,
            width_factor,
            value,
            generation_flags,
            horizontal_alignment,
            vertical_alignment,
            style_handle: None,
        })
    }
    
    /// Read an ATTRIB entity
    fn read_attrib(&mut self) -> Result<CadTemplate> {
        let mut data = self.read_entity_data()?;
        let mut text_data = self.read_text_data()?;
        
        // R2010+: version
        let version = if self.version >= ACadVersion::AC1024 {
            self.reader.read_raw_char()?
        } else {
            0
        };
        
        let tag = self.reader.read_variable_text(self.version)?;
        let field_length = self.reader.read_bitshort()? as u8;
        let flags = self.reader.read_raw_char()?;
        
        // R2007+: Lock position flag
        let lock_position = if self.version >= ACadVersion::AC1021 {
            self.reader.read_bit()?
        } else {
            false
        };
        
        self.read_entity_handles(&mut data)?;
        text_data.style_handle = Some(self.reader.read_handle_reference(data.handle)?);
        
        self.queue_entity_handles(&data);
        
        Ok(CadTemplate::Attrib {
            entity_data: data,
            text_data,
            version,
            tag,
            flags,
            field_length,
            lock_position,
        })
    }
    
    /// Read an ATTDEF entity
    fn read_attdef(&mut self) -> Result<CadTemplate> {
        let mut data = self.read_entity_data()?;
        let mut text_data = self.read_text_data()?;
        
        // R2010+: version
        let version = if self.version >= ACadVersion::AC1024 {
            self.reader.read_raw_char()?
        } else {
            0
        };
        
        let prompt = self.reader.read_variable_text(self.version)?;
        let tag = self.reader.read_variable_text(self.version)?;
        let field_length = self.reader.read_bitshort()? as u8;
        let flags = self.reader.read_raw_char()?;
        
        // R2007+: Lock position flag
        let lock_position = if self.version >= ACadVersion::AC1021 {
            self.reader.read_bit()?
        } else {
            false
        };
        
        self.read_entity_handles(&mut data)?;
        text_data.style_handle = Some(self.reader.read_handle_reference(data.handle)?);
        
        self.queue_entity_handles(&data);
        
        Ok(CadTemplate::AttDef {
            entity_data: data,
            text_data,
            version,
            prompt,
            tag,
            flags,
            field_length,
            lock_position,
        })
    }
    
    /// Read common dimension data
    fn read_dim_common(&mut self) -> Result<DimCommonData> {
        // R2010+ only: version byte
        let version = if self.version >= ACadVersion::AC1024 {
            self.reader.read_raw_char()?
        } else {
            0
        };
        
        // Extrusion is 3BD, not BE
        let extrusion = self.reader.read_3bitdouble()?;
        // Text midpoint is 2RD, not 2BD
        let text_midpoint = self.reader.read_2raw_double()?;
        let elevation = self.reader.read_bitdouble()?;
        let flags = self.reader.read_raw_char()?;
        let user_text = self.reader.read_variable_text(self.version)?;
        let text_rotation = self.reader.read_bitdouble()?;
        let horiz_dir = self.reader.read_bitdouble()?;
        
        let ins_scale = self.reader.read_3bitdouble()?;
        let ins_rotation = self.reader.read_bitdouble()?;
        
        // R2000+
        let attachment_point = if self.version >= ACadVersion::AC1015 {
            self.reader.read_bitshort()? as u8
        } else {
            0
        };
        
        let linespacing_style = if self.version >= ACadVersion::AC1015 {
            self.reader.read_bitshort()? as u8
        } else {
            0
        };
        
        let linespacing_factor = if self.version >= ACadVersion::AC1015 {
            self.reader.read_bitdouble()?
        } else {
            1.0
        };
        
        let actual_measurement = if self.version >= ACadVersion::AC1015 {
            self.reader.read_bitdouble()?
        } else {
            0.0
        };
        
        // R2007+: Unknown + flip arrow bits
        if self.version >= ACadVersion::AC1021 {
            let _ = self.reader.read_bit()?; // unknown
            let _ = self.reader.read_bit()?; // flip arrow1
            let _ = self.reader.read_bit()?; // flip arrow2
        }
        
        // Clone insertion point (12-pt) is 2RD, not 2BD
        let clone_ins_pt = self.reader.read_2raw_double()?;
        
        Ok(DimCommonData {
            version,
            extrusion,
            text_midpoint,
            elevation,
            flags,
            user_text,
            text_rotation,
            horiz_dir,
            ins_scale,
            ins_rotation,
            attachment_point,
            linespacing_style,
            linespacing_factor,
            actual_measurement,
            clone_ins_pt,
            dimstyle_handle: None,
            block_handle: None,
        })
    }
    
    /// Read DIMENSION handles (common to all dimension types)
    fn read_dim_handles(&mut self, data: &mut DwgEntityData, dim: &mut DimCommonData) -> Result<()> {
        self.read_entity_handles(data)?;
        dim.dimstyle_handle = Some(self.reader.read_handle_reference(data.handle)?);
        dim.block_handle = Some(self.reader.read_handle_reference(data.handle)?);
        Ok(())
    }
    
    /// Read DIMENSION_ORDINATE
    fn read_dim_ordinate(&mut self) -> Result<CadTemplate> {
        let mut data = self.read_entity_data()?;
        let mut dim_common = self.read_dim_common()?;
        
        let def_point = self.reader.read_3bitdouble()?;
        let feature_pt = self.reader.read_3bitdouble()?;
        let leader_pt = self.reader.read_3bitdouble()?;
        let ordinate_type = self.reader.read_raw_char()?;
        
        self.read_dim_handles(&mut data, &mut dim_common)?;
        self.queue_entity_handles(&data);
        
        Ok(CadTemplate::DimOrdinate {
            entity_data: data,
            dim_common,
            def_point,
            feature_pt,
            leader_pt,
            ordinate_type,
        })
    }
    
    /// Read DIMENSION_LINEAR
    fn read_dim_linear(&mut self) -> Result<CadTemplate> {
        let mut data = self.read_entity_data()?;
        let mut dim_common = self.read_dim_common()?;
        
        // readCommonDimensionAlignedData: xline1(13), xline2(14), def_pt(10), ext_line_rotation(52)
        let xline1_pt = self.reader.read_3bitdouble()?;
        let xline2_pt = self.reader.read_3bitdouble()?;
        let def_point = self.reader.read_3bitdouble()?;
        let oblique_angle = self.reader.read_bitdouble()?;
        // Then rotation(50) for DimLinear
        let rotation = self.reader.read_bitdouble()?;
        
        self.read_dim_handles(&mut data, &mut dim_common)?;
        self.queue_entity_handles(&data);
        
        Ok(CadTemplate::DimLinear {
            entity_data: data,
            dim_common,
            def_point,
            xline1_pt,
            xline2_pt,
            rotation,
            oblique_angle,
        })
    }
    
    /// Read DIMENSION_ALIGNED
    fn read_dim_aligned(&mut self) -> Result<CadTemplate> {
        let mut data = self.read_entity_data()?;
        let mut dim_common = self.read_dim_common()?;
        
        // readCommonDimensionAlignedData: xline1(13), xline2(14), def_pt(10), ext_line_rotation(52)
        let xline1_pt = self.reader.read_3bitdouble()?;
        let xline2_pt = self.reader.read_3bitdouble()?;
        let def_point = self.reader.read_3bitdouble()?;
        let _ext_line_rotation = self.reader.read_bitdouble()?;
        
        self.read_dim_handles(&mut data, &mut dim_common)?;
        self.queue_entity_handles(&data);
        
        Ok(CadTemplate::DimAligned {
            entity_data: data,
            dim_common,
            def_point,
            xline1_pt,
            xline2_pt,
        })
    }
    
    /// Read DIMENSION_ANG3PT
    fn read_dim_angular3pt(&mut self) -> Result<CadTemplate> {
        let mut data = self.read_entity_data()?;
        let mut dim_common = self.read_dim_common()?;
        
        let def_point = self.reader.read_3bitdouble()?;
        let xline1_pt = self.reader.read_3bitdouble()?;
        let xline2_pt = self.reader.read_3bitdouble()?;
        let center_pt = self.reader.read_3bitdouble()?;
        
        self.read_dim_handles(&mut data, &mut dim_common)?;
        self.queue_entity_handles(&data);
        
        Ok(CadTemplate::DimAngular3Pt {
            entity_data: data,
            dim_common,
            def_point,
            xline1_pt,
            xline2_pt,
            center_pt,
        })
    }
    
    /// Read DIMENSION_RADIUS
    fn read_dim_radius(&mut self) -> Result<CadTemplate> {
        let mut data = self.read_entity_data()?;
        let mut dim_common = self.read_dim_common()?;
        
        let def_point = self.reader.read_3bitdouble()?;
        let _angle_vertex = self.reader.read_3bitdouble()?;
        let leader_len = self.reader.read_bitdouble()?;
        
        self.read_dim_handles(&mut data, &mut dim_common)?;
        self.queue_entity_handles(&data);
        
        Ok(CadTemplate::DimRadius {
            entity_data: data,
            dim_common,
            def_point,
            leader_len,
        })
    }
    
    /// Read DIMENSION_DIAMETER
    fn read_dim_diameter(&mut self) -> Result<CadTemplate> {
        let mut data = self.read_entity_data()?;
        let mut dim_common = self.read_dim_common()?;
        
        let def_point = self.reader.read_3bitdouble()?;
        let _angle_vertex = self.reader.read_3bitdouble()?;
        let leader_len = self.reader.read_bitdouble()?;
        
        self.read_dim_handles(&mut data, &mut dim_common)?;
        self.queue_entity_handles(&data);
        
        Ok(CadTemplate::DimDiameter {
            entity_data: data,
            dim_common,
            def_point,
            leader_len,
        })
    }
    
    /// Read a HATCH entity
    fn read_hatch(&mut self) -> Result<CadTemplate> {
        let mut data = self.read_entity_data()?;
        
        // R2004+: gradient data
        let is_gradient = if self.version >= ACadVersion::AC1018 {
            self.reader.read_bitlong()? != 0
        } else {
            false
        };
        
        if is_gradient {
            let _ = self.reader.read_bitlong()?;   // reserved
            let _ = self.reader.read_bitdouble()?;  // gradient angle
            let _ = self.reader.read_bitdouble()?;  // gradient shift
            let _ = self.reader.read_bitlong()?;     // single color
            let _ = self.reader.read_bitdouble()?;  // gradient tint
            let num_colors = self.reader.read_bitlong()?.min(1000) as usize;
            for _ in 0..num_colors {
                let _ = self.reader.read_bitdouble()?;  // value
                // CmColor for R2004+: BS(index) + BL(RGB) + RC(flags) + conditional TV strings
                let _ = self.reader.read_bitshort()?;   // color index (always 0)
                let _ = self.reader.read_bitlong()?;    // RGB value
                let flag_byte = self.reader.read_raw_char()?;  // flag byte
                if (flag_byte & 1) != 0 {
                    let _ = self.reader.read_variable_text(self.version)?;  // color name
                }
                if (flag_byte & 2) != 0 {
                    let _ = self.reader.read_variable_text(self.version)?;  // book name
                }
            }
            let _ = self.reader.read_variable_text(self.version)?; // gradient name
        }
        
        // Common hatch data
        let elevation = self.reader.read_bitdouble()?;
        let extrusion = self.reader.read_3bitdouble()?;
        let pattern_name = self.reader.read_variable_text(self.version)?;
        let is_solid_fill = self.reader.read_bit()?;
        let is_associative = self.reader.read_bit()?;
        
        let num_paths = self.reader.read_bitlong()? as usize;
        if num_paths > 10000 {
            return Err(DxfError::Parse(format!("Hatch: unreasonable num_paths {}", num_paths)));
        }
        let mut boundary_paths = Vec::with_capacity(num_paths);
        let mut has_derived_boundary = false;
        
        for _ in 0..num_paths {
            let path_flag = self.reader.read_bitlong()? as u32;
            let mut path = HatchBoundaryPath {
                flags: path_flag,
                ..Default::default()
            };
            
            if (path_flag & 4) != 0 {
                has_derived_boundary = true;
            }
            
            let is_polyline = (path_flag & 2) != 0;
            
            if is_polyline {
                // Polyline path
                path.polyline_has_bulge = self.reader.read_bit()?;
                path.polyline_closed = self.reader.read_bit()?;
                let num_verts = self.reader.read_bitlong()? as usize;
                if num_verts > 100000 {
                    return Err(DxfError::Parse(format!("Hatch: unreasonable vertex count {}", num_verts)));
                }
                
                for _ in 0..num_verts {
                    let pt = self.reader.read_2raw_double()?;  // 2RD
                    let bulge = if path.polyline_has_bulge {
                        self.reader.read_bitdouble()?
                    } else {
                        0.0
                    };
                    path.polyline_vertices.push((pt, bulge));
                }
            } else {
                // Non-polyline edges
                let num_edges = self.reader.read_bitlong()? as usize;
                if num_edges > 100000 {
                    return Err(DxfError::Parse(format!("Hatch: unreasonable edge count {}", num_edges)));
                }
                for _ in 0..num_edges {
                    let edge_type = self.reader.read_raw_char()?;
                    match edge_type {
                        1 => {
                            // Line: 2RD start, 2RD end
                            let start = self.reader.read_2raw_double()?;
                            let end = self.reader.read_2raw_double()?;
                            path.edges.push(HatchEdge::Line { start, end });
                        }
                        2 => {
                            // Circular arc: 2RD center, BD radius, BD start, BD end, B ccw
                            let center = self.reader.read_2raw_double()?;
                            let radius = self.reader.read_bitdouble()?;
                            let start_angle = self.reader.read_bitdouble()?;
                            let end_angle = self.reader.read_bitdouble()?;
                            let is_ccw = self.reader.read_bit()?;
                            path.edges.push(HatchEdge::CircularArc {
                                center, radius, start_angle, end_angle, is_ccw,
                            });
                        }
                        3 => {
                            // Elliptic arc: 2RD center, 2RD major, BD ratio, BD start, BD end, B ccw
                            let center = self.reader.read_2raw_double()?;
                            let major_axis = self.reader.read_2raw_double()?;
                            let minor_ratio = self.reader.read_bitdouble()?;
                            let start_angle = self.reader.read_bitdouble()?;
                            let end_angle = self.reader.read_bitdouble()?;
                            let is_ccw = self.reader.read_bit()?;
                            path.edges.push(HatchEdge::EllipticArc {
                                center, major_axis, minor_ratio, start_angle, end_angle, is_ccw,
                            });
                        }
                        4 => {
                            // Spline edge
                            let degree = self.reader.read_bitlong()?;
                            let rational = self.reader.read_bit()?;
                            let periodic = self.reader.read_bit()?;
                            let num_knots = self.reader.read_bitlong()?.min(100000) as usize;
                            let num_ctrl = self.reader.read_bitlong()?.min(100000) as usize;
                            
                            let mut knots = Vec::with_capacity(num_knots);
                            for _ in 0..num_knots {
                                knots.push(self.reader.read_bitdouble()?);
                            }
                            
                            let mut control_points = Vec::with_capacity(num_ctrl);
                            let mut weights = Vec::new();
                            for _ in 0..num_ctrl {
                                control_points.push(self.reader.read_2raw_double()?);  // 2RD
                                if rational {
                                    weights.push(self.reader.read_bitdouble()?);
                                }
                            }
                            
                            // R2010+: Fit data
                            let fit_data = if self.version >= ACadVersion::AC1024 {
                                let num_fit = self.reader.read_bitlong()? as usize;
                                if num_fit > 0 {
                                    let mut fit_pts = Vec::with_capacity(num_fit);
                                    for _ in 0..num_fit {
                                        fit_pts.push(self.reader.read_2raw_double()?);  // 2RD
                                    }
                                    let start_tan = self.reader.read_2raw_double()?;
                                    let end_tan = self.reader.read_2raw_double()?;
                                    Some((start_tan, end_tan, fit_pts))
                                } else {
                                    None
                                }
                            } else {
                                None
                            };
                            
                            path.edges.push(HatchEdge::Spline {
                                degree, rational, periodic, knots,
                                control_points, weights, fit_data,
                            });
                        }
                        _ => {}
                    }
                }
            }
            
            // Boundary object handles count (BL) - actual handles are in handle stream
            let num_handles = self.reader.read_bitlong()?.min(10000) as usize;
            path.boundary_handles = Vec::with_capacity(num_handles);
            
            boundary_paths.push(path);
        }
        
        // After boundary paths: BS style, BS pattern_type
        let _style = self.reader.read_bitshort()?;
        let pattern_type = self.reader.read_bitshort()?;
        
        // Pattern definition (only if NOT solid fill)
        let mut pattern_angle = 0.0;
        let mut pattern_scale = 0.0;
        let mut pattern_double = false;
        let mut pattern_def_lines = Vec::new();
        
        if !is_solid_fill {
            pattern_angle = self.reader.read_bitdouble()?;
            pattern_scale = self.reader.read_bitdouble()?;
            pattern_double = self.reader.read_bit()?;
            
            let num_def_lines = self.reader.read_bitshort()? as usize;
            pattern_def_lines = Vec::with_capacity(num_def_lines);
            for _ in 0..num_def_lines {
                let angle = self.reader.read_bitdouble()?;
                let base_point = self.reader.read_2bitdouble()?;  // 2BD
                let offset = self.reader.read_2bitdouble()?;      // 2BD
                let num_dashes = self.reader.read_bitshort()? as usize;
                
                let mut dash_lengths = Vec::with_capacity(num_dashes);
                for _ in 0..num_dashes {
                    dash_lengths.push(self.reader.read_bitdouble()?);
                }
                
                pattern_def_lines.push(HatchPatternDefLine {
                    angle, base_point, offset, dash_lengths,
                });
            }
        }
        
        // Pixel size (only if any path was derived)
        if has_derived_boundary {
            let _pixel_size = self.reader.read_bitdouble()?;
        }
        
        // Seed points: BL count, then 2RD per seed
        let num_seed_points_raw = self.reader.read_bitlong()?;
        let num_seed_points = num_seed_points_raw.min(100000) as i16;
        let mut seed_points = Vec::with_capacity(num_seed_points.max(0) as usize);
        for _ in 0..num_seed_points.max(0) {
            seed_points.push(self.reader.read_2raw_double()?);  // 2RD
        }
        
        self.read_entity_handles(&mut data)?;
        self.queue_entity_handles(&data);
        
        Ok(CadTemplate::Hatch {
            entity_data: data,
            elevation,
            extrusion,
            pattern_name,
            is_solid_fill,
            is_associative,
            pattern_type,
            pattern_angle,
            pattern_scale,
            pattern_double,
            num_seed_points,
            seed_points,
            boundary_paths,
            pattern_def_lines,
        })
    }
    
    /// Read a TEXTSTYLE table entry
    fn read_textstyle(&mut self) -> Result<CadTemplate> {
        let mut data = DwgObjectData::default();
        
        // R2000-R2007: First read RL - size of object data in bits (before handles)
        if self.version >= ACadVersion::AC1015 && self.version < ACadVersion::AC1024 {
            let _obj_size_bits = self.reader.read_raw_long()? as u64;
        }
        
        data.handle = self.reader.read_handle()?;
        self.read_extended_data_obj(&mut data)?;
        
        data.num_reactors = self.reader.read_bitlong()? as u32;
        
        if self.version >= ACadVersion::AC1018 {
            data.xdic_missing = self.reader.read_bit()?;
        }
        
        let name = self.reader.read_variable_text(self.version)?;
        
        // readXrefDependantBit
        let xref_dep = self.read_xref_dependant_bit()?;
        
        let vertical = self.reader.read_bit()?;
        let shape_file = self.reader.read_bit()?;
        
        let fixed_height = self.reader.read_bitdouble()?;
        let width_factor = self.reader.read_bitdouble()?;
        let oblique_angle = self.reader.read_bitdouble()?;
        let generation_flags = self.reader.read_raw_char()?;
        let last_height = self.reader.read_bitdouble()?;
        let font_name = self.reader.read_variable_text(self.version)?;
        let big_font_name = self.reader.read_variable_text(self.version)?;
        
        // Handle references
        data.owner_handle = Some(self.reader.read_handle_reference(data.handle)?);
        
        for _ in 0..data.num_reactors {
            data.reactor_handles.push(self.reader.read_handle_reference(data.handle)?);
        }
        
        if !data.xdic_missing {
            data.xdic_handle = Some(self.reader.read_handle_reference(data.handle)?);
        }
        
        let flags = if vertical { 4 } else { 0 }
            | if shape_file { 1 } else { 0 }
            | if xref_dep { 16 } else { 0 };
        
        Ok(CadTemplate::TextStyle {
            object_data: data,
            name,
            flags,
            fixed_height,
            width_factor,
            oblique_angle,
            generation_flags,
            last_height,
            font_name,
            big_font_name,
        })
    }
    
    /// Read a DIMSTYLE table entry (simplified)
    fn read_dimstyle(&mut self) -> Result<CadTemplate> {
        let mut data = DwgObjectData::default();
        
        // R2000-R2007: First read RL - size of object data in bits (before handles)
        if self.version >= ACadVersion::AC1015 && self.version < ACadVersion::AC1024 {
            let _obj_size_bits = self.reader.read_raw_long()? as u64;
        }
        
        data.handle = self.reader.read_handle()?;
        self.read_extended_data_obj(&mut data)?;
        
        data.num_reactors = self.reader.read_bitlong()? as u32;
        
        if self.version >= ACadVersion::AC1018 {
            data.xdic_missing = self.reader.read_bit()?;
        }
        
        let name = self.reader.read_variable_text(self.version)?;
        
        // readXrefDependantBit
        let xref_dep = self.read_xref_dependant_bit()?;
        
        // For simplicity, read just the basic dimension style values
        // (Full implementation would read 70+ dimension variables)
        
        let flags: i16 = if xref_dep { 16 } else { 0 };
        
        // Skip detailed dimension variables and handles for now
        // This is a stub implementation
        
        Ok(CadTemplate::DimStyle {
            object_data: data,
            name,
            flags,
            dimpost: String::new(),
            dimapost: String::new(),
            dimscale: 1.0,
            dimasz: 0.18,
            dimexo: 0.0625,
            dimdli: 0.38,
            dimexe: 0.18,
            dimrnd: 0.0,
            dimdle: 0.0,
            dimtp: 0.0,
            dimtm: 0.0,
            dimtxt: 0.18,
            dimcen: 0.09,
            dimtsz: 0.0,
            dimaltf: 25.4,
            dimlfac: 1.0,
            dimtvp: 0.0,
            dimtfac: 1.0,
            dimgap: 0.09,
            dimtol: false,
            dimlim: false,
            dimtih: true,
            dimtoh: true,
            dimse1: false,
            dimse2: false,
            dimtad: 0,
            dimzin: 0,
            dimalt: false,
            dimaltd: 2,
            dimtofl: false,
            dimsah: false,
            dimtix: false,
            dimsoxd: false,
            dimclrd: 0,
            dimclre: 0,
            dimclrt: 0,
            dimadec: 0,
            dimdec: 4,
            dimtdec: 4,
            dimaltu: 2,
            dimalttd: 2,
            dimaunit: 0,
            dimfrac: 0,
            dimlunit: 2,
            dimdsep: b'.',
            dimtmove: 0,
            dimjust: 0,
            dimsd1: false,
            dimsd2: false,
            dimtolj: 1,
            dimtzin: 0,
            dimaltz: 0,
            dimalttz: 0,
            dimfit: 3,
            dimupt: false,
            dimatfit: 3,
            dimtxsty_handle: None,
            dimldrblk_handle: None,
            dimblk_handle: None,
            dimblk1_handle: None,
            dimblk2_handle: None,
            dimltype_handle: None,
            dimltex1_handle: None,
            dimltex2_handle: None,
        })
    }
    
    /// Read a control object and queue its entry handles
    /// Control objects (BLOCK_CONTROL, LAYER_CONTROL, etc.) contain handles to all entries
    fn read_control_object(&mut self) -> Result<()> {
        // R2000-R2007: First read RL - size of object data in bits (before handles)
        if self.version >= ACadVersion::AC1015 && self.version < ACadVersion::AC1024 {
            let _obj_size_bits = self.reader.read_raw_long()? as u64;
        }
        
        // Read object handle
        let handle = self.reader.read_handle()?;
        
        // Extended data (EED) - loop format
        {
            let mut eed_size = self.reader.read_bitshort()?;
            while eed_size != 0 {
                let _app_handle = self.reader.read_handle()?;
                let sz = eed_size as usize;
                if sz > 100000 {
                    return Err(DxfError::Parse(format!("EED size too large: {}", sz)));
                }
                let _ = self.reader.read_bytes(sz)?;
                eed_size = self.reader.read_bitshort()?;
            }
        }
        
        // Reactor count
        let num_reactors = self.reader.read_bitlong()? as usize;
        
        // R2004+: have xdic_missing flag
        if self.version >= ACadVersion::AC1018 {
            let _ = self.reader.read_bit()?;
        }
        
        // Number of entries in this control table
        let num_entries = self.reader.read_bitlong()? as usize;
        
        // Read null handle (parent or owner)
        let _ = self.reader.read_handle_reference(handle)?;
        
        // Read XDicObjHandle (optional)
        if num_reactors > 0 {
            for _ in 0..num_reactors {
                let _ = self.reader.read_handle_reference(handle)?;
            }
        }
        
        // Now read entry handles
        for _ in 0..num_entries {
            match self.reader.read_handle_reference(handle) {
                Ok(entry_handle) if entry_handle != 0 => {
                    self.handle_queue.push_back(entry_handle);
                }
                _ => {}
            }
        }
        
        Ok(())
    }
    
    /// Queue handles from entity data for reading
    fn queue_entity_handles(&mut self, data: &DwgEntityData) {
        if let Some(h) = data.owner_handle {
            self.handle_queue.push_back(h);
        }
        for h in &data.reactor_handles {
            self.handle_queue.push_back(*h);
        }
        if let Some(h) = data.xdic_handle {
            self.handle_queue.push_back(h);
        }
        if let Some(h) = data.layer_handle {
            self.handle_queue.push_back(h);
        }
        if let Some(h) = data.linetype_handle {
            self.handle_queue.push_back(h);
        }
    }
}
