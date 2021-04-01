#![allow(clippy::many_single_char_names)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]
#![warn(clippy::default_trait_access)]
#![warn(clippy::if_not_else)]
#![warn(clippy::needless_continue)]
#![warn(clippy::redundant_closure_for_method_calls)]
#![warn(clippy::explicit_iter_loop)]
#![warn(clippy::explicit_into_iter_loop)]

mod algebra;
pub mod change_of_basis;
pub mod module;
pub mod steenrod_evaluator;
pub mod steenrod_parser;
//pub mod dense_bigraded_algebra;

pub use crate::algebra::*;
