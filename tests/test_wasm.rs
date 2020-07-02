//! Tests for the WASM target
//!
//! Note that a relatively recent version of node needs to be available
//! for these tests to run.
//!
//! These tests will only run if the "wasm" feature is active.
//!
//! Note that the actual tests are found in `test_wasm.js`. This file
//! just serves as a runner.

#[cfg(feature = "wasm")]
use std::path::Path;
#[cfg(feature = "wasm")]
use std::process::Command;
#[cfg(feature = "wasm")]
use std::str;

#[cfg(feature = "wasm")]
fn build_node_pkg() {
    // Build the node pkg
    let res = Command::new("make")
        .arg("debug-wasm")
        .output()
        .expect("Could not spawn make");
    assert!(res.status.success(), "{:?}", res)
}

#[cfg(feature = "wasm")]
#[test]
fn test_node_pkg() {
    build_node_pkg();
    let test_file = Path::new(file!()).parent().unwrap().join("test_wasm.js");
    let res = Command::new("node")
        .arg(test_file)
        .output()
        .expect("WASM tests failed!");
    assert!(
        res.status.success(),
        "stdout = {:?}, stderr = {:?}",
        str::from_utf8(res.stdout.as_slice()),
        str::from_utf8(res.stderr.as_slice()),
    );
}
