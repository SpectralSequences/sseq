use std::sync::{Mutex, MutexGuard};
use std::sync::Arc;

use crate::fp_vector::{FpVector, FpVectorT};
use crate::matrix::{Matrix, Subspace, QuasiInverse};
use crate::module::{Module, BoundedModule};
use bivec::BiVec;

pub trait ModuleHomomorphism {
    type Source : Module;
    type Target : Module;

    fn source(&self) -> Arc<Self::Source>;
    fn target(&self) -> Arc<Self::Target>;
    fn degree_shift(&self) -> i32;

    fn min_degree(&self) -> i32 {
        self.source().min_degree()
    }

    /// Calling this function when `input_idx < source().dimension(input_degree)` results in
    /// undefined behaviour. Implementations are encouraged to panic when this happens (this is
    /// usually the case because of out-of-bounds errors.
    fn apply_to_basis_element(&self, result : &mut FpVector, coeff : u32, input_degree : i32, input_idx : usize);

    fn apply(&self, result : &mut FpVector, coeff : u32, input_degree : i32, input : &FpVector){
        let p = self.prime();
        for (i, v) in input.iter().enumerate() {
            if v==0 { continue; }
            self.apply_to_basis_element(result, (coeff * v) % p, input_degree, i);
        }
    }
    
    fn prime(&self) -> u32 {
        self.source().prime()
    }

    fn lock(&self) -> MutexGuard<i32>;

    fn max_kernel_degree(&self) -> i32;

    fn set_kernel(&self, lock : &MutexGuard<i32>, degree : i32, kernel : Subspace);
    fn kernel(&self, degree : i32) -> Option<&Subspace>;

    fn set_quasi_inverse(&self, lock : &MutexGuard<i32>, degree : i32, quasi_inverse : QuasiInverse);    
    fn quasi_inverse(&self, degree : i32) -> Option<&QuasiInverse>;

    fn image(&self, degree : i32) -> Option<&Subspace> {
        let option_quasi_inverse = self.quasi_inverse(degree);
        return option_quasi_inverse.and_then(|quasi_inverse| quasi_inverse.image.as_ref() );
    }

    fn compute_kernels_and_quasi_inverses_through_degree(&self, lock : &MutexGuard<i32>, degree : i32){
        for i in self.max_kernel_degree() + 1 ..= degree {
            self.compute_kernel_and_quasi_inverse(lock, degree);
        }
    }

    fn compute_kernel_and_quasi_inverse(&self, lock : &MutexGuard<i32>, degree : i32){
        let p = self.prime();
        self.source().compute_basis(degree);
        self.target().compute_basis(degree);
        let source_dimension = self.source().dimension(degree);
        let target_dimension = self.target().dimension(degree);
        let padded_target_dimension = FpVector::padded_dimension(p, target_dimension);
        let columns = padded_target_dimension + source_dimension;
        let mut matrix = Matrix::new(p, source_dimension, columns);
        self.get_matrix(&mut matrix, degree, 0, 0);
        for i in 0..source_dimension {
            matrix[i].set_entry(padded_target_dimension + i, 1);
        }
        let mut pivots = vec![-1;columns];
        matrix.row_reduce(&mut pivots);
        let quasi_inverse = matrix.compute_quasi_inverse(&pivots, target_dimension, padded_target_dimension);
        self.set_quasi_inverse(&lock, degree, quasi_inverse);
        let kernel = matrix.compute_kernel(&pivots, padded_target_dimension);
        self.set_kernel(&lock, degree, kernel);
    }
    // fn get_image_pivots(&self, degree : i32) -> Option<&Vec<isize>> {
    //     let image = self.get_image(degree);
    //     return image.map(|subspace| &subspace.column_to_pivot_row );
    // }
    
    fn get_matrix(&self, matrix : &mut Matrix, degree : i32, start_row : usize, start_column : usize) -> (usize, usize) {
        let source_dimension = self.source().dimension(degree);
        let target_dimension = self.target().dimension(degree);
        if target_dimension == 0 {
            return (0, 0);
        }
        assert!(source_dimension <= matrix.rows());
        assert!(target_dimension <= matrix.columns());
        for input_idx in 0 .. source_dimension {
            // Writing into slice.
            // Can we take ownership from matrix and then put back? 
            // If source is smaller than target, just allow add to ignore rest of input would work here.
            let output_vector = &mut matrix[start_row + input_idx];
            let old_slice = output_vector.slice();
            output_vector.set_slice(start_column, start_column + target_dimension);
            self.apply_to_basis_element(output_vector, 1, degree, input_idx);
            output_vector.restore_slice(old_slice);
        }
        return (start_row + source_dimension, start_column + target_dimension);
    }
}

// Maybe we should use static dispatch here? This would also get rid of a bunch of casting.
pub struct ZeroHomomorphism<S : Module, T : Module> {
    source : Arc<S>,
    target : Arc<T>,
    max_degree : Mutex<i32>,
    degree_shift : i32
}

impl<S : Module, T : Module> ZeroHomomorphism<S, T> {
    pub fn new(source : Arc<S>, target : Arc<T>, degree_shift : i32) -> Self {
        let max_degree =  Mutex::new(source.min_degree() - 1);
        ZeroHomomorphism {
            source,
            target,
            max_degree,
            degree_shift
        }
    }
}

impl<S : Module, T : Module> ModuleHomomorphism for ZeroHomomorphism<S, T> {
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

    fn apply_to_basis_element(&self, _result : &mut FpVector, _coeff : u32, _input_degree : i32, _input_idx : usize){}

    fn lock(&self) -> MutexGuard<i32> {
        self.max_degree.lock().unwrap()
    }

    fn max_kernel_degree(&self) -> i32 { 1000000 }

    fn set_quasi_inverse(&self, lock : &MutexGuard<i32>, degree : i32, kernel : QuasiInverse){}
    fn quasi_inverse(&self, degree : i32) -> Option<&QuasiInverse>{ None }

    fn set_kernel(&self, lock : &MutexGuard<i32>, degree : i32, kernel : Subspace){}
    fn kernel(&self, degree : i32) -> Option<&Subspace> { None }
}

pub struct FDModuleHomomorphism<S : BoundedModule, T : Module> {
    source : Arc<S>,
    target : Arc<T>,
    degree_shift : i32,
    matrices : BiVec<Matrix>,
    quasi_inverses : BiVec<QuasiInverse>
}

impl<S : BoundedModule, T : Module> ModuleHomomorphism for FDModuleHomomorphism<S, T> {
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

impl<S : BoundedModule, T : Module> FDModuleHomomorphism<S, T> {
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
            // TODO : throw away the extra columns
            matrix.set_slice(0, source_dim, 0, target_dim);
            matrices.push(matrix);
        }

        FDModuleHomomorphism {
            source,
            target,
            degree_shift,
            matrices,
            quasi_inverses
        }
    }

    /// This function replaces the source of the FDModuleHomomorphism and does nothing else.
    /// This is useful for changing the type of the source (but not the mathematical module
    /// itself). This is intended to be used in conjunction with `BoundedModule::to_fd_module`
    pub fn replace_source<S_ : BoundedModule>(self, source : Arc<S_>) -> FDModuleHomomorphism<S_, T> {
        FDModuleHomomorphism {
            source : source,
            target : self.target,
            degree_shift : self.degree_shift,
            matrices : self.matrices,
            quasi_inverses : self.quasi_inverses
        }
    }

    /// See `replace_source`
    pub fn replace_target<T_ : BoundedModule>(self, target : Arc<T_>) -> FDModuleHomomorphism<S, T_> {
        FDModuleHomomorphism {
            source : self.source,
            target : target,
            degree_shift : self.degree_shift,
            matrices : self.matrices,
            quasi_inverses : self.quasi_inverses
        }
    }
}

pub trait ZeroHomomorphismT<S : Module, T : Module> {
    fn zero_homomorphism(s : Arc<S>, t : Arc<T>, degree_shift : i32) -> Self;
}

impl<S: BoundedModule, T : Module> ZeroHomomorphismT<S, T> for FDModuleHomomorphism<S, T> {
    fn zero_homomorphism(source : Arc<S>, target : Arc<T>, degree_shift : i32) -> Self {
        FDModuleHomomorphism {
            source, target, degree_shift,
            matrices : BiVec::new(0),
            quasi_inverses : BiVec::new(0)
        }
    }
}
