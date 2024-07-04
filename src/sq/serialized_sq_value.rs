use std::io::{Read, Write};

use anyhow::{anyhow, Result};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use ordered_float::OrderedFloat;

use super::{
	shared::{Readable, Writable},
	sq_value::SQValue,
};

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub struct MetaDataEmulator {
	version: u8, // should be u32
	name: String,
	file_name: String,
	creation_date: String,
	modification_date: String,
	meta_data: Box<SerializedSQValue>, // array containing a table as its only element
}

impl Readable for MetaDataEmulator {
	fn from_reader<R: Read + ReadBytesExt>(reader: &mut R) -> Result<Self> {
		Ok(Self {
			version: reader.read_u8()?,
			name: String::from_reader(reader)?,
			file_name: String::from_reader(reader)?,
			creation_date: String::from_reader(reader)?,
			modification_date: String::from_reader(reader)?,
			meta_data: Box::new(SerializedSQValue::from_reader(reader)?),
		})
	}
}

impl Writable for MetaDataEmulator {
	fn write_into<W: Write + WriteBytesExt>(&self, writer: &mut W) -> Result<()> {
		writer.write_u8(self.version)?;
		self.name.write_into(writer)?;
		self.file_name.write_into(writer)?;
		self.creation_date.write_into(writer)?;
		self.modification_date.write_into(writer)?;
		self.meta_data.write_into(writer)?;
		Ok(())
	}
}

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub enum SerializedSQValue {
	None,
	Unknown,
	Null,
	Bool(bool),
	String(String),
	U8(u8),
	U16(u16),
	U32(u32),
	I8(i8),
	I16(i16),
	I32(i32),
	Float(OrderedFloat<f32>),
	Table(Vec<(SerializedSQValue, SerializedSQValue)>),
	Array(Vec<SerializedSQValue>),
	Serialized(Vec<SerializedSQValue>, MetaDataEmulator),
}

impl Readable for SerializedSQValue {
	fn from_reader<R: Read + ReadBytesExt>(reader: &mut R) -> Result<Self>
	where
		Self: Sized,
	{
		let sq_type = reader.read_u8()?;
		match sq_type {
			0 => Ok(Self::None),
			1 => Ok(Self::Unknown),
			2 => Ok(Self::Null),
			3 => Ok(Self::Bool(reader.read_u8()? != 0)),
			4 => Ok(Self::String(String::from_reader(reader)?)),
			5 => Ok(Self::U8(reader.read_u8()?)),
			6 => Ok(Self::U16(reader.read_u16::<LittleEndian>()?)),
			7 => Ok(Self::U32(reader.read_u32::<LittleEndian>()?)),
			8 => Ok(Self::I8(reader.read_i8()?)),
			9 => Ok(Self::I16(reader.read_i16::<LittleEndian>()?)),
			10 => Ok(Self::I32(reader.read_i32::<LittleEndian>()?)),
			11 => Ok(Self::Float(OrderedFloat(
				reader.read_f32::<LittleEndian>()?,
			))),
			12..=14 => {
				let len = SerializedSQValue::from_reader(reader)?;
				let len = len.try_into()?;
				if let SQValue::Int(len) = len {
					if sq_type == 12 {
						let mut table = Vec::new();
						let len = len / 2; // in sq they are serialized individually
						for _ in 0..len {
							table.push((Self::from_reader(reader)?, Self::from_reader(reader)?));
						}
						Ok(Self::Table(table))
					} else {
						let mut array = Vec::new();
						for _ in 0..len {
							array.push(SerializedSQValue::from_reader(reader)?);
						}
						if sq_type == 13 {
							Ok(Self::Array(array))
						} else {
							let meta_data = MetaDataEmulator::from_reader(reader)?;
							Ok(Self::Serialized(array, meta_data))
						}
					}
				} else {
					Err(anyhow!(
						"Invalid SerializedSQValue for collection length {:?}",
						len
					))
				}
			}
			_ => Err(anyhow!("Invalid SerializedSQValue")),
		}
	}
}

impl Writable for SerializedSQValue {
	fn write_into<W: Write + WriteBytesExt>(&self, writer: &mut W) -> Result<()> {
		match self {
			Self::None => writer.write_u8(0)?,
			Self::Unknown => writer.write_u8(1)?,
			Self::Null => writer.write_u8(2)?,
			Self::Bool(b) => {
				writer.write_u8(3)?;
				writer.write_u8(if *b { 1 } else { 0 })?;
			}
			Self::String(s) => {
				writer.write_u8(4)?;
				s.write_into(writer)?;
			}
			Self::U8(u) => {
				writer.write_u8(5)?;
				writer.write_u8(*u)?;
			}
			Self::U16(u) => {
				writer.write_u8(6)?;
				writer.write_u16::<LittleEndian>(*u)?;
			}
			Self::U32(u) => {
				writer.write_u8(7)?;
				writer.write_u32::<LittleEndian>(*u)?;
			}
			Self::I8(i) => {
				writer.write_u8(8)?;
				writer.write_i8(*i)?;
			}
			Self::I16(i) => {
				writer.write_u8(9)?;
				writer.write_i16::<LittleEndian>(*i)?;
			}
			Self::I32(i) => {
				writer.write_u8(10)?;
				writer.write_i32::<LittleEndian>(*i)?;
			}
			Self::Float(f) => {
				writer.write_u8(11)?;
				writer.write_f32::<LittleEndian>(f.into_inner())?;
			}
			Self::Table(t) => {
				writer.write_u8(12)?;
				Into::<SerializedSQValue>::into(SQValue::Int((t.len() * 2).try_into()?))
					.write_into(writer)?;
				for (key, value) in t {
					key.write_into(writer)?;
					value.write_into(writer)?;
				}
			}
			Self::Array(a) => {
				writer.write_u8(13)?;
				Into::<SerializedSQValue>::into(SQValue::Int(a.len().try_into()?))
					.write_into(writer)?;
				for value in a {
					value.write_into(writer)?;
				}
			}
			Self::Serialized(a, meta_emu) => {
				writer.write_u8(14)?;
				Into::<SerializedSQValue>::into(SQValue::Int(a.len().try_into()?))
					.write_into(writer)?;
				for value in a {
					value.write_into(writer)?;
				}
				meta_emu.write_into(writer)?;
			}
		};
		Ok(())
	}
}

const I16_MIN: i32 = i16::MIN as i32;
const I8_MIN: i32 = i8::MIN as i32;
const U8_MIN: i32 = u8::MIN as i32;
const U8_MAX: i32 = u8::MAX as i32;
const U16_MAX: i32 = u16::MAX as i32;

impl From<SQValue> for SerializedSQValue {
	#[allow(overlapping_range_endpoints)]
	#[allow(clippy::match_overlapping_arm)]
	fn from(value: SQValue) -> Self {
		match value {
			SQValue::Null => SerializedSQValue::Null,
			SQValue::Bool(bool) => SerializedSQValue::Bool(bool),
			SQValue::String(string) => SerializedSQValue::String(string),
			SQValue::Int(int) => match int {
				i32::MIN..=I16_MIN => SerializedSQValue::I32(int),
				I16_MIN..=I8_MIN => SerializedSQValue::I16(int as i16),
				I8_MIN..=U8_MIN => SerializedSQValue::I8(int as i8),
				U8_MIN..=U8_MAX => SerializedSQValue::U8(int as u8),
				U8_MAX..=U16_MAX => SerializedSQValue::U16(int as u16),
				U16_MAX..=i32::MAX => SerializedSQValue::U32(int as u32),
			},
			SQValue::Float(float) => SerializedSQValue::Float(float),
			SQValue::Table(sq_table) => SerializedSQValue::Table(
				sq_table
					.0
					.into_iter()
					.map(|(key, value)| (key.into(), value.into()))
					.collect(),
			),
			SQValue::Array(array) => {
				SerializedSQValue::Array(array.into_iter().map(Into::into).collect())
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use ordered_float::OrderedFloat;

	use crate::sq::shared::test_readable_writable_impls;

	use super::*;

	#[test]
	fn read_write_meta_data() {
		let meta_data = MetaDataEmulator {
			version: 1,
			name: "name".to_owned(),
			file_name: "file_name".to_owned(),
			creation_date: chrono::Local::now().to_rfc3339(),
			modification_date: chrono::Local::now().to_rfc3339(),
			meta_data: Box::new(SerializedSQValue::Array(vec![SerializedSQValue::Table(
				vec![
					(
						SerializedSQValue::String("key1".to_owned()),
						SerializedSQValue::String("1".to_owned()),
					),
					(
						SerializedSQValue::String("key2".to_owned()),
						SerializedSQValue::String("value2".to_owned()),
					),
				],
			)])),
		};
		test_readable_writable_impls(&meta_data);
	}

	#[test]
	fn read_write_serialized_sq_value() {
		let serialized_sq_value = SerializedSQValue::Array(vec![
			SerializedSQValue::String("key1".to_owned()),
			SerializedSQValue::String("value1".to_owned()),
			SerializedSQValue::Table(vec![
				(
					SerializedSQValue::String("key2".to_owned()),
					SerializedSQValue::String("value2".to_owned()),
				),
				(
					SerializedSQValue::String("key3".to_owned()),
					SerializedSQValue::Array(vec![SerializedSQValue::String("key4".to_owned())]),
				),
			]),
			SerializedSQValue::Null,
			SerializedSQValue::Bool(true),
			SerializedSQValue::I32(1),
			SerializedSQValue::Float(OrderedFloat(1.124)),
		]);
		test_readable_writable_impls(&serialized_sq_value);
	}
}
