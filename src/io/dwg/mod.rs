//! DWG read/write support.

pub mod crc;
pub mod crc8_stream_handler;
pub mod crc32_stream_handler;
pub mod dwg_checksum_calculator;
pub mod dwg_document_builder;
pub mod dwg_header_handles_collection;
pub mod dwg_reader_configuration;
pub mod dwg_section_io;
pub mod dwg_stream_readers;
pub mod file_headers;

pub use crc::{apply_crc8, crc8_decode, crc8_value, crc32_update, CRC_TABLE, CRC32_TABLE};
pub use crc8_stream_handler::Crc8StreamHandler;
pub use crc32_stream_handler::Crc32StreamHandler;
pub use dwg_checksum_calculator::{calculate, compression_calculator, MAGIC_SEQUENCE};
pub use dwg_document_builder::DwgDocumentBuilder;
pub use dwg_header_handles_collection::DwgHeaderHandlesCollection;
pub use dwg_reader_configuration::DwgReaderConfiguration;
pub use dwg_section_io::{check_sentinel, DwgSectionContext};

pub use dwg_stream_readers::{
    DwgLz77Ac18Decompressor, DwgLz77Ac21Decompressor, DwgStreamReader, DwgStreamReaderAc12,
    DwgStreamReaderAc15, DwgStreamReaderAc18, DwgStreamReaderAc21, DwgStreamReaderAc24,
    DwgStreamReaderBase,
};

pub use file_headers::{
    Dwg21CompressedMetadata, DwgFileHeader, DwgFileHeaderAC15, DwgFileHeaderAC18,
    DwgFileHeaderAC21, DwgFileHeaderData, DwgLocalSectionMap, DwgSectionDefinition,
    DwgSectionDescriptor, DwgSectionHash, DwgSectionLocatorRecord, AC15_END_SENTINEL,
    END_SENTINELS, START_SENTINELS,
};
