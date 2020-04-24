use pyo3::{
    prelude::*,
    PyObjectProtocol
};

use python_utils;

// use crate::module_methods;
use crate::algebra::AlgebraRust;
use crate::module::module_rust::ModuleRust;

use algebra::{Algebra as AlgebraT, AdemAlgebraT};
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
        let algebra_rust = AlgebraRust::from_py_object(algebra)?;
        if !algebra_rust.adem_algebra().unstable_enabled {
            return Err(python_utils::exception!(ValueError, "AdemAlgebra must have unstable_enabled to be used as the algebra for KFpn."));
        }
        let mut result = Self::box_and_wrap(
            KFpnRust::new(algebra_rust, n)
        );
        result.freeze()?;
        Ok(result)
    }
}