//! Implementations of JavaScript operators for JSON Values

use serde_json::{Number, Value};
use std::f64;
use std::str::FromStr;

use crate::error::Error;

// numeric characters according to parseFloat
const NUMERICS: &'static [char] = &[
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', '0', '.', '-', '+', 'e', 'E',
];

// TODOS:
// - there are too many tests in docstrings
// - the docstrings are too sarcastic about JS equality

pub fn to_string(value: &Value) -> String {
    match value {
        Value::Object(_) => String::from("[object Object]"),
        Value::Bool(val) => val.to_string(),
        Value::Null => String::from("null"),
        Value::Number(val) => val.to_string(),
        Value::String(val) => String::from(val),
        Value::Array(val) => val
            .iter()
            .map(|i| match i {
                Value::Null => String::from(""),
                _ => to_string(i),
            })
            .collect::<Vec<String>>()
            .join(","),
    }
}

/// Implement something like OrdinaryToPrimitive() with a Number hint.
///
/// If it's possible to return a numeric primitive, returns Some<f64>.
/// Otherwise, return None.
fn to_primitive_number(value: &Value) -> Option<f64> {
    match value {
        // .valueOf() returns the object itself, which is not a primitive
        Value::Object(_) => None,
        // .valueOf() returns the array itself
        Value::Array(_) => None,
        Value::Bool(val) => {
            if *val {
                Some(1.0)
            } else {
                Some(0.0)
            }
        }
        Value::Null => Some(0.0),
        Value::Number(val) => val.as_f64(),
        Value::String(_) => None, // already a primitive
    }
}

pub fn str_to_number<S: AsRef<str>>(string: S) -> Option<f64> {
    let s = string.as_ref();
    if s == "" {
        Some(0.0)
    } else {
        f64::from_str(s).ok()
    }
}

enum Primitive {
    String(String),
    Number(f64),
}

#[allow(dead_code)]
enum PrimitiveHint {
    String,
    Number,
    Default,
}

fn to_primitive(value: &Value, hint: PrimitiveHint) -> Primitive {
    match hint {
        PrimitiveHint::String => Primitive::String(to_string(value)),
        _ => to_primitive_number(value)
            .map(Primitive::Number)
            .unwrap_or(Primitive::String(to_string(value))),
    }
}

/// Do our best to convert something into a number.
///
/// Should be pretty much equivalent to calling Number(value) in JS,
/// returning None where that would return NaN.
pub fn to_number(value: &Value) -> Option<f64> {
    match to_primitive(value, PrimitiveHint::Number) {
        Primitive::Number(num) => Some(num),
        Primitive::String(string) => str_to_number(string),
    }
}

/// Compare values in the JavaScript `==` style
///
/// Implements the Abstract Equality Comparison algorithm (`==` in JS)
/// as defined [here](https://www.ecma-international.org/ecma-262/5.1/#sec-11.9.3).
///
/// ```rust
/// use serde_json::json;
/// use jsonlogic_rs::js_op::abstract_eq;
///
/// assert!(
///   abstract_eq(
///     &json!(null),
///     &json!(null),
///   )
/// );
/// assert!(
///   abstract_eq(
///     &json!(1.0),
///     &json!(1),
///   )
/// );
/// assert!(
///   abstract_eq(
///     &json!("foo"),
///     &json!("foo"),
///   )
/// );
/// assert!(
///   abstract_eq(
///     &json!(true),
///     &json!(true),
///   )
/// );
/// assert!(
///   abstract_eq(
///     &json!("1"),
///     &json!(1.0),
///   )
/// );
/// assert!(
///   abstract_eq(
///     &json!(1.0),
///     &json!("1"),
///   )
/// );
/// assert!(
///   abstract_eq(
///     &json!(true),
///     &json!("1"),
///   )
/// );
/// assert!(
///   abstract_eq(
///     &json!(true),
///     &json!(1.0),
///   )
/// );
/// assert!(
///   abstract_eq(
///     &json!({}),
///     &json!("[object Object]"),
///   )
/// );
///
/// assert!(
///   ! abstract_eq(
///     &json!({}),
///     &json!({}),
///   )
/// );
/// assert!(
///   ! abstract_eq(
///     &json!([]),
///     &json!([]),
///   )
/// );
/// ```
pub fn abstract_eq(first: &Value, second: &Value) -> bool {
    // Follows the ECMA specification 2019:7.2.14 (Abstract Equality Comparison)
    match (first, second) {
        // 1. If Type(x) is the same as Type(y), then
        //   a. If Type(x) is Undefined, return true.
        //      - No need to handle this case, b/c undefined is not in JSON
        //   b. If Type(x) is Null, return true.
        (Value::Null, Value::Null) => true,
        //   c. If Type(x) is Number, then
        (Value::Number(x), Value::Number(y)) => {
            // i. If x is NaN, return false.
            //    - we can ignore this case, b/c NaN is not in JSON
            // ii. If y is NaN, return false.
            //    - same here
            // iii. If x is the same Number value as y, return true.
            x.as_f64()
                .map(|x_val| y.as_f64().map(|y_val| x_val == y_val).unwrap_or(false))
                .unwrap_or(false)
            // x.as_f64() == y.as_f64()
            // iv. If x is +0 and y is −0, return true.
            //     - with serde's Number, this is handled by the above
            // v. If x is −0 and y is +0, return true.
            //    - same here
            // vi. Return false.
            //     - done!
        }
        //   d. If Type(x) is String, then return true if x and y are exactly
        //      the same sequence of characters (same length and same characters
        //      in corresponding positions). Otherwise, return false.
        (Value::String(x), Value::String(y)) => x == y,
        //   e. If Type(x) is Boolean, return true if x and y are both true
        //      or both false. Otherwise, return false.
        (Value::Bool(x), Value::Bool(y)) => x == y,
        //   f. Return true if x and y refer to the same object. Otherwise, return false.
        //      - not applicable to comparisons from JSON
        // 2. If x is null and y is undefined, return true.
        //    - not applicable to JSON b/c there is no undefined
        // 3. If x is undefined and y is null, return true.
        //    - not applicable to JSON b/c there is no undefined
        // 4. If Type(x) is Number and Type(y) is String, return the result of
        //    the comparison x == ToNumber(y).
        (Value::Number(x), Value::String(y)) => {
            // the empty string is 0
            let y_res = str_to_number(y);
            y_res
                .map(|y_number| {
                    x.as_f64()
                        .map(|x_number| x_number == y_number)
                        .unwrap_or(false)
                })
                .unwrap_or(false)
        }
        // 5. If Type(x) is String and Type(y) is Number, return the result
        //    of the comparison ToNumber(x) == y.
        (Value::String(x), Value::Number(y)) => {
            let x_res = str_to_number(x);
            x_res
                .map(|x_number| {
                    y.as_f64()
                        .map(|y_number| x_number == y_number)
                        .unwrap_or(false)
                })
                .unwrap_or(false)
        }
        // 6. If Type(x) is Boolean, return the result of the comparison ToNumber(x) == y.
        (Value::Bool(x), _) => match x {
            true => Number::from_f64(1 as f64)
                .map(|num| {
                    let value = Value::Number(num);
                    abstract_eq(&value, second)
                })
                .unwrap_or(false),
            false => Number::from_f64(0 as f64)
                .map(|num| {
                    let value = Value::Number(num);
                    abstract_eq(&value, second)
                })
                .unwrap_or(false),
        },
        // 7. If Type(y) is Boolean, return the result of the comparison x == ToNumber(y).
        (_, Value::Bool(y)) => match y {
            true => Number::from_f64(1 as f64)
                .map(|num| {
                    let value = Value::Number(num);
                    abstract_eq(first, &value)
                })
                .unwrap_or(false),
            false => Number::from_f64(0 as f64)
                .map(|num| {
                    let value = Value::Number(num);
                    abstract_eq(first, &value)
                })
                .unwrap_or(false),
        },
        // 8. If Type(x) is either String, Number, or Symbol and Type(y) is
        //    Object, return the result of the comparison x == ToPrimitive(y).
        // NB: the only type of Objects we get in JSON are regular old arrays
        //     and regular old objects. ToPrimitive on the former yields a
        //     stringification of its values, stuck together with commands,
        //     but with no brackets on the outside. ToPrimitive on the later
        //     is just always [object Object].
        (Value::String(_), Value::Array(_)) | (Value::Number(_), Value::Array(_)) => {
            abstract_eq(first, &Value::String(to_string(second)))
        }
        (Value::String(_), Value::Object(_)) | (Value::Number(_), Value::Object(_)) => {
            abstract_eq(first, &Value::String(to_string(second)))
        }
        // 9. If Type(x) is Object and Type(y) is either String, Number, or
        //    Symbol, return the result of the comparison ToPrimitive(x) == y.
        (Value::Object(_), Value::String(_)) | (Value::Object(_), Value::Number(_)) => {
            abstract_eq(&Value::String(to_string(first)), second)
        }
        (Value::Array(_), Value::String(_)) | (Value::Array(_), Value::Number(_)) => {
            abstract_eq(&Value::String(to_string(first)), second)
        }
        _ => false,
    }
}

/// Perform JS-style strict equality
///
/// Items are strictly equal if:
/// - They are the same non-primitive object
/// - They are a primitive object of the same type with the same value
///
/// ```rust
/// use serde_json::json;
/// use jsonlogic_rs::js_op::strict_eq;
///
/// // References of the same type and value are strictly equal
/// assert!(strict_eq(&json!(1), &json!(1)));
/// assert!(strict_eq(&json!(false), &json!(false)));
/// assert!(strict_eq(&json!("foo"), &json!("foo")));
///
/// // "Abstract" type conversion is not performed for strict equality
/// assert!(!strict_eq(&json!(0), &json!(false)));
/// assert!(!strict_eq(&json!(""), &json!(0)));
///
/// // Objects only compare equal if they are the same reference
/// assert!(!strict_eq(&json!([]), &json!([])));
/// assert!(!strict_eq(&json!({}), &json!({})));
///
/// let arr = json!([]);
/// let obj = json!({});
/// assert!(strict_eq(&arr, &arr));
/// assert!(strict_eq(&obj, &obj));
/// ```
///
pub fn strict_eq(first: &Value, second: &Value) -> bool {
    if std::ptr::eq(first, second) {
        return true;
    };
    match (first, second) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Number(x), Value::Number(y)) => x
            .as_f64()
            .and_then(|x_val| y.as_f64().map(|y_val| x_val == y_val))
            .unwrap_or(false),
        (Value::String(x), Value::String(y)) => x == y,
        _ => false,
    }
}

pub fn strict_ne(first: &Value, second: &Value) -> bool {
    !strict_eq(first, second)
}

/// Perform JS-style abstract less-than
///
///
/// ```rust
/// use serde_json::json;
/// use jsonlogic_rs::js_op::abstract_lt;
///
/// assert_eq!(abstract_lt(&json!(-1), &json!(0)), true);
/// assert_eq!(abstract_lt(&json!("-1"), &json!(0)), true);
/// assert_eq!(abstract_lt(&json!(0), &json!(1)), true);
/// assert_eq!(abstract_lt(&json!(0), &json!("1")), true);
/// assert_eq!(abstract_lt(&json!(0), &json!("a")), false);
/// ```
pub fn abstract_lt(first: &Value, second: &Value) -> bool {
    match (
        to_primitive(first, PrimitiveHint::Number),
        to_primitive(second, PrimitiveHint::Number),
    ) {
        (Primitive::String(f), Primitive::String(s)) => f < s,
        (Primitive::Number(f), Primitive::Number(s)) => f < s,
        (Primitive::String(f), Primitive::Number(s)) => {
            if let Some(f) = str_to_number(f) {
                f < s
            } else {
                false
            }
        }
        (Primitive::Number(f), Primitive::String(s)) => {
            if let Some(s) = str_to_number(s) {
                f < s
            } else {
                false
            }
        }
    }
}

/// JS-style abstract gt
///
/// ```rust
/// use serde_json::json;
/// use jsonlogic_rs::js_op::abstract_gt;
///
/// assert_eq!(abstract_gt(&json!(0), &json!(-1)), true);
/// assert_eq!(abstract_gt(&json!(0), &json!("-1")), true);
/// assert_eq!(abstract_gt(&json!(1), &json!(0)), true);
/// assert_eq!(abstract_gt(&json!("1"), &json!(0)), true);
/// ```
pub fn abstract_gt(first: &Value, second: &Value) -> bool {
    match (
        to_primitive(first, PrimitiveHint::Number),
        to_primitive(second, PrimitiveHint::Number),
    ) {
        (Primitive::String(f), Primitive::String(s)) => f > s,
        (Primitive::Number(f), Primitive::Number(s)) => f > s,
        (Primitive::String(f), Primitive::Number(s)) => {
            if let Some(f) = str_to_number(f) {
                f > s
            } else {
                false
            }
        }
        (Primitive::Number(f), Primitive::String(s)) => {
            if let Some(s) = str_to_number(s) {
                f > s
            } else {
                false
            }
        }
    }
}

/// Abstract inequality
pub fn abstract_ne(first: &Value, second: &Value) -> bool {
    !abstract_eq(first, second)
}

/// Provide abstract <= comparisons
pub fn abstract_lte(first: &Value, second: &Value) -> bool {
    abstract_lt(first, second) || abstract_eq(first, second)
}

/// Provide abstract >= comparisons
pub fn abstract_gte(first: &Value, second: &Value) -> bool {
    abstract_gt(first, second) || abstract_eq(first, second)
}

/// Get the max of an array of values, performing abstract type conversion
pub fn abstract_max(items: &Vec<&Value>) -> Result<f64, Error> {
    items
        .into_iter()
        .map(|v| {
            to_number(v).ok_or(Error::InvalidArgument {
                value: (*v).clone(),
                operation: "max".into(),
                reason: "Could not convert value to number".into(),
            })
        })
        .fold(Ok(f64::NEG_INFINITY), |acc, cur| {
            let max = acc?;
            match cur {
                Ok(num) => {
                    if num > max {
                        Ok(num)
                    } else {
                        Ok(max)
                    }
                }
                _ => cur,
            }
        })
}

/// Get the max of an array of values, performing abstract type conversion
pub fn abstract_min(items: &Vec<&Value>) -> Result<f64, Error> {
    items
        .into_iter()
        .map(|v| {
            to_number(v).ok_or(Error::InvalidArgument {
                value: (*v).clone(),
                operation: "max".into(),
                reason: "Could not convert value to number".into(),
            })
        })
        .fold(Ok(f64::INFINITY), |acc, cur| {
            let min = acc?;
            match cur {
                Ok(num) => {
                    if num < min {
                        Ok(num)
                    } else {
                        Ok(min)
                    }
                }
                _ => cur,
            }
        })
}

/// Do plus
pub fn abstract_plus(first: &Value, second: &Value) -> Value {
    let first_num = to_primitive_number(first);
    let second_num = to_primitive_number(second);

    match (first_num, second_num) {
        (Some(f), Some(s)) => {
            return Value::Number(Number::from_f64(f + s).unwrap());
        }
        _ => {}
    };

    let first_string = to_string(first);
    let second_string = to_string(second);

    Value::String(first_string.chars().chain(second_string.chars()).collect())
}

/// Add values, parsing to floats first.
///
/// The JSONLogic reference implementation uses the JS `parseFloat` operation
/// on the parameters, which behaves quite differently from the normal JS
/// numeric conversion with `Number(val)`. While the latter uses the
/// `toPrimitive` method on the base object Prototype, the former first
/// converts any incoming value to a string, and then tries to parse it
/// as a float. The upshot is that things that normally parse fine into
/// numbers in JS, like bools and null, convert to NaN, because you can't
/// make "false" into a number.
///
/// The JSONLogic reference implementation deals with any values that
/// evaluate to NaN by returning null. We instead will return an error,
/// the behavior for non-numeric inputs is not specified in the spec,
/// and returning errors seems like a more reasonable course of action
/// than returning null.
pub fn parse_float_add(vals: &Vec<&Value>) -> Result<f64, Error> {
    vals.into_iter()
        .map(|&v| {
            parse_float(v).ok_or(Error::InvalidArgument {
                value: v.clone(),
                operation: "+".into(),
                reason: "Argument could not be converted to a float".into(),
            })
        })
        .fold(Ok(0.0), |acc, cur| {
            let total = acc?;
            match cur {
                Ok(num) => Ok(total + num),
                _ => cur,
            }
        })
}

/// Multiply values, parsing to floats first
///
/// See notes for parse_float_add on how this differs from normal number
/// conversion as is done for _other_ arithmetic operators in the reference
/// implementation
pub fn parse_float_mul(vals: &Vec<&Value>) -> Result<f64, Error> {
    vals.into_iter()
        .map(|&v| {
            parse_float(v).ok_or(Error::InvalidArgument {
                value: v.clone(),
                operation: "*".into(),
                reason: "Argument could not be converted to a float".into(),
            })
        })
        .fold(Ok(1.0), |acc, cur| {
            let total = acc?;
            match cur {
                Ok(num) => Ok(total * num),
                _ => cur,
            }
        })
}

/// Do minus
pub fn abstract_minus(first: &Value, second: &Value) -> Result<f64, Error> {
    let first_num = to_number(first);
    let second_num = to_number(second);

    if let None = first_num {
        return Err(Error::InvalidArgument {
            value: first.clone(),
            operation: "-".into(),
            reason: "Could not convert value to number.".into(),
        });
    }
    if let None = second_num {
        return Err(Error::InvalidArgument {
            value: second.clone(),
            operation: "-".into(),
            reason: "Could not convert value to number.".into(),
        });
    }

    Ok(first_num.unwrap() - second_num.unwrap())
}

/// Do division
pub fn abstract_div(first: &Value, second: &Value) -> Result<f64, Error> {
    let first_num = to_number(first);
    let second_num = to_number(second);

    if let None = first_num {
        return Err(Error::InvalidArgument {
            value: first.clone(),
            operation: "/".into(),
            reason: "Could not convert value to number.".into(),
        });
    }
    if let None = second_num {
        return Err(Error::InvalidArgument {
            value: second.clone(),
            operation: "/".into(),
            reason: "Could not convert value to number.".into(),
        });
    }

    Ok(first_num.unwrap() / second_num.unwrap())
}

/// Do modulo
pub fn abstract_mod(first: &Value, second: &Value) -> Result<f64, Error> {
    let first_num = to_number(first);
    let second_num = to_number(second);

    if let None = first_num {
        return Err(Error::InvalidArgument {
            value: first.clone(),
            operation: "%".into(),
            reason: "Could not convert value to number.".into(),
        });
    }
    if let None = second_num {
        return Err(Error::InvalidArgument {
            value: second.clone(),
            operation: "%".into(),
            reason: "Could not convert value to number.".into(),
        });
    }

    Ok(first_num.unwrap() % second_num.unwrap())
}

/// Attempt to convert a value to a negative number
pub fn to_negative(val: &Value) -> Result<f64, Error> {
    to_number(val)
        .map(|v| -1.0 * v)
        .ok_or(Error::InvalidArgument {
            value: val.clone(),
            operation: "to_negative".into(),
            reason: "Could not convert value to a number".into(),
        })
}

/// Try to parse a string as a float, javascript style
///
/// Strip whitespace, accumulate any potentially numeric characters at the
/// start of the string and try to convert them into a float. We don't
/// quite follow the spec exactly: we don't deal with infinity
/// and NaN. That is okay, because this is only used in a context dealing
/// with JSON values, which can't be Infinity or NaN.
fn parse_float_string(val: &String) -> Option<f64> {
    let (mut leading_numerics, _, _) = val.trim().chars().fold(
        (Vec::new(), false, false),
        |(mut acc, broke, saw_decimal), c| {
            if broke {
                // if we hit a nonnumeric last iter, just return what we've got
                (acc, broke, saw_decimal)
            } else if NUMERICS.contains(&c) {
                let is_decimal = c == '.';
                if saw_decimal && is_decimal {
                    // if we're a decimal and we've seen one before, break
                    (acc, true, is_decimal)
                } else {
                    // if we're a numeric, stick it on the acc
                    acc.push(c);
                    (acc, broke, saw_decimal || is_decimal)
                }
            } else {
                // return the acc as is and let 'em know we hit a nonnumeric
                (acc, true, saw_decimal)
            }
        },
    );
    // don't bother collecting into a string if we don't need to
    if leading_numerics.len() == 0 {
        return None;
    };
    if let Some('e') | Some('E') = leading_numerics.last() {
        // If the last character is an 'e' or an `E`, remove it, to match
        // edge case where JS ignores a trailing `e` rather than treating it
        // as bad exponential notation, e.g. JS treats 1e as just 1.
        leading_numerics.pop();
    }

    // collect into a string, try to parse as a float, return an option
    leading_numerics
        .iter()
        .collect::<String>()
        .parse::<f64>()
        .ok()
}

/// Attempt to parse a value into a float.
///
/// The implementation should match https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/parseFloat
/// as closely as is reasonable.
pub fn parse_float(val: &Value) -> Option<f64> {
    match val {
        Value::Number(num) => num.as_f64(),
        Value::String(string) => parse_float_string(string),
        _ => parse_float(&Value::String(to_string(&val))),
    }
}

// =====================================================================
// Unit Tests
// =====================================================================

#[cfg(test)]
mod abstract_operations {

    use super::*;
    use serde_json::json;

    fn equal_values() -> Vec<(Value, Value)> {
        vec![
            (json!(null), json!(null)),
            (json!(1), json!(1)),
            (json!(1), json!(1.0)),
            (json!(1.0), json!(1)),
            (json!(0), json!(-0)),
            (json!(-0), json!(0)),
            (json!("foo"), json!("foo")),
            (json!(""), json!("")),
            (json!(true), json!(true)),
            (json!(false), json!(false)),
            (json!(1), json!("1")),
            (json!(1), json!("1.0")),
            (json!(1.0), json!("1.0")),
            (json!(1.0), json!("1")),
            (json!(0), json!("")),
            (json!(0), json!("0")),
            (json!(0), json!("-0")),
            (json!(0), json!("+0")),
            (json!(-1), json!("-1")),
            (json!(-1.0), json!("-1")),
            (json!(true), json!(1)),
            (json!(true), json!("1")),
            (json!(true), json!("1.0")),
            (json!(true), json!([1])),
            (json!(true), json!(["1"])),
            (json!(false), json!(0)),
            (json!(false), json!([])),
            (json!(false), json!([0])),
            (json!(false), json!("")),
            (json!(false), json!("0")),
            (json!("[object Object]"), json!({})),
            (json!("[object Object]"), json!({"a": "a"})),
            (json!(""), json!([])),
            (json!(""), json!([null])),
            (json!(","), json!([null, null])),
            (json!("1,2"), json!([1, 2])),
            (json!("a,b"), json!(["a", "b"])),
            (json!(0), json!([])),
            (json!(false), json!([])),
            (json!(true), json!([1])),
            (json!([]), json!("")),
            (json!([null]), json!("")),
            (json!([null, null]), json!(",")),
            (json!([1, 2]), json!("1,2")),
            (json!(["a", "b"]), json!("a,b")),
            (json!([]), json!(0)),
            (json!([0]), json!(0)),
            (json!([]), json!(false)),
            (json!([0]), json!(false)),
            (json!([1]), json!(true)),
        ]
    }

    fn lt_values() -> Vec<(Value, Value)> {
        vec![
            (json!(-1), json!(0)),
            (json!("-1"), json!(0)),
            (json!(0), json!(1)),
            (json!(0), json!("1")),
            (json!("foo"), json!("foos")),
            (json!(""), json!("a")),
            (json!(""), json!([1])),
            (json!(""), json!([1, 2])),
            (json!(""), json!("1")),
            (json!(""), json!({})),
            (json!(""), json!({"a": 1})),
            (json!(false), json!(true)),
            (json!(false), json!(1)),
            (json!(false), json!("1")),
            (json!(false), json!([1])),
            (json!(null), json!(1)),
            (json!(null), json!(true)),
            (json!(null), json!("1")),
            (json!([]), json!([1])),
            (json!([]), json!([1, 2])),
            (json!(0), json!([1])),
            (json!("0"), json!({})),
            (json!("0"), json!({"a": 1})),
            (json!("0"), json!([1, 2])),
        ]
    }

    fn gt_values() -> Vec<(Value, Value)> {
        vec![
            (json!(0), json!(-1)),
            (json!(0), json!("-1")),
            (json!(1), json!(0)),
            (json!("1"), json!(0)),
            (json!("foos"), json!("foo")),
            (json!("a"), json!("")),
            (json!([1]), json!("")),
            (json!("1"), json!("")),
            (json!("1"), json!("0")),
            (json!(true), json!(false)),
            (json!(1), json!(false)),
            (json!("1"), json!(false)),
            (json!([1]), json!(false)),
            (json!(1), json!(null)),
            (json!(true), json!(null)),
            (json!("1"), json!(null)),
            (json!([1]), json!([])),
            (json!([1, 2]), json!([])),
        ]
    }

    fn ne_values() -> Vec<(Value, Value)> {
        vec![
            (json!([]), json!([])),
            (json!([1]), json!([1])),
            (json!([1, 1]), json!([1, 1])),
            (json!({}), json!({})),
            (json!({"a": 1}), json!({"a": 1})),
            (json!([]), json!({})),
            (json!(0), json!(1)),
            (json!("a"), json!("b")),
            (json!(true), json!(false)),
            (json!(true), json!([0])),
            (json!(1.0), json!(1.1)),
            (json!(null), json!(0)),
            (json!(null), json!("")),
            (json!(null), json!(false)),
            (json!(null), json!(true)),
        ]
    }

    /// Values that do not compare true for anything other than ne.
    fn not_gt_not_lt_not_eq() -> Vec<(Value, Value)> {
        vec![
            (json!(null), json!("")),
            (json!(null), json!("a")),
            (json!(0), json!("a")),
            (json!(0), json!([1, 2])),
            (json!([]), json!([])),
            (json!([1]), json!([1])),
            (json!([1, 2]), json!([1, 2])),
            (json!({}), json!({})),
            (json!(false), json!({})),
            (json!(true), json!({})),
            (json!(false), json!([1, 2])),
            (json!(true), json!([1, 2])),
        ]
    }

    fn plus_cases() -> Vec<(Value, Value, Value)> {
        vec![
            (json!(1), json!(1), json!(2.0)),
            (json!(1), json!(true), json!(2.0)),
            (json!(true), json!(true), json!(2.0)),
            (json!(1), json!(false), json!(1.0)),
            (json!(false), json!(false), json!(0.0)),
            (json!(1), json!(null), json!(1.0)),
            (json!(null), json!(null), json!(0.0)),
            (json!(1), json!("1"), json!("11")),
            (json!(1), json!([1]), json!("11")),
            (json!(1), json!([1, 2]), json!("11,2")),
            (json!(1), json!([1, null, 3]), json!("11,,3")),
            (json!(1), json!({}), json!("1[object Object]")),
        ]
    }

    #[test]
    fn test_to_string_obj() {
        assert_eq!(&to_string(&json!({})), "[object Object]");
        assert_eq!(&to_string(&json!({"a": "b"})), "[object Object]");
    }

    #[test]
    fn test_to_string_array() {
        assert_eq!(&to_string(&json!([])), "");
        assert_eq!(&to_string(&json!([1, 2, 3])), "1,2,3");
        assert_eq!(&to_string(&json!([1, [2, 3], 4])), "1,2,3,4");
        assert_eq!(&to_string(&json!([1, {}, 2])), "1,[object Object],2");
        assert_eq!(&to_string(&json!(["a", "b"])), "a,b");
        assert_eq!(&to_string(&json!([null])), "");
        assert_eq!(&to_string(&json!([null, 1, 2, null])), ",1,2,");
        assert_eq!(&to_string(&json!([true, false])), "true,false");
    }

    #[test]
    fn test_to_string_null() {
        assert_eq!(&to_string(&json!(null)), "null");
    }

    #[test]
    fn test_to_string_bool() {
        assert_eq!(&to_string(&json!(true)), "true");
        assert_eq!(&to_string(&json!(false)), "false");
    }

    #[test]
    fn test_to_string_number() {
        assert_eq!(&to_string(&json!(1.0)), "1.0");
        assert_eq!(&to_string(&json!(1)), "1");
    }

    #[test]
    fn test_abstract_eq() {
        equal_values().iter().for_each(|(first, second)| {
            println!("{:?}-{:?}", &first, &second);
            assert!(abstract_eq(&first, &second), true);
        })
    }

    #[test]
    fn test_abstract_ne() {
        ne_values().iter().for_each(|(first, second)| {
            println!("{:?}-{:?}", &first, &second);
            assert_eq!(abstract_ne(&first, &second), true);
        })
    }

    #[test]
    fn test_abstract_lt() {
        lt_values().iter().for_each(|(first, second)| {
            println!("{:?}-{:?}", &first, &second);
            assert_eq!(abstract_lt(&first, &second), true);
        })
    }

    #[test]
    fn test_abstract_gt() {
        gt_values().iter().for_each(|(first, second)| {
            println!("{:?}-{:?}", &first, &second);
            assert_eq!(abstract_gt(&first, &second), true);
        })
    }

    #[test]
    fn test_eq_values_are_not_lt() {
        equal_values().iter().for_each(|(first, second)| {
            println!("{:?}-{:?}", &first, &second);
            assert_eq!(abstract_lt(&first, &second), false);
        })
    }

    #[test]
    fn test_eq_values_are_not_gt() {
        equal_values().iter().for_each(|(first, second)| {
            println!("{:?}-{:?}", &first, &second);
            assert_eq!(abstract_gt(&first, &second), false);
        })
    }

    #[test]
    fn test_eq_values_are_not_ne() {
        equal_values().iter().for_each(|(first, second)| {
            println!("{:?}-{:?}", &first, &second);
            assert_eq!(abstract_ne(&first, &second), false);
        })
    }

    #[test]
    fn test_lt_values_are_not_eq() {
        lt_values().iter().for_each(|(first, second)| {
            println!("{:?}-{:?}", &first, &second);
            assert_eq!(abstract_eq(&first, &second), false);
        })
    }

    #[test]
    fn test_lt_values_are_not_gt() {
        lt_values().iter().for_each(|(first, second)| {
            println!("{:?}-{:?}", &first, &second);
            assert_eq!(abstract_gt(&first, &second), false);
        })
    }

    #[test]
    fn test_lt_values_are_ne() {
        lt_values().iter().for_each(|(first, second)| {
            println!("{:?}-{:?}", &first, &second);
            assert_eq!(abstract_ne(&first, &second), true);
        })
    }

    #[test]
    fn test_gt_values_are_not_eq() {
        gt_values().iter().for_each(|(first, second)| {
            println!("{:?}-{:?}", &first, &second);
            assert_eq!(abstract_eq(&first, &second), false);
        })
    }

    #[test]
    fn test_gt_values_are_not_lt() {
        gt_values().iter().for_each(|(first, second)| {
            println!("{:?}-{:?}", &first, &second);
            assert_eq!(abstract_lt(&first, &second), false);
        })
    }

    #[test]
    fn test_gt_values_are_ne() {
        gt_values().iter().for_each(|(first, second)| {
            println!("{:?}-{:?}", &first, &second);
            assert_eq!(abstract_ne(&first, &second), true);
        })
    }

    #[test]
    fn test_incomparable() {
        not_gt_not_lt_not_eq().iter().for_each(|(first, second)| {
            println!("{:?}-{:?}", &first, &second);
            assert_eq!(abstract_lt(&first, &second), false);
            assert_eq!(abstract_gt(&first, &second), false);
            assert_eq!(abstract_eq(&first, &second), false);
        })
    }

    // abstract_lte

    #[test]
    fn test_lt_values_are_lte() {
        lt_values().iter().for_each(|(first, second)| {
            println!("{:?}-{:?}", &first, &second);
            assert_eq!(abstract_lte(&first, &second), true);
        })
    }

    #[test]
    fn test_eq_values_are_lte() {
        equal_values().iter().for_each(|(first, second)| {
            println!("{:?}-{:?}", &first, &second);
            assert_eq!(abstract_lte(&first, &second), true);
        })
    }

    #[test]
    fn test_gt_values_are_not_lte() {
        gt_values().iter().for_each(|(first, second)| {
            println!("{:?}-{:?}", &first, &second);
            assert_eq!(abstract_lte(&first, &second), false);
        })
    }

    // abstract_gte

    #[test]
    fn test_gt_values_are_gte() {
        gt_values().iter().for_each(|(first, second)| {
            println!("{:?}-{:?}", &first, &second);
            assert_eq!(abstract_gte(&first, &second), true);
        })
    }

    #[test]
    fn test_eq_values_are_gte() {
        equal_values().iter().for_each(|(first, second)| {
            println!("{:?}-{:?}", &first, &second);
            assert_eq!(abstract_gte(&first, &second), true);
        })
    }

    #[test]
    fn test_lt_values_are_not_gte() {
        lt_values().iter().for_each(|(first, second)| {
            println!("{:?}-{:?}", &first, &second);
            assert_eq!(abstract_gte(&first, &second), false);
        })
    }

    #[test]
    fn test_abstract_plus() {
        plus_cases().iter().for_each(|(first, second, exp)| {
            println!("{:?}-{:?}", &first, &second);
            let result = abstract_plus(&first, &second);
            match result {
                Value::Number(ref i) => match exp {
                    Value::Number(j) => assert_eq!(i, j),
                    _ => assert!(false),
                },
                Value::String(ref i) => match exp {
                    Value::String(j) => assert_eq!(i, j),
                    _ => assert!(false),
                },
                _ => assert!(false),
            }
        })
    }
}

#[cfg(test)]
mod test_abstract_max {
    use super::*;
    use serde_json::json;

    fn max_cases() -> Vec<(Vec<Value>, Result<f64, ()>)> {
        vec![
            (vec![json!(1), json!(2), json!(3)], Ok(3.0)),
            (vec![json!("1"), json!(true), json!([1])], Ok(1.0)),
            (
                vec![json!(""), json!(null), json!([]), json!(false)],
                Ok(0.0),
            ),
            (vec![json!("foo")], Err(())),
            (vec![], Ok(f64::NEG_INFINITY)),
        ]
    }

    #[test]
    fn test_abstract_max() {
        max_cases().into_iter().for_each(|(items, exp)| {
            println!("Max: {:?}", items);
            let res = abstract_max(&items.iter().collect());
            println!("Res: {:?}", res);
            match exp {
                Ok(exp) => assert_eq!(res.unwrap(), exp),
                _ => {
                    res.unwrap_err();
                }
            };
        })
    }
}

#[cfg(test)]
mod test_abstract_min {
    use super::*;
    use serde_json::json;

    fn min_cases() -> Vec<(Vec<Value>, Result<f64, ()>)> {
        vec![
            (vec![json!(1), json!(2), json!(3)], Ok(1.0)),
            (vec![json!("1"), json!(true), json!([1])], Ok(1.0)),
            (
                vec![json!(""), json!(null), json!([]), json!(false)],
                Ok(0.0),
            ),
            (vec![json!("foo")], Err(())),
            (vec![], Ok(f64::INFINITY)),
        ]
    }

    #[test]
    fn test_abstract_min() {
        min_cases().into_iter().for_each(|(items, exp)| {
            println!("Min: {:?}", items);
            let res = abstract_min(&items.iter().collect());
            println!("Res: {:?}", res);
            match exp {
                Ok(exp) => assert_eq!(res.unwrap(), exp),
                _ => {
                    res.unwrap_err();
                }
            };
        })
    }
}

#[cfg(test)]
mod test_abstract_minus {
    use super::*;
    use serde_json::json;

    fn minus_cases() -> Vec<(Value, Value, Result<f64, ()>)> {
        vec![
            (json!(5), json!(2), Ok(3.0)),
            (json!(0), json!(2), Ok(-2.0)),
            (json!("5"), json!(2), Ok(3.0)),
            (json!(["5"]), json!(2), Ok(3.0)),
            (json!(["5"]), json!(true), Ok(4.0)),
            (json!("foo"), json!(true), Err(())),
        ]
    }

    #[test]
    fn test_abstract_minus() {
        minus_cases().into_iter().for_each(|(first, second, exp)| {
            println!("Minus: {:?} - {:?}", first, second);
            let res = abstract_minus(&first, &second);
            println!("Res: {:?}", res);
            match exp {
                Ok(exp) => assert_eq!(res.unwrap(), exp),
                _ => {
                    res.unwrap_err();
                }
            }
        })
    }
}

#[cfg(test)]
mod test_strict {

    use super::*;
    use serde_json::json;

    fn eq_values() -> Vec<(Value, Value)> {
        vec![
            (json!(""), json!("")),
            (json!("foo"), json!("foo")),
            (json!(1), json!(1)),
            (json!(1), json!(1.0)),
            (json!(null), json!(null)),
            (json!(true), json!(true)),
            (json!(false), json!(false)),
        ]
    }

    fn ne_values() -> Vec<(Value, Value)> {
        vec![
            (json!({}), json!({})),
            (json!({"a": "a"}), json!({"a": "a"})),
            (json!([]), json!([])),
            (json!("foo"), json!("noop")),
            (json!(1), json!(2)),
            (json!(0), json!([])),
            (json!(0), json!([0])),
            (json!(false), json!(null)),
            (json!(true), json!(false)),
            (json!(false), json!(true)),
            (json!(false), json!([])),
            (json!(false), json!("")),
        ]
    }

    #[test]
    fn test_strict_eq() {
        eq_values().iter().for_each(|(first, second)| {
            println!("{:?}-{:?}", &first, &second);
            assert!(strict_eq(&first, &second));
        });
        ne_values().iter().for_each(|(first, second)| {
            println!("{:?}-{:?}", &first, &second);
            assert!(!strict_eq(&first, &second));
        });
    }

    #[test]
    fn test_strict_eq_same_obj() {
        let obj = json!({});
        assert!(strict_eq(&obj, &obj))
    }

    #[test]
    fn test_strict_ne() {
        ne_values().iter().for_each(|(first, second)| {
            println!("{:?}-{:?}", &first, &second);
            assert!(strict_ne(&first, &second));
        });
        eq_values().iter().for_each(|(first, second)| {
            println!("{:?}-{:?}", &first, &second);
            assert!(!strict_ne(&first, &second));
        });
    }

    #[test]
    fn test_strict_ne_same_obj() {
        let obj = json!({});
        assert!(!strict_ne(&obj, &obj))
    }
}

#[cfg(test)]
mod test_parse_float {
    use super::*;
    use serde_json::json;

    fn cases() -> Vec<(Value, Option<f64>)> {
        vec![
            (json!(1), Some(1.0)),
            (json!(1.5), Some(1.5)),
            (json!(-1.5), Some(-1.5)),
            (json!("1"), Some(1.0)),
            (json!("1e2"), Some(100.0)),
            (json!("1E2"), Some(100.0)),
            (json!("1.1e2"), Some(110.0)),
            (json!("-1.1e2"), Some(-110.0)),
            (json!("1e-2"), Some(0.01)),
            (json!("1.0"), Some(1.0)),
            (json!("1.1"), Some(1.1)),
            (json!("1.1.1"), Some(1.1)),
            (json!("1234abc"), Some(1234.0)),
            (json!("1e"), Some(1.0)),
            (json!("1E"), Some(1.0)),
            (json!(false), None),
            (json!(true), None),
            (json!(null), None),
            (json!("+5"), Some(5.0)),
            (json!("-5"), Some(-5.0)),
            (json!([]), None),
            (json!([1]), Some(1.0)),
            // this is weird, but correct. it converts to a string first
            // "1,2" and then parses up to the first comma as a number
            (json!([1, 2]), Some(1.0)),
            (json!({}), None),
        ]
    }

    #[test]
    fn test_parse_float() {
        cases()
            .into_iter()
            .for_each(|(input, exp)| assert_eq!(parse_float(&input), exp));
    }
}
