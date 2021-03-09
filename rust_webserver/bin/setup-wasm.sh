#!/bin/sh

WORKING_DIRECTORY="$(pwd)"

cd "$( dirname $0 )"
cd ..

echo "Installing wasm-bindgen-cli"
cargo install wasm-bindgen-cli

echo "Installing rustup target wasm32-unknown-unknown"
rustup target add wasm32-unknown-unknown

cd $WORKING_DIRECTORY
