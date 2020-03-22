#!/bin/sh

VERSION=91
TARGET="x86_64-linux"

if [ -x "$(command -v wasm-opt)" ]; then
    echo "wasm-opt already present. Exitting"
    exit 0
fi

echo "Downloading wasm-opt"
TMP_DIR=$(mktemp -d)
cd $TMP_DIR
wget https://github.com/WebAssembly/binaryen/releases/download/version_"$VERSION"/binaryen-version_"$VERSION"-"$TARGET".tar.gz

echo "Extracting wasm-opt"
tar -xzf binaryen-version_"$VERSION"-"$TARGET".tar.gz

echo "Installing wasm-opt to $HOME/.local/bin"
if [ ! -d $HOME/.local/bin ]; then
    mkdir $HOME/.local/bin
fi
cp binaryen-version_$VERSION/wasm-opt $HOME/.local/bin
