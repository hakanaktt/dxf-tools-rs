use std::io::{Read, Seek};
use std::ops::{Deref, DerefMut};

use crate::error::Result;

use super::{
	dwg_stream_reader_ac18::DwgStreamReaderAc18,
	idwg_stream_reader::DwgStreamReader,
};

/// AC1021 DWG stream reader.
pub struct DwgStreamReaderAc21 {
	inner: DwgStreamReaderAc18,
}

impl DwgStreamReaderAc21 {
	pub fn new<R: Read + Seek + 'static>(stream: R) -> Self {
		Self {
			inner: DwgStreamReaderAc18::new(stream),
		}
	}

	pub fn from_ac18(inner: DwgStreamReaderAc18) -> Self {
		Self { inner }
	}

	pub fn into_ac18(self) -> DwgStreamReaderAc18 {
		self.inner
	}

	pub fn read_text_unicode_ac21(&mut self) -> Result<String> {
		let text_length = self.read_short()?;
		if text_length <= 0 {
			return Ok(String::new());
		}

		let byte_len = (text_length as usize) * 2;
		let bytes = self.read_bytes(byte_len)?;
		let utf16: Vec<u16> = bytes
			.chunks_exact(2)
			.map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
			.collect();
		Ok(String::from_utf16_lossy(&utf16))
	}

	pub fn read_variable_text_ac21(&mut self) -> Result<String> {
		let text_length = self.read_bit_short()?;
		if text_length <= 0 {
			return Ok(String::new());
		}

		let byte_len = (text_length as usize) * 2;
		let bytes = self.read_bytes(byte_len)?;
		let utf16: Vec<u16> = bytes
			.chunks_exact(2)
			.map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
			.collect();
		Ok(String::from_utf16_lossy(&utf16))
	}
}

impl Deref for DwgStreamReaderAc21 {
	type Target = DwgStreamReaderAc18;

	fn deref(&self) -> &Self::Target {
		&self.inner
	}
}

impl DerefMut for DwgStreamReaderAc21 {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.inner
	}
}
