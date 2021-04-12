pub mod combinatorics;

pub mod adem_algebra;
mod algebra_trait;
mod bialgebra_trait;
pub mod field;
pub mod milnor_algebra;
mod polynomial_algebra;
mod steenrod_algebra;

pub use adem_algebra::{AdemAlgebra, AdemAlgebraT};
#[cfg(feature = "json")]
pub use algebra_trait::JsonAlgebra;
pub use algebra_trait::{Algebra, GeneratedAlgebra};
pub use bialgebra_trait::Bialgebra;
pub use field::Field;
pub use milnor_algebra::{MilnorAlgebra, MilnorAlgebraT};
pub use polynomial_algebra::{
    PolynomialAlgebra, PolynomialAlgebraMonomial, PolynomialAlgebraTableEntry,
};
pub use steenrod_algebra::{SteenrodAlgebra, SteenrodAlgebraBorrow, SteenrodAlgebraT};
