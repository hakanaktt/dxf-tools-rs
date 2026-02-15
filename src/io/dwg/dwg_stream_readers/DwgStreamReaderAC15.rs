use std::io::{Read, Seek};
use std::ops::{Deref, DerefMut};

use super::dwg_stream_reader_ac12::DwgStreamReaderAc12;

/// AC1015 DWG stream reader.
pub struct DwgStreamReaderAc15 {
	inner: DwgStreamReaderAc12,
}

impl DwgStreamReaderAc15 {
	pub fn new<R: Read + Seek + 'static>(stream: R) -> Self {
		Self {
			inner: DwgStreamReaderAc12::new(stream),
		}
	}

	pub fn from_ac12(inner: DwgStreamReaderAc12) -> Self {
		Self { inner }
	}

	pub fn into_ac12(self) -> DwgStreamReaderAc12 {
		self.inner
	}
}

impl Deref for DwgStreamReaderAc15 {
	type Target = DwgStreamReaderAc12;

	fn deref(&self) -> &Self::Target {
		&self.inner
	}
}

impl DerefMut for DwgStreamReaderAc15 {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.inner
	}
}
