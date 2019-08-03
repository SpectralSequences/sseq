use std::sync::{Mutex, MutexGuard};

use crate::once::OnceVec;
use crate::fp_vector::FpVector;
use crate::matrix::{Matrix, Subspace, QuasiInverse};
use crate::module::Module;
use crate::module_homomorphism::ModuleHomomorphism;
use crate::free_module::{FreeModule, FreeModuleTableEntry};

pub struct FreeModuleHomomorphism<'a, 'b> {
    pub source : &'b FreeModule<'a>,
    pub target : &'b Module,
    outputs : OnceVec<Vec<FpVector>>, // degree --> input_idx --> output
    kernel : OnceVec<Subspace>,
    quasi_inverse : OnceVec<QuasiInverse>,
    min_degree : i32,
    max_degree : Mutex<i32>,
    degree_shift : i32
}

impl ModuleHomomorphism for FreeModuleHomomorphism<'_, '_> {
    fn get_source(&self) -> &Module {
        self.source
    }
    fn get_target(&self) -> &Module {
        self.target
    }

    fn apply_to_basis_element(&self, result : &mut FpVector, coeff : u32, input_degree : i32, input_index : usize){
        assert!(input_degree >= self.source.min_degree);
        let input_degree_idx = (input_degree - self.source.min_degree) as usize;
        let table = &self.source.table[input_degree_idx];
        self.apply_to_basis_element_with_table(result, coeff, input_degree, table, input_index);
    }

    fn get_lock(&self) -> MutexGuard<i32> {
        self.max_degree.lock().unwrap()
    }

    fn set_kernel(&self, lock : &MutexGuard<i32>, degree : i32, kernel : Subspace){
        assert!(degree >= self.min_degree);
        assert!(degree == **lock + 1);
        let degree_idx = (degree - self.min_degree) as usize;
        assert!(degree_idx == self.kernel.len());
        self.kernel.push(kernel);
    }

    fn get_kernel(&self, degree : i32) -> Option<&Subspace> {
        assert!(degree >= self.min_degree);
        let degree_idx = (degree - self.min_degree) as usize;
        Some(&self.kernel[degree_idx])
    }

    fn set_quasi_inverse(&self, lock : &MutexGuard<i32>, degree : i32, quasi_inverse : QuasiInverse){
        assert!(degree >= self.min_degree);
        assert!(degree == **lock + 1);
        let degree_idx = (degree - self.min_degree) as usize;
        assert!(degree_idx == self.quasi_inverse.len());
        self.quasi_inverse.push(quasi_inverse);
    }

    fn get_quasi_inverse(&self, degree : i32) -> Option<&QuasiInverse> {
        assert!(degree >= self.min_degree);
        let degree_idx = (degree - self.min_degree) as usize;
        Some(&self.quasi_inverse[degree_idx])
    }
}
// // Run FreeModule_ConstructBlockOffsetTable(source, degree) before using this on an input in that degree
// void FreeModuleHomomorphism_applyToBasisElement(FreeModuleHomomorphism *f, Vector *result, uint coeff, int input_degree, uint input_index){

// }


impl<'a, 'b> FreeModuleHomomorphism<'a, 'b> {
    pub fn new(source : &'b FreeModule<'a>, target : &'b Module, min_degree : i32, degree_shift : i32, max_degree : i32) -> Self {
        let num_degrees = max_degree as usize - min_degree as usize;
        let outputs = OnceVec::with_capacity(num_degrees);
        let kernel = OnceVec::with_capacity(num_degrees);
        let quasi_inverse = OnceVec::with_capacity(num_degrees);
        Self {
            source,
            target,
            outputs,
            kernel,
            quasi_inverse,
            min_degree,
            max_degree : Mutex::new(min_degree - 1),
            degree_shift
        }
    }

    pub fn get_output(&self, generator_degree : i32, generator_index : usize ) -> &FpVector {
        assert!(generator_degree >= self.source.min_degree);
        assert!(generator_index < self.source.get_number_of_gens_in_degree(generator_degree));        
        let generator_degree_idx = (generator_degree - self.source.min_degree) as usize;
        return &self.outputs[generator_degree_idx][generator_index];
    }

    // We don't actually mutate &mut matrix, we just slice it.
    pub fn add_generators_from_matrix_rows(&self, lock : &MutexGuard<i32>, degree : i32, matrix : &mut Matrix, first_new_row : usize, first_target_column : usize, new_generators : usize){
        // println!("    add_gens_from_matrix degree : {}, first_new_row : {}, new_generators : {}", degree, first_new_row, new_generators);
        let dimension = self.target.get_dimension(degree);
        // println!("    dimension : {} target name : {}", dimension, self.target.get_name());
        assert!(degree >= self.source.min_degree);
        let degree_idx = (degree - self.source.min_degree) as usize;
        assert!(degree_idx == self.outputs.len());
        assert!(degree == **lock + 1);
        let p = self.get_prime();
        let dimension = self.target.get_dimension(degree + self.degree_shift);
        let mut new_outputs : Vec<FpVector> = Vec::with_capacity(new_generators);
        for _ in 0 .. new_generators {
            new_outputs.push(FpVector::new(p, dimension, 0));
        }
        if dimension == 0 {
            self.outputs.push(new_outputs);
            return;
        }
        for i in 0 .. new_generators {
            let output_vector = &mut matrix[first_new_row + i];
            output_vector.set_slice(0, dimension);
            new_outputs[i].assign(&output_vector);
            output_vector.clear_slice();
        }
        self.outputs.push(new_outputs);
    }

    pub fn apply_to_basis_element_with_table(&self, result : &mut FpVector, coeff : u32, input_degree : i32, table : &FreeModuleTableEntry, input_index : usize){
        assert!(input_degree >= self.source.min_degree);
        assert!(input_index < table.basis_element_to_opgen.len());
        assert!(self.target.get_dimension(input_degree + self.degree_shift) == result.get_dimension());
        let operation_generator = &table.basis_element_to_opgen[input_index];
        let operation_degree = operation_generator.operation_degree;
        let operation_index = operation_generator.operation_index;
        let generator_degree = operation_generator.generator_degree;
        let generator_index = operation_generator.generator_index;
        let output_on_generator = self.get_output(generator_degree, generator_index);
        self.target.act(result, coeff, operation_degree, operation_index, generator_degree + self.degree_shift, output_on_generator);
    }

    pub fn get_matrix_with_table(&self, matrix : &mut Matrix, table : &FreeModuleTableEntry , degree : i32, start_row : usize, start_column : usize) -> (usize, usize) {
        let source_dimension = self.source.get_dimension_with_table(degree, table);
        let target_dimension = self.get_target().get_dimension(degree);
        assert!(source_dimension <= matrix.get_rows());
        assert!(target_dimension <= matrix.get_columns());
        for input_idx in 0 .. source_dimension {
            // Writing into slice.
            // Can we take ownership from matrix and then put back? 
            // If source is smaller than target, just allow add to ignore rest of input would work here.
            let output_vector = &mut matrix[start_row + input_idx];
            output_vector.set_slice(start_column, start_column + target_dimension);
            self.apply_to_basis_element_with_table(output_vector, 1, degree, table, input_idx);
            output_vector.clear_slice();
        }
        return (start_row + source_dimension, start_column + target_dimension);
    } 
}

// // Primarily for Javascript (so we can avoid indexing struct fields).
// void FreeModuleHomomorphism_applyToGenerator(FreeModuleHomomorphism *f, Vector *result, uint coeff, int generator_degree, uint generator_index){
//     Vector *output_on_generator = FreeModuleHomomorphism_getOutput(f, generator_degree, generator_index);
//     Vector_add(result, output_on_generator, coeff);
// }

