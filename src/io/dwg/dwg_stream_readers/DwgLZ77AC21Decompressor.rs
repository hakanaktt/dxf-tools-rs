use crate::error::Result;

/// LZ77 variant used by AC1021+ (DWG 2007 and newer).
///
/// The literal data copy uses a byte-reordering scheme that matches the C#
/// ACadSharp implementation. Each 32-byte chunk is reordered, and remainder
/// chunks use specific sub-byte block copy functions.
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

        Self::copy_reordered(source, state.source_index, dest, *index, state.length);

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
        let initial_index = (dst_index.wrapping_sub(src_offset)) as usize;
        let max_index = initial_index + length as usize;
        let mut src_idx = initial_index;
        let mut dst_pos = dst_index as usize;

        while src_idx < max_index && dst_pos < dst.len() {
            let b = dst.get(src_idx).copied().unwrap_or(0);
            dst[dst_pos] = b;
            src_idx += 1;
            dst_pos += 1;
        }
    }

    // ─── Byte-reordering copy (literal data) ───────────────────────────
    // The AC21 format stores literal data in a reordered form.
    // Each 32-byte chunk has its 4-byte groups rearranged:
    //   src[24..28] → dst[0..4], src[28..32] → dst[4..8],
    //   src[16..20] → dst[8..12], src[20..24] → dst[12..16],
    //   src[8..12]  → dst[16..20], src[12..16] → dst[20..24],
    //   src[0..4]   → dst[24..28], src[4..8]   → dst[28..32]
    // Remainders use sub-byte copy functions with specific ordering.

    fn copy_reordered(src: &[u8], src_index: u32, dst: &mut [u8], dst_index: u32, mut length: u32) {
        let mut si = src_index as usize;
        let mut di = dst_index as usize;

        // Copy full 32-byte chunks with reordering
        while length >= 32 {
            Self::copy4b(src, si + 24, dst, di);
            Self::copy4b(src, si + 28, dst, di + 4);
            Self::copy4b(src, si + 16, dst, di + 8);
            Self::copy4b(src, si + 20, dst, di + 12);
            Self::copy4b(src, si + 8, dst, di + 16);
            Self::copy4b(src, si + 12, dst, di + 20);
            Self::copy4b(src, si, dst, di + 24);
            Self::copy4b(src, si + 4, dst, di + 28);
            si += 32;
            di += 32;
            length -= 32;
        }

        if length == 0 {
            return;
        }

        // Remainder copy with reordering (matches C# m_copyMethods delegate table)
        match length {
            1 => Self::copy1b(src, si, dst, di),
            2 => Self::copy2b(src, si, dst, di),
            3 => Self::copy3b(src, si, dst, di),
            4 => Self::copy4b(src, si, dst, di),
            5 => {
                Self::copy1b(src, si + 4, dst, di);
                Self::copy4b(src, si, dst, di + 1);
            }
            6 => {
                Self::copy1b(src, si + 5, dst, di);
                Self::copy4b(src, si + 1, dst, di + 1);
                Self::copy1b(src, si, dst, di + 5);
            }
            7 => {
                Self::copy2b(src, si + 5, dst, di);
                Self::copy4b(src, si + 1, dst, di + 2);
                Self::copy1b(src, si, dst, di + 6);
            }
            8 => Self::copy8b(src, si, dst, di),
            9 => {
                Self::copy1b(src, si + 8, dst, di);
                Self::copy8b(src, si, dst, di + 1);
            }
            10 => {
                Self::copy1b(src, si + 9, dst, di);
                Self::copy8b(src, si + 1, dst, di + 1);
                Self::copy1b(src, si, dst, di + 9);
            }
            11 => {
                Self::copy2b(src, si + 9, dst, di);
                Self::copy8b(src, si + 1, dst, di + 2);
                Self::copy1b(src, si, dst, di + 10);
            }
            12 => {
                Self::copy4b(src, si + 8, dst, di);
                Self::copy8b(src, si, dst, di + 4);
            }
            13 => {
                Self::copy1b(src, si + 12, dst, di);
                Self::copy4b(src, si + 8, dst, di + 1);
                Self::copy8b(src, si, dst, di + 5);
            }
            14 => {
                Self::copy1b(src, si + 13, dst, di);
                Self::copy4b(src, si + 9, dst, di + 1);
                Self::copy8b(src, si + 1, dst, di + 5);
                Self::copy1b(src, si, dst, di + 13);
            }
            15 => {
                Self::copy2b(src, si + 13, dst, di);
                Self::copy4b(src, si + 9, dst, di + 2);
                Self::copy8b(src, si + 1, dst, di + 6);
                Self::copy1b(src, si, dst, di + 14);
            }
            16 => Self::copy16b(src, si, dst, di),
            17 => {
                Self::copy8b(src, si + 9, dst, di);
                Self::copy1b(src, si + 8, dst, di + 8);
                Self::copy8b(src, si, dst, di + 9);
            }
            18 => {
                Self::copy1b(src, si + 17, dst, di);
                Self::copy16b(src, si + 1, dst, di + 1);
                Self::copy1b(src, si, dst, di + 17);
            }
            19 => {
                Self::copy3b(src, si + 16, dst, di);
                Self::copy16b(src, si, dst, di + 3);
            }
            20 => {
                Self::copy4b(src, si + 16, dst, di);
                Self::copy8b(src, si + 8, dst, di + 4);
                Self::copy8b(src, si, dst, di + 12);
            }
            21 => {
                Self::copy1b(src, si + 20, dst, di);
                Self::copy4b(src, si + 16, dst, di + 1);
                Self::copy8b(src, si + 8, dst, di + 5);
                Self::copy8b(src, si, dst, di + 13);
            }
            22 => {
                Self::copy2b(src, si + 20, dst, di);
                Self::copy4b(src, si + 16, dst, di + 2);
                Self::copy8b(src, si + 8, dst, di + 6);
                Self::copy8b(src, si, dst, di + 14);
            }
            23 => {
                Self::copy3b(src, si + 20, dst, di);
                Self::copy4b(src, si + 16, dst, di + 3);
                Self::copy8b(src, si + 8, dst, di + 7);
                Self::copy8b(src, si, dst, di + 15);
            }
            24 => {
                Self::copy8b(src, si + 16, dst, di);
                Self::copy16b(src, si, dst, di + 8);
            }
            25 => {
                Self::copy8b(src, si + 17, dst, di);
                Self::copy1b(src, si + 16, dst, di + 8);
                Self::copy16b(src, si, dst, di + 9);
            }
            26 => {
                Self::copy1b(src, si + 25, dst, di);
                Self::copy8b(src, si + 17, dst, di + 1);
                Self::copy1b(src, si + 16, dst, di + 9);
                Self::copy16b(src, si, dst, di + 10);
            }
            27 => {
                Self::copy2b(src, si + 25, dst, di);
                Self::copy8b(src, si + 17, dst, di + 2);
                Self::copy1b(src, si + 16, dst, di + 10);
                Self::copy16b(src, si, dst, di + 11);
            }
            28 => {
                Self::copy4b(src, si + 24, dst, di);
                Self::copy8b(src, si + 16, dst, di + 4);
                Self::copy8b(src, si + 8, dst, di + 12);
                Self::copy8b(src, si, dst, di + 20);
            }
            29 => {
                Self::copy1b(src, si + 28, dst, di);
                Self::copy4b(src, si + 24, dst, di + 1);
                Self::copy8b(src, si + 16, dst, di + 5);
                Self::copy8b(src, si + 8, dst, di + 13);
                Self::copy8b(src, si, dst, di + 21);
            }
            30 => {
                Self::copy2b(src, si + 28, dst, di);
                Self::copy4b(src, si + 24, dst, di + 2);
                Self::copy8b(src, si + 16, dst, di + 6);
                Self::copy8b(src, si + 8, dst, di + 14);
                Self::copy8b(src, si, dst, di + 22);
            }
            31 => {
                Self::copy1b(src, si + 30, dst, di);
                Self::copy4b(src, si + 26, dst, di + 1);
                Self::copy8b(src, si + 18, dst, di + 5);
                Self::copy8b(src, si + 10, dst, di + 13);
                Self::copy8b(src, si + 2, dst, di + 21);
                Self::copy2b(src, si, dst, di + 29);
            }
            _ => unreachable!(),
        }
    }

    #[inline]
    fn copy1b(src: &[u8], si: usize, dst: &mut [u8], di: usize) {
        dst[di] = src[si];
    }

    #[inline]
    fn copy2b(src: &[u8], si: usize, dst: &mut [u8], di: usize) {
        // 2-byte reverse
        dst[di] = src[si + 1];
        dst[di + 1] = src[si];
    }

    #[inline]
    fn copy3b(src: &[u8], si: usize, dst: &mut [u8], di: usize) {
        // 3-byte reverse
        dst[di] = src[si + 2];
        dst[di + 1] = src[si + 1];
        dst[di + 2] = src[si];
    }

    #[inline]
    fn copy4b(src: &[u8], si: usize, dst: &mut [u8], di: usize) {
        // 4-byte straight copy (no reorder)
        dst[di] = src[si];
        dst[di + 1] = src[si + 1];
        dst[di + 2] = src[si + 2];
        dst[di + 3] = src[si + 3];
    }

    #[inline]
    fn copy8b(src: &[u8], si: usize, dst: &mut [u8], di: usize) {
        // Two 4-byte straight copies
        Self::copy4b(src, si, dst, di);
        Self::copy4b(src, si + 4, dst, di + 4);
    }

    #[inline]
    fn copy16b(src: &[u8], si: usize, dst: &mut [u8], di: usize) {
        // Swap two 8-byte halves
        Self::copy8b(src, si + 8, dst, di);
        Self::copy8b(src, si, dst, di + 8);
    }

    pub fn decompress_into_new(source: &[u8], initial_offset: u32, length: u32, out_len: usize) -> Result<Vec<u8>> {
        let mut out = vec![0u8; out_len];
        Self::decompress(source, initial_offset, length, &mut out);
        Ok(out)
    }
}
