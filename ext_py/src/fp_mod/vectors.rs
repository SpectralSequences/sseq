use std::io::Cursor;

use fp::{
    prime::Prime,
    vector::{FpSlice as RustFpSlice, FpSliceMut as RustFpSliceMut, FpVector as RustFpVector},
};
use pyo3::{exceptions::PyValueError, types::PyBytes};

use super::*;

/// Length of a slice-like operand argument (`FpVector` or `FpSlice`),
/// computed without retaining a borrow. Used for pre-borrow dimension
/// checks by the `FpSliceMut` operand-taking methods, which accept either
/// an `FpVector` or an `FpSlice`.
pub(crate) fn slice_like_len(operand: &Bound<'_, PyAny>) -> PyResult<usize> {
    if let Ok(slice) = operand.extract::<PyRef<'_, PyFpSlice>>() {
        Ok(slice.span())
    } else if let Ok(vector) = operand.extract::<PyRef<'_, PyFpVector>>() {
        Ok(vector.0.len())
    } else {
        Err(PyValueError::new_err("expected an FpVector or FpSlice"))
    }
}

/// Run `f` on the reconstructed immutable slice for `parent[start..end]`,
/// after revalidating the parent's current dimensions.
///
/// Revalidation only guards the parent's current *dimensions* (vector length
/// or matrix row count and row length). It deliberately does not track
/// logical-coordinate remapping: an operation like `Matrix::trim` with
/// `col_start > 0` shifts the data backwards in each row without shrinking it
/// below the slice's `end`, so a surviving slice silently reads the remapped
/// columns rather than raising. Preventing that would require tracking the
/// origin of every coordinate, which is out of scope for the
/// handle+range design.
pub(crate) fn with_parent_slice<R>(
    parent: &SliceParent,
    start: usize,
    end: usize,
    py: Python<'_>,
    f: impl FnOnce(RustFpSlice<'_>) -> R,
) -> PyResult<R> {
    match parent {
        SliceParent::Vector(v) => {
            let parent = v.try_borrow(py).map_err(borrow_error)?;
            checked_range(start, end, parent.0.len())?;
            Ok(f(parent.0.slice(start, end)))
        }
        SliceParent::MatrixRow { matrix, row } => matrix.with_matrix(py, |m| {
            checked_row(*row, m.rows())?;
            let full = m.row(*row);
            checked_range(start, end, full.len())?;
            Ok(f(full.restrict(start, end)))
        })?,
    }
}

/// Run `f` on the reconstructed mutable slice for `parent[start..end]`,
/// after revalidating the parent's current dimensions.
pub(crate) fn with_parent_slice_mut<R>(
    parent: &SliceParent,
    start: usize,
    end: usize,
    py: Python<'_>,
    f: impl FnOnce(RustFpSliceMut<'_>) -> R,
) -> PyResult<R> {
    match parent {
        SliceParent::Vector(v) => {
            let mut parent = v.try_borrow_mut(py).map_err(borrow_error)?;
            checked_range(start, end, parent.0.len())?;
            Ok(f(parent.0.slice_mut(start, end)))
        }
        SliceParent::MatrixRow { matrix, row } => matrix.with_matrix_mut(py, |m| {
            checked_row(*row, m.rows())?;
            // Validate against the actual current row length, matching the
            // read path (`with_parent_slice`). For a `Matrix` this equals
            // `columns()`, but deriving it from the row keeps both paths
            // consistent regardless of that invariant.
            let row_len = m.row(*row).len();
            checked_range(start, end, row_len)?;
            Ok(f(m.row_mut(*row).slice_mut(start, end)))
        })?,
    }
}

pub(crate) fn checked_index(index: usize, len: usize) -> PyResult<usize> {
    if index < len {
        Ok(index)
    } else {
        Err(PyIndexError::new_err(format!(
            "index {index} out of range for vector of length {len}"
        )))
    }
}

pub(crate) fn py_index(index: isize, len: usize) -> PyResult<usize> {
    let index = if index < 0 {
        len as isize + index
    } else {
        index
    };
    if index >= 0 && (index as usize) < len {
        Ok(index as usize)
    } else {
        Err(PyIndexError::new_err(format!(
            "index {index} out of range for vector of length {len}"
        )))
    }
}

#[pyclass(name = "FpVector")]
pub struct PyFpVector(pub(crate) RustFpVector);

#[pymethods]
impl PyFpVector {
    #[new]
    pub fn new(p: u32, len: usize) -> PyResult<Self> {
        Ok(Self(RustFpVector::new(valid_prime(p)?, len)))
    }

    /// Construct a new zero vector of length `len` over `F_p`. Static-method
    /// alias for the constructor, allowing `FpVector.new(p, len)`.
    #[staticmethod]
    #[pyo3(name = "new")]
    pub fn new_static(p: u32, len: usize) -> PyResult<Self> {
        Self::new(p, len)
    }

    #[staticmethod]
    pub fn new_with_capacity(p: u32, len: usize, capacity: usize) -> PyResult<Self> {
        Ok(Self(RustFpVector::new_with_capacity(
            valid_prime(p)?,
            len,
            capacity,
        )))
    }

    #[staticmethod]
    pub fn from_py(p: u32, entries: Vec<u32>) -> PyResult<Self> {
        Ok(Self(RustFpVector::from_slice(valid_prime(p)?, &entries)))
    }

    #[staticmethod]
    pub fn from_bytes(p: u32, len: usize, data: &[u8]) -> PyResult<Self> {
        RustFpVector::from_bytes(valid_prime(p)?, len, &mut Cursor::new(data))
            .map(Self)
            .map_err(io_err)
    }

    #[getter]
    pub fn prime(&self) -> u32 {
        self.0.prime().as_u32()
    }

    #[getter]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[getter]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn entry(&self, index: usize) -> PyResult<u32> {
        Ok(self.0.entry(checked_index(index, self.0.len())?))
    }

    #[getter]
    pub fn density(&self) -> f32 {
        self.0.density()
    }

    #[getter]
    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    #[getter]
    pub fn first_nonzero(&self) -> Option<(usize, u32)> {
        self.0.first_nonzero()
    }

    pub fn iter_nonzero(&self) -> Vec<(usize, u32)> {
        self.0.as_slice().iter_nonzero().collect()
    }

    pub fn slice(slf: PyRef<'_, Self>, start: usize, end: usize) -> PyResult<PyFpSlice> {
        checked_range(start, end, slf.0.len())?;
        let py = slf.py();
        Ok(PyFpSlice {
            parent: SliceParent::Vector(slf.into_pyobject(py)?.unbind()),
            start,
            end,
        })
    }

    /// Restrict to the sub-range of coordinates `[start, end)`, returning a
    /// read-only `FpSlice` view. For an `FpVector` (whose coordinates start
    /// at 0) this mirrors `slice(start, end)`; named to match the analogous
    /// `FpSlice.restrict` method.
    pub fn restrict(slf: PyRef<'_, Self>, start: usize, end: usize) -> PyResult<PyFpSlice> {
        checked_range(start, end, slf.0.len())?;
        let py = slf.py();
        Ok(PyFpSlice {
            parent: SliceParent::Vector(slf.into_pyobject(py)?.unbind()),
            start,
            end,
        })
    }

    pub fn slice_mut(slf: PyRef<'_, Self>, start: usize, end: usize) -> PyResult<PyFpSliceMut> {
        checked_range(start, end, slf.0.len())?;
        let py = slf.py();
        Ok(PyFpSliceMut {
            parent: SliceParent::Vector(slf.into_pyobject(py)?.unbind()),
            start,
            end,
        })
    }

    /// A read-only `FpSlice` spanning the whole vector; equivalent to
    /// `slice(0, len())`.
    #[getter]
    pub fn r#const(slf: PyRef<'_, Self>) -> PyResult<PyFpSlice> {
        let end = slf.0.len();
        let py = slf.py();
        Ok(PyFpSlice {
            parent: SliceParent::Vector(slf.into_pyobject(py)?.unbind()),
            start: 0,
            end,
        })
    }

    /// A mutable `FpSliceMut` spanning the whole vector; equivalent to
    /// `slice_mut(0, len())`.
    #[getter]
    pub fn r#mut(slf: PyRef<'_, Self>) -> PyResult<PyFpSliceMut> {
        let end = slf.0.len();
        let py = slf.py();
        Ok(PyFpSliceMut {
            parent: SliceParent::Vector(slf.into_pyobject(py)?.unbind()),
            start: 0,
            end,
        })
    }

    pub fn set_entry(&mut self, index: usize, value: u32) -> PyResult<()> {
        self.0.set_entry(checked_index(index, self.0.len())?, value);
        Ok(())
    }

    pub fn scale(&mut self, c: u32) {
        self.0.scale(c)
    }

    pub fn set_to_zero(&mut self) {
        self.0.set_to_zero()
    }

    pub fn add_basis_element(&mut self, index: usize, value: u32) -> PyResult<()> {
        self.0
            .add_basis_element(checked_index(index, self.0.len())?, value);
        Ok(())
    }

    pub fn extend_len(&mut self, len: usize) {
        self.0.extend_len(len)
    }

    pub fn set_scratch_vector_size(&mut self, len: usize) {
        self.0.set_scratch_vector_size(len)
    }

    pub fn to_bytes<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        serialize_to_pybytes(py, |buffer| self.0.to_bytes(buffer))
    }

    pub fn update_from_bytes(&mut self, data: &[u8]) -> PyResult<()> {
        self.0
            .update_from_bytes(&mut Cursor::new(data))
            .map_err(io_err)
    }

    pub fn __len__(&self) -> usize {
        self.0.len()
    }

    pub fn __getitem__(&self, index: isize) -> PyResult<u32> {
        Ok(self.0.entry(py_index(index, self.0.len())?))
    }

    pub fn __setitem__(&mut self, index: isize, value: u32) -> PyResult<()> {
        self.0.set_entry(py_index(index, self.0.len())?, value);
        Ok(())
    }

    pub fn __iter__(slf: PyRef<'_, Self>) -> PyFpVectorIterator {
        PyFpVectorIterator {
            entries: slf.0.iter().collect(),
            index: 0,
        }
    }

    /// Return an owned clone of this vector. Mirrors `FpSlice.to_owned`,
    /// allowing `.to_owned()` to be used uniformly on both vectors and
    /// slices.
    pub fn to_owned(&self) -> PyFpVector {
        Self(self.0.clone())
    }

    pub fn __repr__(&self) -> String {
        format!("FpVector({}, {})", self.prime(), self.0)
    }
}

#[pyclass(name = "FpSlice")]
pub struct PyFpSlice {
    pub(crate) parent: SliceParent,
    pub(crate) start: usize,
    pub(crate) end: usize,
}

impl PyFpSlice {
    fn with_slice<R>(&self, py: Python<'_>, f: impl FnOnce(RustFpSlice<'_>) -> R) -> PyResult<R> {
        with_parent_slice(&self.parent, self.start, self.end, py, f)
    }

    /// Cached span of the handle, used only for computing index bounds.
    /// This does NOT revalidate the parent; callers that touch the parent
    /// go through `with_slice`/`with_slice_mut`, which revalidate.
    fn span(&self) -> usize {
        self.end - self.start
    }

    fn to_owned_checked(&self, py: Python<'_>) -> PyResult<RustFpVector> {
        self.with_slice(py, |s| s.to_owned())
    }
}

#[pymethods]
impl PyFpSlice {
    #[getter]
    pub fn prime(&self, py: Python<'_>) -> PyResult<u32> {
        self.with_slice(py, |s| s.prime().as_u32())
    }

    #[getter]
    pub fn len(&self, py: Python<'_>) -> PyResult<usize> {
        self.with_slice(py, |s| s.len())
    }

    #[getter]
    pub fn is_empty(&self, py: Python<'_>) -> PyResult<bool> {
        self.with_slice(py, |s| s.is_empty())
    }

    pub fn entry(&self, py: Python<'_>, index: usize) -> PyResult<u32> {
        let index = checked_index(index, self.span())?;
        self.with_slice(py, |s| s.entry(index))
    }

    pub fn iter_nonzero(&self, py: Python<'_>) -> PyResult<Vec<(usize, u32)>> {
        self.with_slice(py, |s| s.iter_nonzero().collect())
    }

    #[getter]
    pub fn is_zero(&self, py: Python<'_>) -> PyResult<bool> {
        self.with_slice(py, |s| s.is_zero())
    }

    #[getter]
    pub fn first_nonzero(&self, py: Python<'_>) -> PyResult<Option<(usize, u32)>> {
        self.with_slice(py, |s| s.first_nonzero())
    }

    pub fn restrict(&self, py: Python<'_>, start: usize, end: usize) -> PyResult<Self> {
        checked_range(start, end, self.span())?;
        Ok(Self {
            parent: self.parent.clone_ref(py),
            start: self.start + start,
            end: self.start + end,
        })
    }

    pub fn to_owned(&self, py: Python<'_>) -> PyResult<PyFpVector> {
        Ok(PyFpVector(self.to_owned_checked(py)?))
    }

    pub fn __len__(&self, py: Python<'_>) -> PyResult<usize> {
        self.len(py)
    }

    pub fn __getitem__(&self, py: Python<'_>, index: isize) -> PyResult<u32> {
        let index = py_index(index, self.span())?;
        self.with_slice(py, |s| s.entry(index))
    }

    pub fn __iter__(&self, py: Python<'_>) -> PyResult<PyFpVectorIterator> {
        let entries = self.with_slice(py, |s| s.iter().collect())?;
        Ok(PyFpVectorIterator { entries, index: 0 })
    }

    pub fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        self.with_slice(py, |s| format!("FpSlice({}, {})", s.prime().as_u32(), s))
    }
}

#[pyclass(name = "FpSliceMut")]
pub struct PyFpSliceMut {
    pub(crate) parent: SliceParent,
    pub(crate) start: usize,
    pub(crate) end: usize,
}

impl PyFpSliceMut {
    fn with_slice<R>(&self, py: Python<'_>, f: impl FnOnce(RustFpSlice<'_>) -> R) -> PyResult<R> {
        with_parent_slice(&self.parent, self.start, self.end, py, f)
    }

    /// Cached span of the handle, used only for computing index bounds.
    /// This does NOT revalidate the parent; callers that touch the parent
    /// go through `with_slice`/`with_slice_mut`, which revalidate.
    fn span(&self) -> usize {
        self.end - self.start
    }

    fn with_slice_mut<R>(
        &self,
        py: Python<'_>,
        f: impl FnOnce(RustFpSliceMut<'_>) -> R,
    ) -> PyResult<R> {
        with_parent_slice_mut(&self.parent, self.start, self.end, py, f)
    }

    /// Provide an immutable view of `operand` to `f`, avoiding a deep clone
    /// in the common case. If `operand` is backed by a *different* Python
    /// object from `self`, it is borrowed transiently (no clone) so that the
    /// caller can hold this shared borrow alongside `self`'s mutable borrow.
    /// Only when they share a backing object (genuine aliasing) does this
    /// fall back to an owned copy, sidestepping the PyO3 double-borrow.
    fn with_operand<R>(
        &self,
        py: Python<'_>,
        operand: &PyFpSlice,
        f: impl FnOnce(RustFpSlice<'_>) -> PyResult<R>,
    ) -> PyResult<R> {
        if self.parent.same_object(py, &operand.parent) {
            let owned = operand.to_owned_checked(py)?;
            f(owned.as_slice())
        } else {
            operand.with_slice(py, f)?
        }
    }

    /// Like [`with_operand`], but accepts an operand that is either an
    /// `FpSlice` or a (full) `FpVector`. An `FpSlice` is routed through
    /// [`with_operand`] (preserving its clone-on-alias handling). An
    /// `FpVector` is borrowed transiently and viewed via its full-vector
    /// slice, falling back to an owned copy only when it shares its backing
    /// Python object with the target (genuine aliasing), matching the
    /// `FpSlice` path's behavior.
    fn with_operand_any<R>(
        &self,
        py: Python<'_>,
        operand: &Bound<'_, PyAny>,
        f: impl FnOnce(RustFpSlice<'_>) -> PyResult<R>,
    ) -> PyResult<R> {
        if let Ok(slice) = operand.extract::<PyRef<'_, PyFpSlice>>() {
            self.with_operand(py, &slice, f)
        } else if let Ok(vector) = operand.cast::<PyFpVector>() {
            if self.parent.same_vector(py, vector) {
                let owned = vector.try_borrow().map_err(borrow_error)?.0.clone();
                f(owned.as_slice())
            } else {
                let vector = vector.try_borrow().map_err(borrow_error)?;
                f(vector.0.as_slice())
            }
        } else {
            Err(PyValueError::new_err("expected an FpVector or FpSlice"))
        }
    }

    /// Shared "sandwich" for single-operand binary ops: borrow `other`
    /// (an `FpSlice` or full `FpVector`, via `with_operand_any`), then
    /// borrow `self` mutably (via `with_slice_mut`), verify the primes
    /// match, and run `f` with the mutable target and the operand slice.
    /// Methods with extra pre-checks (length/offset/mask/span) keep those
    /// in front of this call.
    fn with_binary_op<R>(
        &self,
        py: Python<'_>,
        other: &Bound<'_, PyAny>,
        f: impl FnOnce(RustFpSliceMut<'_>, RustFpSlice<'_>) -> R,
    ) -> PyResult<R> {
        self.with_operand_any(py, other, |o| {
            self.with_slice_mut(py, |t| {
                checked_same_prime(t.prime().as_u32(), o.prime().as_u32())?;
                Ok(f(t, o))
            })?
        })
    }
}

#[pymethods]
impl PyFpSliceMut {
    #[getter]
    pub fn prime(&self, py: Python<'_>) -> PyResult<u32> {
        self.with_slice(py, |s| s.prime().as_u32())
    }

    #[getter]
    pub fn len(&self, py: Python<'_>) -> PyResult<usize> {
        self.with_slice(py, |s| s.len())
    }

    #[getter]
    pub fn is_empty(&self, py: Python<'_>) -> PyResult<bool> {
        self.with_slice(py, |s| s.is_empty())
    }

    pub fn set_entry(&self, py: Python<'_>, index: usize, value: u32) -> PyResult<()> {
        let index = checked_index(index, self.span())?;
        self.with_slice_mut(py, |mut s| s.set_entry(index, value))
    }

    pub fn set_to_zero(&self, py: Python<'_>) -> PyResult<()> {
        self.with_slice_mut(py, |mut s| s.set_to_zero())
    }

    pub fn scale(&self, py: Python<'_>, c: u32) -> PyResult<()> {
        self.with_slice_mut(py, |mut s| s.scale(c))
    }

    pub fn add(&self, py: Python<'_>, other: &Bound<'_, PyAny>, c: u32) -> PyResult<()> {
        checked_equal_len(self.span(), slice_like_len(other)?)?;
        self.with_binary_op(py, other, |mut target, other_slice| {
            target.add(other_slice, c);
        })
    }

    pub fn assign(&self, py: Python<'_>, other: &Bound<'_, PyAny>) -> PyResult<()> {
        checked_equal_len(self.span(), slice_like_len(other)?)?;
        self.with_binary_op(py, other, |mut target, other_slice| {
            target.assign(other_slice);
        })
    }

    pub fn add_tensor(
        &self,
        py: Python<'_>,
        offset: usize,
        coeff: u32,
        left: &Bound<'_, PyAny>,
        right: &Bound<'_, PyAny>,
    ) -> PyResult<()> {
        let width = slice_like_len(left)?
            .checked_mul(slice_like_len(right)?)
            .and_then(|width| offset.checked_add(width))
            .ok_or_else(|| PyIndexError::new_err("tensor range overflows usize"))?;
        checked_range(offset, width, self.span())?;
        // Borrow each operand transiently, falling back to an owned copy
        // only for one that shares a backing object with the target. Two
        // shared borrows coexist fine; only the target's mutable borrow can
        // collide with an operand that aliases it.
        self.with_operand_any(py, left, |left_slice| {
            self.with_operand_any(py, right, |right_slice| {
                self.with_slice_mut(py, |mut target| {
                    checked_same_prime(target.prime().as_u32(), left_slice.prime().as_u32())?;
                    checked_same_prime(target.prime().as_u32(), right_slice.prime().as_u32())?;
                    target.add_tensor(offset, coeff, left_slice, right_slice);
                    Ok(())
                })?
            })
        })
    }

    pub fn add_basis_element(&self, py: Python<'_>, index: usize, value: u32) -> PyResult<()> {
        let index = checked_index(index, self.span())?;
        self.with_slice_mut(py, |mut s| s.add_basis_element(index, value))
    }

    pub fn slice_mut(&self, py: Python<'_>, start: usize, end: usize) -> PyResult<Self> {
        checked_range(start, end, self.span())?;
        Ok(Self {
            parent: self.parent.clone_ref(py),
            start: self.start + start,
            end: self.start + end,
        })
    }

    pub fn to_owned(&self, py: Python<'_>) -> PyResult<PyFpVector> {
        Ok(PyFpVector(self.with_slice(py, |s| s.to_owned())?))
    }

    pub fn __len__(&self, py: Python<'_>) -> PyResult<usize> {
        self.len(py)
    }

    pub fn __getitem__(&self, py: Python<'_>, index: isize) -> PyResult<u32> {
        let index = py_index(index, self.span())?;
        self.with_slice(py, |s| s.entry(index))
    }

    pub fn __setitem__(&self, py: Python<'_>, index: isize, value: u32) -> PyResult<()> {
        let index = py_index(index, self.span())?;
        self.with_slice_mut(py, |mut s| s.set_entry(index, value))
    }

    pub fn __iter__(&self, py: Python<'_>) -> PyResult<PyFpVectorIterator> {
        let entries = self.with_slice(py, |s| s.iter().collect())?;
        Ok(PyFpVectorIterator { entries, index: 0 })
    }

    pub fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        self.with_slice(py, |s| format!("FpSliceMut({}, {})", s.prime().as_u32(), s))
    }
}

#[pyclass(name = "FpVectorIterator")]
pub struct PyFpVectorIterator {
    entries: Vec<u32>,
    index: usize,
}

#[pymethods]
impl PyFpVectorIterator {
    pub fn __iter__(slf: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> {
        slf
    }

    pub fn __next__(&mut self) -> Option<u32> {
        let value = self.entries.get(self.index).copied();
        self.index += usize::from(value.is_some());
        value
    }
}

/// Run `f` on a borrowed immutable slice over a vector-like argument
/// (`FpVector` or `FpSlice`), holding the shared borrow only for the
/// duration of the call. This is the read-only sibling of
/// [`with_target_slice_mut`]: it avoids the deep `FpVector` clone that
/// [`extract_input_owned`] performs for every immutable input argument.
///
/// The transient borrow surfaces as a PyO3 borrow conflict (`RuntimeError`)
/// if the same object is simultaneously borrowed mutably elsewhere — e.g.
/// passed as both the input and the mutable target — rather than UB.
///
/// # Error taxonomy
///
/// We first dispatch on the object's *type* and only then attempt the
/// borrow, so the two failure modes stay distinct:
///  * a genuinely wrong type → `ValueError("expected an FpVector or
///    FpSlice")`, and
///  * a correct type that is already borrowed mutably elsewhere (aliasing)
///    → the borrow conflict is propagated verbatim as `RuntimeError`.
///
/// Exposed `pub(crate)` so that other binding modules (e.g. `algebra_py`)
/// reuse it for immutable input element arguments.
pub(crate) fn with_input_slice<R>(
    py: Python<'_>,
    obj: &Bound<'_, PyAny>,
    f: impl FnOnce(RustFpSlice<'_>) -> PyResult<R>,
) -> PyResult<R> {
    if let Ok(vector) = obj.cast::<PyFpVector>() {
        let vector = vector.try_borrow().map_err(borrow_error)?;
        f(vector.0.as_slice())
    } else if let Ok(slice) = obj.cast::<PyFpSlice>() {
        let slice = slice.try_borrow().map_err(borrow_error)?;
        slice.with_slice(py, f)?
    } else {
        Err(PyValueError::new_err("expected an FpVector or FpSlice"))
    }
}

/// Run `f` on the mutable slice backing a vector-like argument
/// (`FpVector` or `FpSliceMut`), used as an output target.
///
/// # Error taxonomy
///
/// We dispatch on the object's *type* before attempting the mutable
/// borrow, so the two failure modes stay distinct:
///  * a genuinely wrong type → `ValueError("expected an FpVector or
///    FpSliceMut")`, and
///  * a correct type that is already borrowed elsewhere (e.g. the same
///    `FpVector` simultaneously passed as a borrowed input via
///    [`with_input_slice`] *and* as this mutable target) → the borrow
///    conflict is propagated verbatim as `RuntimeError`.
///
/// Aliasing the mutable target with an input is therefore rejected with a
/// `RuntimeError` (an intentional API change from the pre-clone-removal
/// behavior, which silently succeeded by cloning the input first).
///
/// Exposed `pub(crate)` so that other binding modules (e.g. `algebra_py`)
/// can accept a bound `fp_py` result argument for the `multiply_*` family;
/// the closure receives the reconstructed `FpSliceMut` and may return a
/// `PyResult` so callers can pre-validate (prime/length) inside the borrow.
pub(crate) fn with_target_slice_mut<R>(
    py: Python<'_>,
    obj: &Bound<'_, PyAny>,
    f: impl FnOnce(RustFpSliceMut<'_>) -> PyResult<R>,
) -> PyResult<R> {
    if let Ok(vector) = obj.cast::<PyFpVector>() {
        let mut vector = vector.try_borrow_mut().map_err(borrow_error)?;
        f(vector.0.as_slice_mut())
    } else if let Ok(slice) = obj.cast::<PyFpSliceMut>() {
        let slice = slice.try_borrow().map_err(borrow_error)?;
        slice.with_slice_mut(py, f)?
    } else {
        Err(PyValueError::new_err("expected an FpVector or FpSliceMut"))
    }
}
