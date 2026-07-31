//! Traits describing algebras, and implementations thereof for different
//! representations of the Steenrod algebra.

pub mod adem_algebra;
pub use adem_algebra::AdemAlgebra;

mod algebra_trait;
pub use algebra_trait::{Algebra, GeneratedAlgebra, MuAlgebra, UnstableAlgebra};

mod bialgebra_trait;
pub use bialgebra_trait::Bialgebra;

pub mod combinatorics;

pub mod field;
pub use field::Field;

pub mod milnor_algebra;
pub use milnor_algebra::MilnorAlgebra;

/// Opt-in: an arithmetic alternative to the Milnor basis index map. Not wired in; see the module
/// docs for what it costs and what it would take to adopt.
#[cfg(feature = "milnor-rank")]
pub mod milnor_rank;

mod steenrod_algebra;
pub use steenrod_algebra::{AlgebraType, SteenrodAlgebra};

pub mod pair_algebra;
