use pyo3::{
    prelude::*,
    PyObjectProtocol
};

use python_utils;

// use crate::module_methods;
use crate::algebra::AlgebraRust;
use crate::module::module_rust::ModuleRust;

use algebra::Algebra as AlgebraT;
use algebra::module::{KFpn as KFpnRust, Module};

crate::module_bindings!(KFpn, KFpnRust);

python_utils::py_repr!(KFpn, "FreedKFpn", {
    Ok(format!(
        "KF{}{}", *inner.algebra().prime(), inner.n
    ))
});

#[pymethods]
impl KFpn {
    #[new]
    #[args(min_degree=0, pyargs="*")]
    fn new(algebra: PyObject, n : i32) -> PyResult<Self> {
        let mut result = Self::box_and_wrap(
            KFpnRust::new(AlgebraRust::from_py_object(algebra)?, n)
        );
        result.freeze()?;
        Ok(result)
    }
}