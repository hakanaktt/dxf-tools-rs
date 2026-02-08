//! DWG Writer — Produces DWG files in AC18 (R2004) format
//!
//! Assembles a complete DWG file from a `CadDocument`:
//! 1. Serialize header variables, classes, objects, and handle map into sections
//! 2. Compress each section with LZ77
//! 3. Wrap in pages with checksums
//! 4. Write section map + page map
//! 5. Write encrypted file header at offset 0

use std::collections::BTreeMap;
use std::path::Path;

use crate::document::{CadDocument, HeaderVariables};
use crate::entities::*;
use crate::error::{DxfError, Result};
use crate::tables::*;
use crate::types::{ACadVersion, Color, Handle, Vector2, Vector3};

use super::crc::{Crc8, Crc32};
use super::compressor::{Lz77AC18Compressor, compression_padding, magic_sequence};
use super::section::{
    DwgSectionDefinition, DwgSectionDescriptor, DwgLocalSectionMap,
    section_names,
};
use super::stream_writer::DwgStreamWriter;
use super::stream_reader::DwgReferenceType;

/// Default decompressed page size for compressed sections
const DEFAULT_DECOMP_SIZE: usize = 0x7400;
/// Data page type constant
const DATA_PAGE_TYPE: u32 = 0x4163043B;
/// Section map page type
const SECTION_MAP_TYPE: u32 = 0x4163003B;
/// Page map page type
const PAGE_MAP_TYPE: u32 = 0x41630E3B;

/// DWG file writer
pub struct DwgWriter {
    version: ACadVersion,
}

impl DwgWriter {
    pub fn new() -> Self {
        Self { version: ACadVersion::AC1018 }
    }

    pub fn with_version(mut self, version: ACadVersion) -> Self {
        self.version = version;
        self
    }

    /// Write a CadDocument to a file path
    pub fn write_to_file(&self, doc: &CadDocument, path: impl AsRef<Path>) -> Result<()> {
        let data = self.write(doc)?;
        std::fs::write(path, &data).map_err(DxfError::Io)?;
        Ok(())
    }

    /// Write a CadDocument to bytes
    pub fn write(&self, doc: &CadDocument) -> Result<Vec<u8>> {
        let version = self.version;
        let mut ctx = WriteContext::new(version);

        // 1. Serialize sections
        let header_data = Self::write_header_section(doc, version)?;
        let classes_data = Self::write_classes_section(version)?;
        let objects_data = Self::write_objects_section(doc, version)?;
        let handles_data = Self::write_handles_section(&objects_data.1)?;
        let auxheader_data = Self::write_aux_header()?;
        let summary_data = vec![0u8; 64];

        // 2. Add sections to context
        ctx.add_section(section_names::HEADER, &header_data, true, DEFAULT_DECOMP_SIZE)?;
        ctx.add_section(section_names::CLASSES, &classes_data, true, DEFAULT_DECOMP_SIZE)?;
        ctx.add_section(section_names::SUMMARY_INFO, &summary_data, false, 0x100)?;
        if !auxheader_data.is_empty() {
            ctx.add_section(section_names::AUX_HEADER, &auxheader_data, true, DEFAULT_DECOMP_SIZE)?;
        }
        ctx.add_section(section_names::AC_DB_OBJECTS, &objects_data.0, true, DEFAULT_DECOMP_SIZE)?;
        ctx.add_section(section_names::HANDLES, &handles_data, true, DEFAULT_DECOMP_SIZE)?;

        // 3. Finalize
        ctx.finalize()
    }

    // =========================================================================
    // Header Variables Section
    // =========================================================================

    fn write_header_section(doc: &CadDocument, version: ACadVersion) -> Result<Vec<u8>> {
        let mut section = Vec::new();
        let hdr = &doc.header;

        section.extend_from_slice(&DwgSectionDefinition::HEADER_START_SENTINEL);

        let mut data_writer = DwgStreamWriter::new(version);
        Self::write_header_variables(&mut data_writer, hdr, version);
        let data = data_writer.into_bytes();

        let size = data.len() as i32;
        section.extend_from_slice(&size.to_le_bytes());

        let mut crc = Crc8::new(0xC0C1);
        crc.update_slice(&size.to_le_bytes());
        crc.update_slice(&data);
        section.extend_from_slice(&data);
        section.extend_from_slice(&crc.value().to_le_bytes());

        section.extend_from_slice(&DwgSectionDefinition::HEADER_END_SENTINEL);
        Ok(section)
    }

    fn write_header_variables(w: &mut DwgStreamWriter, hdr: &HeaderVariables, version: ACadVersion) {
        if version >= ACadVersion::AC1027 {
            w.write_bitlonglong(hdr.required_versions);
        }

        // Unknown doubles/strings
        w.write_bitdouble(0.0);
        w.write_bitdouble(0.0);
        w.write_bitdouble(0.0);
        w.write_bitdouble(0.0);
        w.write_variable_text("");
        w.write_variable_text("");
        w.write_variable_text("");
        w.write_variable_text("");

        w.write_bitlong(24);
        w.write_bitlong(0);

        if version <= ACadVersion::AC1014 {
            w.write_bitshort(0);
        }

        // INSBASE
        w.write_3bitdouble(hdr.model_space_insertion_base);
        // EXTMIN/EXTMAX
        w.write_3bitdouble(hdr.model_space_extents_min);
        w.write_3bitdouble(hdr.model_space_extents_max);
        // LIMMIN/LIMMAX
        w.write_2raw_double(hdr.model_space_limits_min);
        w.write_2raw_double(hdr.model_space_limits_max);

        // ELEVATION
        w.write_bitdouble(hdr.elevation);
        // UCSORG / UCSXDIR / UCSYDIR
        w.write_3bitdouble(hdr.model_space_ucs_origin);
        w.write_3bitdouble(hdr.model_space_ucs_x_axis);
        w.write_3bitdouble(hdr.model_space_ucs_y_axis);

        if version >= ACadVersion::AC1015 {
            w.write_bitshort(hdr.ucs_ortho_view);
        }

        w.write_bitdouble(hdr.elevation);

        // Snap/Grid (default values — these are viewport-specific)
        w.write_bitshort(0); // snap mode
        w.write_bitshort(0); // grid on
        w.write_bitshort(0); // snap style
        w.write_bitshort(0); // snap isopair
        w.write_bitdouble(0.0); // snap rotation
        w.write_2raw_double(Vector2 { x: 0.0, y: 0.0 }); // snap base
        w.write_2raw_double(Vector2 { x: 10.0, y: 10.0 }); // snap spacing
        w.write_2raw_double(Vector2 { x: 10.0, y: 10.0 }); // grid spacing

        // Drawing mode flags
        w.write_bit(hdr.ortho_mode);
        w.write_bit(hdr.regen_mode);
        w.write_bit(hdr.fill_mode);
        w.write_bit(hdr.quick_text_mode);
        w.write_bit(hdr.polyline_linetype_generation);
        w.write_bitdouble(hdr.linetype_scale);
        w.write_bit(hdr.show_model_space); // TILEMODE
        w.write_bit(hdr.limit_check);

        if version >= ACadVersion::AC1018 {
            w.write_bit(false); // undocumented
        }

        w.write_bit(hdr.display_silhouette);
        w.write_bit(hdr.world_view);
        w.write_bitshort(hdr.drag_mode);

        // Unit/precision settings
        w.write_bitshort(hdr.linear_unit_format);
        w.write_bitshort(hdr.linear_unit_precision);
        w.write_bitshort(hdr.angular_unit_format);
        w.write_bitshort(hdr.angular_unit_precision);
        w.write_bitshort(hdr.object_snap_mode as i16);

        if version >= ACadVersion::AC1018 {
            w.write_bitshort(0);
        }
        if version >= ACadVersion::AC1015 {
            w.write_bitshort(hdr.attribute_visibility);
        }
        if version >= ACadVersion::AC1018 {
            w.write_bitshort(0);
        }

        w.write_bitshort(hdr.point_display_mode);
        w.write_bitdouble(hdr.point_display_size);

        w.write_bitdouble(hdr.thickness);
        w.write_bitdouble(hdr.text_height);
        w.write_bitdouble(hdr.trace_width);

        w.write_bitshort(hdr.spline_segments);

        if version <= ACadVersion::AC1014 {
            w.write_bitshort(hdr.surface_tab1);
            w.write_bitshort(hdr.surface_tab2);
            w.write_bitshort(hdr.surface_type);
            w.write_bitshort(hdr.surface_u_density);
            w.write_bitshort(hdr.surface_v_density);
        }

        if version >= ACadVersion::AC1018 {
            w.write_variable_text(&hdr.stylesheet);
        }

        // Dimension variables
        Self::write_dimension_variables(w, hdr, version);

        // Dates
        w.write_julian_date(hdr.create_date_julian);
        w.write_julian_date(hdr.update_date_julian);

        if version >= ACadVersion::AC1018 {
            w.write_bitlong(0);
            w.write_bitlong(0);
            w.write_bitlong(0);
        }

        // Paper space extents/limits
        w.write_3bitdouble(hdr.paper_space_insertion_base);
        w.write_3bitdouble(hdr.paper_space_extents_min);
        w.write_3bitdouble(hdr.paper_space_extents_max);
        w.write_2raw_double(hdr.paper_space_limits_min);
        w.write_2raw_double(hdr.paper_space_limits_max);
        w.write_bitdouble(hdr.paper_elevation);
        w.write_3bitdouble(hdr.paper_space_ucs_origin);
        w.write_3bitdouble(hdr.paper_space_ucs_x_axis);
        w.write_3bitdouble(hdr.paper_space_ucs_y_axis);

        if version >= ACadVersion::AC1015 {
            w.write_bitshort(hdr.paper_ucs_ortho_view);
        }

        w.write_bitshort(hdr.insertion_units);

        if version >= ACadVersion::AC1015 {
            w.write_bitshort(hdr.current_plotstyle_type);
        }

        w.write_bitshort(hdr.max_active_viewports);

        if version <= ACadVersion::AC1014 {
            w.write_bitshort(0); // DIMFIT
        }

        w.write_bitdouble(0.0); // PELEVATION

        if version >= ACadVersion::AC1018 {
            w.write_bitshort(0); // solid history
        }

        // HANDSEED
        w.write_handle(hdr.handle_seed);

        w.write_bitshort(0);
        w.write_bitshort(0);

        if version >= ACadVersion::AC1018 {
            w.write_bitshort(0);
            w.write_bitlong(0);
        }

        w.write_bitshort(hdr.measurement);

        w.write_bitshort(0); // CMLSTYLE
        w.write_bitshort(0); // CMLJUST
        w.write_bitdouble(hdr.multiline_scale);

        if version >= ACadVersion::AC1015 {
            w.write_bitshort(hdr.proxy_graphics);
        }

        w.write_julian_date(hdr.total_editing_time);
        w.write_julian_date(hdr.user_elapsed_time);

        w.write_cmc_color(&hdr.current_entity_color);

        // -------- HANDLE REFERENCES --------
        if version >= ACadVersion::AC1015 {
            w.write_handle_reference(DwgReferenceType::SoftPointer, hdr.block_control_handle.value());
            w.write_handle_reference(DwgReferenceType::SoftPointer, hdr.layer_control_handle.value());
            w.write_handle_reference(DwgReferenceType::SoftPointer, hdr.style_control_handle.value());
            w.write_handle_reference(DwgReferenceType::SoftPointer, hdr.linetype_control_handle.value());
            w.write_handle_reference(DwgReferenceType::SoftPointer, hdr.view_control_handle.value());
            w.write_handle_reference(DwgReferenceType::SoftPointer, hdr.ucs_control_handle.value());
            w.write_handle_reference(DwgReferenceType::SoftPointer, hdr.vport_control_handle.value());
            w.write_handle_reference(DwgReferenceType::SoftPointer, hdr.appid_control_handle.value());
            w.write_handle_reference(DwgReferenceType::SoftPointer, hdr.dimstyle_control_handle.value());
        }

        w.write_handle_reference(DwgReferenceType::SoftPointer, hdr.vpent_hdr_control_handle.value());
        w.write_handle_reference(DwgReferenceType::SoftPointer, hdr.named_objects_dict_handle.value());

        if version >= ACadVersion::AC1015 {
            w.write_handle_reference(DwgReferenceType::SoftPointer, hdr.acad_group_dict_handle.value());
            w.write_handle_reference(DwgReferenceType::SoftPointer, hdr.acad_mlinestyle_dict_handle.value());
            w.write_handle_reference(DwgReferenceType::SoftPointer, hdr.named_objects_dict_handle.value());
        }

        if version >= ACadVersion::AC1015 {
            w.write_handle_reference(DwgReferenceType::SoftPointer, hdr.paper_space_block_handle.value());
            w.write_handle_reference(DwgReferenceType::SoftPointer, hdr.model_space_block_handle.value());
            w.write_handle_reference(DwgReferenceType::SoftPointer, hdr.bylayer_linetype_handle.value());
            w.write_handle_reference(DwgReferenceType::SoftPointer, hdr.byblock_linetype_handle.value());
        }

        if version >= ACadVersion::AC1018 {
            w.write_handle_reference(DwgReferenceType::SoftPointer, hdr.acad_layout_dict_handle.value());
            w.write_handle_reference(DwgReferenceType::SoftPointer, hdr.acad_plotsettings_dict_handle.value());
            w.write_handle_reference(DwgReferenceType::SoftPointer, hdr.acad_plotstylename_dict_handle.value());
        }

        if version >= ACadVersion::AC1021 {
            w.write_handle_reference(DwgReferenceType::SoftPointer, hdr.acad_material_dict_handle.value());
            w.write_handle_reference(DwgReferenceType::SoftPointer, hdr.acad_color_dict_handle.value());
            w.write_handle_reference(DwgReferenceType::SoftPointer, hdr.acad_visualstyle_dict_handle.value());
        }
    }

    fn write_dimension_variables(w: &mut DwgStreamWriter, hdr: &HeaderVariables, version: ACadVersion) {
        w.write_bitdouble(hdr.dim_scale);
        w.write_bitdouble(hdr.dim_arrow_size);
        w.write_bitdouble(hdr.dim_ext_line_offset);
        w.write_bitdouble(hdr.dim_line_increment);
        w.write_bitdouble(hdr.dim_ext_line_extension);
        w.write_bitdouble(hdr.dim_rounding);
        w.write_bitdouble(hdr.dim_line_extension);
        w.write_bitdouble(hdr.dim_tolerance_plus);
        w.write_bitdouble(hdr.dim_tolerance_minus);

        if version >= ACadVersion::AC1021 {
            w.write_bit(false); // DIMFXLON
            w.write_bitdouble(0.0); // DIMFXL
            w.write_bitdouble(0.0); // DIMJOGGED
        }

        w.write_bitdouble(hdr.dim_text_height);
        w.write_bitdouble(hdr.dim_center_mark);
        w.write_bitdouble(hdr.dim_tick_size);
        w.write_bitdouble(hdr.dim_alt_scale);
        w.write_bitdouble(hdr.dim_linear_scale);
        w.write_bitdouble(hdr.dim_text_vertical_pos);
        w.write_bitdouble(hdr.dim_tolerance_scale);
        w.write_bitdouble(hdr.dim_line_gap);

        if version <= ACadVersion::AC1014 {
            w.write_variable_text("");
            w.write_variable_text("");
        }

        if version >= ACadVersion::AC1015 {
            w.write_variable_text(&hdr.dim_post);
            w.write_variable_text(&hdr.dim_alt_post);
        }

        w.write_bit(hdr.dim_tolerance);
        w.write_bit(hdr.dim_limits);
        w.write_bit(hdr.dim_text_inside_horizontal);
        w.write_bit(hdr.dim_text_outside_horizontal);
        w.write_bit(hdr.dim_suppress_ext1);
        w.write_bit(hdr.dim_suppress_ext2);
        w.write_bitshort(hdr.dim_text_above);
        w.write_bitshort(hdr.dim_zero_suppression);
        w.write_bitshort(hdr.dim_alt_zero_suppression);

        if version >= ACadVersion::AC1021 {
            w.write_bitshort(0); // DIMARCSYM
        }

        w.write_bitdouble(hdr.dim_alt_scale);
        w.write_bitshort(hdr.dim_alt_decimal_places);
        w.write_bit(hdr.dim_force_line_inside);
        w.write_bit(hdr.dim_separate_arrows);
        w.write_bit(hdr.dim_force_text_inside);
        w.write_bit(hdr.dim_suppress_outside_ext);

        w.write_cmc_color(&hdr.dim_line_color);
        w.write_cmc_color(&hdr.dim_ext_line_color);
        w.write_cmc_color(&hdr.dim_text_color);

        if version >= ACadVersion::AC1015 {
            w.write_bitshort(hdr.dim_angular_decimal_places);
            w.write_bitshort(hdr.dim_decimal_places);
            w.write_bitshort(hdr.dim_tolerance_decimal_places);
            w.write_bitshort(hdr.dim_alt_units_format);
            w.write_bitshort(hdr.dim_alt_tolerance_decimal_places);
            w.write_bitshort(hdr.dim_angular_units);
            w.write_bitshort(hdr.dim_fraction_format);
            w.write_bitshort(hdr.dim_linear_unit_format);
            w.write_bitshort(hdr.dim_decimal_separator as i16);
            w.write_bitshort(hdr.dim_text_movement);
            w.write_bitshort(hdr.dim_horizontal_justification);
        }

        w.write_bit(hdr.dim_suppress_line1);
        w.write_bit(hdr.dim_suppress_line2);

        if version >= ACadVersion::AC1015 {
            w.write_bitshort(hdr.dim_tolerance_justification);
            w.write_bitshort(hdr.dim_tolerance_zero_suppression);
            w.write_bitshort(hdr.dim_alt_tolerance_zero_suppression);
            w.write_bitshort(hdr.dim_alt_tolerance_zero_tight);
        }

        if version <= ACadVersion::AC1014 {
            w.write_bit(false); // DIMFIT
        }

        w.write_bit(hdr.dim_user_positioned_text);

        if version >= ACadVersion::AC1015 {
            w.write_bitshort(hdr.dim_fit);
        }

        if version >= ACadVersion::AC1021 {
            w.write_bit(false); // DIMTFILL_FLAG
            w.write_cmc_color(&Color::ByBlock);
        }

        if version >= ACadVersion::AC1015 {
            w.write_bitshort(hdr.dim_line_weight);
            w.write_bitshort(hdr.dim_ext_line_weight);
        }
    }

    // =========================================================================
    // Classes Section
    // =========================================================================

    fn write_classes_section(version: ACadVersion) -> Result<Vec<u8>> {
        let mut section = Vec::new();
        section.extend_from_slice(&DwgSectionDefinition::CLASSES_START_SENTINEL);
        let size: i32 = 0;
        section.extend_from_slice(&size.to_le_bytes());
        if version >= ACadVersion::AC1021 {
            section.extend_from_slice(&0i32.to_le_bytes());
        }
        let crc = Crc8::calculate(&size.to_le_bytes(), 0xC0C1);
        section.extend_from_slice(&crc.to_le_bytes());
        section.extend_from_slice(&DwgSectionDefinition::CLASSES_END_SENTINEL);
        Ok(section)
    }

    // =========================================================================
    // Objects Section
    // =========================================================================

    /// Returns (objects_section_data, handle_to_offset_map)
    fn write_objects_section(doc: &CadDocument, version: ACadVersion) -> Result<(Vec<u8>, BTreeMap<u64, i64>)> {
        let mut section = Vec::new();
        let mut handle_map = BTreeMap::new();

        // Write table control objects
        Self::write_table_control_objects(&mut section, &mut handle_map, doc, version)?;
        // Write table entries
        Self::write_table_entries(&mut section, &mut handle_map, doc, version)?;
        // Write block entities
        Self::write_block_entities(&mut section, &mut handle_map, doc, version)?;
        // Write dictionary objects
        Self::write_dictionary_objects(&mut section, &mut handle_map, doc, version)?;
        // Write user entities
        for entity in doc.entities() {
            let handle = entity.as_entity().handle();
            if handle.is_null() { continue; }
            let offset = section.len() as i64;
            Self::write_entity_object(&mut section, entity, doc, version)?;
            handle_map.insert(handle.value(), offset);
        }

        Ok((section, handle_map))
    }

    fn write_table_control_objects(
        section: &mut Vec<u8>, map: &mut BTreeMap<u64, i64>,
        doc: &CadDocument, version: ACadVersion,
    ) -> Result<()> {
        let hdr = &doc.header;

        Self::write_table_control(section, map, hdr.layer_control_handle.value(),
            0x32, doc.layers.len() as i32,
            doc.layers.iter().map(|l| l.handle.value()).collect(), version)?;
        Self::write_table_control(section, map, hdr.linetype_control_handle.value(),
            0x38, doc.line_types.len() as i32,
            doc.line_types.iter().map(|l| l.handle.value()).collect(), version)?;
        Self::write_table_control(section, map, hdr.style_control_handle.value(),
            0x34, doc.text_styles.len() as i32,
            doc.text_styles.iter().map(|s| s.handle.value()).collect(), version)?;
        Self::write_table_control(section, map, hdr.block_control_handle.value(),
            0x30, doc.block_records.len() as i32,
            doc.block_records.iter().map(|b| b.handle.value()).collect(), version)?;
        Self::write_table_control(section, map, hdr.dimstyle_control_handle.value(),
            0x44, doc.dim_styles.len() as i32,
            doc.dim_styles.iter().map(|d| d.handle.value()).collect(), version)?;
        Self::write_table_control(section, map, hdr.vport_control_handle.value(),
            0x40, doc.vports.len() as i32,
            doc.vports.iter().map(|v| v.handle.value()).collect(), version)?;
        Self::write_table_control(section, map, hdr.appid_control_handle.value(),
            0x42, doc.app_ids.len() as i32,
            doc.app_ids.iter().map(|a| a.handle.value()).collect(), version)?;
        Self::write_table_control(section, map, hdr.view_control_handle.value(),
            0x3C, doc.views.len() as i32,
            doc.views.iter().map(|v| v.handle.value()).collect(), version)?;
        Self::write_table_control(section, map, hdr.ucs_control_handle.value(),
            0x3E, doc.ucss.len() as i32,
            doc.ucss.iter().map(|u| u.handle.value()).collect(), version)?;
        Ok(())
    }

    fn write_table_control(
        section: &mut Vec<u8>, map: &mut BTreeMap<u64, i64>,
        handle: u64, object_type: i16, count: i32,
        entry_handles: Vec<u64>, version: ACadVersion,
    ) -> Result<()> {
        if handle == 0 { return Ok(()); }
        let offset = section.len() as i64;

        let mut w = DwgStreamWriter::new(version);
        w.write_object_type(object_type);
        if version >= ACadVersion::AC1015 && version < ACadVersion::AC1024 {
            w.write_raw_long(0); // size_in_bits placeholder
        }
        w.write_handle(handle);
        w.write_bitshort(0); // EED size
        w.write_bitlong(0); // reactors
        if version >= ACadVersion::AC1018 {
            w.write_bit(false); // no xdictionary
        }
        w.write_bitlong(count);
        for &entry_h in &entry_handles {
            w.write_handle_reference(DwgReferenceType::SoftOwnership, entry_h);
        }

        let data = w.into_bytes();
        Self::write_object_wrapper(section, &data)?;
        map.insert(handle, offset);
        Ok(())
    }

    fn write_table_entries(
        section: &mut Vec<u8>, map: &mut BTreeMap<u64, i64>,
        doc: &CadDocument, version: ACadVersion,
    ) -> Result<()> {
        for layer in doc.layers.iter() {
            let offset = section.len() as i64;
            let data = Self::write_layer_entry(layer, &doc.header, version)?;
            Self::write_object_wrapper(section, &data)?;
            map.insert(layer.handle.value(), offset);
        }
        for ltype in doc.line_types.iter() {
            let offset = section.len() as i64;
            let data = Self::write_linetype_entry(ltype, &doc.header, version)?;
            Self::write_object_wrapper(section, &data)?;
            map.insert(ltype.handle.value(), offset);
        }
        for style in doc.text_styles.iter() {
            let offset = section.len() as i64;
            let data = Self::write_textstyle_entry(style, &doc.header, version)?;
            Self::write_object_wrapper(section, &data)?;
            map.insert(style.handle.value(), offset);
        }
        for block_rec in doc.block_records.iter() {
            let offset = section.len() as i64;
            let data = Self::write_block_record_entry(block_rec, doc, version)?;
            Self::write_object_wrapper(section, &data)?;
            map.insert(block_rec.handle.value(), offset);
        }
        for dimstyle in doc.dim_styles.iter() {
            let offset = section.len() as i64;
            let data = Self::write_dimstyle_entry(dimstyle, &doc.header, version)?;
            Self::write_object_wrapper(section, &data)?;
            map.insert(dimstyle.handle.value(), offset);
        }
        for appid in doc.app_ids.iter() {
            let offset = section.len() as i64;
            let data = Self::write_appid_entry(appid, &doc.header, version)?;
            Self::write_object_wrapper(section, &data)?;
            map.insert(appid.handle.value(), offset);
        }
        for vport in doc.vports.iter() {
            let offset = section.len() as i64;
            let data = Self::write_vport_entry(vport, &doc.header, version)?;
            Self::write_object_wrapper(section, &data)?;
            map.insert(vport.handle.value(), offset);
        }
        Ok(())
    }

    /// Write a single object as: MS(size) + data_bytes + CRC16
    fn write_object_wrapper(section: &mut Vec<u8>, object_data: &[u8]) -> Result<()> {
        let size = object_data.len() as u32;
        let mut size_bytes = Vec::new();
        if size >= 0x8000 {
            size_bytes.push((size & 0xFF) as u8);
            size_bytes.push((((size >> 8) & 0x7F) | 0x80) as u8);
            size_bytes.push(((size >> 15) & 0xFF) as u8);
            size_bytes.push(((size >> 23) & 0xFF) as u8);
        } else {
            size_bytes.push((size & 0xFF) as u8);
            size_bytes.push(((size >> 8) & 0xFF) as u8);
        }
        section.extend_from_slice(&size_bytes);

        let mut crc = Crc8::new(0xC0C1);
        crc.update_slice(&size_bytes);
        crc.update_slice(object_data);
        section.extend_from_slice(object_data);
        section.extend_from_slice(&crc.value().to_le_bytes());
        Ok(())
    }

    fn write_layer_entry(layer: &Layer, hdr: &HeaderVariables, version: ACadVersion) -> Result<Vec<u8>> {
        let mut w = DwgStreamWriter::new(version);
        w.write_object_type(0x33); // LAYER
        if version >= ACadVersion::AC1015 && version < ACadVersion::AC1024 {
            w.write_raw_long(0);
        }
        w.write_handle(layer.handle.value());
        w.write_bitshort(0); // EED
        w.write_bitlong(0); // reactors
        if version >= ACadVersion::AC1018 { w.write_bit(false); }

        w.write_variable_text(&layer.name);
        w.write_bit(false);
        w.write_bitshort(0);
        if version >= ACadVersion::AC1015 { w.write_bit(false); }

        let flags: i16 = if layer.is_frozen() { 0x01 } else { 0 }
            | if layer.is_off() { 0x02 } else { 0 }
            | if layer.is_locked() { 0x04 } else { 0 };

        if version >= ACadVersion::AC1015 {
            w.write_bitshort(flags);
        } else {
            w.write_bitshort(flags);
        }

        w.write_cmc_color(&layer.color);

        w.write_handle_reference(DwgReferenceType::HardOwnership, hdr.layer_control_handle.value());
        w.write_handle_reference(DwgReferenceType::SoftPointer, hdr.continuous_linetype_handle.value());

        if version >= ACadVersion::AC1015 {
            w.write_handle_reference(DwgReferenceType::SoftPointer, 0);
            w.write_handle_reference(DwgReferenceType::SoftPointer, 0);
        }
        if version >= ACadVersion::AC1021 {
            w.write_handle_reference(DwgReferenceType::SoftPointer, 0);
            w.write_handle_reference(DwgReferenceType::SoftPointer, 0);
        }
        Ok(w.into_bytes())
    }

    fn write_linetype_entry(ltype: &LineType, hdr: &HeaderVariables, version: ACadVersion) -> Result<Vec<u8>> {
        let mut w = DwgStreamWriter::new(version);
        w.write_object_type(0x39);
        if version >= ACadVersion::AC1015 && version < ACadVersion::AC1024 { w.write_raw_long(0); }
        w.write_handle(ltype.handle.value());
        w.write_bitshort(0);
        w.write_bitlong(0);
        if version >= ACadVersion::AC1018 { w.write_bit(false); }

        w.write_variable_text(&ltype.name);
        w.write_bit(false);
        w.write_bitshort(0);
        if version >= ACadVersion::AC1015 { w.write_bit(false); }

        w.write_variable_text(&ltype.description);
        w.write_bitdouble(ltype.pattern_length);
        w.write_byte(b'A'); // alignment
        w.write_byte(ltype.elements.len() as u8);

        for elem in &ltype.elements {
            w.write_bitdouble(elem.length);
            w.write_bitshort(0); // complex type
            w.write_bitshort(0); // shape number
            w.write_2raw_double(Vector2 { x: 0.0, y: 0.0 });
            w.write_bitdouble(0.0); // scale
            w.write_bitdouble(0.0); // rotation
            w.write_bitshort(0); // shape flag
        }

        w.write_handle_reference(DwgReferenceType::HardOwnership, hdr.linetype_control_handle.value());
        Ok(w.into_bytes())
    }

    fn write_textstyle_entry(style: &TextStyle, hdr: &HeaderVariables, version: ACadVersion) -> Result<Vec<u8>> {
        let mut w = DwgStreamWriter::new(version);
        w.write_object_type(0x35);
        if version >= ACadVersion::AC1015 && version < ACadVersion::AC1024 { w.write_raw_long(0); }
        w.write_handle(style.handle.value());
        w.write_bitshort(0);
        w.write_bitlong(0);
        if version >= ACadVersion::AC1018 { w.write_bit(false); }

        w.write_variable_text(&style.name);
        w.write_bit(false);
        w.write_bitshort(0);
        if version >= ACadVersion::AC1015 { w.write_bit(false); }

        w.write_bit(false); // shape file
        w.write_bit(false); // vertical
        w.write_bitdouble(style.height);
        w.write_bitdouble(style.width_factor);
        w.write_bitdouble(style.oblique_angle);
        w.write_byte(0); // text gen flags
        w.write_bitdouble(style.height);
        w.write_variable_text(&style.font_file);
        w.write_variable_text(&style.big_font_file);

        w.write_handle_reference(DwgReferenceType::HardOwnership, hdr.style_control_handle.value());
        Ok(w.into_bytes())
    }

    fn write_block_record_entry(block_rec: &BlockRecord, doc: &CadDocument, version: ACadVersion) -> Result<Vec<u8>> {
        let mut w = DwgStreamWriter::new(version);
        w.write_object_type(0x31);
        if version >= ACadVersion::AC1015 && version < ACadVersion::AC1024 { w.write_raw_long(0); }
        w.write_handle(block_rec.handle.value());
        w.write_bitshort(0);
        w.write_bitlong(0);
        if version >= ACadVersion::AC1018 { w.write_bit(false); }

        w.write_variable_text(&block_rec.name);
        w.write_bit(false);
        w.write_bitshort(0);
        if version >= ACadVersion::AC1015 { w.write_bit(false); }

        if version >= ACadVersion::AC1015 {
            w.write_bitshort(0); // insert count
        }
        if version >= ACadVersion::AC1018 {
            w.write_bitshort(if block_rec.explodable { 1 } else { 0 });
            w.write_byte(if block_rec.scale_uniformly { 1 } else { 0 });
        }

        w.write_handle_reference(DwgReferenceType::HardOwnership, doc.header.block_control_handle.value());
        w.write_handle_reference(DwgReferenceType::SoftPointer, block_rec.block_entity_handle.value());

        if version <= ACadVersion::AC1015 {
            w.write_handle(0);
            w.write_handle(0);
        }

        w.write_handle_reference(DwgReferenceType::SoftPointer, block_rec.block_end_handle.value());

        if version >= ACadVersion::AC1018 {
            if block_rec.name == "*Model_Space" || block_rec.name == "*Paper_Space" {
                for entity in doc.entities() {
                    w.write_handle_reference(DwgReferenceType::HardOwnership, entity.as_entity().handle().value());
                }
            }
        }

        if version >= ACadVersion::AC1015 {
            w.write_handle_reference(DwgReferenceType::SoftPointer, block_rec.layout.value());
        }

        Ok(w.into_bytes())
    }

    fn write_dimstyle_entry(dimstyle: &DimStyle, hdr: &HeaderVariables, version: ACadVersion) -> Result<Vec<u8>> {
        let mut w = DwgStreamWriter::new(version);
        w.write_object_type(0x45);
        if version >= ACadVersion::AC1015 && version < ACadVersion::AC1024 { w.write_raw_long(0); }
        w.write_handle(dimstyle.handle.value());
        w.write_bitshort(0);
        w.write_bitlong(0);
        if version >= ACadVersion::AC1018 { w.write_bit(false); }

        w.write_variable_text(&dimstyle.name);
        w.write_bit(false);
        w.write_bitshort(0);
        if version >= ACadVersion::AC1015 { w.write_bit(false); }

        w.write_bitshort(0); // flags

        w.write_handle_reference(DwgReferenceType::HardOwnership, hdr.dimstyle_control_handle.value());
        Ok(w.into_bytes())
    }

    fn write_appid_entry(appid: &AppId, hdr: &HeaderVariables, version: ACadVersion) -> Result<Vec<u8>> {
        let mut w = DwgStreamWriter::new(version);
        w.write_object_type(0x43);
        if version >= ACadVersion::AC1015 && version < ACadVersion::AC1024 { w.write_raw_long(0); }
        w.write_handle(appid.handle.value());
        w.write_bitshort(0);
        w.write_bitlong(0);
        if version >= ACadVersion::AC1018 { w.write_bit(false); }

        w.write_variable_text(&appid.name);
        w.write_bit(false);
        w.write_bitshort(0);
        if version >= ACadVersion::AC1015 { w.write_bit(false); }

        w.write_byte(0); // unknown RC

        w.write_handle_reference(DwgReferenceType::HardOwnership, hdr.appid_control_handle.value());
        Ok(w.into_bytes())
    }

    fn write_vport_entry(vport: &VPort, hdr: &HeaderVariables, version: ACadVersion) -> Result<Vec<u8>> {
        let mut w = DwgStreamWriter::new(version);
        w.write_object_type(0x41);
        if version >= ACadVersion::AC1015 && version < ACadVersion::AC1024 { w.write_raw_long(0); }
        w.write_handle(vport.handle.value());
        w.write_bitshort(0);
        w.write_bitlong(0);
        if version >= ACadVersion::AC1018 { w.write_bit(false); }

        w.write_variable_text(&vport.name);
        w.write_bit(false);
        w.write_bitshort(0);
        if version >= ACadVersion::AC1015 { w.write_bit(false); }

        w.write_bitdouble(vport.view_height);
        w.write_bitdouble(vport.aspect_ratio);
        w.write_2bitdouble(vport.view_center);
        w.write_3bitdouble(vport.view_target);
        w.write_3bitdouble(vport.view_direction);
        w.write_bitdouble(0.0); // view twist
        w.write_bitdouble(vport.lens_length);
        w.write_bitdouble(0.0); // front clip
        w.write_bitdouble(0.0); // back clip

        w.write_bit(false); // perspective
        w.write_bit(false); // front clip on
        w.write_bit(false); // back clip on
        w.write_bit(false); // ucs icon on
        w.write_bit(false); // ucs follow

        w.write_bitshort(0); // snap on
        w.write_bitshort(0); // grid on
        w.write_bitshort(0); // snap style
        w.write_bitshort(0); // snap isopair
        w.write_bitdouble(0.0); // snap rotation
        w.write_2bitdouble(Vector2 { x: 0.0, y: 0.0 });
        w.write_2bitdouble(vport.snap_spacing);
        w.write_2bitdouble(vport.grid_spacing);
        w.write_bitshort(100); // circle zoom

        if version >= ACadVersion::AC1015 {
            w.write_bitshort(0); // grid major
        }

        w.write_handle_reference(DwgReferenceType::HardOwnership, hdr.vport_control_handle.value());
        Ok(w.into_bytes())
    }

    fn write_block_entities(
        section: &mut Vec<u8>, map: &mut BTreeMap<u64, i64>,
        doc: &CadDocument, version: ACadVersion,
    ) -> Result<()> {
        for block_rec in doc.block_records.iter() {
            if !block_rec.block_entity_handle.is_null() {
                let offset = section.len() as i64;
                let mut w = DwgStreamWriter::new(version);
                Self::write_entity_common_data(&mut w, block_rec.block_entity_handle,
                    0x04, &Color::ByBlock, "0", version);
                w.write_variable_text(&block_rec.name);
                let data = w.into_bytes();
                Self::write_object_wrapper(section, &data)?;
                map.insert(block_rec.block_entity_handle.value(), offset);
            }
            if !block_rec.block_end_handle.is_null() {
                let offset = section.len() as i64;
                let mut w = DwgStreamWriter::new(version);
                Self::write_entity_common_data(&mut w, block_rec.block_end_handle,
                    0x05, &Color::ByBlock, "0", version);
                let data = w.into_bytes();
                Self::write_object_wrapper(section, &data)?;
                map.insert(block_rec.block_end_handle.value(), offset);
            }
        }
        Ok(())
    }

    fn write_dictionary_objects(
        section: &mut Vec<u8>, map: &mut BTreeMap<u64, i64>,
        doc: &CadDocument, version: ACadVersion,
    ) -> Result<()> {
        let dict_handle = doc.header.named_objects_dict_handle;
        if !dict_handle.is_null() {
            let offset = section.len() as i64;
            let mut w = DwgStreamWriter::new(version);
            w.write_object_type(0x2C); // DICTIONARY
            if version >= ACadVersion::AC1015 && version < ACadVersion::AC1024 { w.write_raw_long(0); }
            w.write_handle(dict_handle.value());
            w.write_bitshort(0);
            w.write_bitlong(0);
            if version >= ACadVersion::AC1018 { w.write_bit(false); }

            let mut entries: Vec<(&str, u64)> = Vec::new();
            if !doc.header.acad_group_dict_handle.is_null() {
                entries.push(("ACAD_GROUP", doc.header.acad_group_dict_handle.value()));
            }
            if !doc.header.acad_mlinestyle_dict_handle.is_null() {
                entries.push(("ACAD_MLINESTYLE", doc.header.acad_mlinestyle_dict_handle.value()));
            }
            if !doc.header.acad_layout_dict_handle.is_null() {
                entries.push(("ACAD_LAYOUT", doc.header.acad_layout_dict_handle.value()));
            }
            if !doc.header.acad_plotsettings_dict_handle.is_null() {
                entries.push(("ACAD_PLOTSETTINGS", doc.header.acad_plotsettings_dict_handle.value()));
            }
            if !doc.header.acad_plotstylename_dict_handle.is_null() {
                entries.push(("ACAD_PLOTSTYLENAME", doc.header.acad_plotstylename_dict_handle.value()));
            }

            w.write_bitlong(entries.len() as i32);
            if version >= ACadVersion::AC1014 { w.write_bitshort(0); }
            if version >= ACadVersion::AC1015 { w.write_byte(0); }

            for (name, handle) in &entries {
                w.write_variable_text(name);
                w.write_handle_reference(DwgReferenceType::SoftOwnership, *handle);
            }

            let data = w.into_bytes();
            Self::write_object_wrapper(section, &data)?;
            map.insert(dict_handle.value(), offset);

            // Sub-dictionaries (empty stubs)
            for (_, handle) in entries {
                if handle != dict_handle.value() {
                    let sub_offset = section.len() as i64;
                    let mut sw = DwgStreamWriter::new(version);
                    sw.write_object_type(0x2C);
                    if version >= ACadVersion::AC1015 && version < ACadVersion::AC1024 { sw.write_raw_long(0); }
                    sw.write_handle(handle);
                    sw.write_bitshort(0);
                    sw.write_bitlong(0);
                    if version >= ACadVersion::AC1018 { sw.write_bit(false); }
                    sw.write_bitlong(0);
                    if version >= ACadVersion::AC1014 { sw.write_bitshort(0); }
                    if version >= ACadVersion::AC1015 { sw.write_byte(0); }
                    let sub_data = sw.into_bytes();
                    Self::write_object_wrapper(section, &sub_data)?;
                    map.insert(handle, sub_offset);
                }
            }
        }
        Ok(())
    }

    // =========================================================================
    // Entity Writing
    // =========================================================================

    fn write_entity_common_data(
        w: &mut DwgStreamWriter, handle: Handle, object_type: i16,
        color: &Color, _layer: &str, version: ACadVersion,
    ) {
        w.write_object_type(object_type);
        if version >= ACadVersion::AC1015 && version < ACadVersion::AC1024 {
            w.write_raw_long(0); // size_in_bits placeholder
        }
        w.write_handle(handle.value());
        w.write_bitshort(0); // EED
        w.write_bit(false); // no graphic
        w.write_2bits(2); // entity mode = model space
        w.write_bitlong(0); // reactors
        if version >= ACadVersion::AC1018 { w.write_bit(false); }

        if version <= ACadVersion::AC1014 {
            w.write_bit(true); // isbylayerlt
            w.write_bit(true); // nolinks
        }

        if version >= ACadVersion::AC1018 {
            w.write_en_color(color, 0xFF);
        } else {
            w.write_color_by_index(color);
        }

        w.write_bitdouble(1.0); // linetype scale
        if version >= ACadVersion::AC1015 {
            w.write_2bits(0); // linetype flags ByLayer
            w.write_2bits(0); // plotstyle flags
        }
        if version >= ACadVersion::AC1021 {
            w.write_2bits(0); // material
            w.write_2bits(0); // shadow
        }
        w.write_bitshort(0); // invisibility
        if version >= ACadVersion::AC1015 {
            w.write_byte(0); // lineweight ByLayer
        }
    }

    fn write_entity_handle_data(
        w: &mut DwgStreamWriter, entity: &dyn Entity, doc: &CadDocument, version: ACadVersion,
    ) {
        let layer_handle = doc.layers.get(entity.layer())
            .map(|l| l.handle.value())
            .unwrap_or(doc.header.current_layer_handle.value());
        w.write_handle_reference(DwgReferenceType::SoftPointer, layer_handle);

        if version <= ACadVersion::AC1014 {
            w.write_handle_reference(DwgReferenceType::SoftPointer, doc.header.bylayer_linetype_handle.value());
        }
        if version <= ACadVersion::AC1015 {
            w.write_handle(0);
            w.write_handle(0);
        }
    }

    /// Write the common dimension data (shared by all dimension types)
    fn write_dim_common(w: &mut DwgStreamWriter, base: &DimensionBase, version: ACadVersion) {
        // R2010+: version byte
        if version >= ACadVersion::AC1024 {
            w.write_byte(base.version);
        }
        // Extrusion
        w.write_3bitdouble(base.normal);
        // Text midpoint (2RD)
        w.write_2raw_double(Vector2 { x: base.text_middle_point.x, y: base.text_middle_point.y });
        // Elevation
        w.write_bitdouble(base.text_middle_point.z);
        // Flags
        w.write_byte(0); // flags1
        // User text
        w.write_variable_text(base.user_text.as_deref().unwrap_or(""));
        // Text rotation
        w.write_bitdouble(base.text_rotation);
        // Horizontal direction
        w.write_bitdouble(base.horizontal_direction);
        // Insert scale (used for dimension block insert)
        w.write_bitdouble(base.insertion_point.x);
        w.write_bitdouble(base.insertion_point.y);
        w.write_bitdouble(base.insertion_point.z);
        // Insert rotation
        w.write_bitdouble(0.0);
        // R2000+: attachment_point, linespacing_style, linespacing_factor, actual_measurement
        if version >= ACadVersion::AC1015 {
            w.write_bitshort(base.attachment_point as i16);
            w.write_bitshort(1); // linespacing_style
            w.write_bitdouble(base.line_spacing_factor);
            w.write_bitdouble(base.actual_measurement);
        }
        // R2007+: unknown bits
        if version >= ACadVersion::AC1021 {
            w.write_bit(false); // unknown 73
            w.write_bit(false); // flip_arrow1
            w.write_bit(false); // flip_arrow2
        }
        // Clone insertion point (2RD)
        w.write_2raw_double(Vector2 { x: base.insertion_point.x, y: base.insertion_point.y });
    }

    /// Write the common dimension handles (shared by all dimension types)
    fn write_dim_handles(w: &mut DwgStreamWriter, entity: &dyn Entity, base: &DimensionBase, doc: &CadDocument, version: ACadVersion) {
        Self::write_entity_handle_data(w, entity, doc, version);
        // Dimension style handle
        w.write_handle_reference(DwgReferenceType::HardPointer, 0); // dimstyle
        // Anonymous block handle
        let block_handle = doc.block_records.get(&base.block_name)
            .map(|b| b.handle.value())
            .unwrap_or(0);
        w.write_handle_reference(DwgReferenceType::HardPointer, block_handle);
    }

    fn write_entity_object(section: &mut Vec<u8>, entity: &EntityType, doc: &CadDocument, version: ACadVersion) -> Result<()> {
        let mut w = DwgStreamWriter::new(version);
        let ent = entity.as_entity();

        match entity {
            EntityType::Line(line) => {
                Self::write_entity_common_data(&mut w, ent.handle(), 0x13, &ent.color(), ent.layer(), version);
                if version >= ACadVersion::AC1015 {
                    let z_zero = line.start.z == 0.0 && line.end.z == 0.0;
                    w.write_bit(z_zero);
                    w.write_raw_double(line.start.x);
                    w.write_2bitdouble_with_default(
                        Vector2 { x: line.start.x, y: 0.0 },
                        Vector2 { x: line.end.x, y: line.end.y },
                    );
                    w.write_raw_double(line.start.y);
                    if !z_zero {
                        w.write_bitdouble(line.start.z);
                        w.write_bitdouble(line.end.z);
                    }
                } else {
                    w.write_3bitdouble(line.start);
                    w.write_3bitdouble(line.end);
                }
                w.write_bit_thickness(line.thickness);
                w.write_bit_extrusion(line.normal);
                Self::write_entity_handle_data(&mut w, ent, doc, version);
            }
            EntityType::Circle(circle) => {
                Self::write_entity_common_data(&mut w, ent.handle(), 0x12, &ent.color(), ent.layer(), version);
                w.write_3bitdouble(circle.center);
                w.write_bitdouble(circle.radius);
                w.write_bit_thickness(circle.thickness);
                w.write_bit_extrusion(circle.normal);
                Self::write_entity_handle_data(&mut w, ent, doc, version);
            }
            EntityType::Arc(arc) => {
                Self::write_entity_common_data(&mut w, ent.handle(), 0x11, &ent.color(), ent.layer(), version);
                w.write_3bitdouble(arc.center);
                w.write_bitdouble(arc.radius);
                w.write_bit_thickness(arc.thickness);
                w.write_bit_extrusion(arc.normal);
                w.write_bitdouble(arc.start_angle);
                w.write_bitdouble(arc.end_angle);
                Self::write_entity_handle_data(&mut w, ent, doc, version);
            }
            EntityType::Point(pt) => {
                Self::write_entity_common_data(&mut w, ent.handle(), 0x1B, &ent.color(), ent.layer(), version);
                w.write_3bitdouble(pt.location);
                w.write_bit_thickness(pt.thickness);
                w.write_bit_extrusion(pt.normal);
                w.write_bitdouble(0.0); // x_axis_angle
                Self::write_entity_handle_data(&mut w, ent, doc, version);
            }
            EntityType::Ellipse(ellipse) => {
                Self::write_entity_common_data(&mut w, ent.handle(), 0x23, &ent.color(), ent.layer(), version);
                w.write_3bitdouble(ellipse.center);
                w.write_3bitdouble(ellipse.major_axis);
                w.write_3bitdouble(ellipse.normal);
                w.write_bitdouble(ellipse.minor_axis_ratio);
                w.write_bitdouble(ellipse.start_parameter);
                w.write_bitdouble(ellipse.end_parameter);
                Self::write_entity_handle_data(&mut w, ent, doc, version);
            }
            EntityType::Text(text) => {
                Self::write_entity_common_data(&mut w, ent.handle(), 0x01, &ent.color(), ent.layer(), version);
                let align_pt = text.alignment_point.unwrap_or(text.insertion_point);
                if version >= ACadVersion::AC1015 {
                    w.write_byte(0); // data_flags = 0 (all fields present)
                    w.write_raw_double(0.0); // elevation
                    w.write_2raw_double(Vector2 { x: text.insertion_point.x, y: text.insertion_point.y });
                    w.write_2bitdouble_with_default(
                        Vector2 { x: text.insertion_point.x, y: text.insertion_point.y },
                        Vector2 { x: align_pt.x, y: align_pt.y },
                    );
                    w.write_bit_extrusion(text.normal);
                    w.write_bit_thickness(0.0);
                    w.write_bitdouble(text.oblique_angle);
                    w.write_bitdouble(text.rotation);
                    w.write_bitdouble(text.height);
                    w.write_bitdouble(text.width_factor);
                    w.write_variable_text(&text.value);
                    w.write_bitshort(0); // generation
                    w.write_bitshort(text.horizontal_alignment as i16);
                    w.write_bitshort(text.vertical_alignment as i16);
                } else {
                    w.write_bitdouble(0.0);
                    w.write_2raw_double(Vector2 { x: text.insertion_point.x, y: text.insertion_point.y });
                    w.write_2raw_double(Vector2 { x: align_pt.x, y: align_pt.y });
                    w.write_bit_extrusion(text.normal);
                    w.write_bit_thickness(0.0);
                    w.write_bitdouble(0.0);
                    w.write_bitdouble(text.rotation);
                    w.write_bitdouble(text.height);
                    w.write_bitdouble(text.width_factor);
                    w.write_variable_text(&text.value);
                    w.write_bitshort(0);
                    w.write_bitshort(text.horizontal_alignment as i16);
                    w.write_bitshort(text.vertical_alignment as i16);
                }
                Self::write_entity_handle_data(&mut w, ent, doc, version);
                w.write_handle_reference(DwgReferenceType::SoftPointer, doc.header.current_text_style_handle.value());
            }
            EntityType::MText(mtext) => {
                Self::write_entity_common_data(&mut w, ent.handle(), 0x2C + 12, &ent.color(), ent.layer(), version);
                w.write_3bitdouble(mtext.insertion_point);
                w.write_3bitdouble(mtext.normal);
                // X direction (derived from rotation)
                let cos_r = mtext.rotation.cos();
                let sin_r = mtext.rotation.sin();
                w.write_3bitdouble(Vector3 { x: cos_r, y: sin_r, z: 0.0 });
                w.write_bitdouble(mtext.rectangle_width);
                w.write_bitdouble(mtext.rectangle_height.unwrap_or(0.0));
                w.write_bitshort(mtext.attachment_point as i16);
                w.write_bitshort(mtext.drawing_direction as i16);
                if version >= ACadVersion::AC1021 {
                    w.write_bitdouble(mtext.rectangle_height.unwrap_or(0.0));
                    w.write_bitdouble(mtext.rectangle_width);
                }
                w.write_bitdouble(mtext.height);
                w.write_variable_text(&mtext.value);
                w.write_bitshort(0); // line spacing style
                w.write_bitdouble(mtext.line_spacing_factor);
                Self::write_entity_handle_data(&mut w, ent, doc, version);
                w.write_handle_reference(DwgReferenceType::SoftPointer, doc.header.current_text_style_handle.value());
            }
            EntityType::LwPolyline(lwpoly) => {
                // LWPOLYLINE is a class-based entity, type code depends on class definitions.
                // For DWG, it uses object type 501-502+ (varies). Simplified: write as basic.
                Self::write_entity_common_data(&mut w, ent.handle(), 0x4D, &ent.color(), ent.layer(), version);
                w.write_bitshort(if lwpoly.is_closed { 1 } else { 0 });

                if lwpoly.constant_width > 0.0 {
                    w.write_bitdouble(lwpoly.constant_width);
                }
                if lwpoly.elevation != 0.0 {
                    w.write_bitdouble(lwpoly.elevation);
                }
                if lwpoly.thickness != 0.0 {
                    w.write_bitdouble(lwpoly.thickness);
                }
                if lwpoly.normal.x != 0.0 || lwpoly.normal.y != 0.0 || (lwpoly.normal.z - 1.0).abs() > 1e-10 {
                    w.write_3bitdouble(lwpoly.normal);
                }

                w.write_bitlong(lwpoly.vertices.len() as i32);
                for v in &lwpoly.vertices {
                    w.write_2raw_double(v.location);
                }

                Self::write_entity_handle_data(&mut w, ent, doc, version);
            }
            EntityType::Solid(solid) => {
                Self::write_entity_common_data(&mut w, ent.handle(), 0x1F, &ent.color(), ent.layer(), version);
                w.write_bit_thickness(solid.thickness);
                w.write_bitdouble(solid.first_corner.z); // elevation
                w.write_2raw_double(Vector2 { x: solid.first_corner.x, y: solid.first_corner.y });
                w.write_2raw_double(Vector2 { x: solid.second_corner.x, y: solid.second_corner.y });
                w.write_2raw_double(Vector2 { x: solid.third_corner.x, y: solid.third_corner.y });
                w.write_2raw_double(Vector2 { x: solid.fourth_corner.x, y: solid.fourth_corner.y });
                w.write_bit_extrusion(solid.normal);
                Self::write_entity_handle_data(&mut w, ent, doc, version);
            }
            EntityType::Insert(insert) => {
                Self::write_entity_common_data(&mut w, ent.handle(), 0x07, &ent.color(), ent.layer(), version);
                w.write_3bitdouble(insert.insert_point);
                if version >= ACadVersion::AC1015 {
                    let x = insert.x_scale;
                    let y = insert.y_scale;
                    let z = insert.z_scale;
                    let flags: u8 = if (x - y).abs() < 1e-10 && (x - z).abs() < 1e-10 && (x - 1.0).abs() < 1e-10 {
                        3
                    } else if (x - 1.0).abs() < 1e-10 { 1 } else { 0 };
                    w.write_2bits(flags);
                    if flags < 3 { w.write_bitdouble_with_default(1.0, x); }
                    if flags == 0 {
                        w.write_bitdouble_with_default(x, y);
                        w.write_bitdouble_with_default(x, z);
                    }
                } else {
                    w.write_3bitdouble(Vector3 { x: insert.x_scale, y: insert.y_scale, z: insert.z_scale });
                }
                w.write_bitdouble(insert.rotation);
                w.write_bit_extrusion(insert.normal);
                w.write_bit(false); // has_attributes = false
                if version >= ACadVersion::AC1018 {
                    // owned object count not needed when no attributes
                }
                Self::write_entity_handle_data(&mut w, ent, doc, version);
                // Block header handle (lookup by name)
                let block_handle = doc.block_records.get(&insert.block_name)
                    .map(|b| b.handle.value())
                    .unwrap_or(0);
                w.write_handle_reference(DwgReferenceType::SoftPointer, block_handle);
            }
            EntityType::Face3D(face) => {
                Self::write_entity_common_data(&mut w, ent.handle(), 0x1C, &ent.color(), ent.layer(), version);
                if version >= ACadVersion::AC1015 {
                    let has_no_flags = face.invisible_edges.bits() == 0;
                    w.write_bit(has_no_flags);
                    let z_zero = face.first_corner.z == 0.0;
                    w.write_bit(z_zero);
                    w.write_3bitdouble(face.first_corner);
                    w.write_3bitdouble_with_default(face.first_corner, face.second_corner);
                    w.write_3bitdouble_with_default(face.second_corner, face.third_corner);
                    w.write_3bitdouble_with_default(face.third_corner, face.fourth_corner);
                    if !has_no_flags {
                        w.write_bitshort(face.invisible_edges.bits() as i16);
                    }
                } else {
                    w.write_3bitdouble(face.first_corner);
                    w.write_3bitdouble(face.second_corner);
                    w.write_3bitdouble(face.third_corner);
                    w.write_3bitdouble(face.fourth_corner);
                    w.write_bitshort(face.invisible_edges.bits() as i16);
                }
                Self::write_entity_handle_data(&mut w, ent, doc, version);
            }
            EntityType::Ray(ray) => {
                Self::write_entity_common_data(&mut w, ent.handle(), 0x28, &ent.color(), ent.layer(), version);
                w.write_3bitdouble(ray.base_point);
                w.write_3bitdouble(ray.direction);
                Self::write_entity_handle_data(&mut w, ent, doc, version);
            }
            EntityType::XLine(xline) => {
                Self::write_entity_common_data(&mut w, ent.handle(), 0x29, &ent.color(), ent.layer(), version);
                w.write_3bitdouble(xline.base_point);
                w.write_3bitdouble(xline.direction);
                Self::write_entity_handle_data(&mut w, ent, doc, version);
            }
            EntityType::Spline(spline) => {
                Self::write_entity_common_data(&mut w, ent.handle(), 0x24, &ent.color(), ent.layer(), version);
                // Scenario: 1=fit points, 2=control points
                let scenario: i16 = if !spline.fit_points.is_empty() && spline.control_points.is_empty() { 1 } else { 2 };
                w.write_bitshort(scenario);
                w.write_bitshort(spline.degree as i16);
                if scenario == 2 {
                    w.write_bitlong(spline.knots.len() as i32);
                    w.write_bitlong(spline.control_points.len() as i32);
                    let weighted = !spline.weights.is_empty();
                    w.write_bit(weighted);
                } else {
                    w.write_bitlong(spline.fit_points.len() as i32);
                }
                // Spline flags
                let mut flags: i16 = 0;
                if spline.flags.closed { flags |= 1; }
                if spline.flags.periodic { flags |= 2; }
                if spline.flags.rational { flags |= 4; }
                // Weight/knot tolerances
                w.write_bitdouble(1e-10); // knot tolerance
                w.write_bitdouble(1e-10); // control tolerance
                if scenario == 2 {
                    for k in &spline.knots {
                        w.write_bitdouble(*k);
                    }
                    for (i, cp) in spline.control_points.iter().enumerate() {
                        w.write_3bitdouble(*cp);
                        if !spline.weights.is_empty() {
                            let wt = spline.weights.get(i).copied().unwrap_or(1.0);
                            w.write_bitdouble(wt);
                        }
                    }
                } else {
                    for fp in &spline.fit_points {
                        w.write_3bitdouble(*fp);
                    }
                }
                Self::write_entity_handle_data(&mut w, ent, doc, version);
            }
            EntityType::Leader(leader) => {
                Self::write_entity_common_data(&mut w, ent.handle(), 0x2D, &ent.color(), ent.layer(), version);
                w.write_bit(false); // unknown bit
                w.write_bitshort(leader.creation_type.to_value());
                w.write_bitshort(leader.path_type.to_value());
                w.write_bitlong(leader.vertices.len() as i32);
                for v in &leader.vertices {
                    w.write_3bitdouble(*v);
                }
                w.write_3bitdouble(Vector3::ZERO); // origin (discarded on read)
                w.write_3bitdouble(leader.normal);
                w.write_3bitdouble(leader.horizontal_direction);
                w.write_3bitdouble(leader.block_offset);
                if version >= ACadVersion::AC1014 {
                    w.write_3bitdouble(leader.annotation_offset);
                }
                if version <= ACadVersion::AC1021 {
                    w.write_bitdouble(leader.text_height);
                    w.write_bitdouble(leader.text_width);
                }
                w.write_bit(leader.hookline_enabled);
                w.write_bit(leader.arrow_enabled);
                if version >= ACadVersion::AC1015 {
                    w.write_bitshort(0); // unknown
                    w.write_bit(false); // unknown
                    w.write_bit(false); // unknown
                }
                Self::write_entity_handle_data(&mut w, ent, doc, version);
                w.write_handle_reference(DwgReferenceType::SoftPointer, leader.annotation_handle.value());
                w.write_handle_reference(DwgReferenceType::HardPointer, 0); // dimstyle handle
            }
            EntityType::Tolerance(tol) => {
                Self::write_entity_common_data(&mut w, ent.handle(), 0x2E, &ent.color(), ent.layer(), version);
                w.write_3bitdouble(tol.insertion_point);
                w.write_3bitdouble(tol.direction);
                w.write_3bitdouble(tol.normal);
                w.write_variable_text(&tol.text);
                Self::write_entity_handle_data(&mut w, ent, doc, version);
                w.write_handle_reference(DwgReferenceType::HardPointer, 0); // dimstyle handle
            }
            EntityType::Shape(shape) => {
                Self::write_entity_common_data(&mut w, ent.handle(), 0x21, &ent.color(), ent.layer(), version);
                w.write_3bitdouble(shape.insertion_point);
                w.write_bitdouble(shape.size);
                w.write_bitdouble(shape.rotation);
                w.write_bitdouble(shape.relative_x_scale);
                w.write_bitdouble(shape.oblique_angle);
                w.write_bitdouble(shape.thickness);
                w.write_bitshort(shape.shape_number as i16);
                w.write_3bitdouble(shape.normal);
                Self::write_entity_handle_data(&mut w, ent, doc, version);
                w.write_handle_reference(DwgReferenceType::HardPointer, shape.style_handle.map_or(0, |h| h.value())); // style handle
            }
            EntityType::Dimension(dim) => {
                match dim {
                    Dimension::Aligned(d) => {
                        Self::write_entity_common_data(&mut w, ent.handle(), 0x16, &ent.color(), ent.layer(), version);
                        Self::write_dim_common(&mut w, &d.base, version);
                        w.write_3bitdouble(d.definition_point);
                        w.write_3bitdouble(d.first_point);
                        w.write_3bitdouble(d.second_point);
                        w.write_bitdouble(d.ext_line_rotation);
                        Self::write_dim_handles(&mut w, ent, &d.base, doc, version);
                    }
                    Dimension::Linear(d) => {
                        Self::write_entity_common_data(&mut w, ent.handle(), 0x15, &ent.color(), ent.layer(), version);
                        Self::write_dim_common(&mut w, &d.base, version);
                        w.write_3bitdouble(d.definition_point);
                        w.write_3bitdouble(d.first_point);
                        w.write_3bitdouble(d.second_point);
                        w.write_bitdouble(d.rotation);
                        w.write_bitdouble(d.ext_line_rotation);
                        Self::write_dim_handles(&mut w, ent, &d.base, doc, version);
                    }
                    Dimension::Radius(d) => {
                        Self::write_entity_common_data(&mut w, ent.handle(), 0x19, &ent.color(), ent.layer(), version);
                        Self::write_dim_common(&mut w, &d.base, version);
                        w.write_3bitdouble(d.definition_point);
                        w.write_bitdouble(d.leader_length);
                        Self::write_dim_handles(&mut w, ent, &d.base, doc, version);
                    }
                    Dimension::Diameter(d) => {
                        Self::write_entity_common_data(&mut w, ent.handle(), 0x1A, &ent.color(), ent.layer(), version);
                        Self::write_dim_common(&mut w, &d.base, version);
                        w.write_3bitdouble(d.definition_point);
                        w.write_bitdouble(d.leader_length);
                        Self::write_dim_handles(&mut w, ent, &d.base, doc, version);
                    }
                    Dimension::Angular3Pt(d) => {
                        Self::write_entity_common_data(&mut w, ent.handle(), 0x17, &ent.color(), ent.layer(), version);
                        Self::write_dim_common(&mut w, &d.base, version);
                        w.write_3bitdouble(d.definition_point);
                        w.write_3bitdouble(d.first_point);
                        w.write_3bitdouble(d.second_point);
                        w.write_3bitdouble(d.angle_vertex);
                        Self::write_dim_handles(&mut w, ent, &d.base, doc, version);
                    }
                    Dimension::Angular2Ln(d) => {
                        Self::write_entity_common_data(&mut w, ent.handle(), 0x18, &ent.color(), ent.layer(), version);
                        Self::write_dim_common(&mut w, &d.base, version);
                        w.write_raw_double(d.dimension_arc.x);
                        w.write_raw_double(d.dimension_arc.y);
                        w.write_3bitdouble(d.first_point);
                        w.write_3bitdouble(d.second_point);
                        w.write_3bitdouble(d.angle_vertex);
                        w.write_3bitdouble(d.definition_point);
                        Self::write_dim_handles(&mut w, ent, &d.base, doc, version);
                    }
                    Dimension::Ordinate(d) => {
                        Self::write_entity_common_data(&mut w, ent.handle(), 0x14, &ent.color(), ent.layer(), version);
                        Self::write_dim_common(&mut w, &d.base, version);
                        w.write_3bitdouble(d.definition_point);
                        w.write_3bitdouble(d.feature_location);
                        w.write_3bitdouble(d.leader_endpoint);
                        w.write_byte(if d.is_ordinate_type_x { 1 } else { 0 });
                        Self::write_dim_handles(&mut w, ent, &d.base, doc, version);
                    }
                }
            }
            _ => {
                return Ok(()); // Skip unsupported entity types
            }
        }

        let data = w.into_bytes();
        Self::write_object_wrapper(section, &data)?;
        Ok(())
    }

    // =========================================================================
    // Handles Section
    // =========================================================================

    fn write_handles_section(handle_map: &BTreeMap<u64, i64>) -> Result<Vec<u8>> {
        let mut section = Vec::new();
        let max_chunk_size = 2032;
        let mut prev_handle: u64 = 0;
        let mut prev_offset: i64 = 0;
        let mut chunk = Vec::new();
        chunk.push(0u8);
        chunk.push(0u8);

        for (&handle, &offset) in handle_map {
            let handle_delta = handle.wrapping_sub(prev_handle);
            let offset_delta = offset - prev_offset;

            Self::encode_modular_char(&mut chunk, handle_delta);
            Self::encode_signed_modular_char(&mut chunk, offset_delta);

            prev_handle = handle;
            prev_offset = offset;

            if chunk.len() >= max_chunk_size - 16 {
                Self::finalize_handle_chunk(&mut section, &mut chunk);
                chunk.push(0u8);
                chunk.push(0u8);
            }
        }

        if chunk.len() > 2 {
            Self::finalize_handle_chunk(&mut section, &mut chunk);
        }

        // Empty terminator
        section.push(0);
        section.push(0);
        let crc = Crc8::calculate(&[0, 0], 0xC0C1);
        // Handle section CRC is big-endian (high byte first)
        section.push((crc >> 8) as u8);
        section.push((crc & 0xFF) as u8);
        Ok(section)
    }

    fn finalize_handle_chunk(section: &mut Vec<u8>, chunk: &mut Vec<u8>) {
        let size = chunk.len() as u16;
        chunk[0] = (size >> 8) as u8;
        chunk[1] = (size & 0xFF) as u8;
        let crc = Crc8::calculate(chunk, 0xC0C1);
        section.extend_from_slice(chunk);
        // Handle section CRC is big-endian (high byte first)
        section.push((crc >> 8) as u8);
        section.push((crc & 0xFF) as u8);
        chunk.clear();
    }

    fn encode_modular_char(buf: &mut Vec<u8>, value: u64) {
        if value == 0 { buf.push(0); return; }
        let mut v = value;
        while v >= 0x80 {
            buf.push(((v & 0x7F) | 0x80) as u8);
            v >>= 7;
        }
        buf.push(v as u8);
    }

    fn encode_signed_modular_char(buf: &mut Vec<u8>, value: i64) {
        let negative = value < 0;
        let mut v = if negative { -value } else { value } as u64;
        while v >= 64 {
            buf.push(((v & 0x7F) | 0x80) as u8);
            v >>= 7;
        }
        let mut final_byte = (v & 0x3F) as u8;
        if negative { final_byte |= 0x40; }
        buf.push(final_byte);
    }

    // =========================================================================
    // Aux Header
    // =========================================================================

    fn write_aux_header() -> Result<Vec<u8>> {
        let mut data = Vec::new();
        data.push(0xFF);
        data.extend_from_slice(&0x0018u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&(-1i32).to_le_bytes());
        data.extend_from_slice(&1u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&0x0018u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&0x0018u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        while data.len() < 123 {
            data.push(0);
        }
        Ok(data)
    }
}

// =============================================================================
// WriteContext — AC18 page/section assembly
// =============================================================================

struct SectionInfo {
    name: String,
    descriptor: DwgSectionDescriptor,
}

struct PageRecord {
    page_number: i32,
    page_size: usize,
    offset_in_stream: usize,
}

struct WriteContext {
    version: ACadVersion,
    output: Vec<u8>,
    sections: Vec<SectionInfo>,
    page_records: Vec<PageRecord>,
    next_page_number: i32,
    compressor: Lz77AC18Compressor,
    magic_seq: [u8; 256],
}

impl WriteContext {
    fn new(version: ACadVersion) -> Self {
        let output = vec![0u8; 0x100]; // Reserve file header space
        Self {
            version,
            output,
            sections: Vec::new(),
            page_records: Vec::new(),
            next_page_number: 1,
            compressor: Lz77AC18Compressor::new(),
            magic_seq: magic_sequence(),
        }
    }

    fn add_section(&mut self, name: &str, data: &[u8], compress: bool, decomp_size: usize) -> Result<()> {
        if data.is_empty() { return Ok(()); }

        let mut descriptor = DwgSectionDescriptor {
            page_type: DATA_PAGE_TYPE as u64,
            name: name.to_string(),
            compressed_size: 0,
            page_count: 0,
            decompressed_size: decomp_size as u64,
            compressed_code: if compress { 2 } else { 1 },
            section_id: self.sections.len() as i32,
            encrypted: 0,
            hash_code: None,
            encoding: None,
            local_sections: Vec::new(),
        };

        let ds = if decomp_size == 0 { data.len() } else { decomp_size };
        let mut offset = 0;

        while offset < data.len() {
            let chunk_end = (offset + ds).min(data.len());
            let chunk = &data[offset..chunk_end];

            let mut padded = vec![0u8; ds];
            padded[..chunk.len()].copy_from_slice(chunk);

            let compressed = if compress {
                self.compressor.compress(&padded)
            } else {
                padded.clone()
            };

            let comp_padding = compression_padding(compressed.len());
            let page_size = 32 + compressed.len() + comp_padding;
            let oda_checksum = Self::adler_checksum(0, &compressed);
            let page_number = self.next_page_number;
            self.next_page_number += 1;

            self.write_alignment_padding();
            let stream_pos = self.output.len();

            // 32-byte data section header
            // Fields: type(4) section_id(4) comp_size(4) page_size(4) offset(8) checksum(4) ODA(4)
            let mut header = [0u8; 32];
            header[0..4].copy_from_slice(&DATA_PAGE_TYPE.to_le_bytes());
            header[4..8].copy_from_slice(&(descriptor.section_id as u32).to_le_bytes());
            header[8..12].copy_from_slice(&(compressed.len() as u32).to_le_bytes());
            header[12..16].copy_from_slice(&(ds as u32).to_le_bytes());
            header[16..24].copy_from_slice(&(offset as u64).to_le_bytes());
            header[24..28].copy_from_slice(&0u32.to_le_bytes()); // checksum placeholder
            header[28..32].copy_from_slice(&oda_checksum.to_le_bytes()); // ODA

            let hdr_checksum = Self::adler_checksum(oda_checksum, &header);
            header[24..28].copy_from_slice(&hdr_checksum.to_le_bytes());

            let mask = 0x4164536Bu32 ^ (stream_pos as u32);
            for i in (0..32).step_by(4) {
                let val = u32::from_le_bytes([header[i], header[i+1], header[i+2], header[i+3]]);
                let masked = val ^ mask;
                header[i..i+4].copy_from_slice(&masked.to_le_bytes());
            }

            self.output.extend_from_slice(&header);
            self.output.extend_from_slice(&compressed);
            for i in 0..comp_padding {
                self.output.push(self.magic_seq[i % 256]);
            }

            self.page_records.push(PageRecord {
                page_number, page_size, offset_in_stream: stream_pos,
            });

            descriptor.local_sections.push(DwgLocalSectionMap {
                page_number,
                offset: offset as u64,
                size: ds as u64,
                page_size: page_size as u64,
                compressed_size: compressed.len() as u64,
                checksum: 0, crc: 0,
            });

            descriptor.page_count += 1;
            descriptor.compressed_size += compressed.len() as u64;

            offset = chunk_end;
        }

        self.sections.push(SectionInfo { name: name.to_string(), descriptor });
        Ok(())
    }

    fn write_alignment_padding(&mut self) {
        let remainder = self.output.len() % 0x20;
        if remainder != 0 {
            let padding = 0x20 - remainder;
            for i in 0..padding {
                self.output.push(self.magic_seq[i % 256]);
            }
        }
    }

    fn finalize(mut self) -> Result<Vec<u8>> {
        // 1. Section map
        let section_map_data = self.build_section_map();
        let section_map_compressed = self.compressor.compress(&section_map_data);
        let section_map_page_number = self.next_page_number;
        self.next_page_number += 1;

        self.write_alignment_padding();
        let section_map_pos = self.output.len();

        let sm_comp_padding = compression_padding(section_map_compressed.len());
        {
            // Compute chained checksum: Adler32(0, header_with_cksum=0) → Adler32(result, compressed)
            let mut sm_header = [0u8; 20];
            sm_header[0..4].copy_from_slice(&SECTION_MAP_TYPE.to_le_bytes());
            sm_header[4..8].copy_from_slice(&(section_map_data.len() as u32).to_le_bytes());
            sm_header[8..12].copy_from_slice(&(section_map_compressed.len() as u32).to_le_bytes());
            sm_header[12..16].copy_from_slice(&2u32.to_le_bytes());
            sm_header[16..20].copy_from_slice(&0u32.to_le_bytes()); // checksum placeholder
            let cksum = Self::adler_checksum(0, &sm_header);
            let cksum = Self::adler_checksum(cksum, &section_map_compressed);
            sm_header[16..20].copy_from_slice(&cksum.to_le_bytes());
            self.output.extend_from_slice(&sm_header);
        }
        self.output.extend_from_slice(&section_map_compressed);
        for i in 0..sm_comp_padding {
            self.output.push(self.magic_seq[i % 256]);
        }

        self.page_records.push(PageRecord {
            page_number: section_map_page_number,
            page_size: 20 + section_map_compressed.len() + sm_comp_padding,
            offset_in_stream: section_map_pos,
        });

        // 2. Page map
        let page_map_data = self.build_page_map();
        let page_map_compressed = self.compressor.compress(&page_map_data);
        let _page_map_page_number = self.next_page_number;
        self.next_page_number += 1;

        self.write_alignment_padding();
        let page_map_pos = self.output.len();

        let pm_comp_padding = compression_padding(page_map_compressed.len());
        {
            // Compute chained checksum: Adler32(0, header_with_cksum=0) → Adler32(result, compressed)
            let mut pm_header = [0u8; 20];
            pm_header[0..4].copy_from_slice(&PAGE_MAP_TYPE.to_le_bytes());
            pm_header[4..8].copy_from_slice(&(page_map_data.len() as u32).to_le_bytes());
            pm_header[8..12].copy_from_slice(&(page_map_compressed.len() as u32).to_le_bytes());
            pm_header[12..16].copy_from_slice(&2u32.to_le_bytes());
            pm_header[16..20].copy_from_slice(&0u32.to_le_bytes()); // checksum placeholder
            let cksum = Self::adler_checksum(0, &pm_header);
            let cksum = Self::adler_checksum(cksum, &page_map_compressed);
            pm_header[16..20].copy_from_slice(&cksum.to_le_bytes());
            self.output.extend_from_slice(&pm_header);
        }
        self.output.extend_from_slice(&page_map_compressed);
        for i in 0..pm_comp_padding {
            self.output.push(self.magic_seq[i % 256]);
        }

        let last_page_id = self.next_page_number - 1;

        // 3. Second header copy at end
        let second_header_addr = self.output.len() as i64;
        let encrypted_header = self.build_encrypted_file_header(
            last_page_id, second_header_addr, second_header_addr,
            page_map_pos as i64, section_map_page_number, _page_map_page_number,
        );
        self.output.extend_from_slice(&encrypted_header);
        self.output.extend_from_slice(&self.magic_seq[236..256]);

        // 4. File header at offset 0
        self.write_file_header_at_zero(
            last_page_id, second_header_addr, second_header_addr,
            page_map_pos as i64, section_map_page_number, _page_map_page_number,
        );

        Ok(self.output)
    }

    fn build_section_map(&self) -> Vec<u8> {
        let mut data = Vec::new();
        let num_desc = self.sections.len() as u32;
        data.extend_from_slice(&num_desc.to_le_bytes());
        data.extend_from_slice(&0x02u32.to_le_bytes());
        data.extend_from_slice(&(DEFAULT_DECOMP_SIZE as u32).to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes());
        data.extend_from_slice(&num_desc.to_le_bytes());

        for section in &self.sections {
            let desc = &section.descriptor;
            data.extend_from_slice(&desc.compressed_size.to_le_bytes());
            data.extend_from_slice(&(desc.page_count as u32).to_le_bytes());
            data.extend_from_slice(&(desc.decompressed_size as u32).to_le_bytes());
            data.extend_from_slice(&1u32.to_le_bytes());
            data.extend_from_slice(&(desc.compressed_code as u32).to_le_bytes());
            data.extend_from_slice(&(desc.section_id as u32).to_le_bytes());
            data.extend_from_slice(&(desc.encrypted as u32).to_le_bytes());
            let mut name_buf = [0u8; 64];
            let name_bytes = desc.name.as_bytes();
            let copy_len = name_bytes.len().min(63);
            name_buf[..copy_len].copy_from_slice(&name_bytes[..copy_len]);
            data.extend_from_slice(&name_buf);

            for local in &desc.local_sections {
                data.extend_from_slice(&(local.page_number as u32).to_le_bytes());
                data.extend_from_slice(&(local.compressed_size as u32).to_le_bytes());
                data.extend_from_slice(&local.offset.to_le_bytes());
            }
        }
        data
    }

    fn build_page_map(&self) -> Vec<u8> {
        let mut data = Vec::new();
        for page in &self.page_records {
            data.extend_from_slice(&(page.page_number as u32).to_le_bytes());
            data.extend_from_slice(&(page.page_size as u32).to_le_bytes());
        }
        data
    }

    fn build_encrypted_file_header(
        &self, last_page_id: i32, _last_section_addr: i64,
        second_header_addr: i64, page_map_address: i64,
        section_map_id: i32, page_map_id: i32,
    ) -> Vec<u8> {
        let mut header = Vec::new();
        header.extend_from_slice(b"AcFssFcAJMB\0");
        header.extend_from_slice(&0u32.to_le_bytes());
        header.extend_from_slice(&0x6Cu32.to_le_bytes());
        header.extend_from_slice(&0x04u32.to_le_bytes());
        header.extend_from_slice(&0u32.to_le_bytes()); // root tree gap
        header.extend_from_slice(&0u32.to_le_bytes()); // left gap
        header.extend_from_slice(&0u32.to_le_bytes()); // right gap
        header.extend_from_slice(&1u32.to_le_bytes()); // unknown
        header.extend_from_slice(&(last_page_id as u32).to_le_bytes());
        header.extend_from_slice(&(second_header_addr as u64).to_le_bytes()); // last section addr
        header.extend_from_slice(&(second_header_addr as u64).to_le_bytes());
        header.extend_from_slice(&0u32.to_le_bytes()); // gap amount
        header.extend_from_slice(&(self.sections.len() as u32).to_le_bytes());
        header.extend_from_slice(&0x20u32.to_le_bytes());
        header.extend_from_slice(&0x80u32.to_le_bytes());
        header.extend_from_slice(&0x40u32.to_le_bytes());
        header.extend_from_slice(&(page_map_id as u32).to_le_bytes());
        let pma = (page_map_address - 0x100) as u64;
        header.extend_from_slice(&pma.to_le_bytes());
        header.extend_from_slice(&(section_map_id as u32).to_le_bytes());
        let saps = self.page_records.len() as u32 + 2;
        header.extend_from_slice(&saps.to_le_bytes());
        header.extend_from_slice(&0u32.to_le_bytes()); // gap array size

        while header.len() < 0x68 { header.push(0); }
        header.extend_from_slice(&0u32.to_le_bytes()); // CRC placeholder

        let crc = crc32_dwg(&header[..0x6C]);
        header[0x68..0x6C].copy_from_slice(&crc.to_le_bytes());

        let magic = magic_sequence();
        for i in 0..header.len() {
            header[i] ^= magic[i % 256];
        }
        header
    }

    fn write_file_header_at_zero(
        &mut self, last_page_id: i32, last_section_addr: i64,
        second_header_addr: i64, page_map_address: i64,
        section_map_id: i32, page_map_id: i32,
    ) {
        let version_str = match self.version {
            ACadVersion::AC1018 => b"AC1018",
            ACadVersion::AC1021 => b"AC1021",
            ACadVersion::AC1024 => b"AC1024",
            ACadVersion::AC1027 => b"AC1027",
            ACadVersion::AC1032 => b"AC1032",
            _ => b"AC1018",
        };

        self.output[0..6].copy_from_slice(version_str);
        self.output[6..11].fill(0);
        self.output[0x0B] = 0;
        self.output[0x0C] = 0x03;
        self.output[0x0D..0x11].copy_from_slice(&0u32.to_le_bytes());
        self.output[0x11] = 33; // R2004
        self.output[0x12] = 0;
        self.output[0x13..0x15].copy_from_slice(&30u16.to_le_bytes());
        self.output[0x15..0x18].fill(0);
        self.output[0x18..0x1C].copy_from_slice(&0u32.to_le_bytes());
        self.output[0x1C..0x20].copy_from_slice(&0u32.to_le_bytes());
        self.output[0x20..0x24].copy_from_slice(&0u32.to_le_bytes());
        self.output[0x24..0x28].copy_from_slice(&0u32.to_le_bytes());
        self.output[0x28..0x2C].copy_from_slice(&0x80u32.to_le_bytes());
        self.output[0x2C..0x30].copy_from_slice(&0u32.to_le_bytes());

        let encrypted = self.build_encrypted_file_header(
            last_page_id, last_section_addr, second_header_addr,
            page_map_address, section_map_id, page_map_id,
        );
        let copy_len = encrypted.len().min(0x80);
        self.output[0x80..0x80 + copy_len].copy_from_slice(&encrypted[..copy_len]);
    }

    fn adler_checksum(seed: u32, data: &[u8]) -> u32 {
        let mut sum1: u32 = seed & 0xFFFF;
        let mut sum2: u32 = seed >> 16;
        let mut index = 0;
        while index < data.len() {
            let chunk_end = (index + 0x15B0).min(data.len());
            while index < chunk_end {
                sum1 = sum1.wrapping_add(data[index] as u32);
                sum2 = sum2.wrapping_add(sum1);
                index += 1;
            }
            sum1 %= 0xFFF1;
            sum2 %= 0xFFF1;
        }
        (sum2 << 16) | (sum1 & 0xFFFF)
    }
}

fn crc32_dwg(data: &[u8]) -> u32 {
    Crc32::calculate(data)
}

impl Default for DwgWriter {
    fn default() -> Self { Self::new() }
}
