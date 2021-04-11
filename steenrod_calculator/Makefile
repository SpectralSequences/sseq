WASM_TARGET = wasm32-unknown-unknown
NAME = steenrod_calculator
WASM_LIB = target/$(WASM_TARGET)/release/$(NAME).wasm
WASM_OUT = dist/
WASM_FILE = $(WASM_OUT)/$(NAME)_wasm_bg.wasm

EXT_SRC = ../ext/Cargo.toml $(shell find ../ext/src/) $(wildcard ../ext/crates/*/Cargo.tml) $(shell find ../ext/crates/*/src/)

all: $(WASM_FILE) $(wildcard files/*)
	cp files/* $(WASM_OUT)

lint:
	cargo fmt -- --check
	cargo clippy --lib --target $(WASM_TARGET)

setup-wasm:
	cargo install wasm-bindgen-cli --version 0.2.72
	rustup target add wasm32-unknown-unknown

$(WASM_LIB): Cargo.toml $(wildcard src/*) $(EXT_SRC) rust-toolchain
	cargo build --lib --target $(WASM_TARGET) --release

$(WASM_FILE): $(WASM_LIB)
	wasm-bindgen --no-typescript --target no-modules --out-dir $(WASM_OUT) --out-name $(NAME)_wasm $(WASM_LIB)
	wasm-opt -O3 $(WASM_FILE) -o $(WASM_FILE) || true
