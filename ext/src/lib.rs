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

pub use algebra::combinatorics;
pub use algebra::module::block_structure;
pub use algebra;
pub use algebra::change_of_basis;
pub use algebra::steenrod_parser;
pub use algebra::steenrod_evaluator;
pub use algebra::module;
pub use algebra::cli_module_loaders;

pub mod chain_complex;
pub mod resolution;
pub mod resolution_homomorphism;
pub mod yoneda;

use crate::chain_complex::FiniteChainComplex;
use crate::module::FiniteModule;
use crate::module::homomorphism::FiniteModuleHomomorphism;
pub type CCC = FiniteChainComplex<FiniteModule, FiniteModuleHomomorphism<FiniteModule>>;

pub mod utils;
pub mod secondary;
