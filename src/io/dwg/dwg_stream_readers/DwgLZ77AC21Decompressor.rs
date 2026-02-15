use crate::error::Result;

/// LZ77 variant used by AC1021+ (DWG 2007 and newer).
pub struct DwgLz77Ac21Decompressor;

#[derive(Debug, Default)]
struct DecodeState {
    source_offset: u32,
    length: u32,
    source_index: u32,
    op_code: u32,
}

impl DwgLz77Ac21Decompressor {
    pub fn decompress(source: &[u8], initial_offset: u32, length: u32, buffer: &mut [u8]) {
        let mut state = DecodeState {
            source_offset: 0,
            length: 0,
            source_index: initial_offset,
            op_code: source[initial_offset as usize] as u32,
        };

        let mut dest_index = 0u32;
        let end_index = state.source_index + length;
        state.source_index += 1;

        if state.source_index >= end_index {
            return;
        }

        if (state.op_code & 0xF0) == 0x20 {
            state.source_index += 3;
            state.length = source[(state.source_index - 1) as usize] as u32 & 7;
        }

        while state.source_index < end_index {
            Self::next_index(&mut state, source, buffer, &mut dest_index);
            if state.source_index >= end_index {
                break;
            }
            dest_index = Self::copy_decompressed_chunks(&mut state, source, end_index, buffer, dest_index);
        }
    }

    fn next_index(state: &mut DecodeState, source: &[u8], dest: &mut [u8], index: &mut u32) {
        if state.length == 0 {
            Self::read_literal_length(state, source);
        }

        let len = state.length as usize;
        let src_start = state.source_index as usize;
        let dst_start = *index as usize;

        if src_start + len <= source.len() && dst_start + len <= dest.len() {
            dest[dst_start..dst_start + len].copy_from_slice(&source[src_start..src_start + len]);
        }

        state.source_index += state.length;
        *index += state.length;
    }

    fn copy_decompressed_chunks(
        state: &mut DecodeState,
        src: &[u8],
        end_index: u32,
        dst: &mut [u8],
        mut dest_index: u32,
    ) -> u32 {
        state.length = 0;
        state.op_code = src[state.source_index as usize] as u32;
        state.source_index += 1;

        Self::read_instructions(state, src);

        loop {
            Self::copy_bytes(dst, dest_index, state.length, state.source_offset);
            dest_index += state.length;

            state.length = state.op_code & 0x07;

            if state.length != 0 || state.source_index >= end_index {
                break;
            }

            state.op_code = src[state.source_index as usize] as u32;
            state.source_index += 1;

            if (state.op_code >> 4) == 0 {
                break;
            }

            if (state.op_code >> 4) == 15 {
                state.op_code &= 15;
            }

            Self::read_instructions(state, src);
        }

        dest_index
    }

    fn read_instructions(state: &mut DecodeState, buffer: &[u8]) {
        match state.op_code >> 4 {
            0 => {
                state.length = (state.op_code & 0xF) + 0x13;
                state.source_offset = buffer[state.source_index as usize] as u32;
                state.source_index += 1;
                state.op_code = buffer[state.source_index as usize] as u32;
                state.source_index += 1;
                state.length = ((state.op_code >> 3) & 0x10) + state.length;
                state.source_offset = ((state.op_code & 0x78) << 5) + 1 + state.source_offset;
            }
            1 => {
                state.length = (state.op_code & 0xF) + 3;
                state.source_offset = buffer[state.source_index as usize] as u32;
                state.source_index += 1;
                state.op_code = buffer[state.source_index as usize] as u32;
                state.source_index += 1;
                state.source_offset = ((state.op_code & 0xF8) << 5) + 1 + state.source_offset;
            }
            2 => {
                state.source_offset = buffer[state.source_index as usize] as u32;
                state.source_index += 1;
                state.source_offset |= (buffer[state.source_index as usize] as u32) << 8;
                state.source_index += 1;

                state.length = state.op_code & 7;

                if (state.op_code & 8) == 0 {
                    state.op_code = buffer[state.source_index as usize] as u32;
                    state.source_index += 1;
                    state.length = (state.op_code & 0xF8) + state.length;
                } else {
                    state.source_offset += 1;
                    state.length = ((buffer[state.source_index as usize] as u32) << 3) + state.length;
                    state.source_index += 1;
                    state.op_code = buffer[state.source_index as usize] as u32;
                    state.source_index += 1;
                    state.length = ((state.op_code & 0xF8) << 8) + state.length + 0x100;
                }
            }
            _ => {
                state.length = state.op_code >> 4;
                state.source_offset = state.op_code & 0x0F;
                state.op_code = buffer[state.source_index as usize] as u32;
                state.source_index += 1;
                state.source_offset = ((state.op_code & 0xF8) << 1) + state.source_offset + 1;
            }
        }
    }

    fn read_literal_length(state: &mut DecodeState, buffer: &[u8]) {
        state.length = state.op_code + 8;

        if state.length == 0x17 {
            let mut n = buffer[state.source_index as usize] as u32;
            state.source_index += 1;
            state.length += n;

            if n == 0xFF {
                loop {
                    n = buffer[state.source_index as usize] as u32;
                    state.source_index += 1;
                    n |= (buffer[state.source_index as usize] as u32) << 8;
                    state.source_index += 1;
                    state.length += n;

                    if n != 0xFFFF {
                        break;
                    }
                }
            }
        }
    }

    fn copy_bytes(dst: &mut [u8], dst_index: u32, length: u32, src_offset: u32) {
        let mut src_index = dst_index.saturating_sub(src_offset) as usize;
        let mut dst_pos = dst_index as usize;
        let max_index = src_index.saturating_add(length as usize);

        while src_index < max_index && dst_pos < dst.len() {
            let b = dst.get(src_index).copied().unwrap_or(0);
            dst[dst_pos] = b;
            src_index += 1;
            dst_pos += 1;
        }
    }

    pub fn decompress_into_new(source: &[u8], initial_offset: u32, length: u32, out_len: usize) -> Result<Vec<u8>> {
        let mut out = vec![0u8; out_len];
        Self::decompress(source, initial_offset, length, &mut out);
        Ok(out)
    }
}
