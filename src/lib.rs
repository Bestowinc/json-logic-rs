use serde_json::map::Map;
use serde_json::Value;
use thiserror;

pub mod js_op;

pub use js_op::{
    abstract_eq,
    abstract_gt,
    abstract_gte,
    abstract_lt,
    abstract_lte,
    abstract_ne,
    strict_eq,
    strict_ne,
};



// enum Operations {

// }


// fn if_(values: Vec<Value>, data: Map<String, Value>) -> Value {

// }


#[derive(thiserror::Error, Debug, PartialEq)]
pub enum Error {
    #[error("Wrong argument count. Expected {expected:?}, got {actual:?}")]
    WrongArgumentCount {
        expected: usize,
        actual: usize,
    }
}


fn is_logic(value: &Value) -> Result<&String, ()> {
    match value {
        Value::Object(obj) => {
            match obj.len() {
                1 => {
                    obj.iter().next().map(
                        |item| item.0
                    )
                    .ok_or(())
                    .and_then(
                        |key| {
                            if vec!["=="].contains(&key.as_str()) { Ok(key) } else { Err(()) }
                        }
                    )
                }
                _ => Err(())
            }
        },
        _ => Err(())
    }
}


fn op_abstract_eq(values: &Vec<Value>, data: &Map<String, Value>) -> Result<Value, Error> {
    match values.len() {
        2 => {
            Ok(Value::Bool(abstract_eq(&values[0], &values[1])))
        }
        _ => Err(Error::WrongArgumentCount{expected: 2, actual: values.len()})
    }
}


fn get_operator<Op>(operator: Op)
    -> impl Fn(&Vec<Value>, &Map<String, Value>) -> Result<Value, Error>
    where Op: AsRef<str>
{
    let op = operator.as_ref();
    match op {
        "==" => op_abstract_eq,
        _ => panic!("bad operator")
    }
}


/// Run JSONLogic for the given rule and data.
///
pub fn jsonlogic(rule: Value, data: Value) -> Result<Value, Error>
{

    match rule {
        Value::Object(ref obj) => {
            if let Ok(op) = is_logic(&rule) {
                match obj.get(op) {
                    Some(val) => match val {
                        Value::Array(args) => {
                            get_operator(op)(args, &Map::new())
                        }
                        _ => panic!("ahh")
                    }
                    None => panic!("ahh")
                }
                // get_operator(op)()
            } else {
                Ok(rule)
            }
        }
        _ => Ok(rule)
    }
}


#[cfg(test)]
mod tests {
    use serde_json::json;
    use super::*;

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
            (json!({"a": 1, "b": 2}), json!({}), Ok(json!({"a": 1, "b": 2}))),
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
        jsonlogic_cases().into_iter().for_each(
            |(rule, data, exp)| {
                println!("Running rule: {:?} with data: {:?}", rule, data);
                let result = jsonlogic(rule, data);
                println!("- Result: {:?}", result);
                println!("- Expected: {:?}", exp);
                if exp.is_ok() {
                    assert_eq!(result.unwrap(), exp.unwrap());
                } else {
                    assert_eq!(result.unwrap_err(), exp.unwrap_err());
                }
            }
        )
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
        Value::Number(v) => {
            v.as_f64().map(
                |v_num| if v_num == 0.0 { false } else { true }
            ).unwrap_or(false)
        }
        Value::String(v) => if v == "" { false } else { true },
        Value::Array(v) => if v.len() == 0 { false } else { true },
        Value::Object(_) => true,
    }
}
