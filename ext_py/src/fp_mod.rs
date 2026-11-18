use std::hash::{DefaultHasher, Hash, Hasher};

use fp::{
    field::{DivError, DynFieldElement, Field, Fp as RustFp, SmallFq as RustSmallFq},
    matrix::Matrix as RustMatrix,
    prime::{self, Prime},
};
use pyo3::{
    basic::CompareOp,
    exceptions::{PyIndexError, PyRuntimeError, PyValueError, PyZeroDivisionError},
    prelude::*,
    types::PyBytes,
    PyResult,
};

mod matrices;
mod vectors;

const MAX_VALID_PRIME: u32 = 1 << 31;

type DynFp = RustFp<prime::ValidPrime>;
type DynSmallFq = RustSmallFq<prime::ValidPrime>;

fn valid_prime(p: u32) -> PyResult<prime::ValidPrime> {
    if !(2..MAX_VALID_PRIME).contains(&p) {
        return Err(PyValueError::new_err(format!("{p} is not prime")));
    }
    prime::ValidPrime::try_from(p).map_err(|_| PyValueError::new_err(format!("{p} is not prime")))
}

/// Build a `SmallFq` from a Python-supplied prime and degree.
fn small_fq(p: u32, degree: u32) -> PyResult<DynSmallFq> {
    let p = valid_prime(p)?;
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

fn checked_range(start: usize, end: usize, len: usize) -> PyResult<()> {
    if start <= end && end <= len {
        Ok(())
    } else {
        Err(PyIndexError::new_err(format!(
            "range {start}..{end} out of range for vector of length {len}"
        )))
    }
}

pub(crate) fn checked_row(row: usize, rows: usize) -> PyResult<usize> {
    if row < rows {
        Ok(row)
    } else {
        Err(PyIndexError::new_err(format!(
            "row {row} out of range for matrix with {rows} rows"
        )))
    }
}

fn borrow_error(err: impl ToString) -> PyErr {
    PyRuntimeError::new_err(err.to_string())
}

/// Map any stringifiable error (e.g. `std::io::Error` from
/// (de)serialization) into the `RuntimeError` used uniformly across the
/// `to_bytes`/`from_bytes` methods.
fn io_err(e: impl ToString) -> PyErr {
    PyRuntimeError::new_err(e.to_string())
}

/// Run a `to_bytes`-style writer into a fresh buffer and wrap the result as
/// `PyBytes`, mapping I/O errors through [`io_err`].
fn serialize_to_pybytes<'py>(
    py: Python<'py>,
    f: impl FnOnce(&mut Vec<u8>) -> std::io::Result<()>,
) -> PyResult<Bound<'py, PyBytes>> {
    let mut buffer = Vec::new();
    f(&mut buffer).map_err(io_err)?;
    Ok(PyBytes::new(py, &buffer))
}

/// Uniform error for using a value that has been moved out (consumed) by a
/// consuming method. Mirrors `borrow_error` for the move-and-invalidate
/// pyclasses (e.g. the augmented matrices).
fn consumed_error(label: &str) -> PyErr {
    PyRuntimeError::new_err(format!("{label} has been consumed"))
}

/// A value that a consuming method can `take()` out, after which any further
/// access raises `RuntimeError("<label> has been consumed")` instead of
/// panicking or operating on stale data. Used to model upstream consuming
/// semantics (`into_*`, `compute_quasi_inverses`) across the PyO3 boundary,
/// where methods borrow the pyclass and cannot move out of `self` directly.
pub(crate) struct Consumable<T> {
    value: Option<T>,
    label: &'static str,
}

impl<T> Consumable<T> {
    pub(crate) fn new(label: &'static str, value: T) -> Self {
        Self {
            value: Some(value),
            label,
        }
    }

    pub(crate) fn get(&self) -> PyResult<&T> {
        self.value
            .as_ref()
            .ok_or_else(|| consumed_error(self.label))
    }

    pub(crate) fn get_mut(&mut self) -> PyResult<&mut T> {
        self.value
            .as_mut()
            .ok_or_else(|| consumed_error(self.label))
    }

    pub(crate) fn take(&mut self) -> PyResult<T> {
        self.value.take().ok_or_else(|| consumed_error(self.label))
    }
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

/// Python `repr` of the field an erased element lives in (e.g. `Fp(2)` or
/// `SmallFq(2, 3)`), matching the `__repr__` of the `Fp`/`SmallFq` classes.
/// Used to build the mismatched-field error message.
fn field_repr(x: DynFieldElement) -> String {
    match x {
        DynFieldElement::Fp(x) => format!("Fp({})", x.field().characteristic().as_u32()),
        DynFieldElement::SmallFq(x) => {
            let f = x.field();
            format!("SmallFq({}, {})", f.characteristic().as_u32(), f.degree())
        }
    }
}

/// The `ValueError` raised when a binary field operation is given operands
/// from two different fields (upstream signals this as `None`/`MismatchedField`).
fn mismatched_field_error(lhs: DynFieldElement, rhs: DynFieldElement) -> PyErr {
    PyValueError::new_err(format!(
        "cannot combine elements from {} and {}",
        field_repr(lhs),
        field_repr(rhs)
    ))
}

/// A matrix-like parent that can back a borrowed row or rectangle view.
///
/// A plain `Matrix` is held directly; an `AugmentedMatrix<N>` is held as its
/// concrete pyclass and accessed through its `Deref<Target = Matrix>` so
/// that segment rectangles and segment rows can revalidate against the inner
/// matrix's current dimensions. We keep the parent Python object alive and
/// reconstruct the underlying Rust matrix view on each call.
pub(crate) enum MatrixParent {
    Matrix(Py<matrices::PyMatrix>),
    Augmented2(Py<matrices::PyAugmentedMatrix2>),
    Augmented3(Py<matrices::PyAugmentedMatrix3>),
}

impl MatrixParent {
    fn clone_ref(&self, py: Python<'_>) -> Self {
        match self {
            Self::Matrix(m) => Self::Matrix(m.clone_ref(py)),
            Self::Augmented2(m) => Self::Augmented2(m.clone_ref(py)),
            Self::Augmented3(m) => Self::Augmented3(m.clone_ref(py)),
        }
    }

    /// Run `f` on the current inner `Matrix`, holding the borrow for the
    /// duration of the call. Deref coercion turns an `&AugmentedMatrix<N>`
    /// into the `&Matrix` expected by `f`.
    fn with_matrix<R>(&self, py: Python<'_>, f: impl FnOnce(&RustMatrix) -> R) -> PyResult<R> {
        match self {
            Self::Matrix(m) => Ok(f(&m.try_borrow(py).map_err(borrow_error)?.0)),
            Self::Augmented2(m) => Ok(f(m.try_borrow(py).map_err(borrow_error)?.0.get()?)),
            Self::Augmented3(m) => Ok(f(m.try_borrow(py).map_err(borrow_error)?.0.get()?)),
        }
    }

    /// Whether `self` and `other` are backed by the same Python object
    /// (same `Matrix`/`AugmentedMatrix` instance). Different enum variants
    /// are necessarily different objects. Used to decide whether a shared
    /// borrow of one and a mutable borrow of the other would collide.
    fn same_object(&self, py: Python<'_>, other: &MatrixParent) -> bool {
        match (self, other) {
            (Self::Matrix(a), Self::Matrix(b)) => a.bind(py).is(b.bind(py)),
            (Self::Augmented2(a), Self::Augmented2(b)) => a.bind(py).is(b.bind(py)),
            (Self::Augmented3(a), Self::Augmented3(b)) => a.bind(py).is(b.bind(py)),
            _ => false,
        }
    }

    /// Run `f` on the current inner `Matrix` mutably, holding the borrow for
    /// the duration of the call.
    fn with_matrix_mut<R>(
        &self,
        py: Python<'_>,
        f: impl FnOnce(&mut RustMatrix) -> R,
    ) -> PyResult<R> {
        match self {
            Self::Matrix(m) => Ok(f(&mut m.try_borrow_mut(py).map_err(borrow_error)?.0)),
            Self::Augmented2(m) => Ok(f(m
                .try_borrow_mut(py)
                .map_err(borrow_error)?
                .0
                .get_mut()?)),
            Self::Augmented3(m) => Ok(f(m
                .try_borrow_mut(py)
                .map_err(borrow_error)?
                .0
                .get_mut()?)),
        }
    }
}

/// The source backing a slice handle: either an owned vector, or a row of a
/// matrix-like parent. In both cases we keep the parent Python object alive
/// and store enough metadata to reconstruct the underlying Rust slice on
/// each call, revalidating against the parent's current dimensions first.
pub(crate) enum SliceParent {
    Vector(Py<vectors::PyFpVector>),
    MatrixRow { matrix: MatrixParent, row: usize },
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

    /// Whether `self` and `other` are backed by the same Python object, so that
    /// taking a shared borrow of one while the other is mutably borrowed would
    /// collide in PyO3.
    ///
    /// Two `MatrixRow`s of the same matrix object count as the same object
    /// regardless of row index: the whole `Matrix` pyclass is borrowed as a
    /// unit.
    fn same_object(&self, py: Python<'_>, other: &SliceParent) -> bool {
        match (self, other) {
            (Self::Vector(a), Self::Vector(b)) => a.bind(py).is(b.bind(py)),
            (Self::MatrixRow { matrix: a, .. }, Self::MatrixRow { matrix: b, .. }) => {
                a.same_object(py, b)
            }
            _ => false,
        }
    }

    /// Whether `self` is backed by the same Python object as the bound
    /// `FpVector` `other`.
    fn same_vector(&self, py: Python<'_>, other: &Bound<'_, vectors::PyFpVector>) -> bool {
        match self {
            Self::Vector(v) => v.bind(py).is(other),
            _ => false,
        }
    }
}

#[pymodule]
#[pyo3(name = "fp")]
pub mod fp_py {
    #[pymodule_export]
    pub use super::matrices::{
        PyAffineSubspace, PyMatrix, PyMatrixSliceMut, PyQuasiInverse, PySubquotient, PySubspace,
        PySubspaceVectorIterator,
    };
    #[pymodule_export]
    pub use super::vectors::{PyFpSlice, PyFpSliceMut, PyFpVector, PyFpVectorIterator};
    use super::{
        matrices::{PyAugmentedMatrix2, PyAugmentedMatrix3},
        *,
    };

    #[pyclass(name = "Fp", frozen, from_py_object)]
    #[derive(Clone, Copy)]
    struct PyFp(DynFp);

    #[pyclass(name = "SmallFq", frozen, from_py_object)]
    #[derive(Clone, Copy)]
    struct PySmallFq(DynSmallFq);

    #[pyclass(name = "FieldElement", frozen, from_py_object)]
    #[derive(Clone, Copy)]
    struct PyFieldElement(DynFieldElement);

    /// The value-equality field types (`PyFp`, `PySmallFq`, `PyFieldElement`)
    /// all expose byte-identical `__richcmp__`/`__hash__` methods: equality
    /// only compares against another instance of the same class (`PyRef<Self>`)
    /// on the wrapped value, `Eq`/`Ne` are the only supported operators, and
    /// hashes go through the shared `py_hash` helper.
    ///
    /// This crate does not enable PyO3's `multiple-pymethods` feature, so each
    /// class may have only one `#[pymethods]` block, and PyO3's `#[pymethods]`
    /// proc-macro additionally rejects `macro_rules!` invocations *inside* the
    /// block. So this macro emits the entire `#[pymethods]` block.
    macro_rules! eq_hash_pymethods {
        ($ty:ident, { $($extra:tt)* }) => {
            #[pymethods]
            impl $ty {
                $($extra)*

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
        };
    }

    eq_hash_pymethods!(PyFp, {
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
            PyFieldElement(DynFieldElement::Fp(self.0.zero()))
        }

        pub fn one(&self) -> PyFieldElement {
            PyFieldElement(DynFieldElement::Fp(self.0.one()))
        }

        pub fn element(&self, value: u32) -> PyFieldElement {
            PyFieldElement(DynFieldElement::Fp(self.0.element(value)))
        }

        pub fn __repr__(&self) -> String {
            format!("Fp({})", self.characteristic())
        }
    });

    eq_hash_pymethods!(PySmallFq, {
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
            PyFieldElement(DynFieldElement::SmallFq(self.0.a()))
        }

        pub fn q(&self) -> u32 {
            self.0.q()
        }

        pub fn zero(&self) -> PyFieldElement {
            PyFieldElement(DynFieldElement::SmallFq(self.0.zero()))
        }

        pub fn one(&self) -> PyFieldElement {
            PyFieldElement(DynFieldElement::SmallFq(self.0.one()))
        }

        pub fn __repr__(&self) -> String {
            format!("SmallFq({}, {})", self.p(), self.degree())
        }
    });

    eq_hash_pymethods!(PyFieldElement, {
        pub fn inv(&self) -> Option<Self> {
            self.0.inv().map(Self)
        }

        pub fn frobenius(&self) -> Self {
            Self(self.0.frobenius())
        }

        pub fn field<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
            match self.0 {
                DynFieldElement::Fp(x) => {
                    Py::new(py, PyFp(x.field())).map(|x| x.into_bound(py).into_any())
                }
                DynFieldElement::SmallFq(x) => {
                    Py::new(py, PySmallFq(x.field())).map(|x| x.into_bound(py).into_any())
                }
            }
        }

        pub fn __add__(&self, rhs: Self) -> PyResult<Self> {
            (self.0 + rhs.0)
                .map(Self)
                .ok_or_else(|| mismatched_field_error(self.0, rhs.0))
        }

        pub fn __sub__(&self, rhs: Self) -> PyResult<Self> {
            (self.0 - rhs.0)
                .map(Self)
                .ok_or_else(|| mismatched_field_error(self.0, rhs.0))
        }

        pub fn __mul__(&self, rhs: Self) -> PyResult<Self> {
            (self.0 * rhs.0)
                .map(Self)
                .ok_or_else(|| mismatched_field_error(self.0, rhs.0))
        }

        pub fn __truediv__(&self, rhs: Self) -> PyResult<Self> {
            self.0.try_div(rhs.0).map(Self).map_err(|e| match e {
                DivError::MismatchedField => mismatched_field_error(self.0, rhs.0),
                DivError::DivisionByZero => PyZeroDivisionError::new_err("division by zero"),
            })
        }

        pub fn __neg__(&self) -> Self {
            Self(-self.0)
        }

        pub fn __int__(&self) -> PyResult<u32> {
            self.0.try_as_u32().ok_or_else(|| {
                PyValueError::new_err("SmallFq elements do not have a canonical integer value")
            })
        }

        pub fn __repr__(&self) -> String {
            match self.0 {
                DynFieldElement::Fp(x) => {
                    format!("FieldElement(Fp({}), {x})", x.field().characteristic())
                }
                DynFieldElement::SmallFq(x) => {
                    let f = x.field();
                    format!(
                        "FieldElement(SmallFq({}, {}), {x})",
                        f.characteristic(),
                        f.degree()
                    )
                }
            }
        }
    });

    #[pymodule_init]
    fn init(m: &Bound<'_, PyModule>) -> PyResult<()> {
        // The `AugmentedMatrix2`/`AugmentedMatrix3` classes are produced by a
        // macro, which `#[pymodule]` cannot auto-collect.
        m.add_class::<PyAugmentedMatrix2>()?;
        m.add_class::<PyAugmentedMatrix3>()?;
        Ok(())
    }
}
