use std::sync::Arc;

use ext::{chain_complex::FreeChainComplex, secondary::SecondaryLift};
use pyo3::prelude::*;

mod algebra_mod;
mod fp_mod;
mod sseq_mod;

pub use algebra_mod::algebra_py;
pub use sseq_mod::sseq_py;

#[pyfunction]
fn query_module(algebra_type: Option<algebra_py::AlgebraType>, save: bool) -> PyResult<Resolution> {
    ext::utils::query_module(algebra_type.map(algebra::AlgebraType::from), save)
        .map(|res| Resolution(Arc::new(res)))
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
}

#[pyfunction]
fn query_module_only(
    prompt: &str,
    algebra: Option<algebra_py::AlgebraType>,
    load_quasi_inverse: bool,
) -> PyResult<Resolution> {
    ext::utils::query_module_only(
        prompt,
        algebra.map(algebra::AlgebraType::from),
        load_quasi_inverse,
    )
    .map(|res| Resolution(Arc::new(res)))
    .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
}

#[pyclass(frozen)]
#[derive(Clone)]
struct Resolution(Arc<ext::resolution::Resolution<ext::CCC>>);

#[pymethods]
impl Resolution {
    fn compute_through_stem(&self, max: sseq_py::Bidegree) {
        self.0.compute_through_stem(max.0)
    }

    fn graded_dimension_string(&self) -> String {
        self.0.graded_dimension_string()
    }
}

#[pyclass(frozen)]
struct SecondaryResolution(
    ext::secondary::SecondaryResolution<ext::resolution::Resolution<ext::CCC>>,
);

#[pymethods]
impl SecondaryResolution {
    #[new]
    fn new(cc: Resolution) -> Self {
        SecondaryResolution(ext::secondary::SecondaryResolution::new(cc.0))
    }

    fn extend_all(&self) {
        self.0.extend_all();
    }

    fn underlying(&self) -> Resolution {
        Resolution(Arc::clone(&self.0.underlying()))
    }
}

#[pymodule]
#[pyo3(name = "ext")]
fn ext_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    ext::utils::init_logging()
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

    let algebra_m = pyo3::wrap_pymodule!(algebra_py)(m.py());
    m.add_submodule(algebra_m.bind(m.py()))?;

    let fp_m = PyModule::new(m.py(), "fp_py")?;
    fp_mod::fp_py(&fp_m)?;
    m.add_submodule(&fp_m)?;

    let sseq_m = pyo3::wrap_pymodule!(sseq_py)(m.py());
    m.add_submodule(sseq_m.bind(m.py()))?;

    m.add_function(wrap_pyfunction!(query_module, m)?)?;
    m.add_function(wrap_pyfunction!(query_module_only, m)?)?;
    m.add_class::<Resolution>()?;
    m.add_class::<SecondaryResolution>()?;
    Ok(())
}
