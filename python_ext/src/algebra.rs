//! Bindings for `algebra::MilnorAlgebra` (and the `Algebra` trait methods
//! used by it).
//!
//! Exposed for the `algebra_dim` example, which needs direct access to the
//! Steenrod algebra without going through a `Resolution`.

use std::sync::Arc;

use algebra::{Algebra, MilnorAlgebra as InnerMA};
use fp::prime::Prime;
use pyo3::prelude::*;

use crate::fp_types::ValidPrime;

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
    fn new(p: &ValidPrime, unstable: bool) -> Self {
        Self {
            inner: Arc::new(InnerMA::new(p.inner, unstable)),
        }
    }

    fn prime(&self) -> ValidPrime {
        ValidPrime {
            inner: self.inner.prime(),
        }
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
