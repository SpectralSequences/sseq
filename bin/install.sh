#!/bin/bash
export BIN="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
export REPOSITORY_ROOT="$(dirname "$BIN")"
export WORKING_DIRECTORY="$(pwd)"
export EXT_REPOSITORY=$($BIN/_find_ext_repository.sh $1)

if [ -z "$EXT_REPOSITORY" ]; then
    ./_query_clone_ext_repository.sh
else
    ln -s $EXT_REPOSITORY $REPOSITORY_ROOT/ext
fi

echo "Installing wasm-bindgen-cli"
cargo install wasm-bindgen-cli
echo "Installing wasm-opt"
$BIN/install-wasm-opt.sh
echo "Installing rustup target wasm32-unknown-unknown"
rustup target add wasm32-unknown-unknown