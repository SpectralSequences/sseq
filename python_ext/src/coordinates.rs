//! Bindings for `sseq::coordinates::{Bidegree, BidegreeGenerator, BidegreeElement}`.

use pyo3::prelude::*;
use pyo3::types::PyType;
use sseq::coordinates as c;

use crate::fp_types::FpVector;

/// A bidegree `(n, s)` with `t = n + s`.
#[pyclass(name = "Bidegree", module = "sseq_ext", frozen, eq, hash, skip_from_py_object)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Bidegree {
    pub inner: c::Bidegree,
}

impl From<c::Bidegree> for Bidegree {
    fn from(inner: c::Bidegree) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl Bidegree {
    #[new]
    #[pyo3(signature = (n, s))]
    fn new(n: i32, s: i32) -> Self {
        Self {
            inner: c::Bidegree::n_s(n, s),
        }
    }

    /// Construct from a stem `n` and homological degree `s`.
    #[classmethod]
    fn n_s(_cls: &Bound<'_, PyType>, n: i32, s: i32) -> Self {
        Self {
            inner: c::Bidegree::n_s(n, s),
        }
    }

    /// Construct from homological degree `s` and internal degree `t`.
    #[classmethod]
    fn s_t(_cls: &Bound<'_, PyType>, s: i32, t: i32) -> Self {
        Self {
            inner: c::Bidegree::s_t(s, t),
        }
    }

    #[classmethod]
    fn zero(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: c::Bidegree::zero(),
        }
    }

    #[getter]
    fn n(&self) -> i32 {
        self.inner.n()
    }

    #[getter]
    fn s(&self) -> i32 {
        self.inner.s()
    }

    #[getter]
    fn t(&self) -> i32 {
        self.inner.t()
    }

    fn __add__(&self, other: &Self) -> Self {
        Self {
            inner: self.inner + other.inner,
        }
    }

    fn __sub__(&self, other: &Self) -> Self {
        Self {
            inner: self.inner - other.inner,
        }
    }

    fn __repr__(&self) -> String {
        format!("Bidegree(n={}, s={})", self.inner.n(), self.inner.s())
    }

    fn __str__(&self) -> String {
        format!("({}, {})", self.inner.n(), self.inner.s())
    }
}

/// A basis element `(degree, idx)` of a bidegree.
#[pyclass(name = "BidegreeGenerator", module = "sseq_ext", frozen, eq, hash, skip_from_py_object)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BidegreeGenerator {
    pub inner: c::BidegreeGenerator,
}

impl From<c::BidegreeGenerator> for BidegreeGenerator {
    fn from(inner: c::BidegreeGenerator) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl BidegreeGenerator {
    #[new]
    fn new(degree: &Bidegree, idx: usize) -> Self {
        Self {
            inner: c::BidegreeGenerator::new(degree.inner, idx),
        }
    }

    #[classmethod]
    fn n_s(_cls: &Bound<'_, PyType>, n: i32, s: i32, idx: usize) -> Self {
        Self {
            inner: c::BidegreeGenerator::n_s(n, s, idx),
        }
    }

    #[classmethod]
    fn s_t(_cls: &Bound<'_, PyType>, s: i32, t: i32, idx: usize) -> Self {
        Self {
            inner: c::BidegreeGenerator::s_t(s, t, idx),
        }
    }

    #[getter]
    fn n(&self) -> i32 {
        self.inner.n()
    }

    #[getter]
    fn s(&self) -> i32 {
        self.inner.s()
    }

    #[getter]
    fn t(&self) -> i32 {
        self.inner.t()
    }

    #[getter]
    fn idx(&self) -> usize {
        self.inner.idx()
    }

    #[getter]
    fn degree(&self) -> Bidegree {
        Bidegree {
            inner: self.inner.degree(),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "BidegreeGenerator(n={}, s={}, idx={})",
            self.inner.n(),
            self.inner.s(),
            self.inner.idx()
        )
    }

    fn __str__(&self) -> String {
        format!("{}", self.inner)
    }

    /// Compact form, e.g. ``(n,s,i)`` (no spaces). Equivalent to Rust's
    /// alternate `Display` form (``{:#}``).
    fn to_string_compact(&self) -> String {
        format!("{:#}", self.inner)
    }
}

/// An element of a bidegree, represented as a vector in the canonical basis.
#[pyclass(name = "BidegreeElement", module = "sseq_ext", skip_from_py_object)]
#[derive(Debug, Clone)]
pub struct BidegreeElement {
    pub inner: c::BidegreeElement,
}

impl From<c::BidegreeElement> for BidegreeElement {
    fn from(inner: c::BidegreeElement) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl BidegreeElement {
    /// Construct from a degree and a vector. The vector is *copied* (so
    /// it's safe to pass a view).
    #[new]
    fn new(py: Python<'_>, degree: &Bidegree, vec: &FpVector) -> PyResult<Self> {
        let owned: fp::vector::FpVector = vec.with_slice_pub(py, |s| s.to_owned())?;
        Ok(Self {
            inner: c::BidegreeElement::new(degree.inner, owned),
        })
    }

    #[getter]
    fn n(&self) -> i32 {
        self.inner.n()
    }

    #[getter]
    fn s(&self) -> i32 {
        self.inner.s()
    }

    #[getter]
    fn t(&self) -> i32 {
        self.inner.t()
    }

    #[getter]
    fn degree(&self) -> Bidegree {
        Bidegree {
            inner: self.inner.degree(),
        }
    }

    /// Return an owned copy of the underlying vector.
    fn vec(&self) -> FpVector {
        FpVector::new_owned(self.inner.vec().to_owned())
    }

    fn to_basis_string(&self) -> String {
        self.inner.to_basis_string()
    }

    fn __repr__(&self) -> String {
        format!("BidegreeElement({})", self.inner)
    }
}
