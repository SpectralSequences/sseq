use pyo3::prelude::*;

#[pymodule]
#[pyo3(name = "fp")]
pub mod fp_py {
    use fp::field::{
        element::FieldElement as RustFieldElement, Field, Fp as RustFp, SmallFq as RustSmallFq,
    };
    use fp::matrix::{
        AffineSubspace as RustAffineSubspace, AugmentedMatrix as RustAugmentedMatrix,
        Matrix as RustMatrix, QuasiInverse as RustQuasiInverse, Subquotient as RustSubquotient,
        Subspace as RustSubspace,
    };
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

    #[pyclass(name = "QuasiInverse")]
    struct PyQuasiInverse(RustQuasiInverse);

    #[pyclass(name = "Subquotient")]
    struct PySubquotient(RustSubquotient);

    #[pyclass(name = "AffineSubspace")]
    struct PyAffineSubspace(RustAffineSubspace);

    /// Lazy iterator over every vector in a subspace.
    ///
    /// The upstream `Subspace::iter_all_vectors` iterator borrows the subspace,
    /// so it cannot be stored alongside an owned subspace in a `#[pyclass]`
    /// without a self-referential struct. Instead we own a clone of the
    /// subspace and an index counter, regenerating the i-th vector on each
    /// `__next__` from the base-`p` decomposition of the index. This keeps
    /// iteration lazy (O(1) memory) while yielding the same owned `FpVector`s
    /// in the same order as the eager version.
    #[pyclass(name = "SubspaceVectorIterator")]
    struct PySubspaceVectorIterator {
        subspace: RustSubspace,
        index: u128,
        total: u128,
    }

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

        /// Compute the quasi-inverse of a row-reduced augmented matrix `[A|0|I]`.
        ///
        /// `last_target_col` is the last column of `A`, and `first_source_col`
        /// is the first column of `I` (typically the padded column count
        /// returned by `augmented_from_vec`). The matrix is expected to already
        /// be row reduced.
        pub fn compute_quasi_inverse(
            &self,
            last_target_col: usize,
            first_source_col: usize,
        ) -> PyResult<PyQuasiInverse> {
            let columns = self.0.columns();
            if last_target_col > columns {
                return Err(PyIndexError::new_err(format!(
                    "last_target_col {last_target_col} out of range for matrix with {columns} columns"
                )));
            }
            if first_source_col > columns {
                return Err(PyIndexError::new_err(format!(
                    "first_source_col {first_source_col} out of range for matrix with {columns} columns"
                )));
            }
            Ok(PyQuasiInverse(
                self.0
                    .compute_quasi_inverse(last_target_col, first_source_col),
            ))
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
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))
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
            self.check_compatible_space(&other.0)?;
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
            Ok(Self(self.0.sum(&other.0)))
        }

        /// Return the basis of the subspace as a list of owned `FpVector`s.
        pub fn iter(&self) -> Vec<PyFpVector> {
            self.0
                .iter()
                .map(|row| PyFpVector(row.to_owned()))
                .collect()
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

    /// Extract an owned copy of a vector-like argument (`FpVector` or
    /// `FpSlice`) for use as an immutable input.
    fn extract_input_owned(py: Python<'_>, obj: &Bound<'_, PyAny>) -> PyResult<RustFpVector> {
        if let Ok(vector) = obj.extract::<PyRef<'_, PyFpVector>>() {
            Ok(vector.0.clone())
        } else if let Ok(slice) = obj.extract::<PyRef<'_, PyFpSlice>>() {
            slice.to_owned_checked(py)
        } else {
            Err(PyValueError::new_err("expected an FpVector or FpSlice"))
        }
    }

    /// Run `f` on the mutable slice backing a vector-like argument
    /// (`FpVector` or `FpSliceMut`), used as an output target.
    fn with_target_slice_mut<R>(
        py: Python<'_>,
        obj: &Bound<'_, PyAny>,
        f: impl FnOnce(RustFpSliceMut<'_>) -> PyResult<R>,
    ) -> PyResult<R> {
        if let Ok(mut vector) = obj.extract::<PyRefMut<'_, PyFpVector>>() {
            f(vector.0.as_slice_mut())
        } else if let Ok(slice) = obj.extract::<PyRef<'_, PyFpSliceMut>>() {
            slice.with_slice_mut(py, f)?
        } else {
            Err(PyValueError::new_err("expected an FpVector or FpSliceMut"))
        }
    }

    #[pymethods]
    impl PyQuasiInverse {
        /// Construct a `QuasiInverse` from an optional `image` (pivot list) and a
        /// `preimage` matrix.
        ///
        /// # Invariant enforced
        ///
        /// `apply` (and `stream_quasi_inverse`) walk `image` and, for every
        /// non-negative pivot entry, consume one row of `preimage` (the rows are
        /// addressed by a running counter that increments once per non-negative
        /// pivot). Upstream `QuasiInverse::new` performs no validation, so without
        /// the checks below a Python caller could supply an `image` whose count of
        /// non-negative pivots exceeds `preimage.rows()`, causing `apply` to index
        /// `preimage.row(row)` out of bounds and panic across the PyO3 boundary.
        ///
        /// We therefore require, when `image` is `Some`:
        ///  * the number of non-negative pivot entries is `<= preimage.rows()`
        ///    (this is the exact invariant that makes `apply` safe), and
        ///  * every non-negative pivot is a valid `preimage` row index, i.e. in
        ///    `0..preimage.rows()` (pivots are row indices into `preimage`).
        ///
        /// When `image` is `None` the image is the standard basis (identity) and
        /// no pivot validation is needed; that path is always safe.
        #[new]
        #[pyo3(signature = (image, preimage))]
        pub fn new(image: Option<Vec<isize>>, preimage: &PyMatrix) -> PyResult<Self> {
            if let Some(pivots) = image.as_ref() {
                let rows = preimage.0.rows();
                let mut nonneg = 0usize;
                for &p in pivots {
                    if p >= 0 {
                        nonneg += 1;
                        if (p as usize) >= rows {
                            return Err(PyValueError::new_err(format!(
                                "inconsistent QuasiInverse: pivot {p} is out of range for a \
                                 preimage with {rows} rows"
                            )));
                        }
                    }
                }
                if nonneg > rows {
                    return Err(PyValueError::new_err(format!(
                        "inconsistent QuasiInverse: image has {nonneg} non-negative pivots but \
                         preimage only has {rows} rows"
                    )));
                }
            }
            Ok(Self(RustQuasiInverse::new(image, preimage.0.clone())))
        }

        /// Deserialize a `QuasiInverse` from bytes produced by [`Self::to_bytes`].
        ///
        /// Note on `image = None`: serialization does not preserve a `None` image.
        /// [`Self::to_bytes`] writes a `None` image as an explicit identity pivot
        /// list `[0, 1, 2, ...]` (matching upstream), so a quasi-inverse built with
        /// `image=None` round-trips to one whose `pivots()` are `Some([0, 1, ...])`
        /// rather than `None`. This is intended upstream behavior and is not changed
        /// here.
        #[staticmethod]
        pub fn from_bytes(p: u32, data: &[u8]) -> PyResult<Self> {
            RustQuasiInverse::from_bytes(valid_prime(p)?, &mut Cursor::new(data))
                .map(Self)
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))
        }

        pub fn prime(&self) -> u32 {
            self.0.prime().as_u32()
        }

        pub fn image_dimension(&self) -> usize {
            self.0.image_dimension()
        }

        pub fn source_dimension(&self) -> usize {
            self.0.source_dimension()
        }

        pub fn target_dimension(&self) -> usize {
            self.0.target_dimension()
        }

        pub fn preimage(&self) -> PyMatrix {
            PyMatrix(self.0.preimage().clone())
        }

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
            let input_owned = extract_input_owned(py, input)?;
            checked_same_prime(self.0.prime().as_u32(), input_owned.prime().as_u32())?;
            checked_equal_len(input_owned.len(), self.0.target_dimension())?;
            with_target_slice_mut(py, target, |target_slice| {
                checked_same_prime(
                    self.0.prime().as_u32(),
                    target_slice.as_slice().prime().as_u32(),
                )?;
                checked_equal_len(target_slice.as_slice().len(), self.0.source_dimension())?;
                // Reduce `coeff` mod p before calling upstream. Upstream computes
                // `(coeff * c) % p`; with `c < p` and an unreduced `coeff` the
                // product `coeff * c` can overflow u32 (debug panic / wrong result
                // in release). Reducing first is mathematically equivalent since
                // `(coeff % p) * c % p == coeff * c % p`.
                let coeff = coeff % self.0.prime().as_u32();
                self.0.apply(target_slice, coeff, input_owned.as_slice());
                Ok(())
            })
        }

        /// Serialize the quasi-inverse to bytes.
        ///
        /// Note: a `None` image (identity) is serialized as an explicit identity
        /// pivot list `[0, 1, 2, ...]` (matching upstream), so it does not survive
        /// a round-trip as `None`; see [`Self::from_bytes`].
        pub fn to_bytes<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
            let mut buffer = Vec::new();
            self.0
                .to_bytes(&mut buffer)
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
            Ok(PyBytes::new(py, &buffer))
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

        pub fn prime(&self) -> u32 {
            self.0.prime().as_u32()
        }

        pub fn dimension(&self) -> usize {
            self.0.dimension()
        }

        pub fn ambient_dimension(&self) -> usize {
            self.0.ambient_dimension()
        }

        pub fn quotient_dimension(&self) -> usize {
            self.0.quotient_dimension()
        }

        pub fn subspace_dimension(&self) -> usize {
            self.0.subspace_dimension()
        }

        pub fn is_empty(&self) -> bool {
            self.0.is_empty()
        }

        /// The quotient (zero) subspace of the subquotient, returned as an owned
        /// `Subspace`.
        pub fn zeros(&self) -> PySubspace {
            PySubspace(self.0.zeros().clone())
        }

        /// The generators of the subquotient, returned as a list of owned
        /// `FpVector`s. Mirrors the choice made for `Subspace.iter`: the
        /// upstream iterator borrows the subquotient, so we materialize owned
        /// vectors rather than expose borrowed slice handles.
        pub fn gens(&self) -> Vec<PyFpVector> {
            self.0
                .gens()
                .map(|row| PyFpVector(row.to_owned()))
                .collect()
        }

        /// The generators of the subspace part of the subquotient, returned as
        /// a list of owned `FpVector`s (see `gens` for the ownership choice).
        pub fn subspace_gens(&self) -> Vec<PyFpVector> {
            self.0
                .subspace_gens()
                .map(|row| PyFpVector(row.to_owned()))
                .collect()
        }

        /// The pivot columns of the complement to the subspace.
        pub fn complement_pivots(&self) -> Vec<usize> {
            self.0.complement_pivots().collect()
        }

        /// The pivot table of the quotient subspace.
        pub fn quotient_pivots(&self) -> Vec<isize> {
            self.0.quotient_pivots().to_vec()
        }

        /// Reduce `vector` in place: project it onto a complement of the
        /// quotient and express it relative to the generators. Returns the list
        /// of coefficients with respect to the generators. After the call,
        /// `vector` holds the residual; a nonzero residual means the vector was
        /// not in the subspace.
        pub fn reduce(&self, vector: &mut PyFpVector) -> PyResult<Vec<u32>> {
            self.check_compatible(&vector.0)?;
            Ok(self.0.reduce(vector.0.as_slice_mut()))
        }

        /// Project `vector` in place onto the complement of the quotient part.
        pub fn reduce_by_quotient(&self, vector: &mut PyFpVector) -> PyResult<()> {
            self.check_compatible(&vector.0)?;
            self.0.reduce_by_quotient(vector.0.as_slice_mut());
            Ok(())
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
        /// in `target`, returning the coefficient lists. `matrix` must map the
        /// ambient space of `source` into the ambient space of `target`.
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
        ///
        /// Upstream `AffineSubspace::new` `assert_eq!`s that the offset length
        /// matches the linear part's ambient dimension and reduces the offset
        /// against the linear part (which requires a shared prime), so we
        /// pre-check both here to raise `ValueError` instead of panicking.
        #[new]
        pub fn new(offset: &PyFpVector, linear_part: &PySubspace) -> PyResult<Self> {
            checked_same_prime(offset.0.prime().as_u32(), linear_part.0.prime().as_u32())?;
            checked_equal_len(offset.0.len(), linear_part.0.ambient_dimension())?;
            Ok(Self(RustAffineSubspace::new(
                offset.0.clone(),
                linear_part.0.clone(),
            )))
        }

        pub fn prime(&self) -> u32 {
            self.0.linear_part().prime().as_u32()
        }

        pub fn ambient_dimension(&self) -> usize {
            self.0.linear_part().ambient_dimension()
        }

        pub fn dimension(&self) -> usize {
            self.0.linear_part().dimension()
        }

        /// Return an owned copy of the (reduced) offset vector.
        ///
        /// We return an owned `FpVector` rather than a borrowed view, matching
        /// the owned-return precedent used by `Subspace`/`Subquotient`. The
        /// offset stored upstream is the input reduced against the linear part,
        /// so it may differ from the vector passed to `new`.
        pub fn offset(&self) -> PyFpVector {
            PyFpVector(self.0.offset().clone())
        }

        /// Return an owned copy (clone) of the linear part `Subspace`,
        /// consistent with the owned-return precedent.
        pub fn linear_part(&self) -> PySubspace {
            PySubspace(self.0.linear_part().clone())
        }

        /// Test whether `vector` (an `FpVector` or `FpSlice`) lies in this
        /// affine subspace.
        pub fn contains(&self, py: Python<'_>, vector: &Bound<'_, PyAny>) -> PyResult<bool> {
            let vector = extract_input_owned(py, vector)?;
            checked_same_prime(self.prime(), vector.prime().as_u32())?;
            checked_equal_len(vector.len(), self.ambient_dimension())?;
            Ok(self.0.contains(vector.as_slice()))
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

    /// `AugmentedMatrix<N>` is a const-generic type, and PyO3 cannot expose a
    /// generic `#[pyclass]`. We therefore bind the two concrete arities used in
    /// the codebase (`N = 2` and `N = 3`) as separate classes `AugmentedMatrix2`
    /// and `AugmentedMatrix3`. To avoid duplicating the shared glue, this
    /// `macro_rules!` macro generates each class from a single definition; the
    /// per-arity methods are spliced in through the `$extra` token block. Each
    /// generated class still goes through `#[pyclass]` / `#[pymethods]`, so this
    /// is not hand-desugared PyO3 registration. However, the `#[pymodule]`
    /// proc-macro cannot see through a `macro_rules!` expansion to auto-collect
    /// the classes, so they are registered explicitly with `add_class` in
    /// `#[pymodule_init]`.
    macro_rules! augmented_matrix_pyclass {
        ($name:ident, $pyname:literal, $n:literal, { $($extra:tt)* }) => {
            #[pyclass(name = $pyname)]
            struct $name(RustAugmentedMatrix<$n>);

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
                    Ok(Self(RustAugmentedMatrix::<$n>::new(
                        valid_prime(p)?,
                        rows,
                        cols,
                    )))
                }

                fn prime(&self) -> u32 {
                    self.0.prime().as_u32()
                }

                fn rows(&self) -> usize {
                    self.0.rows()
                }

                fn columns(&self) -> usize {
                    self.0.columns()
                }

                /// Number of column segments (`N`).
                fn segments(&self) -> usize {
                    $n
                }

                /// The starting column index of each segment.
                fn segment_starts(&self) -> Vec<usize> {
                    self.0.start.to_vec()
                }

                /// The (exclusive) ending column index of each segment.
                fn segment_ends(&self) -> Vec<usize> {
                    self.0.end.to_vec()
                }

                fn pivots(&self) -> Vec<isize> {
                    self.0.pivots().to_vec()
                }

                fn is_zero(&self) -> bool {
                    self.0.is_zero()
                }

                fn to_vec(&self) -> Vec<Vec<u32>> {
                    self.0.to_vec()
                }

                fn row_reduce(&mut self) -> usize {
                    self.0.row_reduce()
                }

                /// Add an identity matrix into the rectangular segment spanning
                /// segment indices `start..=end`. The segment must be square
                /// (its row count equals its column width), matching upstream's
                /// `MatrixSliceMut::add_identity` invariant; otherwise a
                /// `ValueError` is raised rather than panicking.
                fn add_identity(&mut self, start: usize, end: usize) -> PyResult<()> {
                    let cols = segment_cols(&self.0, start, end)?;
                    if self.0.rows() != cols {
                        return Err(PyValueError::new_err(format!(
                            "add_identity requires a square segment: matrix has {} rows but \
                             segment [{start}, {end}] has {cols} columns",
                            self.0.rows()
                        )));
                    }
                    self.0.segment(start, end).add_identity();
                    Ok(())
                }

                /// Return an owned copy of row `i` restricted to the columns of
                /// the segment range `start..=end`.
                ///
                /// Upstream `row_segment` returns a borrowed `FpSlice`. We copy
                /// into an owned `FpVector` instead, matching the owned-return
                /// precedent used elsewhere (e.g. `Subspace.iter`) and avoiding
                /// the borrowed-view machinery; the mutable `row_segment_mut`
                /// and rectangle-returning `segment` are deferred (see below).
                fn row_segment(
                    &self,
                    i: usize,
                    start: usize,
                    end: usize,
                ) -> PyResult<PyFpVector> {
                    checked_row(i, self.0.rows())?;
                    segment_cols(&self.0, start, end)?;
                    Ok(PyFpVector(self.0.row_segment(i, start, end).to_owned()))
                }

                /// Compute the kernel of the augmented matrix (which must be row
                /// reduced), returning an owned `Subspace`. Available for all
                /// arities.
                fn compute_kernel(&self) -> PySubspace {
                    PySubspace(self.0.compute_kernel())
                }

                /// Return the inner `Matrix` as an owned `Matrix`.
                ///
                /// Upstream `into_matrix` consumes `self`, but PyO3 methods
                /// borrow the pyclass and cannot move out of it, so we clone the
                /// inner matrix. The augmented matrix remains usable afterward.
                fn into_matrix(&self) -> PyMatrix {
                    PyMatrix(self.0.inner.clone())
                }

                fn __repr__(&self) -> String {
                    format!(
                        concat!($pyname, "({}, {}x{})"),
                        self.0.prime().as_u32(),
                        self.0.rows(),
                        self.0.columns()
                    )
                }

                $($extra)*
            }
        };
    }

    augmented_matrix_pyclass!(PyAugmentedMatrix2, "AugmentedMatrix2", 2, {
        /// Compute the image of the augmented matrix `[A | I]` (which must be
        /// row reduced), returning an owned `Subspace`.
        fn compute_image(&self) -> PySubspace {
            PySubspace(self.0.compute_image())
        }

        /// Compute the quasi-inverse of the augmented matrix `[A | I]` (which
        /// must be row reduced), returning an owned `QuasiInverse`.
        fn compute_quasi_inverse(&self) -> PyQuasiInverse {
            PyQuasiInverse(self.0.compute_quasi_inverse())
        }
    });

    augmented_matrix_pyclass!(PyAugmentedMatrix3, "AugmentedMatrix3", 3, {
        /// Compute the two quasi-inverses for a row-reduced augmented matrix of
        /// the form `[A | 0 | B | 0 | I]` where `A` is surjective, returning the
        /// pair `(quasi_inverse_of_A, residual_quasi_inverse)`.
        ///
        /// Upstream `compute_quasi_inverses` consumes and heavily mutates the
        /// matrix; since PyO3 cannot move out of a borrowed pyclass we operate
        /// on a clone, leaving the original augmented matrix unchanged.
        fn compute_quasi_inverses(&self) -> (PyQuasiInverse, PyQuasiInverse) {
            let (a, b) = self.0.clone().compute_quasi_inverses();
            (PyQuasiInverse(a), PyQuasiInverse(b))
        }
    });

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
        // The `AugmentedMatrix2`/`AugmentedMatrix3` classes are produced by a
        // `macro_rules!` macro, which the `#[pymodule]` proc-macro cannot see
        // through to auto-collect, so register them explicitly here.
        m.add_class::<PyAugmentedMatrix2>()?;
        m.add_class::<PyAugmentedMatrix3>()?;
        Ok(())
    }
}
