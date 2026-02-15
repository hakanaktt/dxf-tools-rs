use std::io::{Read, Seek};
use std::ops::{Deref, DerefMut};

use crate::error::Result;

use super::{
	dwg_stream_reader_ac21::DwgStreamReaderAc21,
	idwg_stream_reader::{DwgObjectType, DwgStreamReader},
};

/// AC1024+ DWG stream reader.
pub struct DwgStreamReaderAc24 {
	inner: DwgStreamReaderAc21,
}

impl DwgStreamReaderAc24 {
	pub fn new<R: Read + Seek + 'static>(stream: R) -> Self {
		Self {
			inner: DwgStreamReaderAc21::new(stream),
		}
	}

	pub fn from_ac21(inner: DwgStreamReaderAc21) -> Self {
		Self { inner }
	}

	pub fn into_ac21(self) -> DwgStreamReaderAc21 {
		self.inner
	}

	pub fn read_object_type_ac24(&mut self) -> Result<DwgObjectType> {
		let pair = self.read_2_bits()?;
		let value = match pair {
			0 => self.read_byte()? as u16,
			1 => 0x01F0 + self.read_byte()? as u16,
			2 | 3 => self.read_short()? as u16,
			_ => unreachable!(),
		};
		Ok(DwgObjectType(value))
	}
}

impl Deref for DwgStreamReaderAc24 {
	type Target = DwgStreamReaderAc21;

	fn deref(&self) -> &Self::Target {
		&self.inner
	}
}

impl DerefMut for DwgStreamReaderAc24 {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.inner
	}
}
