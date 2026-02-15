//! DWG file header structures and related types.
//!
//! This module contains all types needed to represent the file header
//! portion of a DWG binary file across different AutoCAD versions:
//!
//! - [`DwgFileHeader`] — unified file header with version dispatch
//! - [`DwgSectionLocatorRecord`] — AC15 record-based section locator
//! - [`DwgSectionDescriptor`] — AC18+ named section descriptor
//! - [`DwgLocalSectionMap`] — AC18+ page/section mapping
//! - [`DwgSectionDefinition`] — well-known section names and sentinels
//! - [`DwgSectionHash`] — AC21+ section hash identifiers
//! - [`Dwg21CompressedMetadata`] — AC21 compressed metadata block

mod dwg21_compressed_metadata;
mod dwg_file_header;
mod dwg_local_section_map;
mod dwg_section_definition;
mod dwg_section_descriptor;
mod dwg_section_hash;
mod dwg_section_locator_record;

pub use dwg21_compressed_metadata::Dwg21CompressedMetadata;
pub use dwg_file_header::{
    DwgFileHeader, DwgFileHeaderAC15, DwgFileHeaderAC18, DwgFileHeaderAC21, DwgFileHeaderData,
    AC15_END_SENTINEL,
};
pub use dwg_local_section_map::DwgLocalSectionMap;
pub use dwg_section_definition::{
    DwgSectionDefinition, END_SENTINELS, START_SENTINELS,
};
pub use dwg_section_descriptor::DwgSectionDescriptor;
pub use dwg_section_hash::DwgSectionHash;
pub use dwg_section_locator_record::DwgSectionLocatorRecord;
