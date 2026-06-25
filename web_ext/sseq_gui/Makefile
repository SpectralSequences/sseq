WASM_TARGET = wasm32-unknown-unknown
NAME = sseq_gui
WASM_LIB = target/$(WASM_TARGET)/release/$(NAME).wasm
WASM_OUT = dist/
WASM_FILE = $(WASM_OUT)/$(NAME)_wasm_bg.wasm
EXT = ../../ext/

EXT_SRC = $(EXT)/Cargo.toml $(shell find $(EXT)/src/) $(wildcard $(EXT)/crates/*/Cargo.tml) $(shell find $(EXT)/crates/*/src/)

# The prebuilt std for wasm32-unknown-unknown is compiled with `panic=abort`,
# so `-C panic=unwind` alone is silently ignored. To get real unwinding we must
# rebuild std with `-Z build-std` (nightly only) and enable the wasm
# exception-handling proposal.
WASM_RUSTFLAGS = -C panic=unwind -C target-feature=+exception-handling
WASM_BUILD_STD = -Z build-std=std,panic_unwind

export PATH := $(HOME)/.cargo/bin:$(PATH)

lint:
	cargo clippy --all-features --all-targets
	cargo clippy --no-default-features --all-targets
	cargo fmt -- --check

lint-wasm:
	cargo clippy --lib --target $(WASM_TARGET)

setup-wasm:
	cargo install wasm-bindgen-cli --debug
	rustup target add wasm32-unknown-unknown
	# rust-src is required by `-Z build-std` (see the WASM_BUILD_STD note above).
	rustup component add rust-src

$(WASM_LIB): Cargo.toml $(wildcard src/*) $(EXT_SRC)
	RUSTFLAGS="$$RUSTFLAGS $(WASM_RUSTFLAGS)" \
		cargo build --lib --target $(WASM_TARGET) --release $(WASM_BUILD_STD)

$(WASM_FILE): $(WASM_LIB)
	wasm-bindgen --no-typescript --target no-modules --out-dir $(WASM_OUT) --out-name $(NAME)_wasm $(WASM_LIB)
	wasm-opt -O3 --enable-exception-handling $(WASM_FILE) -o $(WASM_FILE) || true

# Assert that the wasm is actually built with unwinding support: a wasm module
# using the exception-handling proposal contains a `Tag` section. If unwinding
# silently regressed to `panic=abort` (e.g. build-std got dropped), this fails.
test-wasm-unwind: $(WASM_LIB)
	wasm-objdump -h $(WASM_LIB) | grep -qi '^ *Tag ' \
		|| { echo "ERROR: $(WASM_LIB) has no Tag section; panic=unwind is not in effect"; exit 1; }
	@echo "OK: wasm built with exception-handling (panic=unwind)"

.PHONY: wasm serve-wasm clean-wasm clean dummy test selenium selenium-update test-wasm-unwind

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
	python3 -m pytest -s tests --driver chrome
	python3 -m pytest -s tests --driver firefox

selenium-update:
	python3 -m pytest tests --driver firefox --update
