#![cfg_attr(rustfmt, rustfmt_skip)]
use parking_lot::Mutex;
use std::sync::Arc;

use crate::algebra::Algebra;
use crate::module::homomorphism::{IdentityHomomorphism, ModuleHomomorphism, ZeroHomomorphism};
use crate::module::{BoundedModule, Module};
use bivec::BiVec;
use fp::matrix::{Matrix, QuasiInverse, Subspace};
use fp::vector::SliceMut;
use once::OnceBiVec;

pub struct BoundedModuleHomomorphism<S: BoundedModule, T: Module<Algebra = S::Algebra>> {
    pub lock: Mutex<()>,
    pub source: Arc<S>,
    pub target: Arc<T>,
    pub degree_shift: i32,
    pub matrices: BiVec<Matrix>,
    pub quasi_inverses: OnceBiVec<QuasiInverse>,
    pub kernels: OnceBiVec<Subspace>,
}

impl<S: BoundedModule, T: Module<Algebra=S::Algebra>> Clone for BoundedModuleHomomorphism<S, T> {
    fn clone(&self) -> Self {
        Self {
            lock: Mutex::new(()),
            source: Arc::clone(&self.source),
            target: Arc::clone(&self.target),
            degree_shift: self.degree_shift,
            matrices: self.matrices.clone(),
            quasi_inverses: self.quasi_inverses.clone(),
            kernels: self.kernels.clone()
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

    fn quasi_inverse(&self, degree: i32) -> &QuasiInverse {
        &self.quasi_inverses[degree]
    }

    fn kernel(&self, degree: i32) -> &Subspace {
        &self.kernels[degree]
    }

    fn compute_kernels_and_quasi_inverses_through_degree(&self, degree: i32) {
        let _lock = self.lock.lock();

        let max_degree = std::cmp::min(degree + 1, self.matrices.len());
        let next_degree = self.kernels.len();
        assert_eq!(next_degree, self.quasi_inverses.len());

        for i in next_degree..max_degree {
            let (kernel, qi) = self.kernel_and_quasi_inverse(i);
            self.kernels.push(kernel);
            self.quasi_inverses.push(qi);
        }
    }
}

impl<A, S, T> BoundedModuleHomomorphism<S, T>
where
    A: Algebra,
    S: BoundedModule<Algebra = A>,
    T: Module<Algebra = A>,
{
    pub fn new(
        source: Arc<S>,
        target: Arc<T>,
        degree_shift: i32,
    ) -> Self {
        let p = source.prime();
        let min_degree = source.min_degree();
        let max_degree = source.max_degree();
        source.compute_basis(max_degree);
        target.compute_basis(max_degree + degree_shift);

        let mut matrices = BiVec::with_capacity(min_degree, max_degree + 1);

        for i in min_degree..=max_degree {
            let matrix = Matrix::new(p, source.dimension(i), target.dimension(i + degree_shift));
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
            lock: Mutex::new(()),
            quasi_inverses: OnceBiVec::new(min_degree),
            kernels: OnceBiVec::new(min_degree),
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
            let source_dim = source.dimension(source_deg);
            let target_dim = target.dimension(target_deg);

            let mut matrix = Matrix::new(p, source_dim, target_dim);
            f.get_matrix(&mut matrix.as_slice_mut(), source_deg);
            matrices.push(matrix);
        }

        BoundedModuleHomomorphism {
            source,
            target,
            degree_shift,
            lock: Mutex::new(()),
            matrices,
            quasi_inverses: OnceBiVec::new(min_degree),
            kernels: OnceBiVec::new(min_degree),
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
            lock: self.lock,
            matrices: self.matrices,
            quasi_inverses: self.quasi_inverses,
            kernels: self.kernels,
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
            lock: self.lock,
            matrices: self.matrices,
            quasi_inverses: self.quasi_inverses,
            kernels: self.kernels,
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
            lock: Mutex::new(()),
            matrices: BiVec::new(0),
            quasi_inverses: OnceBiVec::new(0),
            kernels: OnceBiVec::new(0),
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
