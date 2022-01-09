use std::sync::Arc;

use crate::algebra::Algebra;
use crate::module::homomorphism::{IdentityHomomorphism, ModuleHomomorphism, ZeroHomomorphism};
use crate::module::{BoundedModule, Module};
use bivec::BiVec;
use fp::matrix::{Matrix, QuasiInverse, Subspace};
use fp::vector::SliceMut;
use once::OnceBiVec;

pub struct BoundedModuleHomomorphism<S: BoundedModule, T: Module<Algebra = S::Algebra>> {
    pub source: Arc<S>,
    pub target: Arc<T>,
    pub degree_shift: i32,
    pub matrices: BiVec<Matrix>,
    pub quasi_inverses: OnceBiVec<QuasiInverse>,
    pub kernels: OnceBiVec<Subspace>,
    pub images: OnceBiVec<Subspace>,
}

impl<S: BoundedModule, T: Module<Algebra = S::Algebra>> Clone for BoundedModuleHomomorphism<S, T> {
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

impl<S: BoundedModule, T: Module<Algebra = S::Algebra>> ModuleHomomorphism
    for BoundedModuleHomomorphism<S, T>
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
        let degree = std::cmp::min(degree, self.matrices.len() as i32 - 1);
        self.kernels.extend(degree, |i| {
            let (image, kernel, qi) = self.auxiliary_data(i);
            self.images.push_checked(image, i);
            self.quasi_inverses.push_checked(qi, i);
            kernel
        });
    }
}

impl<A, S, T> BoundedModuleHomomorphism<S, T>
where
    A: Algebra,
    S: BoundedModule<Algebra = A>,
    T: Module<Algebra = A>,
{
    pub fn new(source: Arc<S>, target: Arc<T>, degree_shift: i32) -> Self {
        let p = source.prime();
        let min_degree = source.min_degree();
        let max_degree = source.max_degree();
        source.compute_basis(max_degree);
        target.compute_basis(max_degree + degree_shift);

        let mut matrices = BiVec::with_capacity(min_degree, max_degree + 1);

        for i in min_degree..=max_degree {
            // Here we use `Module::dimension(&*m, i)` instead of `m.dimension(i)` because there are
            // multiple `dimension` methods in scope and rust-analyzer gets confused if we're not
            // explicit enough.
            let matrix = Matrix::new(
                p,
                Module::dimension(&*source, i),
                Module::dimension(&*target, i + degree_shift),
            );
            matrices.push(matrix);
        }
        Self::from_matrices(source, target, degree_shift, matrices)
    }

    pub fn from_matrices(
        source: Arc<S>,
        target: Arc<T>,
        degree_shift: i32,
        matrices: BiVec<Matrix>,
    ) -> Self {
        let min_degree = target.min_degree();
        BoundedModuleHomomorphism {
            source,
            target,
            degree_shift,
            matrices,
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
        let max_degree = f.source().max_degree() - degree_shift;

        source.compute_basis(max_degree);
        target.compute_basis(max_degree);

        let mut matrices = BiVec::with_capacity(min_degree, max_degree + 1);

        for target_deg in min_degree..=max_degree {
            let source_deg = target_deg + degree_shift;
            // Here we use `Module::dimension(&*m, i)` instead of `m.dimension(i)` because there are
            // multiple `dimension` methods in scope and rust-analyzer gets confused if we're not
            // explicit enough.
            let source_dim = Module::dimension(&*source, source_deg);
            let target_dim = Module::dimension(&*target, target_deg);

            let mut matrix = Matrix::new(p, source_dim, target_dim);
            f.get_matrix(matrix.as_slice_mut(), source_deg);
            matrices.push(matrix);
        }

        BoundedModuleHomomorphism {
            source,
            target,
            degree_shift,
            matrices,
            quasi_inverses: OnceBiVec::new(min_degree),
            kernels: OnceBiVec::new(min_degree),
            images: OnceBiVec::new(min_degree),
        }
    }

    /// This function replaces the source of the BoundedModuleHomomorphism and does nothing else.
    /// This is useful for changing the type of the source (but not the mathematical module
    /// itself). This is intended to be used in conjunction with `BoundedModule::to_fd_module`
    pub fn replace_source<S_: BoundedModule<Algebra = A>>(
        self,
        source: Arc<S_>,
    ) -> BoundedModuleHomomorphism<S_, T> {
        BoundedModuleHomomorphism {
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
    pub fn replace_target<T_: BoundedModule<Algebra = A>>(
        self,
        target: Arc<T_>,
    ) -> BoundedModuleHomomorphism<S, T_> {
        BoundedModuleHomomorphism {
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

impl<S: BoundedModule, T: Module<Algebra = S::Algebra>> ZeroHomomorphism<S, T>
    for BoundedModuleHomomorphism<S, T>
{
    fn zero_homomorphism(source: Arc<S>, target: Arc<T>, degree_shift: i32) -> Self {
        BoundedModuleHomomorphism {
            source,
            target,
            degree_shift,
            matrices: BiVec::new(0),
            quasi_inverses: OnceBiVec::new(0),
            kernels: OnceBiVec::new(0),
            images: OnceBiVec::new(0),
        }
    }
}

impl<S: BoundedModule> IdentityHomomorphism<S> for BoundedModuleHomomorphism<S, S> {
    fn identity_homomorphism(source: Arc<S>) -> Self {
        let p = source.prime();
        let min_degree = source.min_degree();
        let max_degree = source.max_degree();

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
