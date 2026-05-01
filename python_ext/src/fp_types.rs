//! Bindings for `fp::prime::ValidPrime`, `fp::vector::FpVector`,
//! `fp::matrix::{Matrix, AugmentedMatrix, Subspace}`.

use anyhow::anyhow;
use fp::{
    matrix::{self as m, AugmentedMatrix as AM, Matrix as M, Subspace as S},
    prime::{Prime, ValidPrime as VP},
    vector::FpVector as FV,
};
use pyo3::exceptions::{PyIndexError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyType;

/// A prime number, validated at construction time.
#[pyclass(name = "ValidPrime", module = "sseq_ext", frozen, eq, hash, skip_from_py_object)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ValidPrime {
    pub inner: VP,
}

impl From<VP> for ValidPrime {
    fn from(inner: VP) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl ValidPrime {
    #[new]
    fn new(p: u32) -> PyResult<Self> {
        VP::try_from(p)
            .map(|inner| Self { inner })
            .map_err(|e| PyValueError::new_err(format!("Invalid prime: {e}")))
    }

    #[getter]
    fn value(&self) -> u32 {
        self.inner.as_u32()
    }

    fn __int__(&self) -> u32 {
        self.inner.as_u32()
    }

    fn __repr__(&self) -> String {
        format!("ValidPrime({})", self.inner.as_u32())
    }
}

/// A vector over $\mathbb{F}_p$.
#[pyclass(name = "FpVector", module = "sseq_ext", skip_from_py_object)]
#[derive(Debug, Clone)]
pub struct FpVector {
    pub inner: FV,
}

#[pymethods]
impl FpVector {
    #[new]
    fn new(p: &ValidPrime, len: usize) -> Self {
        Self {
            inner: FV::new(p.inner, len),
        }
    }

    /// Build an `FpVector` of length `len(slice)` from a sequence of
    /// non-negative integers.
    #[classmethod]
    fn from_slice(_cls: &Bound<'_, PyType>, p: &ValidPrime, slice: Vec<u32>) -> Self {
        Self {
            inner: FV::from_slice(p.inner, &slice),
        }
    }

    fn prime(&self) -> ValidPrime {
        ValidPrime {
            inner: self.inner.prime(),
        }
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }

    fn entry(&self, index: usize) -> u32 {
        self.inner.entry(index)
    }

    fn set_entry(&mut self, index: usize, value: u32) {
        self.inner.set_entry(index, value);
    }

    fn set_to_zero(&mut self) {
        self.inner.set_to_zero();
    }

    fn is_zero(&self) -> bool {
        self.inner.is_zero()
    }

    fn add_basis_element(&mut self, index: usize, value: u32) {
        self.inner.add_basis_element(index, value);
    }

    /// Return the entries as a Python list of integers.
    fn to_list(&self) -> Vec<u32> {
        self.inner.iter().collect()
    }

    fn __getitem__(&self, index: usize) -> PyResult<u32> {
        if index >= self.inner.len() {
            return Err(PyIndexError::new_err("FpVector index out of range"));
        }
        Ok(self.inner.entry(index))
    }

    fn __setitem__(&mut self, index: usize, value: u32) -> PyResult<()> {
        if index >= self.inner.len() {
            return Err(PyIndexError::new_err("FpVector index out of range"));
        }
        self.inner.set_entry(index, value);
        Ok(())
    }

    fn __repr__(&self) -> String {
        format!("FpVector({:?})", self.to_list())
    }
}

/// A matrix over $\mathbb{F}_p$.
#[pyclass(name = "Matrix", module = "sseq_ext", skip_from_py_object)]
#[derive(Debug, Clone)]
pub struct Matrix {
    pub inner: M,
}

#[pymethods]
impl Matrix {
    #[new]
    fn new(p: &ValidPrime, rows: usize, columns: usize) -> Self {
        Self {
            inner: M::new(p.inner, rows, columns),
        }
    }

    /// Build a matrix from a list of rows, each a list of non-negative integers.
    #[classmethod]
    fn from_vec(_cls: &Bound<'_, PyType>, p: &ValidPrime, rows: Vec<Vec<u32>>) -> Self {
        Self {
            inner: M::from_vec(p.inner, &rows),
        }
    }

    /// `(pivot_count, matrix)` where the second is `[m | I]`.
    #[classmethod]
    fn augmented_from_vec(
        _cls: &Bound<'_, PyType>,
        p: &ValidPrime,
        rows: Vec<Vec<u32>>,
    ) -> (usize, Self) {
        let (cols, mat) = M::augmented_from_vec(p.inner, &rows);
        (
            cols,
            Self {
                inner: mat,
            },
        )
    }

    fn prime(&self) -> ValidPrime {
        ValidPrime {
            inner: self.inner.prime(),
        }
    }

    fn rows(&self) -> usize {
        self.inner.rows()
    }

    fn columns(&self) -> usize {
        self.inner.columns()
    }

    /// Set entry `(row, col)` to `value`.
    fn set_entry(&mut self, row: usize, col: usize, value: u32) -> PyResult<()> {
        if row >= self.inner.rows() {
            return Err(PyIndexError::new_err("row index out of range"));
        }
        self.inner.row_mut(row).set_entry(col, value);
        Ok(())
    }

    fn entry(&self, row: usize, col: usize) -> PyResult<u32> {
        if row >= self.inner.rows() {
            return Err(PyIndexError::new_err("row index out of range"));
        }
        Ok(self.inner.row(row).entry(col))
    }

    /// Return the matrix as a list-of-lists of `u32`.
    fn to_list(&self) -> Vec<Vec<u32>> {
        self.inner.to_vec()
    }

    fn row_reduce(&mut self) -> usize {
        self.inner.row_reduce()
    }

    fn compute_kernel(&self, first_source_column: usize) -> Subspace {
        Subspace {
            inner: self.inner.compute_kernel(first_source_column),
        }
    }

    fn compute_image(&self, last_target_col: usize, first_source_col: usize) -> Subspace {
        Subspace {
            inner: self.inner.compute_image(last_target_col, first_source_col),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "Matrix({}x{}, p={})",
            self.inner.rows(),
            self.inner.columns(),
            self.inner.prime().as_u32()
        )
    }
}

/// An augmented matrix `[A | B]` (or with more segments).
#[pyclass(name = "AugmentedMatrix", module = "sseq_ext")]
pub struct AugmentedMatrix {
    /// Number of segments. We support 2 and 3 dynamically by storing the
    /// concrete type.
    inner: AugmentedInner,
}

enum AugmentedInner {
    N2(AM<2>),
    N3(AM<3>),
}

#[pymethods]
impl AugmentedMatrix {
    /// Construct a new augmented matrix with the given column dimensions.
    /// `columns` must have length 2 or 3.
    #[new]
    fn new(p: &ValidPrime, rows: usize, columns: Vec<usize>) -> PyResult<Self> {
        let inner = match columns.len() {
            2 => AugmentedInner::N2(AM::<2>::new(p.inner, rows, [columns[0], columns[1]])),
            3 => AugmentedInner::N3(AM::<3>::new(
                p.inner,
                rows,
                [columns[0], columns[1], columns[2]],
            )),
            n => {
                return Err(PyValueError::new_err(format!(
                    "AugmentedMatrix only supports 2 or 3 segments, got {n}"
                )))
            }
        };
        Ok(Self { inner })
    }

    fn rows(&self) -> usize {
        match &self.inner {
            AugmentedInner::N2(m) => m.rows(),
            AugmentedInner::N3(m) => m.rows(),
        }
    }

    fn columns(&self) -> usize {
        match &self.inner {
            AugmentedInner::N2(m) => m.columns(),
            AugmentedInner::N3(m) => m.columns(),
        }
    }

    /// Add the identity matrix to the segment `(seg, seg)`.
    fn segment_add_identity(&mut self, seg: usize) {
        match &mut self.inner {
            AugmentedInner::N2(m) => {
                m.segment(seg, seg).add_identity();
            }
            AugmentedInner::N3(m) => {
                m.segment(seg, seg).add_identity();
            }
        }
    }

    /// Set entry `(row, col)` (in the underlying flat matrix coordinates).
    fn set_entry(&mut self, row: usize, col: usize, value: u32) -> PyResult<()> {
        match &mut self.inner {
            AugmentedInner::N2(m) => m.row_mut(row).set_entry(col, value),
            AugmentedInner::N3(m) => m.row_mut(row).set_entry(col, value),
        };
        Ok(())
    }

    /// Set entry `(row, col)` of `segment[seg, seg]` (translates to the
    /// underlying flat coordinates internally).
    fn set_segment_entry(
        &mut self,
        row: usize,
        seg: usize,
        col: usize,
        value: u32,
    ) -> PyResult<()> {
        match &mut self.inner {
            AugmentedInner::N2(m) => {
                m.row_segment_mut(row, seg, seg).set_entry(col, value);
            }
            AugmentedInner::N3(m) => {
                m.row_segment_mut(row, seg, seg).set_entry(col, value);
            }
        }
        Ok(())
    }

    /// Run row reduction.
    fn row_reduce(&mut self) -> usize {
        match &mut self.inner {
            AugmentedInner::N2(m) => m.row_reduce(),
            AugmentedInner::N3(m) => m.row_reduce(),
        }
    }

    fn compute_kernel(&self) -> Subspace {
        let inner = match &self.inner {
            AugmentedInner::N2(m) => m.compute_kernel(),
            AugmentedInner::N3(m) => m.compute_kernel(),
        };
        Subspace { inner }
    }

    fn compute_image(&self) -> PyResult<Subspace> {
        match &self.inner {
            AugmentedInner::N2(m) => Ok(Subspace {
                inner: m.compute_image(),
            }),
            AugmentedInner::N3(_) => Err(PyValueError::new_err(
                "compute_image is only available on 2-segment AugmentedMatrix",
            )),
        }
    }
}

impl AugmentedMatrix {
    /// Apply `hom.act` to the segment `(seg, seg)` of `row`, additively.
    /// Used internally by `ResolutionHomomorphism.act_on_augmented_row`.
    pub fn act_with_homomorphism(
        &mut self,
        row: usize,
        seg: usize,
        coeff: u32,
        g: &crate::coordinates::BidegreeGenerator,
        hom: &crate::homomorphism::ResolutionHomomorphism,
    ) -> PyResult<()> {
        match &mut self.inner {
            AugmentedInner::N2(m) => {
                hom.inner
                    .act(m.row_segment_mut(row, seg, seg), coeff, g.inner);
            }
            AugmentedInner::N3(m) => {
                hom.inner
                    .act(m.row_segment_mut(row, seg, seg), coeff, g.inner);
            }
        }
        Ok(())
    }
}

/// A subspace (used to represent kernels and images).
#[pyclass(name = "Subspace", module = "sseq_ext", skip_from_py_object)]
#[derive(Debug, Clone)]
pub struct Subspace {
    pub inner: S,
}

#[pymethods]
impl Subspace {
    fn dimension(&self) -> usize {
        self.inner.dimension()
    }

    fn ambient_dimension(&self) -> usize {
        self.inner.ambient_dimension()
    }

    fn contains(&self, vec: &FpVector) -> PyResult<bool> {
        if vec.inner.len() != self.inner.ambient_dimension() {
            return Err(anyhow!(
                "Vector length {} doesn't match ambient dimension {}",
                vec.inner.len(),
                self.inner.ambient_dimension()
            )
            .into());
        }
        Ok(self.inner.contains(vec.inner.as_slice()))
    }

    /// Return the basis vectors as a list of `FpVector`s.
    fn basis(&self) -> Vec<FpVector> {
        self.inner
            .basis()
            .map(|s| FpVector { inner: s.to_owned() })
            .collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "Subspace(dim={}, ambient_dim={})",
            self.inner.dimension(),
            self.inner.ambient_dimension()
        )
    }
}

// Helper `From` impls so other modules can construct fp types easily.
impl From<m::Subspace> for Subspace {
    fn from(inner: m::Subspace) -> Self {
        Self { inner }
    }
}

impl From<FV> for FpVector {
    fn from(inner: FV) -> Self {
        Self { inner }
    }
}

impl From<M> for Matrix {
    fn from(inner: M) -> Self {
        Self { inner }
    }
}
