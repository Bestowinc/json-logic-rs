//! String Operations

use serde_json::Value;
use std::cmp;
use std::convert::TryInto;

use crate::error::Error;
use crate::js_op;
use crate::NULL;

/// Concatenate strings.
///
/// Note: the reference implementation just uses JS' builtin string
/// concatenation with implicit casting, so e.g. `cast("foo", {})`
/// evaluates to `"foo[object Object]". Here we explicitly require all
/// arguments to be strings, because the specification explicitly defines
/// `cat` as a string operation.
pub fn cat(items: &Vec<&Value>) -> Result<Value, Error> {
    let mut rv = String::from("");
    items
        .into_iter()
        .map(|i| match i {
            Value::String(i_string) => Ok(i_string.clone()),
            _ => Ok(js_op::to_string(i)),
        })
        .fold(Ok(&mut rv), |acc: Result<&mut String, Error>, i| {
            let rv = acc?;
            rv.push_str(&i?);
            Ok(rv)
        })?;
    Ok(Value::String(rv))
}

/// Get a substring by index
///
/// Note: the reference implementation casts the first argument to a string,
/// but since the specification explicitly defines this as a string operation,
/// the argument types are enforced here to avoid unpredictable behavior.
pub fn substr(items: &Vec<&Value>) -> Result<Value, Error> {
    // We can only have 2 or 3 arguments. Number of arguments is validated elsewhere.
    let (string_arg, idx_arg) = (items[0], items[1]);
    let limit_opt: Option<&Value>;
    if items.len() > 2 {
        limit_opt = Some(items[2]);
    } else {
        limit_opt = None;
    }

    let string = match string_arg {
        Value::String(s) => s,
        _ => {
            return Err(Error::InvalidArgument {
                value: string_arg.clone(),
                operation: "substr".into(),
                reason: "First argument to substr must be a string".into(),
            })
        }
    };
    let idx = match idx_arg {
        Value::Number(n) => {
            if let Some(int) = n.as_i64() {
                int
            } else {
                return Err(Error::InvalidArgument {
                    value: idx_arg.clone(),
                    operation: "substr".into(),
                    reason: "Second argument to substr must be an integer".into(),
                });
            }
        }
        _ => {
            return Err(Error::InvalidArgument {
                value: idx_arg.clone(),
                operation: "substr".into(),
                reason: "Second argument to substr must be a number".into(),
            })
        }
    };
    let limit = limit_opt
        .map(|limit_arg| match limit_arg {
            Value::Number(n) => {
                if let Some(int) = n.as_i64() {
                    Ok(int)
                } else {
                    Err(Error::InvalidArgument {
                        value: limit_arg.clone(),
                        operation: "substr".into(),
                        reason: "Optional third argument to substr must be an integer".into(),
                    })
                }
            }
            _ => Err(Error::InvalidArgument {
                value: limit_arg.clone(),
                operation: "substr".into(),
                reason: "Optional third argument to substr must be a number".into(),
            }),
        })
        .transpose()?;

    let string_len = string.len();

    let idx_abs: usize = idx.abs().try_into().map_err(|e| Error::InvalidArgument {
        value: idx_arg.clone(),
        operation: "substr".into(),
        reason: format!(
            "The number {} is too large to index strings on this system",
            e
        ),
    })?;
    let start_idx = match idx {
        // If the index is negative it means "number of characters prior to the
        // end of the string from which to start", and corresponds to the string
        // length minus the index.
        idx if idx < 0 => string_len.checked_sub(idx_abs).unwrap_or(0),
        // A positive index is simply the starting point. Max starting point
        // is the length, which will yield an empty string.
        _ => cmp::min(string_len, idx_abs),
    };

    let end_idx = match limit {
        None => string_len,
        Some(l) => {
            let limit_abs: usize = l.abs().try_into().map_err(|e| Error::InvalidArgument {
                value: limit_opt.or(Some(&NULL)).map(|v| v.clone()).unwrap(),
                operation: "substr".into(),
                reason: format!(
                    "The number {} is too large to index strings on this system",
                    e
                ),
            })?;
            match l {
                // If the limit is negative, it means "characters before the end
                // at which to stop", corresponding to an index of either 0 or
                // the length of the string minus the limit.
                l if l < 0 => string_len.checked_sub(limit_abs).unwrap_or(0),
                // A positive limit indicates the number of characters to take,
                // so it corresponds to an index of the start index plus the
                // limit (with a maximum value of the string length).
                _ => cmp::min(
                    string_len,
                    start_idx.checked_add(limit_abs).unwrap_or(string_len),
                ),
            }
        }
    };

    let count_in_substr = end_idx.checked_sub(start_idx).unwrap_or(0);

    // Iter over our expected count rather than indexing directly to avoid
    // potential panics if any of our math is wrong.
    Ok(Value::String(
        string
            .chars()
            .skip(start_idx)
            .take(count_in_substr)
            .collect(),
    ))
}
