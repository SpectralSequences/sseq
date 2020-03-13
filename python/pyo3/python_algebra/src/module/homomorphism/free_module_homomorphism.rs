#![allow(unused_imports)]
#![allow(dead_code)]

use pyo3::{
    prelude::*,
    exceptions,
    PyObjectProtocol,
    types::PyDict
};

use python_utils::{
    self,
    py_repr, 
    // rc_wrapper_type, 
    wrapper_type, 
    // immutable_wrapper_type,
    // get_from_kwargs
};

use std::sync::Arc;

use algebra::Algebra as AlgebraT;

use algebra::module::{
    Module as ModuleT, 
    FreeModule as FreeModuleRust, 
    homomorphism::{
        ModuleHomomorphism,
        FreeModuleHomomorphism as FreeModuleHomomorphismRust
    }
};

use python_fp::{
    vector::FpVector,
    matrix::{QuasiInverse, Subspace}
};
use crate::algebra_rust::AlgebraRust;
use crate::module::module_rust::ModuleRust;
use crate::module::FreeModule;

pub enum FreeModuleHomomorphismInner {
    ToFree(Arc<FreeModuleHomomorphismRust<FreeModuleRust<AlgebraRust>>>),
    ToOther(Arc<FreeModuleHomomorphismRust<ModuleRust>>)
}



wrapper_type!(FreeModuleHomomorphism, FreeModuleHomomorphismInner);



macro_rules! fmh_dispatch {
    ($method : ident, $self_ : expr, $( $args : expr ),*) => {
        match $self_ {
            FreeModuleHomomorphismInner::ToFree(morphism) => morphism.$method($($args),*),
            FreeModuleHomomorphismInner::ToOther(morphism) => morphism.$method($($args),*),
        }
    };
}

#[pymethods]
impl FreeModuleHomomorphism {
    fn source(&self) -> PyResult<FreeModule> {
        Ok(FreeModule::wrap(fmh_dispatch!(source, self.inner()?, )))
    }

    fn target(&self) -> PyResult<PyObject> {
        let gil = Python::acquire_gil();
        let py = gil.python();
        match self.inner()? {
            FreeModuleHomomorphismInner::ToFree(morphism) => {
                Ok(FreeModule::wrap(morphism.target()).into_py(py))
            },
            FreeModuleHomomorphismInner::ToOther(morphism) => {
                Ok(ModuleRust::into_py_object(morphism.target()))
            }
        }
    }

    #[getter]
    fn get_degree_shift(&self) -> PyResult<i32> {
        Ok(fmh_dispatch!(degree_shift, self.inner()?, ))
    }

    fn apply_to_basis_element(
        &self,
        result: &mut FpVector,
        coeff: u32,
        input_degree: i32,
        input_index: usize,
    ) -> PyResult<()> {
        fmh_dispatch!(apply_to_basis_element, self.inner()?, result.inner_mut()?, coeff, input_degree, input_index);
        Ok(())
    }

    fn quasi_inverse(&self, degree: i32) -> PyResult<QuasiInverse> {
        Ok(QuasiInverse::wrap_immutable(fmh_dispatch!(quasi_inverse, self.inner()?, degree), self.owner()))
    }

    fn kernel(&self, degree: i32) -> PyResult<Subspace> {
        Ok(Subspace::wrap_immutable(fmh_dispatch!(kernel, self.inner()?, degree), self.owner()))
    }


}
