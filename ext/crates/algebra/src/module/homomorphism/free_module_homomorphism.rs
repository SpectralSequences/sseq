use parking_lot::Mutex;
use std::sync::Arc;

use crate::algebra::Algebra;
use crate::module::free_module::OperationGeneratorPair;
use crate::module::homomorphism::ModuleHomomorphism;
use crate::module::{FreeModule, Module};
use fp::matrix::{MatrixSliceMut, QuasiInverse, Subspace};
use fp::vector::{FpVector, Slice, SliceMut};
use once::OnceBiVec;

pub struct FreeModuleHomomorphism<M: Module> {
    source: Arc<FreeModule<M::Algebra>>,
    target: Arc<M>,
    outputs: OnceBiVec<Vec<FpVector>>, // degree --> input_idx --> output
    pub images: OnceBiVec<Option<Subspace>>,
    pub kernels: OnceBiVec<Option<Subspace>>,
    pub quasi_inverses: OnceBiVec<Option<QuasiInverse>>,
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
        result: SliceMut,
        coeff: u32,
        input_degree: i32,
        input_index: usize,
    ) {
        assert!(input_degree >= self.source.min_degree());
        assert!(input_index < self.source.dimension(input_degree));
        let output_degree = input_degree - self.degree_shift;
        assert_eq!(
            self.target.dimension(output_degree),
            result.as_slice().dimension()
        );
        let OperationGeneratorPair {
            operation_degree,
            generator_degree,
            operation_index,
            generator_index,
        } = *self.source.index_to_op_gen(input_degree, input_index);

        if generator_degree >= self.min_degree() {
            let output_on_generator = self.output(generator_degree, generator_index);
            self.target.act(
                result,
                coeff,
                operation_degree,
                operation_index,
                generator_degree - self.degree_shift,
                output_on_generator.as_slice(),
            );
        }
    }

    fn quasi_inverse(&self, degree: i32) -> Option<&QuasiInverse> {
        self.quasi_inverses
            .get(degree)
            .map(Option::as_ref)
            .flatten()
    }

    fn kernel(&self, degree: i32) -> Option<&Subspace> {
        self.kernels.get(degree).map(Option::as_ref).flatten()
    }

    fn image(&self, degree: i32) -> Option<&Subspace> {
        self.images.get(degree).map(Option::as_ref).flatten()
    }

    fn compute_auxiliary_data_through_degree(&self, degree: i32) {
        let _lock = self.lock.lock();
        let kernel_len = self.kernels.len();
        for i in kernel_len..=degree {
            let (image, kernel, qi) = self.auxiliary_data(i);
            self.images.push_checked(Some(image), i);
            self.kernels.push_checked(Some(kernel), i);
            self.quasi_inverses.push_checked(Some(qi), i);
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
        let kernels = OnceBiVec::new(min_degree);
        let images = OnceBiVec::new(min_degree);
        let quasi_inverses = OnceBiVec::new(min_degree);
        Self {
            source,
            target,
            outputs,
            images,
            kernels,
            quasi_inverses,
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

    pub fn extend_by_zero(&self, degree: i32) {
        let _lock = self.lock.lock();

        // println!("    add_gens_from_matrix degree : {}, first_new_row : {}, new_generators : {}", degree, first_new_row, new_generators);
        // println!("    dimension : {} target name : {}", dimension, self.target.name());
        if degree < self.min_degree {
            return;
        }
        let next_degree = self.next_degree();
        if next_degree > degree {
            return;
        }

        let p = self.prime();
        for i in next_degree..=degree {
            let num_gens = self.source.number_of_gens_in_degree(i);
            let dimension = self.target.dimension(i - self.degree_shift);
            let mut new_outputs: Vec<FpVector> = Vec::with_capacity(num_gens);
            for _ in 0..num_gens {
                new_outputs.push(FpVector::new(p, dimension));
            }
            self.outputs.push_checked(new_outputs, i);
        }
    }

    pub fn add_generators_from_big_vector(&self, degree: i32, outputs_vectors: Slice) {
        let p = self.prime();
        let new_generators = self.source.number_of_gens_in_degree(degree);
        let target_dimension = self.target.dimension(degree - self.degree_shift);
        let mut new_outputs: Vec<FpVector> = Vec::with_capacity(new_generators);
        for _ in 0..new_generators {
            new_outputs.push(FpVector::new(p, target_dimension));
        }
        if target_dimension == 0 {
            self.outputs.push_checked(new_outputs, degree);
            return;
        }
        for (i, new_output) in new_outputs.iter_mut().enumerate() {
            new_output
                .as_slice_mut()
                .assign(outputs_vectors.slice(target_dimension * i, target_dimension * (i + 1)));
        }
        self.outputs.push_checked(new_outputs, degree);
    }

    /// A MatrixSlice will do but there is no applicaiton of this struct, so it doesn't exist
    /// yet...
    pub fn add_generators_from_matrix_rows(&self, degree: i32, mut matrix: MatrixSliceMut) {
        let p = self.prime();
        let new_generators = self.source.number_of_gens_in_degree(degree);
        let target_dimension = self.target.dimension(degree - self.degree_shift);

        let mut new_outputs: Vec<FpVector> = Vec::with_capacity(new_generators);
        for _ in 0..new_generators {
            new_outputs.push(FpVector::new(p, target_dimension));
        }
        if target_dimension == 0 {
            self.outputs.push_checked(new_outputs, degree);
            return;
        }
        for (i, new_output) in new_outputs.iter_mut().enumerate() {
            new_output.as_slice_mut().assign(matrix.row(i));
        }
        self.outputs.push_checked(new_outputs, degree);
    }

    pub fn add_generators_from_rows(&self, degree: i32, rows: Vec<FpVector>) {
        self.outputs.push_checked(rows, degree);
    }

    pub fn apply_to_generator(&self, result: &mut FpVector, coeff: u32, degree: i32, idx: usize) {
        let output_on_gen = self.output(degree, idx);
        result.add(output_on_gen, coeff);
    }

    pub fn set_image(&self, degree: i32, image: Option<Subspace>) {
        self.images.push_checked(image, degree);
    }

    pub fn set_kernel(&self, degree: i32, kernel: Option<Subspace>) {
        self.kernels.push_checked(kernel, degree);
    }

    pub fn set_quasi_inverse(&self, degree: i32, quasi_inverse: Option<QuasiInverse>) {
        self.quasi_inverses.push_checked(quasi_inverse, degree);
    }
}

impl<A: Algebra> FreeModuleHomomorphism<FreeModule<A>> {
    /// Given f: M -> N, compute the dual f*: Hom(N, k) -> Hom(M, k) in source (N) degree t.
    pub fn hom_k(&self, t: i32) -> Vec<Vec<u32>> {
        let source_dim = self.source.number_of_gens_in_degree(t + self.degree_shift);
        let target_dim = self.target.number_of_gens_in_degree(t);
        if target_dim == 0 {
            return vec![];
        }
        let mut result = vec![vec![0; source_dim]; target_dim];

        let offset = self.target.generator_offset(t, t, 0);
        for i in 0..source_dim {
            let output = self.output(t + self.degree_shift, i);
            #[allow(clippy::needless_range_loop)]
            for j in 0..target_dim {
                result[j][i] = output.entry(offset + j);
            }
        }
        result
    }
}

use saveload::{Load, Save};
use std::io;
use std::io::{Read, Write};

impl<M: Module> Save for FreeModuleHomomorphism<M> {
    fn save(&self, buffer: &mut impl Write) -> io::Result<()> {
        self.outputs.save(buffer)?;
        self.images.save(buffer)?;
        self.kernels.save(buffer)?;
        self.quasi_inverses.save(buffer)?;

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

        let _kernels = OnceBiVec::<Subspace>::load(buffer, &(min_degree, p))?;
        let images = OnceBiVec::new(min_degree);
        let kernels = OnceBiVec::new(min_degree);
        let quasi_inverses = OnceBiVec::new(min_degree);

        let len = usize::load(buffer, &())?;
        for _ in 0..len {
            images.push(None);
            kernels.push(None);
            quasi_inverses.push(Some(QuasiInverse::load(buffer, &p)?));
        }

        Ok(Self {
            source,
            target,
            outputs,
            images,
            kernels,
            quasi_inverses,
            min_degree,
            lock: Mutex::new(()),
            degree_shift,
        })
    }
}
