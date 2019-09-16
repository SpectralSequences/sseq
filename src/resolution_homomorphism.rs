use std::sync::Weak;

use once::OnceVec;
use crate::fp_vector::{ FpVector, FpVectorT };
use crate::matrix::Matrix;
use crate::module::{Module, FiniteModule};
use crate::module::homomorphism::{FreeModuleHomomorphism, ModuleHomomorphism};
use crate::chain_complex::{AugmentedChainComplex, ChainComplex};
use crate::chain_complex::ChainComplexConcentratedInDegreeZero as CCDZ;
use crate::resolution::ResolutionInner;

pub struct ResolutionHomomorphism<CC1 : ChainComplex, CC2 : AugmentedChainComplex> {
    name : String,
    source : Weak<ResolutionInner<CC1>>,
    target : Weak<CC2>,
    maps : OnceVec<FreeModuleHomomorphism<CC2::Module>>,
    homological_degree_shift : u32,
    internal_degree_shift : i32
}

impl<CC1 : ChainComplex, CC2 : AugmentedChainComplex> ResolutionHomomorphism<CC1, CC2> {
    pub fn new(
        name : String,
        source : Weak<ResolutionInner<CC1>>, target : Weak<CC2>,
        homological_degree_shift : u32, internal_degree_shift : i32
    ) -> Self {
        Self {
            name,
            source,
            target,
            maps : OnceVec::new(),
            homological_degree_shift,
            internal_degree_shift
        }
    }

    fn get_map_ensure_length(&self, output_homological_degree : u32) -> &FreeModuleHomomorphism<CC2::Module> {
        if output_homological_degree as usize >= self.maps.len() {
            let input_homological_degree = output_homological_degree + self.homological_degree_shift;
            self.maps.push(FreeModuleHomomorphism::new(self.source.upgrade().unwrap().module(input_homological_degree), self.target.upgrade().unwrap().module(output_homological_degree), self.internal_degree_shift));
        }
        return &self.maps[output_homological_degree as usize];
    }

    pub fn get_map(&self, output_homological_degree : u32) -> &FreeModuleHomomorphism<CC2::Module> {
        &self.maps[output_homological_degree as usize]
    }

    /// Extend the resolution homomorphism such that it is defined on degrees
    /// (`source_homological_degree`, `source_degree`).
    pub fn extend(&self, source_homological_degree : u32, source_degree : i32){
        for i in self.homological_degree_shift ..= source_homological_degree {
            let f_cur = self.get_map_ensure_length(i - self.homological_degree_shift);
            let start_degree = *f_cur.lock();
            for j in start_degree + 1 ..= source_degree {
                self.extend_step(i, j, None);
            }
        }
    }

    pub fn extend_step(&self, input_homological_degree : u32, input_internal_degree : i32, extra_images : Option<&Matrix>){
        let output_homological_degree = input_homological_degree - self.homological_degree_shift;
        let f_cur = self.get_map_ensure_length(output_homological_degree);
        let computed_degree = *f_cur.lock();
        if input_internal_degree <= computed_degree {
            assert!(extra_images.is_none());
            return;
        }
        let num_gens = f_cur.source().number_of_gens_in_degree(input_internal_degree);
        let mut outputs = self.extend_step_helper(input_homological_degree, input_internal_degree, extra_images);
        let mut lock = f_cur.lock();
        f_cur.add_generators_from_matrix_rows(&lock, input_internal_degree, &mut outputs, 0, 0);
        *lock += 1;
    }

    fn extend_step_helper(&self, input_homological_degree : u32, input_internal_degree : i32, mut extra_images : Option<&Matrix>) -> Matrix {
        let source = self.source.upgrade().unwrap();
        let target = self.target.upgrade().unwrap();
        let p = source.prime();
        assert!(input_homological_degree >= self.homological_degree_shift);
        let output_homological_degree = input_homological_degree - self.homological_degree_shift;
        let output_internal_degree = input_internal_degree - self.internal_degree_shift;        
        let target_chain_map = target.chain_map(output_homological_degree);
        let target_cc_dimension = target_chain_map.target().dimension(output_internal_degree);
        if let Some(extra_images_matrix) = &extra_images {
            assert!(target_cc_dimension == extra_images_matrix.columns());
        }
        let f_cur = self.get_map(output_homological_degree);
        let num_gens = f_cur.source().number_of_gens_in_degree(input_internal_degree);
        let fx_dimension = f_cur.target().dimension(output_internal_degree);
        let mut outputs_matrix = Matrix::new(p, num_gens, fx_dimension);
        if num_gens == 0 || fx_dimension == 0 {
            return outputs_matrix;
        }
        if output_homological_degree == 0 {
            if let Some(extra_images_matrix) = extra_images {
                target_chain_map.compute_kernels_and_quasi_inverses_through_degree(output_internal_degree);
                let target_chain_map_qi = target_chain_map.quasi_inverse(output_internal_degree);
                assert!(num_gens == extra_images_matrix.rows(),
                    format!("num_gens : {} greater than rows : {} hom_deg : {}, int_deg : {}", 
                    num_gens, extra_images_matrix.rows(), input_homological_degree, input_internal_degree));
                for k in 0 .. num_gens {
                    target_chain_map_qi.apply(&mut outputs_matrix[k], 1, &extra_images_matrix[k]);
                }
            }
            return outputs_matrix;            
        }
        let d_source = source.differential(input_homological_degree);
        let d_target = target.differential(output_homological_degree);
        let f_prev = self.get_map(output_homological_degree - 1);
        assert_eq!(d_source.source().name(), f_cur.source().name());
        assert_eq!(d_source.target().name(), f_prev.source().name());
        assert_eq!(d_target.source().name(), f_cur.target().name());
        assert_eq!(d_target.target().name(), f_prev.target().name());
        let dx_dimension = f_prev.source().dimension(input_internal_degree);
        let fdx_dimension = f_prev.target().dimension(output_internal_degree);
        let mut dx_vector = FpVector::new(p, dx_dimension);
        let mut fdx_vector = FpVector::new(p, fdx_dimension);
        let mut extra_image_row = 0;
        for k in 0 .. num_gens {
            d_source.apply_to_generator(&mut dx_vector, 1, input_internal_degree, k);
            if dx_vector.is_zero() {
                let extra_image_matrix = extra_images.as_mut().expect("Missing extra image rows");
                target_chain_map.compute_kernels_and_quasi_inverses_through_degree(output_internal_degree);
                let target_chain_map_qi = target_chain_map.quasi_inverse(output_internal_degree);
                target_chain_map_qi.apply(&mut outputs_matrix[k], 1, &extra_image_matrix[extra_image_row]);
                extra_image_row += 1;
            } else {
                d_target.compute_kernels_and_quasi_inverses_through_degree(output_internal_degree);
                let d_quasi_inverse = d_target.quasi_inverse(output_internal_degree);
                f_prev.apply(&mut fdx_vector, 1, input_internal_degree, &dx_vector);
                d_quasi_inverse.apply(&mut outputs_matrix[k], 1, &fdx_vector);
                dx_vector.set_to_zero();
                fdx_vector.set_to_zero();
            }
        }
        // let num_extra_image_rows = extra_images.map_or(0, |matrix| matrix.rows());
        // assert!(extra_image_row == num_extra_image_rows, "Extra image rows");
        return outputs_matrix;
    }

}

pub type ResolutionHomomorphismToUnit<CC> = ResolutionHomomorphism<CC, ResolutionInner<CCDZ<FiniteModule>>>;
