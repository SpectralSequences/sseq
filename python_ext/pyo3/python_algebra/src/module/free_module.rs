use pyo3::{
    prelude::*,
    PyObjectProtocol,
};

use python_utils::{
    self,
    py_repr, 
    rc_wrapper_type, 
    wrapper_type, 
};

use algebra::AdemAlgebraT;


use algebra::module::{
    Module, 
    FreeUnstableModule as FreeUnstableModuleRust,
    OperationGeneratorPair as OperationGeneratorPairRust,
};

use python_fp::vector::FpVector;
use crate::algebra::AlgebraRust;

// wrapper_type!(FreeModuleLock, MutexGuard<()>); // causes Lifetime specifier problem
wrapper_type!(OperationGeneratorPair, OperationGeneratorPairRust);

rc_wrapper_type!(FreeUnstableModule, FreeUnstableModuleRust<AlgebraRust>);

py_repr!(FreeUnstableModule, "FreedFreeUnstableModule", {
    Ok(format!(
        "FreeUnstableModule(p={})",
        inner.prime()
    ))
});

crate::module_methods!(FreeUnstableModule);

#[pymethods]
impl FreeUnstableModule {
    #[new]
    pub fn new(algebra: PyObject, name: String, min_degree: i32) -> PyResult<Self> {
        Ok(Self::box_and_wrap(FreeUnstableModuleRust::new(AlgebraRust::from_py_object(algebra)?, name, min_degree)))
    }

    // pub fn lock(&self) -> FreeModuleLock {
    //     FreeModuleLock::box_and_wrap(self.lock.lock())
    // }

    pub fn max_computed_degree(&self) -> PyResult<i32> {
        Ok(self.inner()?.max_computed_degree())
    }

    pub fn number_of_gens_in_degree(&self, degree: i32) -> PyResult<usize> {
        Ok(self.inner()?.number_of_gens_in_degree(degree))
    }

    // pub fn construct_table(&self, degree: i32) -> PyResult<FreeModuleTableEntry> {
    //     Ok(FreeModuleTableEntry::box_and_wrap(self.inner()?.construct_table(degree)))
    // }


    // pub fn add_generators(
    //     &self,
    //     degree: i32,
    //     table: &FreeModuleTableEntry,
    //     num_gens: usize,
    //     names: Option<Vec<String>>,
    // ) -> PyResult<()> {
    //     let inner = self.inner()?;
    //     let lock = inner.lock();
    //     let table_inner = table.inner()?.clone();
    //     inner.add_generators(degree, &lock, table_inner, num_gens, names);
    //     Ok(())
    // }

    pub fn generator_offset(&self, degree: i32, gen_deg: i32, gen_idx: usize) -> PyResult<usize> {
        Ok(self.inner()?.generator_offset(degree, gen_deg, gen_idx))
    }

    pub fn operation_generator_to_index(
        &self,
        op_deg: i32,
        op_idx: usize,
        gen_deg: i32,
        gen_idx: usize,
    ) -> PyResult<usize> {
        Ok(self.inner()?.operation_generator_to_index(op_deg, op_idx, gen_deg, gen_idx))
    }

    pub fn operation_generator_pair_to_idx(&self, op_gen: &OperationGeneratorPair) -> PyResult<usize> {
        Ok(self.inner()?.operation_generator_pair_to_idx(op_gen.inner()?))
    }

    pub fn index_to_op_gen(&self, degree: i32, index: usize) -> PyResult<OperationGeneratorPair> {
        Ok(OperationGeneratorPair::wrap_immutable(self.inner()?.index_to_op_gen(degree, index), self.owner()))
    }

    pub fn element_to_json(&self, degree: i32, elt: &FpVector) -> PyResult<String> {
        Ok(self.inner()?.element_to_json(degree, elt.inner()?).to_string())
    }

    pub fn add_generators_immediate(
        &self,
        degree: i32,
        num_gens: usize,
        gen_names: Option<Vec<String>>,
    ) -> PyResult<()> {
        self.inner()?.add_generators_immediate(degree, num_gens, gen_names);
        Ok(())
    }
    
}