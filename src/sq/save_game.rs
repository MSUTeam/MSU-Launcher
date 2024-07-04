use std::{
	collections::HashMap,
	io::{Cursor, Read},
};

use anyhow::{anyhow, Result};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use bytes::Buf;
use chrono::{NaiveDateTime, Timelike};

use crate::sq::serialized_sq_value::SerializedSQValue;

use super::{
	shared::{Readable, Writable},
	sq_value::SQValue,
};

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub struct SaveGame {
	magic_num: u16,
	layout_version: u8,
	serialization_version: i32,
	creation_date: NaiveDateTime,
	modification_date: NaiveDateTime,
	file_name: String,
	meta_data: HashMap<String, String>,
	magic_num_2: u16,
	pub raw_data: Vec<u8>,
}

impl Default for SaveGame {
	fn default() -> Self {
		Self {
			magic_num: 0xbb,
			layout_version: 2,
			serialization_version: 0,
			creation_date: chrono::Local::now()
				.naive_local()
				.with_nanosecond(0)
				.unwrap(),
			modification_date: chrono::Local::now()
				.naive_local()
				.with_nanosecond(0)
				.unwrap(),
			file_name: String::new(),
			meta_data: HashMap::new(),
			magic_num_2: 0xbb,
			raw_data: Vec::new(),
		}
	}
}

impl Readable for SaveGame {
	fn from_reader<R: Read + ReadBytesExt>(reader: &mut R) -> anyhow::Result<Self>
	where
		Self: Sized,
	{
		let magic_num = reader.read_u16::<LittleEndian>()?;
		let layout_version = reader.read_u8()?;
		let serialization_version = reader.read_i32::<LittleEndian>()?;

		let creation_date = NaiveDateTime::from_reader(reader)?;
		let modification_date = NaiveDateTime::from_reader(reader)?;
		let file_name = String::from_reader(reader)?;

		let mut meta_data = HashMap::new();
		for _ in 0..reader.read_u16::<LittleEndian>()? {
			let key = String::from_reader(reader)?;
			let value = String::from_reader(reader)?;
			meta_data.insert(key, value);
		}

		let magic_num_2 = reader.read_u16::<LittleEndian>()?;

		let mut raw_data = Vec::new();
		reader.read_to_end(&mut raw_data)?;

		Ok(Self {
			magic_num,
			layout_version,
			serialization_version,
			creation_date,
			modification_date,
			file_name,
			meta_data,
			magic_num_2,
			raw_data,
		})
	}
}

impl Writable for SaveGame {
	fn write_into<W: std::io::Write + WriteBytesExt>(&self, writer: &mut W) -> anyhow::Result<()> {
		writer.write_u16::<LittleEndian>(self.magic_num)?;
		writer.write_u8(self.layout_version)?;
		writer.write_i32::<LittleEndian>(self.serialization_version)?;

		self.creation_date.write_into(writer)?;
		self.modification_date.write_into(writer)?;

		self.file_name.write_into(writer)?;

		writer.write_u16::<LittleEndian>(self.meta_data.len().try_into()?)?;
		for (key, value) in &self.meta_data {
			key.write_into(writer)?;
			value.write_into(writer)?;
		}
		writer.write_u16::<LittleEndian>(self.magic_num_2)?;

		writer.write_all(&self.raw_data)?;

		Ok(())
	}
}

impl SaveGame {
	pub fn with_name<S: Into<String>>(mut self, file_name: S) -> Self {
		self.file_name = file_name.into();
		self
	}

	pub fn with_raw_data(mut self, raw_data: Vec<u8>) -> Self {
		self.raw_data = raw_data;
		self
	}

	pub fn parse_content(&self) -> Result<SQValue> {
		let mut reader = Cursor::new(&self.raw_data);
		let sq_value = SerializedSQValue::from_reader(&mut reader)?;
		println!("{:?}", sq_value);
		if reader.has_remaining() {
			Err(anyhow!("Failed to parse all content"))
		} else {
			Ok(sq_value.try_into()?)
		}
	}

	pub fn from_value(value: SQValue) -> Self {
		let mut raw_data = Vec::new();
		let mut writer = Cursor::new(&mut raw_data);
		let serialized = SerializedSQValue::from(value);
		serialized.write_into(&mut writer).unwrap();
		Self::default().with_raw_data(raw_data)
	}
}

#[cfg(test)]
mod tests {
	use ordered_float::OrderedFloat;

	use super::*;
	use crate::sq::shared::test_readable_writable_impls;
	use crate::sq::SQTable;

	#[test]
	fn read_write_save_game() {
		let mut save_game = SaveGame::from_value(SQValue::Array(vec![
			SQValue::String("key1".to_owned()),
			SQValue::String("value1".to_owned()),
			SQValue::Table(SQTable(HashMap::from_iter(
				vec![(
					SQValue::String("key2".to_owned()),
					SQValue::String("value2".to_owned()),
				)]
				.into_iter(),
			))),
			SQValue::Null,
			SQValue::Bool(true),
			SQValue::Int(1),
			SQValue::Float(OrderedFloat(1.124)),
		]));
		save_game
			.meta_data
			.insert("key".to_owned(), "value".to_owned());
		save_game
			.meta_data
			.insert("key2".to_owned(), "value2".to_owned());
		save_game.file_name = "test".to_owned();
		test_readable_writable_impls(&save_game);
	}
}
