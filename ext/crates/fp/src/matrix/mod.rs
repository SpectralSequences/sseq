// mod basis;
mod matrix_inner;
mod quasi_inverse;
mod subquotient;
mod subspace;

// pub use basis::Basis;
pub use matrix_inner::{AugmentedMatrix2, AugmentedMatrix3, Matrix, MatrixSliceMut};
pub use quasi_inverse::QuasiInverse;
pub use subquotient::Subquotient;
pub use subspace::Subspace;
