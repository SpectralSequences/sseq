// use parking_lot::{Mutex, MutexGuard};

use pyo3::{
    prelude::*,
    exceptions,
    PyObjectProtocol,
    types::PyDict
};

use python_utils::{
    self,
    py_repr, 
    rc_wrapper_type, 
    wrapper_type, 
    // immutable_wrapper_type,
    // get_from_kwargs
};

use algebra::module::{
    Module, 
    FreeModule as FreeModuleRust, 
    OperationGeneratorPair as OperationGeneratorPairRust,
    FreeModuleTableEntry as FreeModuleTableEntryRust
};

use python_fp::vector::FpVector;
use crate::algebra::{AlgebraRust, algebra_from_py_object, algebra_into_py_object};
use crate::module_methods;

// wrapper_type!(FreeModuleLock, MutexGuard<()>); // causes Lifetime specifier problem
wrapper_type!(FreeModuleTableEntry, FreeModuleTableEntryRust);

rc_wrapper_type!(FreeModule, FreeModuleRust<AlgebraRust>);

module_methods!(FreeModule);

#[pymethods]
impl FreeModule {
    #[new]
    pub fn new(algebra: PyObject, name: String, min_degree: i32) -> PyResult<Self> {
        Ok(Self::box_and_wrap(FreeModuleRust::new(algebra_from_py_object(algebra)?, name, min_degree)))
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

    pub fn construct_table(&self, degree: i32) -> PyResult<FreeModuleTableEntry> {
        Ok(FreeModuleTableEntry::box_and_wrap(self.inner()?.construct_table(degree)))
    }


    pub fn add_generators(
        &self,
        degree: i32,
        table: &FreeModuleTableEntry,
        num_gens: usize,
        names: Option<Vec<String>>,
    ) -> PyResult<()> {
        let inner = self.inner()?;
        let lock = inner.lock();
        let table_inner = table.inner()?.clone();
        inner.add_generators(degree, &lock, table_inner, num_gens, names);
        Ok(())
    }

    pub fn generator_offset(&self, degree: i32, gen_deg: i32, gen_idx: usize) -> PyResult<usize> {
        self.inner()?.generator_offset(degree, gen_deg, gen_idx)
    }

    pub fn operation_generator_to_index(
        &self,
        op_deg: i32,
        op_idx: usize,
        gen_deg: i32,
        gen_idx: usize,
    ) -> PyResult<usize> {
        self.inner()?.operation_generator_to_index(op_deg, op_idx, gen_deg, gen_idx)
    }

    pub fn operation_generator_pair_to_idx(&self, op_gen: &OperationGeneratorPair) -> usize {
        self.inner()?.operation_generator_pair_to_idx(op_gen.inner()?);
    }
    
}