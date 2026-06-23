use pyo3::prelude::*;

#[pymodule]
#[pyo3(name = "fp")]
pub mod fp_py {
    use fp::field::{
        element::FieldElement as RustFieldElement, Field, Fp as RustFp, SmallFq as RustSmallFq,
    };
    use fp::matrix::{Matrix as RustMatrix, Subspace as RustSubspace};
    use fp::prime::{self, Binomial, Prime};
    use fp::vector::{
        FpSlice as RustFpSlice, FpSliceMut as RustFpSliceMut, FpVector as RustFpVector,
    };
    use pyo3::basic::CompareOp;
    use pyo3::exceptions::{PyIndexError, PyRuntimeError, PyValueError, PyZeroDivisionError};
    use pyo3::types::PyBytes;
    use std::hash::{DefaultHasher, Hash, Hasher};
    use std::io::Cursor;

    use super::*;

    const MAX_VALID_PRIME: u32 = 1 << 31;

    type DynFp = RustFp<prime::ValidPrime>;
    type DynSmallFq = RustSmallFq<prime::ValidPrime>;
    type DynFpElement = RustFieldElement<DynFp>;
    type DynSmallFqElement = RustFieldElement<DynSmallFq>;

    #[pyclass(name = "Fp", frozen, from_py_object)]
    #[derive(Clone, Copy)]
    struct PyFp(DynFp);

    #[pyclass(name = "SmallFq", frozen, from_py_object)]
    #[derive(Clone, Copy)]
    struct PySmallFq(DynSmallFq);

    #[derive(Clone, Copy, PartialEq, Eq, Hash)]
    enum FieldElementKind {
        Fp(DynFpElement),
        SmallFq(DynSmallFqElement),
    }

    #[pyclass(name = "FieldElement", frozen, from_py_object)]
    #[derive(Clone, Copy)]
    struct PyFieldElement(FieldElementKind);

    #[pyclass(name = "FpVector")]
    struct PyFpVector(RustFpVector);

    /// The source backing a slice handle: either an owned vector, or a row of a
    /// matrix. In both cases we keep the parent Python object alive and store
    /// enough metadata to reconstruct the underlying Rust slice on each call,
    /// revalidating against the parent's current dimensions first.
    enum SliceParent {
        Vector(Py<PyFpVector>),
        MatrixRow { matrix: Py<PyMatrix>, row: usize },
    }

    impl SliceParent {
        fn clone_ref(&self, py: Python<'_>) -> Self {
            match self {
                Self::Vector(v) => Self::Vector(v.clone_ref(py)),
                Self::MatrixRow { matrix, row } => Self::MatrixRow {
                    matrix: matrix.clone_ref(py),
                    row: *row,
                },
            }
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
    fn with_parent_slice<R>(
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
            SliceParent::MatrixRow { matrix, row } => {
                let parent = matrix.try_borrow(py).map_err(borrow_error)?;
                checked_row(*row, parent.0.rows())?;
                let full = parent.0.row(*row);
                checked_range(start, end, full.len())?;
                Ok(f(full.restrict(start, end)))
            }
        }
    }

    /// Run `f` on the reconstructed mutable slice for `parent[start..end]`,
    /// after revalidating the parent's current dimensions.
    fn with_parent_slice_mut<R>(
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
            SliceParent::MatrixRow { matrix, row } => {
                let mut parent = matrix.try_borrow_mut(py).map_err(borrow_error)?;
                checked_row(*row, parent.0.rows())?;
                // Validate against the actual current row length, matching the
                // read path (`with_parent_slice`). For a `Matrix` this equals
                // `columns()`, but deriving it from the row keeps both paths
                // consistent regardless of that invariant.
                let row_len = parent.0.row(*row).len();
                checked_range(start, end, row_len)?;
                Ok(f(parent.0.row_mut(*row).slice_mut(start, end)))
            }
        }
    }

    #[pyclass(name = "FpSlice")]
    struct PyFpSlice {
        parent: SliceParent,
        start: usize,
        end: usize,
    }

    #[pyclass(name = "FpSliceMut")]
    struct PyFpSliceMut {
        parent: SliceParent,
        start: usize,
        end: usize,
    }

    #[pyclass(name = "FpVectorIterator")]
    struct PyFpVectorIterator {
        entries: Vec<u32>,
        index: usize,
    }

    #[pyclass(name = "Matrix")]
    struct PyMatrix(RustMatrix);

    #[pyclass(name = "Subspace")]
    struct PySubspace(RustSubspace);

    fn valid_prime(p: u32) -> PyResult<prime::ValidPrime> {
        if p < 2 || p >= MAX_VALID_PRIME {
            return Err(PyValueError::new_err(format!("{p} is not prime")));
        }
        prime::ValidPrime::try_from(p)
            .map_err(|_| PyValueError::new_err(format!("{p} is not prime")))
    }

    fn table_prime(p: u32) -> PyResult<prime::ValidPrime> {
        if fp::PRIMES.contains(&p) {
            valid_prime(p)
        } else {
            Err(PyValueError::new_err(format!(
                "{p} is not a supported table prime"
            )))
        }
    }

    fn small_fq(p: u32, degree: u32) -> PyResult<DynSmallFq> {
        let p = valid_prime(p)?;
        if degree <= 1 {
            return Err(PyValueError::new_err("degree must be greater than 1"));
        }
        if degree > 16 || p.as_u32().checked_pow(degree).is_none_or(|q| q >= 1 << 16) {
            return Err(PyValueError::new_err("field is too large"));
        }
        Ok(DynSmallFq::new(p, degree))
    }

    fn py_hash<T: Hash>(value: &T) -> isize {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        match hasher.finish() as isize {
            -1 => -2,
            hash => hash,
        }
    }

    fn checked_index(index: usize, len: usize) -> PyResult<usize> {
        if index < len {
            Ok(index)
        } else {
            Err(PyIndexError::new_err(format!(
                "index {index} out of range for vector of length {len}"
            )))
        }
    }

    fn py_index(index: isize, len: usize) -> PyResult<usize> {
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

    fn checked_range(start: usize, end: usize, len: usize) -> PyResult<()> {
        if start <= end && end <= len {
            Ok(())
        } else {
            Err(PyIndexError::new_err(format!(
                "range {start}..{end} out of range for vector of length {len}"
            )))
        }
    }

    fn borrow_error(err: impl ToString) -> PyErr {
        PyRuntimeError::new_err(err.to_string())
    }

    fn checked_equal_len(lhs: usize, rhs: usize) -> PyResult<()> {
        if lhs == rhs {
            Ok(())
        } else {
            Err(PyValueError::new_err(format!(
                "length mismatch: {lhs} != {rhs}"
            )))
        }
    }

    fn checked_same_prime(lhs: u32, rhs: u32) -> PyResult<()> {
        if lhs == rhs {
            Ok(())
        } else {
            Err(PyValueError::new_err(format!(
                "prime mismatch: {lhs} != {rhs}"
            )))
        }
    }

    impl FieldElementKind {
        fn field_repr(self) -> String {
            match self {
                Self::Fp(x) => format!("Fp({})", x.field().characteristic().as_u32()),
                Self::SmallFq(x) => {
                    let f = x.field();
                    format!("SmallFq({}, {})", f.characteristic().as_u32(), f.degree())
                }
            }
        }

        fn mismatched_field_error(lhs: Self, rhs: Self) -> PyErr {
            PyValueError::new_err(format!(
                "cannot combine elements from {} and {}",
                lhs.field_repr(),
                rhs.field_repr()
            ))
        }
    }

    impl PyFpSlice {
        fn with_slice<R>(
            &self,
            py: Python<'_>,
            f: impl FnOnce(RustFpSlice<'_>) -> R,
        ) -> PyResult<R> {
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

    impl PyFpSliceMut {
        fn with_slice<R>(
            &self,
            py: Python<'_>,
            f: impl FnOnce(RustFpSlice<'_>) -> R,
        ) -> PyResult<R> {
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
    }

    fn checked_row(row: usize, rows: usize) -> PyResult<usize> {
        if row < rows {
            Ok(row)
        } else {
            Err(PyIndexError::new_err(format!(
                "row {row} out of range for matrix with {rows} rows"
            )))
        }
    }

    #[pymethods]
    impl PyFp {
        #[new]
        pub fn new(p: u32) -> PyResult<Self> {
            Ok(Self(DynFp::new(valid_prime(p)?)))
        }

        pub fn characteristic(&self) -> u32 {
            self.0.characteristic().as_u32()
        }

        pub fn degree(&self) -> u32 {
            self.0.degree()
        }

        pub fn zero(&self) -> PyFieldElement {
            PyFieldElement(FieldElementKind::Fp(self.0.zero()))
        }

        pub fn one(&self) -> PyFieldElement {
            PyFieldElement(FieldElementKind::Fp(self.0.one()))
        }

        pub fn element(&self, value: u32) -> PyFieldElement {
            PyFieldElement(FieldElementKind::Fp(self.0.element(value)))
        }

        pub fn __repr__(&self) -> String {
            format!("Fp({})", self.characteristic())
        }

        pub fn __richcmp__(&self, other: &Bound<'_, PyAny>, op: CompareOp) -> bool {
            let eq = other
                .extract::<PyRef<Self>>()
                .is_ok_and(|other| self.0 == other.0);
            match op {
                CompareOp::Eq => eq,
                CompareOp::Ne => !eq,
                _ => false,
            }
        }

        pub fn __hash__(&self) -> isize {
            py_hash(&self.0)
        }
    }

    #[pymethods]
    impl PySmallFq {
        #[new]
        pub fn new(p: u32, degree: u32) -> PyResult<Self> {
            Ok(Self(small_fq(p, degree)?))
        }

        pub fn p(&self) -> u32 {
            self.0.characteristic().as_u32()
        }

        pub fn degree(&self) -> u32 {
            self.0.degree()
        }

        pub fn a(&self) -> PyFieldElement {
            PyFieldElement(FieldElementKind::SmallFq(self.0.a()))
        }

        pub fn q(&self) -> u32 {
            self.0.q()
        }

        pub fn zero(&self) -> PyFieldElement {
            PyFieldElement(FieldElementKind::SmallFq(self.0.zero()))
        }

        pub fn one(&self) -> PyFieldElement {
            PyFieldElement(FieldElementKind::SmallFq(self.0.one()))
        }

        pub fn __repr__(&self) -> String {
            format!("SmallFq({}, {})", self.p(), self.degree())
        }

        pub fn __richcmp__(&self, other: &Bound<'_, PyAny>, op: CompareOp) -> bool {
            let eq = other
                .extract::<PyRef<Self>>()
                .is_ok_and(|other| self.0 == other.0);
            match op {
                CompareOp::Eq => eq,
                CompareOp::Ne => !eq,
                _ => false,
            }
        }

        pub fn __hash__(&self) -> isize {
            py_hash(&self.0)
        }
    }

    #[pymethods]
    impl PyFieldElement {
        pub fn inv(&self) -> Option<Self> {
            match self.0 {
                FieldElementKind::Fp(x) => x.inv().map(|x| Self(FieldElementKind::Fp(x))),
                FieldElementKind::SmallFq(x) => x.inv().map(|x| Self(FieldElementKind::SmallFq(x))),
            }
        }

        pub fn frobenius(&self) -> Self {
            match self.0 {
                FieldElementKind::Fp(x) => Self(FieldElementKind::Fp(x.frobenius())),
                FieldElementKind::SmallFq(x) => Self(FieldElementKind::SmallFq(x.frobenius())),
            }
        }

        pub fn field<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
            match self.0 {
                FieldElementKind::Fp(x) => {
                    Py::new(py, PyFp(x.field())).map(|x| x.into_bound(py).into_any())
                }
                FieldElementKind::SmallFq(x) => {
                    Py::new(py, PySmallFq(x.field())).map(|x| x.into_bound(py).into_any())
                }
            }
        }

        pub fn __add__(&self, rhs: Self) -> PyResult<Self> {
            match (self.0, rhs.0) {
                (FieldElementKind::Fp(a), FieldElementKind::Fp(b)) if a.field() == b.field() => {
                    Ok(Self(FieldElementKind::Fp(a + b)))
                }
                (FieldElementKind::SmallFq(a), FieldElementKind::SmallFq(b))
                    if a.field() == b.field() =>
                {
                    Ok(Self(FieldElementKind::SmallFq(a + b)))
                }
                (a, b) => Err(FieldElementKind::mismatched_field_error(a, b)),
            }
        }

        pub fn __sub__(&self, rhs: Self) -> PyResult<Self> {
            match (self.0, rhs.0) {
                (FieldElementKind::Fp(a), FieldElementKind::Fp(b)) if a.field() == b.field() => {
                    Ok(Self(FieldElementKind::Fp(a - b)))
                }
                (FieldElementKind::SmallFq(a), FieldElementKind::SmallFq(b))
                    if a.field() == b.field() =>
                {
                    Ok(Self(FieldElementKind::SmallFq(a - b)))
                }
                (a, b) => Err(FieldElementKind::mismatched_field_error(a, b)),
            }
        }

        pub fn __mul__(&self, rhs: Self) -> PyResult<Self> {
            match (self.0, rhs.0) {
                (FieldElementKind::Fp(a), FieldElementKind::Fp(b)) if a.field() == b.field() => {
                    Ok(Self(FieldElementKind::Fp(a * b)))
                }
                (FieldElementKind::SmallFq(a), FieldElementKind::SmallFq(b))
                    if a.field() == b.field() =>
                {
                    Ok(Self(FieldElementKind::SmallFq(a * b)))
                }
                (a, b) => Err(FieldElementKind::mismatched_field_error(a, b)),
            }
        }

        pub fn __truediv__(&self, rhs: Self) -> PyResult<Self> {
            match (self.0, rhs.0) {
                (FieldElementKind::Fp(a), FieldElementKind::Fp(b)) if a.field() == b.field() => (a
                    / b)
                    .map(|x| Self(FieldElementKind::Fp(x)))
                    .ok_or_else(|| PyZeroDivisionError::new_err("division by zero")),
                (FieldElementKind::SmallFq(a), FieldElementKind::SmallFq(b))
                    if a.field() == b.field() =>
                {
                    (a / b)
                        .map(|x| Self(FieldElementKind::SmallFq(x)))
                        .ok_or_else(|| PyZeroDivisionError::new_err("division by zero"))
                }
                (a, b) => Err(FieldElementKind::mismatched_field_error(a, b)),
            }
        }

        pub fn __neg__(&self) -> Self {
            match self.0 {
                FieldElementKind::Fp(x) => Self(FieldElementKind::Fp(-x)),
                FieldElementKind::SmallFq(x) => Self(FieldElementKind::SmallFq(-x)),
            }
        }

        pub fn __int__(&self) -> PyResult<u32> {
            match self.0 {
                FieldElementKind::Fp(x) => Ok(*x),
                FieldElementKind::SmallFq(_) => Err(PyValueError::new_err(
                    "SmallFq elements do not have a canonical integer value",
                )),
            }
        }

        pub fn __repr__(&self) -> String {
            match self.0 {
                FieldElementKind::Fp(x) => {
                    format!("FieldElement(Fp({}), {x})", x.field().characteristic())
                }
                FieldElementKind::SmallFq(x) => {
                    let f = x.field();
                    format!(
                        "FieldElement(SmallFq({}, {}), {x})",
                        f.characteristic(),
                        f.degree()
                    )
                }
            }
        }

        pub fn __richcmp__(&self, other: &Bound<'_, PyAny>, op: CompareOp) -> bool {
            let eq = other
                .extract::<PyRef<Self>>()
                .is_ok_and(|other| self.0 == other.0);
            match op {
                CompareOp::Eq => eq,
                CompareOp::Ne => !eq,
                _ => false,
            }
        }

        pub fn __hash__(&self) -> isize {
            py_hash(&self.0)
        }
    }

    #[pymethods]
    impl PyFpVector {
        #[new]
        pub fn new(p: u32, len: usize) -> PyResult<Self> {
            Ok(Self(RustFpVector::new(valid_prime(p)?, len)))
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
        pub fn from_slice(p: u32, entries: Vec<u32>) -> PyResult<Self> {
            Ok(Self(RustFpVector::from_slice(valid_prime(p)?, &entries)))
        }

        #[staticmethod]
        pub fn from_bytes(p: u32, len: usize, data: &[u8]) -> PyResult<Self> {
            RustFpVector::from_bytes(valid_prime(p)?, len, &mut Cursor::new(data))
                .map(Self)
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))
        }

        pub fn prime(&self) -> u32 {
            self.0.prime().as_u32()
        }

        pub fn len(&self) -> usize {
            self.0.len()
        }

        pub fn is_empty(&self) -> bool {
            self.0.is_empty()
        }

        pub fn entry(&self, index: usize) -> PyResult<u32> {
            Ok(self.0.entry(checked_index(index, self.0.len())?))
        }

        pub fn density(&self) -> f32 {
            self.0.density()
        }

        pub fn is_zero(&self) -> bool {
            self.0.is_zero()
        }

        pub fn first_nonzero(&self) -> Option<(usize, u32)> {
            self.0.first_nonzero()
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

        pub fn slice_mut(slf: PyRef<'_, Self>, start: usize, end: usize) -> PyResult<PyFpSliceMut> {
            checked_range(start, end, slf.0.len())?;
            let py = slf.py();
            Ok(PyFpSliceMut {
                parent: SliceParent::Vector(slf.into_pyobject(py)?.unbind()),
                start,
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
            let mut buffer = Vec::new();
            self.0
                .to_bytes(&mut buffer)
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
            Ok(PyBytes::new(py, &buffer))
        }

        pub fn update_from_bytes(&mut self, data: &[u8]) -> PyResult<()> {
            self.0
                .update_from_bytes(&mut Cursor::new(data))
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))
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

        pub fn __repr__(&self) -> String {
            format!("FpVector({}, {})", self.prime(), self.0)
        }
    }

    #[pymethods]
    impl PyFpSlice {
        pub fn prime(&self, py: Python<'_>) -> PyResult<u32> {
            self.with_slice(py, |s| s.prime().as_u32())
        }

        pub fn len(&self, py: Python<'_>) -> PyResult<usize> {
            self.with_slice(py, |s| s.len())
        }

        pub fn is_empty(&self, py: Python<'_>) -> PyResult<bool> {
            self.with_slice(py, |s| s.is_empty())
        }

        pub fn entry(&self, py: Python<'_>, index: usize) -> PyResult<u32> {
            let index = checked_index(index, self.span())?;
            self.with_slice(py, |s| s.entry(index))
        }

        pub fn iter(&self, py: Python<'_>) -> PyResult<PyFpVectorIterator> {
            let entries = self.with_slice(py, |s| s.iter().collect())?;
            Ok(PyFpVectorIterator { entries, index: 0 })
        }

        pub fn iter_nonzero(&self, py: Python<'_>) -> PyResult<Vec<(usize, u32)>> {
            self.with_slice(py, |s| s.iter_nonzero().collect())
        }

        pub fn is_zero(&self, py: Python<'_>) -> PyResult<bool> {
            self.with_slice(py, |s| s.is_zero())
        }

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

        pub fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
            self.with_slice(py, |s| format!("FpSlice({}, {})", s.prime().as_u32(), s))
        }
    }

    #[pymethods]
    impl PyFpSliceMut {
        pub fn prime(&self, py: Python<'_>) -> PyResult<u32> {
            self.with_slice(py, |s| s.prime().as_u32())
        }

        pub fn len(&self, py: Python<'_>) -> PyResult<usize> {
            self.with_slice(py, |s| s.len())
        }

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

        pub fn add(&self, py: Python<'_>, other: &PyFpSlice, c: u32) -> PyResult<()> {
            checked_equal_len(self.span(), other.span())?;
            let other_owned = other.to_owned_checked(py)?;
            self.with_slice_mut(py, |mut target| {
                checked_same_prime(target.prime().as_u32(), other_owned.prime().as_u32())?;
                target.add(other_owned.as_slice(), c);
                Ok(())
            })?
        }

        pub fn add_offset(
            &self,
            py: Python<'_>,
            other: &PyFpSlice,
            c: u32,
            offset: usize,
        ) -> PyResult<()> {
            checked_equal_len(self.span(), other.span())?;
            checked_range(offset, self.span(), self.span())?;
            let other_owned = other.to_owned_checked(py)?;
            self.with_slice_mut(py, |mut target| {
                checked_same_prime(target.prime().as_u32(), other_owned.prime().as_u32())?;
                target.add_offset(other_owned.as_slice(), c, offset);
                Ok(())
            })?
        }

        pub fn add_masked(
            &self,
            py: Python<'_>,
            other: &PyFpSlice,
            c: u32,
            mask: Vec<usize>,
        ) -> PyResult<()> {
            checked_equal_len(self.span(), mask.len())?;
            if let Some(&index) = mask.iter().find(|&&index| index >= other.span()) {
                return Err(PyIndexError::new_err(format!(
                    "mask index {index} out of range for vector of length {}",
                    other.span()
                )));
            }
            let other_owned = other.to_owned_checked(py)?;
            self.with_slice_mut(py, |mut target| {
                checked_same_prime(target.prime().as_u32(), other_owned.prime().as_u32())?;
                target.add_masked(other_owned.as_slice(), c, &mask);
                Ok(())
            })?
        }

        pub fn add_unmasked(
            &self,
            py: Python<'_>,
            other: &PyFpSlice,
            c: u32,
            mask: Vec<usize>,
        ) -> PyResult<()> {
            if other.span() > mask.len() {
                return Err(PyValueError::new_err(format!(
                    "mask length {} shorter than source length {}",
                    mask.len(),
                    other.span()
                )));
            }
            if let Some(&index) = mask
                .iter()
                .take(other.span())
                .find(|&&index| index >= self.span())
            {
                return Err(PyIndexError::new_err(format!(
                    "mask index {index} out of range for vector of length {}",
                    self.span()
                )));
            }
            let other_owned = other.to_owned_checked(py)?;
            self.with_slice_mut(py, |mut target| {
                checked_same_prime(target.prime().as_u32(), other_owned.prime().as_u32())?;
                target.add_unmasked(other_owned.as_slice(), c, &mask);
                Ok(())
            })?
        }

        pub fn assign(&self, py: Python<'_>, other: &PyFpSlice) -> PyResult<()> {
            checked_equal_len(self.span(), other.span())?;
            let other_owned = other.to_owned_checked(py)?;
            self.with_slice_mut(py, |mut target| {
                checked_same_prime(target.prime().as_u32(), other_owned.prime().as_u32())?;
                target.assign(other_owned.as_slice());
                Ok(())
            })?
        }

        pub fn add_tensor(
            &self,
            py: Python<'_>,
            offset: usize,
            coeff: u32,
            left: &PyFpSlice,
            right: &PyFpSlice,
        ) -> PyResult<()> {
            let width = left
                .span()
                .checked_mul(right.span())
                .and_then(|width| offset.checked_add(width))
                .ok_or_else(|| PyIndexError::new_err("tensor range overflows usize"))?;
            checked_range(offset, width, self.span())?;
            let left_owned = left.to_owned_checked(py)?;
            let right_owned = right.to_owned_checked(py)?;
            self.with_slice_mut(py, |mut target| {
                checked_same_prime(target.prime().as_u32(), left_owned.prime().as_u32())?;
                checked_same_prime(target.prime().as_u32(), right_owned.prime().as_u32())?;
                target.add_tensor(offset, coeff, left_owned.as_slice(), right_owned.as_slice());
                Ok(())
            })?
        }

        pub fn add_basis_element(&self, py: Python<'_>, index: usize, value: u32) -> PyResult<()> {
            let index = checked_index(index, self.span())?;
            self.with_slice_mut(py, |mut s| s.add_basis_element(index, value))
        }

        pub fn as_slice(&self, py: Python<'_>) -> PyFpSlice {
            PyFpSlice {
                parent: self.parent.clone_ref(py),
                start: self.start,
                end: self.end,
            }
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

        pub fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
            self.with_slice(py, |s| format!("FpSliceMut({}, {})", s.prime().as_u32(), s))
        }
    }

    #[pymethods]
    impl PyMatrix {
        #[new]
        pub fn new(p: u32, rows: usize, columns: usize) -> PyResult<Self> {
            Ok(Self(RustMatrix::new(valid_prime(p)?, rows, columns)))
        }

        #[staticmethod]
        pub fn from_rows(
            p: u32,
            rows: Vec<PyRef<'_, PyFpVector>>,
            columns: usize,
        ) -> PyResult<Self> {
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
        pub fn from_vec(p: u32, input: Vec<Vec<u32>>) -> PyResult<Self> {
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
        pub fn augmented_from_vec(p: u32, input: Vec<Vec<u32>>) -> PyResult<(usize, Self)> {
            let p = valid_prime(p)?;
            if input.is_empty() {
                return Err(PyValueError::new_err(
                    "augmented_from_vec requires at least one row",
                ));
            }
            let columns = input[0].len();
            for row in &input {
                checked_equal_len(row.len(), columns)?;
            }
            let (first_source_column, matrix) = RustMatrix::augmented_from_vec(p, &input);
            Ok((first_source_column, Self(matrix)))
        }

        #[staticmethod]
        pub fn from_bytes(p: u32, rows: usize, columns: usize, data: &[u8]) -> PyResult<Self> {
            RustMatrix::from_bytes(valid_prime(p)?, rows, columns, &mut Cursor::new(data))
                .map(Self)
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))
        }

        pub fn prime(&self) -> u32 {
            self.0.prime().as_u32()
        }

        pub fn rows(&self) -> usize {
            self.0.rows()
        }

        pub fn columns(&self) -> usize {
            self.0.columns()
        }

        pub fn pivots(&self) -> Vec<isize> {
            self.0.pivots().to_vec()
        }

        pub fn is_zero(&self) -> bool {
            self.0.is_zero()
        }

        pub fn to_vec(&self) -> Vec<Vec<u32>> {
            self.0.to_vec()
        }

        pub fn to_bytes<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
            let mut buffer = Vec::new();
            self.0
                .to_bytes(&mut buffer)
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
            Ok(PyBytes::new(py, &buffer))
        }

        pub fn row(slf: PyRef<'_, Self>, row: usize) -> PyResult<PyFpSlice> {
            checked_row(row, slf.0.rows())?;
            let end = slf.0.columns();
            let py = slf.py();
            Ok(PyFpSlice {
                parent: SliceParent::MatrixRow {
                    matrix: slf.into_pyobject(py)?.unbind(),
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
                    matrix: slf.into_pyobject(py)?.unbind(),
                    row,
                },
                start: 0,
                end,
            })
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

        pub fn safe_row_op(&mut self, target: usize, source: usize, c: u32) -> PyResult<()> {
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
                    matrix: parent,
                    row,
                },
                start: 0,
                end,
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
            self.0.trim(row_start, row_end, col_start);
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
                .map_err(|e| PyValueError::new_err(e.to_string()))
        }

        pub fn prime(&self) -> u32 {
            self.0.prime().as_u32()
        }

        pub fn dimension(&self) -> usize {
            self.0.dimension()
        }

        pub fn ambient_dimension(&self) -> usize {
            self.0.ambient_dimension()
        }

        pub fn contains(&self, vector: &PyFpVector) -> PyResult<bool> {
            self.check_compatible(&vector.0)?;
            Ok(self.0.contains(vector.0.as_slice()))
        }

        pub fn contains_space(&self, other: &Self) -> PyResult<bool> {
            checked_same_prime(self.0.prime().as_u32(), other.0.prime().as_u32())?;
            checked_equal_len(self.0.ambient_dimension(), other.0.ambient_dimension())?;
            Ok(self.0.contains_space(&other.0))
        }

        pub fn add_vector(&mut self, vector: &PyFpVector) -> PyResult<usize> {
            self.check_compatible(&vector.0)?;
            Ok(self.0.add_vector(vector.0.as_slice()))
        }

        /// Reduce `vector` in place against this subspace, projecting it onto a
        /// complement of the subspace.
        pub fn reduce(&self, vector: &mut PyFpVector) -> PyResult<()> {
            self.check_compatible(&vector.0)?;
            self.0.reduce(vector.0.as_slice_mut());
            Ok(())
        }

        pub fn sum(&self, other: &Self) -> PyResult<Self> {
            checked_same_prime(self.0.prime().as_u32(), other.0.prime().as_u32())?;
            checked_equal_len(self.0.ambient_dimension(), other.0.ambient_dimension())?;
            // `Subspace::sum` calls `Matrix::trim` after `from_matrix`, which
            // discards the matrix pivots and leaves the returned subspace with
            // an empty pivot table (so `dimension`/`iter`/`reduce` would all
            // misbehave). Re-wrap the resulting matrix through `from_matrix` to
            // re-row-reduce and rebuild the pivots before exposing it.
            let summed = self.0.sum(&other.0);
            Ok(Self(RustSubspace::from_matrix((*summed).clone())))
        }

        /// Return the basis of the subspace as a list of owned `FpVector`s.
        pub fn iter(&self) -> Vec<PyFpVector> {
            self.0
                .iter()
                .map(|row| PyFpVector(row.to_owned()))
                .collect()
        }

        /// Return every vector in the subspace as a list of owned `FpVector`s.
        pub fn iter_all_vectors(&self) -> Vec<PyFpVector> {
            self.0.iter_all_vectors().map(PyFpVector).collect()
        }

        pub fn set_to_zero(&mut self) {
            self.0.set_to_zero()
        }

        pub fn set_to_entire(&mut self) {
            self.0.set_to_entire()
        }

        pub fn to_bytes<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
            let mut buffer = Vec::new();
            self.0
                .to_bytes(&mut buffer)
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
            Ok(PyBytes::new(py, &buffer))
        }

        pub fn __len__(&self) -> usize {
            self.0.dimension()
        }

        pub fn __contains__(&self, vector: &PyFpVector) -> PyResult<bool> {
            self.contains(vector)
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

    #[pyfunction]
    fn power_mod(p: u32, b: u32, e: u32) -> PyResult<u32> {
        Ok(valid_prime(p)?.pow_mod(b, e))
    }

    #[pyfunction]
    fn log2(n: usize) -> usize {
        prime::log2(n)
    }

    #[pyfunction]
    fn logp(p: u32, n: u32) -> PyResult<u32> {
        Ok(prime::logp(valid_prime(p)?, n))
    }

    #[pyfunction]
    fn factor_pk(p: u32, n: u32) -> PyResult<(u32, u32)> {
        Ok(prime::factor_pk(valid_prime(p)?, n))
    }

    #[pyfunction]
    fn inverse(p: u32, k: u32) -> PyResult<u32> {
        Ok(prime::inverse(valid_prime(p)?, k))
    }

    #[pyfunction]
    fn minus_one_to_the_n(p: u32, i: i32) -> PyResult<u32> {
        Ok(prime::minus_one_to_the_n(valid_prime(p)?, i))
    }

    #[pyfunction]
    fn is_prime(p: u32) -> bool {
        valid_prime(p).is_ok()
    }

    #[pyfunction]
    fn binomial(p: u32, n: u32, k: u32) -> PyResult<u32> {
        Ok(u32::binomial(table_prime(p)?, n, k))
    }

    #[pyfunction]
    fn multinomial(p: u32, mut l: Vec<u32>) -> PyResult<u32> {
        Ok(u32::multinomial(table_prime(p)?, &mut l))
    }

    #[pyfunction]
    fn binomial_odd_is_zero(p: u32, n: u32, k: u32) -> PyResult<bool> {
        Ok(u32::binomial_odd_is_zero(table_prime(p)?, n, k))
    }

    #[pyfunction]
    fn binomial2(n: u32, k: u32) -> u32 {
        u32::binomial2(n, k)
    }

    #[pyfunction]
    fn multinomial2(l: Vec<u32>) -> u32 {
        u32::multinomial2(&l)
    }

    #[pyfunction]
    fn binomial4(n: u32, k: u32) -> u32 {
        u32::binomial4(n, k)
    }

    #[pyfunction]
    fn binomial4_rec(n: u32, k: u32) -> u32 {
        u32::binomial4_rec(n, k)
    }

    #[pymodule_init]
    fn init(m: &Bound<'_, PyModule>) -> PyResult<()> {
        m.add("F2", PyFp(DynFp::new(prime::TWO)))?;
        m.add("F3", PyFp(DynFp::new(prime::P3.to_dyn())))?;
        m.add("F5", PyFp(DynFp::new(prime::P5.to_dyn())))?;
        m.add("F7", PyFp(DynFp::new(prime::P7.to_dyn())))?;
        m.add("TWO", prime::TWO.as_u32())?;
        m.add("PRIMES", fp::PRIMES.to_vec())?;
        m.add("NUM_PRIMES", fp::NUM_PRIMES)?;
        m.add("PRIME_TO_INDEX_MAP", fp::PRIME_TO_INDEX_MAP.to_vec())?;
        m.add("MAX_MULTINOMIAL_LEN", fp::MAX_MULTINOMIAL_LEN)?;
        m.add("ODD_PRIMES", fp::ODD_PRIMES)?;
        Ok(())
    }
}
