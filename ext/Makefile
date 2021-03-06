.PHONY: all test

all:
	cargo build

test:
	cargo test --workspace

test-concurrent:
	cargo test --features concurrent

lint:
	cargo clippy --workspace --all-targets
