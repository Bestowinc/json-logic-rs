use serde_json::Value;

use std::convert::{From, TryFrom};

use crate::data;
use crate::error::Error;
use crate::op::Operation;

/// A Parsed JSON value
///
/// Parsed values are one of:
///   - A rule: a valid JSONLogic rule which can be evaluated
///   - A raw value: a non-rule, raw JSON value
#[derive(Debug)]
pub enum Parsed<'a> {
    Operation(Operation<'a>),
    Raw(&'a Value),
    Variable(data::Variable<'a>),
    Missing(data::Missing<'a>),
}
impl<'a> Parsed<'a> {
    /// Recursively parse a value
    pub fn from_value(value: &'a Value) -> Result<Self, Error> {
        Ok(data::Variable::from_value(value)?
            .map(Self::Variable)
            .or(Operation::from_value(value)?.map(Self::Operation))
            .or(data::Missing::from_value(value)?.map(Self::Missing))
            .unwrap_or(Self::Raw(value)))
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
            Self::Raw(val) => Ok(Evaluated::Raw(*val)),
            Self::Variable(var) => var.evaluate(data).map(Evaluated::Raw),
            Self::Missing(missing) => missing.evaluate(data).map(Evaluated::New),
        }
    }
}
impl TryFrom<Parsed<'_>> for Value {
    type Error = Error;

    fn try_from(item: Parsed) -> Result<Self, Self::Error> {
        match item {
            Parsed::Operation(op) => Value::try_from(op),
            Parsed::Raw(val) => Ok(val.clone()),
            Parsed::Variable(var) => Ok(Value::from(var)),
            Parsed::Missing(missing) => Ok(Value::from(missing)),
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
