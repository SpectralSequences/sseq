#![allow(clippy::many_single_char_names)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]
#![allow(clippy::implicit_hasher)]
#![warn(clippy::default_trait_access)]
#![warn(clippy::if_not_else)]
#![warn(clippy::needless_continue)]
#![warn(clippy::redundant_closure_for_method_calls)]
#![warn(clippy::explicit_iter_loop)]
#![warn(clippy::explicit_into_iter_loop)]

pub mod combinatorics;
pub mod block_structure;
pub mod algebra;
pub mod change_of_basis;
pub mod steenrod_parser;
pub mod steenrod_evaluator;
pub mod module;
pub mod chain_complex;
pub mod resolution;
pub mod resolution_homomorphism;
pub mod cli_module_loaders;
pub mod yoneda;

use crate::chain_complex::FiniteChainComplex;
use crate::module::FiniteModule;
use crate::module::homomorphism::FiniteModuleHomomorphism;
pub type CCC = FiniteChainComplex<FiniteModule, FiniteModuleHomomorphism<FiniteModule>>;

pub mod utils;
