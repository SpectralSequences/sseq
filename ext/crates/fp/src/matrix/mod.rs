#![cfg_attr(rustfmt, rustfmt_skip)]
// mod basis;
mod matrix_inner;
mod quasi_inverse;
mod subspace;
mod subquotient;

// pub use basis::Basis;
pub use matrix_inner::{Matrix, MatrixSliceMut, AugmentedMatrix2, AugmentedMatrix3};
pub use quasi_inverse::QuasiInverse;
pub use subspace::Subspace;
pub use subquotient::Subquotient;

