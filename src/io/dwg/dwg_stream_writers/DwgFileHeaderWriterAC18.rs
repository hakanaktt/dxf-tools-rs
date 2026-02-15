//! AC18 (R2004+) file header writer — page-based layout with compression.

use std::io::{Cursor, Read, Seek, SeekFrom, Write};

use crate::error::Result;
use crate::io::dwg::{
    calculate, compression_calculator, crc8_value, Crc32StreamHandler, DwgFileHeaderAC18,
    DwgLocalSectionMap, DwgSectionDefinition, DwgSectionDescriptor, MAGIC_SEQUENCE,
};
use crate::types::DxfVersion;

use super::dwg_file_header_writer_base::{
    apply_magic_sequence, apply_mask, check_empty_bytes, get_file_code_page, write_magic_number,
};
use super::dwg_lz77_ac18_compressor::DwgLz77Ac18Compressor;
use super::idwg_stream_writer::{Compressor, DwgFileHeaderWriter};

const AC18_FILE_HEADER_SIZE: usize = 0x100;

struct SectionStreamState {
    position: u64,
}

pub struct DwgFileHeaderWriterAc18 {
    stream: Cursor<Vec<u8>>,
    version: DxfVersion,
    version_string: String,
    code_page: String,
    maintenance_version: i16,
    descriptors: Vec<(String, DwgSectionDescriptor)>,
    local_sections: Vec<DwgLocalSectionMap>,
    // File header fields
    section_array_page_size: u32,
    section_page_map_id: u32,
    section_map_id: u32,
    root_tree_node_gap: i32,
    left_gap: i32,
    right_gap: i32,
    last_page_id: i32,
    last_section_addr: u64,
    second_header_addr: u64,
    gap_amount: u32,
    section_amount: u32,
    gap_array_size: u32,
    page_map_address: u64,
}

impl DwgFileHeaderWriterAc18 {
    pub fn new(
        version: DxfVersion,
        version_string: String,
        code_page: String,
        maintenance_version: i16,
    ) -> Self {
        let mut stream = Cursor::new(Vec::with_capacity(0x10000));
        // Reserve space for file header
        for _ in 0..AC18_FILE_HEADER_SIZE {
            let _ = stream.write_all(&[0]);
        }

        Self {
            stream,
            version,
            version_string,
            code_page,
            maintenance_version,
            descriptors: Vec::new(),
            local_sections: Vec::new(),
            section_array_page_size: 0,
            section_page_map_id: 0,
            section_map_id: 0,
            root_tree_node_gap: 0,
            left_gap: 0,
            right_gap: 0,
            last_page_id: 0,
            last_section_addr: 0,
            second_header_addr: 0,
            gap_amount: 0,
            section_amount: 0,
            gap_array_size: 0,
            page_map_address: 0,
        }
    }

    fn find_descriptor(&self, name: &str) -> Option<&DwgSectionDescriptor> {
        self.descriptors
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, d)| d)
    }

    fn find_descriptor_mut(&mut self, name: &str) -> Option<&mut DwgSectionDescriptor> {
        self.descriptors
            .iter_mut()
            .find(|(n, _)| n == name)
            .map(|(_, d)| d)
    }

    fn create_local_section(
        &mut self,
        descriptor_name: &str,
        buffer: &[u8],
        decompressed_size: usize,
        offset: usize,
        total_size: usize,
        is_compressed: bool,
    ) -> Result<()> {
        let compressed_data =
            self.apply_compression(buffer, decompressed_size, offset, total_size, is_compressed)?;

        let pos = self.stream.position();
        write_magic_number(&mut self.stream, pos);

        let position = self.stream.position();

        let oda = calculate(0, &compressed_data, 0, compressed_data.len());
        let compress_diff = compression_calculator(compressed_data.len() as i32);

        let page_number = self.local_sections.len() as i32 + 1;
        let compressed_size = compressed_data.len() as u64;
        let page_size = compressed_size as i64 + 32 + compress_diff as i64;

        let mut local_map = DwgLocalSectionMap::new();
        local_map.offset = offset as u64;
        local_map.seeker = position as i64;
        local_map.page_number = page_number;
        local_map.oda = oda;
        local_map.compressed_size = compressed_size;
        local_map.decompressed_size = total_size as u64;
        local_map.page_size = page_size;
        local_map.checksum = 0;

        // Get descriptor info
        let (section_id, page_type) = {
            let desc = self.find_descriptor(descriptor_name).unwrap();
            (desc.section_id, desc.page_type)
        };

        // Compute checksum
        let mut checksum_stream = Vec::with_capacity(32);
        Self::write_data_section_to(
            &mut checksum_stream,
            section_id,
            &local_map,
            page_type as i32,
        );
        local_map.checksum =
            calculate(local_map.oda, &checksum_stream, 0, checksum_stream.len()) as u64;

        checksum_stream.clear();
        Self::write_data_section_to(
            &mut checksum_stream,
            section_id,
            &local_map,
            page_type as i32,
        );

        let cs_len = checksum_stream.len();
        apply_mask(
            &mut checksum_stream,
            0,
            cs_len,
            self.stream.position() as i64,
        );

        self.stream.write_all(&checksum_stream)?;
        self.stream.write_all(&compressed_data)?;

        if is_compressed {
            let magic = &*MAGIC_SEQUENCE;
            let write_len = compress_diff.min(magic.len() as i32) as usize;
            self.stream.write_all(&magic[..write_len])?;
        }

        // Update descriptor
        if page_number > 0 {
            if let Some(desc) = self.find_descriptor_mut(descriptor_name) {
                desc.page_count += 1;
            }
        }

        let size = self.stream.position() as i64 - position as i64;
        let mut local_map = local_map;
        local_map.size = size;

        if let Some(desc) = self.find_descriptor_mut(descriptor_name) {
            desc.local_sections.push(local_map.clone());
        }
        self.local_sections.push(local_map);

        Ok(())
    }

    fn apply_compression(
        &self,
        buffer: &[u8],
        decompressed_size: usize,
        offset: usize,
        total_size: usize,
        is_compressed: bool,
    ) -> Result<Vec<u8>> {
        if is_compressed {
            let mut holder = vec![0u8; decompressed_size];
            let copy_len = total_size.min(buffer.len() - offset);
            holder[..copy_len].copy_from_slice(&buffer[offset..offset + copy_len]);

            let mut dest = Vec::new();
            let mut compressor = DwgLz77Ac18Compressor::new();
            compressor.compress(&holder, 0, decompressed_size, &mut dest);
            Ok(dest)
        } else {
            let mut dest = vec![0u8; decompressed_size];
            let copy_len = total_size.min(buffer.len() - offset);
            dest[..copy_len].copy_from_slice(&buffer[offset..offset + copy_len]);
            Ok(dest)
        }
    }

    fn write_descriptors(&mut self) -> Result<()> {
        let mut stream_data = Vec::new();

        // Number of descriptors
        stream_data.extend_from_slice(&(self.descriptors.len() as i32).to_le_bytes());
        // 0x02
        stream_data.extend_from_slice(&2i32.to_le_bytes());
        // 0x7400
        stream_data.extend_from_slice(&0x7400i32.to_le_bytes());
        // 0x00
        stream_data.extend_from_slice(&0i32.to_le_bytes());
        // NumDescriptions
        stream_data.extend_from_slice(&(self.descriptors.len() as i32).to_le_bytes());

        for (_, desc) in &self.descriptors {
            // Size (8)
            stream_data.extend_from_slice(&desc.compressed_size.to_le_bytes());
            // PageCount
            stream_data.extend_from_slice(&desc.page_count.to_le_bytes());
            // Max decompressed size
            stream_data.extend_from_slice(&(desc.decompressed_size as i32).to_le_bytes());
            // Unknown (1)
            stream_data.extend_from_slice(&1i32.to_le_bytes());
            // Compressed code
            stream_data.extend_from_slice(&desc.compressed_code().to_le_bytes());
            // Section id
            stream_data.extend_from_slice(&desc.section_id.to_le_bytes());
            // Encrypted
            stream_data.extend_from_slice(&desc.encrypted.to_le_bytes());

            // Name (64 bytes)
            let mut name_arr = [0u8; 64];
            let name_bytes = desc.name.as_bytes();
            let copy_len = name_bytes.len().min(64);
            name_arr[..copy_len].copy_from_slice(&name_bytes[..copy_len]);
            stream_data.extend_from_slice(&name_arr);

            for local in &desc.local_sections {
                if local.page_number > 0 {
                    stream_data.extend_from_slice(&local.page_number.to_le_bytes());
                    stream_data.extend_from_slice(&(local.compressed_size as i32).to_le_bytes());
                    stream_data.extend_from_slice(&local.offset.to_le_bytes());
                }
            }
        }

        // Section map: 0x4163003b
        let section_holder = self.set_seeker(0x4163003B, &stream_data)?;
        let count = compression_calculator(
            (self.stream.position() as i64 - section_holder.seeker) as i32,
        );
        let magic = &*MAGIC_SEQUENCE;
        let write_len = (count as usize).min(magic.len());
        self.stream.write_all(&magic[..write_len])?;

        let mut sec = section_holder;
        sec.size = self.stream.position() as i64 - sec.seeker;
        self.add_section_internal(sec);

        Ok(())
    }

    fn write_records(&mut self) -> Result<()> {
        let pos = self.stream.position();
        write_magic_number(&mut self.stream, pos);

        let mut section = DwgLocalSectionMap::with_section_map(0x41630E3B);
        self.add_section_internal(section.clone());

        let counter = self.local_sections.len() * 8;
        section.seeker = self.stream.position() as i64;
        let size = counter as i64
            + compression_calculator(counter as i32) as i64;
        section.size = size;

        let mut stream_data = Vec::new();
        for sec in &self.local_sections {
            stream_data.extend_from_slice(&sec.page_number.to_le_bytes());
            stream_data.extend_from_slice(&(sec.size as i32).to_le_bytes());
        }

        self.compress_checksum(&mut section, &stream_data)?;

        let last = self.local_sections.last().unwrap().clone();
        self.gap_amount = 0;
        self.last_page_id = last.page_number;
        self.last_section_addr = (last.seeker + size - 256) as u64;
        self.section_amount = (self.local_sections.len() - 1) as u32;
        self.page_map_address = section.seeker as u64;

        Ok(())
    }

    fn write_file_meta_data(&mut self) -> Result<()> {
        self.second_header_addr = self.stream.position();

        let file_header_data = self.build_file_header();
        self.stream.write_all(&file_header_data)?;

        // Write version string at position 0
        self.stream.seek(SeekFrom::Start(0))?;
        let ver_bytes = self.version_string.as_bytes();
        self.stream.write_all(&ver_bytes[..6.min(ver_bytes.len())])?;

        // 5 bytes of 0
        self.stream.write_all(&[0u8; 5])?;
        // Maintenance release version
        self.stream.write_all(&[self.maintenance_version as u8])?;
        // 0x03
        self.stream.write_all(&[3])?;

        // Preview address
        let preview_seeker = self
            .find_descriptor(DwgSectionDefinition::PREVIEW)
            .and_then(|d| d.local_sections.first())
            .map(|ls| ls.seeker as u32 + 0x20)
            .unwrap_or(0);
        self.stream.write_all(&preview_seeker.to_le_bytes())?;

        // DWG version
        self.stream.write_all(&[33])?;
        // App maintenance release version
        self.stream
            .write_all(&[self.maintenance_version as u8])?;

        // Codepage
        let cp = get_file_code_page(&self.code_page);
        self.stream.write_all(&cp.to_le_bytes())?;
        // 3 zero bytes
        self.stream.write_all(&[0u8; 3])?;

        // SecurityType
        self.stream.write_all(&0i32.to_le_bytes())?;
        // Unknown long
        self.stream.write_all(&0i32.to_le_bytes())?;

        // Summary info address
        let summary_seeker = self
            .find_descriptor(DwgSectionDefinition::SUMMARY_INFO)
            .and_then(|d| d.local_sections.first())
            .map(|ls| ls.seeker as u32 + 0x20)
            .unwrap_or(0);
        self.stream.write_all(&summary_seeker.to_le_bytes())?;

        // VBA Project Addr (0)
        self.stream.write_all(&0u32.to_le_bytes())?;
        // 0x80
        self.stream.write_all(&0x80i32.to_le_bytes())?;

        // App info address
        let app_seeker = self
            .find_descriptor(DwgSectionDefinition::APP_INFO)
            .and_then(|d| d.local_sections.first())
            .map(|ls| ls.seeker as u32 + 0x20)
            .unwrap_or(0);
        self.stream.write_all(&app_seeker.to_le_bytes())?;

        // 80 zero bytes
        self.stream.write_all(&[0u8; 80])?;

        self.stream.write_all(&file_header_data)?;

        let magic = &*MAGIC_SEQUENCE;
        self.stream.write_all(&magic[236..256])?;

        Ok(())
    }

    fn build_file_header(&self) -> Vec<u8> {
        let mut stream = Cursor::new(Vec::with_capacity(0x6C));
        let mut crc_handler = Crc32StreamHandler::new(&mut stream, 0);

        // "AcFssFcAJMB" + null
        let id = b"AcFssFcAJMB\0";
        let _ = crc_handler.write_all(id);

        // Various header fields
        let _ = crc_handler.write_all(&0i32.to_le_bytes()); // 0x00
        let _ = crc_handler.write_all(&0x6Ci32.to_le_bytes()); // 0x6c
        let _ = crc_handler.write_all(&4i32.to_le_bytes()); // 0x04
        let _ = crc_handler.write_all(&self.root_tree_node_gap.to_le_bytes());
        let _ = crc_handler.write_all(&self.left_gap.to_le_bytes());
        let _ = crc_handler.write_all(&self.right_gap.to_le_bytes());
        let _ = crc_handler.write_all(&1i32.to_le_bytes()); // unknown = 1
        let _ = crc_handler.write_all(&self.last_page_id.to_le_bytes());
        let _ = crc_handler.write_all(&self.last_section_addr.to_le_bytes());
        let _ = crc_handler.write_all(&self.second_header_addr.to_le_bytes());
        let _ = crc_handler.write_all(&self.gap_amount.to_le_bytes());
        let _ = crc_handler.write_all(&self.section_amount.to_le_bytes());
        let _ = crc_handler.write_all(&0x20i32.to_le_bytes());
        let _ = crc_handler.write_all(&0x80i32.to_le_bytes());
        let _ = crc_handler.write_all(&0x40i32.to_le_bytes());
        let _ = crc_handler.write_all(&self.section_page_map_id.to_le_bytes());
        let _ = crc_handler.write_all(
            &(self.page_map_address.wrapping_sub(256)).to_le_bytes(),
        );
        let _ = crc_handler.write_all(&self.section_map_id.to_le_bytes());
        let _ = crc_handler.write_all(&self.section_array_page_size.to_le_bytes());
        let _ = crc_handler.write_all(&self.gap_array_size.to_le_bytes());

        // CRC placeholder: write 0, then get seed and overwrite
        let crc_pos = crc_handler.stream_position().unwrap_or(0);
        let _ = crc_handler.write_all(&0u32.to_le_bytes());
        let seed = crc_handler.seed();

        // Go back and write the real CRC
        let _ = crc_handler.seek(SeekFrom::Start(crc_pos));
        let _ = crc_handler.write_all(&seed.to_le_bytes());
        let _ = crc_handler.flush();

        let mut buf = stream.into_inner();
        apply_magic_sequence(&mut buf);
        buf
    }

    fn set_seeker(
        &mut self,
        map_value: i32,
        stream_data: &[u8],
    ) -> Result<DwgLocalSectionMap> {
        let mut holder = DwgLocalSectionMap::with_section_map(map_value);

        let pos = self.stream.position();
        write_magic_number(&mut self.stream, pos);
        holder.seeker = self.stream.position() as i64;

        self.compress_checksum_holder(&mut holder, stream_data)?;

        Ok(holder)
    }

    fn compress_checksum(
        &mut self,
        section: &mut DwgLocalSectionMap,
        stream_data: &[u8],
    ) -> Result<()> {
        self.compress_checksum_holder(section, stream_data)
    }

    fn compress_checksum_holder(
        &mut self,
        section: &mut DwgLocalSectionMap,
        stream_data: &[u8],
    ) -> Result<()> {
        section.decompressed_size = stream_data.len() as u64;

        let mut compressed = Vec::new();
        let mut compressor = DwgLz77Ac18Compressor::new();
        compressor.compress(stream_data, 0, stream_data.len(), &mut compressed);

        section.compressed_size = compressed.len() as u64;

        let mut checksum_data = Vec::new();
        Self::write_page_header_data_to(&mut checksum_data, section);
        section.checksum =
            calculate(0, &checksum_data, 0, checksum_data.len()) as u64;
        section.checksum = calculate(
            section.checksum as u32,
            &compressed,
            0,
            compressed.len(),
        ) as u64;

        let mut final_header = Vec::new();
        Self::write_page_header_data_to(&mut final_header, section);
        self.stream.write_all(&final_header)?;
        self.stream.write_all(&compressed)?;

        Ok(())
    }

    fn add_section_internal(&mut self, section: DwgLocalSectionMap) {
        let page_number = self.local_sections.len() as i32 + 1;
        let mut sec = section;
        sec.page_number = page_number;
        self.local_sections.push(sec);
    }

    fn write_page_header_data_to(dest: &mut Vec<u8>, section: &DwgLocalSectionMap) {
        dest.extend_from_slice(&section.section_map.to_le_bytes());
        dest.extend_from_slice(&(section.decompressed_size as i32).to_le_bytes());
        dest.extend_from_slice(&(section.compressed_size as i32).to_le_bytes());
        dest.extend_from_slice(&section.compression.to_le_bytes());
        dest.extend_from_slice(&(section.checksum as u32).to_le_bytes());
    }

    fn write_data_section_to(
        dest: &mut Vec<u8>,
        section_id: i32,
        map: &DwgLocalSectionMap,
        size: i32,
    ) {
        dest.extend_from_slice(&size.to_le_bytes()); // page type
        dest.extend_from_slice(&section_id.to_le_bytes());
        dest.extend_from_slice(&(map.compressed_size as i32).to_le_bytes());
        dest.extend_from_slice(&(map.page_size as i32).to_le_bytes());
        dest.extend_from_slice(&(map.offset as i64).to_le_bytes());
        dest.extend_from_slice(&(map.checksum as u32).to_le_bytes());
        dest.extend_from_slice(&map.oda.to_le_bytes());
    }

    /// Consume and return the output bytes.
    pub fn into_inner(self) -> Vec<u8> {
        self.stream.into_inner()
    }
}

impl DwgFileHeaderWriter for DwgFileHeaderWriterAc18 {
    fn handle_section_offset(&self) -> i32 {
        0
    }

    fn add_section(
        &mut self,
        name: &str,
        stream: Vec<u8>,
        is_compressed: bool,
        decomp_size: usize,
    ) {
        let mut descriptor = DwgSectionDescriptor::with_name(name);
        let decomp_size = if decomp_size == 0 { 0x7400 } else { decomp_size };
        descriptor.decompressed_size = decomp_size as u64;
        descriptor.compressed_size = stream.len() as u64;
        descriptor.set_compressed_code(if is_compressed { 2 } else { 1 });

        let n_local = stream.len() / decomp_size;
        let mut offset = 0usize;

        // We must add the descriptor first so create_local_section can find it
        self.descriptors.push((name.to_string(), descriptor));

        for _ in 0..n_local {
            let _ = self.create_local_section(
                name,
                &stream,
                decomp_size,
                offset,
                decomp_size,
                is_compressed,
            );
            offset += decomp_size;
        }

        let spare_bytes = stream.len() % decomp_size;
        if spare_bytes > 0 && !check_empty_bytes(&stream, offset, spare_bytes) {
            let _ = self.create_local_section(
                name,
                &stream,
                decomp_size,
                offset,
                spare_bytes,
                is_compressed,
            );
        }
    }

    fn write_file(&mut self) -> Result<()> {
        self.section_array_page_size = (self.local_sections.len() as u32) + 2;
        self.section_page_map_id = self.section_array_page_size;
        self.section_map_id = self.section_array_page_size - 1;

        self.write_descriptors()?;
        self.write_records()?;
        self.write_file_meta_data()?;

        Ok(())
    }
}
