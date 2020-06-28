//! Error handling
//!
use serde_json::Value;
use thiserror;

use crate::op::NumParams;

/// Public error enumeration
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Invalid data - value: {value:?}, reason: {reason:?}")]
    InvalidData { value: Value, reason: String },

    #[error("Invalid rule - operator: '{key:?}', reason: {reason:?}")]
    InvalidOperation { key: String, reason: String },

    #[error("Invalid variable - '{value:?}', reason: {reason:?}")]
    InvalidVariable { value: Value, reason: String },

    #[error("Invalid variable key - '{value:?}', reason: {reason:?}")]
    InvalidVariableKey { value: Value, reason: String },

    #[error("Invalid argument for '{operation}' - '{value:?}', reason: {reason}")]
    InvalidArgument {
        value: Value,
        operation: String,
        reason: String,
    },

    #[error("Invalid variable mapping - {0} is not an object.")]
    InvalidVarMap(Value),

    #[error("Encountered an unexpected error. Please raise an issue on GitHub and include the following error message: {0}")]
    UnexpectedError(String),

    #[error("Wrong argument count - expected: {expected:?}, actual: {actual:?}")]
    WrongArgumentCount { expected: NumParams, actual: usize },
}
