use std::sync::Arc;

use bivec::BiVec;
use fp::{
    matrix::{Matrix, QuasiInverse, Subspace},
    vector::SliceMut,
};
use once::OnceBiVec;

use crate::{
    algebra::Algebra,
    module::{
        homomorphism::{IdentityHomomorphism, ModuleHomomorphism, ZeroHomomorphism},
        Module,
    },
};

/// A ModuleHomomorphism that simply records the matrix of the homomorphism in every degree.
/// This is currently rather bare bones.
pub struct FullModuleHomomorphism<S: Module, T: Module<Algebra = S::Algebra> = S> {
    source: Arc<S>,
    target: Arc<T>,
    degree_shift: i32,
    /// The matrices of the module homomorphism. Unspecified matrices are assumed to be zero
    matrices: OnceBiVec<Matrix>,
    quasi_inverses: OnceBiVec<QuasiInverse>,
    kernels: OnceBiVec<Subspace>,
    images: OnceBiVec<Subspace>,
}

impl<S: Module, T: Module<Algebra = S::Algebra>> Clone for FullModuleHomomorphism<S, T> {
    fn clone(&self) -> Self {
        Self {
            source: Arc::clone(&self.source),
            target: Arc::clone(&self.target),
            degree_shift: self.degree_shift,
            matrices: self.matrices.clone(),
            quasi_inverses: self.quasi_inverses.clone(),
            kernels: self.kernels.clone(),
            images: self.images.clone(),
        }
    }
}

impl<S: Module, T: Module<Algebra = S::Algebra>> ModuleHomomorphism
    for FullModuleHomomorphism<S, T>
{
    type Source = S;
    type Target = T;

    fn source(&self) -> Arc<Self::Source> {
        Arc::clone(&self.source)
    }

    fn target(&self) -> Arc<Self::Target> {
        Arc::clone(&self.target)
    }

    fn degree_shift(&self) -> i32 {
        self.degree_shift
    }

    fn apply_to_basis_element(
        &self,
        mut result: SliceMut,
        coeff: u32,
        input_degree: i32,
        input_idx: usize,
    ) {
        let output_degree = input_degree - self.degree_shift;
        if let Some(matrix) = self.matrices.get(output_degree) {
            result.add(matrix[input_idx].as_slice(), coeff);
        }
    }

    fn image(&self, degree: i32) -> Option<&Subspace> {
        self.images.get(degree)
    }

    fn quasi_inverse(&self, degree: i32) -> Option<&QuasiInverse> {
        self.quasi_inverses.get(degree)
    }

    fn kernel(&self, degree: i32) -> Option<&Subspace> {
        self.kernels.get(degree)
    }

    fn compute_auxiliary_data_through_degree(&self, degree: i32) {
        let degree = std::cmp::min(degree, self.matrices.len() - 1);
        self.kernels.extend(degree, |i| {
            let (image, kernel, qi) = self.auxiliary_data(i);
            self.images.push_checked(image, i);
            self.quasi_inverses.push_checked(qi, i);
            kernel
        });
    }
}

impl<A, S, T> FullModuleHomomorphism<S, T>
where
    A: Algebra,
    S: Module<Algebra = A>,
    T: Module<Algebra = A>,
{
    pub fn new(source: Arc<S>, target: Arc<T>, degree_shift: i32) -> Self {
        let min_degree = source.min_degree();
        Self::from_matrices(source, target, degree_shift, BiVec::new(min_degree))
    }

    pub fn from_matrices(
        source: Arc<S>,
        target: Arc<T>,
        degree_shift: i32,
        matrices: BiVec<Matrix>,
    ) -> Self {
        let min_degree = target.min_degree();
        Self {
            source,
            target,
            degree_shift,
            matrices: OnceBiVec::from_bivec(matrices),
            quasi_inverses: OnceBiVec::new(min_degree),
            kernels: OnceBiVec::new(min_degree),
            images: OnceBiVec::new(min_degree),
        }
    }

    pub fn from<F: ModuleHomomorphism<Source = S, Target = T>>(f: &F) -> Self {
        let source = f.source();
        let target = f.target();
        let degree_shift = f.degree_shift();
        let p = f.prime();

        let min_degree = f.target().min_degree();
        let max_degree = f
            .source()
            .max_degree()
            .expect("FullModuleHomomorphism::from requires source to be bonuded")
            - degree_shift;

        source.compute_basis(max_degree);
        target.compute_basis(max_degree);

        let matrices = OnceBiVec::new(min_degree);

        for target_deg in min_degree..=max_degree {
            let source_deg = target_deg + degree_shift;
            // Here we use `Module::dimension(&*m, i)` instead of `m.dimension(i)` because there are
            // multiple `dimension` methods in scope and rust-analyzer gets confused if we're not
            // explicit enough.
            let source_dim = Module::dimension(&*source, source_deg);
            let target_dim = Module::dimension(&*target, target_deg);

            let mut matrix = Matrix::new(p, source_dim, target_dim);
            f.get_matrix(matrix.as_slice_mut(), source_deg);
            matrices.push_checked(matrix, target_deg);
        }

        Self {
            source,
            target,
            degree_shift,
            matrices,
            quasi_inverses: OnceBiVec::new(min_degree),
            kernels: OnceBiVec::new(min_degree),
            images: OnceBiVec::new(min_degree),
        }
    }

    /// This function replaces the source of the ModuleHomomorphism and does nothing else.
    /// This is useful for changing the type of the source (but not the mathematical module
    /// itself).
    pub fn replace_source<S_: Module<Algebra = A>>(
        self,
        source: Arc<S_>,
    ) -> FullModuleHomomorphism<S_, T> {
        FullModuleHomomorphism {
            source,
            target: self.target,
            degree_shift: self.degree_shift,
            matrices: self.matrices,
            quasi_inverses: self.quasi_inverses,
            kernels: self.kernels,
            images: self.images,
        }
    }

    /// See `replace_source`
    pub fn replace_target<T_: Module<Algebra = A>>(
        self,
        target: Arc<T_>,
    ) -> FullModuleHomomorphism<S, T_> {
        FullModuleHomomorphism {
            source: self.source,
            target,
            degree_shift: self.degree_shift,
            matrices: self.matrices,
            quasi_inverses: self.quasi_inverses,
            kernels: self.kernels,
            images: self.images,
        }
    }
}

impl<S: Module, T: Module<Algebra = S::Algebra>> ZeroHomomorphism<S, T>
    for FullModuleHomomorphism<S, T>
{
    fn zero_homomorphism(source: Arc<S>, target: Arc<T>, degree_shift: i32) -> Self {
        Self::new(source, target, degree_shift)
    }
}

impl<S: Module> IdentityHomomorphism<S> for FullModuleHomomorphism<S, S> {
    fn identity_homomorphism(source: Arc<S>) -> Self {
        let p = source.prime();
        let min_degree = source.min_degree();
        let max_degree = source
            .max_degree()
            .expect("FullModuleHomomorphism::identity_homomorphism requires a bounded module");

        let mut matrices = BiVec::with_capacity(min_degree, max_degree + 1);

        for i in min_degree..=max_degree {
            let dim = source.dimension(i);
            let mut matrix = Matrix::new(p, dim, dim);
            for k in 0..dim {
                matrix[k].set_entry(k, 1);
            }
            matrices.push(matrix);
        }

        Self::from_matrices(Arc::clone(&source), source, 0, matrices)
    }
}
