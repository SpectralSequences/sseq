use std::sync::{Mutex, MutexGuard};
use std::sync::Arc;

use once::OnceBiVec;
use crate::fp_vector::{FpVector, FpVectorT};
use crate::matrix::{Matrix, Subspace, QuasiInverse};
use crate::module::{Module, FreeModule, FreeModuleTableEntry};
use crate::module_homomorphism::ModuleHomomorphism;

pub struct FreeModuleHomomorphism<M : Module> {
    source : Arc<FreeModule>,
    target : Arc<M>,
    pub outputs : OnceBiVec<Vec<FpVector>>, // degree --> input_idx --> output
    kernel : OnceBiVec<Subspace>,
    quasi_inverse : OnceBiVec<QuasiInverse>,
    min_degree : i32,
    max_degree : Mutex<i32>,
    degree_shift : i32
}

impl<M : Module> ModuleHomomorphism for FreeModuleHomomorphism<M> {
    type Source = FreeModule;
    type Target = M;

    fn source(&self) -> Arc<Self::Source> {
        Arc::clone(&self.source)
    }

    fn target(&self) -> Arc<Self::Target> {
        Arc::clone(&self.target)
    }

    fn degree_shift(&self) -> i32 {
        self.degree_shift
    }

    fn apply_to_basis_element(&self, result : &mut FpVector, coeff : u32, input_degree : i32, input_index : usize){
        assert!(input_degree >= self.source.min_degree);
        let table = &self.source.table[input_degree];
        self.apply_to_basis_element_with_table(result, coeff, input_degree, table, input_index);
    }

    fn quasi_inverse(&self, degree : i32) -> &QuasiInverse {
        debug_assert!(degree >= self.min_degree, format!("Degree {} less than min degree {}", degree, self.min_degree));
        &self.quasi_inverse[degree]
    }

    fn kernel(&self, degree : i32) -> &Subspace {
        &self.kernel[degree]
    }

    fn compute_kernels_and_quasi_inverses_through_degree(&self, degree : i32) {
        let lock = self.lock();
        let kernel_len = self.kernel.len();
        let qi_len = self.quasi_inverse.len();
        assert_eq!(kernel_len, qi_len);
        for i in kernel_len ..= degree {
            let (kernel, qi) = self.kernel_and_quasi_inverse(degree);
            self.kernel.push(kernel);
            self.quasi_inverse.push(qi);
        }
    }
}

// // Run FreeModule_ConstructBlockOffsetTable(source, degree) before using this on an input in that degree
// void FreeModuleHomomorphism_applyToBasisElement(FreeModuleHomomorphism *f, Vector *result, uint coeff, int input_degree, uint input_index){

// }


impl<M : Module> FreeModuleHomomorphism<M> {
    pub fn new(source : Arc<FreeModule>, target : Arc<M>, degree_shift : i32) -> Self {
        let min_degree = std::cmp::max(source.min_degree(), target.min_degree() + degree_shift);
        let outputs = OnceBiVec::new(min_degree);
        let kernel = OnceBiVec::new(min_degree);
        let quasi_inverse = OnceBiVec::new(min_degree);
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

    pub fn degree_shift(&self) -> i32 {
        self.degree_shift
    }

    pub fn min_degree(&self) -> i32 {
        self.min_degree
    }

    pub fn output(&self, generator_degree : i32, generator_index : usize) -> &FpVector {
        assert!(generator_degree >= self.min_degree(), 
            format!("generator_degree {} less than min degree {}", generator_degree, self.min_degree()));
        assert!(generator_index < self.source.number_of_gens_in_degree(generator_degree),
            format!("generator_index {} greater than number of generators {}", 
                generator_index, self.source.number_of_gens_in_degree(generator_degree)
        ));
        return &self.outputs[generator_degree][generator_index];
    }

    pub fn extend_by_zero(&self, lock : &MutexGuard<i32>, degree : i32){
        // println!("    add_gens_from_matrix degree : {}, first_new_row : {}, new_generators : {}", degree, first_new_row, new_generators);
        // println!("    dimension : {} target name : {}", dimension, self.target.name());
        if degree < self.min_degree {
            return;
        }
        assert!(degree >= **lock + 1);
        let p = self.prime();
        for i in **lock + 1 ..= degree{
            let num_gens = self.source.number_of_gens_in_degree(i);
            let dimension = self.target.dimension(i - self.degree_shift);
            let mut new_outputs : Vec<FpVector> = Vec::with_capacity(num_gens);
            for _ in 0 .. num_gens {
                new_outputs.push(FpVector::new(p, dimension));
            }
            self.outputs.push(new_outputs);
        }
    }

    // We don't actually mutate vector, we just slice it.
    pub fn add_generators_from_big_vector(&self, lock : &MutexGuard<i32>, degree : i32, outputs_vectors : &mut FpVector){
        // println!("    add_gens_from_matrix degree : {}, first_new_row : {}, new_generators : {}", degree, first_new_row, new_generators);
        // println!("    dimension : {} target name : {}", dimension, self.target.name());
        assert!(degree >= self.min_degree);
        assert_eq!(degree, self.outputs.len());
        assert!(degree == **lock + 1);
        let p = self.prime();
        let new_generators = self.source.number_of_gens_in_degree(degree);
        let target_dimension = self.target.dimension(degree - self.degree_shift);
        let mut new_outputs : Vec<FpVector> = Vec::with_capacity(new_generators);
        for _ in 0 .. new_generators {
            new_outputs.push(FpVector::new(p, target_dimension));
        }
        if target_dimension == 0 {
            self.outputs.push(new_outputs);
            return;
        }
        for i in 0 .. new_generators {
            let old_slice = outputs_vectors.slice();
            outputs_vectors.set_slice(target_dimension * i, target_dimension * (i + 1));
            new_outputs[i].shift_add(&outputs_vectors, 1);
            outputs_vectors.restore_slice(old_slice);
        }
        self.outputs.push(new_outputs);
    }    

    // We don't actually mutate &mut matrix, we just slice it.
    pub fn add_generators_from_matrix_rows(&self, lock : &MutexGuard<i32>, degree : i32, matrix : &mut Matrix, first_new_row : usize, first_target_column : usize){
        // println!("    add_gens_from_matrix degree : {}, first_new_row : {}, new_generators : {}", degree, first_new_row, new_generators);
        // println!("    dimension : {} target name : {}", dimension, self.target.get_name());
        assert!(degree >= self.min_degree);
        assert_eq!(degree, self.outputs.len());
        assert!(degree == **lock + 1);
        let p = self.prime();
        let new_generators = self.source.number_of_gens_in_degree(degree);
        let dimension = self.target.dimension(degree - self.degree_shift);
        let mut new_outputs : Vec<FpVector> = Vec::with_capacity(new_generators);
        for _ in 0 .. new_generators {
            new_outputs.push(FpVector::new(p, dimension));
        }
        if dimension == 0 {
            self.outputs.push(new_outputs);
            return;
        }
        for i in 0 .. new_generators {
            let output_vector = &mut matrix[first_new_row + i];
            let old_slice = output_vector.slice();
            output_vector.set_slice(first_target_column, first_target_column + dimension);
            new_outputs[i].assign(&output_vector);
            output_vector.restore_slice(old_slice);
        }
        self.outputs.push(new_outputs);
    }

    pub fn apply_to_generator(&self, result : &mut FpVector, coeff : u32, degree : i32, idx : usize) {
        let output_on_gen = self.output(degree, idx);
        result.add(output_on_gen, coeff);
    }

    pub fn apply_to_basis_element_with_table(&self, result : &mut FpVector, coeff : u32, input_degree : i32, table : &FreeModuleTableEntry, input_index : usize){
        assert!(input_degree >= self.source.min_degree);
        assert!(input_index < table.basis_element_to_opgen.len());
        let output_degree = input_degree - self.degree_shift;
        assert_eq!(self.target.dimension(output_degree), result.dimension());
        let operation_generator = &table.basis_element_to_opgen[input_index];
        let operation_degree = operation_generator.operation_degree;
        let operation_index = operation_generator.operation_index;
        let generator_degree = operation_generator.generator_degree;
        let generator_index = operation_generator.generator_index;
        if generator_degree >= self.min_degree() {
            let output_on_generator = self.output(generator_degree, generator_index);
            self.target.act(result, coeff, operation_degree, operation_index, generator_degree - self.degree_shift, output_on_generator);            
        }
    }

    pub fn get_matrix(&self, matrix : &mut Matrix, degree : i32, start_row : usize, start_column : usize) -> (usize, usize) {
        self.get_matrix_with_table(matrix, &self.source.table[degree], degree, start_row, start_column)
    }

    /// # Arguments
    ///  * `degree` - The internal degree of the target of the homomorphism.
    pub fn get_matrix_with_table(&self, matrix : &mut Matrix, table : &FreeModuleTableEntry , degree : i32, start_row : usize, start_column : usize) -> (usize, usize) {
        let source_dimension = FreeModule::dimension_with_table(table);
        let target_dimension = self.target().dimension(degree);
        assert!(source_dimension <= matrix.rows());
        assert!(target_dimension <= matrix.columns());
        for input_idx in 0 .. source_dimension {
            // Writing into slice.
            // Can we take ownership from matrix and then put back? 
            // If source is smaller than target, just allow add to ignore rest of input would work here.
            let output_vector = &mut matrix[start_row + input_idx];
            let old_slice = output_vector.slice();
            output_vector.set_slice(start_column, start_column + target_dimension);
            self.apply_to_basis_element_with_table(output_vector, 1, degree, table, input_idx);
            output_vector.restore_slice(old_slice);
        }
        return (start_row + source_dimension, start_column + target_dimension);
    }

    pub fn lock(&self) -> MutexGuard<i32> {
        self.max_degree.lock().unwrap()
    }

    pub fn set_quasi_inverse(&self, lock : &MutexGuard<i32>, degree : i32, quasi_inverse : QuasiInverse){
        assert!(degree == self.quasi_inverse.len());
        self.quasi_inverse.push(quasi_inverse);
    }
}
