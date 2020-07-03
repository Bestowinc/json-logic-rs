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
use std::process::Command;

#[cfg(feature = "wasm")]
#[test]
fn test_node_pkg() {
    let build_res = Command::new("make")
        .arg("debug-wasm")
        .output()
        .expect("Could not spawn make");
    assert!(build_res.status.success(), "{:?}", build_res);

    let test_res = Command::new("make")
        .arg("test-wasm")
        .output()
        .expect("Could not spawn make");
    assert!(test_res.status.success(), "{:?}", test_res);
}
