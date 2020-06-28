//! Numeric Operations

use serde_json::Value;

use crate::error::Error;
use crate::js_op;
use crate::value::to_number_value;

fn compare<F>(func: F, items: &Vec<&Value>) -> Result<Value, Error>
where
    F: Fn(&Value, &Value) -> bool,
{
    if items.len() == 2 {
        Ok(Value::Bool(func(items[0], items[1])))
    } else {
        Ok(Value::Bool(
            func(items[0], items[1]) && func(items[1], items[2]),
        ))
    }
}

/// Do < for either 2 or 3 values
pub fn lt(items: &Vec<&Value>) -> Result<Value, Error> {
    compare(js_op::abstract_lt, items)
}

/// Do <= for either 2 or 3 values
pub fn lte(items: &Vec<&Value>) -> Result<Value, Error> {
    compare(js_op::abstract_lte, items)
}

/// Do > for either 2 or 3 values
pub fn gt(items: &Vec<&Value>) -> Result<Value, Error> {
    compare(js_op::abstract_gt, items)
}

/// Do >= for either 2 or 3 values
pub fn gte(items: &Vec<&Value>) -> Result<Value, Error> {
    compare(js_op::abstract_gte, items)
}

/// Perform subtraction or convert a number to a negative
pub fn minus(items: &Vec<&Value>) -> Result<Value, Error> {
    let value = if items.len() == 1 {
        js_op::to_negative(items[0])?
    } else {
        js_op::abstract_minus(items[0], items[1])?
    };
    to_number_value(value)
}
