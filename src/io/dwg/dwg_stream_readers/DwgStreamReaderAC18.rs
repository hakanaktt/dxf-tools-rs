use std::io::{Read, Seek};
use std::ops::{Deref, DerefMut};

use crate::types::DxfVersion;

use super::dwg_stream_reader_ac15::DwgStreamReaderAc15;
use super::dwg_stream_reader_ac12::DwgStreamReaderAc12;
use super::dwg_stream_reader_base::DwgStreamReaderBase;

/// AC1018 DWG stream reader.
/// Version-specific behavior (ReadCmColor, ReadEnColor) is handled
/// in DwgStreamReaderBase via the `version` field.
pub struct DwgStreamReaderAc18 {
	inner: DwgStreamReaderAc15,
}

impl DwgStreamReaderAc18 {
	pub fn new<R: Read + Seek + 'static>(stream: R) -> Self {
		let mut base = DwgStreamReaderBase::new(Box::new(stream));
		base.version = DxfVersion::AC1018;
		Self {
			inner: DwgStreamReaderAc15::from_ac12(
				DwgStreamReaderAc12::from_base(base),
			),
		}
	}

	pub fn from_ac15(mut inner: DwgStreamReaderAc15) -> Self {
		inner.version = DxfVersion::AC1018;
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
