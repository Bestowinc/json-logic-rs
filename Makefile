# TODO: split sub-language makes into their dirs & call `$(MAKE) -C dir` for them

SHELL = bash

ifeq ($(WINDOWS),true)
	VENV=venv/Scripts/python.exe
else
	VENV=venv/bin/python
endif

ifeq ($(PYTHON),)
	PYTHON := python$(PY_VER)
endif


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
build-py-sdist: $(VENV)
	cargo clean -p jsonlogic
	$(VENV) setup.py sdist

.PHONY: build-py-wheel
build-py-wheel: $(VENV)
	cargo clean -p jsonlogic
	rm -rf dist/*
	$(VENV) setup.py bdist_wheel

.PHONY: develop-py-wheel
develop-py-wheel: $(VENV)
	$(VENV) setup.py bdist_wheel

.PHONY: develop-py
develop-py: $(VENV)
	$(VENV) setup.py develop

.PHONY: setup
setup:
	wasm-pack --version > /dev/null 2>&1 || cargo install wasm-pack

.PHONY: test
test:
	PYTHON=$(PYTHON) WINDOWS=$(WINDOWS) cargo test --all-features

.PHONY: test-wasm
test-wasm:
	node tests/test_wasm.js

.PHONY: test-py
test-py: $(VENV)
	$(VENV) tests/test_py.py

venv: $(VENV)
$(VENV): setup.py pyproject.toml
	$(PYTHON) -m venv venv
	$(VENV) -m pip install setuptools wheel setuptools-rust
