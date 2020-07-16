# json-logic-rs

![Continuous Integration](https://github.com/Bestowinc/json-logic-rs/workflows/Continuous%20Integration/badge.svg?branch=master)

This is an implementation of  the [JsonLogic] specification in Rust.

## Project Status

We implement 100% of the standard supported operations defined [here](http://jsonlogic.com/operations.html).

We also implement the `?:`, which is not described in that specification
but is a direct alias for `if`.

All operations are tested using our own test suite in Rust as well as the
shared tests for all JsonLogic implementations defined [here](http://jsonlogic.com/tests.json).

We are working on adding new operations with improved type safety, as well
as the ability to define functions as JsonLogic. We will communicate with
the broader JsonLogic community to see if we can make them part of the
standard as we do so.

Being built in Rust, we are able to provide the package in a variety of
languages. The table below describes current language support:

| **Language**         | **Available Via**                                                          |
| -------------------- | -------------------------------------------------------------------------- |
| Rust                 | [Cargo](https://crates.io/crates/jsonlogic-rs)                             |
| JavaScript (as WASM) | Node Package via [NPM](https://www.npmjs.com/package/@bestow/jsonlogic-rs) |
| Python               | [PyPI](https://test.pypi.org/project/jsonlogic-rs/0.1.0/)                  |

## Installation

### Rust

To use as a Rust library, add to your `Cargo.toml`:

``` toml
[dependencies]
jsonlogic-rs = "~0.1"
```

If you just want to use the commandline `jsonlogic` binary:

``` sh
cargo install jsonlogic-rs
```

### Node/Browser

You can install JsonLogic using npm or yarn. In NPM:

``` sh
npm install --save @bestow/jsonlogic-rs
```

Note that the package is distributed as a node package, so you'll need to use
`browserify`, `webpack`, or similar to install for the browser.

### Python

Supports Python 3.6+.

Wheels are distributed for many platforms, so you can often just run:

``` sh
pip install jsonlogic-rs
```

If a wheel does _not_ exist for your system, this will attempt to build the
package. In order for the package to build successfully, you MUST have Rust
installed on your local system, and `cargo` MUST be present in your `PATH`.

See [Building](#Building) below for more details.

## Usage

### Rust

```rust
use jsonlogic_rs;
use serde_json::{json, from_str, Value};

// You can pass JSON values deserialized with serde straight into apply().
fn main() {
    let data: Value = from_str(r#"{"a": 7}"#)
    assert_eq!(
        jsonlogic_rs::apply(
            json!({"===": [{"var": "a"}, 7]}),
            data,
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

# If You have serialized JsonLogic and data, the `apply_serialized` method can
# be used instead
res = jsonlogic_rs.apply_serialized(
    '{"===": [{"var": "a"}, 7]}',
    '{"a": 7}'
)
```

### Commandline

``` raw
Parse JSON data with a JsonLogic rule.

When no <data> or <data> is -, read from stdin.

The result is written to stdout as JSON, so multiple calls
can be chained together if desired.

USAGE:
    jsonlogic <logic> [data]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

ARGS:
    <logic>    A JSON logic string
    <data>     A string of JSON data to parse. May be provided as stdin.

EXAMPLES:
    jsonlogic '{"===": [{"var": "a"}, "foo"]}' '{"a": "foo"}'
    jsonlogic '{"===": [1, 1]}' null
    echo '{"a": "foo"}' | jsonlogic '{"===": [{"var": "a"}, "foo"]}'

Inspired by and conformant with the original JsonLogic (jsonlogic.com).
```

Run `jsonlogic --help` the most up-to-date usage.

An example of chaining multiple results:

``` sh
$ echo '{"a": "a"}' \
    | jsonlogic '{"if": [{"===": [{"var": "a"}, "a"]}, {"result": true}, {"result": false}]}' \
    | jsonlogic '{"if": [{"!!": {"var": "result"}}, "result was true", "result was false"]}'

"result was true"
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
