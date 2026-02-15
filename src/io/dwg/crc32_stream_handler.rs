//! CRC-32 stream wrapper for DWG integrity checking.
//!
//! Ported from ACadSharp `CRC32StreamHandler.cs`.

use std::io::{self, Cursor, Read, Seek, SeekFrom, Write};

use super::crc::CRC32_TABLE;

/// A stream wrapper that computes a running CRC-32 over all bytes read/written.
///
/// The CRC seed is stored in bit-inverted form internally (like the C# version)
/// and exposed via [`Crc32StreamHandler::seed`] with the final inversion applied.
pub struct Crc32StreamHandler<S> {
    stream: S,
    /// Internally stored as `!seed` (inverted).
    inverted_seed: u32,
}

impl<S> Crc32StreamHandler<S> {
    /// Create a CRC-32 stream handler wrapping the given stream.
    pub fn new(stream: S, seed: u32) -> Self {
        Self {
            stream,
            inverted_seed: !seed,
        }
    }

    /// Get the current CRC-32 value (with final bit inversion).
    pub fn seed(&self) -> u32 {
        !self.inverted_seed
    }

    /// Consume the wrapper and return the inner stream.
    pub fn into_inner(self) -> S {
        self.stream
    }

    /// Get a reference to the inner stream.
    pub fn inner(&self) -> &S {
        &self.stream
    }

    /// Get a mutable reference to the inner stream.
    pub fn inner_mut(&mut self) -> &mut S {
        &mut self.stream
    }
}

impl Crc32StreamHandler<Cursor<Vec<u8>>> {
    /// Constructor that creates a magic-sequence-decoded stream from a byte array.
    ///
    /// Equivalent to the `CRC32StreamHandler(byte[] arr, uint seed)` constructor
    /// in ACadSharp: XORs each byte with a pseudo-random sequence, then wraps
    /// the result as a `Cursor<Vec<u8>>`.
    pub fn from_magic_bytes(mut arr: Vec<u8>, seed: u32) -> Self {
        let mut rand_seed: i32 = 1;
        for byte in arr.iter_mut() {
            rand_seed = rand_seed.wrapping_mul(0x343FD);
            rand_seed = rand_seed.wrapping_add(0x269EC3);
            let mask = (rand_seed >> 0x10) as u8;
            *byte ^= mask;
        }
        Self {
            stream: Cursor::new(arr),
            inverted_seed: !seed,
        }
    }
}

impl<S: Read> Read for Crc32StreamHandler<S> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.stream.read(buf)?;
        for &byte in &buf[..n] {
            self.inverted_seed = (self.inverted_seed >> 8)
                ^ CRC32_TABLE[((self.inverted_seed ^ byte as u32) & 0xFF) as usize];
        }
        Ok(n)
    }
}

impl<S: Write> Write for Crc32StreamHandler<S> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        for &byte in buf {
            self.inverted_seed = (self.inverted_seed >> 8)
                ^ CRC32_TABLE[((self.inverted_seed ^ byte as u32) & 0xFF) as usize];
        }
        self.stream.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stream.flush()
    }
}

impl<S: Seek> Seek for Crc32StreamHandler<S> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.stream.seek(pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::dwg::crc::crc32_update;

    #[test]
    fn test_seed_inversion() {
        let handler = Crc32StreamHandler::new(Cursor::new(Vec::<u8>::new()), 0x12345678);
        assert_eq!(handler.seed(), 0x12345678);
    }

    #[test]
    fn test_read_updates_crc() {
        let data = vec![0x01, 0x02, 0x03];
        let mut handler = Crc32StreamHandler::new(Cursor::new(data.clone()), 0x00000000);
        let mut buf = vec![0u8; 3];
        handler.read_exact(&mut buf).unwrap();

        // Manually compute expected
        let mut seed = !0u32;
        for &b in &data {
            seed = crc32_update(seed, b);
        }
        assert_eq!(handler.seed(), !seed);
    }

    #[test]
    fn test_magic_bytes_constructor() {
        let arr = vec![0x00; 10];
        let handler = Crc32StreamHandler::from_magic_bytes(arr, 0);
        // Just verify it doesn't panic and produces a valid stream
        assert_eq!(handler.inner().get_ref().len(), 10);
    }

    #[test]
    fn test_write_updates_crc() {
        let data = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let mut handler = Crc32StreamHandler::new(Cursor::new(Vec::new()), 0);
        handler.write_all(&data).unwrap();

        let mut seed = !0u32;
        for &b in &data {
            seed = crc32_update(seed, b);
        }
        assert_eq!(handler.seed(), !seed);
    }
}
