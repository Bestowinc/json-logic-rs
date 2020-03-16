use phf::phf_map;
use serde_json::{Map, Number, Value};
use std::convert::{From, TryFrom};
use std::fmt;
use thiserror;

pub mod js_op;

pub use js_op::{
    abstract_eq, abstract_gt, abstract_gte, abstract_lt, abstract_lte, abstract_ne, strict_eq,
    strict_ne,
};

const NULL: Value = Value::Null;

/// Public error enumeration
#[derive(thiserror::Error, Debug, PartialEq)]
pub enum Error {
    #[error("Invalid data - value: {value:?}, reason: {reason:?}")]
    InvalidData { value: Value, reason: String },
    #[error("Invalid rule - operator: '{key:?}', reason: {reason:?}")]
    InvalidOperation { key: String, reason: &'static str },
    #[error("Invalid variable - '{value:?}', reason: {reason:?}")]
    InvalidVariable { value: Value, reason: String },
    #[error("Invalid variable mapping - {0} is not an object.")]
    InvalidVarMap(Value),
    #[error("Encountered an unexpected error. Please raise an issue on GitHub and include the following error message: {0}")]
    UnexpectedError(String),
    #[error("Wrong argument count - expected: {expected:?}, actual: {actual:?}")]
    WrongArgumentCount { expected: usize, actual: usize },
}

struct Operator {
    symbol: &'static str,
    operator: OperatorFn<'static>,
}
impl Operator {
    fn execute<'a>(&self, items: &Vec<EvaluatedValue>) -> Result<EvaluatedValue<'a>, Error> {
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

type OperatorFn<'a> = fn(&Vec<EvaluatedValue>) -> Result<EvaluatedValue<'a>, Error>;

/// Operator for JS-style abstract equality
fn op_abstract_eq<'a>(items: &Vec<EvaluatedValue>) -> Result<EvaluatedValue<'a>, Error> {
    let to_res = |first: &Value, second: &Value| -> Result<EvaluatedValue<'a>, Error> {
        Ok(EvaluatedValue::New(Value::Bool(abstract_eq(
            &first, &second,
        ))))
    };

    match items[..] {
        [EvaluatedValue::Raw(first), EvaluatedValue::Raw(second)] => to_res(first, second),
        [EvaluatedValue::New(ref first), EvaluatedValue::Raw(second)] => to_res(first, second),
        [EvaluatedValue::Raw(first), EvaluatedValue::New(ref second)] => to_res(first, second),
        [EvaluatedValue::New(ref first), EvaluatedValue::New(ref second)] => to_res(first, second),
        _ => Err(Error::WrongArgumentCount {
            expected: 2,
            actual: items.len(),
        }),
    }
}

static OPERATOR_MAP: phf::Map<&'static str, Operator> = phf_map! {
    "==" => Operator {symbol: "==", operator: op_abstract_eq}
};

/// A Parsed JSON value
///
/// Parsed values are one of:
///   - A rule: a valid JSONLogic rule which can be evaluated
///   - A raw value: a non-rule, raw JSON value
#[derive(Debug)]
enum ParsedValue<'a> {
    Operation(Operation<'a>),
    Raw(&'a Value),
    Variable(Variable<'a>),
}
impl<'a> ParsedValue<'a> {
    /// Recursively parse a value
    fn from_value(value: &'a Value) -> Result<Self, Error> {
        Ok(
            Variable::from_value(value)?
            .map(Self::Variable)
            .or(Operation::from_value(value)?.map(Self::Operation))
            .unwrap_or(Self::Raw(value))
        )
    }

    fn from_values(values: &'a Vec<Value>) -> Result<Vec<Self>, Error> {
        values
            .iter()
            .map(Self::from_value)
            .collect::<Result<Vec<Self>, Error>>()
    }

    fn evaluate(&self, data: &'a Value) -> Result<EvaluatedValue, Error> {
        match self {
            Self::Operation(rule) => rule.evaluate(data),
            Self::Raw(val) => Ok(EvaluatedValue::Raw(*val)),
            Self::Variable(var) => var.interpolate(data).map(EvaluatedValue::Raw),
        }
    }
}

/// An Evaluated JSON value
///
/// An evaluated value is one of:
///   - A new value: either a calculated Rule or a filled Variable
///   - A raw value: a non-rule, raw JSON value
#[derive(Debug)]
enum EvaluatedValue<'a> {
    New(Value),
    Raw(&'a Value),
}

impl TryFrom<ParsedValue<'_>> for Value {
    type Error = Error;

    fn try_from(item: ParsedValue) -> Result<Self, Self::Error> {
        match item {
            ParsedValue::Operation(op) => Value::try_from(op),
            ParsedValue::Raw(val) => Ok(val.clone()),
            ParsedValue::Variable(var) => {
                let mut map = Map::with_capacity(1);
                map.insert("var".into(), var.value.clone());
                Ok(Value::Object(map))
            }
        }
    }
}

impl From<EvaluatedValue<'_>> for Value {
    fn from(item: EvaluatedValue) -> Self {
        match item {
            EvaluatedValue::Raw(val) => val.clone(),
            EvaluatedValue::New(val) => val,
        }
    }
}

impl TryFrom<Operation<'_>> for Value {
    type Error = Error;

    fn try_from(op: Operation) -> Result<Self, Self::Error> {
        let mut rv = Map::with_capacity(1);
        let values = op
            .arguments
            .into_iter()
            .map(Value::try_from)
            .collect::<Result<Vec<Self>, Self::Error>>()?;
        rv.insert(op.operator.symbol.into(), Value::Array(values));
        Ok(Value::Object(rv))
    }
}


#[derive(Debug)]
struct Variable<'a> {
    value: &'a Value,
}
impl<'a> Variable<'a> {
    fn from_value(value: &'a Value) -> Result<Option<Self>, Error> {
        match value {
            Value::Object(map) => {
                if map.len() != 1 {
                    return Ok(None);
                };
                match map.get("var") {
                    Some(var) => match var {
                        Value::String(_) => Ok(Some(Variable { value: var })),
                        Value::Number(_) => Ok(Some(Variable { value: var })),
                        Value::Array(arr) => match arr.len() {
                            0..=2 => Ok(Some(Variable { value: var })),
                            _ => Err(Error::InvalidVariable {
                                value: value.clone(),
                                reason: "Array variables must be of len 0..2 inclusive".into(),
                            }),
                        },
                        _ => Err(Error::InvalidVariable {
                            value: value.clone(),
                            reason: "Variables must be strings, integers, or arrays".into(),
                        }),
                    },
                    None => Ok(None),
                }
            }
            _ => Ok(None),
        }
    }

    fn interpolate(&self, data: &'a Value) -> Result<&'a Value, Error> {
        // if self.name == "" { return data };
        match self.value {
            Value::Null => Ok(data),
            Value::String(var_name) => self.interpolate_string_var(data, var_name),
            Value::Number(idx) => self.interpolate_numeric_var(data, idx),
            Value::Array(var) => self.interpolate_array_var(data, var),
            _ => Err(Error::InvalidVariable{
                value: self.value.clone(),
                reason: "Unsupported variable type. Variables must be strings, integers, arrays, or null.".into()
            })
        }
    }

    fn get_default(&self) -> &'a Value {
        match self.value {
            Value::Array(val) => val.get(1).unwrap_or(&NULL),
            _ => &NULL,
        }
    }

    fn interpolate_array_var(
        &self,
        data: &'a Value,
        var: &'a Vec<Value>,
    ) -> Result<&'a Value, Error> {
        let len = var.len();
        match len {
            0 => Ok(data),
            1 | 2 => match &var[0] {
                Value::String(var_name) => self.interpolate_string_var(data, &var_name),
                Value::Number(var_idx) => self.interpolate_numeric_var(data, &var_idx),
                _ => Err(Error::InvalidVariable {
                    value: Value::Array(var.clone()),
                    reason: "Variables must be strings or integers".into(),
                }),
            },
            _ => Err(Error::InvalidVariable {
                value: Value::Array(var.clone()),
                reason: format!("Array variables must be of len 1 or 2, not {}", len),
            }),
        }
    }

    fn interpolate_numeric_var(
        &self,
        data: &'a Value,
        idx: &'a Number,
    ) -> Result<&'a Value, Error> {
        let default = self.get_default();
        match data {
            Value::Array(val) => {
                idx
                    // Option<u64>
                    .as_u64()
                    // Option<Result<usize, Error>>
                    .map(|i| {
                        usize::try_from(i).map_err(|e| Error::InvalidVariable {
                            value: Value::Number(idx.clone()),
                            reason: format!(
                                "Could not convert value to a system-sized integer: {:?}",
                                e
                            ),
                        })
                    })
                    // Option<Result<Value, Error>>
                    .map(|res| res.map(|i| val.get(i).unwrap_or(default)))
                    // Result<Value, Error>
                    .unwrap_or(Ok(default))
            }
            _ => Err(Error::InvalidVariable {
                value: Value::Number(idx.clone()),
                reason: "Cannot access non-array data with an index variable".into(),
            }),
        }
    }

    fn interpolate_string_var(
        &self,
        data: &'a Value,
        var_name: &'a String,
    ) -> Result<&'a Value, Error> {
        if var_name == "" {
            return Ok(data);
        };
        let default = self.get_default();
        match data {
            Value::Object(_) => var_name.split(".").fold(
                Ok(data), |acc, i| match acc? {
                    // If the current value is an object, try to get the value
                    Value::Object(map) => {
                        // Default to null if not found
                        Ok(map.get(i).unwrap_or(default))
                    },
                    // If the current value is an array, we need an integer
                    // index. If integer conversion fails, return an error.
                    Value::Array(arr) => {
                        i.parse::<usize>().map(
                            |i| arr.get(i).unwrap_or(default)
                        ).map_err(|_| Error::InvalidVariable{
                            value: Value::String(var_name.clone()),
                            reason: "Cannot access array data with non-integer key".into()
                        })
                    },
                    // If the object is any other type, just return the default
                    _ => Ok(default),
                }
            ),
            // A string key is invalid for anything other than an object
            _ => Err(Error::InvalidVariable{
                value: Value::String(var_name.clone()),
                reason: "String indices are not supported for non-object data".into()
            }),
        }
    }
}


#[derive(Debug)]
struct Operation<'a> {
    operator: &'a Operator,
    arguments: Vec<ParsedValue<'a>>,
}
impl<'a> Operation<'a> {
    fn from_value(value: &'a Value) -> Result<Option<Self>, Error> {
        match value {
            // Operations are Objects
            Value::Object(obj) => {
                // Operations have just one key/value pair.
                if obj.len() != 1 { return Ok(None) }

                let key = obj.keys().next().ok_or(Error::UnexpectedError(format!(
                    "could not get first key from len(1) object: {:?}",
                    obj
                )))?;
                let val = obj.get(key).ok_or(Error::UnexpectedError(format!(
                    "could not get value for key '{}' from len(1) object: {:?}",
                    key, obj
                )))?;

                // Operators have known keys
                if let Some(operator) = OPERATOR_MAP.get(key.as_str()) {
                    match val {
                        // Operator values are arrays
                        Value::Array(arguments) => Ok(Some(Operation {
                            operator,
                            arguments: ParsedValue::from_values(arguments)?,
                        })),
                        _ => Err(Error::InvalidOperation {
                            key: key.into(),
                            reason: "Values for operator keys must be arrays",
                        }),
                    }
                } else {
                    Ok(None)
                }
            },
            // We're definitely not an operation
            _ => Ok(None)
        }
    }
    /// Evaluate the operation after recursively evaluating any nested operations
    fn evaluate(&self, data: &Value) -> Result<EvaluatedValue, Error> {
        let arguments = self
            .arguments
            .iter()
            .map(|value| value.evaluate(data))
            .collect::<Result<Vec<EvaluatedValue>, Error>>()?;
        self.operator.execute(&arguments)
    }
}


/// Run JSONLogic for the given operation and data.
///
pub fn jsonlogic(value: &Value, data: &Value) -> Result<Value, Error> {
    let parsed = ParsedValue::from_value(&value)?;
    parsed.evaluate(data).map(|res| res.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn jsonlogic_cases() -> Vec<(Value, Value, Result<Value, ()>)> {
        vec![
            // Passing a static value returns the value unchanged.
            (json!("foo"), json!({}), Ok(json!("foo"))),
            (json!(""), json!({}), Ok(json!(""))),
            (json!([1, 2]), json!({}), Ok(json!([1, 2]))),
            (json!([]), json!({}), Ok(json!([]))),
            (json!(null), json!({}), Ok(json!(null))),
            (json!(0), json!({}), Ok(json!(0))),
            (json!(234), json!({}), Ok(json!(234))),
            (json!({}), json!({}), Ok(json!({}))),
            // Note: as of this writing, this behavior differs from the
            // original jsonlogic implementation, which errors for objects of
            // length one, due to attempting to parse their key as an operation
            (json!({"a": 1}), json!({}), Ok(json!({"a": 1}))),
            (
                json!({"a": 1, "b": 2}),
                json!({}),
                Ok(json!({"a": 1, "b": 2})),
            ),
            // Operators - "=="
            (json!({"==": [1, 1]}), json!({}), Ok(json!(true))),
            (json!({"==": [1, 2]}), json!({}), Ok(json!(false))),
            (json!({"==": [1, "1"]}), json!({}), Ok(json!(true))),
            (json!({"==": [1, [1]]}), json!({}), Ok(json!(true))),
            (json!({"==": [1, true]}), json!({}), Ok(json!(true))),
            // Recursive evaluation
            (
                json!({"==": [true, {"==": [1, 1]}]}),
                json!({}),
                Ok(json!(true)),
            ),
            (
                json!({"==": [{"==": [{"==": [1, 1]}, true]}, {"==": [1, 1]}]}),
                json!({}),
                Ok(json!(true)),
            ),
            // Variable substitution
            (
                json!({"var": "foo"}),
                json!({"foo": "bar"}),
                Ok(json!("bar")),
            ),
            // Index into array data
            (
                json!({"var": 1}),
                json!(["foo", "bar"]),
                Ok(json!("bar"))
            ),
            // Absent variable
            (json!({"var": "foo"}), json!({}), Ok(json!(null))),
            (
                json!({"==": [{"var": "first"}, true]}),
                json!({"first": true}),
                Ok(json!(true)),
            ),
            // Dotted variable substitution
            (
                json!({"var": "foo.bar"}),
                json!({"foo": {"bar": "baz"}}),
                Ok(json!("baz")),
            ),
            // Dotted variable with nested array access
            (
                json!({"var": "foo.1"}),
                json!({"foo": ["bar", "baz", "pop"]}),
                Ok(json!("baz")),
            ),
            // Absent dotted variable
            (
                json!({"var": "foo.bar"}),
                json!({"foo": {"baz": "baz"}}),
                Ok(json!(null)),
            ),
            // Non-object type in dotted variable path
            (
                json!({"var": "foo.bar.baz"}),
                json!({"foo": {"bar": 1}}),
                Ok(json!(null)),
            ),
            (
                json!({"var": "foo.bar"}),
                json!({"foo": "not an object"}),
                Ok(json!(null)),
            ),
        ]
    }

    #[test]
    fn test_jsonlogic() {
        jsonlogic_cases().into_iter().for_each(|(op, data, exp)| {
            println!("Running rule: {:?} with data: {:?}", op, data);
            let result = jsonlogic(&op, &data);
            println!("- Result: {:?}", result);
            println!("- Expected: {:?}", exp);
            if exp.is_ok() {
                assert_eq!(result.unwrap(), exp.unwrap());
            } else {
                result.unwrap_err();
            }
        })
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
///
///
/// ```rust
/// use serde_json::json;
/// use jsonlogic::truthy;
///
/// let trues = [
///     json!(true), json!([1]), json!([1,2]), json!({}), json!({"a": 1}),
///     json!(1), json!(-1), json!("foo")
/// ];
///
/// let falses = [
///     json!(false), json!([]), json!(""), json!(0), json!(null)
/// ];
///
/// trues.iter().for_each(|v| assert!(truthy(&v)));
/// falses.iter().for_each(|v| assert!(!truthy(&v)));
/// ```
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
