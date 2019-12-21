use std::sync::{ Mutex, MutexGuard };
use std::sync::Arc;
use serde_json::json;
use serde_json::Value;


use bivec::BiVec;
use once::OnceBiVec;
use crate::fp_vector::{FpVector, FpVectorT};
use crate::algebra::{Algebra, AlgebraAny};
use crate::module::Module;

#[derive(Debug)]
pub struct OperationGeneratorPair {
    pub operation_degree : i32,
    pub operation_index : usize,
    pub generator_degree : i32,
    pub generator_index : usize
}

pub struct FreeModuleTableEntry {
    pub num_gens : usize,
    pub basis_element_to_opgen : Vec<OperationGeneratorPair>,
    pub generator_to_index : Vec<usize>, 
}

pub struct FreeModule {
    pub algebra : Arc<AlgebraAny>,
    pub name : String,
    pub min_degree : i32,
    pub max_degree : Mutex<i32>,
    gen_names : OnceBiVec<Vec<String>>,
    gen_deg_idx_to_internal_idx : OnceBiVec<usize>,
    pub table : OnceBiVec<FreeModuleTableEntry>
}

impl Module for FreeModule {
    fn name(&self) -> &str {
        &self.name
    }

    fn algebra(&self) -> Arc<AlgebraAny> {
        Arc::clone(&self.algebra)
    }

    fn min_degree(&self) -> i32 {
        self.min_degree
    }

    fn dimension(&self, degree : i32) -> usize {
        if degree < self.min_degree {
            return 0;
        }
        assert!(degree < self.table.len(), format!("Free Module {} not computed through degree {}", self.name(), degree));
        self.table[degree].basis_element_to_opgen.len()
    }

    fn basis_element_to_string(&self, degree : i32, idx : usize) -> String {
        let opgen = self.index_to_op_gen(degree, idx);
        let mut op_str = self.algebra.basis_element_to_string(opgen.operation_degree, opgen.operation_index);
        if &*op_str == "1" {
            op_str = "".to_string();
        } else {
            op_str.push(' ');
        }
        return format!("{}{}", op_str, self.gen_names[opgen.generator_degree][opgen.generator_index]);
    }

    fn act_on_basis(&self, result : &mut FpVector, coeff : u32, op_degree : i32, op_index : usize, mod_degree : i32, mod_index : usize){
        assert!(op_index < self.algebra().dimension(op_degree, mod_degree));
        assert!(self.dimension(op_degree + mod_degree) <= result.dimension());
        let operation_generator = self.index_to_op_gen(mod_degree, mod_index);
        let module_operation_degree = operation_generator.operation_degree;
        let module_operation_index = operation_generator.operation_index;
        let generator_degree = operation_generator.generator_degree; 
        let generator_index  = operation_generator.generator_index;


        // Now all of the output elements are going to be of the form s * x. Find where such things go in the output vector.
        let num_ops = self.algebra().dimension(module_operation_degree + op_degree, generator_degree);
        let output_block_min = self.operation_generator_to_index(module_operation_degree + op_degree, 0, generator_degree, generator_index);
        let output_block_max = output_block_min + num_ops;

        // Now we multiply s * r and write the result to the appropriate position.
        self.algebra().multiply_basis_elements(
            &mut *result.borrow_slice(output_block_min, output_block_max),
            coeff, op_degree, op_index, module_operation_degree, module_operation_index, generator_degree);
    }
}

impl FreeModule {
    pub fn new(algebra : Arc<AlgebraAny>, name : String, min_degree : i32) -> Self {
        let gen_deg_idx_to_internal_idx = OnceBiVec::new(min_degree);
        gen_deg_idx_to_internal_idx.push(0);
        Self {
            algebra,
            name,
            min_degree,
            max_degree : Mutex::new(min_degree - 1),
            gen_names : OnceBiVec::new(min_degree),
            gen_deg_idx_to_internal_idx,
            table : OnceBiVec::new(min_degree)
        }
    }

    pub fn max_computed_degree(&self) -> i32 {
        self.table.max_degree()
    }

    pub fn number_of_gens_in_degree(&self, degree : i32) -> usize {
        if degree < self.min_degree {
            return 0;
        }
        self.table[degree].num_gens
    }

    pub fn construct_table(&self, degree : i32) -> (MutexGuard<i32>, FreeModuleTableEntry) {
        assert!(degree >= self.min_degree);
        let lock = self.max_degree.lock().unwrap();
        assert!(degree == *lock + 1);
        let mut basis_element_to_opgen : Vec<OperationGeneratorPair> = Vec::new();
        let mut generator_to_index : Vec<usize> = Vec::new();
        // gen_to_idx goes internal_gen_idx => start of block.
        // so gen_to_idx_size should be (number of possible degrees + 1) * sizeof(uint*) + number of gens * sizeof(uint).
        // The other part of the table goes idx => opgen
        // The size should be (number of basis elements in current degree) * sizeof(FreeModuleOperationGeneratorPair)
        // A basis element in degree n comes from a generator in degree i paired with an operation in degree n - i.
        let mut offset = 0;
        for gen_deg in self.min_degree .. degree {
            let num_gens = self.number_of_gens_in_degree(gen_deg);
            let op_deg = degree - gen_deg;
            let num_ops = self.algebra().dimension(op_deg, gen_deg);
            for gen_idx in 0 .. num_gens {
                generator_to_index.push(offset);
                for op_idx in 0 .. num_ops {
                    basis_element_to_opgen.push(OperationGeneratorPair {
                        generator_degree : gen_deg,
                        generator_index : gen_idx,
                        operation_degree : op_deg,
                        operation_index : op_idx
                    })
                }
                offset += num_ops;
            }
        }
        (lock,
            FreeModuleTableEntry {
                num_gens : 0,
                basis_element_to_opgen,
                generator_to_index
            }
        )
    }

    pub fn dimension_with_table(table : &FreeModuleTableEntry) -> usize {
        table.basis_element_to_opgen.len()
    }

    pub fn add_generators(&self, degree : i32, mut lock : MutexGuard<i32>, mut table : FreeModuleTableEntry, num_gens : usize, names : Option<Vec<String>>){
        assert!(degree >= self.min_degree);
        assert!(degree == *lock + 1);
        let mut gen_names;
        if let Some(names_vec) = names {
            gen_names = names_vec;
        } else {
            gen_names = Vec::with_capacity(num_gens);
            for i in 0 .. num_gens {
                gen_names.push(format!("x_{{{},{}}}", degree, i));
            }
        }
        self.gen_names.push(gen_names);
        self.add_generators_to_table(degree, &mut table, num_gens);
        self.table.push(table);
        *lock += 1;
    }

    fn add_generators_to_table(&self, degree : i32, table : &mut FreeModuleTableEntry, num_gens : usize){
        // let old_num_gens = table.num_gens;
        // let new_num_gens = old_num_gens + new_generators;
        table.num_gens = num_gens;
        let old_dimension = table.basis_element_to_opgen.len();
        let mut start_of_block = old_dimension;
        let internal_gen_idx = self.gen_deg_idx_to_internal_idx[degree];
        self.gen_deg_idx_to_internal_idx.push(internal_gen_idx + num_gens);
        // let mut gen_to_idx = Vec::with_capacity(num_gens);
        for gen_idx in 0 .. num_gens {
            table.basis_element_to_opgen.push(OperationGeneratorPair {
                generator_degree : degree,
                generator_index : gen_idx,
                operation_degree : 0,
                operation_index : 0
            });
            table.generator_to_index.push(start_of_block);
            start_of_block += 1;
        }
    }

    pub fn generator_offset(&self, degree : i32, gen_deg : i32, gen_idx : usize) -> usize {
        assert!(gen_deg >= self.min_degree);
        let internal_gen_idx = self.gen_deg_idx_to_internal_idx[gen_deg] + gen_idx;
        assert!(internal_gen_idx <= self.gen_deg_idx_to_internal_idx[gen_deg + 1]);
        self.table[degree].generator_to_index[internal_gen_idx]
    }

    pub fn operation_generator_to_index(&self, op_deg : i32, op_idx : usize, gen_deg : i32, gen_idx : usize) -> usize {
        assert!(op_deg >= 0);
        assert!(gen_deg >= self.min_degree);
        let internal_gen_idx = self.gen_deg_idx_to_internal_idx[gen_deg] + gen_idx;
        assert!(internal_gen_idx <= self.gen_deg_idx_to_internal_idx[gen_deg + 1]);
        self.table[op_deg + gen_deg].generator_to_index[internal_gen_idx] + op_idx
    }

    pub fn operation_generator_pair_to_idx(&self, op_gen : &OperationGeneratorPair) -> usize {
        self.operation_generator_to_index(
            op_gen.operation_degree,
            op_gen.operation_index,
            op_gen.generator_degree,
            op_gen.generator_index
        )
    }

    pub fn index_to_op_gen(&self, degree : i32, index : usize) -> &OperationGeneratorPair {
        assert!(degree >= self.min_degree);
        &self.table[degree].basis_element_to_opgen[index]
    }

    pub fn element_to_json(&self, degree : i32, elt : &FpVector) -> Value {
        let mut result = Vec::new();
        let algebra = self.algebra();
        for (i, v) in elt.iter().enumerate(){
            if v == 0 { continue; }
            let opgen = self.index_to_op_gen(degree, i);
            result.push(json!({
                "op" : algebra.json_from_basis(opgen.operation_degree, opgen.operation_index),
                "gen" : self.gen_names[opgen.generator_degree][opgen.generator_index],
                "coeff" : v
            }));
        }
        Value::from(result)
    }

    pub fn add_generators_immediate(&self, degree : i32, num_gens : usize, gen_names : Option<Vec<String>>){
        let (lock, table) = self.construct_table(degree);
        self.add_generators(degree, lock, table, num_gens, gen_names);
    }

    pub fn extend_by_zero(&self, degree : i32){
        let old_max_degree = { *self.max_degree.lock().unwrap() };
        for i in old_max_degree + 1 ..= degree {
            self.add_generators_immediate(i, 0, None)
        }
    }

    // Used by Yoneda. Gets nonempty dimensions.
    pub fn get_degrees_with_gens(&self, max_degree : i32) -> Vec<i32> {
        assert!(max_degree < self.gen_deg_idx_to_internal_idx.len() - 1);
        let mut result = Vec::new();
        for i in self.gen_deg_idx_to_internal_idx.min_degree() .. max_degree {
            if self.gen_deg_idx_to_internal_idx[i+1] > self.gen_deg_idx_to_internal_idx[i] {
                result.push(i);
            }
        }
        result
    }

}

use std::io;
use std::io::{Read, Write};
use saveload::{Save, Load};

impl Save for FreeModule {
    fn save(&self, buffer : &mut impl Write) -> io::Result<()> {
        let num_gens : Vec<usize> = self.table.iter().map(|t| t.num_gens).collect::<Vec<_>>();
        let num_gens : BiVec<usize> = BiVec::from_vec(self.table.min_degree(), num_gens);
        num_gens.save(buffer)
    }
}

impl Load for FreeModule {
    type AuxData = (Arc<AlgebraAny>, i32);

    fn load(buffer : &mut impl Read, data : &Self::AuxData) -> io::Result<Self> {
        let algebra = Arc::clone(&data.0);
        let min_degree = data.1;

        let result = FreeModule::new(algebra, "".to_string(), min_degree);

        let num_gens : BiVec<usize> = Load::load(buffer, &(min_degree, ()))?;
        for (degree, num) in num_gens.iter_enum() {
            result.add_generators_immediate(degree, *num, None);
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)]

    use super::*;

    use crate::algebra::AdemAlgebra;

    #[test]
    fn test_free_mod(){
        let p = 2;
        let A = Arc::new(AlgebraAny::from(AdemAlgebra::new(p, p != 2, false)));
        A.compute_basis(10);
        let M = FreeModule::new(Arc::clone(&A), "".to_string(), 0);
        let (lock, table) = M.construct_table(0);
        M.add_generators(0, lock, table, 1, None);
        let (lock, table) = M.construct_table(1);
        M.add_generators(1, lock, table, 1, None);
        for i in 2..10{
            let (lock, table) = M.construct_table(i);
            M.add_generators(i, lock, table, 0, None);
        }
        let output_deg = 6;
        let output_dim = M.dimension(output_deg);
        for i in 0..9 {
            assert_eq!(M.dimension(i), A.dimension(i,0) + A.dimension(i-1,1));
        }

        for (gen_deg, gen_idx) in &[(0,0), (1,0)]{
            let idx = M.operation_generator_to_index(output_deg - *gen_deg, 0, *gen_deg, *gen_idx);
            println!("index : {}", idx);
        }
        let mut result = FpVector::new(p, output_dim);
        // M.act_on_basis(&mut result, 1, op_deg, op_idx, input_deg, input_idx);
        M.act_on_basis(&mut result, 1, 5, 0, 1, 0);
        println!("{}", result);
        println!("result : {}", M.element_to_string(output_deg, &result));
        result.set_to_zero();
        M.act_on_basis(&mut result, 1, 5, 0, 1, 1);
        println!("{}", result);
        println!("result : {}", M.element_to_string(output_deg, &result));        
        println!("1, 0 : {}", M.basis_element_to_string(1,0));
        println!("1, 1 : {}", M.basis_element_to_string(1,1));
    }

}


// uint FreeModule_element_toJSONString(char *result, FreeModule *this, int degree, Vector *element);
