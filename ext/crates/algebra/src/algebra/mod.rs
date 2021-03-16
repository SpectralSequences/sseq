#![cfg_attr(rustfmt, rustfmt_skip)]
pub mod combinatorics;

mod algebra_trait;
mod bialgebra_trait;
pub mod field;
mod polynomial_algebra;
mod steenrod_algebra;
pub mod adem_algebra;
pub mod milnor_algebra;

pub use algebra_trait::Algebra;
pub use bialgebra_trait::Bialgebra;
pub use field::Field;
pub use steenrod_algebra::{SteenrodAlgebra, SteenrodAlgebraT, SteenrodAlgebraBorrow};
pub use adem_algebra::{AdemAlgebra, AdemAlgebraT};
pub use milnor_algebra::{MilnorAlgebra, MilnorAlgebraT};
pub use polynomial_algebra::{PolynomialAlgebra, PolynomialAlgebraMonomial, PolynomialAlgebraTableEntry};
