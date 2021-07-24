use std::sync::Arc;

use bivec::BiVec;

use crate::algebra::{Algebra, Field};
use crate::module::block_structure::BlockStructure;
use crate::module::homomorphism::FreeModuleHomomorphism;
use crate::module::{BoundedModule, FreeModule, Module};
use fp::vector::{Slice, SliceMut};
use once::OnceBiVec;

pub struct HomModule<M: BoundedModule> {
    algebra: Arc<Field>,
    source: Arc<FreeModule<M::Algebra>>,
    target: Arc<M>,
    pub block_structures: OnceBiVec<BlockStructure>,
}

impl<M: BoundedModule> std::fmt::Display for HomModule<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Hom({}, {})", self.source, self.target)
    }
}

impl<M: BoundedModule> HomModule<M> {
    pub fn new(source: Arc<FreeModule<M::Algebra>>, target: Arc<M>) -> Self {
        let p = source.prime();
        let algebra = Arc::new(Field::new(p));
        let min_degree = source.min_degree() - target.max_degree();
        Self {
            algebra,
            source,
            target,
            block_structures: OnceBiVec::new(min_degree), // fn_degree -> blocks
        }
    }

    pub fn source(&self) -> Arc<FreeModule<M::Algebra>> {
        Arc::clone(&self.source)
    }

    pub fn target(&self) -> Arc<M> {
        Arc::clone(&self.target)
    }

    // Each element of HomModule represents a homomorphism from source to target of a given degree.
    // Turn an FpVector representing an element of the HomModule  into a FreeModuleHomomorphism
    pub fn element_to_homomorphism(&self, degree: i32, x: Slice) -> FreeModuleHomomorphism<M> {
        let result =
            FreeModuleHomomorphism::new(Arc::clone(&self.source), Arc::clone(&self.target), degree);
        let min_nonzero_degree = degree + self.target.min_degree();
        let max_nonzero_degree = degree + self.target.max_degree();
        result.extend_by_zero(min_nonzero_degree - 1);
        let mut used_entries = 0;
        for i in min_nonzero_degree..=max_nonzero_degree {
            let gens = self.source.number_of_gens_in_degree(i);
            let out_dim = self.target.dimension(i - degree);
            result.add_generators_from_big_vector(
                i,
                x.slice(used_entries, used_entries + gens * out_dim),
            );
            used_entries += gens * out_dim;
        }
        result
    }

    pub fn evaluate_basis_map_on_element(
        &self,
        mut result: SliceMut,
        coeff: u32,
        f_degree: i32,
        f_idx: usize,
        x_degree: i32,
        x: Slice,
    ) {
        let out_degree = x_degree - f_degree;
        if out_degree < self.target.min_degree() || out_degree > self.target.max_degree() {
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
        let input_block_start = self
            .source
            .operation_generator_to_index(op_deg, 0, gen_deg, gen_idx);
        let input_block_dim = self.source.algebra().dimension(op_deg, gen_deg);
        let input_block_end = input_block_start + input_block_dim;
        let p = *self.prime();
        for i in input_block_start..input_block_end {
            let v = x.entry(i);
            if v == 0 {
                continue;
            }
            let op_idx = i - input_block_start;
            self.target.act_on_basis(
                result.copy(),
                (coeff * v) % p,
                op_deg,
                op_idx,
                mod_deg,
                mod_idx,
            );
        }
    }
}

impl<M: BoundedModule> Module for HomModule<M> {
    type Algebra = Field;

    fn algebra(&self) -> Arc<Self::Algebra> {
        Arc::clone(&self.algebra)
    }

    fn min_degree(&self) -> i32 {
        self.block_structures.min_degree()
    }

    fn max_computed_degree(&self) -> i32 {
        unimplemented!()
    }

    fn compute_basis(&self, degree: i32) {
        // assertion about source:
        // self.source.compute_basis(degree + self.target.max_degree());
        for d in self.min_degree()..=degree {
            let mut block_sizes = BiVec::with_capacity(
                self.target.min_degree() + d,
                self.target.max_degree() + d + 1,
            );
            for i in self.target.min_degree()..=self.target.max_degree() {
                let target_dim = self.target.dimension(i);
                if target_dim == 0 {
                    block_sizes.push(Vec::new());
                    continue;
                }
                let num_gens = self.source.number_of_gens_in_degree(d + i);
                let mut block_sizes_entry = Vec::with_capacity(num_gens);
                for _ in 0..num_gens {
                    block_sizes_entry.push(target_dim)
                }
                block_sizes.push(block_sizes_entry);
            }
            self.block_structures
                .push(BlockStructure::new(&block_sizes));
        }
    }

    fn dimension(&self, degree: i32) -> usize {
        self.block_structures[degree].total_dimension
    }

    fn act_on_basis(
        &self,
        mut result: SliceMut,
        coeff: u32,
        op_degree: i32,
        op_index: usize,
        _mod_degree: i32,
        mod_index: usize,
    ) {
        assert!(op_degree == 0);
        assert!(op_index == 0);
        result.add_basis_element(mod_index, coeff);
    }

    fn basis_element_to_string(&self, degree: i32, idx: usize) -> String {
        let gen_basis_elt = self.block_structures[degree].index_to_generator_basis_elt(idx);
        let gen_deg = gen_basis_elt.generator_degree;
        let gen_idx = gen_basis_elt.generator_index;
        let gen_mod_idx = self
            .source
            .operation_generator_to_index(0, 0, gen_deg, gen_idx);
        let basis_deg = gen_deg - degree;
        let basis_idx = gen_basis_elt.basis_index;
        return format!(
            "{}*{}v",
            self.target.basis_element_to_string(basis_deg, basis_idx),
            self.source.basis_element_to_string(gen_deg, gen_mod_idx)
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algebra::{AdemAlgebra, SteenrodAlgebra};
    use crate::module::homomorphism::ModuleHomomorphism;
    use crate::module::{FDModule, FreeModule, Module};

    use fp::prime::ValidPrime;
    use fp::vector::FpVector;

    #[allow(non_snake_case)]
    #[allow(clippy::needless_range_loop)]
    #[test]
    fn test_hom_space() {
        let p = ValidPrime::new(2);
        let A = Arc::new(SteenrodAlgebra::from(AdemAlgebra::new(
            p,
            *p != 2,
            false,
            false,
        )));
        A.compute_basis(20);
        let F = Arc::new(FreeModule::new(Arc::clone(&A), "".to_string(), 0));
        F.add_generators(0, 1, None);
        F.add_generators(1, 1, None);
        F.add_generators(2, 1, None);
        F.extend_by_zero(20);
        let joker_json_string = r#"{"type" : "finite dimensional module","name": "Joker", "file_name": "Joker", "p": 2, "generic": false, "gens": {"x0": 0, "x1": 1, "x2": 2, "x3": 3, "x4": 4}, "sq_actions": [{"op": 2, "input": "x0", "output": [{"gen": "x2", "coeff": 1}]}, {"op": 2, "input": "x2", "output": [{"gen": "x4", "coeff": 1}]}, {"op": 1, "input": "x0", "output": [{"gen": "x1", "coeff": 1}]}, {"op": 2, "input": "x1", "output": [{"gen": "x3", "coeff": 1}]}, {"op": 1, "input": "x3", "output": [{"gen": "x4", "coeff": 1}]}, {"op": 3, "input": "x1", "output": [{"gen": "x4", "coeff": 1}]}], "adem_actions": [{"op": [1], "input": "x0", "output": [{"gen": "x1", "coeff": 1}]}, {"op": [1], "input": "x3", "output": [{"gen": "x4", "coeff": 1}]}, {"op": [2], "input": "x0", "output": [{"gen": "x2", "coeff": 1}]}, {"op": [2], "input": "x1", "output": [{"gen": "x3", "coeff": 1}]}, {"op": [2], "input": "x2", "output": [{"gen": "x4", "coeff": 1}]}, {"op": [3], "input": "x1", "output": [{"gen": "x4", "coeff": 1}]}, {"op": [2, 1], "input": "x0", "output": [{"gen": "x3", "coeff": 1}]}, {"op": [3, 1], "input": "x0", "output": [{"gen": "x4", "coeff": 1}]}], "milnor_actions": [{"op": [1], "input": "x0", "output": [{"gen": "x1", "coeff": 1}]}, {"op": [1], "input": "x3", "output": [{"gen": "x4", "coeff": 1}]}, {"op": [2], "input": "x0", "output": [{"gen": "x2", "coeff": 1}]}, {"op": [2], "input": "x1", "output": [{"gen": "x3", "coeff": 1}]}, {"op": [2], "input": "x2", "output": [{"gen": "x4", "coeff": 1}]}, {"op": [0, 1], "input": "x0", "output": [{"gen": "x3", "coeff": 1}]}, {"op": [0, 1], "input": "x1", "output": [{"gen": "x4", "coeff": 1}]}, {"op": [3], "input": "x1", "output": [{"gen": "x4", "coeff": 1}]}, {"op": [1, 1], "input": "x0", "output": [{"gen": "x4", "coeff": 1}]}]}"#;
        let joker_json = serde_json::from_str(joker_json_string).unwrap();
        let M = Arc::new(FDModule::from_json(Arc::clone(&A), &joker_json).unwrap());
        let hom = HomModule::new(Arc::clone(&F), Arc::clone(&M));
        hom.compute_basis(10);
        let dimensions = [1, 2, 3, 3, 3, 2, 1, 0];
        for i in -4..=3 {
            assert_eq!(hom.dimension(i), dimensions[(i + 4) as usize]);
        }
        let f_degree = 0;
        let x_degree = 4;
        let out_degree = x_degree - f_degree;
        let mut x = FpVector::new(p, F.dimension(x_degree));
        let mut result = FpVector::new(p, M.dimension(out_degree));
        let mut expected_result = FpVector::new(p, M.dimension(out_degree));
        let outputs = [[0, 0, 0], [1, 0, 0], [0, 1, 0], [0, 0, 0], [0, 0, 1]];
        for i in 0..x.len() {
            x.set_entry(i, 1);
            println!("\n\nx : {}", F.element_to_string(x_degree, x.as_slice()));
            for f_idx in 0..3 {
                hom.evaluate_basis_map_on_element(
                    result.as_slice_mut(),
                    1,
                    f_degree,
                    f_idx,
                    x_degree,
                    x.as_slice(),
                );
                println!(
                    "f : {} ==> f(x) : {}",
                    hom.basis_element_to_string(f_degree, f_idx),
                    M.element_to_string(out_degree, result.as_slice())
                );
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
        let p = ValidPrime::new(2);
        let A = Arc::new(SteenrodAlgebra::from(AdemAlgebra::new(
            p,
            *p != 2,
            false,
            false,
        )));
        A.compute_basis(20);
        let F = Arc::new(FreeModule::new(Arc::clone(&A), "".to_string(), 0));
        F.add_generators(0, 1, None);
        F.add_generators(1, 1, None);
        F.add_generators(2, 1, None);
        F.extend_by_zero(20);
        let joker_json_string = r#"{"type" : "finite dimensional module","name": "Joker", "file_name": "Joker", "p": 2, "generic": false, "gens": {"x0": 0, "x1": 1, "x2": 2, "x3": 3, "x4": 4}, "sq_actions": [{"op": 2, "input": "x0", "output": [{"gen": "x2", "coeff": 1}]}, {"op": 2, "input": "x2", "output": [{"gen": "x4", "coeff": 1}]}, {"op": 1, "input": "x0", "output": [{"gen": "x1", "coeff": 1}]}, {"op": 2, "input": "x1", "output": [{"gen": "x3", "coeff": 1}]}, {"op": 1, "input": "x3", "output": [{"gen": "x4", "coeff": 1}]}, {"op": 3, "input": "x1", "output": [{"gen": "x4", "coeff": 1}]}], "adem_actions": [{"op": [1], "input": "x0", "output": [{"gen": "x1", "coeff": 1}]}, {"op": [1], "input": "x3", "output": [{"gen": "x4", "coeff": 1}]}, {"op": [2], "input": "x0", "output": [{"gen": "x2", "coeff": 1}]}, {"op": [2], "input": "x1", "output": [{"gen": "x3", "coeff": 1}]}, {"op": [2], "input": "x2", "output": [{"gen": "x4", "coeff": 1}]}, {"op": [3], "input": "x1", "output": [{"gen": "x4", "coeff": 1}]}, {"op": [2, 1], "input": "x0", "output": [{"gen": "x3", "coeff": 1}]}, {"op": [3, 1], "input": "x0", "output": [{"gen": "x4", "coeff": 1}]}], "milnor_actions": [{"op": [1], "input": "x0", "output": [{"gen": "x1", "coeff": 1}]}, {"op": [1], "input": "x3", "output": [{"gen": "x4", "coeff": 1}]}, {"op": [2], "input": "x0", "output": [{"gen": "x2", "coeff": 1}]}, {"op": [2], "input": "x1", "output": [{"gen": "x3", "coeff": 1}]}, {"op": [2], "input": "x2", "output": [{"gen": "x4", "coeff": 1}]}, {"op": [0, 1], "input": "x0", "output": [{"gen": "x3", "coeff": 1}]}, {"op": [0, 1], "input": "x1", "output": [{"gen": "x4", "coeff": 1}]}, {"op": [3], "input": "x1", "output": [{"gen": "x4", "coeff": 1}]}, {"op": [1, 1], "input": "x0", "output": [{"gen": "x4", "coeff": 1}]}]}"#;
        let joker_json = serde_json::from_str(joker_json_string).unwrap();
        let M = Arc::new(FDModule::from_json(Arc::clone(&A), &joker_json).unwrap());
        let hom = HomModule::new(Arc::clone(&F), Arc::clone(&M));
        hom.compute_basis(10);

        let f_degree = 0;
        let hom_dim = hom.dimension(f_degree);
        let f_vec = FpVector::from_slice(p, &[1, 0, 1]);
        let f = hom.element_to_homomorphism(f_degree, f_vec.as_slice());
        let mut result = FpVector::new(p, 1);
        for degree in 0..=4 {
            for i in 0..F.dimension(degree) {
                f.apply_to_basis_element(result.as_slice_mut(), 1, degree, i);
                println!(
                    "f({}) = {}",
                    F.basis_element_to_string(degree, i),
                    M.element_to_string(degree - f_degree, result.as_slice())
                );
                result.set_to_zero();
            }
        }

        for i in 0..hom_dim {
            println!("i : {}, f_i : {}", i, hom.basis_element_to_string(0, i));
        }
    }
}
