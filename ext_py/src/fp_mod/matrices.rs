use std::io::Cursor;

use fp::{
    matrix::{
        AffineSubspace as RustAffineSubspace, AugmentedMatrix as RustAugmentedMatrix,
        Matrix as RustMatrix, MatrixSliceMut as RustMatrixSliceMut,
        QuasiInverse as RustQuasiInverse, Subquotient as RustSubquotient, Subspace as RustSubspace,
    },
    prime::Prime,
    vector::FpVector as RustFpVector,
};
use pyo3::{
    exceptions::{PyIndexError, PyValueError},
    types::PyBytes,
};

use super::{
    vectors::{with_input_slice, with_target_slice_mut, PyFpSlice, PyFpSliceMut, PyFpVector},
    *,
};

#[pyclass(name = "MatrixSliceMut")]
pub struct PyMatrixSliceMut {
    pub(crate) parent: MatrixParent,
    row_start: usize,
    row_end: usize,
    col_start: usize,
    col_end: usize,
}

#[pyclass(name = "Matrix")]
pub struct PyMatrix(pub(crate) RustMatrix);

#[pyclass(name = "Subspace")]
pub struct PySubspace(RustSubspace);

#[pyclass(name = "QuasiInverse")]
pub struct PyQuasiInverse(RustQuasiInverse);

#[pyclass(name = "Subquotient")]
pub struct PySubquotient(RustSubquotient);

#[pyclass(name = "AffineSubspace")]
pub struct PyAffineSubspace(RustAffineSubspace);

/// Lazy iterator over every vector in a subspace.
///
/// The upstream `Subspace::iter_all_vectors` iterator borrows the subspace,
/// so it cannot be stored alongside an owned subspace in a `#[pyclass]`
/// without a self-referential struct. Instead we own a clone of the
/// subspace and an index counter, regenerating the i-th vector on each
/// `__next__` from the base-`p` decomposition of the index.
#[pyclass(name = "SubspaceVectorIterator")]
pub struct PySubspaceVectorIterator {
    subspace: RustSubspace,
    index: u128,
    total: u128,
}

/// Validate a `row_start..row_end` x `col_start..col_end` rectangle against
/// a matrix's current `rows` x `columns`, raising `IndexError` otherwise.
pub(crate) fn checked_rect(
    row_start: usize,
    row_end: usize,
    col_start: usize,
    col_end: usize,
    rows: usize,
    columns: usize,
) -> PyResult<()> {
    if row_start <= row_end && row_end <= rows && col_start <= col_end && col_end <= columns {
        Ok(())
    } else {
        Err(PyIndexError::new_err(format!(
            "rectangle [{row_start}..{row_end}] x [{col_start}..{col_end}] out of range for \
             matrix with {rows} rows and {columns} columns"
        )))
    }
}

/// Validate that `seg` is a segment index in `0..n`.
fn checked_segment(seg: usize, n: usize) -> PyResult<()> {
    if seg < n {
        Ok(())
    } else {
        Err(PyIndexError::new_err(format!(
            "segment {seg} out of range for {n} segments"
        )))
    }
}

/// Validate a `[start, end]` segment-index range against an augmented
/// matrix and return the width (column count) of the spanned rectangle.
fn segment_cols<const N: usize>(
    m: &RustAugmentedMatrix<N>,
    start: usize,
    end: usize,
) -> PyResult<usize> {
    checked_segment(start, N)?;
    checked_segment(end, N)?;
    let lo = m.start[start];
    let hi = m.end[end];
    if lo > hi {
        return Err(PyValueError::new_err(format!(
            "segment range [{start}, {end}] is empty or inverted"
        )));
    }
    Ok(hi - lo)
}

impl PyMatrixSliceMut {
    /// Number of rows spanned by the rectangle (cached; `with_slice_mut`
    /// revalidates against the parent before any data access).
    fn rows_span(&self) -> usize {
        self.row_end - self.row_start
    }

    /// Number of columns spanned by the rectangle (cached; see `rows_span`).
    fn cols_span(&self) -> usize {
        self.col_end - self.col_start
    }

    /// Run `f` on the reconstructed `MatrixSliceMut`, after revalidating the
    /// rectangle against the parent's current dimensions.
    fn with_slice_mut<R>(
        &self,
        py: Python<'_>,
        f: impl FnOnce(RustMatrixSliceMut<'_>) -> R,
    ) -> PyResult<R> {
        self.parent.with_matrix_mut(py, |m| {
            checked_rect(
                self.row_start,
                self.row_end,
                self.col_start,
                self.col_end,
                m.rows(),
                m.columns(),
            )?;
            Ok(f(m.slice_mut(
                self.row_start,
                self.row_end,
                self.col_start,
                self.col_end,
            )))
        })?
    }
}

#[pymethods]
impl PyMatrixSliceMut {
    #[getter]
    pub fn prime(&self, py: Python<'_>) -> PyResult<u32> {
        self.with_slice_mut(py, |s| s.prime().as_u32())
    }

    #[getter]
    pub fn rows(&self, py: Python<'_>) -> PyResult<usize> {
        self.with_slice_mut(py, |s| s.rows())
    }

    #[getter]
    pub fn columns(&self, py: Python<'_>) -> PyResult<usize> {
        self.with_slice_mut(py, |s| s.columns())
    }

    /// Return an immutable `FpSlice` over row `i` of the rectangle. The handle
    /// revalidates against the parent on use.
    pub fn row(&self, py: Python<'_>, i: usize) -> PyResult<PyFpSlice> {
        let row = checked_row(i, self.rows_span())? + self.row_start;
        Ok(PyFpSlice {
            parent: SliceParent::MatrixRow {
                matrix: self.parent.clone_ref(py),
                row,
            },
            start: self.col_start,
            end: self.col_end,
        })
    }

    /// Return a mutable `FpSliceMut` over row `i` of the rectangle; mutating
    /// it writes through to the parent matrix.
    pub fn row_mut(&self, py: Python<'_>, i: usize) -> PyResult<PyFpSliceMut> {
        let row = checked_row(i, self.rows_span())? + self.row_start;
        Ok(PyFpSliceMut {
            parent: SliceParent::MatrixRow {
                matrix: self.parent.clone_ref(py),
                row,
            },
            start: self.col_start,
            end: self.col_end,
        })
    }

    /// Restrict the view to rows `row_start..row_end`, returning a new
    /// `MatrixSliceMut` over the same columns and parent.
    pub fn row_slice(&self, py: Python<'_>, row_start: usize, row_end: usize) -> PyResult<Self> {
        checked_range(row_start, row_end, self.rows_span())?;
        Ok(Self {
            parent: self.parent.clone_ref(py),
            row_start: self.row_start + row_start,
            row_end: self.row_start + row_end,
            col_start: self.col_start,
            col_end: self.col_end,
        })
    }

    /// Return immutable `FpSlice` handles for every row of the rectangle.
    ///
    /// We materialize a list of row handles. TODO: Don't do that.
    pub fn iter(&self, py: Python<'_>) -> PyResult<Vec<PyFpSlice>> {
        (0..self.rows_span()).map(|i| self.row(py, i)).collect()
    }

    /// Return mutable `FpSliceMut` handles for every row of the rectangle.
    ///
    /// As with `iter`, this is an eager list of index-based row handles
    /// rather than a lazy borrowing iterator. Mutating any handle writes
    /// through to the parent matrix.
    /// TODO: don't make a Vec of row_muts?
    pub fn iter_mut(&self, py: Python<'_>) -> PyResult<Vec<PyFpSliceMut>> {
        (0..self.rows_span()).map(|i| self.row_mut(py, i)).collect()
    }

    /// Add an identity matrix into the matrix. Requires a square matrix
    /// otherwise a `ValueError` is raised
    pub fn add_identity(&self, py: Python<'_>) -> PyResult<()> {
        if self.rows_span() != self.cols_span() {
            return Err(PyValueError::new_err(format!(
                "add_identity requires a square rectangle: {} rows but {} columns",
                self.rows_span(),
                self.cols_span()
            )));
        }
        self.with_slice_mut(py, |mut s| s.add_identity())
    }

    fn to_py(&self, py: Python<'_>) -> PyResult<Vec<Vec<u32>>> {
        self.with_slice_mut(py, |s| s.to_vec())
    }

    pub fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        let (prime, rows, columns) =
            self.with_slice_mut(py, |s| (s.prime().as_u32(), s.rows(), s.columns()))?;
        Ok(format!("MatrixSliceMut({prime}, {rows}x{columns})"))
    }
}

#[pymethods]
impl PyMatrix {
    #[new]
    pub fn new(p: u32, rows: usize, columns: usize) -> PyResult<Self> {
        Ok(Self(RustMatrix::new(valid_prime(p)?, rows, columns)))
    }

    #[staticmethod]
    pub fn from_rows(p: u32, rows: Vec<PyRef<'_, PyFpVector>>, columns: usize) -> PyResult<Self> {
        let p = valid_prime(p)?;
        for row in &rows {
            checked_same_prime(row.0.prime().as_u32(), p.as_u32())?;
            checked_equal_len(row.0.len(), columns)?;
        }
        let input = rows.iter().map(|row| row.0.clone()).collect();
        Ok(Self(RustMatrix::from_rows(p, input, columns)))
    }

    #[staticmethod]
    pub fn from_row(p: u32, row: PyRef<'_, PyFpVector>, columns: usize) -> PyResult<Self> {
        let p = valid_prime(p)?;
        checked_same_prime(row.0.prime().as_u32(), p.as_u32())?;
        checked_equal_len(row.0.len(), columns)?;
        Ok(Self(RustMatrix::from_row(p, row.0.clone(), columns)))
    }

    #[staticmethod]
    pub fn from_py(p: u32, input: Vec<Vec<u32>>) -> PyResult<Self> {
        let p = valid_prime(p)?;
        if let Some(first) = input.first() {
            let columns = first.len();
            for row in &input {
                checked_equal_len(row.len(), columns)?;
            }
        }
        Ok(Self(RustMatrix::from_vec(p, &input)))
    }

    #[staticmethod]
    pub fn identity(p: u32, dim: usize) -> PyResult<Self> {
        Ok(Self(RustMatrix::identity(valid_prime(p)?, dim)))
    }

    #[staticmethod]
    pub fn from_bytes(p: u32, rows: usize, columns: usize, data: &[u8]) -> PyResult<Self> {
        RustMatrix::from_bytes(valid_prime(p)?, rows, columns, &mut Cursor::new(data))
            .map(Self)
            .map_err(io_err)
    }

    #[getter]
    pub fn prime(&self) -> u32 {
        self.0.prime().as_u32()
    }

    #[getter]
    pub fn rows(&self) -> usize {
        self.0.rows()
    }

    #[getter]
    pub fn columns(&self) -> usize {
        self.0.columns()
    }

    #[getter]
    pub fn pivots(&self) -> Vec<isize> {
        self.0.pivots().to_vec()
    }

    #[getter]
    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    pub fn to_py(&self) -> Vec<Vec<u32>> {
        self.0.to_vec()
    }

    pub fn to_bytes<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        serialize_to_pybytes(py, |buffer| self.0.to_bytes(buffer))
    }

    pub fn row(slf: PyRef<'_, Self>, row: usize) -> PyResult<PyFpSlice> {
        checked_row(row, slf.0.rows())?;
        let end = slf.0.columns();
        let py = slf.py();
        Ok(PyFpSlice {
            parent: SliceParent::MatrixRow {
                matrix: MatrixParent::Matrix(slf.into_pyobject(py)?.unbind()),
                row,
            },
            start: 0,
            end,
        })
    }

    pub fn row_mut(slf: PyRef<'_, Self>, row: usize) -> PyResult<PyFpSliceMut> {
        checked_row(row, slf.0.rows())?;
        let end = slf.0.columns();
        let py = slf.py();
        Ok(PyFpSliceMut {
            parent: SliceParent::MatrixRow {
                matrix: MatrixParent::Matrix(slf.into_pyobject(py)?.unbind()),
                row,
            },
            start: 0,
            end,
        })
    }

    /// Return mutable `FpSliceMut` handles for every row of the matrix,
    /// one per row `i in 0..rows`.
    pub fn iter_mut(slf: PyRef<'_, Self>) -> PyResult<Vec<PyFpSliceMut>> {
        let py = slf.py();
        let rows = slf.0.rows();
        let end = slf.0.columns();
        let parent = slf.into_pyobject(py)?.unbind();
        Ok((0..rows)
            .map(|row| PyFpSliceMut {
                parent: SliceParent::MatrixRow {
                    matrix: MatrixParent::Matrix(parent.clone_ref(py)),
                    row,
                },
                start: 0,
                end,
            })
            .collect())
    }

    pub fn set_to_zero(&mut self) {
        self.0.set_to_zero()
    }

    pub fn assign(&mut self, other: &Self) -> PyResult<()> {
        checked_same_prime(self.0.prime().as_u32(), other.0.prime().as_u32())?;
        checked_equal_len(self.0.rows(), other.0.rows())?;
        checked_equal_len(self.0.columns(), other.0.columns())?;
        self.0.assign(&other.0);
        Ok(())
    }

    pub fn swap_rows(&mut self, i: usize, j: usize) -> PyResult<()> {
        checked_row(i, self.0.rows())?;
        checked_row(j, self.0.rows())?;
        self.0.swap_rows(i, j);
        Ok(())
    }

    pub fn row_op(&mut self, target: usize, source: usize, c: u32) -> PyResult<()> {
        checked_row(target, self.0.rows())?;
        checked_row(source, self.0.rows())?;
        if target == source {
            return Err(PyValueError::new_err(
                "target and source rows must be distinct",
            ));
        }
        self.0.safe_row_op(target, source, c);
        Ok(())
    }

    pub fn initialize_pivots(&mut self) {
        self.0.initialize_pivots()
    }

    pub fn extend_column_dimension(&mut self, columns: usize) {
        self.0.extend_column_dimension(columns)
    }

    pub fn extend_column_capacity(&mut self, columns: usize) {
        self.0.extend_column_capacity(columns)
    }

    pub fn add_row(slf: PyRef<'_, Self>) -> PyResult<PyFpSliceMut> {
        let py = slf.py();
        let parent = slf.into_pyobject(py)?.unbind();
        let (row, end) = {
            let mut matrix = parent.try_borrow_mut(py).map_err(borrow_error)?;
            matrix.0.add_row();
            (matrix.0.rows() - 1, matrix.0.columns())
        };
        Ok(PyFpSliceMut {
            parent: SliceParent::MatrixRow {
                matrix: MatrixParent::Matrix(parent),
                row,
            },
            start: 0,
            end,
        })
    }

    /// Return a mutable rectangular view over rows `row_start..row_end` and
    /// columns `col_start..col_end`.
    pub fn slice_mut(
        slf: PyRef<'_, Self>,
        row_start: usize,
        row_end: usize,
        col_start: usize,
        col_end: usize,
    ) -> PyResult<PyMatrixSliceMut> {
        checked_rect(
            row_start,
            row_end,
            col_start,
            col_end,
            slf.0.rows(),
            slf.0.columns(),
        )?;
        let py = slf.py();
        Ok(PyMatrixSliceMut {
            parent: MatrixParent::Matrix(slf.into_pyobject(py)?.unbind()),
            row_start,
            row_end,
            col_start,
            col_end,
        })
    }

    pub fn trim(&mut self, row_start: usize, row_end: usize, col_start: usize) -> PyResult<()> {
        checked_range(row_start, row_end, self.0.rows())?;
        if col_start > self.0.columns() {
            return Err(PyIndexError::new_err(format!(
                "column {col_start} out of range for matrix with {} columns",
                self.0.columns()
            )));
        }
        self.0.trim(row_start, row_end, col_start, false);
        Ok(())
    }

    pub fn rotate_down(&mut self, start: usize, end: usize, shift: usize) -> PyResult<()> {
        checked_range(start, end, self.0.rows())?;
        if shift > end - start {
            return Err(PyValueError::new_err(format!(
                "shift {shift} exceeds range length {}",
                end - start
            )));
        }
        self.0.rotate_down(start..end, shift);
        Ok(())
    }

    pub fn row_reduce(&mut self) -> usize {
        self.0.row_reduce()
    }

    pub fn __len__(&self) -> usize {
        self.0.rows()
    }

    pub fn __getitem__(slf: PyRef<'_, Self>, row: usize) -> PyResult<PyFpSlice> {
        Self::row(slf, row)
    }

    pub fn __repr__(&self) -> String {
        format!("Matrix({}, {})", self.prime(), self.0)
    }
}

impl PySubspace {
    /// Validate that `vector` matches this subspace's prime and ambient
    /// dimension, returning an error otherwise.
    fn check_compatible(&self, vector: &RustFpVector) -> PyResult<()> {
        checked_same_prime(self.0.prime().as_u32(), vector.prime().as_u32())?;
        checked_equal_len(vector.len(), self.0.ambient_dimension())?;
        Ok(())
    }

    /// Validate that `other` matches this subspace's prime and ambient
    /// dimension, returning an error otherwise.
    fn check_compatible_space(&self, other: &RustSubspace) -> PyResult<()> {
        checked_same_prime(self.0.prime().as_u32(), other.prime().as_u32())?;
        checked_equal_len(self.0.ambient_dimension(), other.ambient_dimension())?;
        Ok(())
    }
}

#[pymethods]
impl PySubspace {
    #[new]
    pub fn new(p: u32, dim: usize) -> PyResult<Self> {
        Ok(Self(RustSubspace::new(valid_prime(p)?, dim)))
    }

    #[staticmethod]
    pub fn from_matrix(matrix: &PyMatrix) -> Self {
        Self(RustSubspace::from_matrix(matrix.0.clone()))
    }

    #[staticmethod]
    pub fn entire_space(p: u32, dim: usize) -> PyResult<Self> {
        Ok(Self(RustSubspace::entire_space(valid_prime(p)?, dim)))
    }

    #[staticmethod]
    pub fn from_bytes(p: u32, data: &[u8]) -> PyResult<Self> {
        RustSubspace::from_bytes(valid_prime(p)?, &mut Cursor::new(data))
            .map(Self)
            .map_err(io_err)
    }

    #[getter]
    pub fn prime(&self) -> u32 {
        self.0.prime().as_u32()
    }

    #[getter]
    pub fn dimension(&self) -> usize {
        self.0.dimension()
    }

    #[getter]
    pub fn ambient_dimension(&self) -> usize {
        self.0.ambient_dimension()
    }

    /// Test whether `vector` lies in this subspace.
    pub fn contains(&self, py: Python<'_>, vector: &Bound<'_, PyAny>) -> PyResult<bool> {
        with_input_slice(py, vector, |slice| {
            checked_same_prime(self.0.prime().as_u32(), slice.prime().as_u32())?;
            checked_equal_len(slice.len(), self.0.ambient_dimension())?;
            Ok(self.0.contains(slice))
        })
    }

    pub fn contains_space(&self, other: &Self) -> PyResult<bool> {
        self.check_compatible_space(&other.0)?;
        Ok(self.0.contains_space(&other.0))
    }

    pub fn add_vector(&mut self, vector: &PyFpVector) -> PyResult<usize> {
        self.check_compatible(&vector.0)?;
        Ok(self.0.add_vector(vector.0.as_slice()))
    }

    /// Reduce `vector` in place against this subspace
    pub fn reduce(&self, vector: &mut PyFpVector) -> PyResult<()> {
        self.check_compatible(&vector.0)?;
        self.0.reduce(vector.0.as_slice_mut());
        Ok(())
    }

    pub fn sum(&self, other: &Self) -> PyResult<Self> {
        checked_same_prime(self.0.prime().as_u32(), other.0.prime().as_u32())?;
        checked_equal_len(self.0.ambient_dimension(), other.0.ambient_dimension())?;
        Ok(Self(self.0.sum(&other.0)))
    }

    /// Return the basis of the subspace as a list of owned `FpVector`s.
    pub fn iter(&self) -> Vec<PyFpVector> {
        self.0
            .iter()
            .map(|row| PyFpVector(row.to_owned()))
            .collect()
    }

    /// Return the basis of the subspace as a list of owned `FpVector`s.
    /// Mirrors upstream `Subspace::basis`.
    #[getter]
    pub fn basis(&self) -> Vec<PyFpVector> {
        self.iter()
    }

    /// Return a lazy iterator over every vector in the subspace.
    pub fn iter_all_vectors(&self) -> PySubspaceVectorIterator {
        let p = u128::from(self.0.prime().as_u32());
        let dim = self.0.dimension() as u32;
        let total = p.checked_pow(dim).unwrap_or(u128::MAX);
        PySubspaceVectorIterator {
            subspace: self.0.clone(),
            index: 0,
            total,
        }
    }

    pub fn set_to_zero(&mut self) {
        self.0.set_to_zero()
    }

    pub fn set_to_entire(&mut self) {
        self.0.set_to_entire()
    }

    pub fn to_bytes<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        serialize_to_pybytes(py, |buffer| self.0.to_bytes(buffer))
    }

    pub fn __len__(&self) -> usize {
        self.0.dimension()
    }

    pub fn __contains__(&self, py: Python<'_>, vector: &Bound<'_, PyAny>) -> PyResult<bool> {
        self.contains(py, vector)
    }

    pub fn __repr__(&self) -> String {
        format!(
            "Subspace({}, dim={}, ambient={})",
            self.prime(),
            self.0.dimension(),
            self.0.ambient_dimension()
        )
    }
}

#[pymethods]
impl PyQuasiInverse {
    /// Construct a `QuasiInverse` from an optional `image` (pivot list) and a
    /// `preimage` matrix.
    #[new]
    #[pyo3(signature = (image, preimage))]
    pub fn new(image: Option<Vec<isize>>, preimage: &PyMatrix) -> Self {
        Self(RustQuasiInverse::new(image, preimage.0.clone()))
    }

    /// Deserialize a `QuasiInverse` from bytes produced by [`Self::to_bytes`].
    #[staticmethod]
    pub fn from_bytes(p: u32, data: &[u8]) -> PyResult<Self> {
        RustQuasiInverse::from_bytes(valid_prime(p)?, &mut Cursor::new(data))
            .map(Self)
            .map_err(io_err)
    }

    #[getter]
    pub fn prime(&self) -> u32 {
        self.0.prime().as_u32()
    }

    #[getter]
    pub fn image_dimension(&self) -> usize {
        self.0.image_dimension()
    }

    #[getter]
    pub fn source_dimension(&self) -> usize {
        self.0.source_dimension()
    }

    #[getter]
    pub fn target_dimension(&self) -> usize {
        self.0.target_dimension()
    }

    #[getter]
    pub fn preimage(&self) -> PyMatrix {
        PyMatrix(self.0.preimage().clone())
    }

    #[getter]
    pub fn pivots(&self) -> Option<Vec<isize>> {
        self.0.pivots().map(<[isize]>::to_vec)
    }

    /// Apply the quasi-inverse to `input` and add `coeff` times the result
    /// to `target`.
    ///
    /// `input` is a vector in the target space (length `target_dimension`)
    /// and `target` receives the result in the source space (length
    /// `source_dimension`). Both accept either an `FpVector` or the
    /// corresponding slice handle.
    pub fn apply(
        &self,
        py: Python<'_>,
        target: &Bound<'_, PyAny>,
        coeff: u32,
        input: &Bound<'_, PyAny>,
    ) -> PyResult<()> {
        // Borrow the input. If the same object is passed as both `input` and
        // `target`, the nested shared+mutable borrows raises `RuntimeError`.
        with_input_slice(py, input, |input_slice| {
            checked_same_prime(self.0.prime().as_u32(), input_slice.prime().as_u32())?;
            checked_equal_len(input_slice.len(), self.0.target_dimension())?;
            with_target_slice_mut(py, target, |target_slice| {
                checked_same_prime(
                    self.0.prime().as_u32(),
                    target_slice.as_slice().prime().as_u32(),
                )?;
                checked_equal_len(target_slice.as_slice().len(), self.0.source_dimension())?;
                // Reduce `coeff` mod p before calling upstream to avoid overflow
                let coeff = coeff % self.0.prime().as_u32();
                self.0.apply(target_slice, coeff, input_slice);
                Ok(())
            })
        })
    }

    /// Serialize the quasi-inverse to bytes.
    pub fn to_bytes<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        serialize_to_pybytes(py, |buffer| self.0.to_bytes(buffer))
    }

    pub fn __repr__(&self) -> String {
        format!(
            "QuasiInverse({}, image_dim={}, source_dim={}, target_dim={})",
            self.prime(),
            self.0.image_dimension(),
            self.0.source_dimension(),
            self.0.target_dimension()
        )
    }
}

impl PySubquotient {
    /// Validate that `vector` matches this subquotient's prime and ambient
    /// dimension, returning an error otherwise.
    fn check_compatible(&self, vector: &RustFpVector) -> PyResult<()> {
        checked_same_prime(self.0.prime().as_u32(), vector.prime().as_u32())?;
        checked_equal_len(vector.len(), self.0.ambient_dimension())?;
        Ok(())
    }
}

#[pymethods]
impl PySubquotient {
    /// Create a new subquotient of an ambient space of dimension `dim`,
    /// defaulting to the zero subspace.
    #[new]
    pub fn new(p: u32, dim: usize) -> PyResult<Self> {
        Ok(Self(RustSubquotient::new(valid_prime(p)?, dim)))
    }

    /// Create a new subquotient of an ambient space of dimension `dim`,
    /// where the subspace is the full space and the quotient is trivial.
    #[staticmethod]
    pub fn new_full(p: u32, dim: usize) -> PyResult<Self> {
        Ok(Self(RustSubquotient::new_full(valid_prime(p)?, dim)))
    }

    /// Construct the subquotient `(sub + quotient) / quotient` from a chain
    /// of subspaces. The two subspaces must share a prime and ambient
    /// dimension.
    #[staticmethod]
    pub fn from_parts(sub: &PySubspace, quotient: &PySubspace) -> PyResult<Self> {
        checked_same_prime(sub.0.prime().as_u32(), quotient.0.prime().as_u32())?;
        checked_equal_len(sub.0.ambient_dimension(), quotient.0.ambient_dimension())?;
        Ok(Self(RustSubquotient::from_parts(
            sub.0.clone(),
            quotient.0.clone(),
        )))
    }

    #[getter]
    pub fn prime(&self) -> u32 {
        self.0.prime().as_u32()
    }

    #[getter]
    pub fn dimension(&self) -> usize {
        self.0.dimension()
    }

    #[getter]
    pub fn ambient_dimension(&self) -> usize {
        self.0.ambient_dimension()
    }

    #[getter]
    pub fn quotient_dimension(&self) -> usize {
        self.0.quotient_dimension()
    }

    #[getter]
    pub fn subspace_dimension(&self) -> usize {
        self.0.subspace_dimension()
    }

    #[getter]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn zeros(&self) -> PySubspace {
        PySubspace(self.0.zeros().clone())
    }

    /// The generators of the subquotient, returned as a list of owned
    /// `FpVector`s.
    #[getter]
    pub fn gens(&self) -> Vec<PyFpVector> {
        self.0
            .gens()
            .map(|row| PyFpVector(row.to_owned()))
            .collect()
    }

    /// The generators of the subspace part of the subquotient, returned as
    /// a list of owned `FpVector`s.
    #[getter]
    pub fn subspace_gens(&self) -> Vec<PyFpVector> {
        self.0
            .subspace_gens()
            .map(|row| PyFpVector(row.to_owned()))
            .collect()
    }

    /// The pivot columns of the complement to the subspace.
    #[getter]
    pub fn complement_pivots(&self) -> Vec<usize> {
        self.0.complement_pivots().collect()
    }

    /// The pivot table of the quotient subspace.
    #[getter]
    pub fn quotient_pivots(&self) -> Vec<isize> {
        self.0.quotient_pivots().to_vec()
    }

    /// Reduce `vector` in place: project it onto a complement of the
    /// quotient and express it relative to the generators. Returns the list
    /// of coefficients with respect to the generators.
    pub fn reduce(&self, vector: &mut PyFpVector) -> PyResult<Vec<u32>> {
        self.check_compatible(&vector.0)?;
        Ok(self.0.reduce(vector.0.as_slice_mut()))
    }

    /// Project `vector` in place onto the complement of the quotient part.
    pub fn reduce_by_quotient(&self, py: Python<'_>, vector: &Bound<'_, PyAny>) -> PyResult<()> {
        with_target_slice_mut(py, vector, |slice| {
            checked_same_prime(self.0.prime().as_u32(), slice.as_slice().prime().as_u32())?;
            checked_equal_len(slice.as_slice().len(), self.0.ambient_dimension())?;
            self.0.reduce_by_quotient(slice);
            Ok(())
        })
    }

    /// Add `vector` to the quotient part of the subquotient.
    pub fn quotient(&mut self, vector: &PyFpVector) -> PyResult<()> {
        self.check_compatible(&vector.0)?;
        self.0.quotient(vector.0.as_slice());
        Ok(())
    }

    /// Add `vector` as a generator of the subquotient.
    pub fn add_gen(&mut self, vector: &PyFpVector) -> PyResult<()> {
        self.check_compatible(&vector.0)?;
        self.0.add_gen(vector.0.as_slice());
        Ok(())
    }

    /// Remove all generators, leaving the quotient part untouched.
    pub fn clear_gens(&mut self) {
        self.0.clear_gens()
    }

    /// Set the subquotient to be the full ambient space quotiented by zero.
    pub fn set_to_full(&mut self) {
        self.0.set_to_full()
    }

    /// Apply `matrix` to each generator of `source`, then reduce the image
    /// in `target`, returning the coefficient lists.
    #[staticmethod]
    pub fn reduce_matrix(
        matrix: &PyMatrix,
        source: &Self,
        target: &Self,
    ) -> PyResult<Vec<Vec<u32>>> {
        checked_same_prime(source.0.prime().as_u32(), target.0.prime().as_u32())?;
        checked_same_prime(source.0.prime().as_u32(), matrix.0.prime().as_u32())?;
        checked_equal_len(matrix.0.rows(), source.0.ambient_dimension())?;
        checked_equal_len(matrix.0.columns(), target.0.ambient_dimension())?;
        Ok(RustSubquotient::reduce_matrix(
            &matrix.0, &source.0, &target.0,
        ))
    }

    pub fn __len__(&self) -> usize {
        self.0.dimension()
    }

    pub fn __repr__(&self) -> String {
        format!(
            "Subquotient({}, dim={}, ambient={})",
            self.prime(),
            self.0.dimension(),
            self.0.ambient_dimension()
        )
    }
}

impl PyAffineSubspace {
    /// Validate that `other` matches this affine subspace's prime and
    /// ambient dimension, returning an error otherwise.
    fn check_compatible_space(&self, other: &Self) -> PyResult<()> {
        checked_same_prime(self.prime(), other.prime())?;
        checked_equal_len(self.ambient_dimension(), other.ambient_dimension())?;
        Ok(())
    }
}

#[pymethods]
impl PyAffineSubspace {
    /// Construct an affine subspace `offset + linear_part`.
    #[new]
    pub fn new(offset: &PyFpVector, linear_part: &PySubspace) -> Self {
        Self(RustAffineSubspace::new(
            offset.0.clone(),
            linear_part.0.clone(),
        ))
    }

    #[getter]
    pub fn prime(&self) -> u32 {
        self.0.linear_part().prime().as_u32()
    }

    #[getter]
    pub fn ambient_dimension(&self) -> usize {
        self.0.linear_part().ambient_dimension()
    }

    #[getter]
    pub fn dimension(&self) -> usize {
        self.0.linear_part().dimension()
    }

    /// Return an owned copy of the reduced offset vector.
    pub fn offset(&self) -> PyFpVector {
        PyFpVector(self.0.offset().clone())
    }

    /// Return an owned copy of the linear part `Subspace`
    pub fn linear_part(&self) -> PySubspace {
        PySubspace(self.0.linear_part().clone())
    }

    /// Test whether `vector` lies in this affine subspace.
    pub fn contains(&self, py: Python<'_>, vector: &Bound<'_, PyAny>) -> PyResult<bool> {
        with_input_slice(py, vector, |slice| {
            checked_same_prime(self.prime(), slice.prime().as_u32())?;
            checked_equal_len(slice.len(), self.ambient_dimension())?;
            Ok(self.0.contains(slice))
        })
    }

    pub fn contains_space(&self, other: &Self) -> PyResult<bool> {
        self.check_compatible_space(other)?;
        Ok(self.0.contains_space(&other.0))
    }

    /// Return the affine subspace spanned by the union of `self` and
    /// `other`: the sum of the linear parts translated by the sum of the
    /// offsets.
    pub fn sum(&self, other: &Self) -> PyResult<Self> {
        self.check_compatible_space(other)?;
        Ok(Self(self.0.sum(&other.0)))
    }

    pub fn __contains__(&self, py: Python<'_>, vector: &Bound<'_, PyAny>) -> PyResult<bool> {
        self.contains(py, vector)
    }

    pub fn __repr__(&self) -> String {
        format!("AffineSubspace({})", self.0)
    }
}

#[pymethods]
impl PySubspaceVectorIterator {
    pub fn __iter__(slf: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> {
        slf
    }

    pub fn __next__(&mut self) -> Option<PyFpVector> {
        if self.index >= self.total {
            return None;
        }
        let p = u128::from(self.subspace.prime().as_u32());
        let dim = self.subspace.dimension();
        // Decode `index` into base-`p` digits, most significant first, to
        // match the lexicographic order of `combinations` upstream where
        // the first digit (matching the first basis row) varies slowest.
        let mut digits = vec![0u32; dim];
        let mut rem = self.index;
        for slot in digits.iter_mut().rev() {
            *slot = (rem % p) as u32;
            rem /= p;
        }
        let mut vector =
            RustFpVector::new(self.subspace.prime(), self.subspace.ambient_dimension());
        for (&c, row) in digits.iter().zip(self.subspace.iter()) {
            vector.as_slice_mut().add(row, c);
        }
        self.index += 1;
        Some(PyFpVector(vector))
    }
}

/// `AugmentedMatrix<N>` is a const-generic type, and PyO3 cannot expose a
/// generic `#[pyclass]`. We bind the two concrete arities used in the codebase
/// (`N = 2` and `N = 3`) as separate classes `AugmentedMatrix2` and
/// `AugmentedMatrix3`. To avoid duplicating the shared glue, this
/// `macro_rules!` macro generates each class from a single definition. However,
/// `#[pymodule]` cannot see through a `macro_rules!` expansion to auto-collect
/// the classes, so they are registered explicitly with `add_class` in
/// `#[pymodule_init]`.
macro_rules! augmented_matrix_pyclass {
    ($name:ident, $pyname:literal, $n:literal, $variant:ident, { $($extra:tt)* }) => {
        /// The inner `AugmentedMatrix<N>` is held in a [`Consumable`] so the
        /// consuming methods (`into_matrix`, `compute_quasi_inverses`) can
        /// `take()` it out and run the upstream by-value operation.
        #[pyclass(name = $pyname)]
        pub struct $name(pub(crate) Consumable<RustAugmentedMatrix<$n>>);

        #[pymethods]
        impl $name {
            /// Construct an `rows x sum(columns)` augmented matrix whose
            /// column blocks have the given widths. `columns` must contain
            /// exactly `N` segment widths.
            #[new]
            fn new(p: u32, rows: usize, columns: Vec<usize>) -> PyResult<Self> {
                let len = columns.len();
                let cols: [usize; $n] = columns.try_into().map_err(|_| {
                    PyValueError::new_err(format!(
                        "expected {} segment widths, got {len}",
                        $n
                    ))
                })?;
                Ok(Self(Consumable::new(
                    $pyname,
                    RustAugmentedMatrix::<$n>::new(valid_prime(p)?, rows, cols),
                )))
            }

            #[getter]
            fn prime(&self) -> PyResult<u32> {
                Ok(self.0.get()?.prime().as_u32())
            }

            #[getter]
            fn rows(&self) -> PyResult<usize> {
                Ok(self.0.get()?.rows())
            }

            #[getter]
            fn columns(&self) -> PyResult<usize> {
                Ok(self.0.get()?.columns())
            }

            /// Number of column segments (`N`).
            #[getter]
            fn segments(&self) -> usize {
                $n
            }

            /// The starting column index of each segment.
            #[getter]
            fn segment_starts(&self) -> PyResult<Vec<usize>> {
                Ok(self.0.get()?.start.to_vec())
            }

            /// The (exclusive) ending column index of each segment.
            #[getter]
            fn segment_ends(&self) -> PyResult<Vec<usize>> {
                Ok(self.0.get()?.end.to_vec())
            }

            #[getter]
            fn pivots(&self) -> PyResult<Vec<isize>> {
                Ok(self.0.get()?.pivots().to_vec())
            }

            #[getter]
            fn is_zero(&self) -> PyResult<bool> {
                Ok(self.0.get()?.is_zero())
            }

            fn to_py(&self) -> PyResult<Vec<Vec<u32>>> {
                Ok(self.0.get()?.to_vec())
            }

            fn row_reduce(&mut self) -> PyResult<usize> {
                Ok(self.0.get_mut()?.row_reduce())
            }

            /// Add an identity matrix into the rectangular segment spanning
            /// segment indices `start..=end`. The segment must be square
            fn add_identity(&mut self, start: usize, end: usize) -> PyResult<()> {
                let m = self.0.get_mut()?;
                let cols = segment_cols(m, start, end)?;
                if m.rows() != cols {
                    return Err(PyValueError::new_err(format!(
                        "add_identity requires a square segment: matrix has {} rows but \
                         segment [{start}, {end}] has {cols} columns",
                        m.rows()
                    )));
                }
                m.segment(start, end).add_identity();
                Ok(())
            }

            /// Return an owned copy of row `i` restricted to the columns of the
            /// segment range `start..=end`.
            /// TODO: Maybe don't copy?
            fn row_segment(
                &self,
                i: usize,
                start: usize,
                end: usize,
            ) -> PyResult<PyFpVector> {
                let m = self.0.get()?;
                checked_row(i, m.rows())?;
                segment_cols(m, start, end)?;
                Ok(PyFpVector(m.row_segment(i, start, end).to_owned()))
            }

            /// Return a mutable rectangular view spanning all rows and the
            /// columns of segment range `start..=end`, as a `MatrixSliceMut`
            /// over the inner matrix.
            fn segment(
                slf: PyRef<'_, Self>,
                start: usize,
                end: usize,
            ) -> PyResult<PyMatrixSliceMut> {
                let (row_end, col_start, col_end) = {
                    let m = slf.0.get()?;
                    segment_cols(m, start, end)?;
                    (m.rows(), m.start[start], m.end[end])
                };
                let py = slf.py();
                Ok(PyMatrixSliceMut {
                    parent: MatrixParent::$variant(slf.into_pyobject(py)?.unbind()),
                    row_start: 0,
                    row_end,
                    col_start,
                    col_end,
                })
            }

            /// Return a `FpSliceMut` over row `i` restricted to the
            /// columns of segment range `start..=end`.
            fn row_segment_mut(
                slf: PyRef<'_, Self>,
                i: usize,
                start: usize,
                end: usize,
            ) -> PyResult<PyFpSliceMut> {
                let (col_start, col_end) = {
                    let m = slf.0.get()?;
                    checked_row(i, m.rows())?;
                    segment_cols(m, start, end)?;
                    (m.start[start], m.end[end])
                };
                let py = slf.py();
                Ok(PyFpSliceMut {
                    parent: SliceParent::MatrixRow {
                        matrix: MatrixParent::$variant(slf.into_pyobject(py)?.unbind()),
                        row: i,
                    },
                    start: col_start,
                    end: col_end,
                })
            }

            /// Return a `FpSliceMut` over the whole of row `i`
            /// (all columns, across every segment).
            fn row_mut(slf: PyRef<'_, Self>, i: usize) -> PyResult<PyFpSliceMut> {
                let end = {
                    let m = slf.0.get()?;
                    checked_row(i, m.rows())?;
                    m.columns()
                };
                let py = slf.py();
                Ok(PyFpSliceMut {
                    parent: SliceParent::MatrixRow {
                        matrix: MatrixParent::$variant(slf.into_pyobject(py)?.unbind()),
                        row: i,
                    },
                    start: 0,
                    end,
                })
            }

            /// Compute the kernel of the augmented matrix (which must be row
            /// reduced), returning an owned `Subspace`. Available for all
            /// arities.
            fn compute_kernel(&self) -> PyResult<PySubspace> {
                Ok(PySubspace(self.0
                    .get()?
                    .compute_kernel()))
            }

            /// Return the inner `Matrix` as an owned `Matrix`, **consuming**
            /// this augmented matrix.
            ///
            /// After this call the augmented matrix is consumed, so any further
            /// use raises `RuntimeError`.
            #[allow(clippy::wrong_self_convention)]
            fn into_matrix(&mut self) -> PyResult<PyMatrix> {
                Ok(PyMatrix(self.0.take()?.into_matrix()))
            }

            fn __repr__(&self) -> String {
                match self.0.get() {
                    Ok(m) => format!(
                        concat!($pyname, "({}, {}x{})"),
                        m.prime().as_u32(),
                        m.rows(),
                        m.columns()
                    ),
                    Err(_) => concat!($pyname, "(consumed)").to_string(),
                }
            }

            $($extra)*
        }
    };
}

augmented_matrix_pyclass!(PyAugmentedMatrix2, "AugmentedMatrix2", 2, Augmented2, {
    /// Compute the image of the augmented matrix `[A | I]` (which must be
    /// row reduced), returning an owned `Subspace`.
    fn compute_image(&self) -> PyResult<PySubspace> {
        Ok(PySubspace(self.0.get()?.compute_image()))
    }

    /// Compute the quasi-inverse of the augmented matrix `[A | I]` (which
    /// must be row reduced), returning an owned `QuasiInverse`.
    fn compute_quasi_inverse(&self) -> PyResult<PyQuasiInverse> {
        Ok(PyQuasiInverse(self.0.get()?.compute_quasi_inverse()))
    }

    #[staticmethod]
    pub fn from_py(p: u32, input: Vec<Vec<u32>>) -> PyResult<Self> {
        let p = valid_prime(p)?;
        if input.is_empty() {
            return Err(PyValueError::new_err(
                "AugmentedMatrix2.from_py() requires at least one row",
            ));
        }
        let columns = input[0].len();
        for row in &input {
            checked_equal_len(row.len(), columns)?;
        }
        Ok(Self(Consumable::new(
            "AugmentedMatrix2",
            RustMatrix::augmented_from_vec(p, &input),
        )))
    }
});

augmented_matrix_pyclass!(PyAugmentedMatrix3, "AugmentedMatrix3", 3, Augmented3, {
    /// Compute the two quasi-inverses for a row-reduced augmented matrix of the
    /// form `[A | B | I]` where `A` is surjective, returning the pair
    /// `(quasi_inverse_of_A, residual_quasi_inverse)`.
    ///
    /// This consumes the augmented matrix. After this call any further use
    /// raises `RuntimeError`.
    fn compute_quasi_inverses(&mut self) -> PyResult<(PyQuasiInverse, PyQuasiInverse)> {
        let (a, b) = self.0.take()?.compute_quasi_inverses();
        Ok((PyQuasiInverse(a), PyQuasiInverse(b)))
    }
});
