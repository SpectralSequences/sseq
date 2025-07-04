# ext-rs

![ext](https://github.com/spectralsequences/sseq/actions/workflows/ext.yaml/badge.svg)

`ext-rs` is a collection of libraries for doing homological algebra over Fp.
The main and original purpose is to compute Ext of a Steenrod module, but the
library is written to be sufficiently generic to deal with more general
applications.

The library also comes with a number of "example" binaries that use the library
for various purposes. These can be used directly to perform specific
computations, or act as actual examples for how to use the library.

Since `ext-rs` is based on [Rust](https://www.rust-lang.org/), one must install
Rust before using this library.

## Quickstart
### Installing `sseq`

1. **Install Git**  (if not already installed)
   - On Windows: Install [Git for Windows](https://git-scm.com/downloads/win)
   - On macOS: git will be installed automatically when you type a git command in the terminal
   - On Linux: use your package manager

2. **Install Rust** (if not already installed)
   Install Rust via rustup: [download the installer here](https://www.rust-lang.org/tools/install).
   Run the installer and accept the defaults.

3. **Clone the repository**  
   - Open a new terminal. (On windows: hit `Windows + r`, type `cmd`, and hit
   enter to get a command prompt.)
   - Type `git clone --depth 1 https://github.com/SpectralSequences/sseq`
   and hit enter.

4. **Navigate to the project folder**  
   In your terminal type `cd sseq/ext` and hit enter.

5. Now you're ready to run the example in the "First usage" section.

   **Troubleshooting:**
   - If you get a command not found error when running a `cargo` command, make
     sure you're using a new terminal window (not the one you used to install rust).
   - If it says that `Cargo.toml` is not found, then you may have skipped Step 4.

### First usage: how to compute the Adams E2 term

To produce an svg chart of the stable Adams spectral sequence E2 term for a
given module, run:

```shell
cargo run --example chart
```

On your first run, you can just accept the defaults (the 2-primary sphere in a
modest range of degrees) by pressing enter. Other preset Steenrod modules can
be found in the `ext/steenrod_modules/` directory; enter their name (e.g. `C2`)
in the first prompt. If the module you want is not in this list, read the file
`ext/MODULE-SPEC.md` in this repository which explains how to construct your own.

More essential end-user documentation, such as command-line options that will
speed up runtimes, can be found at
[https://spectralsequences.github.io/sseq/docs/ext/](https://spectralsequences.github.io/sseq/docs/ext/).

### Notable functionality
In addition to the standard Ext, more runnable code can be found in
`ext/examples`. All of the "examples" can be run in a similar way as `chart`.
In particular, we highlight the following notable functionality:

* Ext over the odd-primary Steenrod algebra (start by using the module `S_3`
  for the 3-primary sphere in the `chart` example above)
* Adams d2's via the secondary Steenrod algebra (see the `secondary` example)
* Massey products (see the `massey` example)
* Unstable Adams E2 term for spheres (see the `unstable_chart` example)


## Library documentation

Documentation for both the examples and the library itself is hosted at
[https://spectralsequences.github.io/sseq/docs/ext/](https://spectralsequences.github.io/sseq/docs/ext/).

If documentation for a specific version of the library is sought, it can be generated by running

```shell
make docs
```

after which the documentation is placed at `target/doc/`. The link above opens
the file `target/doc/ext/index.html`.

## Development

### Linting

Lint scripts can be run with

```shell
make lint
```

This runs `clippy` and `rustfmt`.

### Tests

There are multiple types of tests

#### Unit tests, doc tests and integration tests

These can be run by

```shell
make test
```

#### Example benchmarks

In the `examples/benchmarks/` folder, we have some benchmarks to test the
examples against. In each file, the first row specifies the argument to run
(the arguments after `cargo run --example`), and the rest of the file is what
the output should be.

These can be run by

```shell
make benchmarks
make benchmarks-concurrent
```

Running

```shell
make fix-benchmarks
```

updates the outputs in all benchmark files to match the current program output.
This is useful for adding new benchmarks.

These can be run for individual benchmarks by e.g.

```shell
make examples/benchmarks/resolve-S_2
make examples/benchmarks/resolve-S_2-concurrent
make examples/benchmarks/resolve-S_2-fixed
```
