.PHONY: test test-rs test-ts build build-rs build-ts

build: build-rs build-ts

build-rs:
	cargo build --release -o bin/figma-mcp

build-ts:
	cd plugin && bun run build

test: test-rs test-ts

test-rs:
	cargo test

test-ts:
	cd plugin && bun test
