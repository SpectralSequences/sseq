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
    immutable_wrapper_type,
    get_from_kwargs
};

use algebra::module::FDModule as FDModuleRust;

use crate::algebra::AlgebraRust;

rc_wrapper_type!(FDModule, FDModuleRust<AlgebraRust>);

