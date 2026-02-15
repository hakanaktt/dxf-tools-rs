use std::io::{Read, Seek};
use std::ops::{Deref, DerefMut};

use crate::types::DxfVersion;

use super::dwg_stream_reader_ac21::DwgStreamReaderAc21;
use super::dwg_stream_reader_ac18::DwgStreamReaderAc18;
use super::dwg_stream_reader_ac15::DwgStreamReaderAc15;
use super::dwg_stream_reader_ac12::DwgStreamReaderAc12;
use super::dwg_stream_reader_base::DwgStreamReaderBase;

/// AC1024+ DWG stream reader.
/// Version-specific behavior (ReadObjectType) is handled
/// in DwgStreamReaderBase via the `version` field.
pub struct DwgStreamReaderAc24 {
	inner: DwgStreamReaderAc21,
}

impl DwgStreamReaderAc24 {
	pub fn new<R: Read + Seek + 'static>(stream: R) -> Self {
		let mut base = DwgStreamReaderBase::new(Box::new(stream));
		base.version = DxfVersion::AC1024;
		Self {
			inner: DwgStreamReaderAc21::from_ac18(
				DwgStreamReaderAc18::from_ac15(
					DwgStreamReaderAc15::from_ac12(
						DwgStreamReaderAc12::from_base(base),
					),
				),
			),
		}
	}

	pub fn from_ac21(mut inner: DwgStreamReaderAc21) -> Self {
		inner.version = DxfVersion::AC1024;
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
