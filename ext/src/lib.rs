//! `ext-rs` is a collection of libraries for doing homological algebra over Fp. The main and
//! original purpose is to compute Ext of a Steenrod module, but the library is written to be
//! sufficiently generic to deal with more general applications.
//!
//! The library also comes with a number of "example" binaries that use the library for various
//! purposes. These can be used directly to perform specific computations, or act as actual
//! examples for how to use the library.
//!
//! This contains a number of sub-crates, which each have their own documentation. A brief overview
//! is as follows:
//!
//! ## algebra
//! This defines algebras, modules and module homomorphisms
//!
//! ## bivec
//! This is a small crate that provides [`BiVec`](`bivec::BiVec`) - a variant of [`Vec`] indexed by an
//! `i32` whose starting index may be non-zero.
//!
//! ## chart
//! This provides some APIs for generating charts
//!
//! ## error
//! Our bespoke error library
//!
//! ## fp
//! This implements linear algebra over $\mathbb{F}_p$, as well as general helper functions about
//! primes.
//!
//! ## once
//! This is a small crate that provides `OnceVec` and `OnceBiVec`, a wrapper around
//! `UnsafeCell<Vec>` (or `BiVec`) that models a `Vec` whose only way of modification is `push`.
//! This models some partially computed infinite data structure, and we think of pushing as simply
//! finding out more of this infinite data structure instead of genuinely mutating it.
//!
//! ## query
//! This contains some helper functions for a command line interface.
//!
//! ## saveload
//! This provides an interface for saving and loading resolutions and other data.
//!
//! ## thread-token
//! This provides some concurrency primitives

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
