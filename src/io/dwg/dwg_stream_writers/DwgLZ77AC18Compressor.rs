//! LZ77 compressor for DWG AC18 (R2004) format.

use super::idwg_stream_writer::Compressor;

pub struct DwgLz77Ac18Compressor {
    source: Vec<u8>,
    block: [i32; 0x8000],
    initial_offset: usize,
    curr_position: usize,
    curr_offset: usize,
    total_offset: usize,
}

impl DwgLz77Ac18Compressor {
    pub fn new() -> Self {
        Self {
            source: Vec::new(),
            block: [-1i32; 0x8000],
            initial_offset: 0,
            curr_position: 0,
            curr_offset: 0,
            total_offset: 0,
        }
    }

    fn restart_block(&mut self) {
        for i in 0..self.block.len() {
            self.block[i] = -1;
        }
    }

    fn write_len(dest: &mut Vec<u8>, mut len: i32) {
        assert!(len > 0);
        while len > 0xFF {
            len -= 0xFF;
            dest.push(0);
        }
        dest.push(len as u8);
    }

    fn write_op_code(dest: &mut Vec<u8>, op_code: i32, compression_offset: i32, value: i32) {
        assert!(compression_offset > 0);
        assert!(value > 0);

        if compression_offset <= value {
            dest.push((op_code | (compression_offset - 2)) as u8);
        } else {
            dest.push(op_code as u8);
            Self::write_len(dest, compression_offset - value);
        }
    }

    fn write_literal_length(&self, dest: &mut Vec<u8>, length: i32) {
        if length <= 0 {
            return;
        }

        if length > 3 {
            Self::write_op_code(dest, 0, length - 1, 0x11);
        }
        let mut num = self.curr_offset;
        for _ in 0..length {
            dest.push(self.source[num]);
            num += 1;
        }
    }

    fn apply_mask(
        &self,
        dest: &mut Vec<u8>,
        mut match_position: i32,
        compression_offset: i32,
        mask: i32,
    ) {
        let curr;
        let next;

        if compression_offset >= 0x0F || match_position > 0x400 {
            if match_position <= 0x4000 {
                match_position -= 1;
                Self::write_op_code(dest, 0x20, compression_offset, 0x21);
            } else {
                match_position -= 0x4000;
                Self::write_op_code(
                    dest,
                    0x10 | ((match_position >> 11) & 8),
                    compression_offset,
                    0x09,
                );
            }
            curr = (match_position & 0xFF) << 2;
            next = match_position >> 6;
        } else {
            match_position -= 1;
            curr = ((compression_offset + 1) << 4) | ((match_position & 0b11) << 2);
            next = match_position >> 2;
        }

        let curr = if mask < 4 { curr | mask } else { curr };

        dest.push(curr as u8);
        dest.push(next as u8);
    }

    fn compress_chunk(&mut self) -> Option<(i32, i32)> {
        let src = &self.source;
        let cp = self.curr_position;

        let v1 = (src[cp + 3] as i32) << 6;
        let v2 = v1 ^ (src[cp + 2] as i32);
        let v3 = (v2 << 5) ^ (src[cp + 1] as i32);
        let v4 = (v3 << 5) ^ (src[cp] as i32);
        let mut value_index = ((v4 + (v4 >> 5)) & 0x7FFF) as usize;

        let mut value = self.block[value_index];
        let mut match_pos = (cp as i32) - value;

        if value >= self.initial_offset as i32 && match_pos <= 0xBFFF {
            if match_pos > 0x400
                && src[cp + 3] != src[value as usize + 3]
            {
                value_index = (value_index & 0x7FF) ^ 0b100000000011111;
                value = self.block[value_index];
                match_pos = (cp as i32) - value;
                if value < self.initial_offset as i32
                    || match_pos > 0xBFFF
                    || (match_pos > 0x400
                        && src[cp + 3] != src[value as usize + 3])
                {
                    self.block[value_index] = cp as i32;
                    return None;
                }
            }
            if src[cp] == src[value as usize]
                && src[cp + 1] == src[value as usize + 1]
                && src[cp + 2] == src[value as usize + 2]
            {
                let mut offset = 3i32;
                let mut index = value as usize + 3;
                let mut curr_off = cp + 3;
                while curr_off < self.total_offset
                    && src[index] == src[curr_off]
                {
                    offset += 1;
                    index += 1;
                    curr_off += 1;
                }

                self.block[value_index] = cp as i32;
                if offset >= 3 {
                    return Some((offset, match_pos));
                }
                return None;
            }
        }

        self.block[value_index] = cp as i32;
        None
    }
}

impl Compressor for DwgLz77Ac18Compressor {
    fn compress(
        &mut self,
        source: &[u8],
        offset: usize,
        total_size: usize,
        dest: &mut Vec<u8>,
    ) {
        self.restart_block();

        self.source = source.to_vec();
        self.initial_offset = offset;
        self.total_offset = offset + total_size;
        self.curr_offset = offset;
        self.curr_position = offset + 4;

        let mut compression_offset: i32 = 0;
        let mut match_pos: i32 = 0;

        while self.curr_position < self.total_offset.saturating_sub(0x13) {
            if let Some((curr_offset, last_match_pos)) = self.compress_chunk() {
                let mask = (self.curr_position - self.curr_offset) as i32;

                if compression_offset != 0 {
                    self.apply_mask(dest, match_pos, compression_offset, mask);
                }

                self.write_literal_length(dest, mask);
                self.curr_position += curr_offset as usize;
                self.curr_offset = self.curr_position;
                compression_offset = curr_offset;
                match_pos = last_match_pos;
            } else {
                self.curr_position += 1;
            }
        }

        let literal_length = (self.total_offset - self.curr_offset) as i32;

        if compression_offset != 0 {
            self.apply_mask(dest, match_pos, compression_offset, literal_length);
        }

        self.write_literal_length(dest, literal_length);

        // 0x11: Terminates the input stream
        dest.push(0x11);
        dest.push(0);
        dest.push(0);
    }
}
