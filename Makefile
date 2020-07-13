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
	cargo clean -p jsonlogic-rs
	rm -rf ./js && wasm-pack build --target nodejs --out-dir js --out-name index --release --scope bestow -- --features wasm

.PHONY: debug-wasm
debug-wasm:
	rm -rf ./js && wasm-pack build --target nodejs --out-dir js --out-name index --debug --scope bestow -- --features wasm

.PHONY: clean-py
clean-py:
	rm -rf build/*
	rm -rf dist/*

.PHONY: build-py-sdist
build-py-sdist: $(VENV) clean-py
	cargo clean -p jsonlogic-rs
	$(VENV) setup.py sdist

.PHONY: build-py-wheel
build-py-wheel: $(VENV) clean-py
	cargo clean -p jsonlogic-rs
	$(VENV) setup.py bdist_wheel

# NOTE: this command may require sudo on linux
.PHONY: build-py-wheel-manylinux
build-py-wheel-manylinux: clean-py
	docker run -v "$$PWD":/io --rm "$(MANYLINUX_IMG)" /io/build-wheels.sh

# NOTE: this command may require sudo on linux
.PHONY: build-py-wheel-manylinux-no-clean
build-py-wheel-manylinux-no-clean:
	docker run -v "$$PWD":/io --rm "$(MANYLINUX_IMG)" /io/build-wheels.sh

.PHONY: build-py-all
build-py-all: $(VENV) clean-py
	cargo clean -p jsonlogic-rs
	$(VENV) setup.py sdist bdist_wheel

.PHONY: develop-py-wheel
develop-py-wheel: $(VENV)
	$(VENV) setup.py bdist_wheel

.PHONY: develop-py
develop-py: $(VENV)
	$(VENV) setup.py develop

.PHONY: distribute-py
distribute-py: $(VENV)
	$(VENV) -m pip install twine
	twine upload -s dist/*

.PHONY: test-distribute-py
test-distribute-py:
	$(VENV) -m pip install twine
	twine upload -s --repository testpypi dist/*

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
