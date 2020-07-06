# json-logic-rs

![Continuous Integration](https://github.com/Bestowinc/json-logic-rs/workflows/Continuous%20Integration/badge.svg?branch=master)

This is an implementation of  the [JSONLogic] specification in Rust.

## Project Status

We implement 100% of the standard supported operations defined [here](http://jsonlogic.com/operations.html).

We also implement the `?:`, which is not described in that specification
but is a direct alias for `if`.

All operations are tested using our own test suite in Rust as well as the
shared tests for all jsonlogic implementations defined [here](http://jsonlogic.com/tests.json).

We are working on adding new operations with improved type safety, as well
as the ability to define functions as jsonlogic. We will communicate with
the broader jsonlogic community to see if we can make them part of the
standard as we do so.

Being built in Rust, we are able to provide the package in a variety of
languages. The table below describes current language support:

<!-- TODO  Add links below -->

| **Language**         | **Available Via**    |
| -------------------- | -------------------- |
| Rust                 | Cargo                |
| JavaScript (as WASM) | Node Package via NPM |
| Python               | PyPI                 |

## Usage

### Rust

```rust
use jsonlogic_rs;
use serde_json::json;

fn main() {
    assert_eq!(
        jsonlogic_rs::apply(
            json!({"===": [{"var": "a"}, 7]}),
            json!({"a": 7}),
        ),
        json!(true)
    );
}
```

### Javascript

```js
const jsonlogic = require("jsonlogic-rs")

jsonlogic.apply(
    {"===": [{"var": "a"}, 7]},
    {"a": 7}
)
```

### Python

```py
import jsonlogic_rs

res = jsonlogic_rs.apply(
    {"===": [{"var": "a"}, 7]},
    {"a": 7}
)

assert res == True
```

## Building

### Prerequisites

You must have Rust installed and `cargo` available in your `PATH`.

If you would like to build or test the Python distribution, Python 3.6 or
newer must be available in your `PATH`. The `venv` module must be part of the
Python distribution (looking at you, Ubuntu).

If you would like to run tests for the WASM package, `node` 10 or newer must be
available in your `PATH`.

### Rust

To build the Rust library, just run `cargo build`.

You can create a release build with `make build`.

### WebAssembly

You can build a debug WASM release with

```sh
make debug-wasm
```

You can build a production WASM release with

```sh
make build-wasm
```

The built WASM package will be in `js/`. This package is directly importable
from `node`, but needs to be browserified in order to be used in the browser.

### Python

To perform a dev install of the Python package, run:

```sh
make develop-py
```

This will automatically create a virtual environment in `venv/`, install
the necessary packages, and then install `jsonlogic_rs` into that environment.

**Note:** from our CI experiences, this may not work for Python 3.8 on Windows.
If you are running this on a Windows machine and can confirm whether or not
this works, let us know!

To build a production source distribution:

```sh
make build-py-sdist
```

To build a wheel (specific to your current system architecture and python
version):

```sh
make build-py-wheel
```

The python distribution consists both of the C extension generated from the
Rust and a thin wrapper found in `py/jsonlogic_rs/`. `make develop-py` will
compile the C extension and place it in that directory, where it will be
importable by your local venv. When building wheels, the wrapper and the C
extension are all packaged together into the resultant wheel, which will
be found in `dist/`. When building an sdist, the Rust extension is not compiled.
The Rust and Python source are distributed together in a `.tar.gz` file, again
found in `dist/`.

[jsonlogic]: http://jsonlogic.com/
