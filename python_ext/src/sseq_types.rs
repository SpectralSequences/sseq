//! Bindings for `sseq::Sseq<2, Adams>`.
//!
//! `Sseq` is mainly used for charting; we expose enough to call
//! `write_to_graph` (in `chart.rs`).

use pyo3::prelude::*;
use sseq::{Adams, Sseq as InnerSseq};

/// `Sseq<2, Adams>`.
#[pyclass(name = "Sseq", module = "sseq_ext")]
pub struct Sseq {
    pub inner: InnerSseq<2, Adams>,
}

#[pymethods]
impl Sseq {
    fn dimension(&self, b: &crate::coordinates::Bidegree) -> usize {
        self.inner.dimension(b.inner)
    }

    fn defined(&self, b: &crate::coordinates::Bidegree) -> bool {
        self.inner.defined(b.inner)
    }
}
