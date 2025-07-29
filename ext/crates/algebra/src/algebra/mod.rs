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

mod steenrod_algebra;
pub use steenrod_algebra::{AlgebraType, SteenrodAlgebra};

pub mod pair_algebra;
