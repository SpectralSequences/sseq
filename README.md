# Steenrod Calculator
This is a Steenrod calculator. It takes in an arbitrary expression in the
Steenrod algebra and expresses it in your favorite basis.

## Set up
We assume rust is already installed. Install wasm-pack from cargo (or via your
package manager):
```console
 $ cargo install wasm-pack
```
Clone [ext](https://github.com/SpectralSequences/ext) and symlink `ext/libraries` into `steenrod_calculator/ext`.

## Build and run
Build and serve with
```
 $ ./build.sh
 $ ./serve.sh [port]
```
This serves the website at `locahost:[port]`. The `[port]` argument is optional and defaults to `8000`.
