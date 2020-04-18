//! Operators
//!

use phf::phf_map;
use serde_json::{Map, Number, Value};
use std::fmt;

use crate::error::Error;
use crate::value::{Evaluated, Parsed};
use crate::{js_op, Parser, NULL};

#[derive(Debug, Clone)]
pub enum NumParams {
    None,
    Any,
    Unary,
    Exactly(usize),
    AtLeast(usize),
    Variadic(std::ops::Range<usize>),
}
impl NumParams {
    fn is_valid_len(&self, len: &usize) -> bool {
        match self {
            Self::None => len == &0,
            Self::Any => true,
            Self::Unary => len == &1,
            Self::AtLeast(num) => len >= num,
            Self::Exactly(num) => len == num,
            Self::Variadic(range) => range.contains(len),
        }
    }
    fn check_len<'a>(&self, len: &'a usize) -> Result<&'a usize, Error> {
        match self.is_valid_len(len) {
            true => Ok(len),
            false => Err(Error::WrongArgumentCount{
                expected: self.clone(),
                actual: len.clone()
            })
        }
    }
}

pub struct Operator {
    symbol: &'static str,
    operator: OperatorFn,
    num_params: NumParams
}
impl Operator {
    pub fn execute(&self, items: &Vec<&Value>) -> Result<Value, Error> {
        (self.operator)(items)
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
    num_params: NumParams,
}
impl LazyOperator {
    pub fn execute(&self, data: &Value, items: &Vec<&Value>) -> Result<Value, Error> {
        (self.operator)(data, items)
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
        num_params: NumParams::Exactly(2)},
    "!=" => Operator {
        symbol: "!=",
        operator: |items| Ok(Value::Bool(js_op::abstract_ne(items[0], items[1]))),
        num_params: NumParams::Exactly(2)},
    "===" => Operator {
        symbol: "===",
        operator: |items| Ok(Value::Bool(js_op::strict_eq(items[0], items[1]))),
        num_params: NumParams::Exactly(2)},
    "!==" => Operator {
        symbol: "!==",
        operator: |items| Ok(Value::Bool(js_op::strict_ne(items[0], items[1]))),
        num_params: NumParams::Exactly(2)},
    "<" => Operator {
        symbol: "<",
        operator: op_lt,
        num_params: NumParams::Variadic(2..4),
    },
    "<=" => Operator {
        symbol: "<=",
        operator: op_lte,
        num_params: NumParams::Variadic(2..4),
    },
    "+" => Operator {
        symbol: "+",
        operator: |items| js_op::parse_float_add(items)
            .map(Number::from_f64)
            .and_then(|opt| opt.ok_or(
                Error::UnexpectedError(
                    "Could not convert sum into a JSON number".into())
                )
            )
            .map(Value::Number),
        num_params: NumParams::Any,
    },
    // Note: the ! and !! behavior conforms to the specification, but not the
    // reference implementation. The specification states: "Note: unary
    // operators can also take a single, non array argument." However,
    // if a non-unary array of arguments is passed to `!` or `!!` in the
    // reference implementation, it treats them as though they were a unary
    // array argument. I have chosen to conform to the spec because it leads
    // to less surprising behavior. I also think that the idea of taking
    // non-array unary arguments is ridiculous, particularly given that
    // the homepage of jsonlogic _also_ states that a "Virtue" of jsonlogic
    // is that it is "Consistent. `{"operator" : ["values" ... ]}` Always"
    "!" => Operator {
        symbol: "!",
        operator: |items| Ok(Value::Bool(!truthy(items[0]))),
        num_params: NumParams::Unary,
    },
    "!!" => Operator {
        symbol: "!!",
        operator: |items| Ok(Value::Bool(truthy(items[0]))),
        num_params: NumParams::Unary,
    },
};

pub const LAZY_OPERATOR_MAP: phf::Map<&'static str, LazyOperator> = phf_map! {
    "if" => LazyOperator {
        symbol: "if",
        operator: op_if,
        num_params: NumParams::AtLeast(3),
    },
    "or" => LazyOperator {
        symbol: "or",
        operator: op_or,
        num_params: NumParams::AtLeast(1),
    },
    "and" => LazyOperator {
        symbol: "and",
        operator: op_and,
        num_params: NumParams::AtLeast(1),
    },
};

/// Implement the "if" operator
///
/// The base case works like: [condition, true, false]
/// However, it can lso work like:
///     [condition, true, condition2, true2, false2]
///     for an if/elseif/else type of operation
fn op_if(data: &Value, args: &Vec<&Value>) -> Result<Value, Error> {
    args.into_iter()
        .enumerate()
        .fold(Ok((NULL, false, false)), |last_res, (i, val)| {
            let (last_eval, was_truthy, should_return) = last_res?;
            // We hit a final value already
            if should_return {
                Ok((last_eval, was_truthy, should_return))
            }
            // Potential false-value, initial evaluation, else-if clause
            else if i % 2 == 0 {
                let parsed = Parsed::from_value(val)?;
                let eval = parsed.evaluate(data)?;
                let is_truthy = match eval {
                    Evaluated::New(ref v) => truthy(v),
                    Evaluated::Raw(v) => truthy(v),
                };
                // We're not sure we're the return value, so don't
                // force a return.
                Ok((Value::from(eval), is_truthy, false))
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
        })
        .map(|rv| rv.0)
}

/// Perform short-circuiting or evaluation
fn op_or(data: &Value, args: &Vec<&Value>) -> Result<Value, Error> {
    enum OrResult {
        Uninitialized,
        Truthy(Value),
        Current(Value),
    }

    let eval = args
        .into_iter()
        .fold(Ok(OrResult::Uninitialized), |last_res, current| {
            let last_eval = last_res?;

            // if we've found a truthy value, don't evaluate anything else
            if let OrResult::Truthy(_) = last_eval {
                return Ok(last_eval);
            }

            let parsed = Parsed::from_value(current)?;
            let evaluated = parsed.evaluate(data)?;

            if truthy_from_evaluated(&evaluated) {
                return Ok(OrResult::Truthy(evaluated.into()));
            }

            Ok(OrResult::Current(evaluated.into()))
        })?;

    match eval {
        OrResult::Truthy(v) => Ok(v),
        OrResult::Current(v) => Ok(v),
        _ => Err(Error::UnexpectedError(
            "Or operation had no values to operate on".into(),
        )),
    }
}

/// Perform short-circuiting and evaluation
fn op_and(data: &Value, args: &Vec<&Value>) -> Result<Value, Error> {
    enum AndResult {
        Uninitialized,
        Falsey(Value),
        Current(Value),
    }

    let eval = args
        .into_iter()
        .fold(Ok(AndResult::Uninitialized), |last_res, current| {
            let last_eval = last_res?;

            if let AndResult::Falsey(_) = last_eval {
                return Ok(last_eval);
            }

            let parsed = Parsed::from_value(current)?;
            let evaluated = parsed.evaluate(data)?;

            if !truthy_from_evaluated(&evaluated) {
                return Ok(AndResult::Falsey(evaluated.into()));
            }

            Ok(AndResult::Current(evaluated.into()))
        })?;

    match eval {
        AndResult::Falsey(v) => Ok(v),
        AndResult::Current(v) => Ok(v),
        _ => Err(Error::UnexpectedError(
            "And operation had no values to operate on".into(),
        )),
    }
}

/// Do < for either 2 or 3 values
fn op_lt(items: &Vec<&Value>) -> Result<Value, Error> {
    if items.len() == 2 {
        return Ok(Value::Bool(js_op::abstract_lt(items[0], items[1])));
    } else {
        return Ok(Value::Bool(
            js_op::abstract_lt(items[0], items[1]) && js_op::abstract_lt(items[1], items[2]),
        ));
    }
}

/// Do <= for either 2 or 3 values
fn op_lte(items: &Vec<&Value>) -> Result<Value, Error> {
    if items.len() == 2 {
        return Ok(Value::Bool(js_op::abstract_lte(items[0], items[1])));
    } else {
        return Ok(Value::Bool(
            js_op::abstract_lte(items[0], items[1]) && js_op::abstract_lte(items[1], items[2]),
        ));
    }
}

/// An operation that doesn't do any recursive parsing or evaluation.
///
/// Any operator functions used must handle parsing of values themselves.
#[derive(Debug)]
pub struct LazyOperation<'a> {
    operator: &'a LazyOperator,
    arguments: Vec<Value>,
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

        // If args value is not an array, and the operator is unary,
        // the value is treated as a unary argument array.
        let args = match val {
            Value::Array(args) => args.to_vec(),
            _ => match op.num_params {
                NumParams::Unary => vec![val.clone()],
                _ => {
                    return Err(Error::InvalidOperation {
                        key: key.clone(),
                        reason: "Arguments to non-unary operations must be arrays".into()
                    })
                }
            },
        };

        op.num_params.check_len(&args.len())?;

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

        // If args value is not an array, and the operator is unary,
        // the value is treated as a unary argument array.
        let args = match val {
            Value::Array(args) => args.iter().collect::<Vec<&Value>>(),
            _ => match op.num_params {
                    NumParams::Unary => vec![val],
                    _ => {
                        return Err(Error::InvalidOperation {
                        key: key.into(),
                        reason: "Arguments for non-unary operator must be arrays".into(),
                    })
                }
            }

        };

        op.num_params.check_len(&args.len())?;

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
            .map(|value| value.evaluate(data).map(Value::from))
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

fn truthy_from_evaluated(evaluated: &Evaluated) -> bool {
    match evaluated {
        Evaluated::New(ref v) => truthy(v),
        Evaluated::Raw(v) => truthy(v),
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
pub fn truthy(val: &Value) -> bool {
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
    fn test_operator_map_symbols() {
        OPERATOR_MAP
            .into_iter()
            .for_each(|(k, op)| assert_eq!(*k, op.symbol))
    }

    /// All lazy operators symbols must match their keys
    #[test]
    fn test_lazy_operator_map_symbols() {
        LAZY_OPERATOR_MAP
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

        trues.iter().for_each(|v| assert!(truthy(&v)));
        falses.iter().for_each(|v| assert!(!truthy(&v)));
    }
}
