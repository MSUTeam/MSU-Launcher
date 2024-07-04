use std::{
	collections::HashMap,
	hash::Hash,
	io::{Read, Write},
};

use anyhow::{anyhow, Result};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use chrono::{DateTime, NaiveDateTime};

pub trait Writable {
	fn write_into<W: Write + WriteBytesExt>(&self, writer: &mut W) -> Result<()>;
}

pub trait Readable {
	fn from_reader<R: Read + ReadBytesExt>(reader: &mut R) -> Result<Self>
	where
		Self: Sized;
}

impl Writable for String {
	fn write_into<W: Write + WriteBytesExt>(&self, writer: &mut W) -> Result<()> {
		writer.write_u16::<LittleEndian>(self.len().try_into()?)?;
		writer.write_all(self.as_bytes())?;
		Ok(())
	}
}

impl Readable for String {
	fn from_reader<R: Read + ReadBytesExt>(reader: &mut R) -> Result<Self> {
		let len = reader.read_u16::<LittleEndian>()?;
		let mut buf = vec![0; len.into()];
		reader.read_exact(&mut buf)?;
		Ok(String::from_utf8(buf).unwrap())
	}
}

impl Writable for NaiveDateTime {
	fn write_into<W: Write + WriteBytesExt>(&self, writer: &mut W) -> Result<()> {
		writer
			.write_i64::<LittleEndian>(self.and_utc().timestamp())
			.unwrap();
		Ok(())
	}
}

impl Readable for NaiveDateTime {
	fn from_reader<R: Read + ReadBytesExt>(reader: &mut R) -> Result<Self> {
		let timestamp = reader.read_i64::<LittleEndian>()?;
		Ok(DateTime::from_timestamp(timestamp, 0).unwrap().naive_utc())
	}
}

impl Readable for bool {
	fn from_reader<R: Read + ReadBytesExt>(reader: &mut R) -> Result<Self> {
		Ok(reader.read_u8()? != 0)
	}
}

impl Writable for bool {
	fn write_into<W: Write + WriteBytesExt>(&self, writer: &mut W) -> Result<()> {
		Ok(writer.write_u8(if *self { 1 } else { 0 })?)
	}
}

impl Writable for u8 {
	fn write_into<W: Write + WriteBytesExt>(&self, writer: &mut W) -> Result<()> {
		Ok(writer.write_u8(*self)?)
	}
}

impl Readable for u8 {
	fn from_reader<R: Read + ReadBytesExt>(reader: &mut R) -> Result<Self> {
		Ok(reader.read_u8()?)
	}
}

impl Writable for u16 {
	fn write_into<W: Write + WriteBytesExt>(&self, writer: &mut W) -> Result<()> {
		Ok(writer.write_u16::<LittleEndian>(*self)?)
	}
}

impl Readable for u16 {
	fn from_reader<R: Read + ReadBytesExt>(reader: &mut R) -> Result<Self> {
		Ok(reader.read_u16::<LittleEndian>()?)
	}
}

// writeU32 is a scam, BB actually writes i32s
impl Writable for u32 {
	fn write_into<W: Write + WriteBytesExt>(&self, writer: &mut W) -> Result<()> {
		i32::try_from(*self)?.write_into(writer)
	}
}

impl Readable for u32 {
	fn from_reader<R: Read + ReadBytesExt>(reader: &mut R) -> Result<Self> {
		Ok(u32::try_from(i32::from_reader(reader)?)?)
	}
}

impl Writable for i8 {
	fn write_into<W: Write + WriteBytesExt>(&self, writer: &mut W) -> Result<()> {
		Ok(writer.write_i8(*self)?)
	}
}

impl Readable for i8 {
	fn from_reader<R: Read + ReadBytesExt>(reader: &mut R) -> Result<Self> {
		Ok(reader.read_i8()?)
	}
}

impl Writable for i16 {
	fn write_into<W: Write + WriteBytesExt>(&self, writer: &mut W) -> Result<()> {
		Ok(writer.write_i16::<LittleEndian>(*self)?)
	}
}

impl Readable for i16 {
	fn from_reader<R: Read + ReadBytesExt>(reader: &mut R) -> Result<Self> {
		Ok(reader.read_i16::<LittleEndian>()?)
	}
}

impl Writable for i32 {
	fn write_into<W: Write + WriteBytesExt>(&self, writer: &mut W) -> Result<()> {
		Ok(writer.write_i32::<LittleEndian>(*self)?)
	}
}

impl Readable for i32 {
	fn from_reader<R: Read + ReadBytesExt>(reader: &mut R) -> Result<Self> {
		Ok(reader.read_i32::<LittleEndian>()?)
	}
}

impl Writable for f32 {
	fn write_into<W: Write + WriteBytesExt>(&self, writer: &mut W) -> Result<()> {
		Ok(writer.write_f32::<LittleEndian>(*self)?)
	}
}

impl Readable for f32 {
	fn from_reader<R: Read + ReadBytesExt>(reader: &mut R) -> Result<Self> {
		Ok(reader.read_f32::<LittleEndian>()?)
	}
}

impl<W> Writable for (W, W)
where
	W: Writable,
{
	fn write_into<R: Write + WriteBytesExt>(&self, writer: &mut R) -> Result<()> {
		self.0.write_into(writer)?;
		self.1.write_into(writer)?;
		Ok(())
	}
}

impl<R> Readable for (R, R)
where
	R: Readable,
{
	fn from_reader<W: Read + ReadBytesExt>(reader: &mut W) -> Result<Self> {
		Ok((R::from_reader(reader)?, R::from_reader(reader)?))
	}
}

impl<W> Writable for Vec<W>
where
	W: Writable,
{
	fn write_into<R: Write + WriteBytesExt>(&self, writer: &mut R) -> Result<()> {
		Into::<SerializedSQValue>::into(SQValue::Int(self.len().try_into()?)).write_into(writer)?;
		for item in self {
			item.write_into(writer)?;
		}
		Ok(())
	}
}

impl<R> Readable for Vec<R>
where
	R: Readable,
{
	fn from_reader<W: Read + ReadBytesExt>(reader: &mut W) -> Result<Self> {
		let len = SerializedSQValue::from_reader(reader)?;
		let len = len.try_into()?;
		if let SQValue::Int(len) = len {
			let mut vec = Vec::new();
			for _ in 0..len {
				vec.push(R::from_reader(reader)?);
			}
			Ok(vec)
		} else {
			Err(anyhow!(
				"Invalid SerializedSQValue for collection length {:?}",
				len
			))
		}
	}
}

impl<W1, W2> Writable for HashMap<W1, W2>
where
	W1: Writable,
	W2: Writable,
{
	fn write_into<R: Write + WriteBytesExt>(&self, writer: &mut R) -> Result<()> {
		Into::<SerializedSQValue>::into(SQValue::Int(self.len().try_into()?)).write_into(writer)?;
		for (key, value) in self {
			key.write_into(writer)?;
			value.write_into(writer)?;
		}
		Ok(())
	}
}

impl<R1, R2> Readable for HashMap<R1, R2>
where
	R1: Readable + Eq + Hash,
	R2: Readable,
{
	fn from_reader<R: Read + ReadBytesExt>(reader: &mut R) -> Result<Self> {
		let len = SerializedSQValue::from_reader(reader)?;
		let len = len.try_into()?;
		if let SQValue::Int(len) = len {
			let mut map = HashMap::new();
			for _ in 0..len {
				let key = R1::from_reader(reader)?;
				let value = R2::from_reader(reader)?;
				map.insert(key, value);
			}
			Ok(map)
		} else {
			Err(anyhow!(
				"Invalid SerializedSQValue for collection length {:?}",
				len
			))
		}
	}
}

#[cfg(test)]
use std::fmt::Debug;

use super::{serialized_sq_value::SerializedSQValue, sq_value::SQValue};
#[cfg(test)]
pub fn test_readable_writable_impls<RW>(value: &RW)
where
	RW: Readable + Writable + Debug + PartialEq,
{
	let mut buf = Vec::new();
	value.write_into(&mut buf).unwrap();
	let mut cursor = std::io::Cursor::new(buf);
	let read = RW::from_reader(&mut cursor).unwrap();
	assert_eq!(read, *value);
}

#[cfg(test)]
mod tests {
	use chrono::{Local, Timelike};

	use super::*;

	#[test]
	fn read_write_string() {
		let s = "Hello, World!";
		test_readable_writable_impls(&s.to_owned());
	}

	#[test]
	fn read_write_datetime() {
		let time = Local::now().naive_local().with_nanosecond(0).unwrap();
		test_readable_writable_impls(&time);
	}
}
