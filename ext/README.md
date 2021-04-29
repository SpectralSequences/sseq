# ext-rs

![rust_ext](https://github.com/spectralsequences/sseq/actions/workflows/rust_ext.yaml/badge.svg)

`ext-rs` is a collection of libraries for doing homological algebra over Fp.
The main and original purpose is to compute Ext of a Steenrod module, but the
library is written to be sufficiently generic to deal with more general
applications.

The library also comes with a number of "example" binaries that use the library
for various purposes. These can be used directly to perform specific
computations, or act as actual examples for how to use the library.

# Examples
Examples are found in the `examples/` directory, and detailed usage guides are
at the topmost comment block. In this section we go over elements common to all
examples.

## Running an example
To run any example, [Rust](https://www.rust-lang.org/) must be installed. We
use nightly builds, but rust should automatically take care of that.

An example can be run by executing the command
```sh
 $ cargo run --example EXAMPLE_NAME
```

There are various flags that can be useful to supply:

 - `--release` compiles the code in release mode. This increases compilation
   time, but results in a much faster binary. This also disables some expensive
   run-time sanity checks.
 - `--no-default-features` disables support for odd primes.
 - `--features concurrent` compiles the program with multi-threading support.

These are supplied right after `cargo run`, in any order. In general, one
should set all of these flags for any non-trivial calculation at the prime 2.

Each example runs interactively, and prompts the user for input. For example,
the following session computes all filtration one products in Ext(C2, k) and
prints them to `stdout`:

```sh
 $ cargo run --features concurrent --example filtration_one
Module (default: S_2): C2
Resolution save file (optional):
Number of threads (default: 2):
Max s (default: 7): 20
Max f (default: 30): 40
```
In each line, the text after the final `:` is input by the user.

In general, we write the output to `stdout` and the prompts to `stderr`. This
allows the user to redirect the output to another file or program.

## Prompts and arguments
Each prompt asks the user for an input, which is submitted by pressing the
Enter/Return key. If the input is invalid, an error message is produced and the
user is queried for the same input again. To exit the program early, one sends
a `SIGTERM`, e.g. via `Ctrl-C`.

Some prompts are optional or have default values. To select the `None` or
default option, simply supply an empty input.

To facilitate batch processing, answers to the prompt can be supplied as
command line arguments instead; the nth command line argument is treated as the
answer to the nth prompt. For example, the previous interaction can be called by
```sh
 $ cargo run --features concurrent --example filtration_one -- C2 "" 2 20 40
```

## Conventions
### Number of threads
The "number of threads" argument is a special case; one can supply it by
setting the `EXT_THREADS` environment variable. If a valid value is set, then
the user is not prompted for input.

The rationale for this behaviour is that this option is only present with
`concurrent` is enabled. By allowing users to set this via an environment
variable, the same arguments can be supplied to concurrent and non-concurrent
versions.

### Module specification
Each Steenrod module is defined in a `json` file, and a collection of such
modules are available in the `steenrod_modules/` subdirectory. New modules can
be defined using the `define_module` example.

Modules are specified using their file names, excluding the `.json` extension.
Module files are searched in the following order:

 1. The current working directory
 2. The `steenrod_modules/` subdirectory of the current directory
 3. The fixed directory `ext/steenrod_modules/` relative to the repository.

For example, the module defined by `steenrod_modules/Ceta.json` can be
specified with the name `Ceta`. It is possible to apply a degree shift to the
module without having to define a new one. For example, to shift `Ceta` by one,
we supply `Ceta[1]`.

When resolving a module, we have to pick a basis of the Steenrod algebra, which
is either the Adem basis or the Milnor basis. The default choice is the Adem
basis, which tends to be faster, but certain applications require the Milnor
basis. In this case we can specify the basis by appending `@basis_name`. For
example, if we want to resolve `Ceta[1]` with the Milnor basis, we can specify
it as `Ceta[1]@milnor`.

### Resolution specification
Most examples act on a (partial) resolution of a module. Usually, these are
supplied as follows:

 1. The program asks for the module to be resolved.
 2. The program asks for a saved resolution of the module, and the user
    supplies a path to the save file, relative to the current directory.
    Generally speaking, the program does not check that this is indeed a
    resolution of the module specified in the first step; supplying the wrong
    save file will lead to nonsensical results.
 3. If a save file is supplied, this is the resolution the example acts on.
    Otherwise the program creates a new resolution and asks for the maximum
    filtration (s) and stem (n) to resolve to. It computes the resolution and
    acts on the result.

The following are two interactions using the different possible options. The
first one specifies a save file while the second does not.

```sh
 $ cargo run --example filtration_one > filtration_one_S_2
Module (default: S_2): S_2
Resolution save file (optional): resolution_S_2.save

 $ cargo run --example filtration_one > filtration_one_C2
Module (default: S_2): C2
Resolution save file (optional):
Max s (default: 7): 20
Max f (default: 30): 40
```

### Ext elements
Each Ext group comes with a basis. The ith basis element of `Ext^{s, n + s}` is
denoted `x_(n, s, i)`. If we want to specify an element in a particular Ext
group, we either write it as a linear combination of the `x_(n, s, i)`, or
written as a vector of the form e.g. `[0, 1, 0]`. In the latter case, we use
our preferred basis and the bidegree is implicit.

### Overview of examples
We give a brief introduction to the examples present. More detailed
explanations are in the top-level comments of each example.

 - `algebra_dim`: Print the dimension of the Steenrod algebra in each degree.
 - `bruner`: Compare our basis with Bruner's.
 - `define_module`: Interactively define a Steenrod module.
 - `differentials`: Print all differentials in the minimal resolution.
 - `filtration_one`: Print all filtration one products.
 - `hidden`: Compute hidden extensions using the output of various other examples.
 - `lift_hom`: Compute the map on Ext induced by a module homomorphism.
 - `massey`: Compute Massey products.
 - `num_gens`: Compute the dimension of Ext in each bidegree.
 - `resolution_size`: Compute the size of the minimal resolution in each bidegree
 - `resolve`: Resolve a module to a fixed (s, t) and potentially save the resolution.
 - `resolve_through_stem`: Resolve a module to a fixed (s, n) and potentially save the resolution.
 - `save_bruner`: Save the resolution in the format used by Bruner's [ext](http://www.rrb.wayne.edu/papers/index.html).
 - `secondary`: Compute d2 differentials using the secondary Steenrod algebra.
 - `steenrod`: Compute Steenrod operations in Ext.
 - `tensor`: Compute the tensor product of two modules.
 - `yoneda`: Compute a Yoneda representative of an Ext class.

# Development
See `DEVELOPMENT.md`.
