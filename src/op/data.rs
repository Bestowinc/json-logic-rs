//! Data Operators

use std::borrow::Cow;
use std::convert::TryFrom;
use std::convert::TryInto;

use serde_json::Value;

use crate::error::Error;
use crate::value::{Evaluated, Parsed};
use crate::NULL;

/// Valid types of variable keys
enum KeyType<'a> {
    Null,
    String(Cow<'a, str>),
    Number(i64),
}
impl<'a> TryFrom<Value> for KeyType<'a> {
    type Error = Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Null => Ok(Self::Null),
            Value::String(s) => Ok(Self::String(Cow::from(s))),
            Value::Number(n) => Ok(Self::Number(n.as_i64().ok_or(
                Error::InvalidVariableKey {
                    value: Value::Number(n),
                    reason: "Numeric keys must be valid integers".into(),
                },
            )?)),
            _ => Err(Error::InvalidVariableKey {
                value: value.clone(),
                reason: "Variable keys must be strings, integers, or null".into(),
            }),
        }
    }
}
impl<'a> TryFrom<&'a Value> for KeyType<'a> {
    type Error = Error;

    fn try_from(value: &'a Value) -> Result<Self, Self::Error> {
        match value {
            Value::Null => Ok(Self::Null),
            Value::String(s) => Ok(Self::String(Cow::from(s))),
            Value::Number(n) => Ok(Self::Number(n.as_i64().ok_or(
                Error::InvalidVariableKey {
                    value: value.clone(),
                    reason: "Numeric keys must be valid integers".into(),
                },
            )?)),
            _ => Err(Error::InvalidVariableKey {
                value: value.clone(),
                reason: "Variable keys must be strings, integers, or null".into(),
            }),
        }
    }
}
impl<'a> TryFrom<Evaluated<'a>> for KeyType<'a> {
    type Error = Error;

    fn try_from(value: Evaluated<'a>) -> Result<Self, Self::Error> {
        match value {
            Evaluated::Raw(v) => v.try_into(),
            Evaluated::New(v) => v.try_into(),
        }
    }
}

/// A get operation that supports negative indexes
fn get<T>(slice: &[T], idx: i64) -> Option<&T> {
    let vec_len = slice.len();
    let usize_idx: usize = idx.abs().try_into().ok()?;

    let adjusted_idx = if idx >= 0 {
        usize_idx
    } else {
        vec_len.checked_sub(usize_idx)?
    };

    slice.get(adjusted_idx)
}

/// Retrieve a variable from the data
///
/// Note that the reference implementation does not support negative
/// indexing for numeric values, but we do.
pub fn var(data: &Value, args: &Vec<&Value>) -> Result<Value, Error> {
    let arg_count = args.len();
    if arg_count == 0 {
        return Ok(data.clone());
    };

    let key = args[0].try_into()?;
    let val = get_key(data, key);

    Ok(val.unwrap_or(if arg_count < 2 {
        NULL
    } else {
        let _parsed_default = Parsed::from_value(args[1])?;
        _parsed_default.evaluate(&data)?.into()
    }))
}

/// Check for keys that are missing from the data
pub fn missing(data: &Value, args: &Vec<&Value>) -> Result<Value, Error> {
    let mut missing_keys: Vec<Value> = Vec::new();

    // This bit of insanity is because for some reason the reference
    // implementation is tested to do this, i.e. if missing is passed
    // multiple args and the first arg is an array, _that_ array is
    // treated as the only argument.
    let inner_vec: Vec<&Value>;
    let adjusted_args = if args.len() > 0 {
        match args[0] {
            Value::Array(vals) => {
                inner_vec = vals.iter().collect();
                &inner_vec
            }
            _ => args,
        }
    } else {
        args
    };

    adjusted_args.into_iter().fold(Ok(()), |had_error, arg| {
        had_error?;
        let key: KeyType = (*arg).try_into()?;
        match key {
            KeyType::Null => Ok(()),
            _ => {
                let val = get_key(data, key);
                if val.is_none() {
                    missing_keys.push((*arg).clone());
                };
                Ok(())
            }
        }
    })?;
    Ok(Value::Array(missing_keys))
}

/// Check whether a minimum threshold of keys are present in the data
///
/// Note that I think this function is confusingly named. `contains_at_least`
/// might be better, or something like that. Regardless, it checks to see how
/// many of the specified keys are present in the data. If there are equal
/// to or more than the threshold value _present_ in the data, an empty
/// array is returned. Otherwise, an array containing all missing keys
/// is returned.
pub fn missing_some(data: &Value, args: &Vec<&Value>) -> Result<Value, Error> {
    let (threshold_arg, keys_arg) = (args[0], args[1]);

    let threshold = match threshold_arg {
        Value::Number(n) => n.as_u64(),
        _ => None,
    }
    .ok_or(Error::InvalidArgument {
        value: threshold_arg.clone(),
        operation: "missing_some".into(),
        reason: "missing_some threshold must be a valid, positive integer".into(),
    })?;

    let keys = match keys_arg {
        Value::Array(keys) => Ok(keys),
        _ => Err(Error::InvalidArgument {
            value: keys_arg.clone(),
            operation: "missig_some".into(),
            reason: "missing_some keys must be an array".into(),
        }),
    }?;

    let mut missing_keys: Vec<Value> = Vec::new();
    let present_count = keys.into_iter().fold(Ok(0 as u64), |last, key| {
        // Don't bother evaluating once we've met the threshold.
        let prev_present_count = last?;
        if prev_present_count >= threshold {
            return Ok(prev_present_count);
        };

        let parsed_key: KeyType = key.try_into()?;
        let current_present_count = match parsed_key {
            // In the reference implementation, I believe null actually is
            // buggy. Since usually, getting "null" as a var against the
            // data returns the whole data, "null" in a `missing_some`
            // list of keys _automatically_ counts as a present key, regardless
            // of what keys are in the data. This behavior is neither in the
            // specification nor the tests, so I'm going to SKIP null keys,
            // since they aren't valid Object or Array keys in JSON.
            KeyType::Null => prev_present_count,
            _ => {
                if get_key(data, parsed_key).is_none() && !missing_keys.contains(key) {
                    missing_keys.push((*key).clone());
                    prev_present_count
                } else {
                    prev_present_count + 1
                }
            }
        };
        Ok(current_present_count)
    })?;

    let met_threshold = present_count >= threshold;

    if met_threshold {
        Ok(Value::Array(vec![]))
    } else {
        Ok(Value::Array(missing_keys))
    }
}

fn get_key(data: &Value, key: KeyType) -> Option<Value> {
    match key {
        // If the key is null, we return the data, always, even if there
        // is a default parameter.
        KeyType::Null => return Some(data.clone()),
        KeyType::String(k) => get_str_key(data, k),
        KeyType::Number(i) => match data {
            Value::Object(_) => get_str_key(data, i.to_string()),
            Value::Array(arr) => get(arr, i).map(Value::clone),
            Value::String(s) => {
                let s_vec: Vec<char> = s.chars().collect();
                get(&s_vec, i).map(|c| c.to_string()).map(Value::String)
            }
            _ => None,
        },
    }
}

fn get_str_key<K: AsRef<str>>(data: &Value, key: K) -> Option<Value> {
    let k = key.as_ref();
    if k == "" {
        return Some(data.clone());
    };
    match data {
        Value::Object(_) | Value::Array(_) | Value::String(_) => {
            // Exterior ref in case we need to make a new value in the match.
            k.split(".").fold(Some(data.clone()), |acc, i| match acc? {
                // If the current value is an object, try to get the value
                Value::Object(map) => map.get(i).map(Value::clone),
                // If the current value is an array, we need an integer
                // index. If integer conversion fails, return None.
                Value::Array(arr) => i
                    .parse::<i64>()
                    .ok()
                    .and_then(|i| get(&arr, i))
                    .map(Value::clone),
                // Same deal if it's a string.
                Value::String(s) => {
                    let s_chars: Vec<char> = s.chars().collect();
                    i.parse::<i64>()
                        .ok()
                        .and_then(|i| get(&s_chars, i))
                        .map(|c| c.to_string())
                        .map(Value::String)
                }
                // This handles cases where we've got an un-indexable
                // type or similar.
                _ => None,
            })
        }
        _ => None,
    }
}
