// use std::sync::Arc;

// use fp::vector::FpVector as FpVectorRust;
// use fp::matrix::{
//     Matrix as MatrixRust,
//     Subspace as SubspaceRust,
//     QuasiInverse as QuasiInverseRust
// };
// use algebra::module::homomorphism::ModuleHomomorphism;
use algebra::module::homomorphism::GenericZeroHomomorphism;


use crate::module::ModuleRust;



pub type ModuleHomomorphismRust = GenericZeroHomomorphism<ModuleRust, ModuleRust>;
// #[allow(dead_code)]
// pub enum ModuleHomomorphismRust {
//     zero : GenericZeroHomomorphism<ModuleRust, ModuleRust>
// }

// impl ModuleHomomorphism for ModuleHomomorphismRust {
//     type Source = ModuleRust;
//     type Target = ModuleRust;

//     fn source(&self) -> Arc<Self::Source> {
        
//     }

//     fn target(&self) -> Arc<Self::Target> {
//         match self {
            
//         }
//     }

//     fn degree_shift(&self) -> i32 {
//         self.degree_shift
//     }

//     fn apply_to_basis_element(
//         &self,
//         result: &mut FpVectorRust,
//         coeff: u32,
//         input_degree: i32,
//         input_index: usize,
//     ) {
//         assert!(input_degree >= self.source.min_degree);
//         let table = &self.source.table[input_degree];
//         self.apply_to_basis_element_with_table(result, coeff, input_degree, table, input_index);
//     }

//     fn quasi_inverse(&self, degree: i32) -> &QuasiInverseRust {
//         debug_assert!(
//             degree >= self.min_degree,
//             "Degree {} less than min degree {}",
//             degree,
//             self.min_degree
//         );
//         &self.quasi_inverse[degree]
//     }

//     fn kernel(&self, degree: i32) -> &SubspaceRust {
//         &self.kernel[degree]
//     }

//     fn get_matrix(&self, matrix: &mut MatrixRust, degree: i32) {
//         self.get_matrix(matrix, degree)
//     }

//     fn compute_kernels_and_quasi_inverses_through_degree(&self, degree: i32) {
//         let _lock = self.lock();
//         let kernel_len = self.kernel.len();
//         let qi_len = self.quasi_inverse.len();
//         assert_eq!(kernel_len, qi_len);
//         for i in kernel_len..=degree {
//             let (kernel, qi) = self.kernel_and_quasi_inverse(i);
//             self.kernel.push(kernel);
//             self.quasi_inverse.push(qi);
//         }
//     }
// }