//! DWG Handles Reader - Reads handle-to-offset map from DWG files
//!
//! The handles section maps object handles to their byte offsets in the
//! AcDbObjects section. This allows random access to objects by handle.

use std::collections::HashMap;
use std::io::{Read, Seek};
use crate::error::{DxfError, Result};
use crate::types::ACadVersion;
use super::stream_reader::{BitReader, DwgStreamReader};

/// Reader for DWG handles section  
pub struct DwgHandleReader<R: Read + Seek> {
    reader: BitReader<R>,
    version: ACadVersion,
}

impl<R: Read + Seek> DwgHandleReader<R> {
    /// Create a new handle reader
    pub fn new(reader: BitReader<R>, version: ACadVersion) -> Self {
        Self { reader, version }
    }
    
    /// Read the handle-to-offset map
    /// 
    /// Returns a HashMap where:
    /// - Key: Object handle (u64)
    /// - Value: Byte offset in AcDbObjects section (i64)
    pub fn read(&mut self) -> Result<HashMap<u64, i64>> {
        self.read_inner()
    }
    
    fn read_inner(&mut self) -> Result<HashMap<u64, i64>> {
        let mut handles: HashMap<u64, i64> = HashMap::new();
        
        // The handles section consists of multiple "pages" of handle data
        // Each page has a header followed by handle records
        
        loop {
            // Read section size as big-endian short (2 bytes)
            let size_hi = self.reader.read_raw_char()?;
            let size_lo = self.reader.read_raw_char()?;
            let section_size = ((size_hi as i32) << 8) | (size_lo as i32);
            
            // Size of 2 means end (just CRC)
            if section_size <= 2 {
                break;
            }
            
            let section_start = self.reader.position();
            // Max section size is 2032 bytes, section_size includes CRC (2 bytes)
            let max_section_offset = (section_size - 2).min(2032) as u64;
            let last_position = section_start + max_section_offset;
            
            let mut last_handle: u64 = 0;
            let mut last_offset: i64 = 0;
            
            while self.reader.position() < last_position {
                // Handle offset (modular char - delta from last)
                let handle_delta = self.reader.read_modular_char()?;
                last_handle = last_handle.wrapping_add(handle_delta);
                
                // Location offset (signed modular char - delta from last)  
                let offset_delta = self.reader.read_signed_modular_char()?;
                last_offset = last_offset.wrapping_add(offset_delta);
                
                if handle_delta > 0 {
                    handles.insert(last_handle, last_offset);
                }
            }
            
            // Read CRC (big-endian, 2 bytes)
            let _crc_hi = self.reader.read_raw_char()?;
            let _crc_lo = self.reader.read_raw_char()?;
        }
        
        Ok(handles)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    
    #[test]
    fn test_handle_reader_empty() {
        // Empty section (just a zero size)
        let data = vec![0x00];
        let reader = BitReader::new(Cursor::new(data), ACadVersion::AC1015);
        let mut handle_reader = DwgHandleReader::new(reader, ACadVersion::AC1015);
        
        let result = handle_reader.read();
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
