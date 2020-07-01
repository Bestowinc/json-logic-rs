# Integration Tests

The `test_lib.rs` use the test JSON available from the JSONLogic
project to validate teh Rust library. The test JSON is checked in under
`data/tests.json`. When tests run, the content of `tests.json` is
validated against the most recent content of the tests returned from
the server. If they don't match, the test fails.

We run that full suite of tests against all implementations.
