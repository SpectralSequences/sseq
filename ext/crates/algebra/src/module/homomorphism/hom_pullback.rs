use std::sync::Arc;

use crate::module::block_structure::BlockStart;
use fp::matrix::{QuasiInverse, Subspace};
use fp::vector::SliceMut;
use once::OnceBiVec;
// use crate::algebra::SteenrodAlgebra;
// use crate::field::Field;
use crate::module::homomorphism::{FreeModuleHomomorphism, ModuleHomomorphism};
use crate::module::HomModule;
use crate::module::{BoundedModule, FreeModule, Module};

/// Given a map `map`: A -> B and `source` = Hom(B, X), `target` = Hom(A, X), produce the induced
/// map `map`^* Hom(B, X) -> Hom(A, X).
pub struct HomPullback<M: BoundedModule> {
    source: Arc<HomModule<M>>,
    target: Arc<HomModule<M>>,
    map: Arc<FreeModuleHomomorphism<FreeModule<M::Algebra>>>,
    images: OnceBiVec<Subspace>,
    kernels: OnceBiVec<Subspace>,
    quasi_inverses: OnceBiVec<QuasiInverse>,
}

impl<M: BoundedModule> HomPullback<M> {
    pub fn new(
        source: Arc<HomModule<M>>,
        target: Arc<HomModule<M>>,
        map: Arc<FreeModuleHomomorphism<FreeModule<M::Algebra>>>,
    ) -> Self {
        let min_degree = source.min_degree();
        Self {
            source,
            target,
            map,
            images: OnceBiVec::new(min_degree),
            kernels: OnceBiVec::new(min_degree),
            quasi_inverses: OnceBiVec::new(min_degree),
        }
    }
}

impl<M: BoundedModule> ModuleHomomorphism for HomPullback<M> {
    type Source = HomModule<M>;
    type Target = HomModule<M>;

    fn source(&self) -> Arc<Self::Source> {
        Arc::clone(&self.source)
    }

    fn target(&self) -> Arc<Self::Target> {
        Arc::clone(&self.target)
    }

    fn degree_shift(&self) -> i32 {
        self.map.degree_shift()
    }

    fn min_degree(&self) -> i32 {
        self.source().min_degree()
    }

    fn apply_to_basis_element(
        &self,
        mut result: SliceMut,
        coeff: u32,
        fn_degree: i32,
        fn_idx: usize,
    ) {
        println!("fn_deg : {}, fn_idx : {}", fn_degree, fn_idx);
        let target_module = self.target.target();
        for out_deg in target_module.min_degree()..=target_module.max_degree() {
            let x_degree = fn_degree + out_deg;
            let num_gens = self.map.source().number_of_gens_in_degree(x_degree);
            for i in 0..num_gens {
                let x_elt = self.map.output(x_degree, i);
                let BlockStart {
                    block_start_index,
                    block_size,
                } = self.source.block_structures[fn_degree].generator_to_block(x_degree, i);
                self.target.evaluate_basis_map_on_element(
                    result.slice_mut(*block_start_index, *block_start_index + block_size),
                    coeff,
                    fn_degree,
                    fn_idx,
                    x_degree,
                    x_elt.as_slice(),
                );
            }
        }
    }

    fn compute_auxiliary_data_through_degree(&self, degree: i32) {
        self.kernels.extend(degree, |i| {
            let (image, kernel, qi) = self.auxiliary_data(i);
            self.images.push_checked(image, i);
            self.quasi_inverses.push_checked(qi, i);
            kernel
        });
    }

    fn quasi_inverse(&self, degree: i32) -> Option<&QuasiInverse> {
        self.quasi_inverses.get(degree)
    }

    fn kernel(&self, degree: i32) -> Option<&Subspace> {
        self.kernels.get(degree)
    }

    fn image(&self, degree: i32) -> Option<&Subspace> {
        self.images.get(degree)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algebra::{AdemAlgebra, Algebra, SteenrodAlgebra};
    use crate::module::FDModule;
    use fp::matrix::Matrix;
    use fp::prime::ValidPrime;
    use fp::vector::FpVector;

    #[allow(non_snake_case)]
    #[test]
    fn test_pullback() {
        let p = ValidPrime::new(2);
        let A = Arc::new(SteenrodAlgebra::from(AdemAlgebra::new(
            p,
            *p != 2,
            false,
            false,
        )));
        A.compute_basis(20);
        let F0 = Arc::new(FreeModule::new(Arc::clone(&A), "F0".to_string(), 0));
        F0.add_generators(0, 1, None);
        F0.add_generators(1, 1, None);
        F0.add_generators(2, 1, None);
        F0.extend_by_zero(20);
        let F1 = Arc::new(FreeModule::new(Arc::clone(&A), "F1".to_string(), 0));
        F1.add_generators(0, 1, None);
        F1.add_generators(1, 1, None);
        F1.add_generators(2, 1, None);
        F1.extend_by_zero(20);
        let d = Arc::new(FreeModuleHomomorphism::new(
            Arc::clone(&F1),
            Arc::clone(&F0),
            0,
        ));
        for i in 0..=1 {
            let mut matrix = Matrix::new(p, 1, F0.dimension(i));
            d.add_generators_from_matrix_rows(i, matrix.as_slice_mut());
        }

        let i = 2;
        let mut matrix = Matrix::from_rows(p, vec![FpVector::from_slice(p, &[1, 1, 1])], 3);
        d.add_generators_from_matrix_rows(i, matrix.as_slice_mut());

        let joker_json_string = r#"{"type" : "finite dimensional module","name": "Joker", "file_name": "Joker", "p": 2, "generic": false, "gens": {"x0": 0, "x1": 1, "x2": 2, "x3": 3, "x4": 4}, "sq_actions": [{"op": 2, "input": "x0", "output": [{"gen": "x2", "coeff": 1}]}, {"op": 2, "input": "x2", "output": [{"gen": "x4", "coeff": 1}]}, {"op": 1, "input": "x0", "output": [{"gen": "x1", "coeff": 1}]}, {"op": 2, "input": "x1", "output": [{"gen": "x3", "coeff": 1}]}, {"op": 1, "input": "x3", "output": [{"gen": "x4", "coeff": 1}]}, {"op": 3, "input": "x1", "output": [{"gen": "x4", "coeff": 1}]}], "adem_actions": [{"op": [1], "input": "x0", "output": [{"gen": "x1", "coeff": 1}]}, {"op": [1], "input": "x3", "output": [{"gen": "x4", "coeff": 1}]}, {"op": [2], "input": "x0", "output": [{"gen": "x2", "coeff": 1}]}, {"op": [2], "input": "x1", "output": [{"gen": "x3", "coeff": 1}]}, {"op": [2], "input": "x2", "output": [{"gen": "x4", "coeff": 1}]}, {"op": [3], "input": "x1", "output": [{"gen": "x4", "coeff": 1}]}, {"op": [2, 1], "input": "x0", "output": [{"gen": "x3", "coeff": 1}]}, {"op": [3, 1], "input": "x0", "output": [{"gen": "x4", "coeff": 1}]}], "milnor_actions": [{"op": [1], "input": "x0", "output": [{"gen": "x1", "coeff": 1}]}, {"op": [1], "input": "x3", "output": [{"gen": "x4", "coeff": 1}]}, {"op": [2], "input": "x0", "output": [{"gen": "x2", "coeff": 1}]}, {"op": [2], "input": "x1", "output": [{"gen": "x3", "coeff": 1}]}, {"op": [2], "input": "x2", "output": [{"gen": "x4", "coeff": 1}]}, {"op": [0, 1], "input": "x0", "output": [{"gen": "x3", "coeff": 1}]}, {"op": [0, 1], "input": "x1", "output": [{"gen": "x4", "coeff": 1}]}, {"op": [3], "input": "x1", "output": [{"gen": "x4", "coeff": 1}]}, {"op": [1, 1], "input": "x0", "output": [{"gen": "x4", "coeff": 1}]}]}"#;
        let joker_json = serde_json::from_str(joker_json_string).unwrap();
        let M = Arc::new(FDModule::from_json(Arc::clone(&A), &joker_json).unwrap());

        let hom0 = Arc::new(HomModule::new(Arc::clone(&F0), Arc::clone(&M)));
        let hom1 = Arc::new(HomModule::new(Arc::clone(&F1), Arc::clone(&M)));

        hom0.compute_basis(10);
        hom1.compute_basis(10);

        for i in 0..3 {
            let mut result = FpVector::new(p, 3);
            d.apply_to_basis_element(result.as_slice_mut(), 1, 2, i);
            println!(
                "d({}) = {}",
                F1.basis_element_to_string(2, i),
                F0.element_to_string(2, result.as_slice())
            );
            result.set_to_zero();
        }
        println!();

        let outputs = [
            [[0, 0, 0], [0, 0, 0], [0, 0, 0]],
            [[0, 0, 0], [0, 0, 0], [0, 0, 0]],
            [[0, 0, 1], [0, 0, 1], [0, 0, 1]],
            [[0, 0, 1], [0, 0, 0], [0, 0, 1]],
            [[0, 0, 1], [0, 0, 0], [0, 0, 1]],
            [[0, 1, 0], [0, 1, 0], [0, 0, 0]],
            [[1, 0, 0], [0, 0, 0], [0, 0, 0]],
        ];

        let pb = HomPullback::new(Arc::clone(&hom0), Arc::clone(&hom1), Arc::clone(&d));
        // let mut result = FpVector::new(p, hom1.dimension(deg));
        // pb.apply_to_basis_element(&mut result, 1, deg, idx);
        for deg in -4..3 {
            let mut result = FpVector::new(p, hom1.dimension(deg));
            let mut desired_result = FpVector::new(p, hom1.dimension(deg));
            // println!("deg : {}, dim : {}", deg, hom0.dimension(deg));
            for idx in 0..hom0.dimension(deg) {
                // println!("deg = {}, idx = {}, f = {}", deg, idx, hom1.basis_element_to_string(deg, idx));
                pb.apply_to_basis_element(result.as_slice_mut(), 1, deg, idx);
                // println!("d^* {} = {}\n", hom1.basis_element_to_string(deg, idx), hom0.element_to_string(deg, &result));
                let desired_output = outputs[(deg + 4) as usize][idx];
                desired_result.copy_from_slice(&desired_output[0..desired_result.len()]);
                assert_eq!(result, desired_result);
                println!("{}", result);
                result.set_to_zero();
            }
            println!("\n");
        }
    }
}
