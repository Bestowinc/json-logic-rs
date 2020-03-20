//! FUnctions

use serde_json::Value;

use crate::error::Error;

/// A (potentially user-defined) function
///
/// The simplest function definition looks like:
///
/// ```jsonc
/// {
///     "def": [        // function definition operator
///         "is_even",  // function name
///         [a],        // function params
///         // function expression
///         {
///             "===": [
///                 {"%": [{"param": "a"}, 2]},
///                 0
///             ]
///         }
///     ]
/// }
/// ```
///
/// Once defined, the above function can be used like:
///
/// ```jsonc
/// {"is_even": [5]}  // false
/// {"is_even": [2]}  // true
/// ```
///
/// Function expressions may use any of the standard operators or any
/// previously defined functions.
///
pub struct Function {
    name: String,
    params: Vec<String>,
    expression: Value,
}
