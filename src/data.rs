//! Data functions and operators

// TODO: it's possible that "missing", "var", et al. could be implemented
// as operators. They were originally done as parsers because there wasn't
// yet a LazyOperator concept.

use serde_json::{Map, Number, Value};
use std::convert::{From, TryFrom};

use crate::error::Error;
use crate::value::Evaluated;
use crate::Parser;

#[derive(Debug)]
pub enum KeyType<'a> {
    String(&'a String),
    Number(&'a Number),
}
impl From<KeyType<'_>> for Value {
    fn from(key: KeyType) -> Self {
        Value::from(&key)
    }
}
impl From<&KeyType<'_>> for Value {
    fn from(key: &KeyType) -> Self {
        match *key {
            KeyType::String(key) => Self::String(key.clone()),
            KeyType::Number(idx) => Self::Number(idx.clone()),
        }
    }
}

#[derive(Debug)]
pub struct Raw<'a> {
    value: &'a Value,
}
impl<'a> Parser<'a> for Raw<'a> {
    fn from_value(value: &'a Value) -> Result<Option<Self>, Error> {
        Ok(Some(Self { value }))
    }
    fn evaluate(&self, _data: &Value) -> Result<Evaluated, Error> {
        Ok(Evaluated::Raw(self.value))
    }
}
impl From<Raw<'_>> for Value {
    fn from(raw: Raw) -> Self {
        raw.value.clone()
    }
}

#[derive(Debug)]
pub struct Missing<'a> {
    values: Vec<KeyType<'a>>,
}
impl<'a> Parser<'a> for Missing<'a> {
    fn from_value(value: &'a Value) -> Result<Option<Self>, Error> {
        match value {
            Value::Object(obj) => {
                if let Some(val) = obj.get("missing") {
                    let keys = keys_from_val(val)?;
                    Ok(Some(Missing { values: keys }))
                } else {
                    Ok(None)
                }
            }
            _ => Ok(None),
        }
    }

    fn evaluate(&self, data: &Value) -> Result<Evaluated, Error> {
        let missing_keys = missing_keys(data, &self.values)?;
        Ok(Evaluated::New(Value::Array(missing_keys)))
    }
}
impl From<Missing<'_>> for Value {
    fn from(missing: Missing) -> Self {
        let mut map = Map::with_capacity(1);
        map.insert("missing".into(), Value::from(missing.values));
        Self::Object(map)
    }
}

#[derive(Debug)]
pub struct MissingSome<'a> {
    minimum: u64,
    keys: Vec<KeyType<'a>>,
}
impl<'a> Parser<'a> for MissingSome<'a> {
    fn from_value(value: &'a Value) -> Result<Option<Self>, Error> {
        match value {
            Value::Object(obj) => Ok(obj
                // Option<Value>
                .get("missing_some")
                // Option<Result<(Value, Value), Error>>
                .map(|val| match val {
                    // Parameters must be an array of len 2
                    Value::Array(vals) => match vals.len() {
                        2 => Ok((&vals[0], &vals[1])),
                        _ => Err(Error::InvalidOperation {
                            key: "missing_some".into(),
                            reason: "parameters to missing_some must be of length 2".into(),
                        }),
                    },
                    _ => Err(Error::InvalidOperation {
                        key: "missing_some".into(),
                        reason: "missing_some parameters must be an array of len(2)".into(),
                    }),
                })
                // Option<(Value, Value)>
                .transpose()?
                // Option<Result<(u64, Value), Error>>
                .map(|(min, keys)| match min {
                    // The first parameter must be a valid integer
                    Value::Number(min) => min
                        .as_u64()
                        .ok_or(Error::InvalidOperation {
                            key: "missing_some".into(),
                            reason: "Could not get unsigned 64-bit integer from first parameter"
                                .into(),
                        })
                        .map(|min| (min, keys)),
                    _ => Err(Error::InvalidOperation {
                        key: "missing_some".into(),
                        reason: "First parameter to missing_some must be a number!".into(),
                    }),
                })
                // Option<(u64, Value)>
                .transpose()?
                // Option<Result<(u64, Vec<KeyType>), Error>>
                .map(|(min, keys)| keys_from_val(&keys).map(|keys| (min, keys)))
                // Option<(u64, Vec<KeyType>)>
                .transpose()?
                // Option<MissingSome>
                .map(|(min, keys)| Self { minimum: min, keys })),
            _ => Ok(None),
        }
    }
    fn evaluate(&self, data: &Value) -> Result<Evaluated, Error> {
        let missing = missing_keys(data, &self.keys)?;
        let present_keys = self.keys.len() - missing.len();
        let val = if (present_keys as u64) >= self.minimum {
            Value::Array(Vec::with_capacity(0))
        } else {
            Value::Array(missing)
        };
        Ok(Evaluated::New(val))
    }
}
impl<'a> From<MissingSome<'a>> for Value {
    fn from(missing: MissingSome) -> Self {
        let mut map = Map::with_capacity(1);
        map.insert(
            "missing_some".into(),
            Value::Array(vec![
                Value::Number(Number::from(missing.minimum)),
                Value::from(missing.keys),
            ]),
        );
        Value::Object(map)
    }
}

fn get_key<'a>(data: &'a Value, key: &KeyType) -> Result<Option<&'a Value>, Error> {
    if let Value::Null = data {
        return Ok(None);
    };
    match key {
        KeyType::String(key) => {
            match data {
                Value::Object(_) | Value::Array(_) => {
                    key.split(".").fold(Ok(Some(data)), |acc, i| match acc? {
                        // If a previous key was not found, just send the None on through
                        None => Ok(None),
                        // If the current value is an object, try to get the value
                        Some(Value::Object(map)) => Ok(map.get(i)),
                        // If the current value is an array, we need an integer
                        // index. If integer conversion fails, return an error.
                        Some(Value::Array(arr)) => i
                            .parse::<usize>()
                            .map(|i| arr.get(i))
                            .map_err(|_| Error::InvalidVariable {
                                value: Value::String(String::from(*key)),
                                reason: "Cannot access array data with non-integer key"
                                    .into(),
                            }),
                        _ => Ok(None),
                    })
                }
                // We can only get string values off of objects or arrays. Anything else is an error.
                _ => Err(Error::InvalidData {
                    value: data.clone(),
                    reason: format!(
                        "Cannot get string key '{:?}' from non-object data",
                        key
                    ),
                }),
            }
        }
        KeyType::Number(idx) => {
            match data {
                Value::Array(val) => {
                    idx
                        // Option<u64>
                        .as_u64()
                        // Result<u64, Error>
                        .ok_or(Error::InvalidVariable {
                            value: Value::Number((*idx).clone()),
                            reason: format!("Could not convert value to u64: {:?}", idx),
                        })
                        // Result<usize, Error>>
                        .and_then(|i| {
                            usize::try_from(i).map_err(|e| Error::InvalidVariable {
                                value: Value::Number((*idx).clone()),
                                reason: format!(
                                    "Could not convert value to a system-sized integer: {:?}",
                                    e
                                ),
                            })
                        })
                        // Result<Option<Value>, Error>
                        .map(|idx| val.get(idx))
                }
                _ => Err(Error::InvalidVariable {
                    value: Value::Number((*idx).clone()),
                    reason: "Cannot access non-array data with an index variable"
                        .into(),
                }),
            }
        }
    }
}

fn keys_from_val<'a>(val: &'a Value) -> Result<Vec<KeyType<'a>>, Error> {
    match val {
        Value::Array(vals) => {
            let mut vals_iter = vals.iter();
            let first = vals_iter.next();
            let missing_vals = match first {
                None => Ok(vals),
                // If the first value is a String, check to
                // be sure the rest are too
                Some(Value::String(_)) => {
                    vals_iter.fold(Ok(()), |acc, each| {
                        match each {
                            Value::String(_) => acc,
                            _ => Err(Error::InvalidOperation{
                                key: "missing".into(),
                                reason: format!("All 'missing' parameters must be of the same type. Expected String, got {:?}.", each)
                            })
                        }
                    })?;
                    Ok(vals)
                }
                // If the first value is a Number, check to
                // be sure the rest are too
                Some(Value::Number(_)) => {
                    vals_iter.fold(Ok(()), |acc, each| {
                        match each {
                            Value::Number(_) => acc,
                            _ => Err(Error::InvalidOperation{
                                key: "missing".into(),
                                reason: format!("All 'missing' parameters must be of the same type. Expected Number, got {:?}.", each)
                            })
                        }
                    })?;
                    Ok(vals)
                }
                _ => Err(Error::InvalidOperation {
                    key: "missing".into(),
                    reason: "'missing' parameters must be strings or numbers".into(),
                }),
            }?;
            missing_vals
                .iter()
                .map(|val| match val {
                    Value::String(key) => Ok(KeyType::String(key)),
                    Value::Number(idx) => Ok(KeyType::Number(idx)),
                    _ => Err(Error::UnexpectedError(
                        "Some keys were not strings or numbers even after validation"
                            .into(),
                    )),
                })
                .collect()
        }
        _ => Err(Error::InvalidOperation {
            key: "missing".into(),
            reason: "Parameters to 'missing' must be an array.".into(),
        }),
    }
}

fn missing_keys(data: &Value, keys: &Vec<KeyType>) -> Result<Vec<Value>, Error> {
    Ok(keys
        .iter()
        .map(|v| get_key(data, v))
        .collect::<Result<Vec<Option<&Value>>, Error>>()?
        .iter()
        .zip(keys.iter())
        .filter(|(val, _key)| val.is_none())
        .map(|(_val, key)| Value::from(key))
        .collect::<Vec<Value>>())
}
