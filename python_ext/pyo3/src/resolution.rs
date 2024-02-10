use std::sync::Arc;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};


use pyo3::prelude::*;
use ext::resolution::Resolution as ResolutionRust;
use ext::chain_complex::{AugmentedChainComplex, ChainComplex, FiniteChainComplex, FreeChainComplex};//, ChainMap};

use python_algebra::module::{
    ModuleRust,
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
    pub fn new(module : PyObject) -> PyResult<Self> {
        let chain_complex = Arc::new(
            FiniteChainComplex::ccdz(
                ModuleRust::from_py_object(module)?
            )
        );
        Ok(Resolution::box_and_wrap(ResolutionRust::new(Arc::clone(&chain_complex))))
    }


    // pub fn extend_through_degree(&self, max_s : u32, max_t : i32) -> PyResult<()> {
    //     let (old_max_s, old_max_t) = self.extended_degree()?;
    //     self.inner()?.extend_through_degree(old_max_s, max_s, old_max_t, max_t);
    //     Ok(())
    // }

    pub fn graded_dimension_string(&self) -> PyResult<String> {
        Ok(self.inner()?.graded_dimension_string())
    }

    pub fn check_has_computed_bidegree(&self, hom_deg : u32, int_deg : i32) -> PyResult<()> {
        if !self.inner()?.has_computed_bidegree(hom_deg, int_deg) {
            Err(python_utils::exception!(IndexError,
                "We haven't computed out to bidegree {} yet.",
                python_utils::bidegree_string(hom_deg, int_deg)
            ))
        } else {
            Ok(())
        }
    }

    pub fn check_cocycle_idx(&self, hom_deg : u32, int_deg : i32, idx : usize) -> PyResult<()> {
        self.check_has_computed_bidegree(hom_deg, int_deg)?;
        if idx >= self.inner()?.number_of_gens_in_bidegree(hom_deg, int_deg) {
            Err(python_utils::exception!(IndexError,
                "Fewer than {} generators in bidegree {}.",
                idx + 1,
                python_utils::bidegree_string(hom_deg, int_deg)
            ))
        } else {
            Ok(())
        }
    }

    pub fn cocycle_string(&self, hom_deg : u32, int_deg : i32, idx : usize) -> PyResult<String> {
        self.check_cocycle_idx(hom_deg, int_deg, idx)?;
        Ok(self.inner()?.cocycle_string(hom_deg, int_deg, idx))
    }

    pub fn bidegree_hash(&self, hom_deg : u32, int_deg : i32) -> PyResult<u64> {
        self.check_has_computed_bidegree(hom_deg, int_deg)?;
        let self_inner = self.inner()?;
        let num_gens = self_inner.number_of_gens_in_bidegree(hom_deg, int_deg);
        let mut hasher = DefaultHasher::new();
        hom_deg.hash(&mut hasher);
        int_deg.hash(&mut hasher);
        num_gens.hash(&mut hasher);
        let ds = self_inner.differential(hom_deg);
        for idx in 0 .. num_gens {
            ds.output(int_deg, idx).hash(&mut hasher);
        }
        Ok(hasher.finish())
    }

    // pub fn complex(&self) -> Arc<CC> {
    //     Arc::clone(&self.complex)
    // }

    pub fn number_of_gens_in_bidegree(&self, homological_degree : u32, internal_degree : i32) -> PyResult<usize> {
        self.check_has_computed_bidegree(homological_degree, internal_degree)?;
        Ok(self.inner()?.module(homological_degree).number_of_gens_in_degree(internal_degree))
    }

    // pub fn prime(&self) -> PyResult<u32> {
    //     Ok(*self.inner()?.complex.prime())
    // }

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

    pub fn chain_map(&self, s : u32) -> PyResult<FreeModuleHomomorphism> {
        Ok(FreeModuleHomomorphism::wrap_to_other(self.inner()?.chain_map(s)))
    }

}


