use crate::fp_vector::FpVector;
use crate::fp_vector::FpVectorTrait;
use crate::matrix::Matrix;
use crate::matrix::QuasiInverse;
use crate::matrix::Subspace;
use crate::module::Module;
use crate::module_homomorphism::ModuleHomomorphism;
use crate::free_module::FreeModule;

pub struct FreeModuleHomomorphism<'a> {
    source : &'a FreeModule<'a>,
    target : &'a Module,
    outputs : Vec<Vec<FpVector>>, // degree --> input_idx --> output
    min_degree : i32,
    degree_shift : i32,
    coimage_to_image_isomorphism : Vec<QuasiInverse<'a>>,
    kernel : Vec<Subspace>
}

impl<'a> ModuleHomomorphism for FreeModuleHomomorphism<'a> {
    fn get_source(&self) -> &Module {
        self.source
    }
    fn get_target(&self) -> &Module {
        self.target
    }

    fn apply_to_basis_element(&self, result : &mut FpVector, coeff : u32, input_degree : i32, input_index : usize){
        assert!(input_degree > self.source.min_degree);
        let input_degree_idx = (input_degree - self.source.min_degree) as usize;
        // assert(((FreeModuleInternal*)f->source)->basis_element_to_opgen_table[shifted_input_degree] != NULL);
        assert!(input_index < self.source.get_dimension(input_degree));
        assert!(self.target.get_dimension(input_degree + self.degree_shift) == result.get_dimension());
        let operation_generator = 
            self.source.index_to_op_gen(input_degree, input_index);
        let operation_degree = operation_generator.operation_degree;
        let operation_index = operation_generator.operation_index;
        let generator_degree = operation_generator.generator_degree;
        let generator_index = operation_generator.generator_index;
        let output_on_generator = self.get_output(generator_degree, generator_index);
        self.target.act(result, coeff, operation_degree, operation_index, generator_degree + self.degree_shift, output_on_generator);
    }
}
// // Run FreeModule_ConstructBlockOffsetTable(source, degree) before using this on an input in that degree
// void FreeModuleHomomorphism_applyToBasisElement(FreeModuleHomomorphism *f, Vector *result, uint coeff, int input_degree, uint input_index){

// }


impl<'a> FreeModuleHomomorphism<'a> {
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
            // Read only slice
            let slice = matrix.vectors[first_new_row + i].slice(0, dimension);
            self.set_output(degree, i, &slice);
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
}

// // Primarily for Javascript (so we can avoid indexing struct fields).
// void FreeModuleHomomorphism_applyToGenerator(FreeModuleHomomorphism *f, Vector *result, uint coeff, int generator_degree, uint generator_index){
//     Vector *output_on_generator = FreeModuleHomomorphism_getOutput(f, generator_degree, generator_index);
//     Vector_add(result, output_on_generator, coeff);
// }

