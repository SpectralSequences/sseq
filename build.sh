#!/bin/sh

TARGET="wasm32-unknown-unknown"
NAME="steenrod_calculator"

echo "Building crate"
cargo build --target $TARGET --release

echo "Running wasm-bindgen"
wasm-bindgen --no-typescript --target no-modules --out-dir dist --out-name "$NAME"_wasm target/$TARGET/release/$NAME.wasm

if [ -x "$(command -v wasm-opt)" ]; then
    echo "Running wasm-opt"
    wasm-opt -Os dist/"$NAME"_wasm_bg.wasm -o dist/"$NAME"_wasm_bg.wasm
fi
