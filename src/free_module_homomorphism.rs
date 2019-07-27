use crate::memory::CVec;
use crate::fp_vector::FpVector;
use crate::matrix::Matrix;
use crate::matrix::QuasiInverse;
use crate::matrix::Subspace;
use crate::module::Module;
use crate::module_homomorphism::ModuleHomomorphism;
use crate::free_module::{FreeModule, FreeModuleTableEntry};

pub struct FreeModuleHomomorphism<'a, 'b, 'c> {
    pub source : &'a FreeModule<'b>,
    pub target : &'c Module,
    outputs : Vec<Vec<FpVector>>, // degree --> input_idx --> output
    min_degree : i32,
    degree_shift : i32,
    pub image_pivots : Vec<CVec<isize>>,
    pub kernels : Vec<Subspace>
}

impl<'a> ModuleHomomorphism for FreeModuleHomomorphism<'_, '_, '_> {
    fn get_source(&self) -> &Module {
        self.source
    }
    fn get_target(&self) -> &Module {
        self.target
    }

    fn apply_to_basis_element(&self, result : &mut FpVector, coeff : u32, input_degree : i32, input_index : usize){
        assert!(input_degree >= self.source.min_degree);
        let input_degree_idx = (input_degree - self.source.min_degree) as usize;
        let table = &self.source.table[input_degree_idx].get();
        self.apply_to_basis_element_with_table(result, coeff, input_degree, table, input_index);
    }

    fn set_kernel(&mut self, degree : i32, kernel : Subspace){
        assert!(degree >= self.get_min_degree());
        let degree_idx = (degree - self.get_min_degree()) as usize;
        assert!(self.kernels.len() == degree_idx);
        self.kernels.push(kernel);
    }

    // fn get_quasi_inverse(&self, degree : u32) -> &QuasiInverse;
    fn set_image_pivots(&mut self, degree : i32, pivots : CVec<isize>){
        assert!(degree >= self.get_min_degree());
        let degree_idx = (degree - self.get_min_degree()) as usize;
        assert!(self.image_pivots.len() == degree_idx);
        self.image_pivots.push(pivots);
    }
}
// // Run FreeModule_ConstructBlockOffsetTable(source, degree) before using this on an input in that degree
// void FreeModuleHomomorphism_applyToBasisElement(FreeModuleHomomorphism *f, Vector *result, uint coeff, int input_degree, uint input_index){

// }


impl<'a, 'b, 'c> FreeModuleHomomorphism<'a, 'b, 'c> {
    pub fn new(source : &'a FreeModule<'b>, target : &'c Module, min_degree : i32, degree_shift : i32) -> Self {
        Self {
            source,
            target,
            outputs : Vec::new(),
            min_degree,
            degree_shift,
            image_pivots : Vec::new(),
            kernels : Vec::new()
        }
    }

    pub fn get_output(&self, generator_degree : i32, generator_index : usize ) -> &FpVector {
        assert!(generator_degree >= self.source.min_degree);
        assert!(generator_index < self.source.get_number_of_gens_in_degree(generator_degree));        
        let generator_degree_idx = (generator_degree - self.source.min_degree) as usize;
        return &self.outputs[generator_degree_idx][generator_index];
    }

    pub fn set_output(&mut self, generator_degree : i32, generator_index : usize, output : &FpVector){
        assert!(generator_degree >= self.source.min_degree);
        assert!(generator_index < self.source.get_number_of_gens_in_degree(generator_degree));
        let generator_degree_idx = (generator_degree - self.source.min_degree) as usize;
        assert!(output.get_dimension() == self.target.get_dimension(generator_degree + self.degree_shift));
        assert!(output.get_offset() == 0);
        self.outputs[generator_degree_idx][generator_index].assign(output);
    }

    // We don't actually mutate &mut matrix, we just slice it.
    pub fn add_generators_from_matrix_rows(&mut self, degree : i32, matrix : &mut Matrix, first_new_row : usize, new_generators : usize){
        let dimension = self.target.get_dimension(degree);
        self.allocate_space_for_new_generators(degree, new_generators);
        for i in 0 .. new_generators {
            let output_vector = &mut matrix.vectors[first_new_row + i];
            output_vector.set_slice(0, dimension);
            self.set_output(degree, i, &output_vector);
            output_vector.clear_slice();
        }
    }

    pub fn allocate_space_for_new_generators(&mut self, degree : i32, new_generators : usize){
        // assert(degree < selmax_degree);
        // assert(f->max_computed_degree <= degree);
        assert!(degree >= self.source.min_degree);
        let degree_idx = (degree - self.source.min_degree) as usize;
        assert!(degree_idx == self.outputs.len());
        let p = self.get_prime();
        let dimension = self.target.get_dimension(degree + self.degree_shift);
        let mut new_outputs : Vec<FpVector> = Vec::with_capacity(new_generators);
        for i in 0 .. new_generators {
            new_outputs.push(FpVector::new(p, dimension, 0));
        }
        self.outputs.push(new_outputs);
    }

    pub fn apply_to_basis_element_with_table(&self, result : &mut FpVector, coeff : u32, input_degree : i32, table : &FreeModuleTableEntry, input_index : usize){
        assert!(input_degree >= self.source.min_degree);
        let input_degree_idx = (input_degree - self.source.min_degree) as usize;
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

    fn get_matrix(&self, matrix : &mut Matrix, table : &FreeModuleTableEntry , degree : i32, start_row : usize, start_column : usize) -> (usize, usize) {
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

