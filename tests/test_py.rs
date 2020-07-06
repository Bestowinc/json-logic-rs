//! Tests for the python bindings
//!
//! Note that Python 3.6+ must be installed for these tests to work.
//!
//! The actual tests are found in `test_py.py`. This file just serves
//! as a runner.

#[cfg(feature = "python")]
use std::process::Command;

#[cfg(feature = "python")]
#[test]
fn test_python_dist() {
    let py_build_res = Command::new("make")
        .arg("develop-py")
        .output()
        .expect("Could not spawn make");
    assert!(py_build_res.status.success(), "{:?}", py_build_res);

    let py_test_res = Command::new("make")
        .arg("test-py")
        .output()
        .expect("Could not spawn make");
    assert!(py_test_res.status.success(), "{:?}", py_test_res);
}
