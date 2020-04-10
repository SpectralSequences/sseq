pub mod combinatorics;

mod algebra_trait;
mod steenrod_algebra;
mod bialgebra_trait;
pub mod adem_algebra;
pub mod milnor_algebra;
pub mod field;

pub use algebra_trait::Algebra;
pub use steenrod_algebra::{SteenrodAlgebra, SteenrodAlgebraT, SteenrodAlgebraBorrow};
pub use bialgebra_trait::Bialgebra;
pub use adem_algebra::{AdemAlgebra, AdemAlgebraT};
pub use milnor_algebra::{MilnorAlgebra, MilnorAlgebraT};
pub use field::Field;
