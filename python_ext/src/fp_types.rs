//! Bindings for `fp::prime::ValidPrime`, `fp::vector::FpVector`,
//! `fp::matrix::{Matrix, AugmentedMatrix, Subspace}`.
//!
//! ## `FpVector` slicing
//!
//! The Python `FpVector` class is a tagged union of three modes:
//!
//! 1. **Owned** — wraps an actual `fp::vector::FpVector` value.
//! 2. **View** — a read-only borrow into another object's storage (an
//!    `FpVector` itself, or a row of a `Matrix`/`AugmentedMatrix`).
//! 3. **ViewMut** — a mutable borrow.
//!
//! Each operation that mutates an `FpVector` checks the mode and raises
//! `BorrowError` on `View` or other unsupported modes. Each operation on a
//! view re-derives the underlying `FpSlice` / `FpSliceMut` from the parent
//! transiently, guarded by pyo3's runtime borrow check on the parent
//! `pyclass`. This means a view becomes safe to use as soon as the parent
//! is no longer being mutated through Python.
//!
//! Methods like `slice` / `slice_mut` / `Matrix.row_view_mut` /
//! `AugmentedMatrix.row_segment_view_mut` return a view. Methods like
//! `Matrix.extend_column_dimension` (not exposed) would invalidate the
//! view's indices; we deliberately don't expose any such method.

use anyhow::anyhow;
use fp::{
    matrix::{self as m, AugmentedMatrix as AM, Matrix as M, Subspace as S},
    prime::{Prime, ValidPrime as VP},
    vector::{FpSlice, FpSliceMut, FpVector as FV},
};
use pyo3::exceptions::{PyBufferError, PyIndexError, PyValueError};
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

/// Where a non-Owned `FpVector` reads/writes its data from.
///
/// SAFETY contract: a view holds a `Py<PyAny>` to its `parent`. The pointer
/// stored here is only dereferenced while a `borrow()` (for `View`) or
/// `borrow_mut()` (for `ViewMut`) is held on the parent. Construction of the
/// pointer is a temporary derivation from a `&mut Matrix` / `&mut FV`, valid
/// because the underlying `FV`/`M` lives inline in the `pyclass` struct,
/// which pyo3 allocates on the heap and pins for the lifetime of the
/// `Py<PyAny>` handle.
///
/// Methods like `Matrix::extend_column_capacity` could reallocate and
/// invalidate indices semantically; we never expose any such method.
enum ViewSource {
    /// A (possibly trivial) sub-range `[start, end)` of a stored `FV`.
    FpVec {
        ptr: *mut FV,
        start: usize,
        end: usize,
    },
    /// Row `row` of a `Matrix`, columns `[start, end)`. AugmentedMatrix
    /// views collapse to this by translating segment indices to column
    /// indices at construction time.
    MatRow {
        ptr: *mut M,
        row: usize,
        start: usize,
        end: usize,
    },
}

// SAFETY: the raw pointers are only used while we hold a Python borrow on
// the owning `Py<PyAny>`, which prevents them from racing with any other
// access. Whether or not Send/Sync is sound for the *parent* is checked by
// pyo3 separately; here we only assert that storing a raw pointer doesn't
// itself add unsoundness beyond what pyo3's GIL model already guarantees.
unsafe impl Send for ViewSource {}
unsafe impl Sync for ViewSource {}

impl ViewSource {
    /// Length of the view.
    #[allow(dead_code)]
    fn len(&self) -> usize {
        match *self {
            ViewSource::FpVec { start, end, .. } => end - start,
            ViewSource::MatRow { start, end, .. } => end - start,
        }
    }

    /// Compose `self` with a sub-range `[sub_start, sub_end)` measured in
    /// view-local coordinates.
    fn subrange(&self, sub_start: usize, sub_end: usize) -> Self {
        match *self {
            ViewSource::FpVec { ptr, start, .. } => ViewSource::FpVec {
                ptr,
                start: start + sub_start,
                end: start + sub_end,
            },
            ViewSource::MatRow {
                ptr, row, start, ..
            } => ViewSource::MatRow {
                ptr,
                row,
                start: start + sub_start,
                end: start + sub_end,
            },
        }
    }

    /// Run `f` with a borrowed `FpSlice` derived from the view.
    ///
    /// SAFETY: the caller must hold an outer borrow on the parent
    /// `pyclass` for the duration of this call.
    unsafe fn with_slice<R>(&self, f: impl FnOnce(FpSlice<'_>) -> R) -> R {
        match *self {
            ViewSource::FpVec { ptr, start, end } => {
                let s = unsafe { (*ptr).slice(start, end) };
                f(s)
            }
            ViewSource::MatRow {
                ptr,
                row,
                start,
                end,
            } => {
                let row_slice = unsafe { (*ptr).row(row) };
                f(row_slice.restrict(start, end))
            }
        }
    }

    /// Run `f` with a borrowed `FpSliceMut` derived from the view. SAFETY:
    /// the caller must hold an outer mutable borrow on the parent.
    unsafe fn with_slice_mut<R>(&mut self, f: impl FnOnce(FpSliceMut<'_>) -> R) -> R {
        match *self {
            ViewSource::FpVec { ptr, start, end } => {
                let s = unsafe { (*ptr).slice_mut(start, end) };
                f(s)
            }
            ViewSource::MatRow {
                ptr,
                row,
                start,
                end,
            } => {
                // We first get a long-lived `&mut Matrix`, then derive the
                // row slice in a single expression that returns through `f`.
                let mat: &mut M = unsafe { &mut *ptr };
                let mut row_slice = mat.row_mut(row);
                f(row_slice.slice_mut(start, end))
            }
        }
    }
}

/// Distinct kinds of parent that an `FpVector` view can borrow from. We use
/// a closed enum so we can call `try_borrow` / `try_borrow_mut` (which
/// require a concrete `pyclass` type) for runtime borrow tracking.
enum ViewOwner {
    FpVector(Py<FpVector>),
    Matrix(Py<Matrix>),
    AugmentedMatrix(Py<AugmentedMatrix>),
}

impl ViewOwner {
    fn clone_ref(&self, py: Python<'_>) -> Self {
        match self {
            ViewOwner::FpVector(p) => ViewOwner::FpVector(p.clone_ref(py)),
            ViewOwner::Matrix(p) => ViewOwner::Matrix(p.clone_ref(py)),
            ViewOwner::AugmentedMatrix(p) => ViewOwner::AugmentedMatrix(p.clone_ref(py)),
        }
    }

    /// Acquire a read borrow on the parent. Returns a guard that pins it.
    fn borrow<'py>(&'py self, py: Python<'py>) -> PyResult<ViewBorrow<'py>> {
        match self {
            ViewOwner::FpVector(p) => Ok(ViewBorrow::FpVector(
                p.bind(py).try_borrow().map_err(|e| {
                    PyBufferError::new_err(format!("Parent FpVector is borrowed: {e}"))
                })?,
            )),
            ViewOwner::Matrix(p) => Ok(ViewBorrow::Matrix(
                p.bind(py).try_borrow().map_err(|e| {
                    PyBufferError::new_err(format!("Parent Matrix is borrowed: {e}"))
                })?,
            )),
            ViewOwner::AugmentedMatrix(p) => Ok(ViewBorrow::AugmentedMatrix(
                p.bind(py).try_borrow().map_err(|e| {
                    PyBufferError::new_err(format!(
                        "Parent AugmentedMatrix is borrowed: {e}"
                    ))
                })?,
            )),
        }
    }

    /// Acquire a write borrow on the parent.
    fn borrow_mut<'py>(&'py self, py: Python<'py>) -> PyResult<ViewBorrowMut<'py>> {
        match self {
            ViewOwner::FpVector(p) => Ok(ViewBorrowMut::FpVector(
                p.bind(py).try_borrow_mut().map_err(|e| {
                    PyBufferError::new_err(format!("Parent FpVector is borrowed: {e}"))
                })?,
            )),
            ViewOwner::Matrix(p) => Ok(ViewBorrowMut::Matrix(
                p.bind(py).try_borrow_mut().map_err(|e| {
                    PyBufferError::new_err(format!("Parent Matrix is borrowed: {e}"))
                })?,
            )),
            ViewOwner::AugmentedMatrix(p) => Ok(ViewBorrowMut::AugmentedMatrix(
                p.bind(py).try_borrow_mut().map_err(|e| {
                    PyBufferError::new_err(format!(
                        "Parent AugmentedMatrix is borrowed: {e}"
                    ))
                })?,
            )),
        }
    }
}

#[allow(dead_code)]
enum ViewBorrow<'py> {
    FpVector(PyRef<'py, FpVector>),
    Matrix(PyRef<'py, Matrix>),
    AugmentedMatrix(PyRef<'py, AugmentedMatrix>),
}

#[allow(dead_code)]
enum ViewBorrowMut<'py> {
    FpVector(PyRefMut<'py, FpVector>),
    Matrix(PyRefMut<'py, Matrix>),
    AugmentedMatrix(PyRefMut<'py, AugmentedMatrix>),
}

enum FpVectorKind {
    Owned(FV),
    /// Read-only view.
    View {
        owner: ViewOwner,
        source: ViewSource,
    },
    /// Mutable view.
    ViewMut {
        owner: ViewOwner,
        source: ViewSource,
    },
}

/// A vector over $\mathbb{F}_p$. May be owned, a read-only view, or a
/// mutable view (see module docs).
#[pyclass(name = "FpVector", module = "sseq_ext", skip_from_py_object)]
pub struct FpVector {
    kind: FpVectorKind,
    /// Cached prime so `prime()` works without re-deriving the slice.
    prime: VP,
    /// Cached length.
    len: usize,
}

impl FpVector {
    /// Construct an owned vector.
    pub fn new_owned(inner: FV) -> Self {
        let prime = inner.prime();
        let len = inner.len();
        Self {
            kind: FpVectorKind::Owned(inner),
            prime,
            len,
        }
    }

    /// True if this is an owned `FpVector` (not a view).
    fn check_owned(&self) -> bool {
        matches!(self.kind, FpVectorKind::Owned(_))
    }

    /// True if this is mutable (owned or `ViewMut`).
    fn is_mutable(&self) -> bool {
        matches!(
            self.kind,
            FpVectorKind::Owned(_) | FpVectorKind::ViewMut { .. }
        )
    }

    /// Run a closure with a `&FpSlice` derived from this vector.
    ///
    /// For views, this acquires a `borrow()` (or `borrow_mut()` for
    /// `ViewMut`) on the parent for the duration of the closure.
    fn with_slice<R>(&self, py: Python<'_>, f: impl FnOnce(FpSlice<'_>) -> R) -> PyResult<R> {
        match &self.kind {
            FpVectorKind::Owned(v) => Ok(f(v.as_slice())),
            FpVectorKind::View { owner, source } => {
                let _guard = owner.borrow(py)?;
                // SAFETY: _guard pins the parent for the duration of this
                // call, the pointer in `source` was derived from that parent
                // and is still valid.
                Ok(unsafe { source.with_slice(f) })
            }
            FpVectorKind::ViewMut { owner, source } => {
                let _guard = owner.borrow_mut(py)?;
                Ok(unsafe { source.with_slice(f) })
            }
        }
    }

    /// Run a closure with a `&mut FpSliceMut` derived from this vector. Errors
    /// out if the vector is a read-only view.
    fn with_slice_mut<R>(
        &mut self,
        py: Python<'_>,
        f: impl FnOnce(FpSliceMut<'_>) -> R,
    ) -> PyResult<R> {
        match &mut self.kind {
            FpVectorKind::Owned(v) => Ok(f(v.as_slice_mut())),
            FpVectorKind::View { .. } => Err(PyBufferError::new_err(
                "FpVector is a read-only view; cannot mutate",
            )),
            FpVectorKind::ViewMut { owner, source } => {
                let _guard = owner.borrow_mut(py)?;
                // SAFETY: _guard ensures exclusive access for the duration.
                Ok(unsafe { source.with_slice_mut(f) })
            }
        }
    }
}

#[pymethods]
impl FpVector {
    /// Construct a new owned zero vector of length `len`.
    #[new]
    fn new(p: &ValidPrime, len: usize) -> Self {
        Self::new_owned(FV::new(p.inner, len))
    }

    /// Build an owned `FpVector` of length `len(slice)` from a sequence of
    /// non-negative integers.
    #[classmethod]
    fn from_slice(_cls: &Bound<'_, PyType>, p: &ValidPrime, slice: Vec<u32>) -> Self {
        Self::new_owned(FV::from_slice(p.inner, &slice))
    }

    fn prime(&self) -> ValidPrime {
        ValidPrime { inner: self.prime }
    }

    fn __len__(&self) -> usize {
        self.len
    }

    /// `True` if this is an owned `FpVector`.
    #[getter]
    fn is_owned(&self) -> bool {
        Self::check_owned(self)
    }

    /// `True` if writes are allowed (owned or mutable view).
    #[getter]
    fn writable(&self) -> bool {
        self.is_mutable()
    }

    fn entry(&self, py: Python<'_>, index: usize) -> PyResult<u32> {
        if index >= self.len {
            return Err(PyIndexError::new_err("FpVector index out of range"));
        }
        self.with_slice(py, |s| s.entry(index))
    }

    fn set_entry(&mut self, py: Python<'_>, index: usize, value: u32) -> PyResult<()> {
        if index >= self.len {
            return Err(PyIndexError::new_err("FpVector index out of range"));
        }
        self.with_slice_mut(py, |mut s| s.set_entry(index, value))
    }

    fn set_to_zero(&mut self, py: Python<'_>) -> PyResult<()> {
        self.with_slice_mut(py, |mut s| s.set_to_zero())
    }

    fn is_zero(&self, py: Python<'_>) -> PyResult<bool> {
        self.with_slice(py, |s| s.is_zero())
    }

    /// Add `value * basis_element[index]` to this vector.
    fn add_basis_element(&mut self, py: Python<'_>, index: usize, value: u32) -> PyResult<()> {
        if index >= self.len {
            return Err(PyIndexError::new_err("FpVector index out of range"));
        }
        self.with_slice_mut(py, |mut s| s.add_basis_element(index, value))
    }

    /// Return the entries as a Python list of integers.
    fn to_list(&self, py: Python<'_>) -> PyResult<Vec<u32>> {
        self.with_slice(py, |s| s.iter().collect())
    }

    /// Return an *owned* copy of this vector. Always succeeds (creates an
    /// independent `FpVector`).
    fn to_owned_vector(&self, py: Python<'_>) -> PyResult<FpVector> {
        self.with_slice(py, |s| Self::new_owned(s.to_owned()))
    }

    /// Return a read-only sub-view `[start, end)`.
    ///
    /// The returned view borrows from the same underlying storage as `self`.
    /// If `self` is a view, the new view points at the same root parent.
    fn slice(slf: Bound<'_, Self>, start: usize, end: usize) -> PyResult<FpVector> {
        let py = slf.py();
        let (prime, len_) = {
            let r = slf.borrow();
            (r.prime, r.len)
        };
        if start > end || end > len_ {
            return Err(PyIndexError::new_err(format!(
                "slice [{start}, {end}) out of range for length {len_}"
            )));
        }
        let r = slf.borrow();
        let kind = match &r.kind {
            FpVectorKind::Owned(v) => {
                let ptr = v as *const FV as *mut FV;
                let owner = ViewOwner::FpVector(slf.clone().unbind());
                FpVectorKind::View {
                    owner,
                    source: ViewSource::FpVec { ptr, start, end },
                }
            }
            FpVectorKind::View { owner, source } => FpVectorKind::View {
                owner: owner.clone_ref(py),
                source: source.subrange(start, end),
            },
            // Downgrade to read-only since `slice` is immutable.
            FpVectorKind::ViewMut { owner, source } => FpVectorKind::View {
                owner: owner.clone_ref(py),
                source: source.subrange(start, end),
            },
        };
        Ok(FpVector {
            kind,
            prime,
            len: end - start,
        })
    }

    /// Return a mutable sub-view `[start, end)`. Errors out if `self` is a
    /// read-only view.
    fn slice_mut(slf: Bound<'_, Self>, start: usize, end: usize) -> PyResult<FpVector> {
        let py = slf.py();
        let (prime, len_) = {
            let r = slf.borrow();
            (r.prime, r.len)
        };
        if start > end || end > len_ {
            return Err(PyIndexError::new_err(format!(
                "slice_mut [{start}, {end}) out of range for length {len_}"
            )));
        }
        // We need a mutable borrow to obtain the raw pointer, but we drop
        // the borrow before constructing the view (so subsequent operations
        // on the view can re-borrow).
        let mut r = slf.borrow_mut();
        let kind = match &mut r.kind {
            FpVectorKind::Owned(v) => {
                let ptr = v as *mut FV;
                drop(r);
                let owner = ViewOwner::FpVector(slf.clone().unbind());
                FpVectorKind::ViewMut {
                    owner,
                    source: ViewSource::FpVec { ptr, start, end },
                }
            }
            FpVectorKind::View { .. } => {
                return Err(PyBufferError::new_err(
                    "Cannot derive a mutable sub-view from a read-only view",
                ));
            }
            FpVectorKind::ViewMut { owner, source } => {
                let new_source = source.subrange(start, end);
                let new_owner = owner.clone_ref(py);
                FpVectorKind::ViewMut {
                    owner: new_owner,
                    source: new_source,
                }
            }
        };
        Ok(FpVector {
            kind,
            prime,
            len: end - start,
        })
    }

    fn __getitem__(&self, py: Python<'_>, index: usize) -> PyResult<u32> {
        self.entry(py, index)
    }

    fn __setitem__(&mut self, py: Python<'_>, index: usize, value: u32) -> PyResult<()> {
        self.set_entry(py, index, value)
    }

    fn __repr__(&self, py: Python<'_>) -> String {
        let kind_str = match self.kind {
            FpVectorKind::Owned(_) => "owned",
            FpVectorKind::View { .. } => "view",
            FpVectorKind::ViewMut { .. } => "view-mut",
        };
        let entries = self.to_list(py).unwrap_or_default();
        format!("FpVector(<{kind_str}> {entries:?})")
    }
}

/// A matrix over $\mathbb{F}_p$.
#[pyclass(name = "Matrix", module = "sseq_ext", skip_from_py_object)]
#[derive(Debug, Clone)]
pub struct Matrix {
    pub inner: M,
}

impl Matrix {
    fn raw_ptr(&mut self) -> *mut M {
        &mut self.inner as *mut M
    }
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
        (cols, Self { inner: mat })
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

    /// Return a read-only view of `row`.
    fn row_view(slf: Bound<'_, Self>, row: usize) -> PyResult<FpVector> {
        let (rows, columns, prime) = {
            let r = slf.borrow();
            (r.inner.rows(), r.inner.columns(), r.inner.prime())
        };
        if row >= rows {
            return Err(PyIndexError::new_err("row index out of range"));
        }
        let ptr = slf.borrow_mut().raw_ptr();
        Ok(FpVector {
            kind: FpVectorKind::View {
                owner: ViewOwner::Matrix(slf.unbind()),
                source: ViewSource::MatRow {
                    ptr,
                    row,
                    start: 0,
                    end: columns,
                },
            },
            prime,
            len: columns,
        })
    }

    /// Return a mutable view of `row`.
    fn row_view_mut(slf: Bound<'_, Self>, row: usize) -> PyResult<FpVector> {
        let (rows, columns, prime) = {
            let r = slf.borrow();
            (r.inner.rows(), r.inner.columns(), r.inner.prime())
        };
        if row >= rows {
            return Err(PyIndexError::new_err("row index out of range"));
        }
        let ptr = slf.borrow_mut().raw_ptr();
        Ok(FpVector {
            kind: FpVectorKind::ViewMut {
                owner: ViewOwner::Matrix(slf.unbind()),
                source: ViewSource::MatRow {
                    ptr,
                    row,
                    start: 0,
                    end: columns,
                },
            },
            prime,
            len: columns,
        })
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

impl AugmentedInner {
    fn rows(&self) -> usize {
        match self {
            AugmentedInner::N2(m) => m.rows(),
            AugmentedInner::N3(m) => m.rows(),
        }
    }

    fn columns(&self) -> usize {
        match self {
            AugmentedInner::N2(m) => m.columns(),
            AugmentedInner::N3(m) => m.columns(),
        }
    }

    fn prime(&self) -> VP {
        match self {
            AugmentedInner::N2(m) => m.prime(),
            AugmentedInner::N3(m) => m.prime(),
        }
    }

    /// `(start, end)` column indices for `segment(start_seg, end_seg)`.
    fn segment_range(&self, start_seg: usize, end_seg: usize) -> (usize, usize) {
        match self {
            AugmentedInner::N2(m) => (m.start[start_seg], m.end[end_seg]),
            AugmentedInner::N3(m) => (m.start[start_seg], m.end[end_seg]),
        }
    }

    /// Pointer to the underlying `Matrix` (the inner field). Both `AM<2>`
    /// and `AM<3>` `Deref` to `Matrix`, so we coerce.
    fn matrix_ptr(&mut self) -> *mut M {
        match self {
            AugmentedInner::N2(m) => &mut m.inner as *mut M,
            AugmentedInner::N3(m) => &mut m.inner as *mut M,
        }
    }
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
        self.inner.rows()
    }

    fn columns(&self) -> usize {
        self.inner.columns()
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

    /// Set entry `(row, col)` of `segment[seg, seg]`.
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

    /// Read-only view of `row`, columns `segment(start_seg, end_seg)`.
    #[pyo3(signature = (row, start_seg=0, end_seg=None))]
    fn row_segment_view(
        slf: Bound<'_, Self>,
        row: usize,
        start_seg: usize,
        end_seg: Option<usize>,
    ) -> PyResult<FpVector> {
        let end_seg = end_seg.unwrap_or(start_seg);
        let (rows, prime, start, end) = {
            let r = slf.borrow();
            let (s, e) = r.inner.segment_range(start_seg, end_seg);
            (r.inner.rows(), r.inner.prime(), s, e)
        };
        if row >= rows {
            return Err(PyIndexError::new_err("row index out of range"));
        }
        let ptr = slf.borrow_mut().inner.matrix_ptr();
        Ok(FpVector {
            kind: FpVectorKind::View {
                owner: ViewOwner::AugmentedMatrix(slf.unbind()),
                source: ViewSource::MatRow {
                    ptr,
                    row,
                    start,
                    end,
                },
            },
            prime,
            len: end - start,
        })
    }

    /// Mutable view of `row`, columns `segment(start_seg, end_seg)`.
    #[pyo3(signature = (row, start_seg=0, end_seg=None))]
    fn row_segment_view_mut(
        slf: Bound<'_, Self>,
        row: usize,
        start_seg: usize,
        end_seg: Option<usize>,
    ) -> PyResult<FpVector> {
        let end_seg = end_seg.unwrap_or(start_seg);
        let (rows, prime, start, end) = {
            let r = slf.borrow();
            let (s, e) = r.inner.segment_range(start_seg, end_seg);
            (r.inner.rows(), r.inner.prime(), s, e)
        };
        if row >= rows {
            return Err(PyIndexError::new_err("row index out of range"));
        }
        let ptr = slf.borrow_mut().inner.matrix_ptr();
        Ok(FpVector {
            kind: FpVectorKind::ViewMut {
                owner: ViewOwner::AugmentedMatrix(slf.unbind()),
                source: ViewSource::MatRow {
                    ptr,
                    row,
                    start,
                    end,
                },
            },
            prime,
            len: end - start,
        })
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

    fn contains(&self, py: Python<'_>, vec: &FpVector) -> PyResult<bool> {
        if vec.len != self.inner.ambient_dimension() {
            return Err(anyhow!(
                "Vector length {} doesn't match ambient dimension {}",
                vec.len,
                self.inner.ambient_dimension()
            )
            .into());
        }
        vec.with_slice(py, |s| self.inner.contains(s))
    }

    /// Return the basis vectors as a list of *owned* `FpVector`s.
    fn basis(&self) -> Vec<FpVector> {
        self.inner
            .basis()
            .map(|s| FpVector::new_owned(s.to_owned()))
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
        Self::new_owned(inner)
    }
}

impl From<M> for Matrix {
    fn from(inner: M) -> Self {
        Self { inner }
    }
}

impl FpVector {
    /// Internal helper: with a slice borrowed from `self`, run `f`. Used by
    /// other binding modules that want to call FpVector-consuming APIs.
    pub fn with_slice_pub<R>(
        &self,
        py: Python<'_>,
        f: impl FnOnce(FpSlice<'_>) -> R,
    ) -> PyResult<R> {
        self.with_slice(py, f)
    }

    pub fn with_slice_mut_pub<R>(
        &mut self,
        py: Python<'_>,
        f: impl FnOnce(FpSliceMut<'_>) -> R,
    ) -> PyResult<R> {
        self.with_slice_mut(py, f)
    }
}

// (no extra impls; `hom.act(view_mut, coeff, gen)` works directly with a
// row-segment view of an `AugmentedMatrix`.)
