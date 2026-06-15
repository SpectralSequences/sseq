//! Bindings for `ext::secondary::{SecondaryResolution, SecondaryHomotopy}`
//! instantiated with the `QueryModuleResolution`.
//!
//! Note that the underlying chain complex must use the Milnor basis, so
//! constructing `SecondaryResolution::new` with an Adem-based resolution will
//! eventually panic.

use std::sync::Arc;

use algebra::SteenrodAlgebra;
use ext::chain_complex::ChainComplex;
use ext::secondary::{
    SecondaryHomotopy as InnerSH, SecondaryLift, SecondaryResolution as InnerSR,
};
use ext::utils::QueryModuleResolution;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use crate::resolution::Resolution;
use crate::sseq_types::Sseq;

/// Wrap `&'static SecondaryHomotopy<SteenrodAlgebra>` borrowed from a
/// `SecondaryResolution`. We hold an owning `Arc` to the parent to keep the
/// reference live.
#[pyclass(name = "SecondaryHomotopy", module = "sseq_ext")]
pub struct SecondaryHomotopy {
    /// Keep the parent alive so the `&SecondaryHomotopy` reference remains
    /// valid for the lifetime of this Python object.
    #[allow(dead_code)]
    parent: Arc<InnerSR<QueryModuleResolution>>,
    /// Pointer into `parent.homotopies[s]`. SAFETY: parent is reference
    /// counted and never moves out of its `OnceBiVec` slot, and the inner
    /// `OnceBiVec` only ever appends.
    inner: *const InnerSH<SteenrodAlgebra>,
}

// SAFETY: the underlying `SecondaryHomotopy<SteenrodAlgebra>` is `Send + Sync`
// when its algebra is, and `SteenrodAlgebra: Send + Sync`.
unsafe impl Send for SecondaryHomotopy {}
unsafe impl Sync for SecondaryHomotopy {}

impl SecondaryHomotopy {
    fn inner(&self) -> &InnerSH<SteenrodAlgebra> {
        // SAFETY: see SAFETY comment on the struct.
        unsafe { &*self.inner }
    }
}

#[pymethods]
impl SecondaryHomotopy {
    fn shift_t(&self) -> i32 {
        self.inner().shift_t
    }

    /// `hom_k(t)`: matrix of `Hom(F_*, k)` at degree `t`. Shape
    /// `(target_dim, source_dim)`.
    fn hom_k(&self, t: i32) -> Vec<Vec<u32>> {
        self.inner().homotopies.hom_k(t)
    }

    fn __repr__(&self) -> String {
        format!("SecondaryHomotopy(shift_t={})", self.inner().shift_t)
    }
}

/// `SecondaryResolution<QueryModuleResolution>`.
#[pyclass(name = "SecondaryResolution", module = "sseq_ext")]
pub struct SecondaryResolution {
    pub inner: Arc<InnerSR<QueryModuleResolution>>,
}

#[pymethods]
impl SecondaryResolution {
    /// `SecondaryResolution::new(resolution)`. The resolution must be over
    /// the Milnor basis; raises `ValueError` otherwise.
    #[new]
    fn new(resolution: &Resolution) -> PyResult<Self> {
        if !matches!(
            &*resolution.inner.algebra(),
            SteenrodAlgebra::MilnorAlgebra(_)
        ) {
            return Err(PyValueError::new_err(
                "SecondaryResolution requires a resolution over the Milnor \
                 basis (e.g. construct(\"S_2\", algebra=\"milnor\"))",
            ));
        }
        Ok(Self {
            inner: Arc::new(InnerSR::new(resolution.arc())),
        })
    }

    /// The underlying `QueryModuleResolution`. Equivalent to the original
    /// resolution that was passed in.
    fn underlying(&self) -> Resolution {
        Resolution {
            inner: self.inner.underlying(),
        }
    }

    /// Compute all data on a single shard `s`, then return without finishing
    /// the rest. See `secondary.rs` for the sharding protocol.
    fn compute_partial(&self, s: i32, py: Python<'_>) {
        py.detach(|| self.inner.compute_partial(s));
    }

    fn extend_all(&self, py: Python<'_>) {
        py.detach(|| self.inner.extend_all());
    }

    /// Return the `SecondaryHomotopy` at homological degree `s` (which is the
    /// part that detects $d_2$ on the source `s - 2 -> s`).
    fn homotopy(&self, s: i32) -> SecondaryHomotopy {
        let parent = Arc::clone(&self.inner);
        let h: &InnerSH<SteenrodAlgebra> = parent.homotopy(s);
        SecondaryHomotopy {
            inner: h as *const _,
            parent,
        }
    }

    /// Build the E_3 page as an `Sseq`.
    fn e3_page(&self, py: Python<'_>) -> Sseq {
        Sseq {
            inner: py.detach(|| self.inner.e3_page()),
        }
    }
}
