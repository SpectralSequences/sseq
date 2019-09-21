use bivec::BiVec;
use once::OnceBiVec;

use crate::fp_vector::{FpVector, FpVectorT};
use crate::algebra::{AlgebraAny, Bialgebra};
use crate::module::{Module, ZeroModule, BoundedModule};

use std::sync::Arc;

pub struct TensorModule<M : Module, N : Module> {
    pub left : Arc<M>,
    pub right : Arc<N>,
    // Use BlockStructure for this?
    pub offsets : OnceBiVec<BiVec<usize>>,
    dimensions : OnceBiVec<usize>
}

impl<M : Module, N : Module> TensorModule<M, N> {
    pub fn new(left : Arc<M>, right : Arc<N>) -> Self {
        TensorModule {
            offsets : OnceBiVec::new(left.min_degree() + right.min_degree()),
            dimensions : OnceBiVec::new(left.min_degree() + right.min_degree()),
            left, right
        }
    }

    pub fn seek_module_num(&self, degree : i32, index : usize) -> i32 {
        match self.offsets[degree].iter().position(|x| *x > index) {
            Some(n) => n as i32 - 1 + self.left.min_degree() + self.right.min_degree(),
            None => self.offsets[degree].len() - 1
        }
    }

    fn act_helper(&self, result : &mut FpVector, coeff : u32, op_degree : i32, op_index : usize, mod_degree : i32, input: &FpVector) {
        let algebra = self.algebra();
        let p = self.prime();

        let coproduct = algebra.coproduct(op_degree, op_index).into_iter();

        let source_offset = &self.offsets[mod_degree];
        let target_offset = &self.offsets[mod_degree + op_degree];

        for (op_deg_l, op_idx_l, op_deg_r, op_idx_r) in coproduct {
            for left_deg in source_offset.min_degree() .. source_offset.len() {
                let right_deg = mod_degree - left_deg;

                let left_source_dim = self.left.dimension(left_deg);
                let right_source_dim = self.right.dimension(right_deg);

                let left_target_dim = self.left.dimension(left_deg + op_deg_l);
                let right_target_dim = self.right.dimension(right_deg + op_deg_r);

                if left_target_dim == 0 || right_target_dim == 0 ||
                    left_source_dim == 0 || right_source_dim == 0 {
                        continue;
                    }

                let mut left_result = FpVector::new(p, left_target_dim);
                let mut right_result = FpVector::new(p, right_target_dim);

                for i in 0 .. left_source_dim {
                    self.left.act_on_basis(&mut left_result, coeff, op_deg_l, op_idx_l, left_deg, i);

                    if left_result.is_zero() {
                        continue;
                    }

                    for j in 0 .. right_source_dim {
                        let idx = source_offset[left_deg] + i * right_source_dim + j;
                        let entry = input.entry(idx);
                        if entry == 0 {
                            continue;
                        }
                        self.right.act_on_basis(&mut right_result, entry, op_deg_r, op_idx_r, right_deg, j);

                        if right_result.is_zero() {
                            continue;
                        }
                        result.add_tensor(target_offset[left_deg + op_deg_l], &left_result, &right_result);

                        right_result.set_to_zero();
                    }
                    left_result.set_to_zero();
                }
            }
        }
    }
}

impl<M : Module, N : Module> Module for TensorModule<M, N> {
    fn algebra(&self) -> Arc<AlgebraAny> {
        self.left.algebra()
    }

    fn name(&self) -> &str {
        "" // Concatenating &str's is hard
    }

    fn min_degree(&self) -> i32 {
        self.left.min_degree() + self.right.min_degree()
    }

    fn compute_basis(&self, degree : i32) {
        self.left.compute_basis(degree - self.right.min_degree());
        self.right.compute_basis(degree - self.left.min_degree());

        for i in self.offsets.len() ..= degree {
            let mut offset_vec = BiVec::with_capacity(self.left.min_degree(), degree - self.left.min_degree() - self.right.min_degree() + 1);
            let mut offset = 0;
            for j in self.left.min_degree() ..= i - self.right.min_degree() {
                offset_vec.push(offset);
                offset += self.left.dimension(j) * self.right.dimension(i - j);
            }
            assert_eq!(offset_vec.len(), i - self.left.min_degree() - self.right.min_degree() + 1);
            self.dimensions.push(offset);
            self.offsets.push(offset_vec);
        }
    }

    fn dimension(&self, degree : i32) -> usize {
        *self.dimensions.get(degree).unwrap_or(&(0 as usize))
    }

    fn act_on_basis(&self, result : &mut FpVector, coeff : u32, op_degree : i32, op_index : usize, mod_degree : i32, mod_index : usize) {
        let mut working_element = FpVector::new(self.prime(), self.dimension(mod_degree));
        working_element.set_entry(mod_index, 1);

        self.act(result, coeff, op_degree, op_index, mod_degree, &working_element);
    }

    fn act(&self, result : &mut FpVector, coeff : u32, op_degree : i32, op_index : usize, mod_degree : i32, input : &FpVector) {
        if op_degree == 0 {
            result.add(input, coeff);
            return;
        }

        let algebra = self.algebra();
        let p = self.prime();
        let decomposition = algebra.decompose(op_degree, op_index);

        match decomposition.len() {
            0 => panic!("Decomposition has length 0"),
            1 => self.act_helper(result, coeff, op_degree, op_index, mod_degree, input),
            n => {
                let (op_degree, op_index) = decomposition[0];

                let mut working_degree = mod_degree;
                let mut working_element = FpVector::new(p, self.dimension(working_degree + op_degree));
                self.act_helper(&mut working_element, coeff, op_degree, op_index, working_degree, input);
                working_degree += op_degree;

                for i in 1 .. n - 1 {
                    let (op_degree, op_index) = decomposition[i];
                    let mut new_element = FpVector::new(p, self.dimension(working_degree + op_degree));
                    self.act_helper(&mut new_element, coeff, op_degree, op_index, working_degree, &working_element);
                    working_element = new_element;
                    working_degree += op_degree;
                }

                let (op_degree, op_index) = decomposition[n - 1];
                self.act_helper(result, coeff, op_degree, op_index, working_degree, &working_element);
            }
        }
    }

    fn basis_element_to_string(&self, degree : i32, idx : usize) -> String { String::from("") }
}

impl<M : BoundedModule, N : BoundedModule> BoundedModule for TensorModule<M, N> {
    fn max_degree(&self) -> i32 {
        self.left.max_degree() + self.right.max_degree()
    }
}

impl<M : ZeroModule, N : ZeroModule> ZeroModule for TensorModule<M, N> {
    fn zero_module(algebra : Arc<AlgebraAny>, min_degree : i32) -> Self {
        TensorModule::new(
            Arc::new(M::zero_module(Arc::clone(&algebra), min_degree)),
            Arc::new(N::zero_module(algebra, min_degree))
        )
    }
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)]

    use super::*;

    use crate::module::FiniteModule;

    #[test]
    fn test_tensor_modules() {
        let k = r#"{"type" : "finite dimensional module","name": "$S_2$", "file_name": "S_2", "p": 2, "generic": false, "gens": {"x0": 0}, "sq_actions": [], "adem_actions": [], "milnor_actions": []}"#;
        let kk = r#"{"type" : "finite dimensional module","name": "$S_2$", "file_name": "S_2", "p": 2, "generic": false, "gens": {"x0": 0, "x1":1, "y1":1}, "sq_actions": [], "adem_actions": [], "milnor_actions": []}"#;

        let c2 = r#"{"type" : "finite dimensional module", "name": "$C(2)$", "p": 2, "generic": false, "gens": {"x0": 0, "x1": 1}, "adem_actions": [{"op": [1], "input": "x0", "output": [{"gen": "x1", "coeff": 1}]}]}"#;

        let ceta = r#"{"type" : "finite dimensional module","name": "$C(\\eta)$", "file_name": "Ceta", "p": 2, "generic": false, "gens": {"x0": 0, "x2": 2}, "adem_actions": [{"op": [2], "input": "x0", "output": [{"gen": "x2", "coeff": 1}]}]}"#;

        let c2ceta = r#"{"type" : "finite dimensional module", "name": "$C(2)\\wedge C(\\eta)$", "file_name": "C2_sm_Ceta", "p": 2, "generic": false, "gens": {"x0*x0": 0, "x0*x2": 2, "x1*x0": 1, "x1*x2": 3}, "adem_actions": [{"op": [1], "input": "x0*x0", "output": [{"gen": "x1*x0", "coeff": 1}]}, {"op": [1], "input": "x0*x2", "output": [{"gen": "x1*x2", "coeff": 1}]}, {"op": [2], "input": "x0*x0", "output": [{"gen": "x0*x2", "coeff": 1}]}, {"op": [2], "input": "x1*x0", "output": [{"gen": "x1*x2", "coeff": 1}]}, {"op": [3], "input": "x0*x0", "output": [{"gen": "x1*x2", "coeff": 1}]}, {"op": [2, 1], "input": "x0*x0", "output": [{"gen": "x1*x2", "coeff": 1}]}]}"#;

        let c2c2 = r#"{"type" : "finite dimensional module", "name": "$C(2)$", "p": 2, "generic": false, "gens": {"x0x0": 0, "x0x1": 1, "x1x0" : 1, "x1x1": 2}, "adem_actions": [{"op": [1], "input": "x0x0", "output": [{"gen": "x1x0", "coeff": 1},{"gen": "x0x1", "coeff": 1}]},{"op": [1], "input": "x0x1", "output": [{"gen": "x1x1", "coeff": 1}]}, {"op": [1], "input": "x1x0", "output": [{"gen": "x1x1", "coeff": 1}]}, {"op": [2], "input": "x0x0", "output": [{"gen": "x1x1", "coeff": 1}]}]}"#;

        let c2kk = r#"{"type" : "finite dimensional module", "name": "$C(2)$", "p": 2, "generic": false, "gens": {"x0x0": 0, "x0x1": 1, "x0y1" : 1, "x1x0" : 1, "x1x1": 2, "x1y1": 2}, "adem_actions": [{"op": [1], "input": "x0x0", "output": [{"gen": "x1x0", "coeff": 1}]},{"op": [1], "input": "x0x1", "output": [{"gen": "x1x1", "coeff": 1}]},{"op": [1], "input": "x0y1", "output": [{"gen": "x1y1", "coeff": 1}]}]}"#;

        let kkc2 = r#"{"type" : "finite dimensional module", "name": "$C(2)$", "p": 2, "generic": false, "gens": {"x0x0": 0, "x0x1": 1, "x1x0" : 1, "y1x0" : 1, "x1x1": 2, "y1x1": 2}, "adem_actions": [{"op": [1], "input": "x0x0", "output": [{"gen": "x0x1", "coeff": 1}]},{"op": [1], "input": "x1x0", "output": [{"gen": "x1x1", "coeff": 1}]},{"op": [1], "input": "y1x0", "output": [{"gen": "y1x1", "coeff": 1}]}]}"#;

        test_tensor_module(k, k, k);
        test_tensor_module(k, c2, c2);
        test_tensor_module(c2, k, c2);
        test_tensor_module(c2, kk, c2kk);
        test_tensor_module(kk, c2, kkc2);
        test_tensor_module(ceta, k, ceta);
        test_tensor_module(k, ceta, ceta);
        test_tensor_module(c2, ceta, c2ceta);
        test_tensor_module(ceta, c2, c2ceta);
        test_tensor_module(c2, c2, c2c2);
    }

    fn test_tensor_module(M : &str, N : &str, T : &str) {
        let mut M = serde_json::from_str(M).unwrap();
        let mut N = serde_json::from_str(N).unwrap();
        let mut T = serde_json::from_str(T).unwrap();

        let A = Arc::new(AlgebraAny::from_json(&M, "adem".to_string()).unwrap());

        let M = Arc::new(FiniteModule::from_json(Arc::clone(&A), &mut M).unwrap());
        let N = Arc::new(FiniteModule::from_json(Arc::clone(&A), &mut N).unwrap());

        let tensor = TensorModule::new(M, N).to_fd_module();
        let T = FiniteModule::from_json(Arc::clone(&A), &mut T).unwrap().as_fd_module().unwrap();

        assert!(tensor == T);
    }
}
