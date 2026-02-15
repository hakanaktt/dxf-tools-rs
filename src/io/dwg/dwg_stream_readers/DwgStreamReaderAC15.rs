use std::io::{Read, Seek};
use std::ops::{Deref, DerefMut};

use crate::types::DxfVersion;

use super::dwg_stream_reader_ac12::DwgStreamReaderAc12;
use super::dwg_stream_reader_base::DwgStreamReaderBase;

/// AC1015 DWG stream reader.
/// Version-specific behavior (read_bit_extrusion, read_bit_thickness)
/// is handled in DwgStreamReaderBase via the `version` field.
pub struct DwgStreamReaderAc15 {
	inner: DwgStreamReaderAc12,
}

impl DwgStreamReaderAc15 {
	pub fn new<R: Read + Seek + 'static>(stream: R) -> Self {
		let mut base = DwgStreamReaderBase::new(Box::new(stream));
		base.version = DxfVersion::AC1015;
		Self {
			inner: DwgStreamReaderAc12::from_base(base),
		}
	}

	pub fn from_ac12(mut inner: DwgStreamReaderAc12) -> Self {
		inner.version = DxfVersion::AC1015;
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
