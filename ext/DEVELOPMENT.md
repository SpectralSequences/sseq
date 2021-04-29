# Library documentation

Documentation for the library itself is hosted at
[https://spectralsequences.github.io/sseq/docs/ext/](https://spectralsequences.github.io/sseq/docs/ext/).
These can be compiled by running
```
$ make docs
```
after which the documentation is placed at `target/doc/`.

# Linting
Lint scripts can be run with
```
 $ make lint
```
This runs `clippy` and `rustfmt`.

# Tests
There are multiple types of tests

## Unit tests, doc tests and integration tests
These can be run by
```
 $ make test
```

## Example benchmarks
In the `examples/benchmarks/` folder, we have some benchmarks to test the
examples against. In each file, the first row specifies the argument to run
(the arguments after `cargo run --example`), and the rest of the file is what
the output should be.

These can be run by
```
 $ make benchmarks
 $ make benchmarks-concurrent
```

Running
```
 $ make fix-benchmarks
```
updates the outputs in all benchmark files to match the current program output.
This is useful for adding new benchmarks.

These can be run for individual benchmarks by e.g.
```
 $ make examples/benchmarks/resolve-S_2
 $ make examples/benchmarks/resolve-S_2-concurrent
 $ make examples/benchmarks/resolve-S_2-fixed
```
