//! DWG Classes Reader - Reads DXF class definitions from DWG files
//!
//! The classes section defines custom object types that may appear in the drawing.
//! This is essential for understanding how to read objects with class numbers
//! beyond the standard object types.

use std::io::{Read, Seek};
use crate::error::{DxfError, Result};
use crate::types::ACadVersion;
use super::stream_reader::{BitReader, DwgStreamReader};
use super::section::DwgSectionDefinition;

/// A DXF class definition
#[derive(Debug, Clone)]
pub struct DxfClass {
    /// Class number (matched to object type code)
    pub class_number: i16,
    /// Proxy flags
    pub proxy_flags: i16,
    /// DXF class name (e.g., "ACAD_PROXY_ENTITY")
    pub dxf_name: String,
    /// C++ class name (e.g., "AcDbProxyEntity")
    pub cpp_class_name: String,
    /// Application name (e.g., "AutoCAD")
    pub application_name: String,
    /// Was-a-proxy flag
    pub was_proxy: bool,
    /// Is entity flag
    pub is_entity: bool,
    /// Instance count in the drawing
    pub instance_count: i32,
    /// DWG version
    pub dwg_version: u32,
    /// Maintenance version
    pub maintenance_version: u32,
    /// Unknown values (version specific)
    pub unknown1: i32,
    pub unknown2: i32,
}

impl Default for DxfClass {
    fn default() -> Self {
        Self {
            class_number: 0,
            proxy_flags: 0,
            dxf_name: String::new(),
            cpp_class_name: String::new(),
            application_name: String::new(),
            was_proxy: false,
            is_entity: false,
            instance_count: 0,
            dwg_version: 0,
            maintenance_version: 0,
            unknown1: 0,
            unknown2: 0,
        }
    }
}

/// Collection of DXF classes
#[derive(Debug, Clone, Default)]
pub struct DxfClassCollection {
    classes: Vec<DxfClass>,
}

impl DxfClassCollection {
    /// Create a new empty collection
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Add a class to the collection
    pub fn add(&mut self, class: DxfClass) {
        self.classes.push(class);
    }
    
    /// Get a class by class number
    pub fn get_by_number(&self, class_number: i16) -> Option<&DxfClass> {
        self.classes.iter().find(|c| c.class_number == class_number)
    }
    
    /// Get a class by DXF name
    pub fn get_by_name(&self, name: &str) -> Option<&DxfClass> {
        self.classes.iter().find(|c| c.dxf_name == name)
    }
    
    /// Get all classes
    pub fn iter(&self) -> impl Iterator<Item = &DxfClass> {
        self.classes.iter()
    }
    
    /// Get the number of classes
    pub fn len(&self) -> usize {
        self.classes.len()
    }
    
    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.classes.is_empty()
    }
    
    /// Convert to a map by class number
    pub fn to_map(&self) -> std::collections::HashMap<i16, &DxfClass> {
        self.classes.iter().map(|c| (c.class_number, c)).collect()
    }
}

/// Reader for DWG classes section
pub struct DwgClassesReader<R: Read + Seek> {
    reader: BitReader<R>,
    version: ACadVersion,
}

impl<R: Read + Seek> DwgClassesReader<R> {
    /// Create a new classes reader
    pub fn new(reader: BitReader<R>, version: ACadVersion) -> Self {
        Self { reader, version }
    }
    
    /// Check if version is R2007+
    fn r2007_plus(&self) -> bool {
        self.version >= ACadVersion::AC1021
    }
    
    /// Check if version is R2004+
    fn r2004_plus(&self) -> bool {
        self.version >= ACadVersion::AC1018
    }
    
    /// Verify a sentinel matches expected bytes
    fn check_sentinel(&mut self, expected: &[u8; 16]) -> Result<bool> {
        let sentinel = self.reader.read_sentinel()?;
        Ok(&sentinel == expected)
    }
    
    /// Read all classes from the section
    pub fn read(&mut self) -> Result<DxfClassCollection> {
        let mut collection = DxfClassCollection::new();
        
        // Check start sentinel
        if !self.check_sentinel(&DwgSectionDefinition::CLASSES_START_SENTINEL)? {
            return Err(DxfError::InvalidHeader("Invalid classes start sentinel".to_string()));
        }
        
        // Read section size
        let _size = self.reader.read_raw_long()?;
        
        // R2007+: Size in bits
        let size_in_bits = if self.r2007_plus() {
            Some(self.reader.read_raw_long()? as u64)
        } else {
            None
        };
        
        let initial_pos = self.reader.position_in_bits();
        
        // Read class data
        // The number of classes is not explicitly stored, we read until we hit the sentinel
        loop {
            let mut class = DxfClass::default();
            
            // Class number
            class.class_number = self.reader.read_bitshort()?;
            
            // Check if we've reached a terminator (class number 0 typically indicates end)
            // But actually we need to check sentinel, let's read the flags first
            
            // Proxy flags
            class.proxy_flags = self.reader.read_bitshort()?;
            
            // DXF class name
            class.dxf_name = self.reader.read_variable_text(self.version)?;
            
            // C++ class name
            class.cpp_class_name = self.reader.read_variable_text(self.version)?;
            
            // Application name
            class.application_name = self.reader.read_variable_text(self.version)?;
            
            // Flags
            class.was_proxy = self.reader.read_bitshort()? != 0;
            let entity_flag = self.reader.read_bitshort()?;
            class.is_entity = (entity_flag & 0x01FF) == 0x01F2 || (entity_flag & 0x01FF) == 0x01F3;
            
            // R2004+: Instance count and version info
            if self.r2004_plus() {
                class.instance_count = self.reader.read_bitlong()?;
                class.dwg_version = self.reader.read_bitlong()? as u32;
                class.maintenance_version = self.reader.read_bitlong()? as u32;
                class.unknown1 = self.reader.read_bitlong()?;
                class.unknown2 = self.reader.read_bitlong()?;
            }
            
            collection.add(class);
            
            // Check if we've read enough data
            if let Some(bits) = size_in_bits {
                let current_pos = self.reader.position_in_bits();
                if current_pos - initial_pos >= bits {
                    break;
                }
            }
            
            // Safety check: Don't read more than 500 classes (reasonable limit)
            if collection.len() >= 500 {
                break;
            }
            
            // Try to peek ahead to see if next data looks like a class
            // This is a heuristic to avoid reading past the end
            // A proper implementation would track the bit position more carefully
        }
        
        // Verify end sentinel
        // Note: We may need to align to byte boundary first
        let _ = self.check_sentinel(&DwgSectionDefinition::CLASSES_END_SENTINEL);
        
        Ok(collection)
    }
}

/// Object type codes for standard DWG objects
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum ObjectType {
    /// Invalid/unused
    Invalid = 0x00,
    /// Text entity
    Text = 0x01,
    /// Attribute entity
    Attrib = 0x02,
    /// Attribute definition
    AttDef = 0x03,
    /// Block begin
    Block = 0x04,
    /// Block end
    Endblk = 0x05,
    /// Sequence end
    Seqend = 0x06,
    /// Insert (block reference)
    Insert = 0x07,
    /// Multileader insert (with attributes)
    MInsert = 0x08,
    // Skip 0x09
    /// Vertex (2D)
    Vertex2D = 0x0A,
    /// Vertex (3D)
    Vertex3D = 0x0B,
    /// Vertex (mesh)
    VertexMesh = 0x0C,
    /// Vertex (pface)
    VertexPFace = 0x0D,
    /// Vertex (pface face)
    VertexPFaceFace = 0x0E,
    /// Polyline (2D)
    Polyline2D = 0x0F,
    /// Polyline (3D)
    Polyline3D = 0x10,
    /// Arc
    Arc = 0x11,
    /// Circle
    Circle = 0x12,
    /// Line
    Line = 0x13,
    /// Dimension ordinate
    DimensionOrdinate = 0x14,
    /// Dimension linear
    DimensionLinear = 0x15,
    /// Dimension aligned
    DimensionAligned = 0x16,
    /// Dimension angular (3-point)
    DimensionAng3Pt = 0x17,
    /// Dimension angular (2-line)
    DimensionAng2Ln = 0x18,
    /// Dimension radius
    DimensionRadius = 0x19,
    /// Dimension diameter
    DimensionDiameter = 0x1A,
    /// Point
    Point = 0x1B,
    /// 3D Face
    Face3D = 0x1C,
    /// Polyline pface
    PolylinePface = 0x1D,
    /// Polyline mesh
    PolylineMesh = 0x1E,
    /// Solid (2D solid fill)
    Solid = 0x1F,
    /// Trace (thick line)
    Trace = 0x20,
    /// Shape
    Shape = 0x21,
    /// Viewport entity
    Viewport = 0x22,
    /// Ellipse
    Ellipse = 0x23,
    /// Spline
    Spline = 0x24,
    /// Region
    Region = 0x25,
    /// 3D Solid
    Solid3D = 0x26,
    /// Body
    Body = 0x27,
    /// Ray
    Ray = 0x28,
    /// XLine (construction line)
    XLine = 0x29,
    /// Dictionary
    Dictionary = 0x2A,
    /// OLE frame (R13)
    OleFrame = 0x2B,
    /// MText
    MText = 0x2C,
    /// Leader
    Leader = 0x2D,
    /// Tolerance
    Tolerance = 0x2E,
    /// MLine
    MLine = 0x2F,
    /// Block control object
    BlockControlObj = 0x30,
    /// Block header
    BlockHeader = 0x31,
    /// Layer control object
    LayerControlObj = 0x32,
    /// Layer
    Layer = 0x33,
    /// Shape file control object
    ShapeFileControlObj = 0x34,
    /// Shape file (text style)
    ShapeFile = 0x35,
    // Skip some
    /// Linetype control object
    LinetypeControlObj = 0x38,
    /// Linetype
    Linetype = 0x39,
    // Skip some
    /// View control object
    ViewControlObj = 0x3C,
    /// View
    View = 0x3D,
    /// UCS control object
    UcsControlObj = 0x3E,
    /// UCS
    Ucs = 0x3F,
    /// VPort control object
    VPortControlObj = 0x40,
    /// VPort
    VPort = 0x41,
    /// AppID control object
    AppIdControlObj = 0x42,
    /// AppID
    AppId = 0x43,
    /// Dim style control object
    DimStyleControlObj = 0x44,
    /// Dim style
    DimStyle = 0x45,
    /// VP entity header control object (R13-R15)
    VpEntHdrCtrlObj = 0x46,
    /// VP entity header (R13-R15)
    VpEntHdr = 0x47,
    /// Group
    Group = 0x48,
    /// MLine style
    MLineStyle = 0x49,
    /// OLE2 frame
    Ole2Frame = 0x4A,
    // R14+ additions
    /// Dummy (R14+)
    Dummy = 0x4B,
    /// Long transaction (R14+)
    LongTransaction = 0x4C,
    /// Lwpolyline (R14+)
    LwPolyline = 0x4D,
    /// Hatch (R14+)
    Hatch = 0x4E,
    /// XRecord
    XRecord = 0x4F,
    /// AcDbPlaceholder
    Placeholder = 0x50,
    // R2000+ additions
    /// VBA project (R2000+)
    VbaProject = 0x51,
    /// Layout (R2000+)
    Layout = 0x52,
    /// Custom class start marker
    CustomClassStart = 0x1F4,
}

impl TryFrom<u16> for ObjectType {
    type Error = ();
    
    fn try_from(value: u16) -> std::result::Result<Self, ()> {
        match value {
            0x00 => Ok(ObjectType::Invalid),
            0x01 => Ok(ObjectType::Text),
            0x02 => Ok(ObjectType::Attrib),
            0x03 => Ok(ObjectType::AttDef),
            0x04 => Ok(ObjectType::Block),
            0x05 => Ok(ObjectType::Endblk),
            0x06 => Ok(ObjectType::Seqend),
            0x07 => Ok(ObjectType::Insert),
            0x08 => Ok(ObjectType::MInsert),
            0x0A => Ok(ObjectType::Vertex2D),
            0x0B => Ok(ObjectType::Vertex3D),
            0x0C => Ok(ObjectType::VertexMesh),
            0x0D => Ok(ObjectType::VertexPFace),
            0x0E => Ok(ObjectType::VertexPFaceFace),
            0x0F => Ok(ObjectType::Polyline2D),
            0x10 => Ok(ObjectType::Polyline3D),
            0x11 => Ok(ObjectType::Arc),
            0x12 => Ok(ObjectType::Circle),
            0x13 => Ok(ObjectType::Line),
            0x14 => Ok(ObjectType::DimensionOrdinate),
            0x15 => Ok(ObjectType::DimensionLinear),
            0x16 => Ok(ObjectType::DimensionAligned),
            0x17 => Ok(ObjectType::DimensionAng3Pt),
            0x18 => Ok(ObjectType::DimensionAng2Ln),
            0x19 => Ok(ObjectType::DimensionRadius),
            0x1A => Ok(ObjectType::DimensionDiameter),
            0x1B => Ok(ObjectType::Point),
            0x1C => Ok(ObjectType::Face3D),
            0x1D => Ok(ObjectType::PolylinePface),
            0x1E => Ok(ObjectType::PolylineMesh),
            0x1F => Ok(ObjectType::Solid),
            0x20 => Ok(ObjectType::Trace),
            0x21 => Ok(ObjectType::Shape),
            0x22 => Ok(ObjectType::Viewport),
            0x23 => Ok(ObjectType::Ellipse),
            0x24 => Ok(ObjectType::Spline),
            0x25 => Ok(ObjectType::Region),
            0x26 => Ok(ObjectType::Solid3D),
            0x27 => Ok(ObjectType::Body),
            0x28 => Ok(ObjectType::Ray),
            0x29 => Ok(ObjectType::XLine),
            0x2A => Ok(ObjectType::Dictionary),
            0x2B => Ok(ObjectType::OleFrame),
            0x2C => Ok(ObjectType::MText),
            0x2D => Ok(ObjectType::Leader),
            0x2E => Ok(ObjectType::Tolerance),
            0x2F => Ok(ObjectType::MLine),
            0x30 => Ok(ObjectType::BlockControlObj),
            0x31 => Ok(ObjectType::BlockHeader),
            0x32 => Ok(ObjectType::LayerControlObj),
            0x33 => Ok(ObjectType::Layer),
            0x34 => Ok(ObjectType::ShapeFileControlObj),
            0x35 => Ok(ObjectType::ShapeFile),
            0x38 => Ok(ObjectType::LinetypeControlObj),
            0x39 => Ok(ObjectType::Linetype),
            0x3C => Ok(ObjectType::ViewControlObj),
            0x3D => Ok(ObjectType::View),
            0x3E => Ok(ObjectType::UcsControlObj),
            0x3F => Ok(ObjectType::Ucs),
            0x40 => Ok(ObjectType::VPortControlObj),
            0x41 => Ok(ObjectType::VPort),
            0x42 => Ok(ObjectType::AppIdControlObj),
            0x43 => Ok(ObjectType::AppId),
            0x44 => Ok(ObjectType::DimStyleControlObj),
            0x45 => Ok(ObjectType::DimStyle),
            0x46 => Ok(ObjectType::VpEntHdrCtrlObj),
            0x47 => Ok(ObjectType::VpEntHdr),
            0x48 => Ok(ObjectType::Group),
            0x49 => Ok(ObjectType::MLineStyle),
            0x4A => Ok(ObjectType::Ole2Frame),
            0x4B => Ok(ObjectType::Dummy),
            0x4C => Ok(ObjectType::LongTransaction),
            0x4D => Ok(ObjectType::LwPolyline),
            0x4E => Ok(ObjectType::Hatch),
            0x4F => Ok(ObjectType::XRecord),
            0x50 => Ok(ObjectType::Placeholder),
            0x51 => Ok(ObjectType::VbaProject),
            0x52 => Ok(ObjectType::Layout),
            _ => Err(()),
        }
    }
}

impl ObjectType {
    /// Check if this is an entity type (vs table/object)
    pub fn is_entity(&self) -> bool {
        let val = *self as u16;
        // Entities are generally in the 0x01-0x2B range plus some later additions
        matches!(val, 0x01..=0x2B | 0x4D | 0x4E)
    }
    
    /// Check if this is a table entry type
    pub fn is_table_entry(&self) -> bool {
        let val = *self as u16;
        matches!(val, 0x31 | 0x33 | 0x35 | 0x39 | 0x3D | 0x3F | 0x41 | 0x43 | 0x45 | 0x47)
    }
    
    /// Check if this is a table control object
    pub fn is_table_control(&self) -> bool {
        let val = *self as u16;
        matches!(val, 0x30 | 0x32 | 0x34 | 0x38 | 0x3C | 0x3E | 0x40 | 0x42 | 0x44 | 0x46)
    }
}
