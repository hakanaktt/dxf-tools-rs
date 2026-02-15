use std::io::{Read, Seek};
use std::ops::{Deref, DerefMut};

use super::dwg_stream_reader_ac21::DwgStreamReaderAc21;

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
