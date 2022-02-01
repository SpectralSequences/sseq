// pub mod resolution;
// pub mod resolution_homomorphism;

use pyo3::prelude::*;
use pyo3::{wrap_pymodule};//, wrap_pyfunction};
// use pyo3::types::PyTuple;

use python_utils;
use python_fp::PyInit_python_fp;
use python_algebra::PyInit_algebra;
use python_algebra::PyInit_module;


#[pymodule]
fn rust_ext(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_wrapped(wrap_pymodule!(python_fp))?;
    python_utils::rename_submodule(m, "python_fp", "fp")?;
    m.add_wrapped(wrap_pymodule!(algebra))?;
    m.add_wrapped(wrap_pymodule!(module))?;
    python_utils::rename_submodule(m, "python_algebra", "algebra")?;
    // m.add_class::<resolution::Resolution>()?;
    // m.add_class::<resolution_homomorphism::ResolutionHomomorphism>()?;
    Ok(())
}