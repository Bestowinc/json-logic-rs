build:
	cargo build --release
	rm -rf ./pkg && wasm-pack build --target web --out-name index --release

setup:
	cargo install wasm-pack
