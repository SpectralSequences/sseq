.PHONY: all test test-concurrent lint dummy

all:
	cargo build

test:
	cargo test --workspace

test-concurrent:
	cargo test --features concurrent

lint:
	cargo clippy --workspace --all-targets

run_examples: $(patsubst examples/%.rs, target/release/examples/%, $(wildcard examples/*.rs))

target/release/examples/%: dummy
	cargo run --all-features --release --example "$(notdir $@)"
