use crate::memory::CVec;
use crate::fp_vector::FpVector;
use crate::matrix::{Matrix, Subspace};
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

    fn copy_kernel_from_matrix(&self, degree : i32, matrix : &mut Matrix, pivots : &CVec<isize>, padded_target_dimension : usize) -> usize {
        let kernel = matrix.compute_kernel(padded_target_dimension, &pivots);
        let kernel_rows = kernel.matrix.get_rows();
        self.set_kernel(degree, kernel);
        return kernel_rows;
    }
    fn set_kernel(&self, degree : i32, kernel : Subspace);
    fn get_kernel(&self, degree : i32) -> Option<&Subspace>;

    fn copy_image_from_matrix(&self, degree : i32, matrix : &mut Matrix, pivots : &CVec<isize>, image_rows : usize, target_dimension : usize){
        let image = matrix.get_image(image_rows, target_dimension, pivots);
        self.set_image(degree, image);
    }

    fn set_image(&self, degree : i32, image : Subspace); 
    fn get_image(&self, degree : i32) -> Option<&Subspace>;    
    fn get_image_pivots(&self, degree : i32) -> Option<&CVec<isize>> {
        let image = self.get_image(degree);
        return image.map(|subspace| &subspace.column_to_pivot_row );
    }

    fn copy_quasi_inverse_from_matrix(&self, degree : i32, matrix : &mut Matrix, image_rows : usize, padded_target_dimension : usize){
        
    }
    fn set_quasi_inverse(&self, degree : i32, quasi_inverse : Matrix);
    fn get_quasi_inverse(&self, degree : i32) -> Option<&Matrix>;
    
    fn get_matrix(&self, matrix : &mut Matrix, degree : i32, start_row : usize, start_column : usize) -> (usize, usize) {
        let source_dimension = self.get_source().get_dimension(degree);
        let target_dimension = self.get_target().get_dimension(degree);
        assert!(source_dimension <= matrix.get_rows());
        assert!(target_dimension <= matrix.get_columns());
        for input_idx in 0 .. source_dimension {
            // Writing into slice.
            // Can we take ownership from matrix and then put back? 
            // If source is smaller than target, just allow add to ignore rest of input would work here.
            let output_vector = &mut matrix[start_row + input_idx];
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

    fn set_kernel(&self, degree : i32, kernel : Subspace){
        
    }
    
    fn get_kernel(&self, degree : i32) -> Option<&Subspace>{
        None
    }

    fn copy_image_from_matrix(&self, _degree : i32, _matrix : &mut Matrix, _pivots : &CVec<isize>, _image_rows : usize, _target_dimension : usize){}

    fn set_image(&self, _degree : i32, _image : Subspace){}

    fn get_image(&self, _degree : i32) -> Option<&Subspace> { None }
    
    fn get_image_pivots(&self, _degree : i32) -> Option<&CVec<isize>> { None }

    fn set_quasi_inverse(&self, degree : i32, quasi_inverse : Matrix){

    }

    fn get_quasi_inverse(&self, degree : i32) -> Option<&Matrix>{ None }

}