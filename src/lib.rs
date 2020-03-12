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

type OperatorFn<'a> = fn(&Vec<Item>, &Map<String, Value>) -> Result<Item<'a>, Error>;

/// Operator for JS-style abstract equality
fn op_abstract_eq<'a>(items: &Vec<Item>, data: &Map<String, Value>) -> Result<Item<'a>, Error> {

    let to_res = |first: &Value, second: &Value| -> Result<Item<'a>, Error> {
        Ok(Item::Evaluated(Value::Bool(abstract_eq(&first, &second))))
    };

    match items[..] {
        [Item::Raw(first), Item::Raw(second)] => to_res(first, second),
        [Item::Evaluated(ref first), Item::Raw(second)] => to_res(first, second),
        [Item::Raw(first), Item::Evaluated(ref second)] => to_res(first, second),
        [Item::Evaluated(ref first), Item::Evaluated(ref second)] => to_res(first, second),
        _ => Err(Error::WrongArgumentCount {
            expected: 2,
            actual: items.len(),
        }),
    }
}

static OPERATOR_MAP: phf::Map<&'static str, OperatorFn> = phf_map! {
    "==" => op_abstract_eq
};

/// An atomic unit of the JSONLogic expression.
/// An item is one of:
///   - A rule: a parsed, valid, JSONLogic rule, which can be evaluated
///   - A raw value: a reference to a pre-existing JSON value
///   - An evaluated value: an owned, non-rule, JSON value generated as a
///     result of rule evaluation
enum Item<'a> {
    Rule(Rule<'a>),
    // Keeping references to pre-existing JSON values significantly reduces
    // the amount of cloning we need to do. For anything that isn't a rule
    // or generated via rule evaluation, we can just pass references around
    // for the entire lifetime of rule evaluation.
    Raw(&'a Value),
    Evaluated(Value),
}

struct Rule<'a> {
    operator: &'a OperatorFn<'a>,
    arguments: Vec<Item<'a>>,
}
impl<'a> Rule<'a> {
    /// Evaluate the rule after recursively evaluating any nested rules
    fn evaluate(&self, data: &Map<String, Value>) -> Result<Item, Error> {
        let arguments = self
            .arguments
            .iter()
            .map(|item| match item {
                Item::Rule(rule) => rule.evaluate(data),
                Item::Raw(val) => Ok(Item::Raw(*val)),
                Item::Evaluated(val) => Ok(Item::Raw(val)),
            })
            .collect::<Result<Vec<Item>, Error>>()?;
        (self.operator)(&arguments, data)
    }
}

fn parse_args<'a>(arguments: &'a Vec<Value>) -> Result<Vec<Item<'a>>, Error> {
    arguments
        .iter()
        .map(parse_value)
        .collect::<Result<Vec<Item>, Error>>()
}

/// Recursively parse a value into a series of Items.
fn parse_value<'a>(value: &'a Value) -> Result<Item<'a>, Error> {
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
                                arguments: parse_args(arguments)?,
                            })),
                            _ => Err(Error::InvalidRule {
                                key: key.into(),
                                reason: "Values for operator keys must be arrays",
                            }),
                        }
                    } else {
                        // If the item's single key is not an operator, it's not a rule
                        // Ok(Item::Raw(&alue::Object(obj)))
                        Ok(Item::Raw(&value))
                    }
                }
                // If the object has < or > 1 key, it's not a rule
                // _ => Ok(Item::Raw(Value::Object(obj))),
                _ => Ok(Item::Raw(&value)),
            }
        }
        // If the item is not an object, it's not a rule
        _ => Ok(Item::Raw(&value)),
    }
}

/// Run JSONLogic for the given rule and data.
///
pub fn jsonlogic<'a>(value: &'a Value, data: Value) -> Result<Value, Error> {
    let parsed = parse_value(&value)?;
    match parsed {
        Item::Rule(rule) => match data {
            Value::Object(data) => {
                let res = rule.evaluate(&data)?;
                match res {
                    Item::Raw(res) => Ok(res.clone()),
                    Item::Evaluated(res) => Ok(res),
                    Item::Rule(_) => Err(Error::UnexpectedError("Evaluated to rule".into())),
                }
            }
            _ => return Err(Error::UnexpectedError("Bad data".into())),
        },
        Item::Raw(val) => Ok(val.clone()),
        Item::Evaluated(_) => Err(Error::UnexpectedError(
            "Parser should not evaluate items".into(),
        )),
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
            // Recursive evaluation
            (json!({"==": [true, {"==": [1, 1]}]}), json!({}), Ok(json!(true))),
        ]
    }

    #[test]
    fn test_jsonlogic() {
        jsonlogic_cases().into_iter().for_each(|(rule, data, exp)| {
            println!("Running rule: {:?} with data: {:?}", rule, data);
            let result = jsonlogic(&rule, data);
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
