//! Run the official tests from the web.

use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

use serde_json;
use serde_json::Value;

use jsonlogic;

struct TestCase {
    logic: Value,
    data: Value,
    result: Value,
}

fn load_tests() -> Vec<TestCase> {
    let mut file = File::open(Path::join(
        Path::new(file!()).parent().unwrap(),
        "data/tests.json",
    ))
    .unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();
    let cases = match serde_json::from_str(&contents).unwrap() {
        Value::Array(cases) => cases,
        _ => panic!("cases aren't array"),
    };
    cases
        .into_iter()
        .filter_map(|case| match case {
            Value::Array(data) => Some(TestCase {
                logic: data[0].clone(),
                data: data[1].clone(),
                result: data[2].clone(),
            }),
            Value::String(_) => None,
            _ => panic!("case can't be destructured!"),
        })
        .collect()
}

#[test]
fn run_cases() {
    let cases = load_tests();
    cases.into_iter().for_each(|case| {
        println!("Running case");
        println!("  logic: {:?}", case.logic);
        println!("  data: {:?}", case.data);
        println!("  expected: {:?}", case.result);
        assert_eq!(
            jsonlogic::jsonlogic(&case.logic, &case.data).unwrap(),
            case.result
        )
    })
}
