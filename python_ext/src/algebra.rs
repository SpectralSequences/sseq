//! Bindings for `algebra::MilnorAlgebra` (and the `Algebra` trait methods
//! used by it).
//!
//! Exposed for the `algebra_dim` example, which needs direct access to the
//! Steenrod algebra without going through a `Resolution`.

use std::sync::Arc;

use algebra::{Algebra, MilnorAlgebra as InnerMA};
use fp::prime::{Prime, ValidPrime};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// The Milnor basis of the (mod-`p`) Steenrod algebra.
#[pyclass(name = "MilnorAlgebra", module = "sseq_ext")]
pub struct MilnorAlgebra {
    pub inner: Arc<InnerMA>,
}

#[pymethods]
impl MilnorAlgebra {
    /// `MilnorAlgebra(p, unstable=False)`. The `unstable` flag enables the
    /// instability bookkeeping needed for unstable resolutions; leave it
    /// `False` for stable computations.
    #[new]
    #[pyo3(signature = (p, unstable=false))]
    fn new(p: u32, unstable: bool) -> PyResult<Self> {
        let p = ValidPrime::try_from(p)
            .map_err(|e| PyValueError::new_err(format!("Invalid prime: {e}")))?;
        Ok(Self {
            inner: Arc::new(InnerMA::new(p, unstable)),
        })
    }

    fn prime(&self) -> u32 {
        self.inner.prime().as_u32()
    }

    /// Compute the basis of the algebra up to and including internal
    /// degree `t`.
    fn compute_basis(&self, t: i32, py: Python<'_>) {
        py.detach(|| self.inner.compute_basis(t));
    }

    /// `dim A_t`.
    fn dimension(&self, t: i32) -> usize {
        self.inner.dimension(t)
    }

    fn __repr__(&self) -> String {
        format!(
            "MilnorAlgebra(p={})",
            self.inner.prime().as_u32()
        )
    }
}
