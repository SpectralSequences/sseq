# Rust-based Ext webserver
[![Build Status](https://travis-ci.com/SpectralSequences/rust_webserver.svg?branch=master)](https://travis-ci.com/SpectralSequences/rust_webserver)

## Spectral Sequence Editor
The first part is a spectral sequence editor. It takes as an input a Steenrod
module, and computes the E_2 page using the resolver. One can then add
differentials and mark classes as permanent, which can be propagated via the
Leibniz rule.

This is managed by a rust backend and uses a websocket to communicate to the
web frontend for display.

To build and run, simply run
```console
$ cargo run --release
```
and then navigate to `http://localhost:8080/`.

The source files are located in `interface/` and `src/`.

## Spectral Sequence Editor (without backend)
This is a variation of the previous version, where the rust backend is compiled
to wasm so that everything runs on the browser. This is more convenient for
distribution but is slower. A live version is available at
[https://spectralsequences.github.io/sseq/](https://spectralsequences.github.io/sseq/).

To setup the build environment, run
```console
 $ make setup-wasm
```

Afterwards, build and serve with
```console
 $ make wasm
 $ make serve-wasm
```
This serves the website at `localhost:[port]`. The `[port]` argument is optional and defaults to `8000`.

The final command merely serves the directory `dist/`, which you may serve with
your favorite alternative webserver. There are two points to take note if
you want to serve the directory yourself.

1. By default, most http servers do not serve `.wasm` files with the correct
   MIME type. For example, you need to add the following line to `.htaccess`
   for Apache:
```
AddType application/wasm .wasm
```

2. During compilation, the `dist/` directory is deleted and then re-created, so
   the actual directory changes every time you compile.

The relevant source files are in `wasm/`.
