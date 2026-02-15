//! DWG stream writers (ported from ACadSharp `DwgStreamWriters`).

#[path = "IDwgStreamWriter.rs"]
pub mod idwg_stream_writer;
#[path = "DwgStreamWriterBase.rs"]
pub mod dwg_stream_writer_base;
#[path = "DwgMergedStreamWriter.rs"]
pub mod dwg_merged_stream_writer;
#[path = "DwgHandleWriter.rs"]
pub mod dwg_handle_writer;
#[path = "DwgClassesWriter.rs"]
pub mod dwg_classes_writer;
#[path = "DwgPreviewWriter.rs"]
pub mod dwg_preview_writer;
#[path = "DwgAppInfoWriter.rs"]
pub mod dwg_app_info_writer;
#[path = "DwgAuxHeaderWriter.rs"]
pub mod dwg_aux_header_writer;
#[path = "DwgHeaderWriter.rs"]
pub mod dwg_header_writer;
#[path = "DwgLZ77AC18Compressor.rs"]
pub mod dwg_lz77_ac18_compressor;
#[path = "DwgLZ77AC21Compressor.rs"]
pub mod dwg_lz77_ac21_compressor;
#[path = "DwgFileHeaderWriterBase.rs"]
pub mod dwg_file_header_writer_base;
#[path = "DwgFileHeaderWriterAC15.rs"]
pub mod dwg_file_header_writer_ac15;
#[path = "DwgFileHeaderWriterAC18.rs"]
pub mod dwg_file_header_writer_ac18;
#[path = "DwgFileHeaderWriterAC21.rs"]
pub mod dwg_file_header_writer_ac21;
#[path = "DwgWriterConfiguration.rs"]
pub mod dwg_writer_configuration;
#[path = "DwgWriter.rs"]
pub mod dwg_writer;

pub use idwg_stream_writer::{Compressor, DwgFileHeaderWriter, DwgStreamWriter, WriteSeek};
pub use dwg_stream_writer_base::DwgStreamWriterBase;
pub use dwg_lz77_ac18_compressor::DwgLz77Ac18Compressor;
pub use dwg_lz77_ac21_compressor::DwgLz77Ac21Compressor;
pub use dwg_preview_writer::DwgPreview;
pub use dwg_writer_configuration::DwgWriterConfiguration;
pub use dwg_writer::{DwgWriter, write_dwg, write_dwg_to_bytes};
