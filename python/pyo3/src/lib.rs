pub mod resolution;
pub mod resolution_homomorphism;

use pyo3::prelude::*;
use pyo3::{wrap_pymodule, wrap_pyfunction};
use pyo3::types::PyTuple;

use python_utils;
use python_fp::PyInit_python_fp;
use python_algebra::PyInit_python_algebra;

#[pyclass(dict)]
struct Test {}

#[pymethods]
impl Test {
    /// This function adds two unsigned 64-bit integers.
    #[args(a=1, pyargs="*")]
    // #[text_signature = "(a /)"]
    #[staticmethod]
    fn add(a: u64, b: u64, pyargs : &PyTuple) -> PyResult<u64> {
        python_utils::check_number_of_positional_arguments!("add", 2, 3, 2+pyargs.len())?;
        Ok(a + b)
    }
}

#[pymodule]
fn rust_algebra(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_wrapped(wrap_pymodule!(python_fp))?;
    python_utils::rename_submodule(m, "python_fp", "fp")?;
    m.add_wrapped(wrap_pymodule!(python_algebra))?;
    python_utils::rename_submodule(m, "python_algebra", "algebra")?;
    m.add_class::<resolution::Resolution>()?;
    m.add_class::<Test>()?;
    Ok(())
}