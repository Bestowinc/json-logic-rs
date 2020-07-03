# TODO: split sub-language makes into their dirs & call `$(MAKE) -C dir` for them

VENV := . venv/bin/activate;

.PHONY: build
build:
	cargo build --release

.PHONY: build-wasm
build-wasm: setup
	cargo clean -p jsonlogic
	rm -rf ./js && wasm-pack build --target nodejs --out-dir js --out-name index --release -- --features wasm

.PHONY: debug-wasm
debug-wasm:
	rm -rf ./js && wasm-pack build --target nodejs --out-dir js --out-name index --debug -- --features wasm

.PHONY: build-py-sdist
build-py-sdist: venv
	cargo clean -p jsonlogic
	$(VENV) python setup.py sdist

.PHONY: build-py-wheel
build-py-wheel: venv
	cargo clean -p jsonlogic
	$(VENV) python setup.py bdist_wheel

.PHONY: develop-py
develop-py: venv
	$(VENV) python setup.py develop

.PHONY: setup
setup:
	wasm-pack --version > /dev/null 2>&1 || cargo install wasm-pack

.PHONY: test
test:
	cargo test --all-features

.PHONY: test-wasm
test-wasm:
	node tests/test_wasm.js

.PHONY: test-py
test-py: venv
	python tests/test_py.py

venv: setup.py pyproject.toml
	python3 -m venv venv
	$(VENV) pip install setuptools wheel setuptools-rust
	touch venv
