//! Boolean Logic Operations

use serde_json::Value;

use crate::error::Error;
use crate::value::{Evaluated, Parsed};
use crate::NULL;

/// Implement the "if" operator
///
/// The base case works like: [condition, true, false]
/// However, it can lso work like:
///     [condition, true, condition2, true2, false2]
///     for an if/elseif/else type of operation
pub fn if_(data: &Value, args: &Vec<&Value>) -> Result<Value, Error> {
    // Special case incorrect arguments. These are not defined in the
    // specification, but they are defined in the test cases.
    match args.len() {
        0 => {
            return Ok(NULL);
        }
        // It's not totally clear to me why this would be the behavior,
        // rather than returning NULL regardless of how the single argument
        // evaluates, but this is I can gather is the expected behavior
        // from the tests.
        1 => {
            let parsed = Parsed::from_value(args[0])?;
            let evaluated = parsed.evaluate(&data)?;
            return Ok(evaluated.into());
        }
        _ => {}
    }

    args.into_iter()
        .enumerate()
        // Our accumulator is:
        //  - last conditional evaluation value,
        //  - whether that evaluation is truthy,
        //  - whether we know we should return without further evaluation
        .fold(Ok((NULL, false, false)), |last_res, (i, val)| {
            let (last_eval, was_truthy, should_return) = last_res?;
            // We hit a final value already
            if should_return {
                Ok((last_eval, was_truthy, should_return))
            }
            // Potential false-value, initial evaluation, or else-if clause
            else if i % 2 == 0 {
                let parsed = Parsed::from_value(val)?;
                let eval = parsed.evaluate(data)?;
                let is_truthy = match eval {
                    Evaluated::New(ref v) => truthy(v),
                    Evaluated::Raw(v) => truthy(v),
                };
                // We're not sure we're the return value, so don't
                // force a return.
                Ok((eval.into(), is_truthy, false))
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
                    // Return a null for the last eval to handle cases
                    // where there is an incorrect number of arguments.
                    Ok((NULL, was_truthy, should_return))
                }
            }
        })
        .map(|rv| rv.0)
}

/// Perform short-circuiting or evaluation
pub fn or(data: &Value, args: &Vec<&Value>) -> Result<Value, Error> {
    enum OrResult {
        Uninitialized,
        Truthy(Value),
        Current(Value),
    }

    let eval =
        args.into_iter()
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
pub fn and(data: &Value, args: &Vec<&Value>) -> Result<Value, Error> {
    enum AndResult {
        Uninitialized,
        Falsey(Value),
        Current(Value),
    }

    let eval =
        args.into_iter()
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

pub fn truthy_from_evaluated(evaluated: &Evaluated) -> bool {
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
