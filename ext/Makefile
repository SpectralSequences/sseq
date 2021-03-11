.PHONY: all test test-concurrent lint dummy

SHELL = bash

LINT = crates/fp/src/prime.rs src/secondary.rs

all:
	cargo build

test:
	cargo test --workspace

test-concurrent:
	cargo test --features concurrent

lint:
	shopt -s globstar
	cargo clippy --workspace --all-targets
	rustfmt --check tests/*.rs examples/*.rs
	rustfmt --check crates/{error,once,query,saveload,thread-token,chart}/**/*.rs
	rustfmt --check $(LINT)

run_examples: $(patsubst examples/%.rs, target/release/examples/%, $(wildcard examples/*.rs))

target/release/examples/%: dummy
	cargo run --all-features --release --example "$(notdir $@)"

docs:
	RUSTDOCFLAGS="--html-in-header gh-pages/katex-header.html" cargo doc --all --no-deps --document-private-items
