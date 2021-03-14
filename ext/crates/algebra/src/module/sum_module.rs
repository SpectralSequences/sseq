#![cfg_attr(rustfmt, rustfmt_skip)]
use bivec::BiVec;
use once::OnceBiVec;

use crate::module::block_structure::{BlockStart, BlockStructure, GeneratorBasisEltPair};
use crate::module::{BoundedModule, Module, ZeroModule};
use fp::vector::FpVector;

use std::sync::Arc;

pub struct SumModule<M: Module> {
    // We need these because modules might be empty
    algebra: Arc<M::Algebra>,
    min_degree: i32,
    pub modules: Vec<Arc<M>>,
    // Use BlockStructure for this?
    pub block_structures: OnceBiVec<BlockStructure>,
}

impl<M: Module> SumModule<M> {
    pub fn new(algebra: Arc<M::Algebra>, modules: Vec<Arc<M>>, min_degree: i32) -> Self {
        SumModule {
            algebra,
            modules,
            min_degree,
            block_structures: OnceBiVec::new(min_degree),
        }
    }

    pub fn get_module_num(&self, degree: i32, index: usize) -> usize {
        self.block_structures[degree]
            .index_to_generator_basis_elt(index)
            .generator_index
    }

    pub fn offset(&self, degree: i32, module_num: usize) -> usize {
        self.block_structures[degree]
            .generator_to_block(degree, module_num)
            .block_start_index
    }
}

impl<M: Module> Module for SumModule<M> {
    type Algebra = M::Algebra;

    fn algebra(&self) -> Arc<Self::Algebra> {
        Arc::clone(&self.algebra)
    }

    fn name(&self) -> String {
        if self.modules.is_empty() {
            String::from("0")
        } else {
            let mut name = self.modules[0].name();
            for n in self.modules[1..].iter().map(|m| m.name()) {
                name.push_str(&n)
            }
            name
        }
    }

    fn min_degree(&self) -> i32 {
        self.min_degree
    }

    fn compute_basis(&self, degree: i32) {
        for module in &self.modules {
            module.compute_basis(degree);
        }
        for i in self.block_structures.len()..=degree {
            let mut block_sizes = BiVec::new(i);
            block_sizes.push(self.modules.iter().map(|m| m.dimension(i)).collect());
            self.block_structures
                .push(BlockStructure::new(&block_sizes));
        }
    }

    fn max_computed_degree(&self) -> i32 {
        self.block_structures.len()
    }

    fn dimension(&self, degree: i32) -> usize {
        match self.block_structures.get(degree) {
            Some(x) => x.total_dimension,
            None => 0,
        }
    }

    fn act_on_basis(
        &self,
        result: &mut FpVector,
        coeff: u32,
        op_degree: i32,
        op_index: usize,
        mod_degree: i32,
        mod_index: usize,
    ) {
        let target_degree = mod_degree + op_degree;
        let GeneratorBasisEltPair {
            generator_index: module_num,
            basis_index,
            ..
        } = self.block_structures[mod_degree].index_to_generator_basis_elt(mod_index);
        let BlockStart {
            block_start_index: target_offset,
            block_size: target_module_dimension,
        } = self.block_structures[target_degree].generator_to_block(target_degree, *module_num);
        let module = &self.modules[*module_num];

        module.act_on_basis(
            &mut *result.borrow_slice(*target_offset, target_offset + target_module_dimension),
            coeff,
            op_degree,
            op_index,
            mod_degree,
            *basis_index,
        );
    }

    fn basis_element_to_string(&self, degree: i32, index: usize) -> String {
        let GeneratorBasisEltPair {
            generator_index: module_num,
            basis_index,
            ..
        } = self.block_structures[degree].index_to_generator_basis_elt(index);
        self.modules[*module_num].basis_element_to_string(degree, *basis_index)
    }
}

impl<M: BoundedModule> BoundedModule for SumModule<M> {
    fn max_degree(&self) -> i32 {
        self.modules
            .iter()
            .map(|m| m.max_degree())
            .max()
            .unwrap_or(self.min_degree - 1)
    }
}

impl<M: Module> ZeroModule for SumModule<M> {
    fn zero_module(algebra: Arc<M::Algebra>, min_degree: i32) -> Self {
        SumModule::new(algebra, vec![], min_degree)
    }
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)]

    use super::*;

    use crate::algebra::{AdemAlgebra, SteenrodAlgebra};
    use crate::module::FiniteModule;

    #[test]
    fn test_sum_modules() {
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

    fn test_sum_module(M: Vec<&str>, S: &str) {
        let p = fp::prime::ValidPrime::new(2);
        let A = Arc::new(SteenrodAlgebra::from(AdemAlgebra::new(p, *p != 2, false, false)));

        let M: Vec<Arc<FiniteModule>> = M
            .into_iter()
            .map(|s| {
                let mut m = serde_json::from_str(s).unwrap();
                Arc::new(FiniteModule::from_json(Arc::clone(&A), &mut m).unwrap())
            })
            .collect::<Vec<_>>();

        let sum = SumModule::new(Arc::clone(&A), M, 0).to_fd_module();

        let mut S = serde_json::from_str(S).unwrap();
        let S = FiniteModule::from_json(Arc::clone(&A), &mut S)
            .unwrap()
            .into_fd_module()
            .unwrap();

        if let Err(msg) = sum.test_equal(&S) {
            panic!("Test case failed. {}", msg);
        }
    }
}
