//! DWG header (drawing variable) section writer.
//!
//! This writes the CadHeader (HeaderVariables) section with version-conditional
//! blocks matching the DWG binary spec for R13 through R2018+.

use std::io::{Cursor, Read as StdRead, Seek, SeekFrom, Write};

use crate::document::HeaderVariables;
use crate::error::Result;
use crate::io::dwg::dwg_section_io::DwgSectionContext;
use crate::io::dwg::{crc8_value, DwgSectionDefinition, START_SENTINELS, END_SENTINELS};
use crate::types::{DxfVersion, Handle, Vector3};
use crate::io::dwg::dwg_stream_readers::idwg_stream_reader::DwgReferenceType;

use super::dwg_stream_writer_base::DwgStreamWriterBase;
use super::idwg_stream_writer::DwgStreamWriter;

pub struct DwgHeaderWriter;

impl DwgHeaderWriter {
    /// Write the header section, returning the raw bytes for embedding.
    pub fn write(version: DxfVersion, header: &HeaderVariables) -> Result<Vec<u8>> {
        let ctx = DwgSectionContext::new(version, DwgSectionDefinition::HEADER);

        let mut section_stream = Cursor::new(Vec::<u8>::new());
        let mut writer: Box<dyn DwgStreamWriter> = if ctx.r2007_plus {
            let w = DwgStreamWriterBase::get_merged_writer(version, Box::new(Cursor::new(Vec::new())), "windows-1252");
            w
        } else {
            DwgStreamWriterBase::get_stream_writer(version, Box::new(Cursor::new(Vec::new())), "windows-1252")
        };

        if ctx.r2007_plus {
            writer.save_position_for_size()?;
        }

        // R2013+:
        if ctx.r2013_plus {
            writer.write_bit_long_long(header.required_versions)?;
        }

        // Common: Unknown defaults
        writer.write_bit_double(412148564080.0)?;
        writer.write_bit_double(1.0)?;
        writer.write_bit_double(1.0)?;
        writer.write_bit_double(1.0)?;

        writer.write_variable_text("m")?;
        writer.write_variable_text("")?;
        writer.write_variable_text("")?;
        writer.write_variable_text("")?;

        writer.write_bit_long(24)?;
        writer.write_bit_long(0)?;

        // R13-R14 Only:
        if ctx.r13_14_only {
            writer.write_bit_short(0)?;
        }

        // Pre-2004: current viewport entity (null handle)
        if ctx.r2004_pre {
            writer.handle_reference(0)?; // null
        }

        // Common mode flags
        writer.write_bit(header.associate_dimensions)?;
        writer.write_bit(header.update_dimensions_while_dragging)?;

        if ctx.r13_14_only {
            writer.write_bit(false)?; // DIMSAV
        }

        writer.write_bit(header.polyline_linetype_generation)?;
        writer.write_bit(header.ortho_mode)?;
        writer.write_bit(header.regen_mode)?;
        writer.write_bit(header.fill_mode)?;
        writer.write_bit(header.quick_text_mode)?;
        writer.write_bit(header.paper_space_linetype_scaling)?; // PSLTSCALE
        writer.write_bit(header.limit_check)?;

        if ctx.r13_14_only {
            writer.write_bit(header.blip_mode)?;
        }

        if ctx.r2004_plus {
            writer.write_bit(false)?; // Undocumented
        }

        writer.write_bit(header.user_timer)?;
        writer.write_bit(header.spline_frame)?; // SKPOLY
        writer.write_bit(header.angle_direction != 0)?; // ANGDIR
        writer.write_bit(header.spline_frame)?; // SPLFRAME

        if ctx.r13_14_only {
            writer.write_bit(header.attribute_request)?; // ATTREQ
            writer.write_bit(header.attribute_dialog)?;  // ATTDIA
        }

        writer.write_bit(header.mirror_text)?;
        writer.write_bit(header.world_view)?;

        if ctx.r13_14_only {
            writer.write_bit(false)?; // WIREFRAME
        }

        writer.write_bit(header.show_model_space)?; // TILEMODE
        writer.write_bit(header.paper_space_limit_check)?;
        writer.write_bit(header.retain_xref_visibility)?;

        if ctx.r13_14_only {
            writer.write_bit(header.delete_objects)?; // DELOBJ
        }

        writer.write_bit(header.display_silhouette)?;
        writer.write_bit(false)?; // PELLIPSE
        writer.write_bit_short(header.proxy_graphics)?;

        if ctx.r13_14_only {
            writer.write_bit_short(header.drag_mode)?;
        }

        writer.write_bit_short(header.tree_depth)?;
        writer.write_bit_short(header.linear_unit_format)?;
        writer.write_bit_short(header.linear_unit_precision)?;
        writer.write_bit_short(header.angular_unit_format)?;
        writer.write_bit_short(header.angular_unit_precision)?;

        if ctx.r13_14_only {
            writer.write_bit_short(header.object_snap_mode as i16)?;
        }

        writer.write_bit_short(header.attribute_visibility)?;

        if ctx.r13_14_only {
            writer.write_bit_short(header.coords_mode)?;
        }

        writer.write_bit_short(header.point_display_mode)?;

        if ctx.r13_14_only {
            writer.write_bit_short(header.pick_style)?;
        }

        if ctx.r2004_plus {
            writer.write_bit_long(0)?;
            writer.write_bit_long(0)?;
            writer.write_bit_long(0)?;
        }

        // User short variables
        writer.write_bit_short(header.user_int1)?;
        writer.write_bit_short(header.user_int2)?;
        writer.write_bit_short(header.user_int3)?;
        writer.write_bit_short(header.user_int4)?;
        writer.write_bit_short(header.user_int5)?;

        writer.write_bit_short(header.spline_segments)?;
        writer.write_bit_short(header.surface_u_density)?;
        writer.write_bit_short(header.surface_v_density)?;
        writer.write_bit_short(header.surface_type)?;
        writer.write_bit_short(header.surface_tab1)?;
        writer.write_bit_short(header.surface_tab2)?;
        writer.write_bit_short(header.spline_type)?;
        writer.write_bit_short(header.shade_edge)?;
        writer.write_bit_short(header.shade_diffuse)?;
        writer.write_bit_short(0)?; // UNITMODE
        writer.write_bit_short(header.max_active_viewports)?;
        writer.write_bit_short(header.isolines)?;
        writer.write_bit_short(header.multiline_justification)?;
        writer.write_bit_short(header.text_quality)?;

        writer.write_bit_double(header.linetype_scale)?;
        writer.write_bit_double(header.text_height)?;
        writer.write_bit_double(header.trace_width)?;
        writer.write_bit_double(header.sketch_increment)?;
        writer.write_bit_double(header.fillet_radius)?;
        writer.write_bit_double(header.thickness)?;
        writer.write_bit_double(header.angle_base)?;
        writer.write_bit_double(header.point_display_size)?;
        writer.write_bit_double(header.polyline_width)?;
        writer.write_bit_double(header.user_real1)?;
        writer.write_bit_double(header.user_real2)?;
        writer.write_bit_double(header.user_real3)?;
        writer.write_bit_double(header.user_real4)?;
        writer.write_bit_double(header.user_real5)?;
        writer.write_bit_double(header.chamfer_distance_a)?;
        writer.write_bit_double(header.chamfer_distance_b)?;
        writer.write_bit_double(header.chamfer_length)?;
        writer.write_bit_double(header.chamfer_angle)?;
        writer.write_bit_double(header.facet_resolution)?;
        writer.write_bit_double(header.multiline_scale)?;
        writer.write_bit_double(header.current_entity_linetype_scale)?;

        writer.write_variable_text(&header.menu_name)?;

        // TDCREATE / TDUPDATE as BitLong pairs (Julian day, ms)
        // Approximate: store as raw doubles split
        let (c_jdate, c_ms) = julian_from_f64(header.create_date_julian);
        let (u_jdate, u_ms) = julian_from_f64(header.update_date_julian);
        writer.write_date_time(c_jdate, c_ms)?;
        writer.write_date_time(u_jdate, u_ms)?;

        if ctx.r2004_plus {
            writer.write_bit_long(0)?;
            writer.write_bit_long(0)?;
            writer.write_bit_long(0)?;
        }

        // TDINDWG / TDUSRTIMER
        let (te_days, te_ms) = julian_from_f64(header.total_editing_time);
        let (ue_days, ue_ms) = julian_from_f64(header.user_elapsed_time);
        writer.write_time_span(te_days, te_ms)?;
        writer.write_time_span(ue_days, ue_ms)?;

        // CECOLOR
        writer.write_cm_color(&header.current_entity_color)?;

        // HANDSEED — writes to the main stream (not handle)
        writer.handle_reference(header.handle_seed)?;

        // Handle references
        writer.handle_reference_typed(DwgReferenceType::HardPointer, header.current_layer_handle.value())?;
        writer.handle_reference_typed(DwgReferenceType::HardPointer, header.current_text_style_handle.value())?;
        writer.handle_reference_typed(DwgReferenceType::HardPointer, header.current_linetype_handle.value())?;

        if ctx.r2007_plus {
            // CMATERIAL
            writer.handle_reference_typed(DwgReferenceType::HardPointer, header.current_material_handle.value())?;
        }

        writer.handle_reference_typed(DwgReferenceType::HardPointer, header.current_dimstyle_handle.value())?;
        writer.handle_reference_typed(DwgReferenceType::HardPointer, header.current_multiline_style_handle.value())?;

        if ctx.r2000_plus {
            writer.write_bit_double(header.viewport_scale_factor)?;
        }

        // Paper space extents
        writer.write_3_bit_double(&header.paper_space_insertion_base)?;
        writer.write_3_bit_double(&header.paper_space_extents_min)?;
        writer.write_3_bit_double(&header.paper_space_extents_max)?;
        writer.write_2_raw_double(&header.paper_space_limits_min)?;
        writer.write_2_raw_double(&header.paper_space_limits_max)?;
        writer.write_bit_double(header.paper_elevation)?;
        writer.write_3_bit_double(&header.paper_space_ucs_origin)?;
        writer.write_3_bit_double(&header.paper_space_ucs_x_axis)?;
        writer.write_3_bit_double(&header.paper_space_ucs_y_axis)?;

        // PUCSNAME
        writer.handle_reference_typed(DwgReferenceType::HardPointer, 0)?;

        if ctx.r2000_plus {
            writer.handle_reference_typed(DwgReferenceType::HardPointer, header.paper_ucs_ortho_ref.value())?;
            writer.write_bit_short(header.paper_ucs_ortho_view)?;
            writer.handle_reference_typed(DwgReferenceType::HardPointer, 0)?;
            // Orthographic origins (PUCSORGTOP/BOTTOM/LEFT/RIGHT/FRONT/BACK)
            writer.write_3_bit_double(&Vector3::ZERO)?;
            writer.write_3_bit_double(&Vector3::ZERO)?;
            writer.write_3_bit_double(&Vector3::ZERO)?;
            writer.write_3_bit_double(&Vector3::ZERO)?;
            writer.write_3_bit_double(&Vector3::ZERO)?;
            writer.write_3_bit_double(&Vector3::ZERO)?;
        }

        // Model space extents
        writer.write_3_bit_double(&header.model_space_insertion_base)?;
        writer.write_3_bit_double(&header.model_space_extents_min)?;
        writer.write_3_bit_double(&header.model_space_extents_max)?;
        writer.write_2_raw_double(&header.model_space_limits_min)?;
        writer.write_2_raw_double(&header.model_space_limits_max)?;
        writer.write_bit_double(header.elevation)?;
        writer.write_3_bit_double(&header.model_space_ucs_origin)?;
        writer.write_3_bit_double(&header.model_space_ucs_x_axis)?;
        writer.write_3_bit_double(&header.model_space_ucs_y_axis)?;

        // UCSNAME
        writer.handle_reference_typed(DwgReferenceType::HardPointer, 0)?;

        if ctx.r2000_plus {
            writer.handle_reference_typed(DwgReferenceType::HardPointer, header.ucs_ortho_ref.value())?;
            writer.write_bit_short(header.ucs_ortho_view)?;
            writer.handle_reference_typed(DwgReferenceType::HardPointer, 0)?;
            writer.write_3_bit_double(&Vector3::ZERO)?;
            writer.write_3_bit_double(&Vector3::ZERO)?;
            writer.write_3_bit_double(&Vector3::ZERO)?;
            writer.write_3_bit_double(&Vector3::ZERO)?;
            writer.write_3_bit_double(&Vector3::ZERO)?;
            writer.write_3_bit_double(&Vector3::ZERO)?;

            writer.write_variable_text(&header.dim_post)?;
            writer.write_variable_text(&header.dim_alt_post)?;
        }

        // R13-R14 dimension flags
        if ctx.r13_14_only {
            writer.write_bit(header.dim_tolerance)?;
            writer.write_bit(header.dim_limits)?;
            writer.write_bit(header.dim_text_inside_horizontal)?;
            writer.write_bit(header.dim_text_outside_horizontal)?;
            writer.write_bit(header.dim_suppress_ext1)?;
            writer.write_bit(header.dim_suppress_ext2)?;
            writer.write_bit(header.dim_alternate_units)?;
            writer.write_bit(header.dim_force_line_inside)?;
            writer.write_bit(header.dim_separate_arrows)?;
            writer.write_bit(header.dim_force_text_inside)?;
            writer.write_bit(header.dim_suppress_outside_ext)?;
            writer.write_byte(header.dim_alt_decimal_places as u8)?;
            writer.write_byte(header.dim_zero_suppression as u8)?;
            writer.write_bit(header.dim_suppress_line1)?;
            writer.write_bit(header.dim_suppress_line2)?;
            writer.write_byte(header.dim_tolerance_justification as u8)?;
            writer.write_byte(header.dim_horizontal_justification as u8)?;
            writer.write_byte(header.dim_fit as u8)?;
            writer.write_bit(header.dim_user_positioned_text)?;
            writer.write_byte(header.dim_tolerance_zero_suppression as u8)?;
            writer.write_byte(header.dim_alt_tolerance_zero_suppression as u8)?;
            writer.write_byte(header.dim_alt_tolerance_zero_tight as u8)?;
            writer.write_byte(header.dim_text_above as u8)?;
            writer.write_bit_short(0)?; // DIMUNIT
            writer.write_bit_short(header.dim_angular_decimal_places)?;
            writer.write_bit_short(header.dim_decimal_places)?;
            writer.write_bit_short(header.dim_tolerance_decimal_places)?;
            writer.write_bit_short(header.dim_alt_units_format)?;
            writer.write_bit_short(header.dim_alt_tolerance_decimal_places)?;

            writer.handle_reference_typed(DwgReferenceType::HardPointer, header.dim_text_style_handle.value())?;
        }

        // Common dimension values
        writer.write_bit_double(header.dim_scale)?;
        writer.write_bit_double(header.dim_arrow_size)?;
        writer.write_bit_double(header.dim_ext_line_offset)?;
        writer.write_bit_double(header.dim_line_increment)?;
        writer.write_bit_double(header.dim_ext_line_extension)?;
        writer.write_bit_double(header.dim_rounding)?;
        writer.write_bit_double(header.dim_line_extension)?;
        writer.write_bit_double(header.dim_tolerance_plus)?;
        writer.write_bit_double(header.dim_tolerance_minus)?;

        // R2007+ dimension extensions
        if ctx.r2007_plus {
            writer.write_bit_double(0.0)?; // DIMFXL
            writer.write_bit_double(std::f64::consts::FRAC_PI_4)?; // DIMJOGANG
            writer.write_bit_short(0)?; // DIMTFILL
            writer.write_cm_color(&crate::types::Color::ByBlock)?; // DIMTFILLCLR
        }

        // R2000+ dimension flags
        if ctx.r2000_plus {
            writer.write_bit(header.dim_tolerance)?;
            writer.write_bit(header.dim_limits)?;
            writer.write_bit(header.dim_text_inside_horizontal)?;
            writer.write_bit(header.dim_text_outside_horizontal)?;
            writer.write_bit(header.dim_suppress_ext1)?;
            writer.write_bit(header.dim_suppress_ext2)?;
            writer.write_bit_short(header.dim_text_above)?;
            writer.write_bit_short(header.dim_zero_suppression)?;
            writer.write_bit_short(header.dim_alt_zero_suppression)?;
        }

        if ctx.r2007_plus {
            writer.write_bit_short(0)?; // DIMARCSYM
        }

        // Common dim text/center/tick
        writer.write_bit_double(header.dim_text_height)?;
        writer.write_bit_double(header.dim_center_mark)?;
        writer.write_bit_double(header.dim_tick_size)?;
        writer.write_bit_double(header.dim_alt_scale)?;
        writer.write_bit_double(header.dim_linear_scale)?;
        writer.write_bit_double(header.dim_text_vertical_pos)?;
        writer.write_bit_double(header.dim_tolerance_scale)?;
        writer.write_bit_double(header.dim_line_gap)?;

        if ctx.r13_14_only {
            writer.write_variable_text(&header.dim_post)?;
            writer.write_variable_text(&header.dim_alt_post)?;
            writer.write_variable_text(&header.dim_arrow_block)?;
            writer.write_variable_text(&header.dim_arrow_block1)?;
            writer.write_variable_text(&header.dim_arrow_block2)?;
        }

        if ctx.r2000_plus {
            writer.write_bit_double(header.dim_alt_rounding)?;
            writer.write_bit(header.dim_alternate_units)?;
            writer.write_bit_short(header.dim_alt_decimal_places)?;
            writer.write_bit(header.dim_force_line_inside)?;
            writer.write_bit(header.dim_separate_arrows)?;
            writer.write_bit(header.dim_force_text_inside)?;
            writer.write_bit(header.dim_suppress_outside_ext)?;
        }

        // Common colors
        writer.write_cm_color(&header.dim_line_color)?;
        writer.write_cm_color(&header.dim_ext_line_color)?;
        writer.write_cm_color(&header.dim_text_color)?;

        if ctx.r2000_plus {
            writer.write_bit_short(header.dim_angular_decimal_places)?;
            writer.write_bit_short(header.dim_decimal_places)?;
            writer.write_bit_short(header.dim_tolerance_decimal_places)?;
            writer.write_bit_short(header.dim_alt_units_format)?;
            writer.write_bit_short(header.dim_alt_tolerance_decimal_places)?;
            writer.write_bit_short(header.dim_angular_units)?;
            writer.write_bit_short(header.dim_fraction_format)?;
            writer.write_bit_short(header.dim_linear_unit_format)?;
            writer.write_bit_short(header.dim_decimal_separator as i16)?;
            writer.write_bit_short(header.dim_text_movement)?;
            writer.write_bit_short(header.dim_horizontal_justification)?;
            writer.write_bit(header.dim_suppress_line1)?;
            writer.write_bit(header.dim_suppress_line2)?;
            writer.write_bit_short(header.dim_tolerance_justification)?;
            writer.write_bit_short(header.dim_tolerance_zero_suppression)?;
            writer.write_bit_short(header.dim_alt_tolerance_zero_suppression)?;
            writer.write_bit_short(header.dim_alt_tolerance_zero_tight)?;
            writer.write_bit(header.dim_user_positioned_text)?;
            writer.write_bit_short(header.dim_fit)?;
        }

        if ctx.r2007_plus {
            writer.write_bit(false)?; // DIMFXLON
        }

        if version >= DxfVersion::AC1024 {
            writer.write_bit(false)?; // DIMTXTDIRECTION
            writer.write_bit_double(0.0)?; // DIMALTMZF
            writer.write_variable_text("")?; // DIMALTMZS
            writer.write_bit_double(0.0)?; // DIMMZF
            writer.write_variable_text("")?; // DIMMZS
        }

        if ctx.r2000_plus {
            // Dimension handles
            writer.handle_reference_typed(DwgReferenceType::HardPointer, header.dim_text_style_handle.value())?;
            writer.handle_reference_typed(DwgReferenceType::HardPointer, 0)?; // DIMLDRBLK
            writer.handle_reference_typed(DwgReferenceType::HardPointer, header.dim_arrow_block_handle.value())?;
            writer.handle_reference_typed(DwgReferenceType::HardPointer, header.dim_arrow_block1_handle.value())?;
            writer.handle_reference_typed(DwgReferenceType::HardPointer, header.dim_arrow_block2_handle.value())?;
        }

        if ctx.r2007_plus {
            writer.handle_reference_typed(DwgReferenceType::HardPointer, header.dim_linetype_handle.value())?;
            writer.handle_reference_typed(DwgReferenceType::HardPointer, header.dim_linetype1_handle.value())?;
            writer.handle_reference_typed(DwgReferenceType::HardPointer, header.dim_linetype2_handle.value())?;
        }

        if ctx.r2000_plus {
            writer.write_bit_short(header.dim_line_weight)?;
            writer.write_bit_short(header.dim_ext_line_weight)?;
        }

        // Table control handles
        writer.handle_reference_typed(DwgReferenceType::HardOwnership, header.block_control_handle.value())?;
        writer.handle_reference_typed(DwgReferenceType::HardOwnership, header.layer_control_handle.value())?;
        writer.handle_reference_typed(DwgReferenceType::HardOwnership, header.style_control_handle.value())?;
        writer.handle_reference_typed(DwgReferenceType::HardOwnership, header.linetype_control_handle.value())?;
        writer.handle_reference_typed(DwgReferenceType::HardOwnership, header.view_control_handle.value())?;
        writer.handle_reference_typed(DwgReferenceType::HardOwnership, header.ucs_control_handle.value())?;
        writer.handle_reference_typed(DwgReferenceType::HardOwnership, header.vport_control_handle.value())?;
        writer.handle_reference_typed(DwgReferenceType::HardOwnership, header.appid_control_handle.value())?;
        writer.handle_reference_typed(DwgReferenceType::HardOwnership, header.dimstyle_control_handle.value())?;

        if ctx.r13_15_only {
            writer.handle_reference_typed(DwgReferenceType::HardOwnership, header.vpent_hdr_control_handle.value())?;
        }

        // Dictionary handles
        writer.handle_reference_typed(DwgReferenceType::HardPointer, header.acad_group_dict_handle.value())?;
        writer.handle_reference_typed(DwgReferenceType::HardPointer, header.acad_mlinestyle_dict_handle.value())?;
        writer.handle_reference_typed(DwgReferenceType::HardOwnership, header.named_objects_dict_handle.value())?;

        if ctx.r2000_plus {
            writer.write_bit_short(1)?; // TSTACKALIGN
            writer.write_bit_short(70)?; // TSTACKSIZE
            writer.write_variable_text(&header.hyperlink_base)?;
            writer.write_variable_text(&header.stylesheet)?;

            writer.handle_reference_typed(DwgReferenceType::HardPointer, header.acad_layout_dict_handle.value())?;
            writer.handle_reference_typed(DwgReferenceType::HardPointer, header.acad_plotsettings_dict_handle.value())?;
            writer.handle_reference_typed(DwgReferenceType::HardPointer, header.acad_plotstylename_dict_handle.value())?;
        }

        if ctx.r2004_plus {
            writer.handle_reference_typed(DwgReferenceType::HardPointer, header.acad_material_dict_handle.value())?;
            writer.handle_reference_typed(DwgReferenceType::HardPointer, header.acad_color_dict_handle.value())?;
        }

        if ctx.r2007_plus {
            writer.handle_reference_typed(DwgReferenceType::HardPointer, header.acad_visualstyle_dict_handle.value())?;
            if ctx.r2013_plus {
                writer.handle_reference_typed(DwgReferenceType::HardPointer, 0)?; // DICT VISUALSTYLE
            }
        }

        if ctx.r2000_plus {
            // Flags BL: CELWEIGHT, ENDCAPS, JOINSTYLE, LWDISPLAY, XEDIT, EXTNAMES, PSTYLEMODE, OLESTARTUP
            let mut flags = (header.current_line_weight as i32 & 0x1F)
                | ((header.end_caps as i32) << 5)
                | ((header.join_style as i32) << 7);
            if !header.lineweight_display {
                flags |= 0x200;
            }
            if !header.xedit {
                flags |= 0x400;
            }
            if header.extended_names {
                flags |= 0x800;
            }
            if header.plotstyle_mode {
                flags |= 0x2000;
            }
            if header.ole_startup {
                flags |= 0x4000;
            }
            writer.write_bit_long(flags)?;

            writer.write_bit_short(header.insertion_units)?;
            writer.write_bit_short(header.current_plotstyle_type)?;

            if header.current_plotstyle_type == 3 {
                writer.handle_reference_typed(DwgReferenceType::HardPointer, 0)?;
            }

            writer.write_variable_text(&header.fingerprint_guid)?;
            writer.write_variable_text(&header.version_guid)?;
        }

        if ctx.r2004_plus {
            writer.write_byte(header.sort_entities as u8)?;
            writer.write_byte(header.index_control as u8)?;
            writer.write_byte(header.hide_text as u8)?;
            writer.write_byte(header.xclip_frame as u8)?;
            writer.write_byte(header.dimension_associativity as u8)?;
            writer.write_byte(header.halo_gap as u8)?;
            writer.write_bit_short(header.obscured_color)?;
            writer.write_bit_short(header.intersection_color)?;
            writer.write_byte(header.obscured_linetype as u8)?;
            writer.write_byte(header.intersection_display as u8)?;
            writer.write_variable_text(&header.project_name)?;
        }

        // Block record handles
        writer.handle_reference_typed(DwgReferenceType::HardPointer, header.paper_space_block_handle.value())?;
        writer.handle_reference_typed(DwgReferenceType::HardPointer, header.model_space_block_handle.value())?;
        writer.handle_reference_typed(DwgReferenceType::HardPointer, header.bylayer_linetype_handle.value())?;
        writer.handle_reference_typed(DwgReferenceType::HardPointer, header.byblock_linetype_handle.value())?;
        writer.handle_reference_typed(DwgReferenceType::HardPointer, header.continuous_linetype_handle.value())?;

        // R2007+ miscellaneous
        if ctx.r2007_plus {
            writer.write_bit(header.camera_display)?;
            writer.write_bit_long(0)?;
            writer.write_bit_long(0)?;
            writer.write_bit_double(0.0)?;

            writer.write_bit_double(header.steps_per_second)?;
            writer.write_bit_double(header.step_size)?;
            writer.write_bit_double(0.0)?; // 3DDWFPREC
            writer.write_bit_double(header.lens_length)?;
            writer.write_bit_double(header.camera_height)?;
            writer.write_byte(0)?; // SOLIDHIST
            writer.write_byte(0)?; // SHOWHIST
            writer.write_bit_double(0.0)?; // PSOLWIDTH
            writer.write_bit_double(0.0)?; // PSOLHEIGHT
            writer.write_bit_double(header.loft_angle1)?;
            writer.write_bit_double(header.loft_angle2)?;
            writer.write_bit_double(header.loft_magnitude1)?;
            writer.write_bit_double(header.loft_magnitude2)?;
            writer.write_bit_short(header.loft_param)?;
            writer.write_byte(header.loft_normals as u8)?;
            writer.write_bit_double(header.latitude)?;
            writer.write_bit_double(header.longitude)?;
            writer.write_bit_double(header.north_direction)?;
            writer.write_bit_long(header.timezone)?;
            writer.write_byte(0)?; // LIGHTGLYPHDISPLAY
            writer.write_byte(b'0')?; // TILEMODELIGHTSYNCH
            writer.write_byte(0)?; // DWFFRAME
            writer.write_byte(0)?; // DGNFRAME

            writer.write_bit(false)?; // unknown

            // INTERFERECOLOR
            writer.write_cm_color(&crate::types::Color::ByBlock)?;

            writer.handle_reference_typed(DwgReferenceType::HardPointer, 0)?; // INTERFEREOBJVS
            writer.handle_reference_typed(DwgReferenceType::HardPointer, 0)?; // INTERFEREVPVS
            writer.handle_reference_typed(DwgReferenceType::HardPointer, 0)?; // DRAGVS

            writer.write_byte(0)?; // CSHADOW
            writer.write_bit_double(header.shadow_plane_location)?;
        }

        // R14+ unknown shorts
        if version >= DxfVersion::AC1014 {
            writer.write_bit_short(-1)?;
            writer.write_bit_short(-1)?;
            writer.write_bit_short(-1)?;
            writer.write_bit_short(-1)?;

            if ctx.r2004_plus {
                writer.write_bit_long(0)?;
                writer.write_bit_long(0)?;
                writer.write_bit(false)?;
            }
        }

        writer.write_spear_shift()?;

        // Wrap with sentinels and CRC
        Self::wrap_with_sentinels_and_crc(version, header, &mut *writer)
    }

    fn wrap_with_sentinels_and_crc(
        version: DxfVersion,
        _header: &HeaderVariables,
        writer: &mut dyn DwgStreamWriter,
    ) -> Result<Vec<u8>> {
        // Extract section data
        let section_data = {
            let ws = writer.stream();
            ws.seek(SeekFrom::Start(0))?;
            let mut buf = Vec::new();
            StdRead::read_to_end(ws, &mut buf)?;
            buf
        };

        let start_sentinel = START_SENTINELS
            .get(DwgSectionDefinition::HEADER)
            .copied()
            .unwrap_or([0u8; 16]);
        let end_sentinel = END_SENTINELS
            .get(DwgSectionDefinition::HEADER)
            .copied()
            .unwrap_or([0u8; 16]);

        let mut output = Vec::new();
        output.extend_from_slice(&start_sentinel);

        // CRC8 wrapping: size + possible 64-bit extension + data
        let mut crc_data = Vec::new();
        crc_data.extend_from_slice(&(section_data.len() as i32).to_le_bytes());

        // R2010+ with maintenance > 3 or R2018+
        if (version >= DxfVersion::AC1024 && version.maintenance_version() > 3)
            || version >= DxfVersion::AC1032
        {
            crc_data.extend_from_slice(&0i32.to_le_bytes());
        }

        crc_data.extend_from_slice(&section_data);

        let crc = crc8_value(0xC0C1, &crc_data, 0, crc_data.len());
        output.extend_from_slice(&crc_data);
        output.extend_from_slice(&(crc as u16).to_le_bytes());

        output.extend_from_slice(&end_sentinel);

        Ok(output)
    }
}

/// Convert f64 julian date to (day, milliseconds) pair.
fn julian_from_f64(julian: f64) -> (i32, i32) {
    let day = julian as i32;
    let frac = julian - day as f64;
    let ms = (frac * 86_400_000.0) as i32;
    (day, ms)
}
