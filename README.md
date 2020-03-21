# Steenrod Calculator
[![Build Status](https://travis-ci.com/SpectralSequences/steenrod_calculator.svg?branch=master)](https://travis-ci.com/SpectralSequences/steenrod_calculator)

This is a Steenrod calculator. It takes in an arbitrary expression in the
Steenrod algebra and expresses it in your favorite basis.

A live version is available at [https://spectralsequences.github.io/steenrod_calculator/](https://spectralsequences.github.io/steenrod_calculator/).

## Set up
We assume rust is already installed. Install wasm-bindgen from cargo (or via your
package manager) and clone `ext`:
```console
 $ cargo install wasm-bindgen-cli
 $ git clone --depth 1 https://github.com/SpectralSequences/ext/
```
Note that if `ext` is already cloned elsewhere, one can replace the clone with
a symlink.

One can optionally use `wasm-opt` to optimize the resulting binary. To install `wasm-opt`, run
```console
 $ ./install-wasm-opt.sh
```
The build script will use `wasm-opt` during the build process if it is available.

## Build and run
Build and serve with
```
 $ ./build.sh
 $ ./serve.sh [port]
```
This serves the website at `locahost:[port]`. The `[port]` argument is optional and defaults to `8000`.
