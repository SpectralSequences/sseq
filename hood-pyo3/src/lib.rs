use pyo3::prelude::*;
use pyo3::{wrap_pymodule};

use python_utils;
use fp_python::PyInit_python_fp;
use algebra_python::PyInit_python_algebra;


#[pymodule]
fn rust_algebra(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_wrapped(wrap_pymodule!(python_fp))?;
    python_utils::rename_submodule(m, "python_fp", "fp")?;
    m.add_wrapped(wrap_pymodule!(python_algebra))?;
    python_utils::rename_submodule(m, "python_algebra", "algebra")?;
    Ok(())
}