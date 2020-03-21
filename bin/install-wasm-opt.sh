#!/bin/sh

VERSION=91
TARGET="x86_64-linux"

if [ -x "$(command -v wasm-opt)" ]; then
    echo "wasm-opt already present. Exitting"
    exit 0
fi

echo "Downloading wasm-opt"
if [ ! -d $HOME/bin ]; then
    mkdir $HOME/bin 
    export SHELL_RESTART="1"
fi

TMP_DIR=$(mktemp -d)
cd $TMP_DIR
wget https://github.com/WebAssembly/binaryen/releases/download/version_"$VERSION"/binaryen-version_"$VERSION"-"$TARGET".tar.gz
tar -xzf binaryen-version_"$VERSION"-"$TARGET".tar.gz
echo "Installing wasm-opt to $HOME/bin"
cp binaryen-version_$VERSION/wasm-opt $HOME/bin

if [ -n "$SHELL_RESTART" ]; then
    echo "Restart your shell to finish installation."
fi

