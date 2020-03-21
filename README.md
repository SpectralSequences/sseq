# Steenrod Calculator
[![Build Status](https://travis-ci.com/SpectralSequences/steenrod_calculator.svg?branch=master)](https://travis-ci.com/SpectralSequences/steenrod_calculator)

This is a Steenrod calculator. It takes in an arbitrary expression in the
Steenrod algebra and expresses it in your favorite basis.

A live version is available at [https://spectralsequences.github.io/steenrod_calculator/](https://spectralsequences.github.io/steenrod_calculator/).

## Set up
We assume rust is already installed. Run the installation script in `bin/`:
package manager) and clone `ext`:
```console
 $ bin/install.sh
```

One can optionally use `wasm-opt` to optimize the resulting binary. To install `wasm-opt`, run
```console
 $ bin/install-wasm-opt.sh
```
The build script will use `wasm-opt` during the build process if it is available.

## Build and run
Build and serve with
```console
 $ bin/build.sh
 $ bin/serve.sh [port]
```
This serves the website at `localhost:[port]`. The `[port]` argument is optional and defaults to `8000`.
