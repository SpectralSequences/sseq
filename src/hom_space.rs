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
use crate::module_homomorphism::ModuleHomomorphism;
use crate::free_module_homomorphism::FreeModuleHomomorphism;

pub struct HomSpace<M : FiniteDimensionalModuleT> {
    algebra : Rc<AlgebraAny>,
    source : Rc<FreeModule>,
    target : Rc<M>,
    pub block_structures : OnceBiVec<BlockStructure>,
}

impl<M : FiniteDimensionalModuleT> HomSpace<M> {
    pub fn new(source : Rc<FreeModule>, target : Rc<M>) -> Self {
        let p = source.prime();
        let algebra = Rc::new(AlgebraAny::from(Field::new(p)));
        let min_degree = source.min_degree() - target.max_degree();
        Self {
            algebra,
            source,
            target,
            block_structures : OnceBiVec::new(min_degree), // fn_degree -> blocks
        }
    }

    pub fn source(&self) -> Rc<FreeModule> {
        Rc::clone(&self.source)
    }

    pub fn target(&self) -> Rc<M> {
        Rc::clone(&self.target)
    }

    pub fn element_to_homomorphism(&self, degree : i32, x : &mut FpVector) -> FreeModuleHomomorphism<M> {
        let result = FreeModuleHomomorphism::new(Rc::clone(&self.source), Rc::clone(&self.target), degree);
        {// Restrict scope of lock
            let mut lock = result.lock();
            let min_nonzero_degree = degree + self.target.min_degree();
            let max_nonzero_degree = degree + self.target.max_degree();
            result.extend_by_zero(&lock, min_nonzero_degree - 1);
            *lock = min_nonzero_degree - 1;
            let mut used_entries = 0;
            let old_slice = x.slice();
            for i in min_nonzero_degree ..= max_nonzero_degree {
                let gens = self.source.number_of_gens_in_degree(i);
                let out_dim = self.target.dimension(i - degree);
                x.set_slice(used_entries, used_entries + gens * out_dim);
                used_entries += gens * out_dim;
                result.add_generators_from_big_vector(&lock, i, x);
                *lock += 1;
                x.restore_slice(old_slice);
            }
        }
        return result;
    }

    pub fn evaluate_basis_map_on_element(&self, result : &mut FpVector, coeff : u32, f_degree : i32, f_idx : usize, x_degree : i32, x : &FpVector){
        let out_degree = x_degree - f_degree;
        if out_degree < self.target.min_degree()
          || out_degree > self.target.max_degree() {
              return;
        }
        let gen_basis_elt = self.block_structures[f_degree].index_to_generator_basis_elt(f_idx);
        let gen_deg = gen_basis_elt.generator_degree;
        let gen_idx = gen_basis_elt.generator_index;
        let op_deg = x_degree - gen_deg;
        let mod_deg = gen_deg - f_degree;
        let mod_idx = gen_basis_elt.basis_index;
        if op_deg < 0 {
            return;
        }
        let input_block_start = self.source.operation_generator_to_index(op_deg, 0, gen_deg, gen_idx);
        let input_block_dim = self.source.algebra().dimension(op_deg, gen_deg);
        let input_block_end = input_block_start + input_block_dim;
        let p = self.source.prime();
        for i in input_block_start .. input_block_end {
            let v = x.entry(i);
            if v == 0 {
                continue;
            }
            let op_idx = i - input_block_start;
            self.target.act_on_basis(result, (coeff * v) % p, op_deg, op_idx, mod_deg, mod_idx);
        }
    }



}

impl<M : FiniteDimensionalModuleT> Module for HomSpace<M> {
    fn algebra(&self) -> Rc<AlgebraAny> {
        Rc::clone(&self.algebra)
    }

    fn name(&self) -> &str {
        &""
    }

    fn min_degree(&self) -> i32 {
        self.block_structures.min_degree()
    }

    fn compute_basis(&self, degree : i32) {
        // assertion about source:
        // self.source.compute_basis(degree + self.target.max_degree());
        for d in self.min_degree() ..= degree {
            let mut block_sizes = BiVec::with_capacity(self.target.min_degree() + d, self.target.max_degree() + d + 1);
            for i in self.target.min_degree() ..= self.target.max_degree() {
                let target_dim = self.target.dimension(i);
                if target_dim == 0 {
                    block_sizes.push(Vec::new());
                    continue;
                }
                let num_gens = self.source.number_of_gens_in_degree(d + i);
                let mut block_sizes_entry = Vec::with_capacity(num_gens);                
                for i in 0 .. num_gens {
                    block_sizes_entry.push(target_dim)
                }
                block_sizes.push(block_sizes_entry);
            }
            self.block_structures.push(BlockStructure::new(&block_sizes));
        }
    }

    fn dimension(&self, degree : i32) -> usize {
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

#[cfg(test)]
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
        let hom = HomSpace::new(Rc::clone(&F), Rc::clone(&M));
        hom.compute_basis(10);
        let dimensions = [1,2,3,3,3,2,1,0];
        for i in -4 ..= 3 {
            assert_eq!(hom.dimension(i), dimensions[(i + 4) as usize]);
        }
        let f_degree = 0;
        let x_degree = 4;
        let hom_dim = hom.dimension(f_degree);
        let out_degree = x_degree - f_degree;
        let mut x = FpVector::new(2, F.dimension(x_degree));
        let mut result = FpVector::new(2, M.dimension(out_degree));
        let mut expected_result = FpVector::new(2, M.dimension(out_degree));
        let outputs = [[0, 0, 0], [1, 0, 0], [0, 1, 0], [0,0,0], [0,0, 1]];
        for i in 0 .. x.dimension(){
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

    #[allow(non_snake_case)]
    #[test]
    fn test_hom_space_elt_to_map() {
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
        let hom = HomSpace::new(Rc::clone(&F), Rc::clone(&M));
        hom.compute_basis(10);

        let f_degree = 0;
        let hom_dim = hom.dimension(f_degree);
        let mut f_vec = FpVector::from_vec(p, &[1, 0, 1]);
        let f = hom.element_to_homomorphism(f_degree, &mut f_vec);
        let mut result = FpVector::new(p, 1);
        for degree in 0 ..= 4 {
            for i in 0 .. F.dimension(degree) {
                f.apply_to_basis_element(&mut result, 1, degree, i);
                println!("f({}) = {}", F.basis_element_to_string(degree, i), M.element_to_string(degree - f_degree, &result));
                result.set_to_zero();
            }
        }

        for i in 0 .. hom_dim {
            println!("i : {}, f_i : {}", i, hom.basis_element_to_string(0, i));
        }
        
    }    
}
