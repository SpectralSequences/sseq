pub mod resolution;
pub mod resolution_homomorphism;

use pyo3::prelude::*;
use pyo3::{wrap_pymodule};

use python_utils;
use python_fp::PyInit_python_fp;
use python_algebra::PyInit_python_algebra;


#[pymodule]
fn rust_algebra(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_wrapped(wrap_pymodule!(python_fp))?;
    python_utils::rename_submodule(m, "python_fp", "fp")?;
    m.add_wrapped(wrap_pymodule!(python_algebra))?;
    python_utils::rename_submodule(m, "python_algebra", "algebra")?;
    m.add_class::<resolution::Resolution>()?;
    Ok(())
}