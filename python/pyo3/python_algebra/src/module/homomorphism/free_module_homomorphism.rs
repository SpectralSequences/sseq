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

use crate::algebra_rust::AlgebraRust;
use crate::module::module_rust::ModuleRust;
use crate::module::FreeModule;

pub enum FreeModuleHomomorphismInner {
    ToFree(Arc<FreeModuleHomomorphismRust<FreeModuleRust<AlgebraRust>>>),
    ToOther(Arc<FreeModuleHomomorphismRust<ModuleRust>>)
}



wrapper_type!(FreeModuleHomomorphism, FreeModuleHomomorphismInner);



macro_rules! fmh_dispatch {
    ($method : ident, $self_ : expr $(, $args : ident )*) => {
        match $self_ {
            FreeModuleHomomorphismInner::ToFree(morphism) => morphism.$method($($args),*),
            FreeModuleHomomorphismInner::ToOther(morphism) => morphism.$method($($args),*),
        }
    };
}

#[pymethods]
impl FreeModuleHomomorphism {
    fn source(&self) -> PyResult<FreeModule> {
        Ok(FreeModule::wrap(fmh_dispatch!(source, self.inner()?)))
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

    
}
