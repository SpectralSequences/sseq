use pyo3::{
    prelude::*,
    PyObjectProtocol,
    types::PyTuple
};

use python_utils;

// use crate::module_methods;
use crate::algebra::AlgebraRust;
use crate::module::module_rust::ModuleRust;

use algebra::Algebra as AlgebraT;
use algebra::module::{RealProjectiveSpace as RealProjectiveSpaceRust, Module};

crate::module_bindings!(RealProjectiveSpace, RealProjectiveSpaceRust);

python_utils::py_repr!(RealProjectiveSpace, "FreedRealProjectiveSpace", {
    Ok(format!(
        "RealProjectiveSpace(min={}, max=??)",
        inner.min
    ))
});

#[pymethods]
impl RealProjectiveSpace {
    #[new]
    #[args(min_degree=0, pyargs="*")]
    fn new(algebra: PyObject, min : i32, pyargs : &PyTuple) -> PyResult<Self> {
        if pyargs.len() > 0 {
            return Err(python_utils::exception!(NotImplementedError))
        }
        let mut result = Self::box_and_wrap(
            RealProjectiveSpaceRust::new(AlgebraRust::from_py_object(algebra)?, min, None, false)
        );
        result.freeze()?;
        Ok(result)
    }
}