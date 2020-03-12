use phf::phf_map;
use serde_json::map::Map;
use serde_json::Value;
use thiserror;

pub mod js_op;

pub use js_op::{
    abstract_eq, abstract_gt, abstract_gte, abstract_lt, abstract_lte, abstract_ne, strict_eq,
    strict_ne,
};

/// Public error enumeration
#[derive(thiserror::Error, Debug, PartialEq)]
pub enum Error {
    #[error("Invalid rule! operator: '{key:?}', reason: {reason:?}")]
    InvalidRule { key: String, reason: &'static str },
    #[error("Encountered an unexpected error. Please raise an issue on GitHub and include the following error message: {0}")]
    UnexpectedError(String),
    #[error("Wrong argument count. Expected {expected:?}, got {actual:?}")]
    WrongArgumentCount { expected: usize, actual: usize },
}

type OperatorFn = fn(&Vec<Item>, &Map<String, Value>) -> Result<Item, Error>;

/// Operator for JS-style abstract equality
fn op_abstract_eq(items: &Vec<Item>, data: &Map<String, Value>) -> Result<Item, Error> {
    match &items[..] {
        [Item::Raw(first), Item::Raw(second)] => {
            let raw_value = Value::Bool(abstract_eq(&first, &second));
            Ok(Item::Raw(raw_value))
        }
        _ => Err(Error::WrongArgumentCount {
            expected: 2,
            actual: items.len(),
        }),
    }
}

static OPERATOR_MAP: phf::Map<&'static str, OperatorFn> = phf_map! {
    "==" => op_abstract_eq
};

enum Item {
    Rule(Rule),
    Raw(Value),
}

struct Rule {
    operator: &'static OperatorFn,
    arguments: Vec<Item>,
}

fn parse_args(arguments: Vec<Value>) -> Result<Vec<Item>, Error> {
    arguments
        .into_iter()
        .map(parse_value)
        .collect::<Result<Vec<Item>, Error>>()
}

/// Recursively parse a value into a series of Items.
fn parse_value(value: Value) -> Result<Item, Error> {
    match value {
        Value::Object(obj) => {
            match obj.len() {
                1 => {
                    let key = obj.keys().next().ok_or(Error::UnexpectedError(format!(
                        "could not get first key from len(1) object: {:?}",
                        obj
                    )))?;
                    let val = obj.get(key).ok_or(Error::UnexpectedError(format!(
                        "could not get value for key '{}' from len(1) object: {:?}",
                        key, obj
                    )))?;
                    if let Some(operator) = OPERATOR_MAP.get(key.as_str()) {
                        match val {
                            // But only if the value is an array
                            Value::Array(arguments) => Ok(Item::Rule(Rule {
                                operator,
                                arguments: parse_args(arguments.to_vec())?,
                            })),
                            _ => Err(Error::InvalidRule {
                                key: key.into(),
                                reason: "Values for operator keys must be arrays",
                            }),
                        }
                    } else {
                        // If the item's single key is not an operator, it's not a rule
                        Ok(Item::Raw(Value::Object(obj)))
                    }
                }
                // If the object has < or > 1 key, it's not a rule
                _ => Ok(Item::Raw(Value::Object(obj))),
                // _ => Ok(Item::Raw(value)),
            }
        }
        // If the item is not an object, it's not a rule
        _ => Ok(Item::Raw(value)),
    }
}

/// Run JSONLogic for the given rule and data.
///
pub fn jsonlogic(value: Value, data: Value) -> Result<Value, Error> {
    let parsed = parse_value(value)?;
    match parsed {
        Item::Rule(rule) => {
            let operator = rule.operator;
            match operator(&rule.arguments, &Map::new())? {
                Item::Raw(item) => Ok(item),
                _ => Err(Error::UnexpectedError(
                    "Got a rule as a final result.".into(),
                )),
            }
        }
        Item::Raw(val) => Ok(val),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn jsonlogic_cases() -> Vec<(Value, Value, Result<Value, Error>)> {
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
        ]
    }

    #[test]
    fn test_jsonlogic() {
        jsonlogic_cases().into_iter().for_each(|(rule, data, exp)| {
            println!("Running rule: {:?} with data: {:?}", rule, data);
            let result = jsonlogic(rule, data);
            println!("- Result: {:?}", result);
            println!("- Expected: {:?}", exp);
            if exp.is_ok() {
                assert_eq!(result.unwrap(), exp.unwrap());
            } else {
                assert_eq!(result.unwrap_err(), exp.unwrap_err());
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
