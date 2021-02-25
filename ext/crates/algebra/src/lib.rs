#![macro_use]
#![feature(drain_filter)]
#![feature(try_blocks)]

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

mod algebra;
pub mod change_of_basis;
pub mod steenrod_parser;
pub mod steenrod_evaluator;
pub mod module;
pub mod cli_module_loaders;
//pub mod dense_bigraded_algebra;

pub use crate::algebra::*;
