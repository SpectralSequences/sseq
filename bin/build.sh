#!/bin/sh

TARGET="wasm32-unknown-unknown"
NAME="steenrod_calculator"
WASM_OUT="$NAME"_wasm

WORKING_DIRECTORY="$(pwd)"

cd "$( dirname $0 )"
cd ..

if [ -d "$HOME/.local/bin" ]; then
    PATH="$HOME/.local/bin:$PATH"
fi

if [ -d "$HOME/.cargo/bin" ]; then
    PATH="$HOME/.cargo/bin:$PATH"
fi

echo "Building crate"
cargo build --target $TARGET --release

echo "Running wasm-bindgen"
wasm-bindgen --no-typescript --target no-modules --out-dir dist --out-name $WASM_OUT target/$TARGET/release/$NAME.wasm

if [ -x "$(command -v wasm-opt)" ]; then
    echo "Running wasm-opt"
    wasm-opt -Os dist/"$WASM_OUT"_bg.wasm -o dist/"$WASM_OUT"_bg.wasm
fi

cd $WORKING_DIRECTORY
