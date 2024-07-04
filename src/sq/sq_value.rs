use std::collections::HashMap;
use std::hash::Hash;

use anyhow::anyhow;
use ordered_float::OrderedFloat;

use super::serialized_sq_value::SerializedSQValue;

// need a separate struct for tables because of the recursion of SQValue
// which make it impossible to derive hash for SQValue
#[derive(Debug, Clone, Default)]
pub struct SQTable(pub HashMap<SQValue, SQValue>);

impl PartialEq for SQTable {
	fn eq(&self, other: &Self) -> bool {
		if self.0.len() != other.0.len() {
			return false;
		}
		for (key, value) in &self.0 {
			if let Some(other_value) = other.0.get(key) {
				if value != other_value {
					return false;
				}
			} else {
				return false;
			}
		}
		true
	}
}
impl Eq for SQTable {}
impl Hash for SQTable {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		for (key, value) in &self.0 {
			key.hash(state);
			value.hash(state);
		}
	}
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum SQValue {
	Null,
	Bool(bool),
	String(String),
	Int(i32),
	Float(OrderedFloat<f32>),
	Table(SQTable),
	Array(Vec<SQValue>),
}

impl TryFrom<SerializedSQValue> for SQValue {
	type Error = anyhow::Error;

	fn try_from(value: SerializedSQValue) -> std::result::Result<Self, Self::Error> {
		Ok(match value {
			SerializedSQValue::None => return Err(anyhow!("Tried to convert None Value")),
			SerializedSQValue::Unknown => return Err(anyhow!("Tried to convert Unknown Value")),
			SerializedSQValue::Null => Self::Null,
			SerializedSQValue::Bool(b) => Self::Bool(b),
			SerializedSQValue::String(s) => Self::String(s),
			SerializedSQValue::U8(u) => Self::Int(u.into()),
			SerializedSQValue::U16(u) => Self::Int(u.into()),
			SerializedSQValue::U32(u) => Self::Int(u.try_into()?),
			SerializedSQValue::I8(i) => Self::Int(i.into()),
			SerializedSQValue::I16(i) => Self::Int(i.into()),
			SerializedSQValue::I32(i) => Self::Int(i),
			SerializedSQValue::Float(f) => Self::Float(f),
			SerializedSQValue::Table(t) => {
				let mut table = SQTable::default();
				for (key, value) in t {
					table.0.insert(key.try_into()?, value.try_into()?);
				}
				Self::Table(table)
			}
			SerializedSQValue::Array(a) => {
				let mut array = Vec::new();
				for value in a {
					array.push(value.try_into()?);
				}
				Self::Array(array)
			}
			SerializedSQValue::Serialized(..) => {
				return Err(anyhow!("Tried to convert Serialized Value"))
			}
		})
	}
}

#[cfg(test)]
mod tests {
	use crate::sq::shared::test_readable_writable_impls;

	use super::*;

	#[test]
	fn read_write_sq_value() {
		let value = SQValue::Array(vec![
			SQValue::String("key1".to_owned()),
			SQValue::String("value1".to_owned()),
			SQValue::Table(SQTable(
				vec![(
					SQValue::String("key2".to_owned()),
					SQValue::String("value2".to_owned()),
				)]
				.into_iter()
				.collect(),
			)),
			SQValue::Null,
			SQValue::Bool(true),
			SQValue::Int(1),
			SQValue::Float(OrderedFloat(1.124)),
		]);
		let serialized_value: SerializedSQValue = value.clone().into();
		test_readable_writable_impls(&serialized_value);

		let deserialized_value: SQValue = serialized_value.try_into().unwrap();
		assert_eq!(deserialized_value, value);
	}
}
