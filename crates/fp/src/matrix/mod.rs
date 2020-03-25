mod matrix_inner;
mod basis;
mod quasi_inverse;
mod subspace;

pub use matrix_inner::{Matrix, AugmentedMatrix2, AugmentedMatrix3};
pub use quasi_inverse::QuasiInverse;
pub use subspace::Subspace;
pub use basis::Basis;

