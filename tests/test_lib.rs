//! Run the official tests from the web.

use std::fs::File;
use std::io::prelude::*;
use std::path::Path;



use serde_json::Value;



struct TestCase {
    logic: Value,
    data: Value,
    result: Value,
}

const TEST_URL: &str = "http://jsonlogic.com/tests.json";

fn load_file_json() -> Value {
    let mut file = File::open(Path::join(
        Path::new(file!()).parent().unwrap(),
        "data/tests.json",
    ))
    .unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();
    serde_json::from_str(&contents).unwrap()
}

fn load_tests() -> Vec<TestCase> {
    let loaded_json = load_file_json();
    let cases = match loaded_json {
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
#[ignore]
fn check_test_file() {
    let resp_res = reqwest::blocking::get(TEST_URL).unwrap().text();
    let resp = match resp_res {
        Ok(r) => r,
        Err(e) => {
            println!("Failed to get new version of test JSON: {:?}", e);
            return ;
        }
    };
    let http_json: Value = serde_json::from_str(&resp).unwrap();
    let file_json = load_file_json();
    assert_eq!(http_json, file_json);
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
            jsonlogic_rs::apply(&case.logic, &case.data).unwrap(),
            case.result
        )
    })
}
