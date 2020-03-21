[![Build Status](https://travis-ci.com/SpectralSequences/ext.svg?branch=master)](https://travis-ci.com/SpectralSequences/ext)

# Overview
The purpose of this repository is to compute Ext over the Steenrod algebra, and
facilitate the computation of the corresponding Adams spectral sequence. The
library is written in a way that makes it easy to adapt this to compute Ext
over other algebras.

The repository is separated into two distinct workspaces. `libraries` contains
helper crates and all the homological algebra logic. `binaries` provide several
binaries on top of this, which is mostly glue around the `libraries`. The
reason is that when we use the binary for calculations, we often have to edit
the binary and recompile. Structuring the repository this way allows us to
compile the library with -O3 and the binary with -O0. This massively improves
the compile time with a much smaller runtime overhead.

# Libraries
## Top-level crate
The main crate implements most of the homolgoical algebra.

## fp
This implements linear algebra over Fp, as well as general helper functions
about primes.

## bivec
This is a small crate that provides `BiVec` - a variant of `Vec` indexed by an
`i32` whose starting index may be non-zero.

## once
This is a small crate that provides `OnceVec` and `OnceBiVec`, a wrapper around `UnsafeCell<Vec>` (or `BiVec`) that models a `Vec` whose only way of modification is `push`. This models some partially computed infinite data structure, and we think of pushing as simply finding out more of this infinite data structure instead of genuinely mutating it.

## query
This contains some helper functions for a command line interface.

## saveload
This provides an interface for saving and loading resolutions and other data.

# Binaries
## Top-level crate
By default, the binary computes Ext and displays the result in an ASCII graph.
This is mainly used for testing purposes. It also comes with a CLI interface
for defining Steenrod modules, which may be used "in production".

At the moment, it also has an interface for calculating Steenrod operations in
Ext using the algorithm described in
[https://arxiv.org/abs/1909.03117](https://arxiv.org/abs/1909.03117) via the
`steenrod` subcommand (`cargo run --release steenrod`), but the intention is to
expose this via `ext-websocket` once it is sufficiently presentable (the
current algorithm can be very slow, and the speed cannot be easily determined
a priori).

There is also an alternative entry point, `cargo run test`, which runs
custom-written code in `binaries/src/test.rs`. This is used for ad hoc
calculations, and the content of the this file is probably what the author
happened to be working on when they had to commit something else.

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
[https://spectralsequences.github.io/ext/](https://spectralsequences.github.io/ext/).

Read the README file in `ext-websocket/` for more details.

## compressor
This is a utility for further compressing the history file constructed by the
previous interface (again, see the README in `ext-websocket/` for more
details). It is not very well polished. To use it, save the file to compress as
`compressor/old.hist`, and then run `cargo run --release`. The compressed file
will be saved at `compressor/new.hist`.

This program is multithreaded, and to change the number of threads used, edit
the `NUM_THREAD` variable in `compressor/src/main.rs`.

# Compilation
Most users will have no reason to compile the `libraries` crate apart from
running `cargo test`, `cargo doc` and `cargo clippy` when editing the code.

In general, you will want to work in the `binaries` directory. To compile the
main binary, simply run
```
$ cargo build
```
This will automatically download and manage the dependencies, and the compiled
binary can be found at `target/debug/binary`.

This by default resolves the sphere at p = 2 to degree 30. See `binary
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
As usual, the latter command triggers the former if needed. This can also be viewed on [https://spectralsequences.github.io/ext/doc/rust_ext/](https://spectralsequences.github.io/rust_ext/doc/ext/)
