use serde_json::{Number, Value};
use std::f64;
use std::str::FromStr;

fn to_string(value: &Value) -> String {
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


fn to_primitive_number(value: &Value) -> Option<f64> {
    match value {
        Value::Object(_) => None,
        Value::Array(val) => { if val.len() == 0 { Some(0.0) } else { None } },
        Value::Bool(val) => { if *val { Some(1.0) } else { Some(0.0) } },
        Value::Null => Some(0.0),
        Value::Number(val) => val.as_f64(),
        Value::String(_) => None,  // already a primitive
    }
}


fn str_to_number<S: AsRef<str>>(string: S) -> Option<f64> {
    let s = string.as_ref();
    if s == "" { Some(0.0) } else { f64::from_str(s).ok() }
}

enum Primitive {
    String(String),
    Number(f64),
}


enum PrimitiveHint {
    String,
    Number,
    Default,
}


fn to_primitive(value: &Value, hint: PrimitiveHint) -> Primitive {
    match hint {
        PrimitiveHint::String => {
            Primitive::String(to_string(value))
        },
        _ => {
            to_primitive_number(value)
            .map(Primitive::Number)
            .unwrap_or(Primitive::String(to_string(value)))
        }
    }
}


/// Compare values in the JavaScript `==` style
///
/// Implements the Abstract Equality Comparison algorithm (`==` in JS)
/// as defined [here](https://www.ecma-international.org/ecma-262/5.1/#sec-11.9.3).
///
/// ```rust
/// use serde_json::json;
/// use jsonlogic::abstract_eq;
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

pub fn strict_eq(first: &Value, second: &Value) -> bool {
    if std::ptr::eq(first, second) {
        return true
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

/// Perform JS-style abstract less-than
///
///
/// ```rust
/// use serde_json::json;
/// use jsonlogic::abstract_lt;
///
/// assert_eq!(abstract_lt(&json!(-1), &json!(0)), true);
/// assert_eq!(abstract_lt(&json!("-1"), &json!(0)), true);
/// assert_eq!(abstract_lt(&json!(0), &json!(1)), true);
/// assert_eq!(abstract_lt(&json!(0), &json!("1")), true);
/// assert_eq!(abstract_lt(&json!(0), &json!("a")), false);
/// ```
///
/// ```rust
/// use serde_json::json;
/// use jsonlogic::abstract_lt;
///
/// assert_eq!(abstract_lt(&json!("foo"), &json!("foos")), true);
/// assert_eq!(abstract_lt(&json!(""), &json!("a")), true);
/// assert_eq!(abstract_lt(&json!(""), &json!([1])), true);
/// assert_eq!(abstract_lt(&json!(""), &json!("1")), true);
/// ```
///
/// ```rust
/// use serde_json::json;
/// use jsonlogic::abstract_lt;
///
/// assert_eq!(abstract_lt(&json!(false), &json!(true)), true);
/// assert_eq!(abstract_lt(&json!(false), &json!(1)), true);
/// assert_eq!(abstract_lt(&json!(false), &json!("1")), true);
/// assert_eq!(abstract_lt(&json!(false), &json!([1])), true);
/// ```
///
/// ```rust
/// use serde_json::json;
/// use jsonlogic::abstract_lt;
///
/// assert_eq!(abstract_lt(&json!(null), &json!(1)), true);
/// assert_eq!(abstract_lt(&json!(null), &json!(true)), true);
/// assert_eq!(abstract_lt(&json!(null), &json!("1")), true);
/// assert_eq!(abstract_lt(&json!(null), &json!("")), false);
/// assert_eq!(abstract_lt(&json!(null), &json!("a")), false);
/// ```
///
/// ```rust
/// use serde_json::json;
/// use jsonlogic::abstract_lt;
///
/// assert_eq!(abstract_lt(&json!([]), &json!([1])), true);
/// assert_eq!(abstract_lt(&json!([]), &json!([])), false);
/// assert_eq!(abstract_lt(&json!([]), &json!([1,2])), false);
/// ```
pub fn abstract_lt(first: &Value, second: &Value) -> bool {
    match (to_primitive(first, PrimitiveHint::Number), to_primitive(second, PrimitiveHint::Number)) {
        (Primitive::String(f), Primitive::String(s)) => { f < s },
        (Primitive::Number(f), Primitive::Number(s)) => { f < s },
        (Primitive::String(f), Primitive::Number(s)) => {
            if let Some(f) = str_to_number(f) { f < s } else { false }
        },
        (Primitive::Number(f), Primitive::String(s)) => {
            if let Some(s) = str_to_number(s) { f < s } else { false }
        }
    }
}


/// JS-style abstract gt
///
/// ```rust
/// use serde_json::json;
/// use jsonlogic::abstract_gt;
///
/// assert_eq!(abstract_gt(&json!(-1), &json!(0)), false);
/// assert_eq!(abstract_gt(&json!("-1"), &json!(0)), false);
/// assert_eq!(abstract_gt(&json!(0), &json!(1)), false);
/// assert_eq!(abstract_gt(&json!(0), &json!("1")), false);
/// // Neither gt nor lt, this one
/// assert_eq!(abstract_gt(&json!(0), &json!("a")), false);
/// ```
///
/// ```rust
/// use serde_json::json;
/// use jsonlogic::abstract_gt;
///
/// assert_eq!(abstract_gt(&json!("foo"), &json!("foos")), false);
/// assert_eq!(abstract_gt(&json!(""), &json!("a")), false);
/// assert_eq!(abstract_gt(&json!(""), &json!([1])), false);
/// assert_eq!(abstract_gt(&json!(""), &json!("1")), false);
/// ```
///
/// ```rust
/// use serde_json::json;
/// use jsonlogic::abstract_gt;
///
/// assert_eq!(abstract_gt(&json!(false), &json!(true)), false);
/// assert_eq!(abstract_gt(&json!(false), &json!(1)), false);
/// assert_eq!(abstract_gt(&json!(false), &json!("1")), false);
/// assert_eq!(abstract_gt(&json!(false), &json!([1])), false);
/// ```
///
/// ```rust
/// use serde_json::json;
/// use jsonlogic::abstract_gt;
///
/// assert_eq!(abstract_gt(&json!(null), &json!(1)), false);
/// assert_eq!(abstract_gt(&json!(null), &json!(true)), false);
/// assert_eq!(abstract_gt(&json!(null), &json!("1")), false);
/// assert_eq!(abstract_gt(&json!(null), &json!("")), false);
/// assert_eq!(abstract_gt(&json!(null), &json!("a")), false);
/// ```
///
/// ```rust
/// use serde_json::json;
/// use jsonlogic::abstract_gt;
///
/// assert_eq!(abstract_gt(&json!([]), &json!([1])), false);
/// assert_eq!(abstract_gt(&json!([]), &json!([])), false);
/// assert_eq!(abstract_gt(&json!([]), &json!([1,2])), false);
/// ```
pub fn abstract_gt(first: &Value, second: &Value) -> bool {
    match (to_primitive(first, PrimitiveHint::Number), to_primitive(second, PrimitiveHint::Number)) {
        (Primitive::String(f), Primitive::String(s)) => { f > s },
        (Primitive::Number(f), Primitive::Number(s)) => { f > s },
        (Primitive::String(f), Primitive::Number(s)) => {
            if let Some(f) = str_to_number(f) { f > s } else { false }
        },
        (Primitive::Number(f), Primitive::String(s)) => {
            if let Some(s) = str_to_number(s) { f > s } else { false }
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use serde_json::json;

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
        assert_eq!(&to_string(&json!(1.0)), "1");
        assert_eq!(&to_string(&json!(1)), "1");
    }
}

#[cfg(test)]
mod test_abstract_eq {

    use super::*;
    use serde_json::json;

    #[test]
    fn test_nulls_eq() {
        assert!(abstract_eq(&json!(null), &json!(null)));
    }

    #[test]
    fn test_numbers_eq() {
        assert!(abstract_eq(&json!(1), &json!(1)));
        assert!(abstract_eq(&json!(1), &json!(1.0)));
        assert!(abstract_eq(&json!(1.0), &json!(1)));
        assert!(abstract_eq(&json!(1.0), &json!(1.0)));
        assert!(abstract_eq(&json!(0), &json!(-0)));
        assert!(abstract_eq(&json!(-0), &json!(0)));
    }

    #[test]
    fn test_numbers_integers_ne() {
        assert!(!abstract_eq(&json!(0), &json!(1)));
        assert!(!abstract_eq(&json!(1.000001), &json!(1)));
    }

    #[test]
    fn test_strings_eq() {
        assert!(abstract_eq(&json!("foo"), &json!("foo")));
        assert!(abstract_eq(&json!(""), &json!("")));
    }

    #[test]
    fn test_strings_ne() {
        assert!(!abstract_eq(&json!("foos"), &json!("foo")));
        assert!(!abstract_eq(&json!(""), &json!("0")));
    }

    #[test]
    fn test_bools_eq() {
        assert!(abstract_eq(&json!(true), &json!(true)));
        assert!(abstract_eq(&json!(false), &json!(false)));
    }

    #[test]
    fn test_bools_ne() {
        assert!(!abstract_eq(&json!(true), &json!(false)));
        assert!(!abstract_eq(&json!(false), &json!(true)));
    }

    #[test]
    /// Now we get into the fun JS
    fn test_number_string_eq() {
        assert!(abstract_eq(&json!(1), &json!("1")));
        assert!(abstract_eq(&json!(1), &json!("1.0")));
        assert!(abstract_eq(&json!(1.0), &json!("1.0")));
        assert!(abstract_eq(&json!(1.0), &json!("1")));
        assert!(abstract_eq(&json!(0), &json!("")));
        assert!(abstract_eq(&json!(0), &json!("0")));
        assert!(abstract_eq(&json!(0), &json!("-0")));
        assert!(abstract_eq(&json!(0), &json!("+0")));
        assert!(abstract_eq(&json!(-1), &json!("-1")));
        assert!(abstract_eq(&json!(-1.0), &json!("-1")));
    }

    #[test]
    fn test_number_string_ne() {
        assert!(!abstract_eq(&json!(0), &json!("1")));
        assert!(!abstract_eq(&json!(1), &json!("")));
        assert!(!abstract_eq(&json!(1), &json!("1.001")));
        assert!(!abstract_eq(&json!(1.0), &json!("2")));
    }

    #[test]
    fn test_string_number_eq() {
        assert!(abstract_eq(&json!("1"), &json!(1)));
        assert!(abstract_eq(&json!("1.0"), &json!(1)));
        assert!(abstract_eq(&json!("1.0"), &json!(1.0)));
        assert!(abstract_eq(&json!("1"), &json!(1.0)));
        assert!(abstract_eq(&json!(""), &json!(0)));
        assert!(abstract_eq(&json!("0"), &json!(0)));
        assert!(abstract_eq(&json!("-0"), &json!(0)));
        assert!(abstract_eq(&json!("+0"), &json!(0)));
        assert!(abstract_eq(&json!("-1"), &json!(-1)));
        assert!(abstract_eq(&json!("-1"), &json!(-1.0)));
    }

    #[test]
    fn test_string_number_ne() {
        assert!(!abstract_eq(&json!("1"), &json!(0)));
        assert!(!abstract_eq(&json!(""), &json!(1)));
        assert!(!abstract_eq(&json!("1.001"), &json!(1)));
        assert!(!abstract_eq(&json!("2"), &json!(1.0)));
    }

    #[test]
    fn test_bool_other_eq() {
        assert!(abstract_eq(&json!(true), &json!(1)));
        assert!(abstract_eq(&json!(true), &json!("1")));
        assert!(abstract_eq(&json!(true), &json!("1.0")));
        assert!(abstract_eq(&json!(true), &json!([1])));
        assert!(abstract_eq(&json!(true), &json!(["1"])));
        assert!(abstract_eq(&json!(false), &json!(0)));
        assert!(abstract_eq(&json!(false), &json!([])));
        assert!(abstract_eq(&json!(false), &json!([0])));
        assert!(abstract_eq(&json!(false), &json!("")));
        assert!(abstract_eq(&json!(false), &json!("0")));
    }

    #[test]
    fn test_bool_other_ne() {
        assert!(!abstract_eq(&json!(true), &json!([1, 2])));
        assert!(!abstract_eq(&json!(true), &json!([0])));
        assert!(!abstract_eq(&json!(true), &json!(null)));
        assert!(!abstract_eq(&json!(false), &json!({})));
        assert!(!abstract_eq(&json!(false), &json!(null)));
        assert!(!abstract_eq(&json!(false), &json!([0, 1])));
    }

    #[test]
    fn test_other_object_eq() {
        assert!(abstract_eq(&json!("[object Object]"), &json!({})));
        assert!(abstract_eq(&json!("[object Object]"), &json!({"a": "a"})));
    }

    #[test]
    fn test_other_object_ne() {
        assert!(!abstract_eq(&json!(""), &json!({})));
        assert!(!abstract_eq(&json!(0), &json!({})));
        assert!(!abstract_eq(&json!(1), &json!({})));
        assert!(!abstract_eq(&json!([]), &json!({})));
        assert!(!abstract_eq(&json!([1]), &json!({})));
        assert!(!abstract_eq(&json!([0]), &json!({})));
        assert!(!abstract_eq(&json!(null), &json!({})));
        assert!(!abstract_eq(&json!({}), &json!({})));
    }

    #[test]
    fn test_object_other_eq() {
        assert!(abstract_eq(&json!({}), &json!("[object Object]")));
        assert!(abstract_eq(&json!({"a": "a"}), &json!("[object Object]")));
    }

    #[test]
    fn test_object_other_ne() {
        assert!(!abstract_eq(&json!({}), &json!("")));
        assert!(!abstract_eq(&json!({}), &json!(0)));
        assert!(!abstract_eq(&json!({}), &json!(1)));
        assert!(!abstract_eq(&json!({}), &json!([])));
        assert!(!abstract_eq(&json!({}), &json!([1])));
        assert!(!abstract_eq(&json!({}), &json!([0])));
        assert!(!abstract_eq(&json!({}), &json!(null)));
        assert!(!abstract_eq(&json!({}), &json!({})));
    }

    #[test]
    fn test_other_array_eq() {
        assert!(abstract_eq(&json!(""), &json!([])));
        assert!(abstract_eq(&json!(""), &json!([null])));
        assert!(abstract_eq(&json!(","), &json!([null, null])));
        assert!(abstract_eq(&json!("1,2"), &json!([1, 2])));
        assert!(abstract_eq(&json!("a,b"), &json!(["a", "b"])));
        assert!(abstract_eq(&json!(0), &json!([])));
        assert!(abstract_eq(&json!(false), &json!([])));
        assert!(abstract_eq(&json!(true), &json!([1])));
    }

    #[test]
    fn test_other_array_ne() {
        assert!(!abstract_eq(&json!(null), &json!([])));
        assert!(!abstract_eq(&json!([]), &json!([])));
        assert!(!abstract_eq(&json!({}), &json!([])));
    }

    #[test]
    fn test_array_other_eq() {
        assert!(abstract_eq(&json!([]), &json!("")));
        assert!(abstract_eq(&json!([null]), &json!("")));
        assert!(abstract_eq(&json!([null, null]), &json!(",")));
        assert!(abstract_eq(&json!([1, 2]), &json!("1,2")));
        assert!(abstract_eq(&json!(["a", "b"]), &json!("a,b")));
        assert!(abstract_eq(&json!([]), &json!(0)));
        assert!(abstract_eq(&json!([]), &json!(false)));
        assert!(abstract_eq(&json!([1]), &json!(true)));
    }

    #[test]
    fn test_array_other_ne() {
        assert!(!abstract_eq(&json!([]), &json!(null)));
        assert!(!abstract_eq(&json!([]), &json!([])));
        assert!(!abstract_eq(&json!([]), &json!({})));
    }
}

#[cfg(test)]
mod test_strict_eq {

    use super::*;
    use serde_json::json;

    #[test]
    fn test_strict_eq() {
        assert!(strict_eq(&json!(""), &json!("")));
        assert!(strict_eq(&json!("foo"), &json!("foo")));
        assert!(strict_eq(&json!(1), &json!(1)));
        assert!(strict_eq(&json!(1), &json!(1.0)));
        assert!(strict_eq(&json!(null), &json!(null)));
        assert!(strict_eq(&json!(true), &json!(true)));
        assert!(strict_eq(&json!(false), &json!(false)));
    }

    #[test]
    fn test_strict_eq_same_obj() {
        let obj = json!({});
        assert!(strict_eq(&obj, &obj))
    }

    #[test]
    fn test_strict_ne() {
        assert!(!strict_eq(&json!({}), &json!({})));
        assert!(!strict_eq(&json!({"a": "a"}), &json!({"a": "a"})));
        assert!(!strict_eq(&json!([]), &json!([])));
        assert!(!strict_eq(&json!("foo"), &json!("noop")));
        assert!(!strict_eq(&json!(1), &json!(2)));
        assert!(!strict_eq(&json!(0), &json!([])));
        assert!(!strict_eq(&json!(0), &json!([0])));
        assert!(!strict_eq(&json!(false), &json!(null)));
        assert!(!strict_eq(&json!(true), &json!(false)));
        assert!(!strict_eq(&json!(false), &json!(true)));
        assert!(!strict_eq(&json!(false), &json!([])));
        assert!(!strict_eq(&json!(false), &json!("")));
    }
}
