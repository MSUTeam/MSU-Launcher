use std::io::{Read, Write};

use anyhow::Result;
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

#[cfg(test)]
use std::fmt::Debug;
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
