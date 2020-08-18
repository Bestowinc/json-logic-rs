//! Array Operations
//!
//! Note that some array operations also operate on strings as arrays
//! of characters.

use serde_json::{Map, Value};

use crate::error::Error;
use crate::op::logic;
use crate::value::{Evaluated, Parsed};

/// Map an operation onto values
pub fn map(data: &Value, args: &Vec<&Value>) -> Result<Value, Error> {
    let (items, expression) = (args[0], args[1]);

    let _parsed = Parsed::from_value(items)?;
    let evaluated_items = _parsed.evaluate(data)?;

    let values: Vec<&Value> = match evaluated_items {
        Evaluated::New(Value::Array(ref vals)) => vals.iter().collect(),
        Evaluated::Raw(Value::Array(vals)) => vals.iter().collect(),
        // null is treated as an empty array in the reference tests,
        // for whatever reason
        Evaluated::New(Value::Null) => vec![],
        Evaluated::Raw(Value::Null) => vec![],
        _ => {
            return Err(Error::InvalidArgument {
                value: args[0].clone(),
                operation: "map".into(),
                reason: format!(
                    "First argument to map must evaluate to an array. Got {:?}",
                    evaluated_items
                ),
            })
        }
    };

    let parsed_expression = Parsed::from_value(expression)?;

    values
        .iter()
        .map(|v| parsed_expression.evaluate(v).map(Value::from))
        .collect::<Result<Vec<Value>, Error>>()
        .map(Value::Array)
}

/// Filter values by some predicate
pub fn filter(data: &Value, args: &Vec<&Value>) -> Result<Value, Error> {
    let (items, expression) = (args[0], args[1]);

    let _parsed = Parsed::from_value(items)?;
    let evaluated_items = _parsed.evaluate(data)?;

    let values: Vec<Value> = match evaluated_items {
        Evaluated::New(Value::Array(vals)) => vals,
        Evaluated::Raw(Value::Array(vals)) => {
            vals.into_iter().map(|v| v.clone()).collect()
        }
        // null is treated as an empty array in the reference tests,
        // for whatever reason
        Evaluated::New(Value::Null) => vec![],
        Evaluated::Raw(Value::Null) => vec![],
        _ => {
            return Err(Error::InvalidArgument {
                value: args[0].clone(),
                operation: "map".into(),
                reason: format!(
                    "First argument to filter must evaluate to an array. Got {:?}",
                    evaluated_items
                ),
            })
        }
    };

    let parsed_expression = Parsed::from_value(expression)?;

    let value_vec: Vec<Value> = Vec::with_capacity(values.len());
    values
        .into_iter()
        .fold(Ok(value_vec), |acc, cur| {
            let mut filtered = acc?;
            let predicate = parsed_expression.evaluate(&cur)?;

            match logic::truthy_from_evaluated(&predicate) {
                true => {
                    filtered.push(cur);
                    Ok(filtered)
                }
                false => Ok(filtered),
            }
        })
        .map(Value::Array)
}

/// Reduce values into a single result
///
/// Note this differs from the reference implementation of jsonlogic
/// (but not the spec), in that it evaluates the initializer as a
/// jsonlogic expression rather than a raw value.
pub fn reduce(data: &Value, args: &Vec<&Value>) -> Result<Value, Error> {
    let (items, expression, initializer) = (args[0], args[1], args[2]);

    let _parsed_items = Parsed::from_value(items)?;
    let evaluated_items = _parsed_items.evaluate(data)?;

    let _parsed_initializer = Parsed::from_value(initializer)?;
    let evaluated_initializer = _parsed_initializer.evaluate(data)?;

    let values: Vec<Value> = match evaluated_items {
        Evaluated::New(Value::Array(vals)) => vals,
        Evaluated::Raw(Value::Array(vals)) => vals.iter().map(|v| v.clone()).collect(),
        // null is treated as an empty array in the reference tests,
        // for whatever reason
        Evaluated::New(Value::Null) => vec![],
        Evaluated::Raw(Value::Null) => vec![],
        _ => {
            return Err(Error::InvalidArgument {
                value: args[0].clone(),
                operation: "map".into(),
                reason: format!(
                    "First argument to filter must evaluate to an array. Got {:?}",
                    evaluated_items
                ),
            })
        }
    };

    let parsed_expression = Parsed::from_value(expression)?;

    values
        .into_iter()
        .fold(Ok(Value::from(evaluated_initializer)), |acc, cur| {
            let accumulator = acc?;
            let mut data = Map::with_capacity(2);
            data.insert("current".into(), cur);
            data.insert("accumulator".into(), accumulator);

            parsed_expression
                .evaluate(&Value::Object(data))
                .map(Value::from)
        })
}

/// Return whether all members of an array or string satisfy a predicate.
///
/// The predicate does not need to return true or false explicitly. Its
/// return is evaluated using the "truthy" definition specified in the
/// jsonlogic spec.
pub fn all(data: &Value, args: &Vec<&Value>) -> Result<Value, Error> {
    let (first_arg, second_arg) = (args[0], args[1]);

    // The first argument must be an array of values or a string of chars
    // We won't bother parsing yet if the value is anything other than
    // an object, because we can short-circuit this function if any of
    // the items fail to match the predicate. However, we will parse
    // if it's an object, in case it evaluates to a string or array, which
    // we will then pass on

    let _new_item: Value;
    let potentially_evaled_first_arg = match first_arg {
        Value::Object(_) => {
            let parsed = Parsed::from_value(first_arg)?;
            let evaluated = parsed.evaluate(data)?;
            _new_item = evaluated.into();
            &_new_item
        }
        _ => first_arg,
    };

    let _new_arr: Vec<Value>;
    let items = match potentially_evaled_first_arg {
        Value::Array(items) => items,
        Value::String(string) => {
            _new_arr = string
                .chars()
                .into_iter()
                .map(|c| Value::String(c.to_string()))
                .collect();
            &_new_arr
        }
        Value::Null => {
            _new_arr = Vec::new();
            &_new_arr
        }
        _ => {
            return Err(Error::InvalidArgument {
                value: first_arg.clone(),
                operation: "all".into(),
                reason: format!(
                "First argument to all must evaluate to an array, string, or null, got {}",
                potentially_evaled_first_arg
            ),
            })
        }
    };

    // Special-case the empty array, since it for some reason is specified
    // to return false.
    if items.len() == 0 {
        return Ok(Value::Bool(false));
    }

    // Note we _expect_ the predicate to be an operator, but it doesn't
    // necessarily have to be. all([1, 2, 3], 1) is a valid operation,
    // returning 1 for each of the items and thus evaluating to true.
    let predicate = Parsed::from_value(second_arg)?;

    let result = items.into_iter().fold(Ok(true), |acc, i| {
        acc.and_then(|res| {
            // "Short-circuit": return false if the previous eval was false
            if !res {
                return Ok(false);
            };
            let _parsed_item = Parsed::from_value(i)?;
            // Evaluate each item as we go, in case we can short-circuit
            let evaluated_item = _parsed_item.evaluate(data)?;
            Ok(logic::truthy_from_evaluated(
                &predicate.evaluate(&evaluated_item.into())?,
            ))
        })
    })?;

    Ok(Value::Bool(result))
}

/// Return whether some members of an array or string satisfy a predicate.
///
/// The predicate does not need to return true or false explicitly. Its
/// return is evaluated using the "truthy" definition specified in the
/// jsonlogic spec.
pub fn some(data: &Value, args: &Vec<&Value>) -> Result<Value, Error> {
    let (first_arg, second_arg) = (args[0], args[1]);

    // The first argument must be an array of values or a string of chars
    // We won't bother parsing yet if the value is anything other than
    // an object, because we can short-circuit this function if any of
    // the items fail to match the predicate. However, we will parse
    // if it's an object, in case it evaluates to a string or array, which
    // we will then pass on

    let _new_item: Value;
    let potentially_evaled_first_arg = match first_arg {
        Value::Object(_) => {
            let parsed = Parsed::from_value(first_arg)?;
            let evaluated = parsed.evaluate(data)?;
            _new_item = evaluated.into();
            &_new_item
        }
        _ => first_arg,
    };

    let _new_arr: Vec<Value>;
    let items = match potentially_evaled_first_arg {
        Value::Array(items) => items,
        Value::String(string) => {
            _new_arr = string
                .chars()
                .into_iter()
                .map(|c| Value::String(c.to_string()))
                .collect();
            &_new_arr
        }
        Value::Null => {
            _new_arr = Vec::new();
            &_new_arr
        }
        _ => {
            return Err(Error::InvalidArgument {
                value: first_arg.clone(),
                operation: "all".into(),
                reason: format!(
                "First argument must evaluate to an array, a string, or null, got {}",
                potentially_evaled_first_arg
            ),
            })
        }
    };

    // Special-case the empty array, since it for some reason is specified
    // to return false.
    if items.len() == 0 {
        return Ok(Value::Bool(false));
    }

    // Note we _expect_ the predicate to be an operator, but it doesn't
    // necessarily have to be. all([1, 2, 3], 1) is a valid operation,
    // returning 1 for each of the items and thus evaluating to true.
    let predicate = Parsed::from_value(second_arg)?;

    let result = items.into_iter().fold(Ok(false), |acc, i| {
        acc.and_then(|res| {
            // "Short-circuit": return false if the previous eval was false
            if res {
                return Ok(true);
            };
            let _parsed_item = Parsed::from_value(i)?;
            // Evaluate each item as we go, in case we can short-circuit
            let evaluated_item = _parsed_item.evaluate(data)?;
            Ok(logic::truthy_from_evaluated(
                &predicate.evaluate(&evaluated_item.into())?,
            ))
        })
    })?;

    Ok(Value::Bool(result))
}

/// Return whether no members of an array or string satisfy a predicate.
///
/// The predicate does not need to return true or false explicitly. Its
/// return is evaluated using the "truthy" definition specified in the
/// jsonlogic spec.
pub fn none(data: &Value, args: &Vec<&Value>) -> Result<Value, Error> {
    some(data, args).and_then(|had_some| match had_some {
        Value::Bool(res) => Ok(Value::Bool(!res)),
        _ => Err(Error::UnexpectedError(
            "Unexpected return type from op_some".into(),
        )),
    })
}

/// Merge one to n arrays, flattening them by one level.
///
/// Values that are not arrays are (effectively) converted to arrays
/// before flattening.
pub fn merge(items: &Vec<&Value>) -> Result<Value, Error> {
    let rv_vec: Vec<Value> = Vec::new();
    Ok(Value::Array(items.into_iter().fold(
        rv_vec,
        |mut acc, i| {
            match i {
                Value::Array(i_vals) => {
                    i_vals.into_iter().for_each(|val| acc.push((*val).clone()));
                }
                _ => acc.push((**i).clone()),
            };
            acc
        },
    )))
}

/// Perform containment checks with "in"
// TODO: make this a lazy operator, since we don't need to parse things
// later on in the list if we find something that matches early.
pub fn in_(items: &Vec<&Value>) -> Result<Value, Error> {
    let needle = items[0];
    let haystack = items[1];

    match haystack {
        // Note: our containment check for array values is actually a bit
        // more robust than JS. This by default does array equality (e.g.
        // `[[1,2], [3,4]].contains([1,2]) == true`), as well as object
        // equality (e.g. `[{"a": "b"}].contains({"a": "b"}) == true`).
        // Given that anyone relying on this behavior in the existing jsonlogic
        // implementation is relying on broken, undefined behavior, it seems
        // okay to update that behavior to work in a more intuitive way.
        Value::Null => Ok(Value::Bool(false)),
        Value::Array(possibles) => Ok(Value::Bool(possibles.contains(needle))),
        Value::String(haystack_string) => {
            // Note: the reference implementation uses the regular old
            // String.prototype.indexOf() function to check for containment,
            // but that does JS type coercion, leading to crazy things like
            // `"foo[object Object]".indexOf({}) === 3`. Since the MDN docs
            // _explicitly_ say that the argument to indexOf should be a string,
            // we're going to take the same stance here, and throw an error
            // if the needle is a non-string for a haystack that's a string.
            let needle_string =
                match needle {
                    Value::String(needle_string) => needle_string,
                    _ => return Err(Error::InvalidArgument {
                        value: needle.clone(),
                        operation: "in".into(),
                        reason:
                            "If second argument is a string, first argument must also be a string."
                                .into(),
                    }),
                };
            Ok(Value::Bool(haystack_string.contains(needle_string)))
        }
        _ => Err(Error::InvalidArgument {
            value: haystack.clone(),
            operation: "in".into(),
            reason: "Second argument must be an array or a string".into(),
        }),
    }
}
