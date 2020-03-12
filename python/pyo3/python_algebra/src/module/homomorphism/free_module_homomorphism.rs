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
    immutable_wrapper_type,
    // get_from_kwargs
};


use algebra::Algebra as AlgebraT;

use algebra::module::{
    Module as ModuleT, 
    FreeModule as FreeModuleRust, 
    FreeModuleHomomorphism as FreeModuleHomomorphismRust
};

rc_wrapper_type!(FreeModuleHomomorphism, FreeModuleHomomorphismRust<AlgebraRust>);