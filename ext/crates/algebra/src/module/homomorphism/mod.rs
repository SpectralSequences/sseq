use std::sync::Arc;

use fp::{
    matrix::{Matrix, MatrixSliceMut},
    prime::ValidPrime,
};

use crate::{
    algebra::{Algebra, BaseRingOf, Field, Ring, Scalar},
    linear_algebra::{
        BaseSlice, BaseSliceMut, BaseSliceMutOf, BaseSliceOf, GradedDvr, QuasiInverseOf,
        SubmoduleOf,
    },
    module::Module,
};

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
// If `concurrent` is disabled, `MaybeIndexedParallelIterator` implements `Iterator`, and so has an
// `enumerate` method. However, if it is disabled, it does *not* implement `Iterator`, and the
// `enumerate` method is provided by `rayon::prelude::IndexedParallelIterator`, which is loaded in
// the `maybe_rayon` prelude.
#[allow(unused_imports)]
use maybe_rayon::prelude::*;
pub use quotient_homomorphism::{QuotientHomomorphism, QuotientHomomorphismSource};

/// A trait that represents a homomorphism between two modules.
///
/// Each `ModuleHomomorphism` may come with auxiliary data, namely the kernel, image and
/// quasi_inverse at each degree (the quasi-inverse is a map that is a right inverse when restricted
/// to the image). These are computed via
/// [`ModuleHomomorphism::compute_auxiliary_data_through_degree`] and retrieved through
/// [`ModuleHomomorphism::kernel`], [`ModuleHomomorphism::quasi_inverse`] and
/// [`ModuleHomomorphism::image`].
///
/// Note that an instance of a `ModuleHomomorphism` need not have the data available, even after
/// `compute_auxiliary_data_through_degree` is invoked.
pub trait ModuleHomomorphism: Send + Sync
where
    BaseRingOf<<Self::Source as Module>::Algebra>: GradedDvr,
{
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
        result: BaseSliceMutOf<'_, <Self::Source as Module>::Algebra>,
        coeff: Scalar<<Self::Source as Module>::Algebra>,
        input_degree: i32,
        input_idx: usize,
    );

    #[allow(unused_variables)]
    fn kernel(&self, degree: i32) -> Option<&SubmoduleOf<<Self::Source as Module>::Algebra>> {
        None
    }

    #[allow(unused_variables)]
    fn quasi_inverse(
        &self,
        degree: i32,
    ) -> Option<&QuasiInverseOf<<Self::Source as Module>::Algebra>> {
        None
    }

    #[allow(unused_variables)]
    fn image(&self, degree: i32) -> Option<&SubmoduleOf<<Self::Source as Module>::Algebra>> {
        None
    }

    #[allow(unused_variables)]
    fn compute_auxiliary_data_through_degree(&self, degree: i32) {}

    fn apply(
        &self,
        mut result: BaseSliceMutOf<'_, <Self::Source as Module>::Algebra>,
        coeff: Scalar<<Self::Source as Module>::Algebra>,
        input_degree: i32,
        input: BaseSliceOf<'_, <Self::Source as Module>::Algebra>,
    ) {
        let ring = self.target().base_ring();
        for (i, v) in input.iter_nonzero() {
            self.apply_to_basis_element(result.reborrow(), ring.mul(coeff, v), input_degree, i);
        }
    }

    fn prime(&self) -> ValidPrime {
        self.source().prime()
    }

    /// The base ring the homomorphism is linear over (a `Copy` handle, returned by value).
    fn base_ring(&self) -> BaseRingOf<<Self::Source as Module>::Algebra> {
        self.source().base_ring()
    }

    fn min_degree(&self) -> i32 {
        self.source().min_degree()
    }

    /// Compute the auxiliary data associated to the homomorphism at input degree `degree`. Returns
    /// it in the order image, kernel, quasi_inverse
    fn auxiliary_data(
        &self,
        degree: i32,
    ) -> (
        SubmoduleOf<<Self::Source as Module>::Algebra>,
        SubmoduleOf<<Self::Source as Module>::Algebra>,
        QuasiInverseOf<<Self::Source as Module>::Algebra>,
    ) {
        let output_degree = degree - self.degree_shift();
        self.source().compute_basis(degree);
        self.target().compute_basis(output_degree);
        let source_dimension = self.source().dimension(degree);
        let target_dimension = self.target().dimension(output_degree);
        let ring = self.base_ring();
        ring.image_kernel_quasi_inverse(source_dimension, target_dimension, |i, row| {
            self.apply_to_basis_element(row, ring.one(), degree, i);
        })
    }

    /// Write the matrix of the homomorphism at input degree `degree` to `matrix`.
    ///
    /// The (sliced) dimensions of `matrix` must be equal to source_dimension x
    /// target_dimension
    ///
    /// This works with `fp`'s concrete matrices, so it is available only when the base ring is a
    /// field.
    fn get_matrix(&self, mut matrix: MatrixSliceMut, degree: i32)
    where
        <Self::Source as Module>::Algebra: Algebra<BaseRing = Field>,
    {
        assert_eq!(self.source().dimension(degree), matrix.rows());
        assert_eq!(
            self.target().dimension(degree - self.degree_shift()),
            matrix.columns()
        );

        if matrix.columns() == 0 {
            return;
        }

        matrix
            .maybe_par_iter_mut()
            .enumerate()
            .for_each(|(i, row)| {
                self.apply_to_basis_element(row, self.target().base_ring().one(), degree, i)
            });
    }

    /// Get the values of the homomorphism on the specified inputs to `matrix`.
    ///
    /// Available only when the base ring is a field (it produces a concrete `fp` matrix).
    fn get_partial_matrix(&self, degree: i32, inputs: &[usize]) -> Matrix
    where
        <Self::Source as Module>::Algebra: Algebra<BaseRing = Field>,
    {
        let mut matrix = Matrix::new(self.prime(), inputs.len(), self.target().dimension(degree));

        if matrix.columns() == 0 {
            return matrix;
        }

        matrix
            .maybe_par_iter_mut()
            .enumerate()
            .for_each(|(i, row)| {
                self.apply_to_basis_element(row, self.target().base_ring().one(), degree, inputs[i])
            });

        matrix
    }

    /// Attempt to apply quasi inverse to the input. Returns whether the operation was
    /// successful. This is required to either always succeed or always fail for each degree.
    #[must_use]
    fn apply_quasi_inverse(
        &self,
        result: BaseSliceMutOf<'_, <Self::Source as Module>::Algebra>,
        degree: i32,
        input: BaseSliceOf<'_, <Self::Source as Module>::Algebra>,
    ) -> bool {
        if let Some(qi) = self.quasi_inverse(degree) {
            let ring = self.base_ring();
            ring.apply_quasi_inverse(qi, result, ring.one(), input);
            true
        } else {
            false
        }
    }
}

pub trait ZeroHomomorphism<S: Module, T: Module<Algebra = S::Algebra>>:
    ModuleHomomorphism<Source = S, Target = T>
where
    BaseRingOf<S::Algebra>: GradedDvr,
{
    fn zero_homomorphism(s: Arc<S>, t: Arc<T>, degree_shift: i32) -> Self;
}

pub trait IdentityHomomorphism<S: Module>: ModuleHomomorphism<Source = S, Target = S>
where
    BaseRingOf<S::Algebra>: GradedDvr,
{
    fn identity_homomorphism(s: Arc<S>) -> Self;
}
