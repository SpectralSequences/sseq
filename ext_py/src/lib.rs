use pyo3::prelude::*;

mod algebra_mod;
mod fp_mod;
mod sseq_mod;

pub use algebra_mod::algebra_py;
pub use fp_mod::fp_py;
pub use sseq_mod::sseq_py;

#[pymodule]
#[pyo3(name = "ext")]
mod ext_py {
    use std::sync::Arc;

    use ext::{chain_complex::FreeChainComplex, secondary::SecondaryLift};

    #[pymodule_export]
    use super::algebra_py;
    #[pymodule_export]
    use super::fp_py;
    #[pymodule_export]
    use super::sseq_py;
    use super::*;

    #[pyfunction]
    pub fn query_module(
        algebra_type: Option<algebra_py::AlgebraType>,
        save: bool,
    ) -> PyResult<Resolution> {
        ext::utils::query_module(algebra_type.map(algebra::AlgebraType::from), save)
            .map(|res| Resolution(Arc::new(res)))
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    #[pyfunction]
    pub fn query_module_only(
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
    pub struct Resolution(Arc<ext::resolution::Resolution<ext::CCC>>);

    #[pymethods]
    impl Resolution {
        pub fn compute_through_stem(&self, max: sseq_py::Bidegree) {
            self.0.compute_through_stem(max.0)
        }

        pub fn graded_dimension_string(&self) -> String {
            self.0.graded_dimension_string()
        }
    }

    #[pyclass(frozen)]
    pub struct SecondaryResolution(
        ext::secondary::SecondaryResolution<ext::resolution::Resolution<ext::CCC>>,
    );

    #[pymethods]
    impl SecondaryResolution {
        #[new]
        pub fn new(cc: Resolution) -> Self {
            SecondaryResolution(ext::secondary::SecondaryResolution::new(cc.0))
        }

        pub fn extend_all(&self) {
            self.0.extend_all();
        }

        pub fn underlying(&self) -> Resolution {
            Resolution(Arc::clone(&self.0.underlying()))
        }
    }

    #[pymodule_init]
    fn init(_m: &Bound<'_, PyModule>) -> PyResult<()> {
        ext::utils::init_logging()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(())
    }
}
