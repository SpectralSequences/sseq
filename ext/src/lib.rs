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
