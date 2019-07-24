use crate::fp_vector::FpVector;
use crate::algebra::Algebra;
use crate::module::Module;

pub struct OperationGeneratorPair {
    pub operation_degree : i32,
    pub operation_index : usize,
    pub generator_degree : i32,
    pub generator_index : usize
}

pub struct FreeModule<'a> {
    algebra : &'a Algebra,
    name : String,
    pub min_degree : i32,
    number_of_generators_in_degree : Vec<usize>,
    // private fields
    // degree --> idx --> OpGen
    basis_element_to_opgen_table : Vec<Vec<OperationGeneratorPair>>,
    // degree --> generator_degree -> generator_index -> index
    generator_to_index_table : Vec<Vec<Vec<usize>>>
}

impl<'a> Module for FreeModule<'a> {

    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_algebra(&self) -> &Algebra {
        self.algebra
    }

    fn get_dimension(&self, degree : i32) -> usize {
        if degree < self.min_degree {
            return 0;
        }
        let degree_idx = (degree - self.min_degree) as usize;
        assert!(degree_idx < self.basis_element_to_opgen_table.len());
        return self.basis_element_to_opgen_table[degree_idx].len();
    }

    fn act_on_basis(&self, result : &mut FpVector, coeff : u32, op_degree : i32, op_index : usize, mod_degree : i32, mod_index : usize){
        assert!(op_index < self.get_algebra().get_dimension(op_degree, mod_degree));
        assert!(self.get_dimension(op_degree + mod_degree) <= result.get_dimension());
        let operation_generator = self.index_to_op_gen(mod_degree, mod_index);
        let module_operation_degree = operation_generator.operation_degree;
        let module_operation_index = operation_generator.operation_index;
        let generator_degree = operation_generator.generator_degree; 
        let generator_index  = operation_generator.generator_index;
        // Now all of the output elements are going to be of the form s * x. Find where such things go in the output vector.
        let num_ops = self.get_algebra().get_dimension(module_operation_degree + op_degree, generator_degree);
        let output_block_min = self.operation_generator_to_index(module_operation_degree + op_degree, 0, generator_degree, generator_index);
        let output_block_max = output_block_min + num_ops;
        // Writing into slice (can we take ownership? make new vector with 0's outside range and add separately? is it okay?)
        let output_block = result.slice(output_block_min, output_block_max); 
        // Now we multiply s * r and write the result to the appropriate position.
        self.get_algebra().multiply_basis_elements(output_block, coeff, op_degree, op_index, module_operation_degree, module_operation_index, generator_degree);
    }
}

impl<'a> FreeModule<'a> {
    pub fn new(algebra : &'a Algebra, name : String, min_degree : i32) -> Self {
        Self {
            algebra,
            name,
            min_degree,
            number_of_generators_in_degree : Vec::new(),
            basis_element_to_opgen_table :  Vec::new(),
            generator_to_index_table : Vec::new()
        }
    }

    pub fn get_number_of_gens_in_degree(&self, degree : i32) -> usize {
        if degree < self.min_degree {
            return 0;
        }
        let degree_idx = (degree - self.min_degree) as usize;
        assert!(degree_idx < self.number_of_generators_in_degree.len());
        return self.number_of_generators_in_degree[degree_idx];
    }

    pub fn construct_block_offset_table(&mut self, degree : i32){
        assert!(degree >= self.min_degree);
        let degree_idx = (degree - self.min_degree) as usize;
        assert!(self.basis_element_to_opgen_table.len() == degree_idx);
        assert!(self.generator_to_index_table.len() == degree_idx);
        // assert!(self.generator_to_index_table[degree_idx] == NULL);
        let mut basis_element_to_opgen_entry : Vec<OperationGeneratorPair> = Vec::with_capacity(degree_idx + 1);
        let mut generator_to_index_entry : Vec<Vec<usize>> = Vec::with_capacity(degree_idx + 1);
        // gen_to_idx goes gen_degree => gen_idx => start of block.
        // so gen_to_idx_size should be (number of possible degrees + 1) * sizeof(uint*) + number of gens * sizeof(uint).
        // The other part of the table goes idx => opgen
        // The size should be (number of basis elements in current degree) * sizeof(FreeModuleOperationGeneratorPair)
        // A basis element in degree n comes from a generator in degree i paired with an operation in degree n - i.
        let mut offset = 0;
        for gen_deg in self.min_degree .. degree + 1 {
            let num_gens = self.get_number_of_gens_in_degree(gen_deg);
            let mut gentoidx_degree : Vec<usize> = Vec::with_capacity(num_gens);
            let op_deg = degree - gen_deg;
            let num_ops = self.get_algebra().get_dimension(op_deg, gen_deg);
            for gen_idx in 0 .. num_gens {
                gentoidx_degree.push(offset);
                for op_idx in 0 .. num_ops {
                    basis_element_to_opgen_entry.push(OperationGeneratorPair {
                        generator_degree : gen_deg,
                        generator_index : gen_idx,
                        operation_degree : op_deg,
                        operation_index : op_idx
                    })
                }
                offset += num_ops;
            }
            generator_to_index_entry.push(gentoidx_degree);
        }
        self.basis_element_to_opgen_table.push(basis_element_to_opgen_entry);
        self.generator_to_index_table.push(generator_to_index_entry);
    }
    
    pub fn add_generators(&mut self, degree : i32, new_generators : usize){
        assert!(degree >= self.min_degree);
        let degree_idx = (degree - self.min_degree) as usize;
        assert!(self.generator_to_index_table.len() == degree_idx);
        self.number_of_generators_in_degree.push(new_generators);
        self.construct_block_offset_table(degree);
    }

    pub fn add_generators_after_opgen_table(&mut self, degree : i32, new_generators : usize){
        assert!(degree >= self.min_degree);
        let degree_idx = (degree - self.min_degree) as usize;
        assert!(self.generator_to_index_table.len() == degree_idx + 1);
        let old_dimension = self.get_dimension(degree);
        let old_num_gens = self.number_of_generators_in_degree[degree_idx];
        let new_num_gens = old_num_gens + new_generators;
        self.number_of_generators_in_degree[degree_idx] = new_num_gens;
        let mut start_of_block = old_dimension;
        for gen_idx in old_num_gens .. new_num_gens {
            self.basis_element_to_opgen_table[degree_idx].push(OperationGeneratorPair {
                generator_degree : degree,
                generator_index : gen_idx,
                operation_degree : 0,
                operation_index : 0
            });
            self.generator_to_index_table[degree_idx][gen_idx].push(start_of_block);
            start_of_block += 1;
        }
        
    }

    pub fn operation_generator_to_index(&self, op_deg : i32, op_idx : usize, gen_deg : i32, gen_idx : usize) -> usize {
        assert!(op_deg >= self.min_degree);
        assert!(gen_deg >= self.min_degree);
        let op_deg_idx = (op_deg - self.min_degree) as usize;
        let gen_deg_idx = (gen_deg - self.min_degree) as usize;
        return self.generator_to_index_table[op_deg_idx][gen_deg_idx][gen_idx] + op_idx;
    }

    pub fn index_to_op_gen(&self, degree : i32, index : usize) -> &OperationGeneratorPair {
        assert!(degree >= self.min_degree);
        let degree_idx = (degree - self.min_degree) as usize;
        return &self.basis_element_to_opgen_table[degree_idx][index];
    }
}



// uint FreeModule_element_toJSONString(char *result, FreeModule *this, int degree, Vector *element);