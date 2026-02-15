use std::io::{Read, Seek};
use std::ops::{Deref, DerefMut};

use crate::types::DxfVersion;

use super::dwg_stream_reader_ac18::DwgStreamReaderAc18;
use super::dwg_stream_reader_ac15::DwgStreamReaderAc15;
use super::dwg_stream_reader_ac12::DwgStreamReaderAc12;
use super::dwg_stream_reader_base::DwgStreamReaderBase;

/// AC1021 DWG stream reader.
/// Version-specific behavior (ReadTextUnicode, ReadVariableText)
/// is handled in DwgStreamReaderBase via the `version` field.
pub struct DwgStreamReaderAc21 {
	inner: DwgStreamReaderAc18,
}

impl DwgStreamReaderAc21 {
	pub fn new<R: Read + Seek + 'static>(stream: R) -> Self {
		let mut base = DwgStreamReaderBase::new(Box::new(stream));
		base.version = DxfVersion::AC1021;
		Self {
			inner: DwgStreamReaderAc18::from_ac15(
				DwgStreamReaderAc15::from_ac12(
					DwgStreamReaderAc12::from_base(base),
				),
			),
		}
	}

	pub fn from_ac18(mut inner: DwgStreamReaderAc18) -> Self {
		inner.version = DxfVersion::AC1021;
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
