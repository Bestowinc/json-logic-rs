build:
	cargo build --release
	rm -rf ./pkg && wasm-pack build --target bundler --out-name index --release

setup:
	cargo install wasm-pack

test:
	cargo test
