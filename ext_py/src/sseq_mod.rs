use pyo3::prelude::*;

#[pymodule]
#[pyo3(name = "sseq")]
pub mod sseq_py {
    use std::{
        collections::hash_map::DefaultHasher,
        hash::{Hash, Hasher},
        sync::Mutex,
    };

    use ::fp::prime::{self};
    use pyo3::{
        basic::CompareOp,
        exceptions::{PyIndexError, PyValueError},
    };

    use super::*;
    use crate::fp_py::PyFpVector;

    type RsBidegree = ::sseq::coordinates::Bidegree;
    type RsBidegreeElement = ::sseq::coordinates::BidegreeElement;
    type RsBidegreeGenerator = ::sseq::coordinates::BidegreeGenerator;

    /// Upper bound on accepted primes, mirroring `fp_py::valid_prime`.
    const MAX_VALID_PRIME: u32 = 1 << 31;

    /// Convert a plain `int` prime from Python into a `ValidPrime`, raising
    /// `ValueError` (never panicking) for a non-prime. `ValidPrime` itself is
    /// never exposed to Python. Mirrors the `valid_prime` helper in `fp_mod`.
    fn valid_prime(p: u32) -> PyResult<prime::ValidPrime> {
        if p < 2 || p >= MAX_VALID_PRIME {
            return Err(PyValueError::new_err(format!("{p} is not prime")));
        }
        prime::ValidPrime::try_from(p)
            .map_err(|_| PyValueError::new_err(format!("{p} is not prime")))
    }

    /// Hash via the upstream `Hash` impl, folding `-1` to `-2` (CPython reserves
    /// `-1` for "hash failed"). Mirrors `fp_mod::py_hash`.
    fn py_hash<T: Hash>(value: &T) -> isize {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        match hasher.finish() as isize {
            -1 => -2,
            hash => hash,
        }
    }

    /// A bidegree `(n, s)` (equivalently `(s, t)` with `t = n + s`), the index
    /// type of a (bi)graded object. This is the `N = 2` case of the upstream
    /// `MultiDegree`.
    ///
    /// Coordinate conventions (from upstream `MultiDegree`):
    ///  - `n` = stem = first coordinate; `s` = filtration = second coordinate.
    ///  - `t` = internal degree = `n + s`.
    ///  - `x = n`, `y = s` (chart coordinates).
    #[pyclass(from_py_object)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Bidegree(pub RsBidegree);

    impl From<Bidegree> for RsBidegree {
        fn from(value: Bidegree) -> Self {
            value.0
        }
    }

    impl From<RsBidegree> for Bidegree {
        fn from(value: RsBidegree) -> Self {
            Bidegree(value)
        }
    }

    #[pymethods]
    impl Bidegree {
        /// Construct from `(s, t)`: filtration `s` and internal degree `t`
        /// (`n = t - s`).
        #[staticmethod]
        pub fn s_t(s: i32, t: i32) -> Self {
            Bidegree(RsBidegree::s_t(s, t))
        }

        /// Construct from `(n, s)`: stem `n` and filtration `s`.
        #[staticmethod]
        pub fn n_s(n: i32, s: i32) -> Self {
            Bidegree(RsBidegree::n_s(n, s))
        }

        /// Construct from chart coordinates `(x, y) = (n, s)`.
        #[staticmethod]
        pub fn x_y(x: i32, y: i32) -> Self {
            Bidegree(RsBidegree::x_y(x, y))
        }

        #[getter]
        pub fn n(&self) -> i32 {
            self.0.n()
        }

        #[getter]
        pub fn s(&self) -> i32 {
            self.0.s()
        }

        #[getter]
        pub fn t(&self) -> i32 {
            self.0.t()
        }

        #[getter]
        pub fn x(&self) -> i32 {
            self.0.x()
        }

        #[getter]
        pub fn y(&self) -> i32 {
            self.0.y()
        }

        /// The raw coordinate pair `(n, s)`.
        #[getter]
        pub fn coords(&self) -> (i32, i32) {
            let [n, s] = self.0.coords();
            (n, s)
        }

        pub fn __add__(&self, other: &Bidegree) -> Bidegree {
            Bidegree(self.0 + other.0)
        }

        pub fn __sub__(&self, other: &Bidegree) -> Bidegree {
            Bidegree(self.0 - other.0)
        }

        pub fn __str__(&self) -> String {
            format!("{}", self.0)
        }

        pub fn __repr__(&self) -> String {
            format!("Bidegree.n_s({}, {})", self.0.n(), self.0.s())
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

    /// An element of a (bi)graded vector space: a `Bidegree` together with a
    /// coordinate vector in the basis for that bidegree. This is the `N = 2`
    /// case of the upstream `MultiDegreeElement`.
    #[pyclass(from_py_object)]
    #[derive(Debug, Clone)]
    pub struct BidegreeElement(pub RsBidegreeElement);

    #[pymethods]
    impl BidegreeElement {
        /// Construct from a `Bidegree` and a bound `fp_py.FpVector`. The vector
        /// is cloned into the element (upstream stores an owned `FpVector`).
        #[new]
        pub fn new(degree: &Bidegree, vec: PyRef<'_, PyFpVector>) -> Self {
            BidegreeElement(RsBidegreeElement::new(degree.0, vec.as_rust().clone()))
        }

        #[getter]
        pub fn degree(&self) -> Bidegree {
            Bidegree(self.0.degree())
        }

        #[getter]
        pub fn n(&self) -> i32 {
            self.0.n()
        }

        #[getter]
        pub fn s(&self) -> i32 {
            self.0.s()
        }

        #[getter]
        pub fn t(&self) -> i32 {
            self.0.t()
        }

        #[getter]
        pub fn x(&self) -> i32 {
            self.0.x()
        }

        #[getter]
        pub fn y(&self) -> i32 {
            self.0.y()
        }

        /// A copy of the representing vector as an owned `fp_py.FpVector`.
        pub fn vec(&self) -> PyFpVector {
            PyFpVector::from_rust(self.0.clone().into_vec())
        }

        /// The representing vector as an owned `fp_py.FpVector`. Equivalent to
        /// `vec` here (the binding holds `&self`, so it clones rather than
        /// moving the inner vector).
        pub fn into_vec(&self) -> PyFpVector {
            PyFpVector::from_rust(self.0.clone().into_vec())
        }

        /// String representation as a linear combination of basis generators,
        /// e.g. `2 x_(n, s, 1) + x_(n, s, 2)`.
        pub fn to_basis_string(&self) -> String {
            self.0.to_basis_string()
        }

        pub fn __str__(&self) -> String {
            format!("{}", self.0)
        }

        pub fn __repr__(&self) -> String {
            format!("BidegreeElement({})", self.0)
        }
    }

    /// A *basis* element of a (bi)graded vector space: a `Bidegree` together
    /// with an index into the canonical basis for that bidegree. This is the
    /// `N = 2` case of the upstream `MultiDegreeGenerator`.
    #[pyclass(from_py_object)]
    #[derive(Debug, Clone, Copy)]
    pub struct BidegreeGenerator(pub RsBidegreeGenerator);

    #[pymethods]
    impl BidegreeGenerator {
        /// Construct from a `Bidegree` and a basis index `idx`.
        #[new]
        pub fn new(degree: &Bidegree, idx: usize) -> Self {
            BidegreeGenerator(RsBidegreeGenerator::new(degree.0, idx))
        }

        #[staticmethod]
        pub fn s_t(s: i32, t: i32, idx: usize) -> Self {
            BidegreeGenerator(RsBidegreeGenerator::s_t(s, t, idx))
        }

        #[staticmethod]
        pub fn n_s(n: i32, s: i32, idx: usize) -> Self {
            BidegreeGenerator(RsBidegreeGenerator::n_s(n, s, idx))
        }

        #[getter]
        pub fn degree(&self) -> Bidegree {
            Bidegree(self.0.degree())
        }

        #[getter]
        pub fn idx(&self) -> usize {
            self.0.idx()
        }

        #[getter]
        pub fn n(&self) -> i32 {
            self.0.n()
        }

        #[getter]
        pub fn s(&self) -> i32 {
            self.0.s()
        }

        #[getter]
        pub fn t(&self) -> i32 {
            self.0.t()
        }

        #[getter]
        pub fn x(&self) -> i32 {
            self.0.x()
        }

        #[getter]
        pub fn y(&self) -> i32 {
            self.0.y()
        }

        /// Build the `BidegreeElement` that is this basis vector in an ambient
        /// space of dimension `ambient` over `F_p`. Raises `ValueError` for a
        /// non-prime `p` and `IndexError` if `idx >= ambient` (upstream would
        /// otherwise panic in `set_entry`).
        pub fn into_element(&self, p: u32, ambient: usize) -> PyResult<BidegreeElement> {
            let p = valid_prime(p)?;
            let idx = self.0.idx();
            if idx >= ambient {
                return Err(PyIndexError::new_err(format!(
                    "basis index {idx} out of range for ambient dimension {ambient}"
                )));
            }
            Ok(BidegreeElement(self.0.into_element(p, ambient)))
        }

        pub fn __str__(&self) -> String {
            format!("{}", self.0)
        }

        pub fn __repr__(&self) -> String {
            format!("BidegreeGenerator({})", self.0)
        }
    }

    /// A range of bidegrees: all `s` up to a maximum, and for each such `s` a
    /// maximum `t` given by a Python callable `t(s)`. This is the argument
    /// carrier consumed by `iter_s_t`; it mirrors the upstream
    /// `BidegreeRange`, whose maximal-`t` function is here a Python callback.
    #[pyclass]
    pub struct BidegreeRange {
        s: i32,
        t: Py<PyAny>,
    }

    #[pymethods]
    impl BidegreeRange {
        /// `s` is the (exclusive) maximal filtration; `t` is a callable mapping
        /// a filtration `s` to its (exclusive) maximal internal degree.
        #[new]
        pub fn new(s: i32, t: Py<PyAny>) -> Self {
            BidegreeRange { s, t }
        }

        #[getter]
        pub fn s(&self) -> i32 {
            self.s
        }

        /// Evaluate the maximal-`t` callback at filtration `s`.
        pub fn t(&self, py: Python<'_>, s: i32) -> PyResult<i32> {
            self.t.call1(py, (s,))?.extract(py)
        }

        /// Restrict to a smaller maximal filtration `s` (`s <= self.s`),
        /// reusing the same `t` callback. Raises `ValueError` otherwise
        /// (upstream asserts `s <= self.s`).
        pub fn restrict(&self, py: Python<'_>, s: i32) -> PyResult<Self> {
            if s > self.s {
                return Err(PyValueError::new_err(format!(
                    "cannot restrict range with max s = {} to a larger s = {s}",
                    self.s
                )));
            }
            Ok(BidegreeRange {
                s,
                t: self.t.clone_ref(py),
            })
        }
    }

    /// Read the value the `iter_s_t` callback returned for a given bidegree as a
    /// half-open range of internal degrees. Accepts `None` (interpreted as the
    /// empty range `t..t`) or a 2-tuple `(start, end)`.
    fn extract_callback_range(obj: &Bound<'_, PyAny>, t: i32) -> PyResult<std::ops::Range<i32>> {
        if obj.is_none() {
            return Ok(t..t);
        }
        if let Ok((start, end)) = obj.extract::<(i32, i32)>() {
            return Ok(start..end);
        }
        Err(PyValueError::new_err(
            "iter_s_t callback must return None or a 2-tuple (start, end) of ints",
        ))
    }

    /// Visit a range of bidegrees, calling `callback(bidegree)` for each.
    ///
    /// `min` is the minimal bidegree (inclusive); `max` is a `BidegreeRange`
    /// giving the exclusive maximal `s` and, per `s`, the exclusive maximal `t`.
    /// `callback` is invoked once per visited bidegree and should return the
    /// half-open range of internal degrees that have now been computed (as a
    /// `(start, end)` tuple, starting at `bidegree.t`), or `None` for the empty
    /// range. Exceptions raised by `callback` (or by the range's `t` callback)
    /// are propagated.
    ///
    /// Raises `ValueError` if `min.s >= max.s` (an empty / inverted range).
    #[pyfunction]
    pub fn iter_s_t(
        py: Python<'_>,
        callback: Py<PyAny>,
        min: &Bidegree,
        max: PyRef<'_, BidegreeRange>,
    ) -> PyResult<()> {
        let min_b = min.0;
        let max_s = max.s;
        if min_b.s() >= max_s {
            return Err(PyValueError::new_err(format!(
                "empty bidegree range: require min.s ({}) < max.s ({max_s})",
                min_b.s()
            )));
        }

        // The first error raised by either the user callback or the `t`
        // callback. The upstream `iter_s_t` requires the closures (and any data
        // they capture) to be `Sync`, so the shared error slot is a `Mutex`.
        // `PyErr` is `Send`, so this is `Sync`. Captured by reference from the
        // closures below; read back after the iteration to re-raise.
        let err: Mutex<Option<PyErr>> = Mutex::new(None);

        // Auxiliary data the range's `t` closure depends on: the Python `t`
        // callback. `Py<PyAny>` is `Send + Sync` (and `Ungil`), so it can be
        // moved across the GIL-release boundary below.
        let t_cb = max.t.clone_ref(py);

        // Release the GIL around the (synchronous, and potentially
        // parallel-in-future if `sseq/concurrent` is ever enabled) upstream
        // iteration so other Python threads can run. Each callback invocation
        // briefly re-acquires the GIL via `Python::attach`. This avoids a
        // deadlock that would otherwise occur if the upstream iteration spawned
        // worker threads while this thread held the GIL.
        //
        // Everything captured here is `Ungil`: `&Mutex<Option<PyErr>>`,
        // `Py<PyAny>` callbacks (`&callback`, `&t_cb`), and the `Copy`
        // bidegrees. No GIL-bound borrow (`Bound`/`PyRef`/`Python`) crosses the
        // boundary, so the closure satisfies `detach`'s `Ungil` bound.
        py.detach(|| {
            let record_err = |slot: &Mutex<Option<PyErr>>, e: PyErr| {
                let mut guard = slot.lock().unwrap();
                if guard.is_none() {
                    *guard = Some(e);
                }
            };

            let t_closure = |aux: &Py<PyAny>, s: i32| -> i32 {
                Python::attach(
                    |py| match aux.call1(py, (s,)).and_then(|r| r.extract::<i32>(py)) {
                        Ok(v) => v,
                        Err(e) => {
                            record_err(&err, e);
                            s
                        }
                    },
                )
            };

            let range = ::sseq::coordinates::BidegreeRange::new(&t_cb, max_s, &t_closure);

            let f = |b: RsBidegree| -> std::ops::Range<i32> {
                // Short-circuit cheaply once an error has been recorded.
                if err.lock().unwrap().is_some() {
                    return b.t()..b.t();
                }
                Python::attach(|py| {
                    let arg = Bidegree(b);
                    match callback.call1(py, (arg,)) {
                        Ok(ret) => match extract_callback_range(ret.bind(py), b.t()) {
                            Ok(rng) => rng,
                            Err(e) => {
                                record_err(&err, e);
                                b.t()..b.t()
                            }
                        },
                        Err(e) => {
                            record_err(&err, e);
                            b.t()..b.t()
                        }
                    }
                })
            };

            ::sseq::coordinates::iter_s_t(&f, min_b, range);
        });

        match err.into_inner().unwrap() {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }
}
