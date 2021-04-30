//! `ext-rs` is a collection of libraries for doing homological algebra over $F_p$. The main and
//! original purpose is to compute Ext of a Steenrod module, but the library is written to be
//! sufficiently generic to deal with more general applications.
//!
//! # Subcrates
//! This contains a number of sub-crates, which each have their own documentation. A brief overview
//! is as follows:
//!
//! | Name | Description |
//! | --- | --- |
//! | [`algebra`] | This defines algebras, modules and module homomorphisms |
//! | [`bivec`] | This is a small crate that provides [`BiVec`](`bivec::BiVec`) - a variant of [`Vec`] indexed by an `i32` whose starting index may be non-zero. |
//! | [`chart`] | This provides some APIs for generating charts |
//! | [`error`] | Our bespoke error library |
//! | [`fp`] | This implements linear algebra over $\mathbb{F}_p$, as well as general helper functions about primes. |
//! | [`once`] | This provides `OnceVec` and `OnceBiVec`, a push-only vector with non-blocking reads. This models some partially computed infinite data structure, and we think of pushing as simply finding out more of this infinite data structure instead of genuinely mutating it. |
//! | [`query`] | This contains some helper functions for a command line interface. |
//! | [`saveload`] | This provides an interface for saving and loading resolutions and other data. |
//! | [`thread_token`] | This provides some concurrency primitives |
//!
//! # Examples
//!
//! The library also comes with a number of "example" binaries that use the library for various
//! purposes. These can be used directly to perform specific computations, or act as actual
//! examples for how to use the library.
//!
//! General usage guides can be found in the README; click on the links below for more individual
//! details.
//!
//! | Name | Description |
//! | --- | --- |
//! | [algebra_dim](../algebra_dim/index.html) | Print the dimension of the Steenrod algebra in each degree. |
//! | [`bruner`](../bruner/index.html) | Compare our basis with Bruner's. |
//! | [`define_module`](../define_module/index.html) | Interactively define a Steenrod module. |
//! | [`differentials`](../differentials/index.html) | Print all differentials in the minimal resolution. |
//! | [`filtration_one`](../filtration_one/index.html) | Print all filtration one products. |
//! | [`hidden`](../hidden/index.html) | Compute hidden extensions using the output of various other examples. |
//! | [`lift_hom`](../lift_hom/index.html) | Compute the map on Ext induced by a module homomorphism. |
//! | [`massey`](../massey/index.html) | Compute Massey products. |
//! | [`num_gens`](../num_gens/index.html) | Compute the dimension of Ext in each bidegree. |
//! | [`resolution_size`](../resolution_size/index.html) | Compute the size of the minimal resolution in each bidegree |
//! | [`resolve`](../resolve/index.html) | Resolve a module to a fixed (s, t) and potentially save the resolution. |
//! | [`resolve_through_stem`](../resolve_through_stem/index.html) | Resolve a module to a fixed (s, n) and potentially save the resolution. |
//! | [`save_bruner`](../save_bruner/index.html) | Save the resolution in the format used by Bruner's [ext](http://www.rrb.wayne.edu/papers/index.html). |
//! | [`secondary`](../secondary/index.html) | Compute $d_2$ differentials using the secondary Steenrod algebra. |
//! | [`steenrod`](../steenrod/index.html) | Compute Steenrod operations in Ext. |
//! | [`tensor`](../tensor/index.html) | Compute the tensor product of two modules. |
//! | [`yoneda`](../yoneda/index.html) | Compute a Yoneda representative of an Ext class. |

#![feature(hash_raw_entry)]
#![allow(clippy::upper_case_acronyms)]

pub mod chain_complex;
pub mod resolution;
pub mod resolution_homomorphism;

pub mod yoneda;

use crate::chain_complex::FiniteChainComplex;
use algebra::module::homomorphism::FiniteModuleHomomorphism;
use algebra::module::FiniteModule;
pub type CCC = FiniteChainComplex<FiniteModule, FiniteModuleHomomorphism<FiniteModule>>;

pub mod secondary;
pub mod utils;
