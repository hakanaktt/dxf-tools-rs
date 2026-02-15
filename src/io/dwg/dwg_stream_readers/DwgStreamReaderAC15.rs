use std::io::{Read, Seek};
use std::ops::{Deref, DerefMut};

use crate::{error::Result, types::Vector3};

use super::dwg_stream_reader_ac12::DwgStreamReaderAc12;
use super::idwg_stream_reader::DwgStreamReader;

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

	pub fn read_bit_extrusion(&mut self) -> Result<Vector3> {
		if self.read_bit()? {
			Ok(Vector3::new(0.0, 0.0, 1.0))
		} else {
			self.read_3_bit_double()
		}
	}

	pub fn read_bit_thickness(&mut self) -> Result<f64> {
		if self.read_bit()? {
			Ok(0.0)
		} else {
			self.read_bit_double()
		}
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
