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

use std::sync::{Arc, Weak};

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
    matrix::{Matrix, QuasiInverse, Subspace}
};
use crate::algebra::AlgebraRust;
use crate::module::module_rust::ModuleRust;
use crate::module::FreeModule;

#[pyclass(dict)]
#[repr(transparent)]
pub struct FreeModuleHomomorphism {
    inner : FreeModuleHomomorphismInner
}

pub enum FreeModuleHomomorphismInner {
    ToFree(Arc<FreeModuleHomomorphismRust<FreeModuleRust<AlgebraRust>>>),
    ToOther(Arc<FreeModuleHomomorphismRust<ModuleRust>>),
    Null
}



macro_rules! fmh_dispatch {
    ($method : ident, $self_ : expr, $( $args : expr ),*) => {
        match &$self_.inner {
            FreeModuleHomomorphismInner::ToFree(morphism) => Ok(morphism.$method($($args),*)),
            FreeModuleHomomorphismInner::ToOther(morphism) => Ok(morphism.$method($($args),*)),
            FreeModuleHomomorphismInner::Null => Err(python_utils::exception!(ReferenceError, "Null..."))
        }
    };
}

impl FreeModuleHomomorphism {
    fn owner(&self) -> PyResult<Weak<()>> {
        match &self.inner {
            FreeModuleHomomorphismInner::ToFree(arc) => {
                Ok(python_utils::arc_to_final(arc))
            },
            FreeModuleHomomorphismInner::ToOther(arc) => {
                Ok(python_utils::arc_to_final(arc))
            },
            FreeModuleHomomorphismInner::Null => {
                Err(python_utils::exception!(ReferenceError, "Null..."))
            }
        }
    }

    pub fn wrap_to_free(morphism_to_free : Arc<FreeModuleHomomorphismRust<FreeModuleRust<AlgebraRust>>>) -> FreeModuleHomomorphism {
        Self {
            inner : FreeModuleHomomorphismInner::ToFree(morphism_to_free)
        }
    }

    pub fn wrap_to_other(morphism_to_other : Arc<FreeModuleHomomorphismRust<ModuleRust>>) -> FreeModuleHomomorphism {
        Self {
            inner : FreeModuleHomomorphismInner::ToOther(morphism_to_other)
        }
    }    
}


#[pymethods]
impl FreeModuleHomomorphism {

    fn free(&mut self) {
        self.inner = FreeModuleHomomorphismInner::Null;
    }

    fn source(&self) -> PyResult<FreeModule> {
        Ok(FreeModule::wrap_immutable(fmh_dispatch!(source, self, )?))
    }

    fn target(&self) -> PyResult<PyObject> {
        let gil = Python::acquire_gil();
        let py = gil.python();
        match &self.inner {
            FreeModuleHomomorphismInner::ToFree(morphism) => {
                Ok(FreeModule::wrap_immutable(morphism.target()).into_py(py))
            },
            FreeModuleHomomorphismInner::ToOther(morphism) => {
                Ok(ModuleRust::into_py_object(morphism.target()))
            },
            FreeModuleHomomorphismInner::Null => {
                Err(python_utils::exception!(ReferenceError, "Null..."))
            }
        }
    }

    #[getter]
    fn get_degree_shift(&self) -> PyResult<i32> {
        Ok(fmh_dispatch!(degree_shift, self, )?)
    }

    #[getter]
    pub fn get_min_degree(&self) -> PyResult<i32> {
        Ok(fmh_dispatch!(min_degree, self, )?)
    }

    #[getter]
    pub fn get_next_degree(&self) -> PyResult<i32> {
        Ok(fmh_dispatch!(next_degree, self, )?)
    }

    pub fn output(&self, generator_degree: i32, generator_index: usize) -> PyResult<FpVector> {
        Ok(FpVector::wrap_immutable(fmh_dispatch!(output, self, generator_degree, generator_index)?, self.owner()?))
    }

    pub fn extend_by_zero(&self, degree: i32) -> PyResult<()> {
        fmh_dispatch!(extend_by_zero, self, degree)?;
        Ok(())
    }

    fn apply_to_basis_element(
        &self,
        result: &mut FpVector,
        coeff: u32,
        input_degree: i32,
        input_index: usize,
    ) -> PyResult<()> {
        fmh_dispatch!(apply_to_basis_element, self, result.inner_mut()?.as_slice_mut(), coeff, input_degree, input_index)?;
        Ok(())
    }

    pub fn apply_to_generator(&self, result: &mut FpVector, coeff: u32, degree: i32, idx: usize) -> PyResult<()> {
        fmh_dispatch!(apply_to_generator, self, result.inner_mut()?, coeff, degree, idx)?;
        Ok(())
    }

    fn quasi_inverse(&self, degree: i32) -> PyResult<Option<QuasiInverse>> {
        let owner = self.owner()?;
        Ok(fmh_dispatch!(quasi_inverse, self, degree)?.map(|x| QuasiInverse::wrap_immutable(x, owner)))
    }

    fn kernel(&self, degree: i32) -> PyResult<Option<Subspace>> {
        let owner = self.owner()?;
        Ok(fmh_dispatch!(kernel, self, degree)?.map(|x| Subspace::wrap_immutable(x, owner)))
    }

    pub fn get_matrix(&self, matrix: &mut Matrix, degree: i32) -> PyResult<()> {
        fmh_dispatch!(get_matrix, self, matrix.inner_mut()?.as_slice_mut(), degree)?;
        Ok(())
    }

    // pub fn set_kernel(&self, degree: i32, kernel: Subspace) {
    //     let inner = self.inner()?;

    // }



}
