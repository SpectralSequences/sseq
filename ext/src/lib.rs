//! `ext-rs` is a collection of libraries for doing homological algebra over $\F_p$. The
//! main and original purpose is to compute Ext of a Steenrod module, but the library is written to
//! be sufficiently generic to deal with more general applications.
//!
//! # Examples
//!
//! The library comes with a number of "example" binaries that use the library for various
//! purposes. These can be used directly to perform specific computations, or act as actual
//! examples for how to use the library.
//!
//! ## Running an example
//!
//! An example can be run by executing the command
//! ```sh
//!  $ cargo run --example EXAMPLE_NAME
//! ```
//!
//! There are various flags that can be useful to supply:
//!
//!  - `--release` compiles the code in release mode. This increases compilation
//!    time, but results in a much faster binary. This also disables some expensive
//!    run-time sanity checks.
//!  - `--no-default-features` disables support for odd primes.
//!  - `--features concurrent` compiles the program with multi-threading support. It defaults to
//!    using all CPU cores, and can be configured via `RAYON_NUM_THREADS`.
//!
//! These are supplied right after `cargo run`, in any order. In general, one
//! should set all of these flags for any non-trivial calculation at the prime 2. See the
//! [Features](#features) section for other features that can be enabled.
//!
//! Each example runs interactively, and prompts the user for input. For example,
//! the following session computes all filtration one products in $\Ext(C2, \F_2)$ and
//! prints them to `stdout`:
//!
//! ```sh
//!  $ cargo run --features concurrent --example filtration_one
//! Module (default: S_2): C2
//! Module save directory (optional):
//! Max n (default: 30): 40
//! Max s (default: 7): 20
//! ```
//! In each line, the text after the final `:` is input by the user.
//!
//! In general, we write the output to `stdout` and the prompts to `stderr`. This
//! allows the user to redirect the output to another file or program.
//!
//! ## Prompts and arguments
//! Each prompt asks the user for an input, which is submitted by pressing the
//! Enter/Return key. If the input is invalid, an error message is produced and the
//! user is queried for the same input again. To exit the program early, one sends
//! a `SIGTERM`, e.g. via `Ctrl-C`.
//!
//! Some prompts are optional or have default values. To select the `None` or
//! default option, simply supply an empty input.
//!
//! To facilitate batch processing, answers to the prompt can be supplied as
//! command line arguments instead; the nth command line argument is treated as the
//! answer to the nth prompt. For example, the previous interaction can be called by
//! ```sh
//!  $ cargo run --features concurrent --example filtration_one -- C2 "" 2 40 20
//! ```
//!
//! ## Conventions
//!
//! ### Module specification
//! Each Steenrod module is defined in a `json` file, and a collection of such
//! modules are available in the `steenrod_modules/` subdirectory. New modules can
//! be defined using the [`define_module`](../define_module/index.html) example.
//!
//! Modules are specified using their file names, excluding the `.json` extension.
//! Module files are searched in the following order:
//!
//!  1. The current working directory
//!  2. The `steenrod_modules/` subdirectory of the current directory
//!  3. The fixed directory `ext/steenrod_modules/` relative to the repository.
//!
//! For example, the module defined by `steenrod_modules/Ceta.json` can be
//! specified with the name `Ceta`. It is possible to apply a degree shift to the
//! module without having to define a new one. For example, to shift `Ceta` by one,
//! we supply `Ceta[1]`.
//!
//! When resolving a module, we have to pick a basis of the Steenrod algebra, which
//! is either the Adem basis or the Milnor basis. The default choice is the Milnor
//! basis. We can specify the basis by appending `@basis_name`. For example, if we
//! want to resolve `Ceta[1]` with the Adem basis, we can specify it as `Ceta[1]@adem`.
//!
//! ### Ext elements
//! Each Ext group comes with a basis. The ith basis element of $\Ext^{s, n + s}$ is
//! denoted `x_(n, s, i)`. If we want to specify an element in a particular Ext
//! group, we either write it as a linear combination of the `x_(n, s, i)`, or
//! written as a vector of the form e.g. `[0, 1, 0]`. In the latter case, the bidegree is implicit.
//!
//! ### Save directory
//! For most scripts, one can specify a save directory for each module. All save data relating to
//! the module will be saved in this directory, including resolution data, products, secondary
//! Steenrod algebra computations etc. For products, the data is saved in the save directory of the
//! source of the chain map.
//!
//! In general, the data for each bidegree (or each generator in some cases) is stored in a
//! separate file in an appropriate directory. This lets us only load the data we need when doing a
//! computation, and protects against corruption when the program is terminated halfway through
//! writing (only the data for said bidegree would be corrupted).
//!
//! For products, the subdirectory will be named after the name of the product. One must not reuse
//! a name for different products; the script may produce and write erroneous results silently in
//! such cases (though it practice it is likely to hit some error sooner or later).
//!
//! If the script is compiled with the `zstd` feature, then it supports reading from zstd
//! compressed save files, where each save file is individually compressed. The script will first
//! look for the uncompressed file. If it does not exist, it then looks for the file with the same
//! name but with a `.zst` extension.
//!
//! Note that any new save file will still be written uncompressed. To compress the files, one must
//! run the `zstd` program on each file in the save directory. It is safe to remove the original
//! file after compression (i.e. run with the `--rm` option).
//!
//! # List of examples
//! Click on the individual examples for further information.
//!
//! | Name | Description |
//! | --- | --- |
//! | [algebra_dim](../algebra_dim/index.html) | Print the dimension of the Steenrod algebra in each degree. |
//! | [bruner](../bruner/index.html) | Compare our basis with Bruner's. |
//! | [define_module](../define_module/index.html) | Interactively define a Steenrod module. |
//! | [differentials](../differentials/index.html) | Print all differentials in the minimal resolution. |
//! | [filtration_one](../filtration_one/index.html) | Print all filtration one products. |
//! | [lift_hom](../lift_hom/index.html) | Compute the map $\Ext(N, k) \to \Ext(M, k)$ induced by an element in $\Ext(M, N)$. |
//! | [mahowald_invariant](../mahowald_invariant/index.html) | Compute (algebraic) Mahowald invariants. |
//! | [massey](../massey/index.html) | Compute Massey products. |
//! | [num_gens](../num_gens/index.html) | Compute the dimension of Ext in each bidegree. |
//! | [resolution_size](../resolution_size/index.html) | Compute the size of the minimal resolution in each bidegree |
//! | [resolve](../resolve/index.html) | Resolve a module to a fixed $(s, t)$ and potentially save the resolution. |
//! | [resolve_through_stem](../resolve_through_stem/index.html) | Resolve a module to a fixed $(s, n)$ and potentially save the resolution. |
//! | [save_bruner](../save_bruner/index.html) | Save the resolution in the format used by Bruner's [ext](http://www.rrb.wayne.edu/papers/index.html). |
//! | [secondary](../secondary/index.html) | Compute $d_2$ differentials using the secondary Steenrod algebra. |
//! | [secondary_product](../secondary_product/index.html) | Compute products in $\Mod_{C\lambda^2}$ using the secondary Steenrod algebra. |
//! | [secondary_massey](../secondary_massey/index.html) | Compute Massey products in $\Mod_{C\lambda^2}$ using the secondary Steenrod algebra. |
//! | [steenrod](../steenrod/index.html) | Compute Steenrod operations in Ext. |
//! | [tensor](../tensor/index.html) | Compute the tensor product of two modules. |
//! | [yoneda](../yoneda/index.html) | Compute a Yoneda representative of an Ext class. |
//!
//! # Subcrates
//! This contains a number of sub-crates, which each have their own documentation. A brief overview
//! is as follows:
//!
//! | Name | Description |
//! | --- | --- |
//! | [algebra] | This defines algebras, modules and module homomorphisms |
//! | [bivec] | This is a small crate that provides [`BiVec`](`bivec::BiVec`) - a variant of [`Vec`] indexed by an `i32` whose starting index may be non-zero. |
//! | [chart] | This provides some APIs for generating charts |
//! | [fp] | This implements linear algebra over $\mathbb{F}_p$, as well as general helper functions about primes. |
//! | [once] | This provides `OnceVec` and `OnceBiVec`, a push-only vector with non-blocking reads. This models some partially computed infinite data structure, and we think of pushing as simply finding out more of this infinite data structure instead of genuinely mutating it. |
//! | [query] | This contains some helper functions for a command line interface. |
//!
//! # Features
//!
//! - `odd-primes`: This enables support for odd primes, and is enabled by default. Disabling this
//!   feature offers significant improvements at the prime 2.
//! - `concurrent`: Use multiple threads for computations. The number of threads used can be
//!   configured via the `RAYON_NUM_THREADS` environment variable.
//! - `zstd`: Support reading zstd-compressed save files.
//! - `cache-multiplication`: Precompute and cache the multiplication table under the Milnor basis.
//!    This is only feasible when using a small, finite subalgebra, e.g. when working with
//!    $\mathrm{tmf}$ modules.
//! - `logging`: Print timing information of the computations to stderr.
//! - `nassau`: Use Nassau's algorithm to compute the minimal resolution instead of the usual
//!    minimal resolution algorithm. When this feature is enabled, only finite dimensional modules
//!    at the prime 2 can be resolved.

#![allow(clippy::upper_case_acronyms)]
#![deny(clippy::use_self)]

pub mod chain_complex;
pub mod resolution;
pub mod resolution_homomorphism;
pub mod save;

pub mod yoneda;

use algebra::module::SteenrodModule;

use crate::chain_complex::FiniteChainComplex;
pub type CCC = FiniteChainComplex<SteenrodModule>;

pub mod nassau;
pub mod secondary;
pub mod utils;

// Ensure dependencies don't accidentally activate odd primes
#[cfg(not(feature = "odd-primes"))]
const _: () = assert!(!fp::ODD_PRIMES);
