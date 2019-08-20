use std::sync::{Mutex, MutexGuard};
use std::rc::Rc;

use crate::once::OnceBiVec;
use crate::fp_vector::{FpVector, FpVectorT};
use crate::matrix::{Matrix, Subspace, QuasiInverse};
// use crate::block_structure::BlockStructure;
// use crate::algebra::AlgebraAny;
// use crate::field::Field;
use crate::module::Module;
use crate::free_module::FreeModule;
use crate::finite_dimensional_module::FiniteDimensionalModuleT;
use crate::hom_space::HomSpace;
use crate::module_homomorphism::ModuleHomomorphism;
use crate::free_module_homomorphism::FreeModuleHomomorphism;

pub struct HomPullback<M : FiniteDimensionalModuleT> {
    source : Rc<HomSpace<M>>,
    target : Rc<HomSpace<M>>,
    map : Rc<FreeModuleHomomorphism<FreeModule>>,
    kernel : OnceBiVec<Subspace>,
    quasi_inverse : OnceBiVec<QuasiInverse>,    
    max_computed_degree : Mutex<i32>,
}

impl<M : FiniteDimensionalModuleT> HomPullback<M> {
    pub fn new(source : Rc<HomSpace<M>>, target : Rc<HomSpace<M>>, map : Rc<FreeModuleHomomorphism<FreeModule>>) -> Self {
        let min_degree = source.min_degree();
        Self {
            source,
            target,
            map,
            max_computed_degree : Mutex::new(min_degree - 1),
            kernel : OnceBiVec::new(min_degree),
            quasi_inverse : OnceBiVec::new(min_degree),
        }
    }
}

impl<M : FiniteDimensionalModuleT> ModuleHomomorphism<HomSpace<M>, HomSpace<M>> for HomPullback<M> {
    fn source(&self) -> Rc<HomSpace<M>> {
        Rc::clone(&self.source)
    }

    fn target(&self) -> Rc<HomSpace<M>> {
        Rc::clone(&self.target)
    }

    fn degree_shift(&self) -> i32 {
        self.map.degree_shift()
    }

    fn min_degree(&self) -> i32 {
        self.source().min_degree()
    }

    fn apply_to_basis_element(&self, result : &mut FpVector, coeff : u32, fn_degree : i32, fn_idx : usize) {
        println!("fn_deg : {}, fn_idx : {}", fn_degree, fn_idx);
        let p = self.prime();
        let target_module = self.target.target();
        for out_deg in target_module.min_degree() ..= target_module.max_degree() {
            let x_degree = fn_degree + out_deg;
            let num_gens = self.map.source().number_of_gens_in_degree(x_degree);
            let old_slice = result.slice();
            for i in 0 .. num_gens {
                let x_elt = self.map.output(x_degree, i);
                let (block_start, block_size) = self.source.block_structures[fn_degree].generator_to_block(x_degree, i);
                result.set_slice(block_start, block_start + block_size);
                self.target.evaluate_basis_map_on_element(result, coeff, fn_degree, fn_idx, x_degree, &x_elt);
                result.restore_slice(old_slice);
            }
        }
    }

    fn lock(&self) -> MutexGuard<i32> {
        self.max_computed_degree.lock().unwrap()
    }

    fn max_kernel_degree(&self) -> i32 {
        self.kernel.max_degree()
    }

    fn set_quasi_inverse(&self, lock : &MutexGuard<i32>, degree : i32, quasi_inverse : QuasiInverse){
        assert!(degree >= self.min_degree());
        assert_eq!(degree, self.quasi_inverse.len());
        self.quasi_inverse.push(quasi_inverse);
    }

    fn quasi_inverse(&self, degree : i32) -> Option<&QuasiInverse> {
        Some(&self.quasi_inverse[degree])
    }

    fn set_kernel(&self, lock : &MutexGuard<i32>, degree : i32, kernel : Subspace) {
        assert!(degree >= self.min_degree());
        assert!(degree == self.kernel.len());
        self.kernel.push(kernel);
    }

    fn kernel(&self, degree : i32) -> Option<&Subspace> {
        Some(&self.kernel[degree])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matrix::Matrix;
    use crate::finite_dimensional_module::FiniteDimensionalModule;
    use crate::algebra::{Algebra, AlgebraAny};
    use crate::adem_algebra::AdemAlgebra;
    use serde_json;

    #[allow(non_snake_case)]
    #[test]
    fn test_pullback(){
        let p = 2;
        let A = Rc::new(AlgebraAny::from(AdemAlgebra::new(p, p != 2, false)));
        A.compute_basis(20);
        let F0 = Rc::new(FreeModule::new(Rc::clone(&A), "F0".to_string(), 0));
        F0.add_generators_immediate(0, 1, None);
        F0.add_generators_immediate(1, 1, None);
        F0.add_generators_immediate(2, 1, None);
        F0.extend_by_zero(20);
        let F1 = Rc::new(FreeModule::new(Rc::clone(&A), "F1".to_string(), 0));
        F1.add_generators_immediate(0, 1, None);
        F1.add_generators_immediate(1, 1, None);
        F1.add_generators_immediate(2, 1, None);
        F1.extend_by_zero(20);
        let d = Rc::new(FreeModuleHomomorphism::new(Rc::clone(&F1), Rc::clone(&F0), 0));
        let mut lock = d.lock();
        
        for i in 0 ..= 1 {
            let mut matrix = Matrix::new(p, 1, F0.dimension(i));
            d.add_generators_from_matrix_rows(&lock, i, &mut matrix, 0, 0);
            *lock += 1;
        }

        let i = 2;
        let mut matrix = Matrix::from_rows(p, vec![FpVector::from_vec(p, &[1, 1, 1])]);
        d.add_generators_from_matrix_rows(&lock, i, &mut matrix, 0, 0);
        *lock += 1;

        let joker_json_string = r#"{"type" : "finite dimensional module","name": "Joker", "file_name": "Joker", "p": 2, "generic": false, "gens": {"x0": 0, "x1": 1, "x2": 2, "x3": 3, "x4": 4}, "sq_actions": [{"op": 2, "input": "x0", "output": [{"gen": "x2", "coeff": 1}]}, {"op": 2, "input": "x2", "output": [{"gen": "x4", "coeff": 1}]}, {"op": 1, "input": "x0", "output": [{"gen": "x1", "coeff": 1}]}, {"op": 2, "input": "x1", "output": [{"gen": "x3", "coeff": 1}]}, {"op": 1, "input": "x3", "output": [{"gen": "x4", "coeff": 1}]}, {"op": 3, "input": "x1", "output": [{"gen": "x4", "coeff": 1}]}], "adem_actions": [{"op": [1], "input": "x0", "output": [{"gen": "x1", "coeff": 1}]}, {"op": [1], "input": "x3", "output": [{"gen": "x4", "coeff": 1}]}, {"op": [2], "input": "x0", "output": [{"gen": "x2", "coeff": 1}]}, {"op": [2], "input": "x1", "output": [{"gen": "x3", "coeff": 1}]}, {"op": [2], "input": "x2", "output": [{"gen": "x4", "coeff": 1}]}, {"op": [3], "input": "x1", "output": [{"gen": "x4", "coeff": 1}]}, {"op": [2, 1], "input": "x0", "output": [{"gen": "x3", "coeff": 1}]}, {"op": [3, 1], "input": "x0", "output": [{"gen": "x4", "coeff": 1}]}], "milnor_actions": [{"op": [1], "input": "x0", "output": [{"gen": "x1", "coeff": 1}]}, {"op": [1], "input": "x3", "output": [{"gen": "x4", "coeff": 1}]}, {"op": [2], "input": "x0", "output": [{"gen": "x2", "coeff": 1}]}, {"op": [2], "input": "x1", "output": [{"gen": "x3", "coeff": 1}]}, {"op": [2], "input": "x2", "output": [{"gen": "x4", "coeff": 1}]}, {"op": [0, 1], "input": "x0", "output": [{"gen": "x3", "coeff": 1}]}, {"op": [0, 1], "input": "x1", "output": [{"gen": "x4", "coeff": 1}]}, {"op": [3], "input": "x1", "output": [{"gen": "x4", "coeff": 1}]}, {"op": [1, 1], "input": "x0", "output": [{"gen": "x4", "coeff": 1}]}]}"#;
        let mut joker_json = serde_json::from_str(&joker_json_string).unwrap();
        let M = Rc::new(FiniteDimensionalModule::from_json(Rc::clone(&A), &mut joker_json));

        let hom0 = Rc::new(HomSpace::new(Rc::clone(&F0), Rc::clone(&M)));
        let hom1 = Rc::new(HomSpace::new(Rc::clone(&F1), Rc::clone(&M)));

        hom0.compute_basis(10);
        hom1.compute_basis(10);




        for i in 0 .. 3 {
            let mut result = FpVector::new(p, 3);
            d.apply_to_basis_element(&mut result, 1, 2, i);
            println!("d({}) = {}", F1.basis_element_to_string(2, i), F0.element_to_string(2, &result));
            result.set_to_zero();
        }
        println!("");

        let outputs = [
            [[0, 0, 0],
            [0, 0, 0],
            [0, 0, 0]],

            [[0, 0, 0],
            [0, 0, 0],
            [0, 0, 0]],

            [[0, 0, 1],
            [0, 0, 1],
            [0, 0, 1]],

            [[0, 0, 1],
            [0, 0, 0],
            [0, 0, 1]],

            [[0, 0, 1],
            [0, 0, 0],
            [0, 0, 1]],

            [[0, 1, 0],
            [0, 1, 0],
            [0, 0, 0]],

            [[1, 0, 0],
            [0, 0, 0],
            [0, 0, 0]],
        ];

        let pb = HomPullback::new(Rc::clone(&hom0), Rc::clone(&hom1), Rc::clone(&d));
        // let mut result = FpVector::new(p, hom1.dimension(deg));
        // pb.apply_to_basis_element(&mut result, 1, deg, idx);
        for deg in -4 .. 3 {
            let mut result = FpVector::new(p, hom1.dimension(deg));
            let mut desired_result = FpVector::new(p, hom1.dimension(deg));
            // println!("deg : {}, dim : {}", deg, hom0.dimension(deg));
            for idx in 0 .. hom0.dimension(deg) {
                // println!("deg = {}, idx = {}, f = {}", deg, idx, hom1.basis_element_to_string(deg, idx));
                pb.apply_to_basis_element(&mut result, 1, deg, idx);
                // println!("d^* {} = {}\n", hom1.basis_element_to_string(deg, idx), hom0.element_to_string(deg, &result));
                let desired_output = outputs[(deg + 4) as usize][idx];
                desired_result.pack(&desired_output[0..desired_result.dimension()]);
                assert_eq!(result, desired_result);
                println!("{}", result);
                result.set_to_zero();
            }
            println!("\n");
        }
    }
}