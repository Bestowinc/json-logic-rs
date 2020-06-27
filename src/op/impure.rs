//! Impure Operations

use serde_json::Value;

use crate::error::Error;

/// Log the Operation's Value(s)
///
/// The reference implementation ignores any arguments beyond the first,
/// and the specification seems to indicate that the first argument is
/// the only one considered, so we're doing the same.
pub fn log(items: &Vec<&Value>) -> Result<Value, Error> {
    println!("{}", items[0]);
    Ok(items[0].clone())
}
