use serde_json::{Number, Value};

use crate::error::Error;
use crate::op::{DataOperation, LazyOperation, Operation};
use crate::Parser;

/// A Parsed JSON value
///
/// Parsed values are one of:
///   - An operation whose arguments are eagerly evaluated
///   - An operation whose arguments are lazily evaluated
///   - A raw value: a non-rule, raw JSON value
#[derive(Debug)]
pub enum Parsed<'a> {
    Operation(Operation<'a>),
    LazyOperation(LazyOperation<'a>),
    DataOperation(DataOperation<'a>),
    Raw(Raw<'a>),
}
impl<'a> Parsed<'a> {
    /// Recursively parse a value
    pub fn from_value(value: &'a Value) -> Result<Self, Error> {
        Operation::from_value(value)?
            .map(Self::Operation)
            // .or(Operation::from_value(value)?.map(Self::Operation))
            .or(LazyOperation::from_value(value)?.map(Self::LazyOperation))
            .or(DataOperation::from_value(value)?.map(Self::DataOperation))
            .or(Raw::from_value(value)?.map(Self::Raw))
            .ok_or(Error::UnexpectedError(format!(
                "Failed to parse Value {:?}",
                value
            )))
    }

    pub fn from_values(values: Vec<&'a Value>) -> Result<Vec<Self>, Error> {
        values
            .into_iter()
            .map(Self::from_value)
            .collect::<Result<Vec<Self>, Error>>()
    }

    pub fn evaluate(&self, data: &'a Value) -> Result<Evaluated, Error> {
        match self {
            Self::Operation(op) => op.evaluate(data),
            Self::LazyOperation(op) => op.evaluate(data),
            Self::DataOperation(op) => op.evaluate(data),
            Self::Raw(val) => val.evaluate(data),
        }
    }
}
impl From<Parsed<'_>> for Value {
    fn from(item: Parsed) -> Value {
        match item {
            Parsed::Operation(op) => Value::from(op),
            Parsed::LazyOperation(op) => Value::from(op),
            Parsed::DataOperation(op) => Value::from(op),
            Parsed::Raw(raw) => Value::from(raw),
        }
    }
}

/// A Raw JSON value
///
/// Raw values are those that are not any known operation. A raw value may
/// be of any valid JSON type.
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

/// An Evaluated JSON value
///
/// An evaluated value is one of:
///   - A new value: either a calculated Rule or a filled Variable
///   - A raw value: a non-rule, raw JSON value
#[derive(Debug)]
pub enum Evaluated<'a> {
    New(Value),
    Raw(&'a Value),
}

impl From<Evaluated<'_>> for Value {
    fn from(item: Evaluated) -> Self {
        match item {
            Evaluated::Raw(val) => val.clone(),
            Evaluated::New(val) => val,
        }
    }
}

pub fn to_number_value(number: f64) -> Result<Value, Error> {
    if number.fract() == 0.0 {
        Ok(Value::Number(Number::from(number as i64)))
    } else {
        Number::from_f64(number)
            .ok_or(Error::UnexpectedError(format!(
                "Could not make JSON number from result {:?}",
                number
            )))
            .map(Value::Number)
    }
}
