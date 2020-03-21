use bivec::BiVec;
use once::OnceBiVec;

use crate::algebra::{Algebra, Bialgebra};
use crate::module::block_structure::BlockStructure;
use crate::module::{BoundedModule, Module, ZeroModule};
use fp::vector::{FpVector, FpVectorT};
use fp::prime::minus_one_to_the_n;

use std::sync::Arc;

// This really only makes sense when the algebra is a bialgebra, but associated type bounds are
// unstable. Since the methods are only defined when A is a bialgebra, this is not too much of a
// problem.
pub struct TensorModule<M: Module, N: Module<Algebra = M::Algebra>> {
    pub left: Arc<M>,
    pub right: Arc<N>,
    block_structures: OnceBiVec<BlockStructure>,
}

impl<A, M, N> TensorModule<M, N>
where
    A: Algebra + Bialgebra,
    M: Module<Algebra = A>,
    N: Module<Algebra = A>,
{
    pub fn new(left: Arc<M>, right: Arc<N>) -> Self {
        TensorModule {
            block_structures: OnceBiVec::new(left.min_degree() + right.min_degree()),
            left,
            right,
        }
    }

    pub fn seek_module_num(&self, degree: i32, index: usize) -> i32 {
        self.block_structures[degree]
            .index_to_generator_basis_elt(index)
            .generator_degree
    }

    pub fn offset(&self, degree: i32, left_degree: i32) -> usize {
        self.block_structures[degree]
            .generator_to_block(left_degree, 0)
            .block_start_index
    }

    fn act_helper(
        &self,
        result: &mut FpVector,
        coeff: u32,
        op_degree: i32,
        op_index: usize,
        mod_degree: i32,
        input: &FpVector,
    ) {
        let algebra = self.algebra();
        let p = self.prime();

        let coproduct = algebra.coproduct(op_degree, op_index).into_iter();
        let output_degree = mod_degree + op_degree;

        let borrow_output = self.left.borrow_output() && self.right.borrow_output();

        for (op_deg_l, op_idx_l, op_deg_r, op_idx_r) in coproduct {
            let mut idx = 0;
            for left_deg in self.left.min_degree()..=mod_degree {
                let right_deg = mod_degree - left_deg;

                let left_source_dim = self.left.dimension(left_deg);
                let right_source_dim = self.right.dimension(right_deg);

                let left_target_dim = self.left.dimension(left_deg + op_deg_l);
                let right_target_dim = self.right.dimension(right_deg + op_deg_r);

                if left_target_dim == 0
                    || right_target_dim == 0
                    || left_source_dim == 0
                    || right_source_dim == 0
                {
                    idx += left_source_dim * right_source_dim;
                    continue;
                }
                if borrow_output {
                    for i in 0..left_source_dim {
                        let left_result = self
                            .left
                            .act_on_basis_borrow(op_deg_l, op_idx_l, left_deg, i);

                        if left_result.is_zero_pure() {
                            idx += right_source_dim;
                            continue;
                        }

                        for j in 0..right_source_dim {
                            let entry = input.entry(idx);
                            idx += 1;
                            if entry == 0 {
                                continue;
                            }
                            let right_result = self
                                .right
                                .act_on_basis_borrow(op_deg_r, op_idx_r, right_deg, j);

                            if right_result.is_zero_pure() {
                                continue;
                            }
                            result.add_tensor(
                                self.offset(output_degree, left_deg + op_deg_l),
                                coeff * entry * minus_one_to_the_n(*self.prime(), op_deg_r * left_deg),
                                &left_result,
                                &right_result,
                            );
                        }
                    }
                } else {
                    let mut left_result = FpVector::new(p, left_target_dim);
                    let mut right_result = FpVector::new(p, right_target_dim);

                    for i in 0..left_source_dim {
                        self.left.act_on_basis(
                            &mut left_result,
                            coeff,
                            op_deg_l,
                            op_idx_l,
                            left_deg,
                            i,
                        );

                        if left_result.is_zero() {
                            idx += right_source_dim;
                            continue;
                        }

                        for j in 0..right_source_dim {
                            let entry = input.entry(idx);
                            idx += 1;
                            if entry == 0 {
                                continue;
                            }
                            self.right.act_on_basis(
                                &mut right_result,
                                entry,
                                op_deg_r,
                                op_idx_r,
                                right_deg,
                                j,
                            );

                            if right_result.is_zero() {
                                continue;
                            }
                            result.add_tensor(
                                self.offset(output_degree, left_deg + op_deg_l),
                                minus_one_to_the_n(*self.prime(), op_deg_r * left_deg),
                                &left_result,
                                &right_result,
                            );

                            right_result.set_to_zero();
                        }
                        left_result.set_to_zero();
                    }
                }
            }
        }
    }
}
impl<A, M, N> Module for TensorModule<M, N>
where
    A: Algebra + Bialgebra,
    M: Module<Algebra = A>,
    N: Module<Algebra = A>,
{
    type Algebra = A;

    fn algebra(&self) -> Arc<A> {
        self.left.algebra()
    }

    fn name(&self) -> String {
        format!("{} (x) {}", self.left.name(), self.right.name())
    }

    fn min_degree(&self) -> i32 {
        self.left.min_degree() + self.right.min_degree()
    }

    fn compute_basis(&self, degree: i32) {
        self.left.compute_basis(degree - self.right.min_degree());
        self.right.compute_basis(degree - self.left.min_degree());
        if degree < self.block_structures.len() {
            return;
        }
        for i in self.block_structures.len()..=degree {
            let mut block_sizes = BiVec::with_capacity(
                self.left.min_degree(),
                degree - self.left.min_degree() - self.right.min_degree() + 1,
            );
            for j in self.left.min_degree()..=i - self.right.min_degree() {
                let mut block_sizes_entry = Vec::with_capacity(self.left.dimension(j));
                for _ in 0..self.left.dimension(j) {
                    block_sizes_entry.push(self.right.dimension(i - j))
                }
                block_sizes.push(block_sizes_entry);
            }
            assert_eq!(
                block_sizes.len(),
                i - self.left.min_degree() - self.right.min_degree() + 1
            );
            self.block_structures
                .push(BlockStructure::new(&block_sizes));
        }
    }

    fn dimension(&self, degree: i32) -> usize {
        self.compute_basis(degree);
        match self.block_structures.get(degree) {
            Some(x) => x.total_dimension,
            None => panic!("Hi!"),
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
        let mut working_element = FpVector::new(self.prime(), self.dimension(mod_degree));
        working_element.set_entry(mod_index, 1);

        self.act(
            result,
            coeff,
            op_degree,
            op_index,
            mod_degree,
            &working_element,
        );
    }

    fn act(
        &self,
        result: &mut FpVector,
        coeff: u32,
        op_degree: i32,
        op_index: usize,
        mod_degree: i32,
        input: &FpVector,
    ) {
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
                let mut working_element =
                    FpVector::new(p, self.dimension(working_degree + op_degree));
                self.act_helper(
                    &mut working_element,
                    coeff,
                    op_degree,
                    op_index,
                    working_degree,
                    input,
                );
                working_degree += op_degree;

                for &(op_degree, op_index) in &decomposition[1..n - 1] {
                    let mut new_element =
                        FpVector::new(p, self.dimension(working_degree + op_degree));
                    self.act_helper(
                        &mut new_element,
                        coeff,
                        op_degree,
                        op_index,
                        working_degree,
                        &working_element,
                    );
                    working_element = new_element;
                    working_degree += op_degree;
                }

                let (op_degree, op_index) = decomposition[n - 1];
                self.act_helper(
                    result,
                    coeff,
                    op_degree,
                    op_index,
                    working_degree,
                    &working_element,
                );
            }
        }
    }

    fn basis_element_to_string(&self, degree: i32, idx: usize) -> String {
        let left_degree = self.seek_module_num(degree, idx);
        let right_degree = degree - left_degree;
        let inner_index = idx - self.offset(degree, left_degree);

        let right_dim = self.right.dimension(right_degree);

        let left_index = inner_index / right_dim;
        let right_index = inner_index % right_dim;

        format!(
            "{}.{}",
            self.left.basis_element_to_string(left_degree, left_index),
            self.right
                .basis_element_to_string(right_degree, right_index)
        )
    }
}

impl<A, M, N> BoundedModule for TensorModule<M, N>
where
    A: Algebra + Bialgebra,
    M: Module<Algebra = A> + BoundedModule,
    N: Module<Algebra = A> + BoundedModule,
{
    fn max_degree(&self) -> i32 {
        self.left.max_degree() + self.right.max_degree()
    }
}

impl<A, M, N> ZeroModule for TensorModule<M, N>
where
    A: Algebra + Bialgebra,
    M: Module<Algebra = A> + ZeroModule,
    N: Module<Algebra = A> + ZeroModule,
{
    fn zero_module(algebra: Arc<A>, min_degree: i32) -> Self {
        TensorModule::new(
            Arc::new(M::zero_module(Arc::clone(&algebra), min_degree)),
            Arc::new(N::zero_module(algebra, min_degree)),
        )
    }
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)]

    use super::*;

    use crate::algebra::SteenrodAlgebra;
    use crate::module::FiniteModule;

    #[test]
    fn test_tensor_modules() {
        let k = r#"{"type" : "finite dimensional module","name": "$S_2$", "file_name": "S_2", "p": 2, "generic": false, "gens": {"x0": 0}, "actions": []}"#;
        let kk = r#"{"type" : "finite dimensional module","name": "$S_2$", "file_name": "S_2", "p": 2, "generic": false, "gens": {"x0": 0, "x1":1, "y1":1}, "actions": []}"#;

        let c2 = r#"{"type" : "finite dimensional module", "name": "$C(2)$", "p": 2, "generic": false, "gens": {"x0": 0, "x1": 1}, "actions": ["Sq1 x0 = x1"]}"#;

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

    fn test_tensor_module(M: &str, N: &str, T: &str) {
        let mut M = serde_json::from_str(M).unwrap();
        let mut N = serde_json::from_str(N).unwrap();
        let mut T = serde_json::from_str(T).unwrap();

        let A = Arc::new(SteenrodAlgebra::from_json(&M, "adem".to_string()).unwrap());

        let M = Arc::new(FiniteModule::from_json(Arc::clone(&A), &mut M).unwrap());
        let N = Arc::new(FiniteModule::from_json(Arc::clone(&A), &mut N).unwrap());

        let tensor = TensorModule::new(M, N).to_fd_module();
        let T = FiniteModule::from_json(Arc::clone(&A), &mut T)
            .unwrap()
            .into_fd_module()
            .unwrap();

        if let Err(msg) = tensor.test_equal(&T) {
            panic!("Test case failed. {}", msg);
        }
    }
}
