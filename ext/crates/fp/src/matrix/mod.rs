// mod basis;
mod affine;
mod matrix_inner;
mod quasi_inverse;
mod subquotient;
mod subspace;

mod m4ri;

// pub use basis::Basis;
pub use affine::AffineSubspace;
pub use matrix_inner::{AugmentedMatrix, Matrix, MatrixSliceMut};
pub use quasi_inverse::QuasiInverse;
pub use subquotient::Subquotient;
pub use subspace::Subspace;
#[cfg(feature = "proptest")]
pub use {matrix_inner::arbitrary::*, subquotient::arbitrary::*, subspace::arbitrary::*};
