use std::io::{Read, Seek};
use std::ops::{Deref, DerefMut};

use super::dwg_stream_reader_ac15::DwgStreamReaderAc15;

/// AC1018 DWG stream reader.
pub struct DwgStreamReaderAc18 {
	inner: DwgStreamReaderAc15,
}

impl DwgStreamReaderAc18 {
	pub fn new<R: Read + Seek + 'static>(stream: R) -> Self {
		Self {
			inner: DwgStreamReaderAc15::new(stream),
		}
	}

	pub fn from_ac15(inner: DwgStreamReaderAc15) -> Self {
		Self { inner }
	}

	pub fn into_ac15(self) -> DwgStreamReaderAc15 {
		self.inner
	}
}

impl Deref for DwgStreamReaderAc18 {
	type Target = DwgStreamReaderAc15;

	fn deref(&self) -> &Self::Target {
		&self.inner
	}
}

impl DerefMut for DwgStreamReaderAc18 {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.inner
	}
}
