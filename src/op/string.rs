//! String Operations

use serde_json::Value;

use crate::error::Error;

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
            Value::String(i_string) => Ok(i_string),
            _ => Err(Error::InvalidArgument {
                value: (**i).clone(),
                operation: "cat".into(),
                reason: "All arguments to `cat` must be strings".into(),
            }),
        })
        .fold(Ok(&mut rv), |acc: Result<&mut String, Error>, i| {
            let rv = acc?;
            rv.push_str(i?);
            Ok(rv)
        })?;
    Ok(Value::String(rv))
}
