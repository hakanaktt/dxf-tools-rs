//! DWG read/write support.

pub mod dwg_stream_readers;
pub mod file_headers;

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
