use pyo3::prelude::*;
use pyo3::{wrap_pymodule};

use python_utils;
use fp_python::PyInit_fp_python;
use algebra_python::PyInit_algebra_python;


#[pymodule]
fn rust_algebra(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_wrapped(wrap_pymodule!(fp_python))?;
    python_utils::rename_submodule(m, "fp_python", "fp_linear_algebra")?;
    m.add_wrapped(wrap_pymodule!(algebra_python))?;
    python_utils::rename_submodule(m, "algebra_python", "algebra")?;
    Ok(())
}