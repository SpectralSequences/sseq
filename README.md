[![Build Status](https://travis-ci.org/hoodmane/rust_ext.svg?branch=master)](https://travis-ci.org/hoodmane/rust_ext)

# Overview
The main crate provides a library for computing Ext resolutions. It produces a
binary that displays the result in an ASCII graph. This is mainly used for
testing purposes. It also comes with a CLI interface for defining Steenrod
modules, which may be used "in production".

There are three further sub-crates.

## ext-websocket
This is what you should use in general.

The ext-websocket crate uses the rust code as a backend and relays the results
of the computation to the JS frontend via a websocket. This is intended to be
run and used locally --- you don't want to expose a web interface that could
heavily drain your server resources, and relaying the result involves moving a
lot of data. It is usually somewhat reasonable to run the backend on servers in
the local network, and the frontend on your computer, but when the network is
slow, running a browser on the server and using ssh X-forwarding might be a
better idea.

There is also a version that compiles all the rust code into wasm and lets
everything run in the browser. A live and cutting-edge version of this can be
found at
[https://hoodmane.github.io/rust_ext/](https://hoodmane.github.io/rust_ext/).

Read the README file in `ext-websocket/` for more details.

## compressor
This is a utility for further compressing the history file constructed by the
previous interface (again, see the README in `ext-websocket/` for more
details). It is not very well polished. To use it, save the file to compress as
`compressor/old.hist`, and then run `cargo run --release`. The compressed file
will be saved at `compressor/new.hist`.

This program is multithreaded, and to change the number of threads used, edit
the `NUM_THREAD` variable in `compressor/src/main.rs`.

## bivec
This is a small crate that provides `BiVec` - a variant of `Vec` indexed by an
`i32` whose starting index may be non-zero.

# Compilation
To compile the main crate, simply run
```
$ cargo build
```
This will automatically download and manage the dependencies, and the compiled
binary can be found at `target/debug/rust_ext`.

This by default resolves the sphere at p = 2 to degree 30. See `rust_ext
--help` for more configuration options.

Once can also run the resolver directly via
```
$ cargo run
```
This will compile the code (if necessary) and then run the binary. Command line
options can be passed with `--`, e.g. `cargo run -- --help`. In particular,
`cargo run -- module` will start an interactive interface for defining a
module.

To compile and run a properly optimized version, use
```
$ cargo build --release
$ cargo run --release
```
The compiled binaries can be found at `target/release`. This binary is usually
much faster but compilation takes longer.

To run the tests, do
```
$ cargo test
```

## Documentation
To compile the code documentation, run
```
$ cargo doc --no-deps
```
To view the docuemntation, run
```
$ cargo doc --no-deps --open
```
As usual, the latter command triggers the former if needed. This can also be viewed on [https://hoodmane.github.io/rust_ext/doc/rust_ext/](https://hoodmane.github.io/rust_ext/doc/rust_ext/)
