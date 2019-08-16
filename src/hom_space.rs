use std::rc::Rc;

use bivec::BiVec;

use crate::once::OnceBiVec;
use crate::fp_vector::{FpVector, FpVectorT};
use crate::block_structure::BlockStructure;
use crate::algebra::{Algebra, AlgebraAny};
use crate::field::Field;
use crate::module::Module;
use crate::free_module::FreeModule;
use crate::finite_dimensional_module::FiniteDimensionalModuleT;

struct HomModule<M : FiniteDimensionalModuleT> {
    algebra : Rc<AlgebraAny>,
    source : Rc<FreeModule>,
    target : Rc<M>,
    pub block_structures : OnceBiVec<BlockStructure>,
    pub output_block_structures : BlockStructure
}

impl<M : FiniteDimensionalModuleT> HomModule<M> {
    pub fn new(source : Rc<FreeModule>, target : Rc<M>) -> Self {
        let p = source.prime();
        let algebra = Rc::new(AlgebraAny::from(Field::new(p)));
        let min_degree = source.get_min_degree() - target.max_degree();
        let mut block_sizes = BiVec::with_capacity(target.get_min_degree(), target.max_degree() + 1);
        for i in target.get_min_degree() ..= target.max_degree() {
            println!("i : {}, target_dim : {}", i, target.get_dimension(i));
            block_sizes.push(vec![target.get_dimension(i)]);
        }
        let output_block_structures = BlockStructure::new(&block_sizes);
        Self {
            algebra,
            source,
            target,
            block_structures : OnceBiVec::new(min_degree),
            output_block_structures
        }
    }

    pub fn evaluate_basis_map_on_element(&self, result : &mut FpVector, coeff : u32, f_degree : i32, f_idx : usize, x_degree : i32, x : &FpVector){
        let out_degree = x_degree - f_degree;
        if out_degree < self.target.get_min_degree()
          || out_degree > self.target.max_degree() {
              return;
        }
        let gen_basis_elt = self.block_structures[f_degree].index_to_generator_basis_elt(f_idx);
        let gen_deg = gen_basis_elt.generator_degree;
        let gen_idx = gen_basis_elt.generator_index;
        let op_deg = out_degree - gen_deg;
        let mod_deg = gen_deg - f_degree;
        let mod_idx = gen_basis_elt.basis_index;
        let input_block_start = self.source.operation_generator_to_index(op_deg, 0, gen_deg, gen_idx);
        let input_block_dim = self.source.get_algebra().get_dimension(op_deg, gen_deg);
        let input_block_end = input_block_start + input_block_dim;
        let p = self.source.prime();
        for i in input_block_start .. input_block_end {
            let v = x.get_entry(i);
            let op_idx = i - input_block_start;
            self.target.act_on_basis(result, (coeff * v) % p, op_deg, op_idx, mod_deg, mod_idx);
        }
    }

    // pub fn evaluate_on_basis(&self, result : &mut FpVector, coeff : u32, degree : i32, f : &FpVector, x_idx : usize) {
    //     assert!(degree <= block_structures.max_degree());        
    //     assert!(f.get_dimension() == self.get_dimension(degree));
    //     let operation_generator = self.source.index_to_op_gen(x_idx);
    //     let gen_deg = operation_generator.generator_degree;
    //     let gen_idx = operation_generator.generator_index;
    //     let op_deg = operation_generator.operation_degree;
    //     let op_idx = operation_generator.operation_index;
    //     let (block_start, block_size) = self.block_structures[degree].generator_to_block(generator_degree, generator_index);
    //     let old_slice = f.get_slice();
    //     f.set_slice(block_min, block_max);
    //     self.target.act(result, coeff, op_deg, op_idx, gen_deg, f);
    //     f.restore_slice(old_slice);
    // }

    // pub fn evaluate(&self, result : &mut FpVector, coeff : u32, degree : i32, f : FpVector, x : FpVector) {
    //     assert!(degree <= block_structures.max_degree());
    //     assert!(f.get_dimension() == self.get_dimension(degree));
    //     assert!(x.get_dimension() == self.source.get_dimension(degree));
    //     if generator_degree >= self.get_min_degree() {
    //         let output_on_generator = self.get_output(generator_degree, generator_index);
    //         self.target.act(result, coeff, operation_degree, operation_index, generator_degree - self.degree_shift, output_on_generator);            
    //     }
    //     for (i, v) in x.iter().enumerate() {
    //         self.evaluate_on_basis(result, (coeff * v) % p, degree, f, x_idx)
    //     }
    // }
}

impl<M : FiniteDimensionalModuleT> Module for HomModule<M> {
    fn get_algebra(&self) -> Rc<AlgebraAny> {
        Rc::clone(&self.algebra)
    }

    fn get_name(&self) -> &str {
        &""
    }

    fn get_min_degree(&self) -> i32 {
        self.block_structures.min_degree()
    }

    fn compute_basis(&self, degree : i32) {
        // assertion about source:
        // self.source.compute_basis(degree + self.target.max_degree());
        for d in self.get_min_degree() ..= degree {
            let mut block_sizes = BiVec::with_capacity(self.target.get_min_degree() + d, self.target.max_degree() + d + 1);
            for i in self.target.get_min_degree() ..= self.target.max_degree() {
                let target_dim = self.target.get_dimension(i);
                if target_dim == 0 {
                    block_sizes.push(Vec::new());
                    continue;
                }
                let num_gens = self.source.get_number_of_gens_in_degree(d + i);
                let mut block_sizes_entry = Vec::with_capacity(num_gens);                
                for i in 0 .. num_gens {
                    block_sizes_entry.push(target_dim)
                }
                block_sizes.push(block_sizes_entry);
            }
            self.block_structures.push(BlockStructure::new(&block_sizes));
        }
    }

    fn get_dimension(&self, degree : i32) -> usize {
        self.block_structures[degree].total_dimension
    }

    fn act_on_basis(&self, result : &mut FpVector, coeff : u32, op_degree : i32, op_index : usize, mod_degree : i32, mod_index : usize) {
        assert!(op_degree == 0);
        assert!(op_index == 0);
        result.add_basis_element(mod_index, coeff);    
    }
    
    fn basis_element_to_string(&self, degree : i32, idx : usize) -> String {
        let gen_basis_elt = self.block_structures[degree].index_to_generator_basis_elt(idx);
        let gen_deg = gen_basis_elt.generator_degree;
        let gen_idx = gen_basis_elt.generator_index;
        let gen_mod_idx = self.source.operation_generator_to_index(0, 0, gen_deg, gen_idx);
        let basis_deg = gen_deg - degree;
        let basis_idx = gen_basis_elt.basis_index;
        return format!("{}*{}v", self.target.basis_element_to_string(basis_deg, basis_idx), self.source.basis_element_to_string(gen_deg, gen_mod_idx));
    }
}

mod tests {
    use super::*;
    use crate::finite_dimensional_module::FiniteDimensionalModule;
    use crate::adem_algebra::AdemAlgebra;
    use serde_json;

    #[allow(non_snake_case)]
    #[test]
    fn test_hom_space() {
        let p = 2;
        let A = Rc::new(AlgebraAny::from(AdemAlgebra::new(p, p != 2, false)));
        A.compute_basis(20);
        let F = Rc::new(FreeModule::new(Rc::clone(&A), "".to_string(), 0));
        F.add_generators_immediate(0, 1, None);
        F.add_generators_immediate(1, 1, None);
        F.add_generators_immediate(2, 1, None);
        F.extend_by_zero(20);
        let joker_json_string = r#"{"type" : "finite dimensional module","name": "Joker", "file_name": "Joker", "p": 2, "generic": false, "gens": {"x0": 0, "x1": 1, "x2": 2, "x3": 3, "x4": 4}, "sq_actions": [{"op": 2, "input": "x0", "output": [{"gen": "x2", "coeff": 1}]}, {"op": 2, "input": "x2", "output": [{"gen": "x4", "coeff": 1}]}, {"op": 1, "input": "x0", "output": [{"gen": "x1", "coeff": 1}]}, {"op": 2, "input": "x1", "output": [{"gen": "x3", "coeff": 1}]}, {"op": 1, "input": "x3", "output": [{"gen": "x4", "coeff": 1}]}, {"op": 3, "input": "x1", "output": [{"gen": "x4", "coeff": 1}]}], "adem_actions": [{"op": [1], "input": "x0", "output": [{"gen": "x1", "coeff": 1}]}, {"op": [1], "input": "x3", "output": [{"gen": "x4", "coeff": 1}]}, {"op": [2], "input": "x0", "output": [{"gen": "x2", "coeff": 1}]}, {"op": [2], "input": "x1", "output": [{"gen": "x3", "coeff": 1}]}, {"op": [2], "input": "x2", "output": [{"gen": "x4", "coeff": 1}]}, {"op": [3], "input": "x1", "output": [{"gen": "x4", "coeff": 1}]}, {"op": [2, 1], "input": "x0", "output": [{"gen": "x3", "coeff": 1}]}, {"op": [3, 1], "input": "x0", "output": [{"gen": "x4", "coeff": 1}]}], "milnor_actions": [{"op": [1], "input": "x0", "output": [{"gen": "x1", "coeff": 1}]}, {"op": [1], "input": "x3", "output": [{"gen": "x4", "coeff": 1}]}, {"op": [2], "input": "x0", "output": [{"gen": "x2", "coeff": 1}]}, {"op": [2], "input": "x1", "output": [{"gen": "x3", "coeff": 1}]}, {"op": [2], "input": "x2", "output": [{"gen": "x4", "coeff": 1}]}, {"op": [0, 1], "input": "x0", "output": [{"gen": "x3", "coeff": 1}]}, {"op": [0, 1], "input": "x1", "output": [{"gen": "x4", "coeff": 1}]}, {"op": [3], "input": "x1", "output": [{"gen": "x4", "coeff": 1}]}, {"op": [1, 1], "input": "x0", "output": [{"gen": "x4", "coeff": 1}]}]}"#;
        let mut joker_json = serde_json::from_str(&joker_json_string).unwrap();
        let M = Rc::new(FiniteDimensionalModule::from_json(Rc::clone(&A), &mut joker_json));
        println!("M min_deg : {}, max_deg : {}", M.get_min_degree(), M.max_degree());
        let hom = HomModule::new(Rc::clone(&F), Rc::clone(&M));
        hom.compute_basis(10);
        for i in -4 .. 10 {
            println!("deg {} : {}", i, hom.get_dimension(i));
        }
        let f_degree = 0;
        let x_degree = 4;
        let hom_dim = hom.get_dimension(f_degree);
        let out_degree = x_degree - f_degree;
        let mut x = FpVector::new(2, F.get_dimension(x_degree));
        let mut result = FpVector::new(2, M.get_dimension(out_degree));
        let mut expected_result = FpVector::new(2, M.get_dimension(out_degree));
        let outputs = [[0, 0, 0], [1, 0, 0], [0, 1, 0], [0,0,0], [0,0, 1]];
        for i in 0 .. x.get_dimension(){
            x.set_entry(i, 1);
            println!("\n\nx : {}", F.element_to_string(x_degree, &x));
            for f_idx in 0 .. 3 {
                hom.evaluate_basis_map_on_element(&mut result, 1, f_degree, f_idx, x_degree, &x);
                println!("f : {} ==> f(x) : {}", hom.basis_element_to_string(f_degree, f_idx), M.element_to_string(out_degree, &result));
                expected_result.set_entry(0, outputs[i][f_idx]);
                assert_eq!(result, expected_result);
                result.set_to_zero();
            }
            x.set_to_zero();
        }
    }
}