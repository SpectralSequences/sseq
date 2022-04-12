WASM_TARGET = wasm32-unknown-unknown
NAME = sseq_gui
WASM_LIB = target/$(WASM_TARGET)/release/$(NAME).wasm
WASM_OUT = dist/
WASM_FILE = $(WASM_OUT)/$(NAME)_wasm_bg.wasm
EXT = ../../ext/

EXT_SRC = $(EXT)/Cargo.toml $(shell find $(EXT)/src/) $(wildcard $(EXT)/crates/*/Cargo.tml) $(shell find $(EXT)/crates/*/src/)

export PATH := $(HOME)/.cargo/bin:$(PATH)

lint:
	cargo clippy --all-features --all-targets
	cargo clippy --no-default-features --all-targets
	cargo fmt -- --check

lint-wasm:
	cargo clippy --lib --target $(WASM_TARGET)

setup-wasm:
	cargo install wasm-bindgen-cli --debug --version 0.2.78
	rustup target add wasm32-unknown-unknown

$(WASM_LIB): Cargo.toml $(wildcard src/*) $(EXT_SRC)
	cargo build --lib --target $(WASM_TARGET) --release

$(WASM_FILE): $(WASM_LIB)
	wasm-bindgen --no-typescript --target no-modules --out-dir $(WASM_OUT) --out-name $(NAME)_wasm $(WASM_LIB)
	wasm-opt -O3 $(WASM_FILE) -o $(WASM_FILE) || true

.PHONY: wasm serve-wasm clean-wasm clean dummy test selenium selenium-update

wasm: $(WASM_FILE) $(wildcard interface/*) $(wildcard wasm/*) $(wildcard $(EXT)/steenrod_modules/*)
	# Must be done in this order since both contain index.js and we want the wasm version
	cp interface/* $(WASM_OUT)
	cp wasm/* $(WASM_OUT)
	cp vendor/* $(WASM_OUT)
	cp -r $(EXT)/steenrod_modules $(WASM_OUT)

serve-wasm:
	python3 -m http.server --directory dist 8080

clean-wasm:
	rm -r dist/*

clean: clean-wasm
	rm -r target

lint-selenium:
	flake8 --ignore E501 tests
	black --diff --check tests

selenium:
	python3 -m pytest tests --driver firefox
	python3 -m pytest tests --driver chrome

selenium-update:
	python3 -m pytest tests --driver firefox --update
