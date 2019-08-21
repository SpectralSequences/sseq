[![Build Status](https://travis-ci.org/hoodmane/rust_ext.svg?branch=master)](https://travis-ci.org/hoodmane/rust_ext)

# Overview
The project is seperated into two parts --- the Rust backend and the javascript/HTML frontend. The Rust backend is compiled into web assembly for use by the javascript, but can also be run and tested separately.

A live and cutting-edge version of the web interface can be found at [https://hoodmane.github.io/rust_ext/](https://hoodmane.github.io/rust_ext/).

# Javascript/webassembly
To download javascript dependencies, run
```
$ npm install
```
This only has to be done once. Afterwards, to compile the web assembly and produce the website, run
```
$ npm run build
```
The files produced will be placed in `dist/` . To view the website, run
```
$ npm run serve
```
and navigate to http://locahost:8000/ . This command merely runs a webserve to serve the directory. There are two points to take note if you want to serve the directory yourself.

1. By default, most http servers do not serve `.wasm` files with the correct MIME type. For example, you need to add the following line to `.htaccess` for Apache:
```
AddType application/wasm .wasm
```

2. During compilation, the `dist/` directory is deleted and then re-created, so the actual directory changes every time you compile.

Note that you do not have to compile the Rust part before compiling the webassembly. In fact, these two operations overwrite each others' files.
# Rust
To compile the Rust part, simply run
```
$ cargo build
```
This will automatically download and manage the dependencies, and the compiled binary can be found at `target/debug/rust_ext`.

This by default resolves the sphere at p = 2 to degree 30. See `rust_ext --help` for more configuration options.

Once can also run the resolver directly via
```
$ cargo run
```
This will compile the code (if necessary) and then run the binary. Command line options can be passed with `--`, e.g. `cargo run -- --help`.

To compile and run a properly optimized version, use
```
$ cargo build --release
$ cargo run --release
```
The compiled binaries can be found at `target/release`. This binary is usually much faster but compilation takes longer.

To run the tests, do
```
$ cargo test
```

To compile the documentation, run
```
$ cargo doc --no-deps
```
To view the docuemntation, run
```
$ cargo doc --no-deps --open
```
As usual, the latter command triggers the former if needed. This can also be viewed on [https://hoodmane.github.io/rust_ext/doc/rust_ext/](https://hoodmane.github.io/rust_ext/doc/rust_ext/)

# Websocket interface

There is an alternative interface that uses the Rust binary to perform calculations. To use this, run
```
$ cargo run --release
```
in the directory `ext-websocket`. There are as before some variations on the exact command you run. After this, navigate to http://localhost:8080/ . This is equipped with an interface to manipulate spectral sequences.

Currently, the redo/undo and load functions have pretty poor performance, both because of the amount of calculation required and also because the JS interface is pretty slow. A significant amount of the CPU usage by the JS interface is in redrawing the canvas, and this can be avoided by navigating to a different tab.
