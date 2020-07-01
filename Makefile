# TODO: split sub-language makes into their dirs & call `$(MAKE) -C dir` for them

.PHONY: build-js run-js

build:
	cargo build --release

build-wasm:
	cargo clean -p jsonlogic
	rm -rf ./js && wasm-pack build --target nodejs --out-dir js --out-name index --release -- --features wasm

debug-wasm:
	rm -rf ./js && wasm-pack build --target nodejs --out-dir js --out-name index --debug -- --features wasm

run-js:
	cd js && npm run-script serve

build-py:
	cargo clean -p jsonlogic
	cd py && source ./venv/bin/activate && python ./setup.py bdist_wheel

develop-py:
	cargo clean -p jsonlogic
	cd py && source ./venv/bin/activate && python ./setup.py develop

setup:
	cargo install wasm-pack

test:
	cargo test --all-features
