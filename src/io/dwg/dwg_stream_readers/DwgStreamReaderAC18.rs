use std::io::{Read, Seek};
use std::ops::{Deref, DerefMut};

use crate::{
	error::Result,
	types::{Color, Transparency},
};

use super::{
	dwg_stream_reader_ac15::DwgStreamReaderAc15,
	idwg_stream_reader::DwgStreamReader,
};

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

	pub fn read_cm_color_ac18(&mut self, use_text_stream: bool) -> Result<Color> {
		let _color_index = self.read_bit_short()?;
		let rgb = self.read_bit_long()? as u32;
		let arr = rgb.to_le_bytes();

		let color = if rgb == 0xC000_0000 {
			Color::ByLayer
		} else if (rgb & 0x0100_0000) != 0 {
			Color::from_index(arr[0] as i16)
		} else {
			Color::from_rgb(arr[2], arr[1], arr[0])
		};

		let id = self.read_byte()?;
		if (id & 1) == 1 {
			let _ = self.read_variable_text()?;
		}
		if (id & 2) == 2 {
			let _ = self.read_variable_text()?;
		}

		if !use_text_stream {
			return Ok(color);
		}

		Ok(color)
	}

	pub fn read_en_color_ac18(&mut self) -> Result<(Color, Transparency, bool)> {
		let size = self.read_bit_short()?;
		if size == 0 {
			return Ok((Color::ByBlock, Transparency::OPAQUE, false));
		}

		let flags = (size as u16) & 0xFF00;
		let color = if (flags & 0x4000) != 0 {
			Color::ByBlock
		} else if (flags & 0x8000) != 0 {
			let rgb = self.read_bit_long()? as u32;
			let arr = rgb.to_le_bytes();
			Color::from_rgb(arr[2], arr[1], arr[0])
		} else {
			Color::from_index((size & 0x0FFF) as i16)
		};

		let transparency = if (flags & 0x2000) != 0 {
			let value = self.read_bit_long()? as u32;
			Transparency::from_alpha_value(value)
		} else {
			Transparency::BY_LAYER
		};

		let is_book_color = (flags & 0x4000) != 0;
		Ok((color, transparency, is_book_color))
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
