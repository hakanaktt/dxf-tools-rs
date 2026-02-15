use std::io::{Read, Seek};
use std::ops::{Deref, DerefMut};

use crate::types::DxfVersion;

use super::dwg_stream_reader_base::DwgStreamReaderBase;

/// AC1012/AC1014 DWG stream reader.
pub struct DwgStreamReaderAc12 {
	inner: DwgStreamReaderBase,
}

impl DwgStreamReaderAc12 {
	pub fn new<R: Read + Seek + 'static>(stream: R) -> Self {
		let mut base = DwgStreamReaderBase::new(Box::new(stream));
		base.version = DxfVersion::AC1012;
		Self { inner: base }
	}

	pub fn from_base(inner: DwgStreamReaderBase) -> Self {
		Self { inner }
	}

	pub fn into_base(self) -> DwgStreamReaderBase {
		self.inner
	}
}

impl Deref for DwgStreamReaderAc12 {
	type Target = DwgStreamReaderBase;

	fn deref(&self) -> &Self::Target {
		&self.inner
	}
}

impl DerefMut for DwgStreamReaderAc12 {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.inner
	}
}
