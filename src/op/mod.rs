//! Operators
//!
//! This module contains the global operator map, which defines the available
//! JsonLogic operations. Note that some "operations", notably data-related
//! operations like "var" and "missing", are not included here, because they are
//! implemented as parsers rather than operators.

// TODO: it's possible that "missing", "var", et al. could be implemented
// as operators. They were originally done differently because there wasn't
// yet a LazyOperator concept.

use phf::phf_map;
use serde_json::{Map, Value};
use std::fmt;

use crate::error::Error;
use crate::value::to_number_value;
use crate::value::{Evaluated, Parsed};
use crate::{js_op, Parser};

mod array;
mod data;
mod impure;
mod logic;
mod numeric;
mod string;

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
        operator: |items| Ok(Value::Bool(!logic::truthy(items[0]))),
        num_params: NumParams::Unary,
    },
    "!!" => Operator {
        symbol: "!!",
        operator: |items| Ok(Value::Bool(logic::truthy(items[0]))),
        num_params: NumParams::Unary,
    },
    "<" => Operator {
        symbol: "<",
        operator: numeric::lt,
        num_params: NumParams::Variadic(2..4),
    },
    "<=" => Operator {
        symbol: "<=",
        operator: numeric::lte,
        num_params: NumParams::Variadic(2..4),
    },
    // Note: this is actually an _expansion_ on the specification and the
    // reference implementation. The spec states that < and <= can be used
    // for 2-3 arguments, with 3 arguments doing a "between" style test,
    // e.g. `1 < 2 < 3 == true`. However, this isn't explicitly supported
    // for > and >=, and the reference implementation simply ignores any
    // third value for these operators. This to me violates the principle
    // of least surprise, so we do support those operations.
    ">" => Operator {
        symbol: ">",
        operator: numeric::gt,
        num_params: NumParams::Variadic(2..4),
    },
    ">=" => Operator {
        symbol: ">=",
        operator: numeric::gte,
        num_params: NumParams::Variadic(2..4),
    },
    "+" => Operator {
        symbol: "+",
        operator: |items| js_op::parse_float_add(items).and_then(to_number_value),
        num_params: NumParams::Any,
    },
    "-" => Operator {
        symbol: "-",
        operator: numeric::minus,
        num_params: NumParams::Variadic(1..3),
    },
    "*" => Operator {
        symbol: "*",
        operator: |items| js_op::parse_float_mul(items).and_then(to_number_value),
        num_params: NumParams::AtLeast(1),
    },
    "/" => Operator {
        symbol: "/",
        operator: |items| js_op::abstract_div(items[0], items[1])
            .and_then(to_number_value),
        num_params: NumParams::Exactly(2),
    },
    "%" => Operator {
        symbol: "%",
        operator: |items| js_op::abstract_mod(items[0], items[1])
            .and_then(to_number_value),
        num_params: NumParams::Exactly(2),
    },
    "max" => Operator {
        symbol: "max",
        operator: |items| js_op::abstract_max(items)
            .and_then(to_number_value),
        num_params: NumParams::AtLeast(1),
    },
    "min" => Operator {
        symbol: "min",
        operator: |items| js_op::abstract_min(items)
            .and_then(to_number_value),
        num_params: NumParams::AtLeast(1),
    },
    "merge" => Operator {
        symbol: "merge",
        operator: array::merge,
        num_params: NumParams::Any,
    },
    "in" => Operator {
        symbol: "in",
        operator: array::in_,
        num_params: NumParams::Exactly(2),
    },
    "cat" => Operator {
        symbol: "cat",
        operator: string::cat,
        num_params: NumParams::Any,
    },
    "substr" => Operator {
        symbol: "substr",
        operator: string::substr,
        num_params: NumParams::Variadic(2..4),
    },
    "log" => Operator {
        symbol: "log",
        operator: impure::log,
        num_params: NumParams::Unary,
    },
};

pub const DATA_OPERATOR_MAP: phf::Map<&'static str, DataOperator> = phf_map! {
    "var" => DataOperator {
        symbol: "var",
        operator: data::var,
        num_params: NumParams::Variadic(0..3)
    },
    "missing" => DataOperator {
        symbol: "missing",
        operator: data::missing,
        num_params: NumParams::Any,
    },
    "missing_some" => DataOperator {
        symbol: "missing_some",
        operator: data::missing_some,
        num_params: NumParams::Exactly(2),
    },
};

pub const LAZY_OPERATOR_MAP: phf::Map<&'static str, LazyOperator> = phf_map! {
    // Logical operators
    "if" => LazyOperator {
        symbol: "if",
        operator: logic::if_,
        num_params: NumParams::Any,
    },
    // Note this operator isn't defined in the specification, but is
    // present in the tests as what looks like an alias for "if".
    "?:" => LazyOperator {
        symbol: "?:",
        operator: logic::if_,
        num_params: NumParams::Any,
    },
    "or" => LazyOperator {
        symbol: "or",
        operator: logic::or,
        num_params: NumParams::AtLeast(1),
    },
    "and" => LazyOperator {
        symbol: "and",
        operator: logic::and,
        num_params: NumParams::AtLeast(1),
    },
    "map" => LazyOperator {
        symbol: "map",
        operator: array::map,
        num_params: NumParams::Exactly(2),
    },
    "filter" => LazyOperator {
        symbol: "filter",
        operator: array::filter,
        num_params: NumParams::Exactly(2),
    },
    "reduce" => LazyOperator {
        symbol: "reduce",
        operator: array::reduce,
        num_params: NumParams::Exactly(3),
    },
    "all" => LazyOperator {
        symbol: "all",
        operator: array::all,
        num_params: NumParams::Exactly(2),
    },
    "some" => LazyOperator {
        symbol: "some",
        operator: array::some,
        num_params: NumParams::Exactly(2),
    },
    "none" => LazyOperator {
        symbol: "none",
        operator: array::none,
        num_params: NumParams::Exactly(2),
    },
};

#[derive(Debug, Clone)]
pub enum NumParams {
    None,
    Any,
    Unary,
    Exactly(usize),
    AtLeast(usize),
    Variadic(std::ops::Range<usize>), // [inclusive, exclusive)
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
            false => Err(Error::WrongArgumentCount {
                expected: self.clone(),
                actual: len.clone(),
            }),
        }
    }
    fn can_accept_unary(&self) -> bool {
        match self {
            Self::None => false,
            Self::Any => true,
            Self::Unary => true,
            Self::AtLeast(num) => num >= &1,
            Self::Exactly(num) => num == &1,
            Self::Variadic(range) => range.contains(&1),
        }
    }
}

trait CommonOperator {
    fn param_info(&self) -> &NumParams;
}

pub struct Operator {
    symbol: &'static str,
    operator: OperatorFn,
    num_params: NumParams,
}
impl Operator {
    pub fn execute(&self, items: &Vec<&Value>) -> Result<Value, Error> {
        (self.operator)(items)
    }
}
impl CommonOperator for Operator {
    fn param_info(&self) -> &NumParams {
        &self.num_params
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
impl CommonOperator for LazyOperator {
    fn param_info(&self) -> &NumParams {
        &self.num_params
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

/// An operator that operates on passed in data.
///
/// Data operators' arguments can be lazily evaluated, but unlike
/// regular operators, they still need access to data even after the
/// evaluation of their arguments.
pub struct DataOperator {
    symbol: &'static str,
    operator: DataOperatorFn,
    num_params: NumParams,
}
impl DataOperator {
    pub fn execute(&self, data: &Value, items: &Vec<&Value>) -> Result<Value, Error> {
        (self.operator)(data, items)
    }
}
impl CommonOperator for DataOperator {
    fn param_info(&self) -> &NumParams {
        &self.num_params
    }
}
impl fmt::Debug for DataOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Operator")
            .field("symbol", &self.symbol)
            .field("operator", &"<operator fn>")
            .finish()
    }
}

type OperatorFn = fn(&Vec<&Value>) -> Result<Value, Error>;
type LazyOperatorFn = fn(&Value, &Vec<&Value>) -> Result<Value, Error>;
type DataOperatorFn = fn(&Value, &Vec<&Value>) -> Result<Value, Error>;

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
        op_from_map(&LAZY_OPERATOR_MAP, value).and_then(|opt| {
            opt.map(|op| {
                Ok(LazyOperation {
                    operator: op.op,
                    arguments: op.args.into_iter().map(|v| v.clone()).collect(),
                })
            })
            .transpose()
        })
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
        op_from_map(&OPERATOR_MAP, value).and_then(|opt| {
            opt.map(|op| {
                Ok(Operation {
                    operator: op.op,
                    arguments: Parsed::from_values(op.args)?,
                })
            })
            .transpose()
        })
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

#[derive(Debug)]
pub struct DataOperation<'a> {
    operator: &'a DataOperator,
    arguments: Vec<Parsed<'a>>,
}
impl<'a> Parser<'a> for DataOperation<'a> {
    fn from_value(value: &'a Value) -> Result<Option<Self>, Error> {
        op_from_map(&DATA_OPERATOR_MAP, value).and_then(|opt| {
            opt.map(|op| {
                Ok(DataOperation {
                    operator: op.op,
                    arguments: Parsed::from_values(op.args)?,
                })
            })
            .transpose()
        })
    }

    /// Evaluate the operation after recursively evaluating any nested operations
    fn evaluate(&self, data: &'a Value) -> Result<Evaluated, Error> {
        let arguments = self
            .arguments
            .iter()
            .map(|value| value.evaluate(data).map(Value::from))
            .collect::<Result<Vec<Value>, Error>>()?;
        self.operator
            .execute(data, &arguments.iter().collect())
            .map(Evaluated::New)
    }
}
impl From<DataOperation<'_>> for Value {
    fn from(op: DataOperation) -> Value {
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

struct OpArgs<'a, 'b, T> {
    op: &'a T,
    args: Vec<&'b Value>,
}

fn op_from_map<'a, 'b, T: CommonOperator>(
    map: &'a phf::Map<&'static str, T>,
    value: &'b Value,
) -> Result<Option<OpArgs<'a, 'b, T>>, Error> {
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
    let op = match map.get(key.as_str()) {
        Some(op) => op,
        _ => return Ok(None),
    };

    let err_for_non_unary = || {
        Err(Error::InvalidOperation {
            key: key.clone(),
            reason: "Arguments to non-unary operations must be arrays".into(),
        })
    };

    let param_info = op.param_info();
    // If args value is not an array, and the operator is unary,
    // the value is treated as a unary argument array.
    let args = match val {
        Value::Array(args) => args.iter().collect::<Vec<&Value>>(),
        _ => match param_info.can_accept_unary() {
            true => vec![val],
            false => return err_for_non_unary(),
        },
    };

    param_info.check_len(&args.len())?;

    Ok(Some(OpArgs { op, args }))
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
