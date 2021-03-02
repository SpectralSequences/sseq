.PHONY: all test

all:
	cargo build

test:
	cargo test --workspace
	cargo test --features concurrent

lint:
	cargo clippy
