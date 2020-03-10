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
    // wrapper_type, 
    // immutable_wrapper_type,
    // get_from_kwargs
};

use crate::algebra::AlgebraRust;
use algebra::module::FreeModule as FreeModuleRust;

rc_wrapper_type!(FreeModule, FreeModuleRust<AlgebraRust>);