use std::sync::Arc;

use pyo3::prelude::*;

use ext::resolution::ResolutionInner as ResolutionRust;
use ext::chain_complex::{ChainComplex, FiniteChainComplex, FreeChainComplex};//, ChainMap};

use python_algebra::module::ModuleRust;
use python_algebra::module::{
    FDModule, 
    FreeModule,
    homomorphism::{
        FreeModuleHomomorphism,
        ModuleHomomorphismRust
    }
};

pub type CCRust = FiniteChainComplex<ModuleRust, ModuleHomomorphismRust>;
python_utils::rc_wrapper_type!(Resolution, ResolutionRust<CCRust>);

#[pymethods]
impl Resolution {
    #[new]
    pub fn new(module : &FDModule) -> PyResult<Self> {
        let chain_complex = Arc::new(FiniteChainComplex::ccdz(Arc::clone(module.to_arc()?)));
        Ok(Resolution::box_and_wrap(ResolutionRust::new(Arc::clone(&chain_complex))))
    }

    pub fn extend_through_degree(&self, next_s : u32, max_s : u32, next_t : i32, max_t : i32) -> PyResult<()> {
        self.inner()?.extend_through_degree(next_s, max_s, next_t, max_t);
        Ok(())
    }

    pub fn graded_dimension_string(&self, max_degree : i32 , max_hom_deg : u32) -> PyResult<String> {
        Ok(self.inner()?.graded_dimension_string(max_degree, max_hom_deg))
    } 

    pub fn step_resolution(&self, s : u32, t : i32) -> PyResult<()> {
        self.inner()?.step_resolution(s, t);
        Ok(())
    }

    pub fn cocycle_string(&self, hom_deg : u32, int_deg : i32, idx : usize) -> PyResult<String> {
        Ok(self.inner()?.cocycle_string(hom_deg, int_deg, idx))
    }

    // pub fn complex(&self) -> Arc<CC> {
    //     Arc::clone(&self.complex)
    // }

    pub fn number_of_gens_in_bidegree(&self, homological_degree : u32, internal_degree : i32) -> PyResult<usize> {
        Ok(self.inner()?.module(homological_degree).number_of_gens_in_degree(internal_degree))
    }

    pub fn prime(&self) -> PyResult<u32> {
        Ok(*self.inner()?.complex().prime())
    }

    pub fn module(&self, homological_degree : u32) -> PyResult<FreeModule> {
        Ok(FreeModule::wrap_immutable(self.inner()?.module(homological_degree)))
    }

    #[getter]
    pub fn get_min_degree(&self) -> PyResult<i32> {
        Ok(self.inner()?.min_degree())
    }

    pub fn differential(&self, s : u32) -> PyResult<FreeModuleHomomorphism> {
        Ok(FreeModuleHomomorphism::wrap_to_free(self.inner()?.differential(s)))
    }

}