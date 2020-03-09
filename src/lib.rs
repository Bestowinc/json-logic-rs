use serde_json::Value;

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

fn get_operator<Op, T>(operator: Op) -> impl Fn(Vec<T>) -> Value
    where Op: AsRef<str>, T: AsRef<Value>
{
    let op = operator.as_ref();
    match op {
        "==" => |vals: Vec<T>| { Value::Bool(true) },
        _ => panic!("bad operator")
    }
}


/// Run JSONLogic for the given rule and data.
///
// pub fn jsonlogic<R: AsRef<Value>, D: AsRef<Value::Object>>(rule: R, data: D) -> Value {

// }


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
