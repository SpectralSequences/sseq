use std::sync::Arc;

use crate::module::Module;
use fp::matrix::{AugmentedMatrix, Matrix, MatrixSliceMut, QuasiInverse, Subspace};
use fp::prime::ValidPrime;
use fp::vector::{prelude::*, Slice, SliceMut};

#[cfg(feature = "concurrent")]
use rayon::prelude::*;

mod free_module_homomorphism;
mod full_module_homomorphism;
mod generic_zero_homomorphism;
mod hom_pullback;
mod quotient_homomorphism;

pub use free_module_homomorphism::{
    FreeModuleHomomorphism, MuFreeModuleHomomorphism, UnstableFreeModuleHomomorphism,
};
pub use full_module_homomorphism::FullModuleHomomorphism;
pub use generic_zero_homomorphism::GenericZeroHomomorphism;
pub use hom_pullback::HomPullback;
pub use quotient_homomorphism::{QuotientHomomorphism, QuotientHomomorphismSource};

/// Each `ModuleHomomorphism` may come with auxiliary data, namely the kernel, image and
/// quasi_inverse at each degree (the quasi-inverse is a map that is a right inverse when
/// restricted to the image). These are computed via
/// [`ModuleHomomorphism::compute_auxiliary_data_through_degree`] and retrieved through
/// [`ModuleHomomorphism::kernel`], [`ModuleHomomorphism::quasi_inverse`] and
/// [`ModuleHomomorphism::image`].
///
/// Note that an instance of a `ModuleHomomorphism` need not have the data available, even after
/// `compute_auxiliary_data_through_degree` is invoked.
pub trait ModuleHomomorphism: Send + Sync {
    type Source: Module;
    type Target: Module<Algebra = <Self::Source as Module>::Algebra>;

    fn source(&self) -> Arc<Self::Source>;
    fn target(&self) -> Arc<Self::Target>;
    fn degree_shift(&self) -> i32;

    /// Calling this function when `input_idx < source().dimension(input_degree)` results in
    /// undefined behaviour. Implementations are encouraged to panic when this happens (this is
    /// usually the case because of out-of-bounds errors.
    fn apply_to_basis_element(
        &self,
        result: SliceMut,
        coeff: u32,
        input_degree: i32,
        input_idx: usize,
    );

    #[allow(unused_variables)]
    fn kernel(&self, degree: i32) -> Option<&Subspace> {
        None
    }

    #[allow(unused_variables)]
    fn quasi_inverse(&self, degree: i32) -> Option<&QuasiInverse> {
        None
    }

    #[allow(unused_variables)]
    fn image(&self, degree: i32) -> Option<&Subspace> {
        None
    }

    #[allow(unused_variables)]
    fn compute_auxiliary_data_through_degree(&self, degree: i32) {}

    fn apply(&self, mut result: SliceMut, coeff: u32, input_degree: i32, input: Slice) {
        let p = self.prime();
        for (i, v) in input.iter_nonzero() {
            self.apply_to_basis_element(result.copy(), (coeff * v) % *p, input_degree, i);
        }
    }

    fn prime(&self) -> ValidPrime {
        self.source().prime()
    }

    fn min_degree(&self) -> i32 {
        self.source().min_degree()
    }

    /// Compute the auxiliary data associated to the homomorphism at input degree `degree`. Returns
    /// it in the order image, kernel, quasi_inverse
    fn auxiliary_data(&self, degree: i32) -> (Subspace, Subspace, QuasiInverse) {
        let p = self.prime();
        let output_degree = degree - self.degree_shift();
        self.source().compute_basis(degree);
        self.target().compute_basis(output_degree);
        let source_dimension = self.source().dimension(degree);
        let target_dimension = self.target().dimension(output_degree);
        let mut matrix =
            AugmentedMatrix::<2>::new(p, source_dimension, [target_dimension, source_dimension]);

        self.get_matrix(matrix.segment(0, 0), degree);
        matrix.segment(1, 1).add_identity();

        matrix.row_reduce();

        (
            matrix.compute_image(),
            matrix.compute_kernel(),
            matrix.compute_quasi_inverse(),
        )
    }

    /// Write the matrix of the homomorphism at input degree `degree` to `matrix`.
    ///
    /// The (sliced) dimensions of `matrix` must be equal to source_dimension x
    /// target_dimension
    fn get_matrix(&self, mut matrix: MatrixSliceMut, degree: i32) {
        assert_eq!(self.source().dimension(degree), matrix.rows());
        assert_eq!(
            self.target().dimension(degree - self.degree_shift()),
            matrix.columns()
        );

        if matrix.columns() == 0 {
            return;
        }

        #[cfg(not(feature = "concurrent"))]
        for (i, row) in matrix.iter_mut().enumerate() {
            self.apply_to_basis_element(row, 1, degree, i);
        }

        #[cfg(feature = "concurrent")]
        matrix
            .par_iter_mut()
            .enumerate()
            .for_each(|(i, row)| self.apply_to_basis_element(row, 1, degree, i));
    }

    /// Get the values of the homomorphism on the specified inputs to `matrix`.
    fn get_partial_matrix(&self, degree: i32, inputs: &[usize]) -> Matrix {
        let mut matrix = Matrix::new(self.prime(), inputs.len(), self.target().dimension(degree));

        if matrix.columns() == 0 {
            return matrix;
        }

        #[cfg(not(feature = "concurrent"))]
        for (row, &v) in matrix.iter_mut().zip(inputs) {
            self.apply_to_basis_element(row.as_slice_mut(), 1, degree, v);
        }

        #[cfg(feature = "concurrent")]
        matrix.par_iter_mut().enumerate().for_each(|(i, row)| {
            self.apply_to_basis_element(row.as_slice_mut(), 1, degree, inputs[i])
        });

        matrix
    }

    /// Attempt to apply quasi inverse to the input. Returns whether the operation was
    /// successful. This is required to either always succeed or always fail for each degree.
    #[must_use]
    fn apply_quasi_inverse(&self, result: SliceMut, degree: i32, input: Slice) -> bool {
        if let Some(qi) = self.quasi_inverse(degree) {
            qi.apply(result, 1, input);
            true
        } else {
            false
        }
    }
}

pub trait ZeroHomomorphism<S: Module, T: Module<Algebra = S::Algebra>>:
    ModuleHomomorphism<Source = S, Target = T>
{
    fn zero_homomorphism(s: Arc<S>, t: Arc<T>, degree_shift: i32) -> Self;
}

pub trait IdentityHomomorphism<S: Module>: ModuleHomomorphism<Source = S, Target = S> {
    fn identity_homomorphism(s: Arc<S>) -> Self;
}
