use crate::fp_vector::FpVector;
use crate::matrix::Matrix;
use crate::module::Module;

pub trait ModuleHomomorphism {
    fn get_source(&self) -> &Module;
    fn get_target(&self) -> &Module;

    fn get_min_degree(&self) -> i32 {
        self.get_source().get_min_degree()
    }

    fn apply_to_basis_element(&self, result : &mut FpVector, coeff : u32, input_degree : i32, input_idx : usize);
    
    fn get_prime(&self) -> u32 {
        self.get_source().get_prime()
    }
    
    fn get_matrix(&self, matrix : &mut Matrix, degree : i32, start_row : usize, start_column : usize) -> (usize, usize) {
        let source_dimension = self.get_source().get_dimension(degree);
        let target_dimension = self.get_target().get_dimension(degree);
        assert!(source_dimension <= matrix.rows);
        assert!(target_dimension <= matrix.columns);
        for input_idx in 0 .. source_dimension {
            // Writing into slice.
            // Can we take ownership from matrix and then put back? 
            // If source is smaller than target, just allow add to ignore rest of input would work here.
            let output_vector = &mut matrix.vectors[start_row + input_idx];
            output_vector.set_slice(start_column, start_column + target_dimension);
            self.apply_to_basis_element(output_vector, 1, degree, input_idx);
            output_vector.clear_slice();
        }
        return (start_row + source_dimension, start_column + target_dimension);
    }    
}

pub struct ZeroHomomorphism<'a> {
    source : &'a Module,
    target : &'a Module,
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

    fn apply_to_basis_element(&self, _result : &mut FpVector, _coeff : u32, _input_degree : i32, _input_idx : usize){}

}