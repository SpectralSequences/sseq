use std::sync::Arc;

use crate::chain_complex::{AugmentedChainComplex, FreeChainComplex};
use crate::resolution::Resolution;
use crate::CCC;
use algebra::module::homomorphism::{FreeModuleHomomorphism, ModuleHomomorphism};
use algebra::module::Module;
use algebra::SteenrodAlgebra;
use fp::matrix::Matrix;
use fp::vector::{FpVector, SliceMut};
use once::OnceVec;

pub struct ResolutionHomomorphism<CC1, CC2>
where
    CC1: FreeChainComplex<Algebra = <CC2::Module as Module>::Algebra>,
    CC2: AugmentedChainComplex,
{
    #[allow(dead_code)]
    name: String,
    pub source: Arc<CC1>,
    pub target: Arc<CC2>,
    maps: OnceVec<FreeModuleHomomorphism<CC2::Module>>,
    pub homological_degree_shift: u32,
    pub internal_degree_shift: i32,
}

impl<CC1, CC2> ResolutionHomomorphism<CC1, CC2>
where
    CC1: FreeChainComplex<Algebra = <CC2::Module as Module>::Algebra>,
    CC2: AugmentedChainComplex,
{
    pub fn new(
        name: String,
        source: Arc<CC1>,
        target: Arc<CC2>,
        homological_degree_shift: u32,
        internal_degree_shift: i32,
    ) -> Self {
        Self {
            name,
            source,
            target,
            maps: OnceVec::new(),
            homological_degree_shift,
            internal_degree_shift,
        }
    }

    fn get_map_ensure_length(
        &self,
        output_homological_degree: u32,
    ) -> &FreeModuleHomomorphism<CC2::Module> {
        if output_homological_degree as usize >= self.maps.len() {
            let input_homological_degree =
                output_homological_degree + self.homological_degree_shift;
            self.maps.push(FreeModuleHomomorphism::new(
                self.source.module(input_homological_degree),
                self.target.module(output_homological_degree),
                self.internal_degree_shift,
            ));
        }
        &self.maps[output_homological_degree as usize]
    }

    pub fn get_map(&self, output_homological_degree: u32) -> &FreeModuleHomomorphism<CC2::Module> {
        &self.maps[output_homological_degree as usize]
    }

    pub fn into_chain_maps(self) -> Vec<FreeModuleHomomorphism<CC2::Module>> {
        self.maps.into_vec()
    }

    /// Extend the resolution homomorphism such that it is defined on degrees
    /// (`source_homological_degree`, `source_degree`).
    pub fn extend(&self, source_homological_degree: u32, source_degree: i32) {
        self.source
            .compute_through_bidegree(source_homological_degree, source_degree);
        self.target.compute_through_bidegree(
            source_homological_degree - self.homological_degree_shift,
            source_degree - self.internal_degree_shift,
        );
        for i in self.homological_degree_shift..=source_homological_degree {
            let f_cur = self.get_map_ensure_length(i - self.homological_degree_shift);
            for j in f_cur.next_degree()..=source_degree {
                self.extend_step(i, j, None);
            }
        }
    }

    pub fn extend_step(
        &self,
        input_homological_degree: u32,
        input_internal_degree: i32,
        extra_images: Option<&Matrix>,
    ) {
        let output_homological_degree = input_homological_degree - self.homological_degree_shift;
        let output_internal_degree = input_internal_degree - self.internal_degree_shift;
        self.target
            .compute_through_bidegree(output_homological_degree, output_internal_degree);

        let f_cur = self.get_map_ensure_length(output_homological_degree);
        if input_internal_degree < f_cur.next_degree() {
            assert!(extra_images.is_none());
            return;
        }
        let mut outputs = self.extend_step_helper(
            input_homological_degree,
            input_internal_degree,
            extra_images,
        );
        f_cur.add_generators_from_matrix_rows(input_internal_degree, outputs.as_slice_mut());
    }

    fn extend_step_helper(
        &self,
        input_homological_degree: u32,
        input_internal_degree: i32,
        mut extra_images: Option<&Matrix>,
    ) -> Matrix {
        let p = self.source.prime();
        assert!(input_homological_degree >= self.homological_degree_shift);
        let output_homological_degree = input_homological_degree - self.homological_degree_shift;
        let output_internal_degree = input_internal_degree - self.internal_degree_shift;
        let f_cur = self.get_map(output_homological_degree);
        let num_gens = f_cur
            .source()
            .number_of_gens_in_degree(input_internal_degree);
        let fx_dimension = f_cur.target().dimension(output_internal_degree);
        let mut outputs_matrix = Matrix::new(p, num_gens, fx_dimension);
        if num_gens == 0 || fx_dimension == 0 {
            return outputs_matrix;
        }
        if output_homological_degree == 0 {
            if let Some(extra_images_matrix) = extra_images {
                let target_chain_map = self.target.chain_map(output_homological_degree);
                let target_cc_dimension =
                    target_chain_map.target().dimension(output_internal_degree);
                assert!(target_cc_dimension == extra_images_matrix.columns());

                target_chain_map.compute_auxiliary_data_through_degree(output_internal_degree);
                assert!(
                    num_gens == extra_images_matrix.rows(),
                    "num_gens : {} greater than rows : {} hom_deg : {}, int_deg : {}",
                    num_gens,
                    extra_images_matrix.rows(),
                    input_homological_degree,
                    input_internal_degree
                );
                for k in 0..num_gens {
                    target_chain_map.apply_quasi_inverse(
                        outputs_matrix[k].as_slice_mut(),
                        output_internal_degree,
                        extra_images_matrix[k].as_slice(),
                    );
                }
            }
            return outputs_matrix;
        }
        let d_source = self.source.differential(input_homological_degree);
        let d_target = self.target.differential(output_homological_degree);
        let f_prev = self.get_map(output_homological_degree - 1);
        assert!(Arc::ptr_eq(&d_source.source(), &f_cur.source()));
        assert!(Arc::ptr_eq(&d_source.target(), &f_prev.source()));
        assert!(Arc::ptr_eq(&d_target.source(), &f_cur.target()));
        assert!(Arc::ptr_eq(&d_target.target(), &f_prev.target()));
        let fdx_dimension = f_prev.target().dimension(output_internal_degree);
        let mut fdx_vector = FpVector::new(p, fdx_dimension);
        let mut extra_image_row = 0;
        for k in 0..num_gens {
            let dx_vector = d_source.output(input_internal_degree, k);
            if dx_vector.is_zero() {
                let target_chain_map = self.target.chain_map(output_homological_degree);
                let target_cc_dimension =
                    target_chain_map.target().dimension(output_internal_degree);
                if let Some(extra_images_matrix) = &extra_images {
                    assert!(target_cc_dimension == extra_images_matrix.columns());
                }

                let extra_image_matrix = extra_images.as_mut().expect("Missing extra image rows");
                target_chain_map.compute_auxiliary_data_through_degree(output_internal_degree);
                target_chain_map.apply_quasi_inverse(
                    outputs_matrix[k].as_slice_mut(),
                    output_internal_degree,
                    extra_image_matrix[extra_image_row].as_slice(),
                );
                extra_image_row += 1;
            } else {
                d_target.compute_auxiliary_data_through_degree(output_internal_degree);
                f_prev.apply(
                    fdx_vector.as_slice_mut(),
                    1,
                    input_internal_degree,
                    dx_vector.as_slice(),
                );
                d_target.apply_quasi_inverse(
                    outputs_matrix[k].as_slice_mut(),
                    output_internal_degree,
                    fdx_vector.as_slice(),
                );
                fdx_vector.set_to_zero();
            }
        }
        // let num_extra_image_rows = extra_images.map_or(0, |matrix| matrix.rows());
        // assert!(extra_image_row == num_extra_image_rows, "Extra image rows");
        outputs_matrix
    }
}

use crate::chain_complex::{BoundedChainComplex, ChainComplex};
use algebra::module::homomorphism::FiniteModuleHomomorphism;
use algebra::module::{BoundedModule, FiniteModule};

impl<M, ACC, TCC> ResolutionHomomorphism<Resolution<CCC>, ACC>
where
    M: Module<Algebra = SteenrodAlgebra>,
    ACC: AugmentedChainComplex<Algebra = SteenrodAlgebra, TargetComplex = TCC>,
    TCC: BoundedChainComplex<Algebra = SteenrodAlgebra, Module = M>,
{
    pub fn from_module_homomorphism(
        name: String,
        source: Arc<Resolution<CCC>>,
        target: Arc<ACC>,
        f: &FiniteModuleHomomorphism<M>,
    ) -> Self {
        assert_eq!(source.target().max_s(), 1);
        assert_eq!(target.target().max_s(), 1);

        let source_module = source.target().module(0);
        let target_module = target.target().module(0);
        assert!(Arc::ptr_eq(&source_module, &f.source()));
        assert!(Arc::ptr_eq(&target_module, &f.target()));

        let p = source.prime();
        let degree_shift = f.degree_shift();

        let max_degree = match &*source_module {
            FiniteModule::FDModule(m) => m.max_degree(),
            FiniteModule::FPModule(m) => m.generators.get_max_generator_degree(),
            FiniteModule::RealProjectiveSpace(_) => panic!("Real Projective Space not supported"),
        };

        let hom = Self::new(name, source, target, 0, degree_shift);

        source_module.compute_basis(max_degree);
        target_module.compute_basis(degree_shift + max_degree);

        // These are just asserts.
        hom.source.compute_through_bidegree(0, max_degree);
        hom.target
            .compute_through_bidegree(0, degree_shift + max_degree);

        let source_chain_map = hom.source.chain_map(0);
        let target_chain_map = hom.target.chain_map(0);
        target_chain_map.compute_auxiliary_data_through_degree(degree_shift + max_degree);

        let g = hom.get_map_ensure_length(0);

        for t in source_module.min_degree()..=max_degree {
            let num_gens = hom.source.module(0).number_of_gens_in_degree(t);

            let mut fx = FpVector::new(p, target_module.dimension(t + degree_shift));

            let mut outputs_matrix = Matrix::new(
                p,
                num_gens,
                hom.target.module(0).dimension(t + degree_shift),
            );
            if num_gens == 0 || fx.dimension() == 0 {
                g.add_generators_from_matrix_rows(t, outputs_matrix.as_slice_mut());
                continue;
            }
            for j in 0..num_gens {
                f.apply(
                    fx.as_slice_mut(),
                    1,
                    t,
                    source_chain_map.output(t, j).as_slice(),
                );
                target_chain_map.apply_quasi_inverse(
                    outputs_matrix[j].as_slice_mut(),
                    t + degree_shift,
                    fx.as_slice(),
                );
                fx.set_to_zero();
            }
            g.add_generators_from_matrix_rows(t, outputs_matrix.as_slice_mut());
        }
        hom
    }
}

impl<CC1, CC2> ResolutionHomomorphism<CC1, CC2>
where
    CC1: FreeChainComplex,
    CC2: AugmentedChainComplex + FreeChainComplex<Algebra = CC1::Algebra>,
{
    pub fn act(&self, mut result: SliceMut, s: u32, t: i32, idx: usize) {
        let source_s = s - self.homological_degree_shift;
        let source_t = t - self.internal_degree_shift;

        assert_eq!(
            result.as_slice().dimension(),
            self.source
                .module(source_s)
                .number_of_gens_in_degree(source_t)
        );

        let target_module = self.target.module(s);

        let map = self.get_map(s);
        for i in 0..result.as_slice().dimension() {
            let j = target_module.operation_generator_to_index(0, 0, t, idx);
            result.add_basis_element(i, map.output(t, i).entry(j));
        }
    }
}
