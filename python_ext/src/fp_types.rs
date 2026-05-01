//! Bindings for `fp::vector::FpVector` and
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
use pyo3::exceptions::{PyBufferError, PyIndexError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PySlice, PyType};

/// Convert a plain `u32` (from Python) to a validated `VP`.
fn vp_from_u32(p: u32) -> PyResult<VP> {
    VP::try_from(p).map_err(|e| PyValueError::new_err(format!("Invalid prime: {e}")))
}

/// Resolve a Python `slice` to `(start, end)` indices into a sequence of
/// length `len`. Step must be 1 (or unset).
fn resolve_slice(slice: &Bound<'_, PySlice>, len: usize) -> PyResult<(usize, usize)> {
    let indices = slice.indices(len as isize)?;
    if indices.step != 1 {
        return Err(PyValueError::new_err(
            "FpVector slicing only supports step=1",
        ));
    }
    let start = indices.start.max(0) as usize;
    let stop = indices.stop.max(0) as usize;
    let stop = stop.min(len);
    let start = start.min(stop);
    Ok((start, stop))
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

#[derive(Copy, Clone, PartialEq, Eq)]
enum FpKindTag {
    Owned,
    View,
    ViewMut,
}

/// A vector over $\mathbb{F}_p$. May be owned, a read-only view, or a
/// mutable view (see module docs).
#[pyclass(name = "FpVector", module = "sseq_ext", skip_from_py_object, weakref)]
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

    fn kind_tag(&self) -> FpKindTag {
        match self.kind {
            FpVectorKind::Owned(_) => FpKindTag::Owned,
            FpVectorKind::View { .. } => FpKindTag::View,
            FpVectorKind::ViewMut { .. } => FpKindTag::ViewMut,
        }
    }

    /// Slice that always returns a `View` (read-only) regardless of whether
    /// `self` is `Owned`, `View`, or `ViewMut`.
    fn slice_as_view(slf: Bound<'_, Self>, start: usize, end: usize) -> PyResult<Self> {
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
        let kind = match &slf.borrow().kind {
            FpVectorKind::Owned(v) => {
                let ptr = v as *const FV as *mut FV;
                FpVectorKind::View {
                    owner: ViewOwner::FpVector(slf.clone().unbind()),
                    source: ViewSource::FpVec { ptr, start, end },
                }
            }
            FpVectorKind::View { owner, source } => FpVectorKind::View {
                owner: owner.clone_ref(py),
                source: source.subrange(start, end),
            },
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

    /// Slice that always returns a `ViewMut`. Errors if `self` is a
    /// read-only view.
    fn slice_as_view_mut(slf: Bound<'_, Self>, start: usize, end: usize) -> PyResult<Self> {
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
        let kind = match &mut slf.borrow_mut().kind {
            FpVectorKind::Owned(v) => {
                let ptr = v as *mut FV;
                FpVectorKind::ViewMut {
                    owner: ViewOwner::FpVector(slf.clone().unbind()),
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
                FpVectorKind::ViewMut {
                    owner: owner.clone_ref(py),
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
    fn new(p: u32, len: usize) -> PyResult<Self> {
        let p = vp_from_u32(p)?;
        Ok(Self::new_owned(FV::new(p, len)))
    }

    /// Build an owned `FpVector` of length `len(slice)` from a sequence of
    /// non-negative integers.
    #[classmethod]
    fn from_slice(_cls: &Bound<'_, PyType>, p: u32, slice: Vec<u32>) -> PyResult<Self> {
        let p = vp_from_u32(p)?;
        Ok(Self::new_owned(FV::from_slice(p, &slice)))
    }

    fn prime(&self) -> u32 {
        self.prime.as_u32()
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

    /// `vec[i]` returns the i-th entry. `view[a:b]` returns a sub-view (same
    /// mutability as `view`). On an *owned* `FpVector`, slicing is not
    /// allowed — use `vec.const[a:b]` (read-only) or `vec.mut[a:b]`
    /// (mutable) instead.
    fn __getitem__<'py>(
        slf: Bound<'py, Self>,
        key: Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = slf.py();
        if let Ok(i) = key.extract::<usize>() {
            let r = slf.borrow();
            if i >= r.len {
                return Err(PyIndexError::new_err("FpVector index out of range"));
            }
            let val = r.with_slice(py, |s| s.entry(i))?;
            return Ok(val.into_pyobject(py)?.into_any());
        }
        if let Ok(slice) = key.cast::<PySlice>() {
            let len_ = slf.borrow().len;
            let (start, end) = resolve_slice(slice, len_)?;
            let kind = slf.borrow().kind_tag();
            let view = match kind {
                FpKindTag::Owned => {
                    return Err(PyValueError::new_err(
                        "Cannot slice an owned FpVector directly. Use \
                         `v.const[a:b]` for a read-only view or `v.mut[a:b]` \
                         for a mutable view.",
                    ));
                }
                FpKindTag::View => Self::slice_as_view(slf, start, end)?,
                FpKindTag::ViewMut => Self::slice_as_view_mut(slf, start, end)?,
            };
            return Ok(Py::new(py, view)?.into_bound(py).into_any());
        }
        Err(PyTypeError::new_err(
            "FpVector indices must be int or slice",
        ))
    }

    fn __setitem__(&mut self, py: Python<'_>, index: usize, value: u32) -> PyResult<()> {
        if index >= self.len {
            return Err(PyIndexError::new_err("FpVector index out of range"));
        }
        self.with_slice_mut(py, |mut s| s.set_entry(index, value))
    }

    /// Read-only view over the whole vector. Slicing this view (`v.const[a:b]`)
    /// returns a sub-`View`.
    #[getter(r#const)]
    fn r_const(slf: Bound<'_, Self>) -> PyResult<FpVector> {
        let len = slf.borrow().len;
        Self::slice_as_view(slf, 0, len)
    }

    /// Mutable view over the whole vector. Errors if `self` is a read-only
    /// view. Slicing this view (`v.mut[a:b]`) returns a sub-`ViewMut`.
    #[getter(r#mut)]
    fn r_mut(slf: Bound<'_, Self>) -> PyResult<FpVector> {
        let len = slf.borrow().len;
        Self::slice_as_view_mut(slf, 0, len)
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
#[pyclass(name = "Matrix", module = "sseq_ext", skip_from_py_object, weakref)]
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
    fn new(p: u32, rows: usize, columns: usize) -> PyResult<Self> {
        let p = vp_from_u32(p)?;
        Ok(Self {
            inner: M::new(p, rows, columns),
        })
    }

    /// Build a matrix from a list of rows, each a list of non-negative integers.
    #[classmethod]
    fn from_vec(_cls: &Bound<'_, PyType>, p: u32, rows: Vec<Vec<u32>>) -> PyResult<Self> {
        let p = vp_from_u32(p)?;
        Ok(Self {
            inner: M::from_vec(p, &rows),
        })
    }

    /// `(pivot_count, matrix)` where the second is `[m | I]`.
    #[classmethod]
    fn augmented_from_vec(
        _cls: &Bound<'_, PyType>,
        p: u32,
        rows: Vec<Vec<u32>>,
    ) -> PyResult<(usize, Self)> {
        let p = vp_from_u32(p)?;
        let (cols, mat) = M::augmented_from_vec(p, &rows);
        Ok((cols, Self { inner: mat }))
    }

    fn prime(&self) -> u32 {
        self.inner.prime().as_u32()
    }

    fn rows(&self) -> usize {
        self.inner.rows()
    }

    fn columns(&self) -> usize {
        self.inner.columns()
    }

    /// `m[row, col]` returns the entry at `(row, col)` as an integer.
    fn __getitem__(&self, key: (usize, usize)) -> PyResult<u32> {
        let (row, col) = key;
        if row >= self.inner.rows() {
            return Err(PyIndexError::new_err("row index out of range"));
        }
        if col >= self.inner.columns() {
            return Err(PyIndexError::new_err("column index out of range"));
        }
        Ok(self.inner.row(row).entry(col))
    }

    /// `m[row, col] = value` sets the entry at `(row, col)`.
    fn __setitem__(&mut self, key: (usize, usize), value: u32) -> PyResult<()> {
        let (row, col) = key;
        if row >= self.inner.rows() {
            return Err(PyIndexError::new_err("row index out of range"));
        }
        if col >= self.inner.columns() {
            return Err(PyIndexError::new_err("column index out of range"));
        }
        self.inner.row_mut(row).set_entry(col, value);
        Ok(())
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

    /// Read-only row accessor. `m.const[row]` returns an `FpVector` view of
    /// the row; chain `.const[row][a:b]` for a partial row.
    #[getter(r#const)]
    fn r_const(slf: Py<Self>) -> MatrixView {
        MatrixView { matrix: slf }
    }

    /// Mutable row accessor. `m.mut[row]` returns a mutable `FpVector` view.
    #[getter(r#mut)]
    fn r_mut(slf: Py<Self>) -> MatrixViewMut {
        MatrixViewMut { matrix: slf }
    }

    fn __repr__(&self) -> String {
        format!(
            "Matrix({}x{}, p={})",
            self.inner.rows(),
            self.inner.columns(),
            self.inner.prime().as_u32()
        )
    }

    /// **Test hook.** Hold a mutable Python borrow on `self` for the
    /// duration of `vec.set_entry(0, 1)`. If `vec` is a view that lives
    /// in `self`, this should raise `BufferError` because the parent is
    /// already borrowed.
    ///
    /// This is exposed solely so the safety-test suite can exercise the
    /// runtime borrow-check path without requiring re-entrant pyclass
    /// callbacks (which we never produce in normal operation).
    #[pyo3(name = "_test_op_during_self_borrow_mut")]
    fn test_op_during_self_borrow_mut(
        slf: Bound<'_, Self>,
        py: Python<'_>,
        vec: &mut FpVector,
    ) -> PyResult<()> {
        // Hold the borrow_mut for the whole call.
        let _guard = slf.try_borrow_mut().map_err(|e| {
            PyBufferError::new_err(format!("test hook: cannot borrow self: {e}"))
        })?;
        // Try a write through the view; should fail if the view's owner is
        // `self`, because the parent is already borrow_mut'd.
        vec.with_slice_mut_pub(py, |mut s| s.set_entry(0, 1))
    }
}

/// An augmented matrix `[A | B]` (or with more segments).
#[pyclass(name = "AugmentedMatrix", module = "sseq_ext", weakref)]
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
    fn new(p: u32, rows: usize, columns: Vec<usize>) -> PyResult<Self> {
        let p = vp_from_u32(p)?;
        let inner = match columns.len() {
            2 => AugmentedInner::N2(AM::<2>::new(p, rows, [columns[0], columns[1]])),
            3 => AugmentedInner::N3(AM::<3>::new(
                p,
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

    /// `am[row, col]` returns the entry at `(row, col)` (in flat-matrix
    /// coordinates) as an integer. For segment-relative access use
    /// `am.const[row, seg][col]`.
    fn __getitem__(&self, key: (usize, usize)) -> PyResult<u32> {
        let (row, col) = key;
        if row >= self.inner.rows() {
            return Err(PyIndexError::new_err("row index out of range"));
        }
        if col >= self.inner.columns() {
            return Err(PyIndexError::new_err("column index out of range"));
        }
        let val = match &self.inner {
            AugmentedInner::N2(m) => m.row(row).entry(col),
            AugmentedInner::N3(m) => m.row(row).entry(col),
        };
        Ok(val)
    }

    /// `am[row, col] = value` sets the entry at `(row, col)` in flat-matrix
    /// coordinates.
    fn __setitem__(&mut self, key: (usize, usize), value: u32) -> PyResult<()> {
        let (row, col) = key;
        if row >= self.inner.rows() {
            return Err(PyIndexError::new_err("row index out of range"));
        }
        if col >= self.inner.columns() {
            return Err(PyIndexError::new_err("column index out of range"));
        }
        match &mut self.inner {
            AugmentedInner::N2(m) => m.row_mut(row).set_entry(col, value),
            AugmentedInner::N3(m) => m.row_mut(row).set_entry(col, value),
        };
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

    /// Read-only row accessor. Use as:
    ///
    /// - `am.const[row]` — view of the entire row.
    /// - `am.const[row, seg]` — view of one segment.
    /// - `am.const[row, (start_seg, end_seg)]` — view spanning a contiguous
    ///   range of segments (inclusive on both ends).
    #[getter(r#const)]
    fn r_const(slf: Py<Self>) -> AugmentedMatrixView {
        AugmentedMatrixView { matrix: slf }
    }

    /// Mutable row accessor. Same indexing as `.const`.
    #[getter(r#mut)]
    fn r_mut(slf: Py<Self>) -> AugmentedMatrixViewMut {
        AugmentedMatrixViewMut { matrix: slf }
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

// ---------------------------------------------------------------------------
// Row accessors for Matrix and AugmentedMatrix.
//
// These are thin wrappers that exist solely to support `__getitem__` syntax:
//
//     m.const[row]        # row view of m
//     m.mut[row]          # mutable row view of m
//     am.const[row, seg]  # segment view
//
// They take a `Py<Matrix>` (or `Py<AugmentedMatrix>`) and produce
// `FpVector` views in `View` / `ViewMut` mode.
// ---------------------------------------------------------------------------

fn make_matrix_row_view(
    py: Python<'_>,
    matrix: &Py<Matrix>,
    row: usize,
    mutable: bool,
) -> PyResult<FpVector> {
    let bound = matrix.bind(py);
    let (rows, columns, prime) = {
        let r = bound.borrow();
        (r.inner.rows(), r.inner.columns(), r.inner.prime())
    };
    if row >= rows {
        return Err(PyIndexError::new_err("row index out of range"));
    }
    let ptr = bound.borrow_mut().raw_ptr();
    let owner = ViewOwner::Matrix(matrix.clone_ref(py));
    let source = ViewSource::MatRow {
        ptr,
        row,
        start: 0,
        end: columns,
    };
    Ok(FpVector {
        kind: if mutable {
            FpVectorKind::ViewMut { owner, source }
        } else {
            FpVectorKind::View { owner, source }
        },
        prime,
        len: columns,
    })
}

/// Read-only row accessor. Created by `Matrix.const`. Index with `[row]`
/// to get an `FpVector` view of the row.
#[pyclass(name = "MatrixView", module = "sseq_ext", weakref)]
pub struct MatrixView {
    matrix: Py<Matrix>,
}

#[pymethods]
impl MatrixView {
    fn __getitem__(&self, py: Python<'_>, row: usize) -> PyResult<FpVector> {
        make_matrix_row_view(py, &self.matrix, row, false)
    }

    fn __len__(&self, py: Python<'_>) -> usize {
        self.matrix.bind(py).borrow().inner.rows()
    }

    fn __repr__(&self) -> String {
        "MatrixView(<read-only>)".to_owned()
    }
}

/// Mutable row accessor. Created by `Matrix.mut`. Index with `[row]` to get
/// a mutable `FpVector` view.
#[pyclass(name = "MatrixViewMut", module = "sseq_ext", weakref)]
pub struct MatrixViewMut {
    matrix: Py<Matrix>,
}

#[pymethods]
impl MatrixViewMut {
    fn __getitem__(&self, py: Python<'_>, row: usize) -> PyResult<FpVector> {
        make_matrix_row_view(py, &self.matrix, row, true)
    }

    fn __len__(&self, py: Python<'_>) -> usize {
        self.matrix.bind(py).borrow().inner.rows()
    }

    fn __repr__(&self) -> String {
        "MatrixViewMut(<mutable>)".to_owned()
    }
}

/// Resolve the `seg` index of an `AugmentedMatrix` accessor key. The key is
/// either a single integer (single segment) or a 2-tuple `(start_seg,
/// end_seg)` (inclusive-inclusive range).
fn resolve_seg_key(seg: &Bound<'_, PyAny>) -> PyResult<(usize, usize)> {
    if let Ok(s) = seg.extract::<usize>() {
        return Ok((s, s));
    }
    if let Ok((start_seg, end_seg)) = seg.extract::<(usize, usize)>() {
        return Ok((start_seg, end_seg));
    }
    Err(PyTypeError::new_err(
        "AugmentedMatrix segment index must be int or (int, int)",
    ))
}

fn make_augmented_row_view(
    py: Python<'_>,
    matrix: &Py<AugmentedMatrix>,
    row: usize,
    seg_range: Option<(usize, usize)>,
    mutable: bool,
) -> PyResult<FpVector> {
    let bound = matrix.bind(py);
    let (rows, prime, start, end) = {
        let r = bound.borrow();
        match seg_range {
            None => (
                r.inner.rows(),
                r.inner.prime(),
                0,
                r.inner.columns(),
            ),
            Some((start_seg, end_seg)) => {
                let (s, e) = r.inner.segment_range(start_seg, end_seg);
                (r.inner.rows(), r.inner.prime(), s, e)
            }
        }
    };
    if row >= rows {
        return Err(PyIndexError::new_err("row index out of range"));
    }
    let ptr = bound.borrow_mut().inner.matrix_ptr();
    let owner = ViewOwner::AugmentedMatrix(matrix.clone_ref(py));
    let source = ViewSource::MatRow {
        ptr,
        row,
        start,
        end,
    };
    Ok(FpVector {
        kind: if mutable {
            FpVectorKind::ViewMut { owner, source }
        } else {
            FpVectorKind::View { owner, source }
        },
        prime,
        len: end - start,
    })
}

/// Read-only accessor for `AugmentedMatrix`. Created by `AugmentedMatrix.const`.
#[pyclass(name = "AugmentedMatrixView", module = "sseq_ext", weakref)]
pub struct AugmentedMatrixView {
    matrix: Py<AugmentedMatrix>,
}

#[pymethods]
impl AugmentedMatrixView {
    fn __getitem__<'py>(
        &self,
        py: Python<'py>,
        key: Bound<'py, PyAny>,
    ) -> PyResult<FpVector> {
        if let Ok(row) = key.extract::<usize>() {
            return make_augmented_row_view(py, &self.matrix, row, None, false);
        }
        // (row, seg) — where seg is an int or (start_seg, end_seg).
        if let Ok((row, seg)) = key.extract::<(usize, Bound<'py, PyAny>)>() {
            let seg_range = resolve_seg_key(&seg)?;
            return make_augmented_row_view(py, &self.matrix, row, Some(seg_range), false);
        }
        Err(PyTypeError::new_err(
            "AugmentedMatrix index must be `row` or `(row, seg)` or `(row, (start, end))`",
        ))
    }

    fn __len__(&self, py: Python<'_>) -> usize {
        self.matrix.bind(py).borrow().inner.rows()
    }

    fn __repr__(&self) -> String {
        "AugmentedMatrixView(<read-only>)".to_owned()
    }
}

/// Mutable accessor for `AugmentedMatrix`. Created by `AugmentedMatrix.mut`.
#[pyclass(name = "AugmentedMatrixViewMut", module = "sseq_ext", weakref)]
pub struct AugmentedMatrixViewMut {
    matrix: Py<AugmentedMatrix>,
}

#[pymethods]
impl AugmentedMatrixViewMut {
    fn __getitem__<'py>(
        &self,
        py: Python<'py>,
        key: Bound<'py, PyAny>,
    ) -> PyResult<FpVector> {
        if let Ok(row) = key.extract::<usize>() {
            return make_augmented_row_view(py, &self.matrix, row, None, true);
        }
        if let Ok((row, seg)) = key.extract::<(usize, Bound<'py, PyAny>)>() {
            let seg_range = resolve_seg_key(&seg)?;
            return make_augmented_row_view(py, &self.matrix, row, Some(seg_range), true);
        }
        Err(PyTypeError::new_err(
            "AugmentedMatrix index must be `row` or `(row, seg)` or `(row, (start, end))`",
        ))
    }

    fn __len__(&self, py: Python<'_>) -> usize {
        self.matrix.bind(py).borrow().inner.rows()
    }

    fn __repr__(&self) -> String {
        "AugmentedMatrixViewMut(<mutable>)".to_owned()
    }
}
