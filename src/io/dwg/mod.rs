//! DWG file format reading and writing support
//!
//! This module provides functionality for reading and writing AutoCAD DWG binary files.
//!
//! ## Supported Versions
//!
//! - AC1012 (AutoCAD R13)
//! - AC1014 (AutoCAD R14)
//! - AC1015 (AutoCAD 2000/2000i/2002)
//! - AC1018 (AutoCAD 2004/2005/2006)
//! - AC1021 (AutoCAD 2007/2008/2009)
//! - AC1024 (AutoCAD 2010/2011/2012)
//! - AC1027 (AutoCAD 2013-2017)
//! - AC1032 (AutoCAD 2018+)
//!
//! ## Example
//!
//! ```rust,ignore
//! use acadrust::io::dwg::DwgReader;
//!
//! let doc = DwgReader::from_file("drawing.dwg")?.read()?;
//! println!("DWG Version: {:?}", doc.version);
//! ```

pub mod crc;
pub mod stream_reader;
pub mod file_header;
pub mod section;
pub mod decompressor;
pub mod reader;
pub mod header_reader;
pub mod classes_reader;
pub mod handle_reader;
pub mod object_reader;
pub mod section_reader;
pub mod template_builder;

// Re-export commonly used types
pub use crc::{Crc8, Crc32};
pub use stream_reader::{DwgStreamReader, BitReader, DwgReferenceType};
pub use file_header::{
    DwgFileHeader, DwgFileHeaderType, DwgFileHeaderAC15, DwgFileHeaderAC18, DwgFileHeaderAC21,
    CodePage,
};
pub use section::{
    DwgSectionDefinition, DwgSectionDescriptor, DwgSectionLocatorRecord,
    DwgLocalSectionMap, Dwg21CompressedMetadata, DwgSectionHash,
    section_names,
};
pub use decompressor::{Lz77AC18Decompressor, Lz77AC21Decompressor};
pub use reader::{DwgReader, DwgReaderConfiguration, is_dwg_file, get_dwg_version};
pub use header_reader::{DwgHeaderReader, DwgHeaderHandles};
pub use classes_reader::{DxfClass, DxfClassCollection, DwgClassesReader, ObjectType};
pub use handle_reader::DwgHandleReader;
pub use object_reader::{DwgObjectReader, DwgEntityData, DwgObjectData, CadTemplate};
pub use section_reader::{DwgSectionData, DwgSectionReaderAC15, DwgSectionReaderAC18};
pub use template_builder::DwgTemplateBuilder;

// Writer modules
pub mod stream_writer;
pub mod compressor;
pub mod writer;

pub use stream_writer::DwgStreamWriter;
pub use compressor::Lz77AC18Compressor;
pub use writer::DwgWriter;
