.PHONY: all test test-concurrent lint dummy

all:
	cargo build

test:
	cargo test --workspace

test-concurrent:
	cargo test --features concurrent

lint:
	cargo clippy --workspace --all-targets
	rustfmt --check tests/*.rs examples/*.rs
	rustfmt --check crates/{error,once,query,saveload,thread-token}/**/*.rs

run_examples: $(patsubst examples/%.rs, target/release/examples/%, $(wildcard examples/*.rs))

target/release/examples/%: dummy
	cargo run --all-features --release --example "$(notdir $@)"
