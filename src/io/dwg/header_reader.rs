//! DWG Header Reader - Reads drawing header variables from DWG files
//!
//! The header section contains all drawing variables like DIMSCALE, LTSCALE,
//! layer/style/block references, and other settings.

use std::io::{Read, Seek};
use crate::document::HeaderVariables;
use crate::error::{DxfError, Result};
use crate::types::{ACadVersion, Color, Handle, Vector2, Vector3};
use super::stream_reader::{BitReader, DwgStreamReader};
use super::section::DwgSectionDefinition;

/// Collection of handles referenced from the header section
#[derive(Debug, Clone, Default)]
pub struct DwgHeaderHandles {
    // Table control handles
    pub block_control: Option<u64>,
    pub layer_control: Option<u64>,
    pub style_control: Option<u64>,
    pub linetype_control: Option<u64>,
    pub view_control: Option<u64>,
    pub ucs_control: Option<u64>,
    pub vport_control: Option<u64>,
    pub appid_control: Option<u64>,
    pub dimstyle_control: Option<u64>,
    pub vp_entity_header_control: Option<u64>,  // R13-R15 only
    
    // Block record handles
    pub model_space: Option<u64>,
    pub paper_space: Option<u64>,
    
    // Dictionary handles
    pub named_objects_dict: Option<u64>,
    
    // Standard linetype handles
    pub bylayer_linetype: Option<u64>,
    pub byblock_linetype: Option<u64>,
    pub continuous_linetype: Option<u64>,
    
    // Current entity references
    pub current_layer: Option<u64>,
    pub current_textstyle: Option<u64>,
    pub current_linetype: Option<u64>,
    pub current_dimstyle: Option<u64>,
    pub current_multiline_style: Option<u64>,
    
    // Dimension style sub-handles
    pub dim_textstyle: Option<u64>,
    pub dim_linetype1: Option<u64>,
    pub dim_linetype2: Option<u64>,
    pub dim_arrow1: Option<u64>,
    pub dim_arrow2: Option<u64>,
    pub dim_leader_arrow: Option<u64>,
    
    // Group dictionary
    pub group_dict: Option<u64>,
    pub mline_style_dict: Option<u64>,
    pub color_dict: Option<u64>,
    pub material_dict: Option<u64>,
    pub visualstyle_dict: Option<u64>,
    pub plotstyle_dict: Option<u64>,
    pub tablestyle_dict: Option<u64>,
    pub mleaderstyle_dict: Option<u64>,
    
    // UCS references
    pub ucs_origin: Option<u64>,
    pub ucs_xaxis: Option<u64>,
    pub ucs_yaxis: Option<u64>,
    pub ucs_ortho_ref: Option<u64>,
    pub pucs_origin: Option<u64>,
    pub pucs_xaxis: Option<u64>,
    pub pucs_yaxis: Option<u64>,
    pub pucs_ortho_ref: Option<u64>,
    
    // Viewport handle (R2000+)
    pub current_viewport: Option<u64>,
    
    // Layouts
    pub layout_dict: Option<u64>,
    pub current_layout: Option<u64>,
    
    // Detail view style (R2013+)
    pub detail_viewstyle_dict: Option<u64>,
    pub section_viewstyle_dict: Option<u64>,
    
    // Interference (R2007+)
    pub interference_object: Option<u64>,
    pub interference_vport: Option<u64>,
    
    // Dragvs (R2007+)
    pub drag_visualstyle: Option<u64>,
}

impl DwgHeaderHandles {
    /// Get all non-None handles as an iterator
    pub fn get_handles(&self) -> Vec<u64> {
        let mut handles = Vec::new();
        
        // Use a macro to reduce repetition
        macro_rules! push_if_some {
            ($($field:ident),*) => {
                $(
                    if let Some(h) = self.$field {
                        handles.push(h);
                    }
                )*
            };
        }
        
        push_if_some!(
            block_control, layer_control, style_control, linetype_control,
            view_control, ucs_control, vport_control, appid_control,
            dimstyle_control, vp_entity_header_control,
            model_space, paper_space, named_objects_dict,
            bylayer_linetype, byblock_linetype, continuous_linetype,
            current_layer, current_textstyle, current_linetype,
            current_dimstyle, current_multiline_style,
            dim_textstyle, dim_linetype1, dim_linetype2,
            dim_arrow1, dim_arrow2, dim_leader_arrow,
            group_dict, mline_style_dict, color_dict, material_dict,
            visualstyle_dict, plotstyle_dict, tablestyle_dict, mleaderstyle_dict,
            ucs_origin, ucs_xaxis, ucs_yaxis, ucs_ortho_ref,
            pucs_origin, pucs_xaxis, pucs_yaxis, pucs_ortho_ref,
            current_viewport, layout_dict, current_layout,
            detail_viewstyle_dict, section_viewstyle_dict,
            interference_object, interference_vport, drag_visualstyle
        );
        
        handles
    }
}

/// Reader for DWG header section
pub struct DwgHeaderReader<R: Read + Seek> {
    reader: BitReader<R>,
    version: ACadVersion,
    maintenance_version: i32,
}

impl<R: Read + Seek> DwgHeaderReader<R> {
    /// Create a new header reader
    pub fn new(reader: BitReader<R>, version: ACadVersion, maintenance_version: i32) -> Self {
        Self {
            reader,
            version,
            maintenance_version,
        }
    }
    
    /// Check if version is R13-R14 only
    fn r13_14_only(&self) -> bool {
        self.version >= ACadVersion::AC1012 && self.version <= ACadVersion::AC1014
    }
    
    /// Check if version is R2004+ 
    fn r2004_plus(&self) -> bool {
        self.version >= ACadVersion::AC1018
    }
    
    /// Check if version is R2007+
    fn r2007_plus(&self) -> bool {
        self.version >= ACadVersion::AC1021
    }
    
    /// Check if version is R2010+
    fn r2010_plus(&self) -> bool {
        self.version >= ACadVersion::AC1024
    }
    
    /// Check if version is R2013+
    fn r2013_plus(&self) -> bool {
        self.version >= ACadVersion::AC1027
    }
    
    /// Check if version is R2018+
    fn r2018_plus(&self) -> bool {
        self.version >= ACadVersion::AC1032
    }
    
    /// Check if earlier than R2004
    fn pre_r2004(&self) -> bool {
        self.version < ACadVersion::AC1018
    }
    
    /// Verify a sentinel matches expected bytes
    fn check_sentinel(&mut self, expected: &[u8; 16]) -> Result<bool> {
        let sentinel = self.reader.read_sentinel()?;
        Ok(&sentinel == expected)
    }
    
    /// Read the header section
    pub fn read(&mut self, header: &mut HeaderVariables) -> Result<DwgHeaderHandles> {
        let mut handles = DwgHeaderHandles::default();
        
        // Check start sentinel
        if !self.check_sentinel(&DwgSectionDefinition::HEADER_START_SENTINEL)? {
            return Err(DxfError::InvalidHeader("Invalid header start sentinel".to_string()));
        }
        
        // Read size of section
        let _size = self.reader.read_raw_long()?;
        
        // R2010+ with maintenance version > 3 or R2018+: read extra 4 bytes
        if (self.r2010_plus() && self.maintenance_version > 3) || self.r2018_plus() {
            let _unknown = self.reader.read_raw_long()?;
        }
        
        let initial_pos = self.reader.position_in_bits();
        
        // R2007+: Handle string data positioning
        if self.r2007_plus() {
            // Size in bits
            let _size_in_bits = self.reader.read_raw_long()?;
            // TODO: Set up merged reader for text data
        }
        
        // R2013+: REQUIREDVERSIONS
        if self.r2013_plus() {
            header.required_versions = self.reader.read_bitlonglong()?;
        }
        
        // Common: Unknown values
        let _unknown_bd1 = self.reader.read_bitdouble()?;
        let _unknown_bd2 = self.reader.read_bitdouble()?;
        let _unknown_bd3 = self.reader.read_bitdouble()?;
        let _unknown_bd4 = self.reader.read_bitdouble()?;
        
        // Unknown text strings
        let _ = self.reader.read_variable_text(self.version)?;
        let _ = self.reader.read_variable_text(self.version)?;
        let _ = self.reader.read_variable_text(self.version)?;
        let _ = self.reader.read_variable_text(self.version)?;
        
        // Unknown longs
        let _unknown_bl1 = self.reader.read_bitlong()?;
        let _unknown_bl2 = self.reader.read_bitlong()?;
        
        // R13-R14: Additional unknown
        if self.r13_14_only() {
            let _unknown_bs = self.reader.read_bitshort()?;
        }
        
        // Pre-2004: Current viewport entity header
        if self.pre_r2004() {
            handles.vp_entity_header_control = Some(self.reader.read_handle()?);
        }
        
        // Common boolean flags
        header.associate_dimensions = self.reader.read_bit()?;
        header.update_dimensions_while_dragging = self.reader.read_bit()?;
        
        // R13-R14: DIMSAV
        if self.r13_14_only() {
            let _dimsav = self.reader.read_bit()?;
        }
        
        // Common flags continued
        header.polyline_linetype_generation = self.reader.read_bit()?;
        header.ortho_mode = self.reader.read_bit()?;
        header.regen_mode = self.reader.read_bit()?;
        header.fill_mode = self.reader.read_bit()?;
        header.quick_text_mode = self.reader.read_bit()?;
        header.paper_space_linetype_scaling = self.reader.read_bit()?;
        header.limit_check = self.reader.read_bit()?;
        
        // R13-R14: BLIPMODE
        if self.r13_14_only() {
            header.blip_mode = self.reader.read_bit()?;
        }
        
        // R2004+: Unknown bit
        if self.r2004_plus() {
            let _ = self.reader.read_bit()?;
        }
        
        // Common continued
        header.user_timer = self.reader.read_bit()?;
        let _skpoly = self.reader.read_bit()?;
        header.angle_direction = self.reader.read_bit()? as i16;
        header.spline_frame = self.reader.read_bit()?;
        
        // R13-R14: ATTREQ, ATTDIA
        if self.r13_14_only() {
            header.attribute_request = self.reader.read_bit()?;
            header.attribute_dialog = self.reader.read_bit()?;
        }
        
        header.mirror_text = self.reader.read_bit()?;
        header.world_view = self.reader.read_bit()?;
        
        // R13-R14: WIREFRAME
        if self.r13_14_only() {
            let _wireframe = self.reader.read_bit()?;
        }
        
        header.show_model_space = self.reader.read_bit()?;
        header.paper_space_limit_check = self.reader.read_bit()?;
        header.retain_xref_visibility = self.reader.read_bit()?;
        
        // R13-R14: DELOBJ
        if self.r13_14_only() {
            header.delete_objects = self.reader.read_bit()?;
        }
        
        header.display_silhouette = self.reader.read_bit()?;
        let _pellipse = self.reader.read_bit()?;
        
        // PROXYGRAPHICS
        header.proxy_graphics = self.reader.read_bitshort()?;
        
        // R13-R14: DRAGMODE
        if self.r13_14_only() {
            header.drag_mode = self.reader.read_bitshort()?;
        }
        
        // TREEDEPTH
        header.tree_depth = self.reader.read_bitshort()?;
        
        // LUNITS, LUPREC, AUNITS, AUPREC
        header.linear_unit_format = self.reader.read_bitshort()?;
        header.linear_unit_precision = self.reader.read_bitshort()?;
        header.angular_unit_format = self.reader.read_bitshort()?;
        header.angular_unit_precision = self.reader.read_bitshort()?;
        
        // Object snap mode (R13 uses BS, later uses BL)
        if self.version <= ACadVersion::AC1014 {
            header.object_snap_mode = self.reader.read_bitshort()? as i32;
        } else {
            // R2004+: object snap is stored differently
        }
        
        // ATTMODE
        header.attribute_visibility = self.reader.read_bitshort()?;
        
        // R13-R14: COORDS
        if self.r13_14_only() {
            header.coords_mode = self.reader.read_bitshort()?;
        }
        
        // PDMODE
        header.point_display_mode = self.reader.read_bitshort()?;
        
        // R13-R14: PICKSTYLE
        if self.r13_14_only() {
            header.pick_style = self.reader.read_bitshort()?;
        }
        
        // R2004+: Unknown BL
        if self.r2004_plus() {
            let _unknown_bl = self.reader.read_bitlong()?;
        }
        
        // R2007+: Unknown BL
        if self.r2007_plus() {
            let _unknown_bl = self.reader.read_bitlong()?;
        }
        
        // R2007+: Unknown BL
        if self.r2007_plus() {
            let _unknown_bl = self.reader.read_bitlong()?;
        }
        
        // USERI1-5
        header.user_int1 = self.reader.read_bitshort()?;
        header.user_int2 = self.reader.read_bitshort()?;
        header.user_int3 = self.reader.read_bitshort()?;
        header.user_int4 = self.reader.read_bitshort()?;
        header.user_int5 = self.reader.read_bitshort()?;
        
        // SPLINESEGS
        header.spline_segments = self.reader.read_bitshort()?;
        
        // SURFU, SURFV
        header.surface_u_density = self.reader.read_bitshort()?;
        header.surface_v_density = self.reader.read_bitshort()?;
        
        // SURFTYPE
        header.surface_type = self.reader.read_bitshort()?;
        
        // SURFTAB1, SURFTAB2
        header.surface_tab1 = self.reader.read_bitshort()?;
        header.surface_tab2 = self.reader.read_bitshort()?;
        
        // SPLINETYPE
        header.spline_type = self.reader.read_bitshort()?;
        
        // SHADEDGE, SHADEDIF
        header.shade_edge = self.reader.read_bitshort()?;
        header.shade_diffuse = self.reader.read_bitshort()?;
        
        // UNITMODE
        let _unitmode = self.reader.read_bitshort()?;
        
        // MAXACTVP
        header.max_active_viewports = self.reader.read_bitshort()?;
        
        // ISOLINES
        header.isolines = self.reader.read_bitshort()?;
        
        // CMLJUST
        header.multiline_justification = self.reader.read_bitshort()?;
        
        // TEXTQLTY
        header.text_quality = self.reader.read_bitshort()?;
        
        // LTSCALE
        header.linetype_scale = self.reader.read_bitdouble()?;
        
        // TEXTSIZE
        header.text_height = self.reader.read_bitdouble()?;
        
        // TRACEWID
        header.trace_width = self.reader.read_bitdouble()?;
        
        // SKETCHINC
        header.sketch_increment = self.reader.read_bitdouble()?;
        
        // FILLETRAD
        header.fillet_radius = self.reader.read_bitdouble()?;
        
        // THICKNESS
        header.thickness = self.reader.read_bitdouble()?;
        
        // ANGBASE
        header.angle_base = self.reader.read_bitdouble()?;
        
        // PDSIZE
        header.point_display_size = self.reader.read_bitdouble()?;
        
        // PLINEWID
        header.polyline_width = self.reader.read_bitdouble()?;
        
        // USERR1-5
        header.user_real1 = self.reader.read_bitdouble()?;
        header.user_real2 = self.reader.read_bitdouble()?;
        header.user_real3 = self.reader.read_bitdouble()?;
        header.user_real4 = self.reader.read_bitdouble()?;
        header.user_real5 = self.reader.read_bitdouble()?;
        
        // CHAMFERA, CHAMFERB
        header.chamfer_distance_a = self.reader.read_bitdouble()?;
        header.chamfer_distance_b = self.reader.read_bitdouble()?;
        
        // CHAMFERC, CHAMFERD
        header.chamfer_length = self.reader.read_bitdouble()?;
        header.chamfer_angle = self.reader.read_bitdouble()?;
        
        // FACETRES
        header.facet_resolution = self.reader.read_bitdouble()?;
        
        // CMLSCALE
        header.multiline_scale = self.reader.read_bitdouble()?;
        
        // CELTSCALE
        header.current_entity_linetype_scale = self.reader.read_bitdouble()?;
        
        // MENUNAME string (R2007+ uses different handling)
        header.menu_name = self.reader.read_variable_text(self.version)?;
        
        // TDCREATE, TDUPDATE
        header.create_date_julian = self.reader.read_julian_date()?;
        header.update_date_julian = self.reader.read_julian_date()?;
        
        // R2004+: Unknown BL
        if self.r2004_plus() {
            let _unknown_bl = self.reader.read_bitlong()?;
            let _unknown_bl2 = self.reader.read_bitlong()?;
            let _unknown_bl3 = self.reader.read_bitlong()?;
        }
        
        // TDINDWG, TDUSRTIMER
        header.total_editing_time = self.reader.read_julian_date()?;
        header.user_elapsed_time = self.reader.read_julian_date()?;
        
        // CECOLOR
        header.current_entity_color = self.reader.read_cmc_color()?;
        
        // Read handle references at the end of header section
        self.read_handles(&mut handles)?;
        
        // TODO: Read more header variables...
        
        Ok(handles)
    }
    
    /// Read handle references from header
    fn read_handles(&mut self, handles: &mut DwgHeaderHandles) -> Result<()> {
        // Table control handles
        handles.block_control = Some(self.reader.read_handle()?);
        handles.layer_control = Some(self.reader.read_handle()?);
        handles.style_control = Some(self.reader.read_handle()?);
        handles.linetype_control = Some(self.reader.read_handle()?);
        handles.view_control = Some(self.reader.read_handle()?);
        handles.ucs_control = Some(self.reader.read_handle()?);
        handles.vport_control = Some(self.reader.read_handle()?);
        handles.appid_control = Some(self.reader.read_handle()?);
        handles.dimstyle_control = Some(self.reader.read_handle()?);
        
        // R13-R15: VP entity header control
        if self.version <= ACadVersion::AC1015 {
            handles.vp_entity_header_control = Some(self.reader.read_handle()?);
        }
        
        // Named objects dictionary
        handles.named_objects_dict = Some(self.reader.read_handle()?);
        
        // R13-R14: MLINESTYLE
        if self.r13_14_only() {
            handles.mline_style_dict = Some(self.reader.read_handle()?);
        }
        
        // R2000+: More handles
        if self.version >= ACadVersion::AC1015 {
            handles.group_dict = Some(self.reader.read_handle()?);
            handles.mline_style_dict = Some(self.reader.read_handle()?);
            handles.named_objects_dict = Some(self.reader.read_handle()?); // Repeat?
        }
        
        // Block records
        handles.paper_space = Some(self.reader.read_handle()?);
        handles.model_space = Some(self.reader.read_handle()?);
        
        // Standard linetypes
        handles.bylayer_linetype = Some(self.reader.read_handle()?);
        handles.byblock_linetype = Some(self.reader.read_handle()?);
        handles.continuous_linetype = Some(self.reader.read_handle()?);
        
        // R2007+: Additional dictionaries
        if self.r2007_plus() {
            handles.visualstyle_dict = Some(self.reader.read_handle()?);
        }
        
        // R2000+: Current refs
        if self.version >= ACadVersion::AC1015 {
            handles.current_layer = Some(self.reader.read_handle()?);
            handles.current_textstyle = Some(self.reader.read_handle()?);
            handles.current_linetype = Some(self.reader.read_handle()?);
            handles.current_dimstyle = Some(self.reader.read_handle()?);
            handles.current_multiline_style = Some(self.reader.read_handle()?);
        }
        
        Ok(())
    }
}
