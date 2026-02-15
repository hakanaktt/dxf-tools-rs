//! CRC-8 stream wrapper for DWG integrity checking.
//!
//! Ported from ACadSharp `CRC8StreamHandler.cs`.
//!
//! The DWG file format uses a modification of a standard cyclic redundancy check
//! as an error-detecting mechanism. This wrapper transparently updates the CRC
//! seed as data is read from or written to the inner stream.

use std::io::{self, Read, Seek, SeekFrom, Write};

use super::crc::{crc8_decode, crc8_value};

/// A stream wrapper that computes a running CRC-8 over all bytes read/written.
///
/// This method is used extensively in pre-R13 files, but seems only to be used
/// in the header for R13 and beyond.
pub struct Crc8StreamHandler<S> {
    stream: S,
    seed: u16,
}

impl<S> Crc8StreamHandler<S> {
    /// Create a new CRC-8 stream handler wrapping the given stream.
    pub fn new(stream: S, seed: u16) -> Self {
        Self { stream, seed }
    }

    /// Get the current CRC seed value.
    pub fn seed(&self) -> u16 {
        self.seed
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

impl<S: Read> Read for Crc8StreamHandler<S> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.stream.read(buf)?;
        for &byte in &buf[..n] {
            self.seed = crc8_decode(self.seed, byte);
        }
        Ok(n)
    }
}

impl<S: Write> Write for Crc8StreamHandler<S> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        for &byte in buf {
            self.seed = crc8_decode(self.seed, byte);
        }
        self.stream.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stream.flush()
    }
}

impl<S: Seek> Seek for Crc8StreamHandler<S> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.stream.seek(pos)
    }
}

/// Compute CRC-8 over a sub-range of a buffer (convenience free function).
///
/// Equivalent to `CRC8StreamHandler.GetCRCValue` in ACadSharp.
pub fn get_crc8_value(seed: u16, buffer: &[u8], start: usize, count: usize) -> u16 {
    crc8_value(seed, buffer, start, count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_read_updates_seed() {
        let data = vec![0x01, 0x02, 0x03];
        let mut handler = Crc8StreamHandler::new(Cursor::new(data.clone()), 0x0000);
        let mut buf = vec![0u8; 3];
        handler.read_exact(&mut buf).unwrap();

        // Manually compute expected seed
        let expected = crc8_value(0x0000, &data, 0, 3);
        assert_eq!(handler.seed(), expected);
    }

    #[test]
    fn test_write_updates_seed() {
        let mut backing = Vec::new();
        let data = vec![0xAA, 0xBB, 0xCC];

        let mut handler = Crc8StreamHandler::new(Cursor::new(&mut backing), 0x1234);
        handler.write_all(&data).unwrap();

        let expected = crc8_value(0x1234, &data, 0, 3);
        assert_eq!(handler.seed(), expected);
    }

    #[test]
    fn test_seek_does_not_affect_seed() {
        let data = vec![0x00; 10];
        let mut handler = Crc8StreamHandler::new(Cursor::new(data), 0xABCD);
        handler.seek(SeekFrom::Start(5)).unwrap();
        assert_eq!(handler.seed(), 0xABCD);
    }
}
