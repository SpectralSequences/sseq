#!/bin/sh

wasm-pack build --target no-modules --no-typescript --out-name=steenrod_calculator_wasm --out-dir=dist
rm dist/package.json dist/.gitignore
