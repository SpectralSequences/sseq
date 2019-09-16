use std::sync::MutexGuard;
use std::sync::Arc;

use crate::fp_vector::{FpVector, FpVectorT};
use crate::matrix::{Matrix, Subspace, QuasiInverse};
use crate::module::{Module, BoundedModule};
use crate::module_homomorphism::{ModuleHomomorphism, ZeroHomomorphismT};
use bivec::BiVec;


pub struct BoundedModuleHomomorphism<S : BoundedModule, T : Module> {
    source : Arc<S>,
    target : Arc<T>,
    degree_shift : i32,
    matrices : BiVec<Matrix>,
    quasi_inverses : BiVec<QuasiInverse>
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

    fn set_quasi_inverse(&self, lock : &MutexGuard<i32>, degree : i32, kernel : QuasiInverse){}

    fn quasi_inverse(&self, degree : i32) -> Option<&QuasiInverse> {
        self.quasi_inverses.get(degree)
    }

    fn lock(&self) -> MutexGuard<i32> { unimplemented!(); }
    fn max_kernel_degree(&self) -> i32 { unimplemented!() }
    fn set_kernel(&self, lock : &MutexGuard<i32>, degree : i32, kernel : Subspace) { unimplemented!() }
    fn kernel(&self, degree : i32) -> Option<&Subspace> { unimplemented!(); }
}

impl<S : BoundedModule, T : Module> BoundedModuleHomomorphism<S, T> {
    pub fn from<F : ModuleHomomorphism<Source=S, Target=T>>(f : F) -> Self {
        let source = f.source();
        let target = f.target();
        let degree_shift = f.degree_shift();
        let p = f.prime();

        let min_degree = f.target().min_degree();
        let max_degree = f.source().max_degree() - degree_shift;

        source.compute_basis(max_degree);
        target.compute_basis(max_degree);

        let mut matrices = BiVec::with_capacity(min_degree, max_degree + 1);
        let mut quasi_inverses = BiVec::with_capacity(min_degree, max_degree + 1);
        for target_deg in min_degree ..= max_degree {
            let source_deg = target_deg + degree_shift;

            let source_dim = source.dimension(source_deg);
            let target_dim = target.dimension(target_deg);
            let padded_target_dim = FpVector::padded_dimension(p, target_dim);

            if source_dim == 0 {
                matrices.push(Matrix::new(p, 0, target_dim));
                quasi_inverses.push(QuasiInverse {
                    image : Some(Subspace::new(p, 0, target_dim)),
                    preimage : Matrix::new(p, 0, 0)
                });
                continue;
            } else if target_dim == 0 {
                matrices.push(Matrix::new(p, source_dim, 0));
                quasi_inverses.push(QuasiInverse {
                    image : None,
                    preimage : Matrix::new(p, 0, source_dim)
                });
                continue;
            }

            let mut matrix_rows = Vec::with_capacity(source_dim);
            for i in 0 .. source_dim {
                let mut result = FpVector::new(p, padded_target_dim + source_dim);

                result.set_slice(0, target_dim);
                f.apply_to_basis_element(&mut result, 1, source_deg, i);
                result.clear_slice();

                result.set_entry(padded_target_dim + i, 1);
                matrix_rows.push(result);
            }
            let mut matrix = Matrix::from_rows(p, matrix_rows);
            let mut pivots = vec![-1; matrix.columns()];

            matrix.row_reduce(&mut pivots);

            quasi_inverses.push(matrix.compute_quasi_inverse(&pivots, target_dim, padded_target_dim));
            matrix.set_slice(0, source_dim, 0, target_dim);
            matrix.into_slice();
            matrices.push(matrix);
        }

        BoundedModuleHomomorphism {
            source,
            target,
            degree_shift,
            matrices,
            quasi_inverses
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
            matrices : self.matrices,
            quasi_inverses : self.quasi_inverses
        }
    }

    /// See `replace_source`
    pub fn replace_target<T_ : BoundedModule>(self, target : Arc<T_>) -> BoundedModuleHomomorphism<S, T_> {
        BoundedModuleHomomorphism {
            source : self.source,
            target : target,
            degree_shift : self.degree_shift,
            matrices : self.matrices,
            quasi_inverses : self.quasi_inverses
        }
    }
}

impl<S: BoundedModule, T : Module> ZeroHomomorphismT<S, T> for BoundedModuleHomomorphism<S, T> {
    fn zero_homomorphism(source : Arc<S>, target : Arc<T>, degree_shift : i32) -> Self {
        BoundedModuleHomomorphism {
            source, target, degree_shift,
            matrices : BiVec::new(0),
            quasi_inverses : BiVec::new(0)
        }
    }
}
