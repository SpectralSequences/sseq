WASM_TARGET = wasm32-unknown-unknown
NAME = ext_webserver
WASM_LIB = target/$(WASM_TARGET)/release/$(NAME).wasm
WASM_OUT = dist/
WASM_FILE = $(WASM_OUT)/$(NAME)_wasm_bg.wasm

EXT_SRC = ../ext/Cargo.toml $(shell find ../ext/src/) $(wildcard ../ext/crates/*/Cargo.tml) $(shell find ../ext/crates/*/src/)

export PATH := $(HOME)/.cargo/bin:$(PATH)

lint:
	cargo clippy --all-features --all-targets
	cargo clippy --no-default-features --all-targets
	cargo fmt -- --check

lint-wasm:
	cargo clippy --lib --target $(WASM_TARGET)

setup-wasm:
	cargo install wasm-bindgen-cli --debug --version 0.2.73
	rustup target add wasm32-unknown-unknown

$(WASM_LIB): Cargo.toml $(wildcard src/*) $(EXT_SRC)
	cargo build --lib --target $(WASM_TARGET) --release

$(WASM_FILE): $(WASM_LIB)
	wasm-bindgen --no-typescript --target no-modules --out-dir $(WASM_OUT) --out-name $(NAME)_wasm $(WASM_LIB)
	wasm-opt -O3 $(WASM_FILE) -o $(WASM_FILE) || true

.PHONY: wasm serve-wasm clean-wasm clean dummy run_examples test

wasm: $(WASM_FILE) $(wildcard interface/*) $(wildcard wasm/*) $(wildcard ../ext/steenrod_modules/*)
	# Must be done in this order since both contain index.js and we want the wasm version
	cp interface/* $(WASM_OUT)
	cp wasm/* $(WASM_OUT)
	cp -r ../ext/steenrod_modules $(WASM_OUT)

serve-wasm:
	cd dist && python -m http.server

clean-wasm:
	rm -r dist/*

clean: clean-wasm
	rm -r target

test:
	cargo test

run_examples: $(patsubst examples/%.rs, target/debug/examples/%, $(wildcard examples/*.rs))

target/debug/examples/%: dummy
	cargo run --example "$(notdir $@)"
