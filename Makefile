.PHONY: build test lint check flows wasm

BUILD_FLAGS ?=

default: build

build:
	greentic-component build --manifest ./component.manifest.json $(BUILD_FLAGS)

flows:
	greentic-component flow update

wasm:
	cargo build --target wasm32-wasip2 --release

check:
	greentic-component doctor target/wasm32-wasip2/release/component_templates.wasm --manifest ./component.manifest.json

lint:
	cargo fmt --all
	cargo clippy --workspace --all-targets -- -D warnings

test:
	cargo test --workspace --all-targets
