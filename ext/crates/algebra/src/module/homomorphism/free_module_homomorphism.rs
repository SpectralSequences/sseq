use parking_lot::{Mutex, MutexGuard};
use std::sync::Arc;

use crate::module::homomorphism::ModuleHomomorphism;
use crate::module::{FreeModule, Module};
use fp::matrix::{Matrix, QuasiInverse, Subspace};
use fp::vector::{FpVector, FpVectorT};
use once::OnceBiVec;

pub struct FreeModuleHomomorphism<M: Module> {
    source: Arc<FreeModule<M::Algebra>>,
    target: Arc<M>,
    outputs: OnceBiVec<Vec<FpVector>>, // degree --> input_idx --> output
    pub kernel: OnceBiVec<Subspace>,
    pub quasi_inverse: OnceBiVec<QuasiInverse>,
    min_degree: i32,
    lock: Mutex<()>,
    /// degree shift, such that ouptut_degree = input_degree - degree_shift
    degree_shift: i32,
}

impl<M: Module> ModuleHomomorphism for FreeModuleHomomorphism<M> {
    type Source = FreeModule<M::Algebra>;
    type Target = M;

    fn source(&self) -> Arc<Self::Source> {
        Arc::clone(&self.source)
    }

    fn target(&self) -> Arc<Self::Target> {
        Arc::clone(&self.target)
    }

    fn degree_shift(&self) -> i32 {
        self.degree_shift
    }

    fn apply_to_basis_element(
        &self,
        result: &mut FpVector,
        coeff: u32,
        input_degree: i32,
        input_index: usize,
    ) {
        assert!(input_degree >= self.source.min_degree);
        assert!(input_index < self.source.basis_element_to_opgen[input_degree].len());
        let output_degree = input_degree - self.degree_shift;
        assert_eq!(self.target.dimension(output_degree), result.dimension());
        let operation_generator = &self.source.basis_element_to_opgen[input_degree][input_index];
        let operation_degree = operation_generator.operation_degree;
        let operation_index = operation_generator.operation_index;
        let generator_degree = operation_generator.generator_degree;
        let generator_index = operation_generator.generator_index;
        if generator_degree >= self.min_degree() {
            let output_on_generator = self.output(generator_degree, generator_index);
            self.target.act(
                result,
                coeff,
                operation_degree,
                operation_index,
                generator_degree - self.degree_shift,
                output_on_generator,
            );
        }
    }

    fn quasi_inverse(&self, degree: i32) -> &QuasiInverse {
        debug_assert!(
            degree >= self.min_degree,
            "Degree {} less than min degree {}",
            degree,
            self.min_degree
        );
        &self.quasi_inverse[degree]
    }

    fn kernel(&self, degree: i32) -> &Subspace {
        &self.kernel[degree]
    }

    fn get_matrix(&self, matrix: &mut Matrix, degree: i32) {
        self.get_matrix(matrix, degree)
    }

    fn compute_kernels_and_quasi_inverses_through_degree(&self, degree: i32) {
        let _lock = self.lock();
        let kernel_len = self.kernel.len();
        let qi_len = self.quasi_inverse.len();
        assert_eq!(kernel_len, qi_len);
        for i in kernel_len..=degree {
            let (kernel, qi) = self.kernel_and_quasi_inverse(i);
            self.kernel.push(kernel);
            self.quasi_inverse.push(qi);
        }
    }
}

// // Run FreeModule_ConstructBlockOffsetTable(source, degree) before using this on an input in that degree
// void FreeModuleHomomorphism_applyToBasisElement(FreeModuleHomomorphism *f, Vector *result, uint coeff, int input_degree, uint input_index){

// }

impl<M: Module> FreeModuleHomomorphism<M> {
    pub fn new(source: Arc<FreeModule<M::Algebra>>, target: Arc<M>, degree_shift: i32) -> Self {
        let min_degree = std::cmp::max(source.min_degree(), target.min_degree() + degree_shift);
        let outputs = OnceBiVec::new(min_degree);
        let kernel = OnceBiVec::new(min_degree);
        let quasi_inverse = OnceBiVec::new(min_degree);
        Self {
            source,
            target,
            outputs,
            kernel,
            quasi_inverse,
            min_degree,
            lock: Mutex::new(()),
            degree_shift,
        }
    }

    pub fn degree_shift(&self) -> i32 {
        self.degree_shift
    }

    pub fn min_degree(&self) -> i32 {
        self.min_degree
    }

    pub fn next_degree(&self) -> i32 {
        self.outputs.len()
    }

    pub fn output(&self, generator_degree: i32, generator_index: usize) -> &FpVector {
        assert!(
            generator_degree >= self.min_degree(),
            "generator_degree {} less than min degree {}",
            generator_degree,
            self.min_degree()
        );
        assert!(
            generator_index < self.source.number_of_gens_in_degree(generator_degree),
            "generator_index {} greater than number of generators {}",
            generator_index,
            self.source.number_of_gens_in_degree(generator_degree)
        );
        &self.outputs[generator_degree][generator_index]
    }

    pub fn extend_by_zero_safe(&self, degree: i32) {
        if self.outputs.len() > degree {
            return;
        }
        let lock = self.lock();
        self.extend_by_zero(&lock, degree);
    }
    
    pub fn extend_by_zero(&self, lock: &MutexGuard<()>, degree: i32) {
        self.check_mutex(lock);

        // println!("    add_gens_from_matrix degree : {}, first_new_row : {}, new_generators : {}", degree, first_new_row, new_generators);
        // println!("    dimension : {} target name : {}", dimension, self.target.name());
        if degree < self.min_degree {
            return;
        }
        let next_degree = self.next_degree();
        assert!(degree >= next_degree);
        let p = self.prime();
        for i in next_degree..=degree {
            let num_gens = self.source.number_of_gens_in_degree(i);
            let dimension = self.target.dimension(i - self.degree_shift);
            let mut new_outputs: Vec<FpVector> = Vec::with_capacity(num_gens);
            for _ in 0..num_gens {
                new_outputs.push(FpVector::new(p, dimension));
            }
            self.outputs.push(new_outputs);
        }
    }

    // We don't actually mutate vector, we just slice it.
    pub fn add_generators_from_big_vector(
        &self,
        lock: &MutexGuard<()>,
        degree: i32,
        outputs_vectors: &mut FpVector,
    ) {
        self.check_mutex(lock);
        assert_eq!(degree, self.outputs.len());

        let p = self.prime();
        let new_generators = self.source.number_of_gens_in_degree(degree);
        let target_dimension = self.target.dimension(degree - self.degree_shift);
        let mut new_outputs: Vec<FpVector> = Vec::with_capacity(new_generators);
        for _ in 0..new_generators {
            new_outputs.push(FpVector::new(p, target_dimension));
        }
        if target_dimension == 0 {
            self.outputs.push(new_outputs);
            return;
        }
        for (i, new_output) in new_outputs.iter_mut().enumerate() {
            let old_slice = outputs_vectors.slice();
            outputs_vectors.set_slice(target_dimension * i, target_dimension * (i + 1));
            new_output.shift_assign(&outputs_vectors);
            outputs_vectors.restore_slice(old_slice);
        }
        self.outputs.push(new_outputs);
    }

    pub fn add_generators_from_matrix_rows(
        &self,
        lock: &MutexGuard<()>,
        degree: i32,
        matrix: &Matrix
    ) {
        self.check_mutex(lock);
        assert_eq!(degree, self.outputs.len());
        
        let p = self.prime();
        let new_generators = self.source.number_of_gens_in_degree(degree);
        let target_dimension = self.target.dimension(degree - self.degree_shift);
        
        let mut new_outputs: Vec<FpVector> = Vec::with_capacity(new_generators);
        for _ in 0..new_generators {
            new_outputs.push(FpVector::new(p, target_dimension));
        }
        if target_dimension == 0 {
            self.outputs.push(new_outputs);
            return;
        }
        for (i, new_output) in new_outputs.iter_mut().enumerate() {
            new_output.assign(&matrix[i]);
        }
        self.outputs.push(new_outputs);
    }

    pub fn add_generators_from_rows(
        &self,
        lock: &MutexGuard<()>,
        degree: i32,
        rows: Vec<FpVector>,
    ) {
        self.check_mutex(lock);
        assert_eq!(degree, self.outputs.len());
        self.outputs.push(rows);
    }

    pub fn apply_to_generator(&self, result: &mut FpVector, coeff: u32, degree: i32, idx: usize) {
        let output_on_gen = self.output(degree, idx);
        result.add(output_on_gen, coeff);
    }

    pub fn get_matrix(&self, matrix: &mut Matrix, degree: i32) {
        // let source_dimension = FreeModule::<M::Algebra>::dimension_with_table(table);
        // let target_dimension = self.target().dimension(degree);
        // if source_dimension != matrix.rows() {
        //     panic!(
        //         "get_matrix_with_table for homomorphism {} -> {} in degree {}: table source dimension {} not equal to number of matrix rows {}.", 
        //         self.source().name(),
        //         self.target().name(),
        //         degree,
        //         source_dimension, 
        //         matrix.rows()
        //     );
        // }
        // if target_dimension != matrix.columns() {
        //     panic!(
        //         "get_matrix_with_table for homomorphism {} -> {} in degree {}: table target dimension {} not equal to number of matrix columns {}.", 
        //         self.source().name(),
        //         self.target().name(),
        //         degree,
        //         target_dimension, 
        //         matrix.columns()
        //     );
        // }

        for (i, row) in matrix.iter_mut().enumerate() {
            self.apply_to_basis_element(row, 1, degree, i);
        }
    }

    pub fn lock(&self) -> MutexGuard<()> {
        self.lock.lock()
    }

    pub fn set_kernel(&self, lock: &MutexGuard<()>, degree: i32, kernel: Subspace) {
        self.check_mutex(lock);
        assert!(degree == self.kernel.len());
        self.kernel.push(kernel);
    }

    pub fn set_quasi_inverse(
        &self,
        lock: &MutexGuard<()>,
        degree: i32,
        quasi_inverse: QuasiInverse,
    ) {
        self.check_mutex(lock);
        assert!(degree == self.quasi_inverse.len());
        self.quasi_inverse.push(quasi_inverse);
    }

    fn check_mutex(&self, lock: &MutexGuard<()>) {
        assert!(std::ptr::eq(parking_lot::lock_api::MutexGuard::mutex(&lock), &self.lock));
    }
}


use saveload::{Load, Save};
use std::io;
use std::io::{Read, Write};

impl<M: Module> Save for FreeModuleHomomorphism<M> {
    fn save(&self, buffer: &mut impl Write) -> io::Result<()> {
        self.outputs.save(buffer)?;

        Ok(())
    }
}

impl<M: Module> Load for FreeModuleHomomorphism<M> {
    type AuxData = (Arc<FreeModule<M::Algebra>>, Arc<M>, i32);

    fn load(buffer: &mut impl Read, data: &Self::AuxData) -> io::Result<Self> {
        let source: Arc<FreeModule<M::Algebra>> = Arc::clone(&data.0);
        let target: Arc<M> = Arc::clone(&data.1);
        let degree_shift = data.2;
        let min_degree = std::cmp::max(source.min_degree(), target.min_degree() + degree_shift);
        let p = source.prime();

        let outputs: OnceBiVec<Vec<FpVector>> = Load::load(buffer, &(min_degree, p))?;
        let max_degree = outputs.max_degree();

        let kernel = OnceBiVec::new(min_degree);
        let quasi_inverse = OnceBiVec::new(min_degree);

        for _ in min_degree..=max_degree {
            kernel.push(Subspace::new(p, 0, 0));
            quasi_inverse.push(QuasiInverse {
                image: None,
                preimage: Matrix::new(p, 0, 0),
            });
        }

        Ok(Self {
            source,
            target,
            outputs,
            kernel,
            quasi_inverse,
            min_degree,
            lock: Mutex::new(()),
            degree_shift,
        })
    }
}
