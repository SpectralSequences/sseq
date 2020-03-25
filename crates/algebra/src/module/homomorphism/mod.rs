use std::sync::Arc;

use crate::module::Module;
use fp::matrix::{AugmentedMatrix2, Matrix, QuasiInverse, Subspace};
use fp::prime::ValidPrime;
use fp::vector::FpVector;

mod bounded_module_homomorphism;
mod finite_module_homomorphism;
mod fp_module_homomorphism;
mod free_module_homomorphism;
mod generic_zero_homomorphism;
mod hom_pullback;
mod quotient_homomorphism;
mod truncated_homomorphism;

pub use bounded_module_homomorphism::BoundedModuleHomomorphism;
pub use finite_module_homomorphism::FiniteModuleHomomorphism;
pub use fp_module_homomorphism::{FPModuleHomomorphism, FPModuleT};
pub use free_module_homomorphism::FreeModuleHomomorphism;
pub use generic_zero_homomorphism::GenericZeroHomomorphism;
pub use hom_pullback::HomPullback;
pub use quotient_homomorphism::{QuotientHomomorphism, QuotientHomomorphismSource};
pub use truncated_homomorphism::{TruncatedHomomorphism, TruncatedHomomorphismSource};

pub trait ModuleHomomorphism: Send + Sync + 'static {
    type Source: Module;
    type Target: Module<Algebra = <Self::Source as Module>::Algebra>;
    const CUSTOM_QI: bool = false;

    fn source(&self) -> Arc<Self::Source>;
    fn target(&self) -> Arc<Self::Target>;
    fn degree_shift(&self) -> i32;

    /// Calling this function when `input_idx < source().dimension(input_degree)` results in
    /// undefined behaviour. Implementations are encouraged to panic when this happens (this is
    /// usually the case because of out-of-bounds errors.
    fn apply_to_basis_element(
        &self,
        result: &mut FpVector,
        coeff: u32,
        input_degree: i32,
        input_idx: usize,
    );

    fn kernel(&self, degree: i32) -> &Subspace;

    fn quasi_inverse(&self, degree: i32) -> &QuasiInverse;

    fn compute_kernels_and_quasi_inverses_through_degree(&self, degree: i32);

    fn apply(&self, result: &mut FpVector, coeff: u32, input_degree: i32, input: &FpVector) {
        let p = self.prime();
        for (i, v) in input.iter().enumerate() {
            if v == 0 {
                continue;
            }
            self.apply_to_basis_element(result, (coeff * v) % *p, input_degree, i);
        }
    }

    fn prime(&self) -> ValidPrime {
        self.source().prime()
    }

    fn min_degree(&self) -> i32 {
        self.source().min_degree()
    }

    /// Returns the image of the module homomorphism in degree `degree`. If `None`, the image
    /// is the whole space.
    fn image(&self, degree: i32) -> &Option<Subspace> {
        &self.quasi_inverse(degree).image
    }

    /// A version of kernel_and_quasi_inverse that, in fact, doesn't compute the kernel.
    fn calculate_quasi_inverse(&self, degree: i32) -> QuasiInverse {
        let p = self.prime();
        self.source().compute_basis(degree);
        self.target().compute_basis(degree);
        let source_dimension = self.source().dimension(degree);
        let target_dimension = self.target().dimension(degree);
        let mut matrix =
            AugmentedMatrix2::new(p, source_dimension, &[target_dimension, source_dimension]);

        self.get_matrix(&mut *matrix.segment(0, 0), degree);
        matrix.segment(1, 1).set_identity(source_dimension, 0, 0);

        matrix.initialize_pivots();
        matrix.row_reduce();
        matrix.compute_quasi_inverse()
    }

    fn kernel_and_quasi_inverse(&self, degree: i32) -> (Subspace, QuasiInverse) {
        let p = self.prime();
        self.source().compute_basis(degree);
        self.target().compute_basis(degree);
        let source_dimension = self.source().dimension(degree);
        let target_dimension = self.target().dimension(degree);
        let mut matrix =
            AugmentedMatrix2::new(p, source_dimension, &[target_dimension, source_dimension]);

        self.get_matrix(&mut *matrix.segment(0, 0), degree);
        matrix.segment(1, 1).set_identity(source_dimension, 0, 0);

        matrix.initialize_pivots();
        matrix.row_reduce();

        let quasi_inverse = matrix.compute_quasi_inverse();
        let kernel = matrix.compute_kernel();
        (kernel, quasi_inverse)
    }

    /// The (sliced) dimensions of `matrix` must be equal to source_dimension x
    /// target_dimension
    fn get_matrix(&self, matrix: &mut Matrix, degree: i32) {
        if self.target().dimension(degree) == 0 {
            return;
        }

        assert_eq!(self.source().dimension(degree), matrix.rows());
        assert_eq!(self.target().dimension(degree), matrix.columns());

        for (i, row) in matrix.iter_mut().enumerate() {
            self.apply_to_basis_element(row, 1, degree, i);
        }
    }

    fn apply_quasi_inverse(&self, result: &mut FpVector, degree: i32, input: &FpVector) {
        let qi = self.quasi_inverse(degree);
        qi.apply(result, 1, input);
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
