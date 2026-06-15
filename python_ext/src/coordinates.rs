//! Bindings for `sseq::coordinates::{Bidegree, BidegreeGenerator, BidegreeElement}`.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyTuple, PyType};
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

    /// Human-readable form, e.g. ``(n, s, idx)``.
    fn __str__(&self) -> String {
        format!("{}", self.inner)
    }

    /// Format-spec mini-language. Supported specs:
    ///
    /// - ``""`` or ``"full"`` — same as ``str(g)``: ``"(n, s, idx)"``.
    /// - ``"compact"`` — ``"(n,s,idx)"`` (no spaces).
    fn __format__(&self, spec: &str) -> PyResult<String> {
        match spec {
            "" | "full" => Ok(format!("{}", self.inner)),
            "compact" => Ok(format!("{:#}", self.inner)),
            other => Err(PyValueError::new_err(format!(
                "Unknown format spec for BidegreeGenerator: {:?} \
                 (expected '', 'full', or 'compact')",
                other
            ))),
        }
    }

    /// Iterate over ``(degree, idx)`` so the generator can be unpacked:
    /// ``b, i = g``.
    fn __iter__(slf: PyRef<'_, Self>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let degree = Bidegree {
            inner: slf.inner.degree(),
        }
        .into_pyobject(py)?
        .into_any()
        .unbind();
        let idx = slf.inner.idx().into_pyobject(py)?.into_any().unbind();
        let tuple = PyTuple::new(py, [degree, idx])?;
        Ok(tuple.try_iter()?.unbind().into())
    }

    fn __len__(&self) -> usize {
        2
    }

    /// `g[0]` is the degree (a `Bidegree`), `g[1]` is the index. Supports
    /// negative indices, completing the sequence protocol implied by
    /// `__len__`/`__iter__`.
    fn __getitem__(&self, py: Python<'_>, idx: isize) -> PyResult<Py<PyAny>> {
        match idx {
            0 | -2 => Ok(Bidegree {
                inner: self.inner.degree(),
            }
            .into_pyobject(py)?
            .into_any()
            .unbind()),
            1 | -1 => Ok(self.inner.idx().into_pyobject(py)?.into_any().unbind()),
            _ => Err(pyo3::exceptions::PyIndexError::new_err(
                "BidegreeGenerator index out of range (expected 0 or 1)",
            )),
        }
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
    #[getter]
    fn vec(&self) -> FpVector {
        FpVector::new_owned(self.inner.vec().to_owned())
    }

    fn to_basis_string(&self) -> String {
        self.inner.to_basis_string()
    }

    /// Value equality: two `BidegreeElement`s are equal iff they have the
    /// same degree and the same underlying vector.
    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn __hash__(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.inner.hash(&mut hasher);
        hasher.finish()
    }

    fn __repr__(&self) -> String {
        format!("BidegreeElement({})", self.inner)
    }
}
