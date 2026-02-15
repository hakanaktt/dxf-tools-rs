//! DWG checksum (Adler-like) calculator and magic sequence generator.
//!
//! Ported from ACadSharp `DwgCheckSumCalculator.cs`.

use once_cell::sync::Lazy;
use std::cmp;

/// Pre-computed 256-byte magic sequence used for DWG section encoding.
///
/// Generated from a linear congruential generator with:
/// - multiplier: `0x343FD`
/// - increment:  `0x269EC3`
/// - initial seed: `1`
///
/// Each byte is `(seed >> 16) & 0xFF` after advancing the generator.
pub static MAGIC_SEQUENCE: Lazy<[u8; 256]> = Lazy::new(|| {
    let mut seq = [0u8; 256];
    let mut rand_seed: i32 = 1;
    for byte in seq.iter_mut() {
        rand_seed = rand_seed.wrapping_mul(0x343FD);
        rand_seed = rand_seed.wrapping_add(0x269EC3);
        *byte = (rand_seed >> 0x10) as u8;
    }
    seq
});

/// Calculate the number of padding bytes needed for 32-byte alignment.
///
/// Returns `0x1F - (length + 0x20 - 1) % 0x20`, i.e. the number of bytes
/// to append so that the total length is a multiple of 32.
pub fn compression_calculator(length: i32) -> i32 {
    0x1F - (length + 0x20 - 1) % 0x20
}

/// Adler-like checksum used in DWG section data.
///
/// This is a modified Adler-32 with modulus `0xFFF1` and a chunk size of
/// `0x15B0` (5552) — identical to zlib's Adler-32 implementation.
///
/// # Arguments
///
/// * `seed`   - Initial checksum value (lower 16 bits = sum1, upper 16 bits = sum2).
/// * `buffer` - Source data.
/// * `offset` - Starting byte offset into `buffer`.
/// * `size`   - Number of bytes to process.
pub fn calculate(seed: u32, buffer: &[u8], offset: usize, size: usize) -> u32 {
    let mut sum1 = seed & 0xFFFF;
    let mut sum2 = seed >> 16;
    let mut index = offset;
    let mut remaining = size;

    while remaining != 0 {
        let chunk_size = cmp::min(0x15B0, remaining);
        remaining -= chunk_size;

        for _ in 0..chunk_size {
            sum1 += buffer[index] as u32;
            sum2 += sum1;
            index += 1;
        }

        sum1 %= 0xFFF1;
        sum2 %= 0xFFF1;
    }

    (sum2 << 16) | (sum1 & 0xFFFF)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_magic_sequence_first_bytes() {
        // Verify the sequence is deterministic
        let seq = &*MAGIC_SEQUENCE;
        assert_eq!(seq.len(), 256);
        // First iteration: seed = 1 * 0x343FD + 0x269EC3 = 0x29D303
        // byte = (0x29D303 >> 16) = 0x29 = 41
        assert_eq!(seq[0], 0x29);
    }

    #[test]
    fn test_compression_calculator() {
        // 0 bytes → 0x1F - (0 + 0x1F) % 0x20 = 0x1F - 31 = 0
        assert_eq!(compression_calculator(0), 0);
        // 1 byte → 0x1F - (1 + 0x1F) % 0x20 = 0x1F - 0 = 31
        assert_eq!(compression_calculator(1), 0x1F);
        // 32 bytes → 0x1F - (32 + 0x1F) % 0x20 = 0x1F - 31 = 0
        assert_eq!(compression_calculator(32), 0);
        // 33 bytes → 0x1F - (33 + 0x1F) % 0x20 = 0x1F - 0 = 31
        assert_eq!(compression_calculator(33), 0x1F);
    }

    #[test]
    fn test_calculate_empty() {
        // Adler of zero bytes with seed 1 should return 1
        let result = calculate(1, &[], 0, 0);
        assert_eq!(result, 1);
    }

    #[test]
    fn test_calculate_known() {
        // "ABC" with seed = 0x0001_0001 (sum1=1, sum2=1)
        let data = b"ABC";
        let result = calculate(0x0001_0001, data, 0, data.len());

        // sum1 = (1+65+66+67) % 0xFFF1 = 199
        // sum2 = (1 + (1+65) + (1+65+66) + (1+65+66+67)) % 0xFFF1
        //      = (1 + 66 + 132 + 199) % 0xFFF1 = 398
        assert_eq!(result & 0xFFFF, 199);
        assert_eq!(result >> 16, 398);
    }
}
