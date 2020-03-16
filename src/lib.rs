use phf::phf_map;
use serde_json::map::Map;
use serde_json::Value;
use std::convert::{From, TryFrom};
// use std::eq
use std::collections::HashMap;
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
    #[error("Invalid rule! operator: '{key:?}', reason: {reason:?}")]
    InvalidRule { key: String, reason: &'static str },
    #[error("Invalid variable: '{value:?}', reason: {reason:?}")]
    InvalidVariable { value: Value, reason: &'static str },
    #[error("Invalid variable mapping! {0} is not an object.")]
    InvalidVarMap(Value),
    #[error("Encountered an unexpected error. Please raise an issue on GitHub and include the following error message: {0}")]
    UnexpectedError(String),
    #[error("Wrong argument count. Expected {expected:?}, got {actual:?}")]
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
    Rule(Rule<'a>),
    Raw(&'a Value),
    Variable(Variable<'a>),
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
            ParsedValue::Rule(rule) => Value::try_from(rule),
            ParsedValue::Raw(val) => Ok(val.clone()),
            ParsedValue::Variable(var) => {
                let mut map = Map::with_capacity(1);
                map.insert("var".into(), Value::String(var.name.clone()));
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

impl TryFrom<Rule<'_>> for Value {
    type Error = Error;

    fn try_from(rule: Rule) -> Result<Self, Self::Error> {
        let mut rv = Map::with_capacity(1);
        let values = rule
            .arguments
            .into_iter()
            .map(Value::try_from)
            .collect::<Result<Vec<Self>, Self::Error>>()?;
        rv.insert(rule.operator.symbol.into(), Value::Array(values));
        Ok(Value::Object(rv))
    }
}

pub trait VarMap {
    fn get<S: AsRef<str>>(&self, key: S) -> Result<Option<&Value>, Error>;
}

impl VarMap for HashMap<String, Value> {
    fn get<S: AsRef<str>>(&self, key: S) -> Result<Option<&Value>, Error> {
        Ok(HashMap::get(&self, key.as_ref()))
    }
}

impl VarMap for Map<String, Value> {
    fn get<S: AsRef<str>>(&self, key: S) -> Result<Option<&Value>, Error> {
        Ok(Map::get(&self, key.as_ref()))
    }
}

impl VarMap for Value {
    fn get<S: AsRef<str>>(&self, key: S) -> Result<Option<&Value>, Error> {
        match self {
            Value::Object(map) => Ok(map.get(key.as_ref())),
            _ => Err(Error::InvalidVarMap(self.clone())),
        }
    }
}

#[derive(Debug)]
struct Variable<'a> {
    name: &'a String,
}

#[derive(Debug)]
struct Rule<'a> {
    operator: &'a Operator,
    arguments: Vec<ParsedValue<'a>>,
}
impl<'a> Rule<'a> {
    /// Evaluate the rule after recursively evaluating any nested rules
    fn evaluate<D: VarMap>(&self, data: &'a D) -> Result<EvaluatedValue, Error> {
        let arguments = self
            .arguments
            .iter()
            .map(|value| match value {
                ParsedValue::Rule(rule) => rule.evaluate(data),
                ParsedValue::Raw(val) => Ok(EvaluatedValue::Raw(*val)),
                ParsedValue::Variable(var) => {
                    Ok(EvaluatedValue::Raw(interpolate_variable(data, &var)))
                }
            })
            .collect::<Result<Vec<EvaluatedValue>, Error>>()?;
        self.operator.execute(&arguments)
    }
}

fn parse_args<'a>(arguments: &'a Vec<Value>) -> Result<Vec<ParsedValue<'a>>, Error> {
    arguments
        .iter()
        .map(parse)
        .collect::<Result<Vec<ParsedValue>, Error>>()
}

fn interpolate_variable<'a, D: VarMap>(
    data: &'a D,
    variable: &Variable,
) -> &'a Value {
    variable.name.split(".").fold(
        &NULL,
        |acc, i| {
            match acc {
                Value::Null => data.get(i).ok().unwrap_or(None).unwrap_or(&NULL),
                Value::Object(map) => map.get(i).unwrap_or(&NULL),
                _ => &NULL
            }
        }
    )
}

/// Recursively parse a value into a series of Items.
fn parse<'a>(value: &'a Value) -> Result<ParsedValue<'a>, Error> {
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
                    if key == "var" {
                        match val {
                            Value::String(var_name) => {
                                Ok(ParsedValue::Variable(Variable { name: var_name }))
                            }
                            _ => Err(Error::InvalidVariable {
                                value: val.clone(),
                                reason: "Variable values must be strings.",
                            }),
                        }
                    } else if let Some(operator) = OPERATOR_MAP.get(key.as_str()) {
                        match val {
                            // But only if the value is an array
                            Value::Array(arguments) => Ok(ParsedValue::Rule(Rule {
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
                        Ok(ParsedValue::Raw(&value))
                    }
                }
                // If the object has < or > 1 key, it's not a rule
                // _ => Ok(Item::Raw(Value::Object(obj))),
                _ => Ok(ParsedValue::Raw(&value)),
            }
        }
        // If the item is not an object, it's not a rule
        _ => Ok(ParsedValue::Raw(&value)),
    }
}

/// Run JSONLogic for the given rule and data.
///
pub fn jsonlogic<'a, D: VarMap>(value: &'a Value, data: D) -> Result<Value, Error> {
    let parsed = parse(&value)?;
    match parsed {
        ParsedValue::Rule(rule) => {
            let res = rule.evaluate(&data)?;
            Ok(res.into())
        }
        ParsedValue::Raw(val) => Ok(val.clone()),
        ParsedValue::Variable(var) => Ok(interpolate_variable(&data, &var).clone()),
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
            // Absent variable
            (
                json!({"var": "foo"}),
                json!({}),
                Ok(json!(null)),
            ),
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
