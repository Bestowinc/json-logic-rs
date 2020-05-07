# TODO: split sub-language makes into their dirs & call `$(MAKE) -C dir` for them

.PHONY: build-js run-js

build:
	cargo build --release

build-js:
	cargo clean
	rm -rf ./js/jsonlogic && wasm-pack build --target bundler --out-dir js/jsonlogic --out-name index --release -- --features javascript

run-js:
	cd js && npm run-script serve

build-py:
	cargo clean
	cd py && source ./venv/bin/activate && python ./setup.py bdist_wheel

develop-py:
	cargo clean
	cd py && source ./venv/bin/activate && python ./setup.py develop

setup:
	cargo install wasm-pack
	python3 -m venv py/venv

test:
	cargo test
