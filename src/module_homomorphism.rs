use crate::fp_vector::FpVector;
use crate::fp_vector::FpVectorTrait;
use crate::matrix::Matrix;
use crate::matrix::Subspace;
use crate::matrix::QuasiInverse;
use crate::module::Module;

pub trait ModuleHomomorphism {
    fn get_source(&self) -> &Module;
    fn get_target(&self) -> &Module;
    fn apply_to_basis_element(&self, result : &mut FpVector, coeff : u32, input_degree : i32, input_idx : usize);
    
    fn get_prime(&self) -> u32 {
        self.get_source().get_prime()
    }
    
    fn get_matrix(&self, matrix : &mut Matrix, degree : i32) {
        let source_dimension = self.get_source().get_dimension(degree);
        let target_dimension = self.get_target().get_dimension(degree);
        assert!(source_dimension <= matrix.rows);
        assert!(target_dimension <= matrix.columns);
        for input_idx in 0 .. source_dimension {
            // Writing into slice.
            // Can we take ownership from matrix and then put back? 
            // If source is smaller than target, just allow add to ignore rest of input would work here.
            let mut slice = matrix.vectors[input_idx].slice(0, target_dimension);
            self.apply_to_basis_element(&mut slice, 1, degree, input_idx)
        }
    }

    // fn get_kernel(&self, degree : u32) -> &Subspace;
    // fn set_kernel(&self, degree : u32, kernel : &Subspace);

    // fn get_quasi_inverse(&self, degree : u32) -> &QuasiInverse;
    // fn set_quasi_inverse(&self, degree : u32, quasi_inverse : &QuasiInverse);
}

pub struct ZeroHomomorphism<'a> {
    source : &'a Module,
    target : &'a Module
}

impl<'a> ZeroHomomorphism<'a> {
    pub fn new(source : &'a Module, target : &'a Module) -> Self {
        ZeroHomomorphism {
            source,
            target
        }
    }
}

impl<'a> ModuleHomomorphism for ZeroHomomorphism<'a> {
    fn get_source(&self) -> &Module {
        return self.source;
    }

    fn get_target(&self) -> &Module {
        return self.target;
    }

    fn apply_to_basis_element(&self, result : &mut FpVector, coeff : u32, input_degree : i32, input_idx : usize){}


    // fn get_kernel(&self, degree : u32) -> &Subspace;
    // fn set_kernel(&self, degree : u32, kernel : &Subspace);

    // fn get_quasi_inverse(&self, degree : u32) -> &QuasiInverse;
    // fn set_quasi_inverse(&self, degree : u32, quasi_inverse : &QuasiInverse);

}