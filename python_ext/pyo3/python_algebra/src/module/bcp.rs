use pyo3::{
    prelude::*,
    PyObjectProtocol
};

use python_utils;

// use crate::module_methods;
use crate::algebra::AlgebraRust;
use crate::module::module_rust::ModuleRust;

use algebra::Algebra as AlgebraT;
use algebra::module::{BCp as BCpRust, Module};

crate::module_bindings!(BCp, BCpRust);

python_utils::py_repr!(BCp, "FreedBCp", {
    Ok(format!(
        "BCp()"
    ))
});

#[pymethods]
impl BCp {
    #[new]
    #[args(min_degree=0, pyargs="*")]
    fn new(algebra: PyObject) -> PyResult<Self> {
        let mut result = Self::box_and_wrap(
            BCpRust::new(AlgebraRust::from_py_object(algebra)?)
        );
        result.freeze()?;
        Ok(result)
    }
}