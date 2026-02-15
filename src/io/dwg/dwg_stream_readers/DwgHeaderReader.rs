use std::collections::BTreeMap;

use crate::{
    error::Result,
    types::{DxfVersion, Vector2, Vector3},
};

use super::idwg_stream_reader::DwgStreamReader;

#[derive(Debug, Clone, PartialEq)]
pub enum DwgHeaderValue {
    Bool(bool),
    I32(i32),
    I64(i64),
    F64(f64),
    Text(String),
    Handle(u64),
    Point2(Vector2),
    Point3(Vector3),
    PairI32(i32, i32),
}

/// Raw DWG header bag with semantic keys.
#[derive(Debug, Default, Clone)]
pub struct DwgHeaderData {
    pub vars: BTreeMap<String, DwgHeaderValue>,
}

impl DwgHeaderData {
    fn set(&mut self, key: impl Into<String>, value: DwgHeaderValue) {
        self.vars.insert(key.into(), value);
    }
}

/// Header object pointer handles extracted from DWG HEADER section.
#[derive(Debug, Default, Clone)]
pub struct DwgHeaderHandlesCollection {
    pub handles: BTreeMap<String, u64>,
}

impl DwgHeaderHandlesCollection {
    fn set(&mut self, key: impl Into<String>, value: u64) {
        self.handles.insert(key.into(), value);
    }
}

/// Result of HEADER section parsing.
#[derive(Debug, Default, Clone)]
pub struct DwgHeaderReadResult {
    pub header: DwgHeaderData,
    pub object_pointers: DwgHeaderHandlesCollection,
}

/// Reads DWG HEADER section (semantic port of ACadSharp flow).
pub struct DwgHeaderReader;

impl DwgHeaderReader {
    pub fn read(
        version: DxfVersion,
        acad_maintenance_version: i32,
        reader: &mut dyn DwgStreamReader,
    ) -> Result<DwgHeaderReadResult> {
        let mut out = DwgHeaderReadResult::default();

        // Section size (RL)
        let section_size = reader.read_raw_long()?;
        out.header
            .set("_section_size", DwgHeaderValue::I64(section_size as i64));

        if (Self::r2010_plus(version) && acad_maintenance_version > 3) || Self::r2018_plus(version) {
            out.header.set(
                "_unknown64_part",
                DwgHeaderValue::I64(reader.read_raw_long()?),
            );
        }

        if Self::r2013_plus(version) {
            out.header
                .set("required_versions", DwgHeaderValue::I64(reader.read_bit_long_long()?));
        }

        Self::read_common_prelude(reader, &mut out.header)?;
        Self::read_common_flags(version, reader, &mut out.header)?;
        Self::read_common_numeric(version, reader, &mut out.header)?;
        Self::read_common_dates(reader, &mut out.header)?;

        out.header.set(
            "current_entity_color",
            DwgHeaderValue::I32(reader.read_cm_color(false)?.approximate_index() as i32),
        );

        // HANDSEED is read from main stream in C# implementation; here we use same reader.
        out.header
            .set("handle_seed", DwgHeaderValue::Handle(reader.handle_reference()?));

        Self::read_primary_handles(version, reader, &mut out.object_pointers)?;
        Self::read_space_data(version, reader, &mut out.header, &mut out.object_pointers)?;
        Self::read_object_pointer_groups(version, reader, &mut out.object_pointers, &mut out.header)?;

        Ok(out)
    }

    fn read_common_prelude(reader: &mut dyn DwgStreamReader, header: &mut DwgHeaderData) -> Result<()> {
        header.set("_unknown_bd_1", DwgHeaderValue::F64(reader.read_bit_double()?));
        header.set("_unknown_bd_2", DwgHeaderValue::F64(reader.read_bit_double()?));
        header.set("_unknown_bd_3", DwgHeaderValue::F64(reader.read_bit_double()?));
        header.set("_unknown_bd_4", DwgHeaderValue::F64(reader.read_bit_double()?));

        header.set("_unknown_tv_1", DwgHeaderValue::Text(reader.read_variable_text()?));
        header.set("_unknown_tv_2", DwgHeaderValue::Text(reader.read_variable_text()?));
        header.set("_unknown_tv_3", DwgHeaderValue::Text(reader.read_variable_text()?));
        header.set("_unknown_tv_4", DwgHeaderValue::Text(reader.read_variable_text()?));

        header.set("_unknown_bl_1", DwgHeaderValue::I32(reader.read_bit_long()?));
        header.set("_unknown_bl_2", DwgHeaderValue::I32(reader.read_bit_long()?));
        Ok(())
    }

    fn read_common_flags(
        version: DxfVersion,
        reader: &mut dyn DwgStreamReader,
        header: &mut DwgHeaderData,
    ) -> Result<()> {
        header.set("dimaso", DwgHeaderValue::Bool(reader.read_bit()?));
        header.set("dimsho", DwgHeaderValue::Bool(reader.read_bit()?));

        if Self::r13_14_only(version) {
            header.set("dimsav", DwgHeaderValue::Bool(reader.read_bit()?));
        }

        header.set("plinegen", DwgHeaderValue::Bool(reader.read_bit()?));
        header.set("orthomode", DwgHeaderValue::Bool(reader.read_bit()?));
        header.set("regenmode", DwgHeaderValue::Bool(reader.read_bit()?));
        header.set("fillmode", DwgHeaderValue::Bool(reader.read_bit()?));
        header.set("qtextmode", DwgHeaderValue::Bool(reader.read_bit()?));
        header.set("psltscale", DwgHeaderValue::Bool(reader.read_bit()?));
        header.set("limcheck", DwgHeaderValue::Bool(reader.read_bit()?));

        if Self::r13_14_only(version) {
            header.set("blipmode", DwgHeaderValue::Bool(reader.read_bit()?));
        }
        if Self::r2004_plus(version) {
            let _ = reader.read_bit()?;
        }

        header.set("usrtimer", DwgHeaderValue::Bool(reader.read_bit()?));
        header.set("skpoly", DwgHeaderValue::Bool(reader.read_bit()?));
        header.set("angdir", DwgHeaderValue::I32(reader.read_bit_as_short()? as i32));
        header.set("splframe", DwgHeaderValue::Bool(reader.read_bit()?));

        if Self::r13_14_only(version) {
            let _ = reader.read_bit()?; // ATTREQ
            let _ = reader.read_bit()?; // ATTDIA
        }

        header.set("mirrtext", DwgHeaderValue::Bool(reader.read_bit()?));
        header.set("worldview", DwgHeaderValue::Bool(reader.read_bit()?));
        if Self::r13_14_only(version) {
            let _ = reader.read_bit()?; // WIREFRAME undocumented
        }

        header.set("tilemode", DwgHeaderValue::Bool(reader.read_bit()?));
        header.set("plimcheck", DwgHeaderValue::Bool(reader.read_bit()?));
        header.set("visretain", DwgHeaderValue::Bool(reader.read_bit()?));
        if Self::r13_14_only(version) {
            let _ = reader.read_bit()?; // DELOBJ
        }

        header.set("dispsilh", DwgHeaderValue::Bool(reader.read_bit()?));
        header.set("pellipse", DwgHeaderValue::Bool(reader.read_bit()?));
        header.set("proxygraphics", DwgHeaderValue::Bool(reader.read_bit_short_as_bool()?));

        Ok(())
    }

    fn read_common_numeric(
        version: DxfVersion,
        reader: &mut dyn DwgStreamReader,
        header: &mut DwgHeaderData,
    ) -> Result<()> {
        if Self::r13_14_only(version) {
            let _ = reader.read_bit_short()?; // DRAGMODE
        }

        header.set("treedepth", DwgHeaderValue::I32(reader.read_bit_short()? as i32));
        header.set("lunits", DwgHeaderValue::I32(reader.read_bit_short()? as i32));
        header.set("luprec", DwgHeaderValue::I32(reader.read_bit_short()? as i32));
        header.set("aunits", DwgHeaderValue::I32(reader.read_bit_short()? as i32));
        header.set("auprec", DwgHeaderValue::I32(reader.read_bit_short()? as i32));

        if Self::r13_14_only(version) {
            header.set("osmode", DwgHeaderValue::I32(reader.read_bit_short()? as i32));
        }

        header.set("attmode", DwgHeaderValue::I32(reader.read_bit_short()? as i32));
        if Self::r13_14_only(version) {
            let _ = reader.read_bit_short()?; // COORDS
        }

        header.set("pdmode", DwgHeaderValue::I32(reader.read_bit_short()? as i32));

        // USERI1..5
        for i in 1..=5 {
            header.set(
                format!("useri{i}"),
                DwgHeaderValue::I32(reader.read_bit_short()? as i32),
            );
        }

        header.set("ltscale", DwgHeaderValue::F64(reader.read_bit_double()?));
        header.set("textsize", DwgHeaderValue::F64(reader.read_bit_double()?));
        header.set("tracewid", DwgHeaderValue::F64(reader.read_bit_double()?));
        header.set("sketchinc", DwgHeaderValue::F64(reader.read_bit_double()?));
        header.set("filletrad", DwgHeaderValue::F64(reader.read_bit_double()?));
        header.set("thickness", DwgHeaderValue::F64(reader.read_bit_double()?));
        header.set("angbase", DwgHeaderValue::F64(reader.read_bit_double()?));
        header.set("pdsize", DwgHeaderValue::F64(reader.read_bit_double()?));
        header.set("plinewid", DwgHeaderValue::F64(reader.read_bit_double()?));

        header.set("menuname", DwgHeaderValue::Text(reader.read_variable_text()?));

        Ok(())
    }

    fn read_common_dates(reader: &mut dyn DwgStreamReader, header: &mut DwgHeaderData) -> Result<()> {
        let (d1, ms1) = reader.read_date_time()?;
        header.set("tdcreate", DwgHeaderValue::PairI32(d1, ms1));

        let (d2, ms2) = reader.read_date_time()?;
        header.set("tdupdate", DwgHeaderValue::PairI32(d2, ms2));

        let (d3, ms3) = reader.read_time_span()?;
        header.set("tdindwg", DwgHeaderValue::PairI32(d3, ms3));

        let (d4, ms4) = reader.read_time_span()?;
        header.set("tdusrtimer", DwgHeaderValue::PairI32(d4, ms4));

        Ok(())
    }

    fn read_primary_handles(
        version: DxfVersion,
        reader: &mut dyn DwgStreamReader,
        pointers: &mut DwgHeaderHandlesCollection,
    ) -> Result<()> {
        pointers.set("CLAYER", reader.handle_reference()?);
        pointers.set("TEXTSTYLE", reader.handle_reference()?);
        pointers.set("CELTYPE", reader.handle_reference()?);

        if Self::r2007_plus(version) {
            pointers.set("CMATERIAL", reader.handle_reference()?);
        }

        pointers.set("DIMSTYLE", reader.handle_reference()?);
        pointers.set("CMLSTYLE", reader.handle_reference()?);
        Ok(())
    }

    fn read_space_data(
        version: DxfVersion,
        reader: &mut dyn DwgStreamReader,
        header: &mut DwgHeaderData,
        pointers: &mut DwgHeaderHandlesCollection,
    ) -> Result<()> {
        if Self::r2000_plus(version) {
            header.set("psvpscale", DwgHeaderValue::F64(reader.read_bit_double()?));
        }

        // PSPACE
        header.set("insbase_pspace", DwgHeaderValue::Point3(reader.read_3_bit_double()?));
        header.set("extmin_pspace", DwgHeaderValue::Point3(reader.read_3_bit_double()?));
        header.set("extmax_pspace", DwgHeaderValue::Point3(reader.read_3_bit_double()?));
        header.set("limmin_pspace", DwgHeaderValue::Point2(reader.read_2_raw_double()?));
        header.set("limmax_pspace", DwgHeaderValue::Point2(reader.read_2_raw_double()?));

        header.set("elevation_pspace", DwgHeaderValue::F64(reader.read_bit_double()?));
        header.set("ucsorg_pspace", DwgHeaderValue::Point3(reader.read_3_bit_double()?));
        header.set("ucsxdir_pspace", DwgHeaderValue::Point3(reader.read_3_bit_double()?));
        header.set("ucsydir_pspace", DwgHeaderValue::Point3(reader.read_3_bit_double()?));

        pointers.set("UCSNAME_PSPACE", reader.handle_reference()?);

        if Self::r2000_plus(version) {
            pointers.set("PUCSORTHOREF", reader.handle_reference()?);
            header.set("PUCSORTHOVIEW", DwgHeaderValue::I32(reader.read_bit_short()? as i32));
            pointers.set("PUCSBASE", reader.handle_reference()?);

            header.set("pucsorgtop", DwgHeaderValue::Point3(reader.read_3_bit_double()?));
            header.set("pucsorgbottom", DwgHeaderValue::Point3(reader.read_3_bit_double()?));
            header.set("pucsorgleft", DwgHeaderValue::Point3(reader.read_3_bit_double()?));
            header.set("pucsorgright", DwgHeaderValue::Point3(reader.read_3_bit_double()?));
            header.set("pucsorgfront", DwgHeaderValue::Point3(reader.read_3_bit_double()?));
            header.set("pucsorgback", DwgHeaderValue::Point3(reader.read_3_bit_double()?));
        }

        // MSPACE
        header.set("insbase_mspace", DwgHeaderValue::Point3(reader.read_3_bit_double()?));
        header.set("extmin_mspace", DwgHeaderValue::Point3(reader.read_3_bit_double()?));
        header.set("extmax_mspace", DwgHeaderValue::Point3(reader.read_3_bit_double()?));
        header.set("limmin_mspace", DwgHeaderValue::Point2(reader.read_2_raw_double()?));
        header.set("limmax_mspace", DwgHeaderValue::Point2(reader.read_2_raw_double()?));
        header.set("elevation_mspace", DwgHeaderValue::F64(reader.read_bit_double()?));
        header.set("ucsorg_mspace", DwgHeaderValue::Point3(reader.read_3_bit_double()?));
        header.set("ucsxdir_mspace", DwgHeaderValue::Point3(reader.read_3_bit_double()?));
        header.set("ucsydir_mspace", DwgHeaderValue::Point3(reader.read_3_bit_double()?));
        pointers.set("UCSNAME_MSPACE", reader.handle_reference()?);

        Ok(())
    }

    fn read_object_pointer_groups(
        version: DxfVersion,
        reader: &mut dyn DwgStreamReader,
        pointers: &mut DwgHeaderHandlesCollection,
        header: &mut DwgHeaderData,
    ) -> Result<()> {
        if Self::r2000_plus(version) {
            pointers.set("UCSORTHOREF", reader.handle_reference()?);
            header.set("UCSORTHOVIEW", DwgHeaderValue::I32(reader.read_bit_short()? as i32));
            pointers.set("UCSBASE", reader.handle_reference()?);

            header.set("dimpost", DwgHeaderValue::Text(reader.read_variable_text()?));
            header.set("dimapost", DwgHeaderValue::Text(reader.read_variable_text()?));
        }

        // table/control object pointers
        for key in [
            "BLOCK_CONTROL_OBJECT",
            "LAYER_CONTROL_OBJECT",
            "STYLE_CONTROL_OBJECT",
            "LINETYPE_CONTROL_OBJECT",
            "VIEW_CONTROL_OBJECT",
            "UCS_CONTROL_OBJECT",
            "VPORT_CONTROL_OBJECT",
            "APPID_CONTROL_OBJECT",
            "DIMSTYLE_CONTROL_OBJECT",
            "DICTIONARY_ACAD_GROUP",
            "DICTIONARY_ACAD_MLINESTYLE",
            "DICTIONARY_NAMED_OBJECTS",
        ] {
            pointers.set(key, reader.handle_reference()?);
        }

        if Self::r2000_plus(version) {
            header.set("hyperlinkbase", DwgHeaderValue::Text(reader.read_variable_text()?));
            header.set("stylesheet", DwgHeaderValue::Text(reader.read_variable_text()?));

            pointers.set("DICTIONARY_LAYOUTS", reader.handle_reference()?);
            pointers.set("DICTIONARY_PLOTSETTINGS", reader.handle_reference()?);
            pointers.set("DICTIONARY_PLOTSTYLES", reader.handle_reference()?);
        }

        if Self::r2004_plus(version) {
            pointers.set("DICTIONARY_MATERIALS", reader.handle_reference()?);
            pointers.set("DICTIONARY_COLORS", reader.handle_reference()?);
        }

        if Self::r2007_plus(version) {
            pointers.set("DICTIONARY_VISUALSTYLE", reader.handle_reference()?);
            if Self::r2013_plus(version) {
                let _ = reader.handle_reference()?;
            }
        }

        // canonical base objects
        pointers.set("PAPER_SPACE", reader.handle_reference()?);
        pointers.set("MODEL_SPACE", reader.handle_reference()?);
        pointers.set("BYLAYER", reader.handle_reference()?);
        pointers.set("BYBLOCK", reader.handle_reference()?);
        pointers.set("CONTINUOUS", reader.handle_reference()?);

        Ok(())
    }

    #[inline]
    fn r13_14_only(v: DxfVersion) -> bool {
        matches!(v, DxfVersion::AC1012 | DxfVersion::AC1014)
    }
    #[inline]
    fn r2000_plus(v: DxfVersion) -> bool {
        v >= DxfVersion::AC1015
    }
    #[inline]
    fn r2004_plus(v: DxfVersion) -> bool {
        v >= DxfVersion::AC1018
    }
    #[inline]
    fn r2007_plus(v: DxfVersion) -> bool {
        v >= DxfVersion::AC1021
    }
    #[inline]
    fn r2010_plus(v: DxfVersion) -> bool {
        v >= DxfVersion::AC1024
    }
    #[inline]
    fn r2013_plus(v: DxfVersion) -> bool {
        v >= DxfVersion::AC1027
    }
    #[inline]
    fn r2018_plus(v: DxfVersion) -> bool {
        v >= DxfVersion::AC1032
    }
}
