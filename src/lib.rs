use serde_json::{ Value, Number };
use std::str::FromStr;
use std::f64;
use std::i64;


/// Compare values in the JavaScript `==` style
///
/// Implements the Abstract Equality Comparison algorithm (`==` in JS)
/// as defined [here](https://www.ecma-international.org/ecma-262/5.1/#sec-11.9.3).
///
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
            x == y
            // iv. If x is +0 and y is −0, return true.
            //     - with serde's Number, this is handled by the above
            // v. If x is −0 and y is +0, return true.
            //    - same here
            // vi. Return false.
            //     - done!
        },
        //   d. If Type(x) is String, then return true if x and y are exactly
        //      the same sequence of characters (same length and same characters
        //      in corresponding positions). Otherwise, return false.
        (Value::String(x), Value::String(y)) => { x == y },
        //   e. If Type(x) is Boolean, return true if x and y are both true
        //      or both false. Otherwise, return false.
        (Value::Bool(x), Value::Bool(y)) => { x == y },
        //   f. Return true if x and y refer to the same object. Otherwise, return false.
        //      - not applicable to comparisons from JSON
        // 2. If x is null and y is undefined, return true.
        //    - not applicable to JSON b/c there is no undefined
        // 3. If x is undefined and y is null, return true.
        //    - not applicable to JSON b/c there is no undefined
        // 4. If Type(x) is Number and Type(y) is String, return the result of
        //    the comparison x == ToNumber(y).
        (Value::Number(x), Value::String(y)) => {
            match f64::from_str(&y) {
                Ok(y_number) => {
                    match x.as_f64() {
                        Some(x_number) => { x_number == y_number }
                        None => false
                    }
                },
                _ => false,
            }
        },
        // 5. If Type(x) is String and Type(y) is Number, return the result
        //    of the comparison ToNumber(x) == y.
        (Value::String(x), Value::Number(y)) => {
            match f64::from_str(&x) {
                Ok(x_number) => {
                    match y.as_f64() {
                        Some(y_number) => { y_number == x_number }
                        None => false
                    }
                },
                _ => false,
            }
        },
        // 6. If Type(x) is Boolean, return the result of the comparison ToNumber(x) == y.
        (Value::Bool(x), _) => {
            match x {
                true => {
                    Number::from_f64(1 as f64).map(|num| {
                        let value = Value::Number(num);
                        abstract_eq(&value, second)
                    } ).unwrap_or(false)
                },
                false => {
                    Number::from_f64(0 as f64).map(|num| {
                        let value = Value::Number(num);
                        abstract_eq(&value, second)
                    } ).unwrap_or(false)
                },
            }
        },
        // 7. If Type(y) is Boolean, return the result of the comparison x == ToNumber(y).
        (_, Value::Bool(y)) => {
            match y {
                true => {
                    Number::from_f64(1 as f64).map(|num| {
                        let value = Value::Number(num);
                        abstract_eq(first, &value)
                    } ).unwrap_or(false)
                },
                false => {
                    Number::from_f64(0 as f64).map(|num| {
                        let value = Value::Number(num);
                        abstract_eq(first, &value)
                    } ).unwrap_or(false)
                },
            }
        },
        _ => false,
    }
}


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
