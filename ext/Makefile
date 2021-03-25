.PHONY: all test test-concurrent lint dummy

all:
	cargo build

test:
	cargo test --workspace

test-concurrent:
	cargo test --features concurrent

lint:
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets
	cargo check --workspace --no-default-features
	cargo check --workspace --all-targets --all-features

run_examples: $(patsubst examples/%.rs, target/release/examples/%, $(wildcard examples/*.rs))

target/release/examples/%: dummy
	cargo run --all-features --release --example "$(notdir $@)"

docs:
	RUSTDOCFLAGS="--html-in-header gh-pages/katex-header.html" cargo doc --all --no-deps --document-private-items
