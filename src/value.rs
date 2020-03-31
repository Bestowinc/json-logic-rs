use serde_json::Value;

use crate::error::Error;
use crate::op::{LazyOperation, Operation};
use crate::{data, Parser};

/// A Parsed JSON value
///
/// Parsed values are one of:
///   - A rule: a valid JSONLogic rule which can be evaluated
///   - A raw value: a non-rule, raw JSON value
#[derive(Debug)]
pub enum Parsed<'a> {
    Operation(Operation<'a>),
    LazyOperation(LazyOperation<'a>),
    Raw(data::Raw<'a>),
    Variable(data::Variable<'a>),
    Missing(data::Missing<'a>),
    MissingSome(data::MissingSome<'a>),
}
impl<'a> Parsed<'a> {
    /// Recursively parse a value
    pub fn from_value(value: &'a Value) -> Result<Self, Error> {
        data::Variable::from_value(value)?
            .map(Self::Variable)
            .or(data::Missing::from_value(value)?.map(Self::Missing))
            .or(data::MissingSome::from_value(value)?.map(Self::MissingSome))
            .or(Operation::from_value(value)?.map(Self::Operation))
            .or(LazyOperation::from_value(value)?.map(Self::LazyOperation))
            .or(data::Raw::from_value(value)?.map(Self::Raw))
            .ok_or(Error::UnexpectedError(format!(
                "Failed to parse Value {:?}",
                value
            )))
    }

    pub fn from_values(values: &'a Vec<Value>) -> Result<Vec<Self>, Error> {
        values
            .iter()
            .map(Self::from_value)
            .collect::<Result<Vec<Self>, Error>>()
    }

    pub fn evaluate(&self, data: &'a Value) -> Result<Evaluated, Error> {
        match self {
            Self::Operation(op) => op.evaluate(data),
            Self::LazyOperation(op) => op.evaluate(data),
            Self::Raw(val) => val.evaluate(data),
            Self::Variable(var) => var.evaluate(data),
            Self::Missing(missing) => missing.evaluate(data),
            Self::MissingSome(missing) => missing.evaluate(data),
        }
    }
}
impl From<Parsed<'_>> for Value {
    fn from(item: Parsed) -> Value {
        match item {
            Parsed::Operation(op) => Value::from(op),
            Parsed::LazyOperation(op) => Value::from(op),
            Parsed::Raw(raw) => Value::from(raw),
            Parsed::Variable(var) => Value::from(var),
            Parsed::Missing(missing) => Value::from(missing),
            Parsed::MissingSome(missing) => Value::from(missing),
        }
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
