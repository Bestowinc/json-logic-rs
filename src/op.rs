//! Operators
//!

use phf::phf_map;
use serde_json::{Map, Value};
use std::fmt;

use crate::error::Error;
use crate::value::{Evaluated, Parsed};
use crate::{js_op, Parser, NULL};

pub struct Operator {
    symbol: &'static str,
    operator: OperatorFn,
    num_params: Option<std::ops::Range<usize>>,
}
impl Operator {
    pub fn execute(&self, items: &Vec<&Value>) -> Result<Value, Error> {
        (self.operator)(items)
    }

    pub fn args_are_valid_len<T>(&self, args: &Vec<T>) -> Result<(), std::ops::Range<usize>> {
        self.num_params
            .as_ref()
            // If we've got a specified len, check the args
            .map(|range| {
                if range.contains(&args.len()) {
                    Ok(())
                } else {
                    Err(range.clone())
                }
            })
            // Otherwise, we're fine.
            .unwrap_or(Ok(()))
    }
}
impl fmt::Debug for Operator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Operator")
            .field("symbol", &self.symbol)
            .field("operator", &"<operator fn>")
            .finish()
    }
}

pub struct LazyOperator {
    symbol: &'static str,
    operator: LazyOperatorFn,
    num_params: Option<std::ops::Range<usize>>,
}
impl LazyOperator {
    pub fn execute(&self, data: &Value, items: &Vec<&Value>) -> Result<Value, Error> {
        (self.operator)(data, items)
    }

    pub fn args_are_valid_len<T>(&self, args: &Vec<T>) -> Result<(), std::ops::Range<usize>> {
        self.num_params
            .as_ref()
            // If we've got a specified len, check the args
            .map(|range| {
                if range.contains(&args.len()) {
                    Ok(())
                } else {
                    Err(range.clone())
                }
            })
            // Otherwise, we're fine.
            .unwrap_or(Ok(()))
    }
}
impl fmt::Debug for LazyOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Operator")
            .field("symbol", &self.symbol)
            .field("operator", &"<operator fn>")
            .finish()
    }
}

type OperatorFn = fn(&Vec<&Value>) -> Result<Value, Error>;
type LazyOperatorFn = fn(&Value, &Vec<&Value>) -> Result<Value, Error>;

pub const OPERATOR_MAP: phf::Map<&'static str, Operator> = phf_map! {
    "==" => Operator {
        symbol: "==",
        operator: |items| Ok(Value::Bool(js_op::abstract_eq(items[0], items[1]))),
        num_params: Some(2..3)},
    "===" => Operator {
        symbol: "===",
        operator: |items| Ok(Value::Bool(js_op::strict_eq(items[0], items[1]))),
        num_params: Some(2..3)},
    "!==" => Operator {
        symbol: "!==",
        operator: |items| Ok(Value::Bool(js_op::strict_ne(items[0], items[1]))),
        num_params: Some(2..3)},
};

pub const LAZY_OPERATOR_MAP: phf::Map<&'static str, LazyOperator> = phf_map! {
    "if" => LazyOperator {
        symbol: "if",
        operator: op_if,
        // note this is a practical limit more than theoretical one. The spec
        // doesn't say anything about not supporting more than 4.2 billion
        // arguments, but we're drawing a line in the sand.
        num_params: Some(3..std::u32::MAX as usize),
    }
};


fn op_if(data: &Value, args: &Vec<&Value>) -> Result<Value, Error> {
    args.into_iter().enumerate().fold(
        Ok((NULL, false, false)),
        |last_res, (i, val)| {
            let (last_eval, was_truthy, should_return) = last_res?;
            // We hit a final value already
            if should_return {
                Ok((last_eval, was_truthy, should_return))
            }
            // Potential false-value, initial evaluation, else-if clause
            else if i % 2 == 0 {
                let parsed = Parsed::from_value(val)?;
                let eval = parsed.evaluate(data)?;
                let truthy = match eval {
                    Evaluated::New(ref v) => is_truthy(v),
                    Evaluated::Raw(v) => is_truthy(v),
                };
                // We're not sure we're the return value, so don't
                // force a return.
                Ok((Value::from(eval), truthy, false))
            }
            // We're a possible true-value
            else {
                // If there was a previous evaluation and it was truthy,
                // return, and indicate we're a final value.
                if was_truthy {
                    let parsed = Parsed::from_value(val)?;
                    let t_eval = parsed.evaluate(data)?;
                    Ok((Value::from(t_eval), true, true))
                } else {
                    // Ignore ourselves
                    Ok((last_eval, was_truthy, should_return))
                }
            }
        }
    ).map(|rv| rv.0)
}


/// An operation that doesn't do any recursive parsing or evaluation.
///
/// Any operator functions used must handle parsing of values themselves.
#[derive(Debug)]
pub struct LazyOperation<'a> {
    operator: &'a LazyOperator,
    arguments: &'a Vec<Value>,
}
impl<'a> Parser<'a> for LazyOperation<'a> {
    fn from_value(value: &'a Value) -> Result<Option<Self>, Error> {
        // We can only be an operation if we're an object
        let obj = match value {
            Value::Object(obj) => obj,
            _ => return Ok(None),
        };
        // With just one key.
        if obj.len() != 1 {
            return Ok(None);
        };

        // We've already validated the length to be one, so any error
        // here is super unexpected.
        let key = obj.keys().next().ok_or(Error::UnexpectedError(format!(
            "could not get first key from len(1) object: {:?}",
            obj
        )))?;
        let val = obj.get(key).ok_or(Error::UnexpectedError(format!(
            "could not get value for key '{}' from len(1) object: {:?}",
            key, obj
        )))?;

        // See if the key is an operator. If it's not, return None.
        let op = match LAZY_OPERATOR_MAP.get(key.as_str()) {
            Some(op) => op,
            _ => return Ok(None),
        };

        // Arguments must be an Array. Anything else is an error.
        let args = match val {
            Value::Array(args) => args,
            _ => {
                return Err(Error::InvalidOperation {
                    key: key.into(),
                    reason: "Values for operator keys must be arrays".into(),
                })
            }
        };

        // If the operator specifies a range for params, check them.
        op.args_are_valid_len(&args)
            .map_err(|exp_range| Error::WrongArgumentCount {
                expected: exp_range,
                actual: args.len(),
            })?;

        Ok(Some(LazyOperation {
            operator: op,
            arguments: args,
        }))
    }

    fn evaluate(&self, data: &'a Value) -> Result<Evaluated, Error> {
        self.operator
            .execute(data, &self.arguments.iter().collect())
            .map(Evaluated::New)
    }
}

impl From<LazyOperation<'_>> for Value {
    fn from(op: LazyOperation) -> Value {
        let mut rv = Map::with_capacity(1);
        rv.insert(
            op.operator.symbol.into(),
            Value::Array(op.arguments.clone()),
        );
        Value::Object(rv)
    }
}

#[derive(Debug)]
pub struct Operation<'a> {
    operator: &'a Operator,
    arguments: Vec<Parsed<'a>>,
}
impl<'a> Parser<'a> for Operation<'a> {
    fn from_value(value: &'a Value) -> Result<Option<Self>, Error> {
        // We can only be an operation if we're an object
        let obj = match value {
            Value::Object(obj) => obj,
            _ => return Ok(None),
        };
        // With just one key.
        if obj.len() != 1 {
            return Ok(None);
        };

        // We've already validated the length to be one, so any error
        // here is super unexpected.
        let key = obj.keys().next().ok_or(Error::UnexpectedError(format!(
            "could not get first key from len(1) object: {:?}",
            obj
        )))?;
        let val = obj.get(key).ok_or(Error::UnexpectedError(format!(
            "could not get value for key '{}' from len(1) object: {:?}",
            key, obj
        )))?;

        // See if the key is an operator. If it's not, return None.
        let op = match OPERATOR_MAP.get(key.as_str()) {
            Some(op) => op,
            _ => return Ok(None),
        };

        // Arguments must be an Array. Anything else is an error.
        let args = match val {
            Value::Array(args) => args,
            _ => {
                return Err(Error::InvalidOperation {
                    key: key.into(),
                    reason: "Values for operator keys must be arrays".into(),
                })
            }
        };

        // If the operator specifies a range for params, check them.
        op.args_are_valid_len(&args)
            .map_err(|exp_range| Error::WrongArgumentCount {
                expected: exp_range,
                actual: args.len(),
            })?;

        Ok(Some(Operation {
            operator: op,
            arguments: Parsed::from_values(args)?,
        }))
    }

    /// Evaluate the operation after recursively evaluating any nested operations
    fn evaluate(&self, data: &'a Value) -> Result<Evaluated, Error> {
        let arguments = self
            .arguments
            .iter()
            .map(|value| value.evaluate(data).map(|evaluated| Value::from(evaluated)))
            .collect::<Result<Vec<Value>, Error>>()?;
        self.operator
            .execute(&arguments.iter().collect())
            .map(Evaluated::New)
    }
}

impl From<Operation<'_>> for Value {
    fn from(op: Operation) -> Value {
        let mut rv = Map::with_capacity(1);
        let values = op
            .arguments
            .into_iter()
            .map(Value::from)
            .collect::<Vec<Value>>();
        rv.insert(op.operator.symbol.into(), Value::Array(values));
        Value::Object(rv)
    }
}

/// Return whether a value is "truthy" by the JSONLogic spec
///
/// The spec (http://jsonlogic.com/truthy) defines truthy values that
/// diverge slightly from raw JavaScript. This ensures a matching
/// interpretation.
///
/// In general, the spec specifies that values are truthy or falsey
/// depending on their containing something, e.g. non-zero integers,
/// non-zero length strings, and non-zero length arrays are truthy.
/// This does not apply to objects, which are always truthy.
pub fn is_truthy(val: &Value) -> bool {
    match val {
        Value::Null => false,
        Value::Bool(v) => *v,
        Value::Number(v) => v
            .as_f64()
            .map(|v_num| if v_num == 0.0 { false } else { true })
            .unwrap_or(false),
        Value::String(v) => {
            if v == "" {
                false
            } else {
                true
            }
        }
        Value::Array(v) => {
            if v.len() == 0 {
                false
            } else {
                true
            }
        }
        Value::Object(_) => true,
    }
}

#[cfg(test)]
mod test_operators {
    use super::*;

    /// All operators symbols must match their keys
    #[test]
    fn test_symbols() {
        OPERATOR_MAP
            .into_iter()
            .for_each(|(k, op)| assert_eq!(*k, op.symbol))
    }
}

#[cfg(test)]
mod test_truthy {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_truthy() {
        let trues = [
            json!(true),
            json!([1]),
            json!([1, 2]),
            json!({}),
            json!({"a": 1}),
            json!(1),
            json!(-1),
            json!("foo"),
        ];

        let falses = [json!(false), json!([]), json!(""), json!(0), json!(null)];

        trues.iter().for_each(|v| assert!(is_truthy(&v)));
        falses.iter().for_each(|v| assert!(!is_truthy(&v)));
    }
}
