# json-logic-rs

![Continuous Integration](https://github.com/Bestowinc/json-logic-rs/workflows/Continuous%20Integration/badge.svg?branch=master)

This s an implementation of  the [JSONLogic] specification in Rust.

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
