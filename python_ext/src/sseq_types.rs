//! Bindings for `sseq::Sseq<2, Adams>` and `sseq::Product<2>`.
//!
//! `Sseq` is mainly used for charting; we expose enough to call
//! `write_to_graph` (in `chart.rs`).

use pyo3::prelude::*;
use sseq::{Adams, Product as InnerProduct, Sseq as InnerSseq};

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

/// `Product<2>` — represents a multiplication-by-class structure for
/// charting. Created by `Resolution.filtration_one_products`.
///
/// The inner type is opaque from Python; pass it back to
/// `Resolution.filtration_one_products` consumers (currently none in this
/// minimal binding).
#[pyclass(name = "Product", module = "sseq_ext")]
pub struct Product {
    #[allow(dead_code)]
    pub inner: InnerProduct<2>,
}
