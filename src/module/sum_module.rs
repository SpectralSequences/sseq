use once::OnceBiVec;

use crate::fp_vector::{FpVector, FpVectorT};
use crate::algebra::AlgebraAny;
use crate::module::{Module, ZeroModule, BoundedModule};

use std::sync::Arc;

pub struct SumModule<M : Module> {
    // We need these because modules might be empty
    algebra : Arc<AlgebraAny>,
    min_degree : i32,
    pub modules : Vec<Arc<M>>,
    // Use BlockStructure for this?
    pub offsets : OnceBiVec<Vec<usize>>,
    dimensions : OnceBiVec<usize>
}

impl<M : Module> SumModule<M> {
    pub fn new(algebra : Arc<AlgebraAny>, modules : Vec<Arc<M>>, min_degree : i32) -> Self {
        SumModule {
            algebra,
            modules,
            min_degree,
            offsets : OnceBiVec::new(min_degree),
            dimensions : OnceBiVec::new(min_degree)
        }
    }

    pub fn seek_module_num(&self, degree : i32, index : usize) -> usize {
        match self.offsets[degree].iter().position(|x| *x > index) {
            Some(n) => n - 1,
            None => self.modules.len() - 1
        }
    }
}

impl<M : Module> Module for SumModule<M> {
    fn algebra(&self) -> Arc<AlgebraAny> {
        Arc::clone(&self.algebra)
    }

    fn name(&self) -> &str {
        "" // Concatenating &str's is hard
    }

    fn min_degree(&self) -> i32 {
        self.min_degree
    }

    fn compute_basis(&self, degree : i32) {
        for module in self.modules.iter() {
            module.compute_basis(degree);
        }

        for i in self.offsets.len() ..= degree {
            let mut offset_vec = Vec::with_capacity(self.modules.len());
            let mut offset = 0;
            for module in self.modules.iter() {
                offset_vec.push(offset);
                offset += module.dimension(i);
            }
            assert_eq!(offset_vec.len(), self.modules.len());
            self.dimensions.push(offset);
            self.offsets.push(offset_vec);
        }
    }

    fn dimension(&self, degree : i32) -> usize {
        *self.dimensions.get(degree).unwrap_or(&(0 as usize))
    }

    fn act_on_basis(&self, result : &mut FpVector, coeff : u32, op_degree : i32, op_index : usize, mod_degree : i32, mod_index : usize) {
        let module_num = self.seek_module_num(mod_degree, mod_index);

        let source_offset = self.offsets[mod_degree][module_num];
        let target_offset = self.offsets[mod_degree + op_degree][module_num];
        let module = &self.modules[module_num];

        let old_slice = result.slice();
        result.set_slice(target_offset, target_offset + module.dimension(mod_degree + op_degree));
        module.act_on_basis(result, coeff, op_degree, op_index, mod_degree, mod_index - source_offset);
        result.restore_slice(old_slice);
    }

    fn basis_element_to_string(&self, degree : i32, idx : usize) -> String {
        let module_num = self.seek_module_num(degree, idx);
        let offset = self.offsets[degree][idx];
        self.modules[module_num].basis_element_to_string(degree, idx - offset)
    }
}

impl<M : BoundedModule> BoundedModule for SumModule<M> {
    fn max_degree(&self) -> i32 {
        self.modules.iter().map(|m| m.max_degree()).max().unwrap_or(self.min_degree - 1)
    }
}

impl<M : Module> ZeroModule for SumModule<M> {
    fn zero_module(algebra : Arc<AlgebraAny>, min_degree : i32) -> Self {
        SumModule::new(algebra, vec![], min_degree)
    }
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)]

    use super::*;

    use crate::module::FiniteModule;
    use crate::algebra::AdemAlgebra;

    #[test]
    fn test_tensor_modules() {
        let k = r#"{"type" : "finite dimensional module","name": "$S_2$", "file_name": "S_2", "p": 2, "generic": false, "gens": {"x0": 0}, "sq_actions": [], "adem_actions": [], "milnor_actions": []}"#;
        let k2 = r#"{"type" : "finite dimensional module","name": "$S_2$", "file_name": "S_2", "p": 2, "generic": false, "gens": {"x0": 0, "y0":0}, "sq_actions": [], "adem_actions": [], "milnor_actions": []}"#;

        let zero = r#"{"type" : "finite dimensional module","name": "$S_2$", "file_name": "S_2", "p": 2, "generic": false, "gens": {}, "adem_actions": []}"#;

        let c2 = r#"{"type" : "finite dimensional module", "name": "$C(2)$", "p": 2, "generic": false, "gens": {"x0": 0, "x1": 1}, "adem_actions": [{"op": [1], "input": "x0", "output": [{"gen": "x1", "coeff": 1}]}]}"#;

        let ceta = r#"{"type" : "finite dimensional module","name": "$C(\\eta)$", "file_name": "Ceta", "p": 2, "generic": false, "gens": {"x0": 0, "x2": 2}, "adem_actions": [{"op": [2], "input": "x0", "output": [{"gen": "x2", "coeff": 1}]}]}"#;

        let c2sumceta = r#"{"type" : "finite dimensional module","name": "$C(\\eta)$", "file_name": "Ceta", "p": 2, "generic": false, "gens": {"x0": 0, "x1": 1,"y0": 0, "y2": 2}, "adem_actions": [{"op": [2], "input": "y0", "output": [{"gen": "y2", "coeff": 1}]}, {"op": [1], "input": "x0", "output": [{"gen": "x1", "coeff": 1}]}]}"#;

        test_sum_module(vec![], zero);
        test_sum_module(vec![k, k], k2);
        test_sum_module(vec![c2, ceta], c2sumceta);
    }

    fn test_sum_module(M : Vec<&str>, S : &str) {
        let p = 2;
        let A = Arc::new(AlgebraAny::from(AdemAlgebra::new(p, p != 2, false)));

        let M : Vec<Arc<FiniteModule>> = M.into_iter().map(|s| {
            let mut m = serde_json::from_str(s).unwrap();
            Arc::new(FiniteModule::from_json(Arc::clone(&A), &mut m).unwrap())
        }).collect::<Vec<_>>();

        let sum = SumModule::new(Arc::clone(&A), M, 0).to_fd_module();

        let mut S = serde_json::from_str(S).unwrap();
        let S = FiniteModule::from_json(Arc::clone(&A), &mut S).unwrap().as_fd_module().unwrap();

        assert!(sum == S);
    }
}
