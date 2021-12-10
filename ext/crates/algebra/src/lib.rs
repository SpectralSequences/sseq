//! Types and traits for working with various algebras and modules, with
//! a focus on the Steenrod algebra and its modules.

// TODO: Write descriptions of each module therein.

pub mod change_of_basis;
pub mod module;
pub mod steenrod_evaluator;
pub mod steenrod_parser;

//pub mod dense_bigraded_algebra;

mod algebra;
pub use crate::algebra::*;
