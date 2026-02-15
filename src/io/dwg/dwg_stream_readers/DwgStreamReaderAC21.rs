use std::io::{Read, Seek};
use std::ops::{Deref, DerefMut};

use super::dwg_stream_reader_ac18::DwgStreamReaderAc18;

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
