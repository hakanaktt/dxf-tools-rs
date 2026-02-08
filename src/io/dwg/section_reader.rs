//! DWG Section Reader - Reads sections from DWG files for different versions
//!
//! R2004+ (AC1018+) files use a page-based format with compressed sections.
//! R2007+ (AC1021+) adds additional encoding and larger page sizes.

use std::io::{Read, Seek, SeekFrom};
use crate::error::{DxfError, Result};
use crate::types::ACadVersion;
use super::file_header::{DwgFileHeaderAC15, DwgFileHeaderAC18, DwgFileHeaderAC21};
use super::decompressor::{Lz77AC18Decompressor, Lz77AC21Decompressor};
use super::section::DwgSectionLocatorRecord;

/// Result of reading a section from DWG file
#[derive(Debug)]
pub struct DwgSectionData {
    /// Section name
    pub name: String,
    /// Decompressed section data
    pub data: Vec<u8>,
}

/// Section reader for AC15 format (R2000)
pub struct DwgSectionReaderAC15<'a, R> {
    reader: &'a mut R,
    header: &'a DwgFileHeaderAC15,
    version: ACadVersion,
}

impl<'a, R: Read + Seek> DwgSectionReaderAC15<'a, R> {
    /// Create a new section reader
    pub fn new(reader: &'a mut R, header: &'a DwgFileHeaderAC15, version: ACadVersion) -> Self {
        Self { reader, header, version }
    }
    
    /// Read a section by name
    pub fn read_section(&mut self, name: &str) -> Result<DwgSectionData> {
        // Find section record and copy the values we need
        let (seeker, size) = {
            let record = self.find_section(name)?;
            (record.seeker as u64, record.size as usize)
        };
        
        // Seek to section start
        self.reader.seek(SeekFrom::Start(seeker))?;
        
        // Read section data
        let mut data = vec![0u8; size];
        self.reader.read_exact(&mut data)?;
        
        Ok(DwgSectionData {
            name: name.to_string(),
            data,
        })
    }
    
    /// Find a section record by name
    fn find_section(&self, name: &str) -> Result<&DwgSectionLocatorRecord> {
        let section_id = match name.to_uppercase().as_str() {
            "HEADER" => 0,
            "CLASSES" => 1,
            "HANDLES" => 2,
            "OBJECTS" | "ACDLOBJECTS" => 3,
            "OBJFREESPACE" => 4,
            "TEMPLATE" => 5,
            _ => return Err(DxfError::Parse(format!("Unknown section: {}", name))),
        };
        
        self.header.records.get(&section_id)
            .ok_or_else(|| DxfError::Parse(format!("Section {} not found", name)))
    }
    
    /// Get the version
    pub fn version(&self) -> ACadVersion {
        self.version
    }
}

/// Section reader for AC18 format (R2004+)
pub struct DwgSectionReaderAC18<'a, R> {
    reader: &'a mut R,
    #[allow(dead_code)]
    header: &'a DwgFileHeaderAC18,
    version: ACadVersion,
    /// Page data extracted from encrypted header
    pages: Vec<PageInfo>,
    /// Section info extracted from section map
    sections: Vec<SectionInfo>,
}

/// Page information
#[derive(Debug, Clone)]
struct PageInfo {
    id: u32,
    offset: u64,
    size: u32,
    compressed_size: u32,
}

/// Section information
#[derive(Debug, Clone)]
struct SectionInfo {
    name: String,
    #[allow(dead_code)]
    section_type: u32,
    page_indices: Vec<usize>,
    #[allow(dead_code)]
    decompressed_size: u64,
}

impl<'a, R: Read + Seek> DwgSectionReaderAC18<'a, R> {
    /// Create a new section reader
    pub fn new(reader: &'a mut R, header: &'a DwgFileHeaderAC18, version: ACadVersion) -> Result<Self> {
        let mut section_reader = Self { 
            reader, 
            header, 
            version,
            pages: Vec::new(),
            sections: Vec::new(),
        };
        
        // Read page map and section map
        section_reader.read_maps()?;
        
        Ok(section_reader)
    }
    
    /// Read page and section maps from encrypted header
    fn read_maps(&mut self) -> Result<()> {
        // Read the encrypted header at offset 0x80
        self.reader.seek(SeekFrom::Start(0x80))?;
        
        // The encrypted header is 0x6C (108) bytes
        let mut encrypted_data = [0u8; 108];
        self.reader.read_exact(&mut encrypted_data)?;
        
        // Decrypt using the magic number XOR sequence
        decrypt_header_ac18(&mut encrypted_data);
        
        // Validate file ID: "AcFssFcAJMB\0"
        let file_id = &encrypted_data[0..12];
        let expected_id = b"AcFssFcAJMB\0";
        if file_id != expected_id {
            // Don't fail, just continue with what we have
        }
        
        // Parse the decrypted header
        // 0x00  12  File ID string "AcFssFcAJMB\0"
        // 0x0C  4   0x00 (unknown)
        // 0x10  4   0x6C (unknown)
        // 0x14  4   0x04 (unknown)
        // 0x18  4   Root tree node gap
        // 0x1C  4   Left gap
        // 0x20  4   Right gap
        // 0x24  4   Unknown (ODA writes 1)
        // 0x28  4   Last section page ID
        // 0x2C  8   Last section page end address
        // 0x34  8   Second header data address
        // 0x3C  4   Gap amount
        // 0x40  4   Section page amount
        // 0x44  4   0x20 (unknown)
        // 0x48  4   0x80 (unknown)
        // 0x4C  4   0x40 (unknown)
        // 0x50  4   Section Page Map ID
        // 0x54  8   Section Page Map address (+0x100)
        // 0x5C  4   Section Map ID
        // 0x60  4   Section page array size
        // 0x64  4   Gap array size
        // 0x68  4   CRC32
        
        let section_page_map_id = u32::from_le_bytes([encrypted_data[0x50], encrypted_data[0x51], encrypted_data[0x52], encrypted_data[0x53]]);
        let section_page_map_address = u64::from_le_bytes([
            encrypted_data[0x54], encrypted_data[0x55], encrypted_data[0x56], encrypted_data[0x57],
            encrypted_data[0x58], encrypted_data[0x59], encrypted_data[0x5A], encrypted_data[0x5B],
        ]) + 0x100; // Add 0x100 header offset
        
        let section_amount = u32::from_le_bytes([encrypted_data[0x40], encrypted_data[0x41], encrypted_data[0x42], encrypted_data[0x43]]);
        let section_map_id = u32::from_le_bytes([encrypted_data[0x5C], encrypted_data[0x5D], encrypted_data[0x5E], encrypted_data[0x5F]]);
        let section_page_array_size = u32::from_le_bytes([encrypted_data[0x60], encrypted_data[0x61], encrypted_data[0x62], encrypted_data[0x63]]);
        
        // Read page map to build pages table
        if section_page_map_address > 0x100 {
            self.read_page_map(section_page_map_address, section_page_map_id, section_page_array_size as usize)?;
        }
        
        // Read section map to build sections table  
        if section_map_id > 0 {
            self.read_section_map(section_map_id)?;
        }
        
        Ok(())
    }
    
    /// Read the page map to build pages table
    fn read_page_map(&mut self, address: u64, _map_id: u32, _array_size: usize) -> Result<()> {
        self.reader.seek(SeekFrom::Start(address))?;
        
        // Read page map header (section size = 0x100)
        let mut page_header = [0u8; 20];
        if self.reader.read_exact(&mut page_header).is_err() {
            return Ok(()); // Skip if can't read
        }
        
        // Section type (should be 0x41630E3B for page map)
        let section_type = u32::from_le_bytes([page_header[0], page_header[1], page_header[2], page_header[3]]);
        if section_type != 0x41630E3B {
            return Ok(());
        }
        
        // Decompressed size
        let decompressed_size = u32::from_le_bytes([page_header[4], page_header[5], page_header[6], page_header[7]]);
        // Compressed size
        let compressed_size = u32::from_le_bytes([page_header[8], page_header[9], page_header[10], page_header[11]]);
        // Compression type
        let compression_type = u32::from_le_bytes([page_header[12], page_header[13], page_header[14], page_header[15]]);
        // Checksum
        let _checksum = u32::from_le_bytes([page_header[16], page_header[17], page_header[18], page_header[19]]);
        
        // Read compressed data
        let mut compressed = vec![0u8; compressed_size as usize];
        self.reader.read_exact(&mut compressed)?;
        
        // Decompress
        let decompressed = if compression_type == 2 {
            Lz77AC18Decompressor::decompress(&compressed, decompressed_size as usize)?
        } else {
            compressed
        };
        
        // Parse page map entries
        // Each entry: page number (4), page size (4) = 8 bytes
        // The actual offset is calculated by accumulating sizes
        let mut offset = 0;
        let mut current_offset: u64 = 0x100; // Start after header
        
        while offset + 8 <= decompressed.len() {
            let page_number = i32::from_le_bytes([
                decompressed[offset], decompressed[offset+1], decompressed[offset+2], decompressed[offset+3]
            ]);
            
            let page_size = i32::from_le_bytes([
                decompressed[offset+4], decompressed[offset+5], decompressed[offset+6], decompressed[offset+7]
            ]);
            
            if page_number >= 0 {
                // Regular page entry
                self.pages.push(PageInfo {
                    id: page_number as u32,
                    offset: current_offset,
                    size: page_size as u32,
                    compressed_size: page_size as u32,
                });
                offset += 8;
            } else {
                // Negative page number = gap (unused data)
                // Skip additional 12 bytes (parent, left, right)
                offset += 8 + 12;
            }
            
            current_offset += page_size.abs() as u64;
        }
        
        Ok(())
    }
    
    /// Read section map to build sections table
    fn read_section_map(&mut self, section_map_id: u32) -> Result<()> {
        // Find the page that contains the section map
        let page = match self.pages.iter().find(|p| p.id == section_map_id) {
            Some(p) => p.clone(),
            None => {
                return Ok(());
            }
        };
        
        // Read the section map page
        self.reader.seek(SeekFrom::Start(page.offset))?;
        
        // Read page header (20 bytes)
        let mut page_header = [0u8; 20];
        self.reader.read_exact(&mut page_header)?;
        
        // Section type (should be 0x4163003B for section map)
        let section_type = u32::from_le_bytes([page_header[0], page_header[1], page_header[2], page_header[3]]);
        if section_type != 0x4163003B {
            return Ok(());
        }
        
        let decompressed_size = u32::from_le_bytes([page_header[4], page_header[5], page_header[6], page_header[7]]);
        let compressed_size = u32::from_le_bytes([page_header[8], page_header[9], page_header[10], page_header[11]]);
        let compression_type = u32::from_le_bytes([page_header[12], page_header[13], page_header[14], page_header[15]]);
        
        // Read compressed data
        let mut compressed = vec![0u8; compressed_size as usize];
        self.reader.read_exact(&mut compressed)?;
        
        // Decompress
        let decompressed = if compression_type == 2 {
            Lz77AC18Decompressor::decompress(&compressed, decompressed_size as usize)?
        } else {
            compressed
        };
        
        // Parse section map header
        // 0x00: 4 bytes - Number of section descriptions
        // 0x04: 4 bytes - 0x02
        // 0x08: 4 bytes - 0x7400
        // 0x0C: 4 bytes - 0x00
        // 0x10: 4 bytes - Unknown
        if decompressed.len() < 20 {
            return Ok(());
        }
        
        let num_descriptions = i32::from_le_bytes([
            decompressed[0], decompressed[1], decompressed[2], decompressed[3]
        ]) as usize;
        
        let mut offset = 20; // Skip header
        
        for _ in 0..num_descriptions {
            // Section descriptor is 0x60 (96) bytes:
            // 0x00: 8 bytes - Size of section (u64)
            // 0x08: 4 bytes - Page count
            // 0x0C: 4 bytes - Max decompressed size (0x7400)
            // 0x10: 4 bytes - Unknown
            // 0x14: 4 bytes - Compressed (1=no, 2=yes)
            // 0x18: 4 bytes - Section ID
            // 0x1C: 4 bytes - Encrypted (0=no, 1=yes)
            // 0x20: 64 bytes - Section name (null-terminated, fixed)
            
            if offset + 96 > decompressed.len() {
                break;
            }
            
            let size = u64::from_le_bytes([
                decompressed[offset], decompressed[offset+1], decompressed[offset+2], decompressed[offset+3],
                decompressed[offset+4], decompressed[offset+5], decompressed[offset+6], decompressed[offset+7]
            ]);
            
            let page_count = i32::from_le_bytes([
                decompressed[offset+8], decompressed[offset+9], decompressed[offset+10], decompressed[offset+11]
            ]);
            
            let _max_decompressed = u32::from_le_bytes([
                decompressed[offset+12], decompressed[offset+13], decompressed[offset+14], decompressed[offset+15]
            ]);
            
            let _unknown1 = i32::from_le_bytes([
                decompressed[offset+16], decompressed[offset+17], decompressed[offset+18], decompressed[offset+19]
            ]);
            
            let _compression = i32::from_le_bytes([
                decompressed[offset+20], decompressed[offset+21], decompressed[offset+22], decompressed[offset+23]
            ]);
            
            let section_id = i32::from_le_bytes([
                decompressed[offset+24], decompressed[offset+25], decompressed[offset+26], decompressed[offset+27]
            ]);
            
            let _encrypted = i32::from_le_bytes([
                decompressed[offset+28], decompressed[offset+29], decompressed[offset+30], decompressed[offset+31]
            ]);
            
            // Read 64-byte name starting at offset+32 (0x20)
            let name_bytes = &decompressed[offset+32..offset+96];
            let name = name_bytes.iter()
                .take_while(|&&b| b != 0)
                .cloned()
                .collect::<Vec<u8>>();
            let name = String::from_utf8_lossy(&name).to_string();
            
            offset += 96;
            
            // Read page list: for each of page_count pages
            // Each page entry: 4 bytes page number, 4 bytes compressed size, 8 bytes offset = 16 bytes
            let page_count_abs = page_count.abs() as usize;
            let mut page_indices = Vec::new();
            
            for _ in 0..page_count_abs {
                if offset + 16 > decompressed.len() {
                    break;
                }
                
                let page_number = i32::from_le_bytes([
                    decompressed[offset], decompressed[offset+1], decompressed[offset+2], decompressed[offset+3]
                ]);
                
                // Page number is 1-based index; find page with that ID
                if page_number > 0 {
                    if let Some(idx) = self.pages.iter().position(|p| p.id == page_number as u32) {
                        page_indices.push(idx);
                    }
                }
                
                offset += 16; // Skip page number + compressed size + offset
            }
            
            if !name.is_empty() {
                self.sections.push(SectionInfo {
                    name,
                    section_type: section_id as u32,
                    page_indices,
                    decompressed_size: size,
                });
            }
        }
        
        Ok(())
    }
    
    /// Read a section by name  
    pub fn read_section(&mut self, name: &str) -> Result<DwgSectionData> {
        // Find section
        let section = self.sections.iter()
            .find(|s| s.name.eq_ignore_ascii_case(name))
            .cloned()
            .ok_or_else(|| DxfError::Parse(format!("Section {} not found", name)))?;
        
        // Collect page info first to avoid borrow conflicts
        let pages_to_read: Vec<PageInfo> = section.page_indices.iter()
            .filter_map(|&idx| self.pages.get(idx).cloned())
            .collect();
        
        // Decompress all pages
        let mut data = Vec::new();
        for page in &pages_to_read {
            let page_data = self.read_page(page)?;
            data.extend(page_data);
        }
        
        // Truncate to the actual section size (pages may be larger due to padding)
        let actual_size = section.decompressed_size as usize;
        if data.len() > actual_size && actual_size > 0 {
            data.truncate(actual_size);
        }
        
        Ok(DwgSectionData {
            name: name.to_string(),
            data,
        })
    }
    
    /// Read and decompress a single page
    fn read_page(&mut self, page: &PageInfo) -> Result<Vec<u8>> {
        self.reader.seek(SeekFrom::Start(page.offset))?;
        
        // Data section pages have a 32-byte encrypted header
        // Each 4-byte value is XOR'd with: 0x4164536B ^ position
        let sec_mask = 0x4164536Bu32 ^ (page.offset as u32);
        
        // Read 32-byte encrypted page header
        let mut page_header = [0u8; 32];
        self.reader.read_exact(&mut page_header)?;
        
        // Decrypt and parse page header fields
        let raw_type = u32::from_le_bytes([page_header[0], page_header[1], page_header[2], page_header[3]]);
        let _section_type = raw_type ^ sec_mask; // Should be 0x4163043B for data sections
        
        let raw_section = u32::from_le_bytes([page_header[4], page_header[5], page_header[6], page_header[7]]);
        let _section_number = raw_section ^ sec_mask;
        
        let raw_compressed = u32::from_le_bytes([page_header[8], page_header[9], page_header[10], page_header[11]]);
        let compressed_size = (raw_compressed ^ sec_mask) as usize;
        
        let raw_decompressed = u32::from_le_bytes([page_header[12], page_header[13], page_header[14], page_header[15]]);
        let decompressed_size = (raw_decompressed ^ sec_mask) as usize;
        
        // Read compressed data
        let mut compressed = vec![0u8; compressed_size];
        self.reader.read_exact(&mut compressed)?;
        
        // Decompress if compressed (check if sizes differ or compressed_size > 0)
        let decompressed = if compressed_size > 0 && decompressed_size > 0 {
            Lz77AC18Decompressor::decompress(&compressed, decompressed_size)?
        } else {
            compressed
        };
        
        Ok(decompressed)
    }
    
    /// Get version
    #[allow(dead_code)]
    pub fn version(&self) -> ACadVersion {
        self.version
    }
}

/// Section reader for AC21 format (R2007+)
pub struct DwgSectionReaderAC21<'a, R> {
    reader: &'a mut R,
    #[allow(dead_code)]
    header: &'a DwgFileHeaderAC21,
    version: ACadVersion,
    /// Page data
    pages: Vec<PageInfo>,
    /// Section info
    sections: Vec<SectionInfo>,
}

impl<'a, R: Read + Seek> DwgSectionReaderAC21<'a, R> {
    /// Create a new section reader
    pub fn new(reader: &'a mut R, header: &'a DwgFileHeaderAC21, version: ACadVersion) -> Result<Self> {
        let mut section_reader = Self { 
            reader, 
            header, 
            version,
            pages: Vec::new(),
            sections: Vec::new(),
        };
        
        // Read page map and section map
        section_reader.read_maps()?;
        
        Ok(section_reader)
    }
    
    /// Read page and section maps
    fn read_maps(&mut self) -> Result<()> {
        // Placeholder - full implementation requires R2007 format parsing
        Ok(())
    }
    
    /// Read a section by name
    pub fn read_section(&mut self, name: &str) -> Result<DwgSectionData> {
        // Find section
        let section = self.sections.iter()
            .find(|s| s.name.eq_ignore_ascii_case(name))
            .cloned()
            .ok_or_else(|| DxfError::Parse(format!("Section {} not found", name)))?;
        
        // Collect page info first to avoid borrow conflicts
        let pages_to_read: Vec<PageInfo> = section.page_indices.iter()
            .filter_map(|&idx| self.pages.get(idx).cloned())
            .collect();
        
        // Decompress all pages
        let mut data = Vec::new();
        for page in &pages_to_read {
            let page_data = self.read_page(page)?;
            data.extend(page_data);
        }
        
        Ok(DwgSectionData {
            name: name.to_string(),
            data,
        })
    }
    
    /// Read and decompress a single page
    fn read_page(&mut self, page: &PageInfo) -> Result<Vec<u8>> {
        self.reader.seek(SeekFrom::Start(page.offset))?;
        
        let mut compressed = vec![0u8; page.compressed_size as usize];
        self.reader.read_exact(&mut compressed)?;
        
        // Decompress using AC21 decompressor
        let mut decompressed = vec![0u8; page.size as usize];
        Lz77AC21Decompressor::decompress(&compressed, 0, compressed.len(), &mut decompressed)?;
        
        Ok(decompressed)
    }
    
    /// Get version
    #[allow(dead_code)]
    pub fn version(&self) -> ACadVersion {
        self.version
    }
}

/// Decrypt XOR-encrypted header data (R2004+) using LFSR
#[allow(dead_code)]
fn decrypt_header(data: &mut [u8], seed: u32) {
    let mut randseed: u32 = seed;
    for byte in data.iter_mut() {
        randseed = randseed.wrapping_mul(0x343FD).wrapping_add(0x269EC3);
        *byte ^= (randseed >> 16) as u8;
    }
}

/// Decrypt AC18 header using LCG (Linear Congruential Generator)
/// The algorithm uses: seed = seed * 0x343fd + 0x269ec3
/// XOR each byte with (seed >> 16) & 0xFF
fn decrypt_header_ac18(data: &mut [u8; 108]) {
    let mut rand_seed: i32 = 1;
    for byte in data.iter_mut() {
        rand_seed = rand_seed.wrapping_mul(0x343fd).wrapping_add(0x269ec3);
        let xor_byte = ((rand_seed >> 16) & 0xFF) as u8;
        *byte ^= xor_byte;
    }
}
