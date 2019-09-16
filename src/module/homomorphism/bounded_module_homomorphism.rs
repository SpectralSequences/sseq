use std::sync::{Mutex, Arc};

use crate::fp_vector::{FpVector, FpVectorT};
use crate::matrix::{Matrix, Subspace, QuasiInverse};
use crate::module::{Module, BoundedModule};
use crate::module::homomorphism::{ModuleHomomorphism, ZeroHomomorphism};
use bivec::BiVec;
use once::OnceBiVec;

pub struct BoundedModuleHomomorphism<S : BoundedModule, T : Module> {
    lock : Mutex<()>,
    source : Arc<S>,
    target : Arc<T>,
    degree_shift : i32,
    matrices : BiVec<Matrix>,
    quasi_inverses : OnceBiVec<QuasiInverse>,
    kernels : OnceBiVec<Subspace>
}

impl<S : BoundedModule, T : Module> ModuleHomomorphism for BoundedModuleHomomorphism<S, T> {
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

    fn apply_to_basis_element(&self, result : &mut FpVector, coeff : u32, input_degree : i32, input_idx : usize) {
        let output_degree = input_degree - self.degree_shift;
        if let Some(matrix) = self.matrices.get(output_degree) {
            result.shift_add(&matrix[input_idx], coeff);
        }
    }

    fn quasi_inverse(&self, degree : i32) -> &QuasiInverse {
        &self.quasi_inverses[degree]
    }

    fn kernel(&self, degree : i32) -> &Subspace {
        &self.kernels[degree]
    }

    fn compute_kernels_and_quasi_inverses_through_degree(&self, degree : i32) {
        let lock = self.lock.lock().unwrap();

        let max_degree = std::cmp::min(degree + 1, self.matrices.len());
        let next_degree = self.kernels.len();
        assert_eq!(next_degree, self.quasi_inverses.len());

        for i in next_degree .. max_degree {
            let (kernel, qi) = self.kernel_and_quasi_inverse(i);
            self.kernels.push(kernel);
            self.quasi_inverses.push(qi);
        }
    }
}

impl<S : BoundedModule, T : Module> BoundedModuleHomomorphism<S, T> {
    pub fn from<F : ModuleHomomorphism<Source=S, Target=T>>(f : &F) -> Self {
        let source = f.source();
        let target = f.target();
        let degree_shift = f.degree_shift();
        let p = f.prime();

        let min_degree = f.target().min_degree();
        let max_degree = f.source().max_degree() - degree_shift;

        source.compute_basis(max_degree);
        target.compute_basis(max_degree);

        let mut matrices = BiVec::with_capacity(min_degree, max_degree + 1);

        for target_deg in min_degree ..= max_degree {
            let source_deg = target_deg + degree_shift;
            let source_dim = source.dimension(source_deg);
            let target_dim = target.dimension(target_deg);

            let mut matrix = Matrix::new(p, source_dim, target_dim);
            f.get_matrix(&mut matrix, source_deg, 0, 0);
            matrices.push(matrix);
        }

        BoundedModuleHomomorphism {
            source,
            target,
            degree_shift,
            lock : Mutex::new(()),
            matrices,
            quasi_inverses : OnceBiVec::new(min_degree),
            kernels : OnceBiVec::new(min_degree)
        }
    }

    /// This function replaces the source of the BoundedModuleHomomorphism and does nothing else.
    /// This is useful for changing the type of the source (but not the mathematical module
    /// itself). This is intended to be used in conjunction with `BoundedModule::to_fd_module`
    pub fn replace_source<S_ : BoundedModule>(self, source : Arc<S_>) -> BoundedModuleHomomorphism<S_, T> {
        BoundedModuleHomomorphism {
            source : source,
            target : self.target,
            degree_shift : self.degree_shift,
            lock : self.lock,
            matrices : self.matrices,
            quasi_inverses : self.quasi_inverses,
            kernels : self.kernels
        }
    }

    /// See `replace_source`
    pub fn replace_target<T_ : BoundedModule>(self, target : Arc<T_>) -> BoundedModuleHomomorphism<S, T_> {
        BoundedModuleHomomorphism {
            source : self.source,
            target : target,
            degree_shift : self.degree_shift,
            lock : self.lock,
            matrices : self.matrices,
            quasi_inverses : self.quasi_inverses,
            kernels : self.kernels
        }
    }
}

impl<S: BoundedModule, T : Module> ZeroHomomorphism<S, T> for BoundedModuleHomomorphism<S, T> {
    fn zero_homomorphism(source : Arc<S>, target : Arc<T>, degree_shift : i32) -> Self {
        BoundedModuleHomomorphism {
            source, target, degree_shift,
            lock : Mutex::new(()),
            matrices : BiVec::new(0),
            quasi_inverses : OnceBiVec::new(0),
            kernels : OnceBiVec::new(0)
        }
    }
}
