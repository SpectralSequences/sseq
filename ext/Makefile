.PHONY: all test test-concurrent lint dummy

all:
	cargo build

test:
	cargo test --features concurrent --lib --tests --workspace

lint:
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets
	cargo check --workspace --no-default-features
	cargo check --workspace --all-targets --all-features

docs:
	RUSTDOCFLAGS="--html-in-header gh-pages/katex-header.html" cargo doc --all --no-deps --document-private-items

fix-benchmarks: $(patsubst examples/benchmarks/%, examples/benchmarks/%-fixed, $(wildcard examples/benchmarks/*))
examples/benchmarks/%-fixed: dummy
	(head -n 1 examples/benchmarks/$* && sh -c "echo '' | cargo run --features concurrent --example $$(head -n 1 examples/benchmarks/$*)") > $@
	if diff --color examples/benchmarks/$* $@; then \
	    rm $@; \
	else \
	    mv $@ examples/benchmarks/$*; \
	fi

benchmarks: $(wildcard examples/benchmarks/*)
examples/benchmarks/%: dummy
	(head -n 1 $@ && sh -c "echo '' | cargo run --features concurrent --example $$(head -n 1 $@)") | diff --color $@ -
