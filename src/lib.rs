use serde_json;
use serde_json::Value;

mod error;
// TODO consider whether this should be public; move doctests if so
pub mod js_op;
mod op;
mod value;

use error::Error;
use value::{Evaluated, Parsed};

const NULL: Value = Value::Null;

trait Parser<'a>: Sized + Into<Value> {
    fn from_value(value: &'a Value) -> Result<Option<Self>, Error>;
    fn evaluate(&self, data: &'a Value) -> Result<Evaluated, Error>;
}

#[cfg(feature = "wasm")]
pub mod javascript_iface {
    use serde_json::Value;
    use wasm_bindgen::prelude::*;

    fn to_serde_value(js_value: JsValue) -> Result<Value, JsValue> {
        // If we're passed a string, try to parse it as JSON. If we fail,
        // we will just return a Value::String, since that's a valid thing
        // to pass in to JSONLogic.
        // js_value
        if js_value.is_string() {
            let js_string = js_value.as_string().expect(
                "Could not convert value to string, even though it was checked to be a string."
            );
            serde_json::from_str(&js_string).or(Ok(Value::String(js_string)))
        } else {
            // If we're passed anything else, convert it directly to a serde Value.
            js_value
                .into_serde::<Value>()
                .map_err(|err| format!("{}", err))
                .map_err(JsValue::from)
        }
    }

    #[wasm_bindgen]
    pub fn apply(value: JsValue, data: JsValue) -> Result<JsValue, JsValue> {
        let value_json = to_serde_value(value)?;
        let data_json = to_serde_value(data)?;

        let res = crate::apply(&value_json, &data_json)
            .map_err(|err| format!("{}", err))
            .map_err(JsValue::from)?;

        JsValue::from_serde(&res)
            .map_err(|err| format!("{}", err))
            .map_err(JsValue::from)
    }
}

#[cfg(feature = "python")]
pub mod python_iface {
    use cpython::exc::ValueError;
    use cpython::{py_fn, py_module_initializer, PyErr, PyResult, Python};

    py_module_initializer!(jsonlogic, initjsonlogic, PyInit_jsonlogic, |py, m| {
        m.add(py, "__doc__", "Python bindings for json-logic-rs")?;
        m.add(py, "apply", py_fn!(py, py_apply(value: &str, data: &str)))?;
        Ok(())
    });

    fn apply(value: &str, data: &str) -> Result<String, String> {
        let value_json =
            serde_json::from_str(value).map_err(|err| format!("{}", err))?;
        let data_json = serde_json::from_str(data).map_err(|err| format!("{}", err))?;

        crate::apply(&value_json, &data_json)
            .map_err(|err| format!("{}", err))
            .map(|res| res.to_string())
    }

    fn py_apply(py: Python, value: &str, data: &str) -> PyResult<String> {
        apply(value, data).map_err(|err| PyErr::new::<ValueError, _>(py, err))
    }
}

/// Run JSONLogic for the given operation and data.
///
pub fn apply(value: &Value, data: &Value) -> Result<Value, Error> {
    let parsed = Parsed::from_value(&value)?;
    parsed.evaluate(data).map(Value::from)
}

#[cfg(test)]
mod jsonlogic_tests {
    use super::*;
    use serde_json::json;

    fn no_op_cases() -> Vec<(Value, Value, Result<Value, ()>)> {
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
        ]
    }

    fn abstract_eq_cases() -> Vec<(Value, Value, Result<Value, ()>)> {
        vec![
            (json!({"==": [1, 1]}), json!({}), Ok(json!(true))),
            (json!({"==": [1, 2]}), json!({}), Ok(json!(false))),
            (json!({"==": [1, "1"]}), json!({}), Ok(json!(true))),
            (
                json!({"==": [{}, "[object Object]"]}),
                json!({}),
                Ok(json!(true)),
            ),
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
            // Wrong number of arguments
            (json!({"==": [1]}), json!({}), Err(())),
            (json!({"==": [1, 1, 1]}), json!({}), Err(())),
        ]
    }

    fn abstract_ne_cases() -> Vec<(Value, Value, Result<Value, ()>)> {
        vec![
            (json!({"!=": [1, 1]}), json!({}), Ok(json!(false))),
            (json!({"!=": [1, 2]}), json!({}), Ok(json!(true))),
            (json!({"!=": [1, "1"]}), json!({}), Ok(json!(false))),
            (
                json!({"!=": [{}, "[object Object]"]}),
                json!({}),
                Ok(json!(false)),
            ),
            (
                json!({"!=": [{"!=": [1, 2]}, 1]}),
                json!({}),
                Ok(json!(false)),
            ),
            // Wrong number of arguments
            (json!({"!=": [1]}), json!({}), Err(())),
            (json!({"!=": [1, 1, 1]}), json!({}), Err(())),
        ]
    }

    fn strict_eq_cases() -> Vec<(Value, Value, Result<Value, ()>)> {
        vec![
            (json!({"===": [1, 1]}), json!({}), Ok(json!(true))),
            (json!({"===": [1, 2]}), json!({}), Ok(json!(false))),
            (json!({"===": [1, "1"]}), json!({}), Ok(json!(false))),
            (
                json!({"===": [{}, "[object Object]"]}),
                json!({}),
                Ok(json!(false)),
            ),
            (json!({"===": [1, [1]]}), json!({}), Ok(json!(false))),
            (json!({"===": [1, true]}), json!({}), Ok(json!(false))),
            // Recursive evaluation
            (
                json!({"===": [true, {"===": [1, 1]}]}),
                json!({}),
                Ok(json!(true)),
            ),
            (
                json!({"===": [{"===": [{"===": [1, 1]}, true]}, {"===": [1, 1]}]}),
                json!({}),
                Ok(json!(true)),
            ),
            // Wrong number of arguments
            (json!({"===": [1]}), json!({}), Err(())),
            (json!({"===": [1, 1, 1]}), json!({}), Err(())),
        ]
    }

    fn strict_ne_cases() -> Vec<(Value, Value, Result<Value, ()>)> {
        vec![
            (json!({"!==": [1, 1]}), json!({}), Ok(json!(false))),
            (json!({"!==": [1, 2]}), json!({}), Ok(json!(true))),
            (json!({"!==": [1, "1"]}), json!({}), Ok(json!(true))),
            (
                json!({"!==": [{}, "[object Object]"]}),
                json!({}),
                Ok(json!(true)),
            ),
            (json!({"!==": [1, [1]]}), json!({}), Ok(json!(true))),
            (json!({"!==": [1, true]}), json!({}), Ok(json!(true))),
            // Recursive evaluation
            (
                json!({"!==": [true, {"!==": [1, 1]}]}),
                json!({}),
                Ok(json!(true)),
            ),
            (
                json!({"!==": [{"!==": [{"!==": [1, 1]}, false]}, {"!==": [1, 1]}]}),
                json!({}),
                Ok(json!(false)),
            ),
            // Wrong number of arguments
            (json!({"!==": [1]}), json!({}), Err(())),
            (json!({"!==": [1, 1, 1]}), json!({}), Err(())),
        ]
    }

    fn var_cases() -> Vec<(Value, Value, Result<Value, ()>)> {
        vec![
            // Variable substitution
            (
                json!({"var": "foo"}),
                json!({"foo": "bar"}),
                Ok(json!("bar")),
            ),
            // Index into array data
            (json!({"var": 1}), json!(["foo", "bar"]), Ok(json!("bar"))),
            // Absent variable
            (json!({"var": "foo"}), json!({}), Ok(json!(null))),
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
            // Dotted variable with nested array access
            (
                json!({"var": "foo.1"}),
                json!({"foo": ["bar", "baz", "pop"]}),
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

    fn missing_cases() -> Vec<(Value, Value, Result<Value, ()>)> {
        vec![
            // "missing" data operator
            (
                json!({"missing": ["a", "b"]}),
                json!({"a": 1, "b": 2}),
                Ok(json!([])),
            ),
            (
                json!({"missing": ["a", "b"]}),
                json!({"a": 1}),
                Ok(json!(["b"])),
            ),
            (json!({"missing": [1, 5]}), json!([1, 2, 3]), Ok(json!([5]))),
        ]
    }

    fn missing_some_cases() -> Vec<(Value, Value, Result<Value, ()>)> {
        vec![
            // "missing_some" data operator
            (
                json!({"missing_some": [1, ["a", "b"]]}),
                json!({"a": 1, "b": 2}),
                Ok(json!([])),
            ),
            (
                json!({"missing_some": [1, ["a", "b", "c"]]}),
                json!({"a": 1, "b": 2}),
                Ok(json!([])),
            ),
            (
                json!({"missing_some": [2, ["a", "b", "c"]]}),
                json!({"a": 1}),
                Ok(json!(["b", "c"])),
            ),
        ]
    }

    fn if_cases() -> Vec<(Value, Value, Result<Value, ()>)> {
        vec![
            (
                json!({"if": [true, "true", "false"]}),
                json!({}),
                Ok(json!("true")),
            ),
            (
                json!({"if": [false, "true", "false"]}),
                json!({}),
                Ok(json!("false")),
            ),
            (
                json!({"if": [false, "true", true, "true2"]}),
                json!({}),
                Ok(json!("true2")),
            ),
            (
                json!({"if": [false, "true", false, "true2", "false2"]}),
                json!({}),
                Ok(json!("false2")),
            ),
            (
                json!({"if": [{"===": [1, 1]}, "true", "false"]}),
                json!({}),
                Ok(json!("true")),
            ),
            (
                json!({"if": [{"===": [1, 2]}, "true", "false"]}),
                json!({}),
                Ok(json!("false")),
            ),
            (
                json!({"if": [{"===": [1, 2]}, "true", {"===": [1, 1]}, "true2"]}),
                json!({}),
                Ok(json!("true2")),
            ),
            (
                json!({"if": [{"===": [1, 2]}, "true", {"===": [1, 2]}, "true2", "false2"]}),
                json!({}),
                Ok(json!("false2")),
            ),
        ]
    }

    fn or_cases() -> Vec<(Value, Value, Result<Value, ()>)> {
        vec![
            (json!({"or": [true]}), json!({}), Ok(json!(true))),
            (json!({"or": [false]}), json!({}), Ok(json!(false))),
            (json!({"or": [false, true]}), json!({}), Ok(json!(true))),
            (
                json!({"or": [false, true, false]}),
                json!({}),
                Ok(json!(true)),
            ),
            (json!({"or": [false, false, 12]}), json!({}), Ok(json!(12))),
            (
                json!({"or": [false, false, 12, 13, 14]}),
                json!({}),
                Ok(json!(12)),
            ),
            (
                json!({"or": [false, false, 0, 12]}),
                json!({}),
                Ok(json!(12)),
            ),
            (
                json!({"or": [false, {"===": [1, 1]}]}),
                json!({}),
                Ok(json!(true)),
            ),
            (
                json!({"or": [false, {"===": [{"var": "foo"}, 1]}]}),
                json!({"foo": 1}),
                Ok(json!(true)),
            ),
            (
                json!({"or": [false, {"===": [{"var": "foo"}, 1]}]}),
                json!({"foo": 2}),
                Ok(json!(false)),
            ),
        ]
    }

    fn and_cases() -> Vec<(Value, Value, Result<Value, ()>)> {
        vec![
            (json!({"and": [true]}), json!({}), Ok(json!(true))),
            (json!({"and": [false]}), json!({}), Ok(json!(false))),
            (json!({"and": [false, true]}), json!({}), Ok(json!(false))),
            (json!({"and": [true, false]}), json!({}), Ok(json!(false))),
            (json!({"and": [true, true]}), json!({}), Ok(json!(true))),
            (
                json!({"and": [false, true, false]}),
                json!({}),
                Ok(json!(false)),
            ),
            (json!({"and": [12, true, 0]}), json!({}), Ok(json!(0))),
            (
                json!({"and": [12, true, 0, 12, false]}),
                json!({}),
                Ok(json!(0)),
            ),
            (json!({"and": [true, true, 12]}), json!({}), Ok(json!(12))),
            (
                json!({"and": [{"===": [1, 1]}, false]}),
                json!({}),
                Ok(json!(false)),
            ),
            (
                json!({"and": [{"===": [{"var": "foo"}, 1]}, true]}),
                json!({"foo": 1}),
                Ok(json!(true)),
            ),
            (
                json!({"and": [{"===": [{"var": "foo"}, 1]}, true]}),
                json!({"foo": 2}),
                Ok(json!(false)),
            ),
        ]
    }

    fn map_cases() -> Vec<(Value, Value, Result<Value, ()>)> {
        vec![
            (
                json!({"map": [[1, 2, 3], {"*": [{"var": ""}, 2]}]}),
                json!(null),
                Ok(json!([2, 4, 6])),
            ),
            (
                json!({"map": [[], {"*": [{"var": ""}, 2]}]}),
                json!(null),
                Ok(json!([])),
            ),
            (
                json!({"map": [{"var": "vals"}, {"*": [{"var": ""}, 2]}]}),
                json!({"vals": [1, 2, 3]}),
                Ok(json!([2, 4, 6])),
            ),
            (
                json!({"map": [{"var": ""}, {"*": [{"var": ""}, 2]}]}),
                json!([1, 2, 3]),
                Ok(json!([2, 4, 6])),
            ),
            (
                json!({"map": [[true, 2, 0, [], {}], {"!!": [{"var": ""}]}]}),
                json!(null),
                Ok(json!([true, true, false, false, true])),
            ),
        ]
    }

    fn filter_cases() -> Vec<(Value, Value, Result<Value, ()>)> {
        vec![
            (
                json!({"filter": [[1, 2, 3], {"%": [{"var": ""}, 2]}]}),
                json!(null),
                Ok(json!([1, 3])),
            ),
            (
                json!({"filter": [[], {"%": [{"var": ""}, 2]}]}),
                json!(null),
                Ok(json!([])),
            ),
            (
                json!({"filter": [[2, 4, 6], {"%": [{"var": ""}, 2]}]}),
                json!(null),
                Ok(json!([])),
            ),
            (
                json!({"filter": [{"var": "vals"}, {"%": [{"var": ""}, 2]}]}),
                json!({"vals": [1, 2, 3]}),
                Ok(json!([1, 3])),
            ),
            (
                json!({"filter": [["aa", "bb", "aa"], {"===": [{"var": ""}, "aa"]}]}),
                json!(null),
                Ok(json!(["aa", "aa"])),
            ),
            (
                json!(
                    {
                        "filter": [
                            [1, 2, 3],
                            {"<": [
                                {"-": [{"var": ""}, 3]},
                                0
                            ]}
                        ]
                    }
                ),
                json!(null),
                Ok(json!([1, 2])),
            ),
        ]
    }

    fn reduce_cases() -> Vec<(Value, Value, Result<Value, ()>)> {
        vec![
            (
                json!(
                    {"reduce":[
                        [1, 2, 3, 4, 5],
                        {"+": [{"var":"current"}, {"var":"accumulator"}]},
                        0
                    ]}
                ),
                json!(null),
                Ok(json!(15)),
            ),
            (
                json!(
                    {"reduce":[
                        {"var": "vals"},
                        {"+": [{"var":"current"}, {"var":"accumulator"}]},
                        0
                    ]}
                ),
                json!({"vals": [1, 2, 3, 4, 5]}),
                Ok(json!(15)),
            ),
            (
                json!(
                    {"reduce":[
                        {"var": "vals"},
                        {"+": [{"var":"current"}, {"var":"accumulator"}]},
                        {"var": "init"}
                    ]}
                ),
                json!({"vals": [1, 2, 3, 4, 5], "init": 0}),
                Ok(json!(15)),
            ),
            (
                json!(
                    {"reduce":[
                        {"var": "vals"},
                        {"and":
                            [{"var": "accumulator"},
                             {"!!": [{"var": "current"}]}]
                        },
                        true,
                    ]}
                ),
                json!({"vals": [1, true, 10, "foo", 1, 1]}),
                Ok(json!(true)),
            ),
            (
                json!(
                    {"reduce":[
                        {"var": "vals"},
                        {"and":
                            [{"var": "accumulator"},
                             {"!!": [{"var": "current"}]}]
                        },
                        true,
                    ]}
                ),
                json!({"vals": [1, true, 10, "foo", 0, 1]}),
                Ok(json!(false)),
            ),
        ]
    }

    fn all_cases() -> Vec<(Value, Value, Result<Value, ()>)> {
        vec![
            // Invalid first arguments
            (json!({"all": [1, 1]}), json!({}), Err(())),
            (json!({"all": [{}, 1]}), json!({}), Err(())),
            (json!({"all": [false, 1]}), json!({}), Err(())),
            // Empty array/string/null
            (json!({"all": [[], 1]}), json!({}), Ok(json!(false))),
            (json!({"all": ["", 1]}), json!({}), Ok(json!(false))),
            (json!({"all": [null, 1]}), json!({}), Ok(json!(false))),
            // Constant predicate
            (json!({"all": [[1, 2], 1]}), json!({}), Ok(json!(true))),
            (json!({"all": [[1, 2], 0]}), json!({}), Ok(json!(false))),
            // Simple predicate
            (
                json!({"all": [[1, 2], {">": [{"var": ""}, 0]}]}),
                json!({}),
                Ok(json!(true)),
            ),
            (
                json!({"all": [[1, 2, -1], {">": [{"var": ""}, 0]}]}),
                json!({}),
                Ok(json!(false)),
            ),
            (
                json!({"all": ["aaaa", {"===": [{"var": ""}, "a"]}]}),
                json!({}),
                Ok(json!(true)),
            ),
            (
                json!({"all": ["aabaa", {"===": [{"var": ""}, "a"]}]}),
                json!({}),
                Ok(json!(false)),
            ),
            // First argument requires evaluation
            (
                json!({"all": [ {"var": "a"}, {"===": [{"var": ""}, "a"]} ]}),
                json!({"a": "a"}),
                Ok(json!(true)),
            ),
            // Expression in array
            (
                json!({"all": [[1, {"+": [1, 1]}], {">": [{"var": ""}, 0]}]}),
                json!({}),
                Ok(json!(true)),
            ),
            (
                json!({"all": [[1, {"+": [-2, 1]}], {">": [{"var": ""}, 0]}]}),
                json!({}),
                Ok(json!(false)),
            ),
            // Validate short-circuit
            (
                // The equality expression is invalid and would return an
                // Err if parsed, b/c it has an invalid number of arguments.
                // Since the value before it invalidates the predicate, though,
                // we should never attempt to evaluate it.
                json!({"all": [[1, -1, {"==": []}], {">": [{"var": ""}, 0]}]}),
                json!({}),
                Ok(json!(false)),
            ),
            (
                // Same as above, but put the error before the invalidating
                // value just to make sure our hypothesis is correct re:
                // getting an error
                json!({"all": [[1, {"==": []}, -1], {">": [{"var": ""}, 0]}]}),
                json!({}),
                Err(()),
            ),
            // Parse data in array
            (
                json!({"all": [[1, {"var": "foo"}], {">": [{"var": ""}, 0]}]}),
                json!({"foo": 1}),
                Ok(json!(true)),
            ),
            (
                json!({"all": [[1, {"var": "foo"}], {">": [{"var": ""}, 0]}]}),
                json!({"foo": -5}),
                Ok(json!(false)),
            ),
            (
                json!({"all": [[1, {"var": "foo"}], {">": [{"var": ""}, 0]}]}),
                json!({"foo": -5}),
                Ok(json!(false)),
            ),
        ]
    }

    fn some_cases() -> Vec<(Value, Value, Result<Value, ()>)> {
        vec![
            // Invalid first arguments
            (json!({"some": [1, 1]}), json!({}), Err(())),
            (json!({"some": [{}, 1]}), json!({}), Err(())),
            (json!({"some": [false, 1]}), json!({}), Err(())),
            // Empty array/string
            (json!({"some": [[], 1]}), json!({}), Ok(json!(false))),
            (json!({"some": ["", 1]}), json!({}), Ok(json!(false))),
            (json!({"some": [null, 1]}), json!({}), Ok(json!(false))),
            // Constant predicate
            (json!({"some": [[1, 2], 1]}), json!({}), Ok(json!(true))),
            (json!({"some": [[1, 2], 0]}), json!({}), Ok(json!(false))),
            // Simple predicate
            (
                json!({"some": [[-5, 2], {">": [{"var": ""}, 0]}]}),
                json!({}),
                Ok(json!(true)),
            ),
            (
                json!({"some": [[-3, 1, 2, -1], {">": [{"var": ""}, 0]}]}),
                json!({}),
                Ok(json!(true)),
            ),
            (
                json!({"some": ["aaaa", {"===": [{"var": ""}, "a"]}]}),
                json!({}),
                Ok(json!(true)),
            ),
            (
                json!({"some": ["aabaa", {"===": [{"var": ""}, "a"]}]}),
                json!({}),
                Ok(json!(true)),
            ),
            (
                json!({"some": ["cdefg", {"===": [{"var": ""}, "a"]}]}),
                json!({}),
                Ok(json!(false)),
            ),
            // Expression in array
            (
                json!({"some": [[-6, {"+": [1, 1]}], {">": [{"var": ""}, 0]}]}),
                json!({}),
                Ok(json!(true)),
            ),
            (
                json!({"some": [[-5, {"+": [-2, 1]}], {">": [{"var": ""}, 0]}]}),
                json!({}),
                Ok(json!(false)),
            ),
            // Validate short-circuit
            (
                // The equality expression is invalid and would return an
                // Err if parsed, b/c it has an invalid number of arguments.
                // Since the value before it validates the predicate, though,
                // we should never attempt to evaluate it.
                json!({"some": [[1, {"==": []}], {">": [{"var": ""}, 0]}]}),
                json!({}),
                Ok(json!(true)),
            ),
            (
                // Same as above, but put the error before the invalidating
                // value just to make sure our hypothesis is correct re:
                // getting an error
                json!({"some": [[-51, {"==": []}, -1], {">": [{"var": ""}, 0]}]}),
                json!({}),
                Err(()),
            ),
            // Parse data in array
            (
                json!({"some": [[-4, {"var": "foo"}], {">": [{"var": ""}, 0]}]}),
                json!({"foo": 1}),
                Ok(json!(true)),
            ),
            (
                json!({"some": [[-4, {"var": "foo"}], {">": [{"var": ""}, 0]}]}),
                json!({"foo": -5}),
                Ok(json!(false)),
            ),
        ]
    }

    fn none_cases() -> Vec<(Value, Value, Result<Value, ()>)> {
        vec![
            // Invalid first arguments
            (json!({"none": [1, 1]}), json!({}), Err(())),
            (json!({"none": [{}, 1]}), json!({}), Err(())),
            (json!({"none": [false, 1]}), json!({}), Err(())),
            // Empty array/string
            (json!({"none": [[], 1]}), json!({}), Ok(json!(true))),
            (json!({"none": ["", 1]}), json!({}), Ok(json!(true))),
            (json!({"none": [null, 1]}), json!({}), Ok(json!(true))),
            // Constant predicate
            (json!({"none": [[1, 2], 1]}), json!({}), Ok(json!(false))),
            (json!({"none": [[1, 2], 0]}), json!({}), Ok(json!(true))),
            // Simple predicate
            (
                json!({"none": [[-5, 2], {">": [{"var": ""}, 0]}]}),
                json!({}),
                Ok(json!(false)),
            ),
            (
                json!({"none": [[-3, 1, 2, -1], {">": [{"var": ""}, 0]}]}),
                json!({}),
                Ok(json!(false)),
            ),
            (
                json!({"none": ["aaaa", {"===": [{"var": ""}, "a"]}]}),
                json!({}),
                Ok(json!(false)),
            ),
            (
                json!({"none": ["aabaa", {"===": [{"var": ""}, "a"]}]}),
                json!({}),
                Ok(json!(false)),
            ),
            (
                json!({"none": ["cdefg", {"===": [{"var": ""}, "a"]}]}),
                json!({}),
                Ok(json!(true)),
            ),
            // Expression in array
            (
                json!({"none": [[-6, {"+": [1, 1]}], {">": [{"var": ""}, 0]}]}),
                json!({}),
                Ok(json!(false)),
            ),
            (
                json!({"none": [[-5, {"+": [-2, 1]}], {">": [{"var": ""}, 0]}]}),
                json!({}),
                Ok(json!(true)),
            ),
            // Validate short-circuit
            (
                // The equality expression is invalid and would return an
                // Err if parsed, b/c it has an invalid number of arguments.
                // Since the value before it validates the predicate, though,
                // we should never attempt to evaluate it.
                json!({"none": [[1, {"==": []}], {">": [{"var": ""}, 0]}]}),
                json!({}),
                Ok(json!(false)),
            ),
            (
                // Same as above, but put the error before the invalidating
                // value just to make sure our hypothesis is correct re:
                // getting an error
                json!({"none": [[-51, {"==": []}, -1], {">": [{"var": ""}, 0]}]}),
                json!({}),
                Err(()),
            ),
            // Parse data in array
            (
                json!({"none": [[-4, {"var": "foo"}], {">": [{"var": ""}, 0]}]}),
                json!({"foo": 1}),
                Ok(json!(false)),
            ),
            (
                json!({"none": [[-4, {"var": "foo"}], {">": [{"var": ""}, 0]}]}),
                json!({"foo": -5}),
                Ok(json!(true)),
            ),
        ]
    }

    fn merge_cases() -> Vec<(Value, Value, Result<Value, ()>)> {
        vec![
            (json!({"merge": []}), json!({}), Ok(json!([]))),
            (json!({"merge": [1]}), json!({}), Ok(json!([1]))),
            (json!({"merge": [1, 2]}), json!({}), Ok(json!([1, 2]))),
            (
                json!({"merge": [[1, 2], 2]}),
                json!({}),
                Ok(json!([1, 2, 2])),
            ),
            (json!({"merge": [[1], [2]]}), json!({}), Ok(json!([1, 2]))),
            (json!({"merge": [1, [2]]}), json!({}), Ok(json!([1, 2]))),
            (
                json!({"merge": [1, [2, [3, 4]]]}),
                json!({}),
                Ok(json!([1, 2, [3, 4]])),
            ),
            (
                json!({"merge": [{"var": "foo"}, [2]]}),
                json!({"foo": 1}),
                Ok(json!([1, 2])),
            ),
            (json!({"merge": [[], [2]]}), json!(null), Ok(json!([2]))),
            (
                json!({"merge": [[[]], [2]]}),
                json!(null),
                Ok(json!([[], 2])),
            ),
            (json!({"merge": [{}, [2]]}), json!(null), Ok(json!([{}, 2]))),
            (
                json!({"merge": [{}, [2], 3, false]}),
                json!(null),
                Ok(json!([{}, 2, 3, false])),
            ),
        ]
    }

    fn cat_cases() -> Vec<(Value, Value, Result<Value, ()>)> {
        vec![
            (json!({"cat": []}), json!({}), Ok(json!(""))),
            (json!({"cat": [1]}), json!({}), Ok(json!("1"))),
            (json!({"cat": ["a"]}), json!({}), Ok(json!("a"))),
            (json!({"cat": ["a", "b"]}), json!({}), Ok(json!("ab"))),
            (json!({"cat": ["a", "b", "c"]}), json!({}), Ok(json!("abc"))),
            (json!({"cat": ["a", "b", 1]}), json!({}), Ok(json!("ab1"))),
        ]
    }

    fn substr_cases() -> Vec<(Value, Value, Result<Value, ()>)> {
        vec![
            // Wrong number of arguments
            (json!({"substr": []}), json!({}), Err(())),
            (json!({"substr": ["foo"]}), json!({}), Err(())),
            (json!({"substr": ["foo", 1, 2, 3]}), json!({}), Err(())),
            // Wrong argument types
            (json!({"substr": [12, 1]}), json!({}), Err(())),
            (json!({"substr": ["foo", "12"]}), json!({}), Err(())),
            // Non-negative indices
            (json!({"substr": ["foo", 0]}), json!({}), Ok(json!("foo"))),
            (json!({"substr": ["foo", 1]}), json!({}), Ok(json!("oo"))),
            (json!({"substr": ["foo", 2]}), json!({}), Ok(json!("o"))),
            // Negative indices
            (json!({"substr": ["foo", -1]}), json!({}), Ok(json!("o"))),
            (json!({"substr": ["foo", -2]}), json!({}), Ok(json!("oo"))),
            (json!({"substr": ["foo", -3]}), json!({}), Ok(json!("foo"))),
            // Out-of-bounds indices
            (json!({"substr": ["foo", 3]}), json!({}), Ok(json!(""))),
            (json!({"substr": ["foo", 20]}), json!({}), Ok(json!(""))),
            (json!({"substr": ["foo", -4]}), json!({}), Ok(json!("foo"))),
            // Non-negative Limits
            (json!({"substr": ["foo", 0, 1]}), json!({}), Ok(json!("f"))),
            (
                json!({"substr": ["foo", 0, 3]}),
                json!({}),
                Ok(json!("foo")),
            ),
            (json!({"substr": ["foo", 0, 0]}), json!({}), Ok(json!(""))),
            (json!({"substr": ["foo", 1, 1]}), json!({}), Ok(json!("o"))),
            // Negative Limits
            (
                json!({"substr": ["foo", 0, -1]}),
                json!({}),
                Ok(json!("fo")),
            ),
            (json!({"substr": ["foo", 0, -2]}), json!({}), Ok(json!("f"))),
            (json!({"substr": ["foo", 0, -3]}), json!({}), Ok(json!(""))),
            // Out-of-bounds limits
            (
                json!({"substr": ["foo", 0, 10]}),
                json!({}),
                Ok(json!("foo")),
            ),
            (json!({"substr": ["foo", 0, -10]}), json!({}), Ok(json!(""))),
            // Negative indices with negative limits
            (
                json!({"substr": ["foo", -3, -2]}),
                json!({}),
                Ok(json!("f")),
            ),
            // Negative indices with positive limits
            (
                json!({"substr": ["foo", -3, 2]}),
                json!({}),
                Ok(json!("fo")),
            ),
            // Out-of-bounds indices with out-of-bounds limits
            (json!({"substr": ["foo", 10, 10]}), json!({}), Ok(json!(""))),
            (
                json!({"substr": ["foo", 10, -10]}),
                json!({}),
                Ok(json!("")),
            ),
            (
                json!({"substr": ["foo", -10, 10]}),
                json!({}),
                Ok(json!("foo")),
            ),
            (
                json!({"substr": ["foo", -10, -10]}),
                json!({}),
                Ok(json!("")),
            ),
        ]
    }

    fn log_cases() -> Vec<(Value, Value, Result<Value, ()>)> {
        vec![
            // Invalid number of arguments
            (json!({"log": []}), json!({}), Err(())),
            (json!({"log": [1, 2]}), json!({}), Err(())),
            // Correct number of arguments
            (json!({"log": [1]}), json!({}), Ok(json!(1))),
            (json!({"log": 1}), json!({}), Ok(json!(1))),
        ]
    }

    fn lt_cases() -> Vec<(Value, Value, Result<Value, ()>)> {
        vec![
            (json!({"<": [1, 2]}), json!({}), Ok(json!(true))),
            (json!({"<": [3, 2]}), json!({}), Ok(json!(false))),
            (
                json!({"<": [1, {"var": "foo"}]}),
                json!({"foo": 5}),
                Ok(json!(true)),
            ),
            (json!({"<": [1, 2, 3]}), json!({}), Ok(json!(true))),
            (json!({"<": [3, 2, 3]}), json!({}), Ok(json!(false))),
            (json!({"<": [1, 2, 1]}), json!({}), Ok(json!(false))),
        ]
    }

    fn gt_cases() -> Vec<(Value, Value, Result<Value, ()>)> {
        vec![
            (json!({">": [1, 2]}), json!({}), Ok(json!(false))),
            (json!({">": [3, 2]}), json!({}), Ok(json!(true))),
            (
                json!({">": [1, {"var": "foo"}]}),
                json!({"foo": 5}),
                Ok(json!(false)),
            ),
            (json!({">": [1, 2, 3]}), json!({}), Ok(json!(false))),
            (json!({">": [3, 2, 3]}), json!({}), Ok(json!(false))),
            (json!({">": [1, 2, 1]}), json!({}), Ok(json!(false))),
            (json!({">": [3, 2, 1]}), json!({}), Ok(json!(true))),
        ]
    }

    fn plus_cases() -> Vec<(Value, Value, Result<Value, ()>)> {
        vec![
            (json!({"+": []}), json!({}), Ok(json!(0))),
            (json!({"+": [1]}), json!({}), Ok(json!(1))),
            (json!({"+": ["1"]}), json!({}), Ok(json!(1))),
            (json!({"+": [1, 1]}), json!({}), Ok(json!(2))),
            (json!({"+": [1, 1, 1]}), json!({}), Ok(json!(3))),
            (json!({"+": [1, 1, false]}), json!({}), Err(())),
            (json!({"+": [1, 1, "1"]}), json!({}), Ok(json!(3))),
            (
                json!({"+": [1, 1, "123abc"]}), // WHY???
                json!({}),
                Ok(json!(125)),
            ),
        ]
    }

    fn minus_cases() -> Vec<(Value, Value, Result<Value, ()>)> {
        vec![
            (json!({"-": "5"}), json!({}), Ok(json!(-5))),
            (json!({"-": [2]}), json!({}), Ok(json!(-2))),
            (json!({"-": [2, 2]}), json!({}), Ok(json!(0))),
            (json!({"-": ["9", [3]]}), json!({}), Ok(json!(6))),
        ]
    }

    fn multiplication_cases() -> Vec<(Value, Value, Result<Value, ()>)> {
        vec![
            (json!({"*": 1}), json!({}), Ok(json!(1))),
            (json!({"*": [1]}), json!({}), Ok(json!(1))),
            (json!({"*": [1, 2]}), json!({}), Ok(json!(2))),
            (json!({"*": [0, 2]}), json!({}), Ok(json!(0))),
            (json!({"*": [1, 2, 3]}), json!({}), Ok(json!(6))),
            (json!({"*": [1, 2, "3"]}), json!({}), Ok(json!(6))),
            (json!({"*": [1, "2abc", "3"]}), json!({}), Ok(json!(6))),
            (json!({"*": []}), json!({}), Err(())),
        ]
    }

    fn division_cases() -> Vec<(Value, Value, Result<Value, ()>)> {
        vec![
            (json!({"/": [2, 1]}), json!({}), Ok(json!(2))),
            (json!({"/": [1, 2]}), json!({}), Ok(json!(0.5))),
            (json!({"/": [1, "2"]}), json!({}), Ok(json!(0.5))),
            (json!({"/": [12, "-2"]}), json!({}), Ok(json!(-6))),
            (json!({"/": []}), json!({}), Err(())),
            (json!({"/": [5]}), json!({}), Err(())),
            (json!({"/": [5, 2, 1]}), json!({}), Err(())),
        ]
    }

    fn modulo_cases() -> Vec<(Value, Value, Result<Value, ()>)> {
        vec![
            (json!({"%": [2, 1]}), json!({}), Ok(json!(0))),
            (json!({"%": [1, 2]}), json!({}), Ok(json!(1))),
            (json!({"%": [1, "2"]}), json!({}), Ok(json!(1))),
            (json!({"%": [12, "-2"]}), json!({}), Ok(json!(0))),
            (json!({"%": []}), json!({}), Err(())),
            (json!({"%": [5]}), json!({}), Err(())),
            (json!({"%": [5, 2, 1]}), json!({}), Err(())),
        ]
    }

    fn max_cases() -> Vec<(Value, Value, Result<Value, ()>)> {
        vec![
            (json!({"max": [1, 2, 3]}), json!({}), Ok(json!(3))),
            (json!({"max": [false, -1, 2]}), json!({}), Ok(json!(2))),
            (json!({"max": [0, -1, true]}), json!({}), Ok(json!(1))),
            (json!({"max": [0, -1, true, [3]]}), json!({}), Ok(json!(3))),
        ]
    }

    fn min_cases() -> Vec<(Value, Value, Result<Value, ()>)> {
        vec![
            (json!({"min": [1, 2, 3]}), json!({}), Ok(json!(1))),
            (json!({"min": [false, 1, 2]}), json!({}), Ok(json!(0))),
            (json!({"min": [0, -1, true]}), json!({}), Ok(json!(-1))),
            (
                json!({"min": [0, [-1], true, [3]]}),
                json!({}),
                Ok(json!(-1)),
            ),
        ]
    }

    fn bang_cases() -> Vec<(Value, Value, Result<Value, ()>)> {
        vec![
            (json!( {"!": []} ), json!({}), Err(())),
            (json!( {"!": [1, 2]} ), json!({}), Err(())),
            (json!({"!": [true]}), json!({}), Ok(json!(false))),
            (json!({"!": [1]}), json!({}), Ok(json!(false))),
            (json!({"!": [0]}), json!({}), Ok(json!(true))),
            (json!({"!": [[]]}), json!({}), Ok(json!(true))),
            (json!({"!": [{}]}), json!({}), Ok(json!(false))),
            (json!({"!": [""]}), json!({}), Ok(json!(true))),
            (json!({"!": ["foo"]}), json!({}), Ok(json!(false))),
            (json!({"!": true}), json!({}), Ok(json!(false))),
        ]
    }

    fn in_cases() -> Vec<(Value, Value, Result<Value, ()>)> {
        vec![
            // Invalid inputs
            (json!( {"in": []} ), json!({}), Err(())),
            (json!( {"in": [1, [], 1]} ), json!({}), Err(())),
            (json!( {"in": [1, "foo"]} ), json!({}), Err(())),
            (json!( {"in": [1, 1]} ), json!({}), Err(())),
            // Valid inputs
            (json!( {"in": [1, null]} ), json!({}), Ok(json!(false))),
            (json!( {"in": [1, [1, 2]]} ), json!({}), Ok(json!(true))),
            (json!( {"in": [1, [0, 2]]} ), json!({}), Ok(json!(false))),
            (json!( {"in": ["f", "foo"]} ), json!({}), Ok(json!(true))),
            (json!( {"in": ["f", "bar"]} ), json!({}), Ok(json!(false))),
            (json!( {"in": ["f", null]} ), json!({}), Ok(json!(false))),
            (
                json!( {"in": [null, [1, null]]} ),
                json!({}),
                Ok(json!(true)),
            ),
            (json!( {"in": [null, [1, 2]]} ), json!({}), Ok(json!(false))),
            (
                json!( {"in": [true, [true, 2]]} ),
                json!({}),
                Ok(json!(true)),
            ),
            (json!( {"in": [true, [1, 2]]} ), json!({}), Ok(json!(false))),
            (
                json!( {"in": [[1, 2], [[1, 2], 2]]} ),
                json!({}),
                Ok(json!(true)),
            ),
            (
                json!( {"in": [[], [[1, 2], 2]]} ),
                json!({}),
                Ok(json!(false)),
            ),
            (
                json!( {"in": [{"a": 1}, [{"a": 1}, 2]]} ),
                json!({}),
                Ok(json!(true)),
            ),
            (
                json!( {"in": [{"a": 1}, [{"a": 2}, 2]]} ),
                json!({}),
                Ok(json!(false)),
            ),
            (
                json!( {"in": [{"a": 1}, [{"a": 1, "b": 2}, 2]]} ),
                json!({}),
                Ok(json!(false)),
            ),
        ]
    }

    fn assert_jsonlogic((op, data, exp): (Value, Value, Result<Value, ()>)) -> () {
        println!("Running rule: {:?} with data: {:?}", op, data);
        let result = apply(&op, &data);
        println!("- Result: {:?}", result);
        println!("- Expected: {:?}", exp);
        if exp.is_ok() {
            assert_eq!(result.unwrap(), exp.unwrap());
        } else {
            result.unwrap_err();
        }
    }

    fn replace_operator(
        old_op: &'static str,
        new_op: &'static str,
        (op, data, exp): (Value, Value, Result<Value, ()>),
    ) -> (Value, Value, Result<Value, ()>) {
        (
            match op {
                Value::Object(obj) => json!({new_op: obj.get(old_op).unwrap()}),
                _ => panic!(),
            },
            data,
            exp,
        )
    }

    fn flip_boolean_exp(
        (op, data, exp): (Value, Value, Result<Value, ()>),
    ) -> (Value, Value, Result<Value, ()>) {
        (
            op,
            data,
            match exp {
                Err(_) => exp,
                Ok(Value::Bool(exp)) => Ok(Value::Bool(!exp)),
                _ => panic!(),
            },
        )
    }

    fn only_boolean(
        wanted: bool,
        (_, _, exp): &(Value, Value, Result<Value, ()>),
    ) -> bool {
        match exp {
            Err(_) => false,
            Ok(Value::Bool(exp)) => *exp == wanted,
            _ => panic!("unexpected type of expectation"),
        }
    }

    #[test]
    fn test_no_op() {
        no_op_cases().into_iter().for_each(assert_jsonlogic)
    }

    #[test]
    fn test_abstract_eq_op() {
        abstract_eq_cases().into_iter().for_each(assert_jsonlogic)
    }

    #[test]
    fn test_abstract_ne_op() {
        abstract_ne_cases().into_iter().for_each(assert_jsonlogic)
    }

    #[test]
    fn test_strict_eq_op() {
        strict_eq_cases().into_iter().for_each(assert_jsonlogic)
    }

    #[test]
    fn test_strict_ne_op() {
        strict_ne_cases().into_iter().for_each(assert_jsonlogic)
    }

    #[test]
    fn test_var_data_op() {
        var_cases().into_iter().for_each(assert_jsonlogic)
    }

    #[test]
    fn test_missing_data_op() {
        missing_cases().into_iter().for_each(assert_jsonlogic)
    }

    #[test]
    fn test_missing_some_data_op() {
        missing_some_cases().into_iter().for_each(assert_jsonlogic)
    }

    #[test]
    fn test_if_op() {
        if_cases().into_iter().for_each(assert_jsonlogic)
    }

    #[test]
    fn test_or_op() {
        or_cases().into_iter().for_each(assert_jsonlogic)
    }

    #[test]
    fn test_and_op() {
        and_cases().into_iter().for_each(assert_jsonlogic)
    }

    #[test]
    fn test_map_op() {
        map_cases().into_iter().for_each(assert_jsonlogic)
    }

    #[test]
    fn test_filter_op() {
        filter_cases().into_iter().for_each(assert_jsonlogic)
    }

    #[test]
    fn test_reduce_op() {
        reduce_cases().into_iter().for_each(assert_jsonlogic)
    }

    #[test]
    fn test_all_op() {
        all_cases().into_iter().for_each(assert_jsonlogic)
    }

    #[test]
    fn test_some_op() {
        some_cases().into_iter().for_each(assert_jsonlogic)
    }

    #[test]
    fn test_none_op() {
        none_cases().into_iter().for_each(assert_jsonlogic)
    }

    #[test]
    fn test_merge_op() {
        merge_cases().into_iter().for_each(assert_jsonlogic)
    }

    #[test]
    fn test_cat_op() {
        cat_cases().into_iter().for_each(assert_jsonlogic)
    }

    #[test]
    fn test_substr_op() {
        substr_cases().into_iter().for_each(assert_jsonlogic)
    }

    #[test]
    fn test_log_op() {
        log_cases().into_iter().for_each(assert_jsonlogic)
    }

    #[test]
    fn test_lt_op() {
        lt_cases().into_iter().for_each(assert_jsonlogic)
    }

    #[test]
    fn test_lte_op() {
        lt_cases()
            .into_iter()
            .map(|case| replace_operator("<", "<=", case))
            .for_each(assert_jsonlogic);
        abstract_eq_cases()
            .into_iter()
            // Only get cases that are equal, since we don't know whether
            // non-equality cases were lt or gt or what.
            .filter(|case| only_boolean(true, case))
            .map(|case| replace_operator("==", "<=", case))
            .for_each(assert_jsonlogic);
    }

    #[test]
    fn test_gt_op() {
        gt_cases().into_iter().for_each(assert_jsonlogic);
    }

    #[test]
    fn test_gte_op() {
        gt_cases()
            .into_iter()
            .map(|case| replace_operator(">", ">=", case))
            .for_each(assert_jsonlogic);
        abstract_eq_cases()
            .into_iter()
            // Only get cases that are equal, since we don't know whether
            // non-equality cases were lt or gt or what.
            .filter(|case| only_boolean(true, case))
            .map(|case| replace_operator("==", ">=", case))
            .for_each(assert_jsonlogic);
    }

    #[test]
    fn test_plus_op() {
        plus_cases().into_iter().for_each(assert_jsonlogic)
    }

    #[test]
    fn test_minus_op() {
        minus_cases().into_iter().for_each(assert_jsonlogic)
    }

    #[test]
    fn test_mul_op() {
        multiplication_cases()
            .into_iter()
            .for_each(assert_jsonlogic)
    }

    #[test]
    fn test_div_op() {
        division_cases().into_iter().for_each(assert_jsonlogic)
    }

    #[test]
    fn test_mod_op() {
        modulo_cases().into_iter().for_each(assert_jsonlogic)
    }

    #[test]
    fn test_max_op() {
        max_cases().into_iter().for_each(assert_jsonlogic)
    }

    #[test]
    fn test_min_op() {
        min_cases().into_iter().for_each(assert_jsonlogic)
    }

    #[test]
    fn test_bang_op() {
        bang_cases().into_iter().for_each(assert_jsonlogic)
    }

    #[test]
    fn test_bang_bang_op() {
        // just assert the opposite for all the bang cases
        bang_cases()
            .into_iter()
            .map(|case| replace_operator("!", "!!", case))
            .map(flip_boolean_exp)
            .for_each(assert_jsonlogic)
    }

    #[test]
    fn test_in_op() {
        in_cases().into_iter().for_each(assert_jsonlogic)
    }
}
