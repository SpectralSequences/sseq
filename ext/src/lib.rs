#![cfg_attr(rustfmt, rustfmt_skip)]
#![feature(hash_raw_entry)]
#![allow(clippy::many_single_char_names)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]
#![allow(clippy::implicit_hasher)]
#![allow(clippy::upper_case_acronyms)]
#![warn(clippy::default_trait_access)]
#![warn(clippy::if_not_else)]
#![warn(clippy::needless_continue)]
#![warn(clippy::redundant_closure_for_method_calls)]
#![warn(clippy::explicit_iter_loop)]
#![warn(clippy::explicit_into_iter_loop)]

pub mod chain_complex;
pub mod resolution;
pub mod resolution_homomorphism;

#[cfg(feature = "yoneda")]
pub mod yoneda;

use crate::chain_complex::FiniteChainComplex;
use algebra::module::FiniteModule;
use algebra::module::homomorphism::FiniteModuleHomomorphism;
pub type CCC = FiniteChainComplex<FiniteModule, FiniteModuleHomomorphism<FiniteModule>>;

pub mod utils;
pub mod secondary;
