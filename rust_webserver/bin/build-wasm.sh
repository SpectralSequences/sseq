#!/bin/sh

TARGET="wasm32-unknown-unknown"
NAME="ext_webserver"
WASM_OUT="$NAME"_wasm
OUT_DIR="dist"

WORKING_DIRECTORY="$(pwd)"

cd "$( dirname $0 )"
cd ..

if [ -d "$HOME/.local/bin" ]; then
    PATH="$HOME/.local/bin:$PATH"
fi

if [ -d "$HOME/.cargo/bin" ]; then
    PATH="$HOME/.cargo/bin:$PATH"
fi

mkdir -p $OUT_DIR

echo "Building crate"
cargo build --lib --target $TARGET --release --no-default-features --features odd-primes

echo "Running wasm-bindgen"
wasm-bindgen --no-typescript --target no-modules --out-dir $OUT_DIR --out-name $WASM_OUT target/$TARGET/release/$NAME.wasm

if [ -x "$(command -v wasm-opt)" ]; then
    echo "Running wasm-opt"
    wasm-opt -O3 $OUT_DIR/"$WASM_OUT"_bg.wasm -o $OUT_DIR/"$WASM_OUT"_bg.wasm
fi

echo "Copying static files"
# It is important to do this in this order; both directories contain index.js
# and we want the one from wasm/*
cp interface/* $OUT_DIR
cp wasm/* $OUT_DIR
cp -r ext/steenrod_modules $OUT_DIR/

cd $WORKING_DIRECTORY
