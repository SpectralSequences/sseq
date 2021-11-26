use bivec::BiVec;
use once::OnceBiVec;

use crate::algebra::{Algebra, Bialgebra};
use crate::module::block_structure::BlockStructure;
use crate::module::{Module, ZeroModule};
use fp::prime::minus_one_to_the_n;
use fp::vector::{prelude::*, FpVector, Slice, SliceMut};

use std::sync::Arc;

// This really only makes sense when the algebra is a bialgebra, but associated type bounds are
// unstable. Since the methods are only defined when A is a bialgebra, this is not too much of a
// problem.
pub struct TensorModule<M: Module, N: Module<Algebra = M::Algebra>> {
    pub left: Arc<M>,
    pub right: Arc<N>,
    block_structures: OnceBiVec<BlockStructure>,
}

impl<M: Module, N: Module<Algebra = M::Algebra>> std::fmt::Display for TensorModule<M, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} (x) {}", self.left, self.right)
    }
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
            .start
    }

    fn act_helper(
        &self,
        mut result: SliceMut,
        coeff: u32,
        op_degree: i32,
        op_index: usize,
        mod_degree: i32,
        input: Slice,
    ) {
        let algebra = self.algebra();
        let p = self.prime();

        let coproduct = algebra.coproduct(op_degree, op_index).into_iter();
        let output_degree = mod_degree + op_degree;

        let mut left_result = FpVector::new(p, 0);
        let mut right_result = FpVector::new(p, 0);

        for (op_deg_l, op_idx_l, op_deg_r, op_idx_r) in coproduct {
            let mut idx = 0;
            for left_deg in self.left.min_degree()..=(mod_degree - self.right.min_degree()) {
                let right_deg = mod_degree - left_deg;

                // Here we use `Module::dimension(&*m, i)` instead of `m.dimension(i)` because there are
                // multiple `dimension` methods in scope and rust-analyzer gets confused if we're not
                // explicit enough.
                let left_source_dim = Module::dimension(&*self.left, left_deg);
                let right_source_dim = Module::dimension(&*self.right, right_deg);

                let left_target_dim = Module::dimension(&*self.left, left_deg + op_deg_l);
                let right_target_dim = Module::dimension(&*self.right, right_deg + op_deg_r);

                if left_target_dim == 0
                    || right_target_dim == 0
                    || left_source_dim == 0
                    || right_source_dim == 0
                {
                    idx += left_source_dim * right_source_dim;
                    continue;
                }

                left_result.set_scratch_vector_size(left_target_dim);
                right_result.set_scratch_vector_size(right_target_dim);

                for i in 0..left_source_dim {
                    self.left.act_on_basis(
                        left_result.as_slice_mut(),
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
                            right_result.as_slice_mut(),
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
                            left_result.as_slice(),
                            right_result.as_slice(),
                        );

                        right_result.set_to_zero();
                    }
                    left_result.set_to_zero();
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

    fn min_degree(&self) -> i32 {
        self.left.min_degree() + self.right.min_degree()
    }

    fn max_computed_degree(&self) -> i32 {
        self.block_structures.len()
    }

    fn compute_basis(&self, degree: i32) {
        self.left.compute_basis(degree - self.right.min_degree());
        self.right.compute_basis(degree - self.left.min_degree());
        self.block_structures.extend(degree, |i| {
            let mut block_sizes =
                BiVec::with_capacity(self.left.min_degree(), i - self.right.min_degree() + 1);
            for j in self.left.min_degree()..=i - self.right.min_degree() {
                // Here we use `Module::dimension(&*m, i)` instead of `m.dimension(i)` because there are
                // multiple `dimension` methods in scope and rust-analyzer gets confused if we're not
                // explicit enough.
                let mut block_sizes_entry = Vec::with_capacity(Module::dimension(&*self.left, j));
                for _ in 0..Module::dimension(&*self.left, j) {
                    block_sizes_entry.push(Module::dimension(&*self.right, i - j))
                }
                block_sizes.push(block_sizes_entry);
            }
            assert_eq!(block_sizes.len(), i - self.right.min_degree() + 1);
            BlockStructure::new(&block_sizes)
        });
    }

    fn dimension(&self, degree: i32) -> usize {
        self.block_structures[degree].total_dimension()
    }

    fn act_on_basis(
        &self,
        result: SliceMut,
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
            working_element.as_slice(),
        );
    }

    fn act(
        &self,
        mut result: SliceMut,
        coeff: u32,
        op_degree: i32,
        op_index: usize,
        mod_degree: i32,
        input: Slice,
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
            1 => self.act_helper(result.copy(), coeff, op_degree, op_index, mod_degree, input),
            n => {
                let (op_degree, op_index) = decomposition[0];

                let mut working_degree = mod_degree;
                let mut working_element =
                    FpVector::new(p, self.dimension(working_degree + op_degree));
                self.act_helper(
                    working_element.as_slice_mut(),
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
                        new_element.as_slice_mut(),
                        coeff,
                        op_degree,
                        op_index,
                        working_degree,
                        working_element.as_slice(),
                    );
                    working_element = new_element;
                    working_degree += op_degree;
                }

                let (op_degree, op_index) = decomposition[n - 1];
                self.act_helper(
                    result.copy(),
                    coeff,
                    op_degree,
                    op_index,
                    working_degree,
                    working_element.as_slice(),
                );
            }
        }
    }

    fn basis_element_to_string(&self, degree: i32, idx: usize) -> String {
        let left_degree = self.seek_module_num(degree, idx);
        let right_degree = degree - left_degree;
        let inner_index = idx - self.offset(degree, left_degree);

        // Here we use `Module::dimension(&*m, i)` instead of `m.dimension(i)` because there are
        // multiple `dimension` methods in scope and rust-analyzer gets confused if we're not
        // explicit enough.
        let right_dim = Module::dimension(&*self.right, right_degree);

        let left_index = inner_index / right_dim;
        let right_index = inner_index % right_dim;

        format!(
            "{}.{}",
            self.left.basis_element_to_string(left_degree, left_index),
            self.right
                .basis_element_to_string(right_degree, right_index)
        )
    }

    fn max_degree(&self) -> Option<i32> {
        Some(self.left.max_degree()? + self.right.max_degree()?)
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
