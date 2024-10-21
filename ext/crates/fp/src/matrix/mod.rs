// mod basis;
mod affine;
mod matrix_inner;
mod quasi_inverse;
mod subquotient;
mod subspace;

mod m4ri;

#[cfg(feature = "proptest")]
pub mod arbitrary {
    pub use super::{
        matrix_inner::arbitrary::*, subquotient::arbitrary::*, subspace::arbitrary::*,
    };
}

// pub use basis::Basis;
pub use affine::AffineSubspace;
pub use matrix_inner::{AugmentedMatrix, Matrix, MatrixSliceMut};
pub use quasi_inverse::QuasiInverse;
pub use subquotient::Subquotient;
pub use subspace::Subspace;
