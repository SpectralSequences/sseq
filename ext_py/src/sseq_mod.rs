use pyo3::prelude::*;

#[pymodule]
#[pyo3(name = "sseq")]
pub mod sseq_py {
    use std::{
        cell::RefCell,
        collections::hash_map::DefaultHasher,
        hash::{Hash, Hasher},
        io::{self, Write},
        rc::Rc,
        sync::Mutex,
    };

    use ::fp::{
        matrix::Matrix as RsMatrix,
        prime::{self, Prime},
        vector::FpVector as RsFpVector,
    };
    use ::once::MultiIndexed;
    use ::sseq::{
        charting::{
            Backend as RsBackend, Orientation as RsOrientation, SvgBackend as RsSvgBackend,
            TikzBackend as RsTikzBackend,
        },
        SseqProfile as RsSseqProfile,
    };
    use pyo3::{
        basic::CompareOp,
        exceptions::{
            PyAttributeError, PyIOError, PyIndexError, PyRuntimeError, PyTypeError, PyValueError,
        },
        types::PyBytes,
    };

    use super::*;
    use crate::fp_py::{
        with_input_slice, with_target_slice_mut, PyFpVector, PyMatrix, PySubquotient, PySubspace,
    };

    type RsBidegree = ::sseq::coordinates::Bidegree;
    type RsBidegreeElement = ::sseq::coordinates::BidegreeElement;
    type RsBidegreeGenerator = ::sseq::coordinates::BidegreeGenerator;
    type RsSseq = ::sseq::Sseq<2, ::sseq::Adams>;
    type RsProduct = ::sseq::Product<2>;
    type RsDifferential = ::sseq::Differential;

    /// The minimal page number for the (cohomological Adams) spectral sequence,
    /// i.e. `Adams::MIN_R`. Differentials and page data are indexed by pages
    /// `>= MIN_R`; binding methods pre-check `r >= MIN_R` to avoid the
    /// below-`min_degree` indexing panics in the upstream `BiVec`s.
    const MIN_R: i32 = <::sseq::Adams as RsSseqProfile<2>>::MIN_R;

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

        /// Support Python format specs, mirroring the upstream `Display` impl's
        /// alternate flag: `format(g, "")` / `f"{g}"` use the spaced
        /// `(n, s, idx)` form, while `format(g, "#")` / `f"{g:#}"` use the
        /// compact `(n,s,idx)` form (Rust's `{:#}`). Any other spec is a
        /// `ValueError`, matching Python's default rejection of unknown specs.
        pub fn __format__(&self, spec: &str) -> PyResult<String> {
            match spec {
                "" => Ok(format!("{}", self.0)),
                "#" => Ok(format!("{:#}", self.0)),
                _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "unsupported format spec {spec:?} for BidegreeGenerator; \
                     only '' and '#' are supported"
                ))),
            }
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

    /// Validate that two primes agree, raising `ValueError` otherwise.
    fn check_same_prime(expected: u32, got: u32) -> PyResult<()> {
        if expected != got {
            return Err(PyValueError::new_err(format!(
                "prime mismatch: expected {expected}, got {got}"
            )));
        }
        Ok(())
    }

    /// Validate that two lengths/dimensions agree, raising `ValueError`
    /// otherwise.
    fn check_equal_len(expected: usize, got: usize) -> PyResult<()> {
        if expected != got {
            return Err(PyValueError::new_err(format!(
                "dimension mismatch: expected {expected}, got {got}"
            )));
        }
        Ok(())
    }

    /// The profile of the (cohomological) Adams spectral sequence: the only
    /// `SseqProfile<2>` implementation upstream, and the default profile used by
    /// `Sseq`. The differentials go `(n, s) -> (n - 1, s + r)`.
    ///
    /// This is the concrete implementation of the upstream `SseqProfile` trait
    /// for the bigraded (`N = 2`) case. The trait itself is not separately
    /// bound: it has no runtime representation, and `Adams` is its sole `N = 2`
    /// implementor. `Sseq` is monomorphized to `Sseq<2, Adams>`, so `Adams` is
    /// always the active profile; this class exposes the profile's
    /// page/bidegree arithmetic for inspection.
    #[pyclass(frozen)]
    #[derive(Clone, Copy)]
    pub struct Adams;

    #[pymethods]
    impl Adams {
        #[new]
        pub fn new() -> Self {
            Adams
        }

        /// The minimal page number, `2`.
        #[classattr]
        #[allow(non_snake_case)]
        pub fn MIN_R() -> i32 {
            MIN_R
        }

        /// The target bidegree of a `d_r` differential out of `b`.
        #[staticmethod]
        pub fn profile(r: i32, b: &Bidegree) -> Bidegree {
            Bidegree(<::sseq::Adams as RsSseqProfile<2>>::profile(r, b.0))
        }

        /// The source bidegree of a `d_r` differential hitting `b` (inverse of
        /// `profile`).
        #[staticmethod]
        pub fn profile_inverse(r: i32, b: &Bidegree) -> Bidegree {
            Bidegree(<::sseq::Adams as RsSseqProfile<2>>::profile_inverse(r, b.0))
        }

        /// The page `r` of a differential with the given bidegree `offset`
        /// between source and target.
        #[staticmethod]
        pub fn differential_length(offset: &Bidegree) -> i32 {
            <::sseq::Adams as RsSseqProfile<2>>::differential_length(offset.0)
        }

        pub fn __repr__(&self) -> &'static str {
            "Adams"
        }
    }

    /// The interface implemented by spectral-sequence profiles. Upstream this is
    /// a trait (`SseqProfile<N>`) with no runtime data; `Adams` is its only
    /// `N = 2` implementation and the default profile for `Sseq`. This marker
    /// pyclass exists so the name `SseqProfile` is available from Python and to
    /// hand back the default profile via `default()`.
    #[pyclass(frozen)]
    #[derive(Clone, Copy)]
    pub struct SseqProfile;

    #[pymethods]
    impl SseqProfile {
        /// The default (and only) profile for the bigraded spectral sequence:
        /// `Adams`.
        #[staticmethod]
        pub fn default() -> Adams {
            Adams
        }

        pub fn __repr__(&self) -> &'static str {
            "SseqProfile"
        }
    }

    /// A product structure on the spectral sequence: multiplication by a fixed
    /// class living in bidegree `b`. For each source bidegree it stores the
    /// matrix of the multiplication map (as a `MultiIndexed<2, Matrix>`).
    ///
    /// `left` records whether the product acts on the left, which affects the
    /// sign in the Leibniz rule. This is the `N = 2` case of the upstream
    /// `Product`.
    #[pyclass(name = "Product")]
    pub struct Product(pub RsProduct);

    #[pymethods]
    impl Product {
        /// Construct a product in bidegree `b`. `matrices` is a list of
        /// `(source_bidegree, Matrix)` pairs giving the multiplication map out
        /// of each source bidegree; the matrix maps the source basis (its rows)
        /// to the target basis (its columns). `left` records the handedness for
        /// the Leibniz sign.
        ///
        /// Raises `ValueError` if two matrices are given for the same bidegree.
        #[new]
        #[pyo3(signature = (b, left, matrices))]
        pub fn new(
            b: &Bidegree,
            left: bool,
            matrices: Vec<(Bidegree, PyRef<'_, PyMatrix>)>,
        ) -> PyResult<Self> {
            let indexed: MultiIndexed<2, RsMatrix> = MultiIndexed::new();
            for (deg, matrix) in matrices {
                indexed
                    .try_insert(deg.0, matrix.as_rust().clone())
                    .map_err(|_| {
                        PyValueError::new_err(format!("duplicate matrix for bidegree {}", deg.0))
                    })?;
            }
            Ok(Product(RsProduct {
                b: b.0,
                left,
                matrices: indexed,
            }))
        }

        /// The bidegree the product lives in (the shift it applies).
        #[getter]
        pub fn b(&self) -> Bidegree {
            Bidegree(self.0.b)
        }

        /// Whether the product acts on the left.
        #[getter]
        pub fn left(&self) -> bool {
            self.0.left
        }

        /// The stored multiplication matrices as a list of
        /// `(source_bidegree, Matrix)` pairs.
        #[getter]
        pub fn matrices(&self) -> Vec<(Bidegree, PyMatrix)> {
            self.0
                .matrices
                .iter()
                .map(|(coords, m)| {
                    (
                        Bidegree(RsBidegree::from(coords)),
                        PyMatrix::from_rust(m.clone()),
                    )
                })
                .collect()
        }

        /// The multiplication matrix out of `source`, or `None` if undefined.
        pub fn get_matrix(&self, source: &Bidegree) -> Option<PyMatrix> {
            self.0
                .matrices
                .get(source.0)
                .map(|m| PyMatrix::from_rust(m.clone()))
        }
    }

    /// A (reduced) differential between two graded vector spaces, stored as the
    /// span of `(source, target)` pairs. This is the building block the
    /// `Sseq` stores per page; it can also be used standalone.
    #[pyclass(name = "Differential")]
    pub struct Differential(pub RsDifferential);

    impl Differential {
        /// Wrap an owned upstream `Differential`.
        pub(crate) fn from_rust(differential: RsDifferential) -> Self {
            Differential(differential)
        }
    }

    #[pymethods]
    impl Differential {
        /// A new zero differential from a `source_dim`-dimensional space to a
        /// `target_dim`-dimensional space over `F_p`. Raises `ValueError` for a
        /// non-prime `p`.
        #[new]
        pub fn new(p: u32, source_dim: usize, target_dim: usize) -> PyResult<Self> {
            Ok(Differential(RsDifferential::new(
                valid_prime(p)?,
                source_dim,
                target_dim,
            )))
        }

        /// The prime of the underlying field.
        pub fn prime(&self) -> u32 {
            self.0.prime().as_u32()
        }

        /// The dimension of the source space.
        #[getter]
        pub fn source_dim(&self) -> usize {
            self.0.source_dim()
        }

        /// The dimension of the target space.
        #[getter]
        pub fn target_dim(&self) -> usize {
            self.0.target_dim()
        }

        /// Add the differential `d(source) = target`. If `target` is `None`,
        /// records that `source` is a cycle (zero differential). Returns whether
        /// a genuinely new differential was added.
        ///
        /// Raises `ValueError` on a prime/length mismatch (`source` must have
        /// length `source_dim`, `target` length `target_dim`) to avoid the
        /// upstream slice-length panic.
        #[pyo3(signature = (source, target=None))]
        pub fn add(
            &mut self,
            py: Python<'_>,
            source: &Bound<'_, PyAny>,
            target: Option<&Bound<'_, PyAny>>,
        ) -> PyResult<bool> {
            let p = self.0.prime().as_u32();
            let source_dim = self.0.source_dim();
            let target_dim = self.0.target_dim();
            with_input_slice(py, source, |src| {
                check_same_prime(p, src.prime().as_u32())?;
                check_equal_len(source_dim, src.len())?;
                match target {
                    None => Ok(self.0.add(src, None)),
                    Some(t) => with_input_slice(py, t, |tgt| {
                        check_same_prime(p, tgt.prime().as_u32())?;
                        check_equal_len(target_dim, tgt.len())?;
                        Ok(self.0.add(src, Some(tgt)))
                    }),
                }
            })
        }

        /// Reset to the zero differential.
        pub fn set_to_zero(&mut self) {
            self.0.set_to_zero();
        }

        /// Whether the recorded differentials are inconsistent. Only meaningful
        /// after the targets have been reduced (which `Sseq.update` does).
        pub fn inconsistent(&self) -> bool {
            self.0.inconsistent()
        }

        /// The recorded `(source, target)` pairs, as a list of `FpVector`
        /// pairs.
        pub fn get_source_target_pairs(&self) -> Vec<(PyFpVector, PyFpVector)> {
            self.0
                .get_source_target_pairs()
                .into_iter()
                .map(|(s, t)| (PyFpVector::from_rust(s), PyFpVector::from_rust(t)))
                .collect()
        }

        /// Evaluate the differential on `source`, adding the result into the
        /// mutable `target`. Assumes every non-pivot column has zero
        /// differential.
        ///
        /// Raises `ValueError` on a prime/length mismatch (`source` length must
        /// be `source_dim`, `target` length `target_dim`).
        pub fn evaluate(
            &self,
            py: Python<'_>,
            source: &Bound<'_, PyAny>,
            target: &Bound<'_, PyAny>,
        ) -> PyResult<()> {
            let p = self.0.prime().as_u32();
            let source_dim = self.0.source_dim();
            let target_dim = self.0.target_dim();
            with_input_slice(py, source, |src| {
                check_same_prime(p, src.prime().as_u32())?;
                check_equal_len(source_dim, src.len())?;
                with_target_slice_mut(py, target, |tgt| {
                    check_same_prime(p, tgt.as_slice().prime().as_u32())?;
                    check_equal_len(target_dim, tgt.as_slice().len())?;
                    self.0.evaluate(src, tgt);
                    Ok(())
                })
            })
        }

        /// Find a preimage of `value` under the differential, i.e. apply the
        /// quasi-inverse. Returns a new `FpVector` of length `source_dim`.
        ///
        /// Note: upstream `Differential::quasi_inverse` writes the preimage into
        /// a caller-supplied slice rather than returning a `QuasiInverse`
        /// object (which is what the API proposal anticipated); we follow
        /// upstream and return the computed preimage vector.
        ///
        /// Raises `ValueError` on a prime/length mismatch (`value` length must
        /// be `target_dim`).
        pub fn quasi_inverse(
            &self,
            py: Python<'_>,
            value: &Bound<'_, PyAny>,
        ) -> PyResult<PyFpVector> {
            let p = self.0.prime();
            let source_dim = self.0.source_dim();
            let target_dim = self.0.target_dim();
            with_input_slice(py, value, |val| {
                check_same_prime(p.as_u32(), val.prime().as_u32())?;
                check_equal_len(target_dim, val.len())?;
                let mut result = RsFpVector::new(p, source_dim);
                self.0.quasi_inverse(result.as_slice_mut(), val);
                Ok(PyFpVector::from_rust(result))
            })
        }
    }

    /// A bigraded spectral sequence with the Adams profile (`Sseq<2, Adams>`),
    /// the only spectral sequence used by the examples.
    ///
    /// # Storage
    ///
    /// Held as a plain owned value in a `#[pyclass]`. Every upstream mutator
    /// (`set_dimension`, `add_differential`, `add_permanent_class`, `update`,
    /// ...) takes `&mut self`, and PyO3 hands out the `&mut` via its runtime
    /// borrow check, so no `Arc`/interior-mutability wrapper is needed. (No
    /// `Sseq` pyclass existed previously: `SecondaryResolution.e3_page`, which
    /// would return one, is not yet bound, so there was nothing to reconcile.)
    #[pyclass(name = "Sseq", unsendable)]
    pub struct Sseq(RsSseq, prime::ValidPrime);

    impl Sseq {
        /// Wrap an upstream `Sseq<2, Adams>` (e.g. the page produced by a chain
        /// complex's `to_sseq`). The prime must match the one the `Sseq` was
        /// built over; it is recorded separately because `Sseq` exposes no
        /// public prime accessor.
        pub(crate) fn from_rust(sseq: RsSseq, p: prime::ValidPrime) -> Self {
            Sseq(sseq, p)
        }

        /// Guard that bidegree `b` has been defined, returning `IndexError`
        /// otherwise (the upstream `data[b]` indexing would panic).
        fn require_defined(&self, b: &Bidegree) -> PyResult<()> {
            if self.0.defined(b.0) {
                Ok(())
            } else {
                Err(PyIndexError::new_err(format!(
                    "bidegree {} is not defined",
                    b.0
                )))
            }
        }

        /// Guard that every intermediate (and final) target bidegree a
        /// `d_r` differential out of `source_b` touches is defined.
        ///
        /// Upstream `add_differential(r, source, _)` calls
        /// `extend_differential(r, source)` and `extend_page_data`, which index
        /// `self.dimension(profile(r', source))` / `self.data[profile(r',
        /// source)]` for *every* page `r'` in `MIN_R..=r` — not just the final
        /// `r`. Any undefined such bidegree panics in `MultiIndexed`'s `Index`
        /// impl, so we pre-check them all and raise `IndexError` naming the
        /// first undefined one. (The `profile_inverse` degrees upstream touches
        /// after recording the differential are already `defined()`-guarded
        /// upstream, so they need no check here.)
        fn require_intermediate_targets_defined(&self, source_b: Bidegree, r: i32) -> PyResult<()> {
            for r_prime in MIN_R..=r {
                let target = Bidegree(<::sseq::Adams as RsSseqProfile<2>>::profile(
                    r_prime, source_b.0,
                ));
                if !self.0.defined(target.0) {
                    return Err(PyIndexError::new_err(format!(
                        "intermediate target bidegree {} (= profile({r_prime}, {})) \
                         of a d_{r} differential is not defined",
                        target.0, source_b.0
                    )));
                }
            }
            Ok(())
        }

        /// Validate that every stored multiplication matrix in `product` is over
        /// the Sseq's prime, raising `ValueError("product prime mismatch")`
        /// otherwise. Upstream `leibniz`/`multiply` only ever exercise the
        /// matrix at the relevant source bidegree, so a stray wrong-prime matrix
        /// might otherwise either go unnoticed or surface as an opaque
        /// `catch_unwind` panic; checking up front gives a clear error.
        fn require_product_prime(&self, product: &Product) -> PyResult<()> {
            let p = self.1.as_u32();
            for (_coords, matrix) in product.0.matrices.iter() {
                if matrix.prime().as_u32() != p {
                    return Err(PyValueError::new_err(format!(
                        "product prime mismatch: Sseq is over F_{p}, but a product \
                         matrix is over F_{}",
                        matrix.prime().as_u32()
                    )));
                }
            }
            Ok(())
        }
    }

    #[pymethods]
    impl Sseq {
        /// A new, empty spectral sequence over `F_p`. Raises `ValueError` for a
        /// non-prime `p`.
        #[new]
        pub fn new(p: u32) -> PyResult<Self> {
            let p = valid_prime(p)?;
            Ok(Sseq(RsSseq::new(p), p))
        }

        /// The prime of the underlying field.
        pub fn prime(&self) -> u32 {
            self.1.as_u32()
        }

        /// Define bidegree `b` to have dimension `dim` (number of generators).
        /// Raises `ValueError` if `b` is already defined (upstream would panic
        /// on the duplicate insert).
        pub fn set_dimension(&mut self, b: &Bidegree, dim: usize) -> PyResult<()> {
            if self.0.defined(b.0) {
                return Err(PyValueError::new_err(format!(
                    "bidegree {} is already defined",
                    b.0
                )));
            }
            self.0.set_dimension(b.0, dim);
            Ok(())
        }

        /// The dimension at bidegree `b`. Raises `IndexError` if `b` is not
        /// defined; use `get_dimension` for the optional form.
        pub fn dimension(&self, b: &Bidegree) -> PyResult<usize> {
            self.require_defined(b)?;
            Ok(self.0.dimension(b.0))
        }

        /// The dimension at bidegree `b`, or `None` if it is not defined.
        pub fn get_dimension(&self, b: &Bidegree) -> Option<usize> {
            self.0.get_dimension(b.0)
        }

        /// Reset all permanent classes, differentials, and page data, marking
        /// every defined bidegree invalid.
        pub fn clear(&mut self) {
            self.0.clear();
        }

        /// The minimal defined bidegree (componentwise), or `(0, 0)` if empty.
        pub fn min(&self) -> Bidegree {
            Bidegree(self.0.min())
        }

        /// The maximal defined bidegree (componentwise), or `(0, 0)` if empty.
        pub fn max(&self) -> Bidegree {
            Bidegree(self.0.max())
        }

        /// Whether bidegree `b` has been defined.
        pub fn defined(&self, b: &Bidegree) -> bool {
            self.0.defined(b.0)
        }

        /// The list of all defined bidegrees, in sorted order.
        pub fn iter_degrees(&self) -> Vec<Bidegree> {
            self.0.iter_degrees().map(Bidegree).collect()
        }

        /// Record that `elem` (a `BidegreeElement`) is a permanent class.
        /// Returns whether a genuinely new permanent class was added.
        ///
        /// Raises `IndexError` if the element's bidegree is undefined and
        /// `ValueError` on a prime/length mismatch (the element's vector must
        /// have length equal to the bidegree's dimension).
        pub fn add_permanent_class(&mut self, elem: &BidegreeElement) -> PyResult<bool> {
            let b = Bidegree(elem.0.degree());
            self.require_defined(&b)?;
            check_same_prime(self.1.as_u32(), elem.0.vec().prime().as_u32())?;
            check_equal_len(self.0.dimension(b.0), elem.0.vec().len())?;
            Ok(self.0.add_permanent_class(&elem.0))
        }

        /// The subspace of permanent classes at bidegree `b`. Raises
        /// `IndexError` if `b` is not defined.
        pub fn permanent_classes(&self, b: &Bidegree) -> PyResult<PySubspace> {
            self.require_defined(b)?;
            Ok(PySubspace::from_rust(self.0.permanent_classes(b.0).clone()))
        }

        /// Add a `d_r` differential with the given `source` class (a
        /// `BidegreeElement`, which carries both the source bidegree and the
        /// source vector) and `target` vector. Returns whether the differential
        /// is new.
        ///
        /// Note: the API proposal described the source as a bare `Bidegree`, but
        /// upstream needs the source *class* (bidegree + vector), so we take a
        /// `BidegreeElement`.
        ///
        /// Guards (all raising clean exceptions instead of panicking):
        ///  - `r >= MIN_R` (`ValueError`),
        ///  - the source bidegree is defined (`IndexError`),
        ///  - *every* intermediate target bidegree `profile(r', source)` for
        ///    `r'` in `MIN_R..=r` is defined (`IndexError`), naming the first
        ///    undefined one. Upstream `add_differential` calls
        ///    `extend_differential(r, source)`/`extend_page_data`, which index
        ///    `self.dimension(profile(r', source))` and
        ///    `self.data[profile(r', source)]` for every page `r'` in that
        ///    range (not just the final `r`), so each must be defined or the
        ///    upstream `MultiIndexed` index panics. The final target is the
        ///    `r' = r` case.
        ///  - prime and length match for both the source vector
        ///    (`= dim(source)`) and `target` (`= dim(target_bidegree)`)
        ///    (`ValueError`).
        pub fn add_differential(
            &mut self,
            py: Python<'_>,
            r: i32,
            source: &BidegreeElement,
            target: &Bound<'_, PyAny>,
        ) -> PyResult<bool> {
            if r < MIN_R {
                return Err(PyValueError::new_err(format!(
                    "page number r = {r} is below the minimal page {MIN_R}"
                )));
            }
            let source_b = Bidegree(source.0.degree());
            self.require_defined(&source_b)?;
            // Guard every bidegree the upstream `extend_differential` /
            // `extend_page_data` path indexes: `profile(r', source)` for every
            // page `r'` in `MIN_R..=r`. The final iteration (`r' = r`) is the
            // differential's actual target.
            self.require_intermediate_targets_defined(source_b, r)?;
            let target_b = Bidegree(<::sseq::Adams as RsSseqProfile<2>>::profile(r, source_b.0));

            let p = self.1.as_u32();
            check_same_prime(p, source.0.vec().prime().as_u32())?;
            check_equal_len(self.0.dimension(source_b.0), source.0.vec().len())?;
            let target_dim = self.0.dimension(target_b.0);
            with_input_slice(py, target, |tgt| {
                check_same_prime(p, tgt.prime().as_u32())?;
                check_equal_len(target_dim, tgt.len())?;
                Ok(self.0.add_differential(r, &source.0, tgt))
            })
        }

        /// The list of differentials at bidegree `b`, one per page starting at
        /// `MIN_R`. Raises `IndexError` if `b` is not defined.
        pub fn differentials(&self, b: &Bidegree) -> PyResult<Vec<Differential>> {
            self.require_defined(b)?;
            Ok(self
                .0
                .differentials(b.0)
                .iter()
                .map(|d| Differential::from_rust(d.clone()))
                .collect())
        }

        /// The differentials that hit bidegree `b`, as a list of
        /// `(r, Differential)` pairs. Raises `IndexError` if `b` is not
        /// defined.
        pub fn differentials_hitting(&self, b: &Bidegree) -> PyResult<Vec<(i32, Differential)>> {
            self.require_defined(b)?;
            Ok(self
                .0
                .differentials_hitting(b.0)
                .map(|(r, d)| (r, Differential::from_rust(d.clone())))
                .collect())
        }

        /// The `E_r` page data (a `Subquotient`) at bidegree `b`. Raises
        /// `IndexError` if `b` is not defined or `r` is out of the computed page
        /// range.
        pub fn page_data(&self, b: &Bidegree, r: i32) -> PyResult<PySubquotient> {
            self.require_defined(b)?;
            let data = self.0.page_data(b.0);
            match data.get(r) {
                Some(sq) => Ok(PySubquotient::from_rust(sq.clone())),
                None => Err(PyIndexError::new_err(format!(
                    "page {r} is out of range [{}, {}) at bidegree {}",
                    data.min_degree(),
                    data.len(),
                    b.0
                ))),
            }
        }

        /// Whether the page data at bidegree `b` is stale (needs recomputing via
        /// `update`/`update_degree`). Raises `IndexError` if `b` is not defined.
        pub fn invalid(&self, b: &Bidegree) -> PyResult<bool> {
            self.require_defined(b)?;
            Ok(self.0.invalid(b.0))
        }

        /// Recompute every invalid bidegree.
        pub fn update(&mut self) {
            self.0.update();
        }

        /// Recompute bidegree `b` and return, per page (starting at `MIN_R`),
        /// the differentials to draw: a list (indexed by page) of lists (indexed
        /// by source generator) of target coordinate lists. Raises `IndexError`
        /// if `b` is not defined.
        pub fn update_degree(&mut self, b: &Bidegree) -> PyResult<Vec<Vec<Vec<u32>>>> {
            self.require_defined(b)?;
            Ok(self.0.update_degree(b.0).into_iter().collect())
        }

        /// Whether the calculations at bidegree `b` are complete (every class on
        /// the final page is known to be permanent). Raises `IndexError` if `b`
        /// is not defined.
        pub fn complete(&self, b: &Bidegree) -> PyResult<bool> {
            self.require_defined(b)?;
            Ok(self.0.complete(b.0))
        }

        /// Whether there is an inconsistent differential involving bidegree `b`.
        /// Raises `IndexError` if `b` is not defined.
        pub fn inconsistent(&self, b: &Bidegree) -> PyResult<bool> {
            self.require_defined(b)?;
            Ok(self.0.inconsistent(b.0))
        }

        /// Multiply the class `elem` by the product `product`. Returns the
        /// resulting `BidegreeElement`, or `None` if the product is not yet
        /// computed at `elem`'s bidegree (or the target bidegree is undefined).
        ///
        /// Raises `ValueError` on a prime mismatch or if the stored product
        /// matrix is incompatible with the element/target dimensions (which
        /// would otherwise panic inside `Matrix::apply`).
        pub fn multiply(
            &self,
            elem: &BidegreeElement,
            product: &Product,
        ) -> PyResult<Option<BidegreeElement>> {
            let elem_b = elem.0.degree();
            check_same_prime(self.1.as_u32(), elem.0.vec().prime().as_u32())?;
            let Some(matrix) = product.0.matrices.get(elem_b) else {
                return Ok(None);
            };
            let target_b = elem_b + product.0.b;
            let Some(target_dim) = self.0.get_dimension(target_b) else {
                return Ok(None);
            };
            check_same_prime(self.1.as_u32(), matrix.prime().as_u32())?;
            check_equal_len(matrix.rows(), elem.0.vec().len())?;
            check_equal_len(matrix.columns(), target_dim)?;
            Ok(self.0.multiply(&elem.0, &product.0).map(BidegreeElement))
        }

        /// Apply the Leibniz rule to propagate differentials. Starting from a
        /// `d_r` differential on `elem` (use `r = 2**31 - 1` if `elem` is a
        /// permanent class), multiply by `source_product` (with the
        /// differential on the product given by `target_product`, or `None` if
        /// the product is permanent). Returns `(r, class)` recording the new
        /// differential, or `None` if no differential was added (trivial, or the
        /// product data is not yet available).
        ///
        /// Guards checked up front (before any mutation): `elem`'s bidegree
        /// must be defined (`IndexError`), `elem`'s vector prime must match
        /// (`ValueError`), and every stored matrix in `source_product` /
        /// `target_product` must be over the Sseq's prime
        /// (`ValueError("product prime mismatch")`).
        ///
        /// The set of bidegrees the rule ultimately touches (it calls
        /// `multiply` and `add_differential` on a *derived* source/page that
        /// depends on both products and the differential length) is not
        /// cleanly determinable from the binding without replaying upstream's
        /// internal control flow, so any remaining upstream precondition (e.g.
        /// an undefined intermediate target bidegree the rule would index) is
        /// contained with `catch_unwind` and surfaced as a `RuntimeError`
        /// rather than crossing the FFI boundary as a panic.
        ///
        /// # Stale state on a caught error
        ///
        /// `leibniz` mutates the owned `Sseq` in place via the same
        /// `add_differential`/`extend_*` path. If it panics partway through,
        /// `catch_unwind` keeps the process memory-safe, but the `Sseq` may be
        /// left **partially mutated** (extra differentials/page-data rows,
        /// degrees flagged invalid). It remains safe to read, but is logically
        /// stale; if `leibniz` raises a `RuntimeError`, rebuild the `Sseq`
        /// rather than trusting its state. The up-front guards above cover the
        /// common misuse cases without entering this path.
        #[pyo3(signature = (r, elem, source_product, target_product=None))]
        pub fn leibniz(
            &mut self,
            r: i32,
            elem: &BidegreeElement,
            source_product: &Product,
            target_product: Option<&Product>,
        ) -> PyResult<Option<(i32, BidegreeElement)>> {
            self.require_defined(&Bidegree(elem.0.degree()))?;
            check_same_prime(self.1.as_u32(), elem.0.vec().prime().as_u32())?;
            self.require_product_prime(source_product)?;
            if let Some(tp) = target_product {
                self.require_product_prime(tp)?;
            }

            let target = target_product.map(|p| &p.0);
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                self.0.leibniz(r, &elem.0, &source_product.0, target)
            }))
            .map_err(|_| {
                pyo3::exceptions::PyRuntimeError::new_err(
                    "leibniz failed: the required differentials/page data are not available; \
                     the Sseq may now be in a partially mutated (stale) state and should be rebuilt",
                )
            })?;
            Ok(result.map(|(r, e)| (r, BidegreeElement(e))))
        }

        /// Chart this spectral sequence to `backend` (an `SvgBackend` or
        /// `TikzBackend`), drawing the `E_r` page.
        ///
        /// - `r`: the page to draw.
        /// - `differentials`: whether to draw the `d_r` differentials.
        /// - `products`: a list of `(name, Product)` pairs; for each, the
        ///   structure lines it induces are drawn (labelled `name`).
        /// - `header`: a Python callable invoked (with a single `None`
        ///   argument) after the grid is drawn. The upstream header receives
        ///   the live Rust backend, which has no Python representation, so the
        ///   callback cannot draw to the chart; pass a no-op `lambda _: None`.
        ///
        /// Dispatches over the two concrete bound backends (keeping the generic
        /// upstream call monomorphic). The backend is *consumed*: its inner
        /// value is moved into `write_to_graph`, whose `Drop` emits the closing
        /// tag, so the output is complete only after this returns and the
        /// backend can no longer be used for manual drawing.
        ///
        /// Raises `TypeError` if `backend` is not an `SvgBackend`/`TikzBackend`,
        /// `RuntimeError` if it was already consumed or if the upstream call
        /// panics (e.g. the sseq's minimal filtration is not 0), and propagates
        /// any exception raised by the file object's `.write` or by `header`.
        pub fn write_to_graph(
            &self,
            backend: &Bound<'_, PyAny>,
            r: i32,
            differentials: bool,
            products: Vec<(String, PyRef<'_, Product>)>,
            header: Py<PyAny>,
        ) -> PyResult<()> {
            let prods: Vec<(String, RsProduct)> = products
                .iter()
                .map(|(name, p)| (name.clone(), clone_product(&p.0)))
                .collect();

            if let Ok(svg) = backend.cast::<SvgBackend>() {
                let mut b = svg.borrow_mut();
                let err = Rc::clone(&b.err);
                run_write_to_graph(
                    &mut b.inner,
                    &err,
                    &self.0,
                    r,
                    differentials,
                    &prods,
                    header,
                )
            } else if let Ok(tikz) = backend.cast::<TikzBackend>() {
                let mut b = tikz.borrow_mut();
                let err = Rc::clone(&b.err);
                run_write_to_graph(
                    &mut b.inner,
                    &err,
                    &self.0,
                    r,
                    differentials,
                    &prods,
                    header,
                )
            } else {
                Err(PyTypeError::new_err(
                    "backend must be an SvgBackend or TikzBackend",
                ))
            }
        }
    }

    // ===================================================================
    // §6.3 Charting backends
    // ===================================================================

    /// Adapter turning a Python file-like object into a Rust [`io::Write`].
    ///
    /// The upstream `SvgBackend<W>`/`TikzBackend<W>` are generic over
    /// `W: io::Write`; Python file objects are not, so this bridges the two.
    /// Each `write` decodes the (always-UTF-8, produced by upstream `write!`)
    /// bytes and calls the Python object's `.write`, trying a `str` first and
    /// falling back to `bytes` for binary files (`io.BytesIO`). The GIL is
    /// (re)acquired per call via `Python::attach`; this is sound whether or not
    /// the caller already holds the GIL (the binding always calls the backend
    /// with the GIL held).
    ///
    /// # Error propagation (never panics across FFI)
    ///
    /// `io::Write` cannot carry a `PyErr`, so a Python exception raised by
    /// `.write`/`.flush` is *recorded* in the shared `err` slot (first error
    /// wins) and surfaced as a generic `io::Error`. The backend pyclass holds a
    /// clone of the same `Rc<RefCell<Option<PyErr>>>`; after the upstream call
    /// returns its `io::Error`, the binding takes the stored `PyErr` back out
    /// and re-raises it (see `raise_io`). Nothing panics.
    pub struct PyFileWriter {
        file: Py<PyAny>,
        err: Rc<RefCell<Option<PyErr>>>,
    }

    impl PyFileWriter {
        fn record(&self, e: PyErr) {
            let mut slot = self.err.borrow_mut();
            if slot.is_none() {
                *slot = Some(e);
            }
        }
    }

    impl Write for PyFileWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            // Short-circuit once an error has been recorded: keep producing
            // errors so the upstream call unwinds promptly to the binding.
            if self.err.borrow().is_some() {
                return Err(io::Error::other("python file .write previously raised"));
            }
            let s = String::from_utf8_lossy(buf);
            Python::attach(|py| {
                // Text files (StringIO, sys.stdout) take str; binary files
                // (BytesIO) take bytes and raise TypeError on str. Try str,
                // then fall back to bytes on a TypeError.
                let res = match self.file.call_method1(py, "write", (s.as_ref(),)) {
                    Ok(_) => Ok(()),
                    Err(e) if e.is_instance_of::<PyTypeError>(py) => {
                        let bytes = PyBytes::new(py, buf);
                        self.file.call_method1(py, "write", (bytes,)).map(|_| ())
                    }
                    Err(e) => Err(e),
                };
                match res {
                    Ok(()) => Ok(buf.len()),
                    Err(e) => {
                        self.record(e);
                        Err(io::Error::other("python file .write raised"))
                    }
                }
            })
        }

        fn flush(&mut self) -> io::Result<()> {
            Python::attach(|py| match self.file.call_method0(py, "flush") {
                Ok(_) => Ok(()),
                // A missing `.flush` is fine (not every file-like has one).
                Err(e) if e.is_instance_of::<PyAttributeError>(py) => Ok(()),
                Err(e) => {
                    self.record(e);
                    Err(io::Error::other("python file .flush raised"))
                }
            })
        }
    }

    /// Convert an upstream `io::Result` back into a `PyResult`, re-raising any
    /// `PyErr` recorded by the [`PyFileWriter`] (or a generic `IOError` if the
    /// `io::Error` did not originate from a Python exception).
    ///
    /// The recorded `PyErr` must be re-raised **even when `res` is `Ok`**. The
    /// closing `</svg>` / `\end{tikzpicture}` tag is emitted by the backend's
    /// `Drop`, which runs when the moved-in backend value is dropped *inside*
    /// the upstream `write_to_graph` call (the backend is taken by value), i.e.
    /// before that call returns `res`. `Drop` cannot propagate its `.write`
    /// error, so it only records it in the shared slot and leaves `res` as
    /// `Ok(())`. If we inspected the slot only on `Err`, a failure that occurs
    /// *only* on the closing-tag write would be silently swallowed and
    /// `write_to_graph` would report success with truncated output. Because the
    /// Drop has already run by the time `raise_io` reads the slot, checking the
    /// slot on the `Ok` path correctly surfaces that error.
    fn raise_io(err: &Rc<RefCell<Option<PyErr>>>, res: io::Result<()>) -> PyResult<()> {
        match res {
            // Even on a successful upstream result, a closing-tag `.write`
            // error recorded by the dropped backend must be re-raised.
            Ok(()) => match err.borrow_mut().take() {
                Some(e) => Err(e),
                None => Ok(()),
            },
            Err(e) => Err(err
                .borrow_mut()
                .take()
                .unwrap_or_else(|| PyIOError::new_err(e.to_string()))),
        }
    }

    /// Chart label placement relative to a bidegree. Mirrors the upstream
    /// `charting::Orientation`. Note: `SvgBackend` only implements `Left` and
    /// `Below` (used for axis labels); `Right`/`Above` raise (see
    /// `SvgBackend.text`).
    #[pyclass(eq, eq_int, name = "Orientation", from_py_object)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Orientation {
        Left,
        Right,
        Above,
        Below,
    }

    impl From<Orientation> for RsOrientation {
        fn from(value: Orientation) -> Self {
            match value {
                Orientation::Left => RsOrientation::Left,
                Orientation::Right => RsOrientation::Right,
                Orientation::Above => RsOrientation::Above,
                Orientation::Below => RsOrientation::Below,
            }
        }
    }

    /// Build a fresh `PyFileWriter` and its shared error slot from a Python
    /// file-like object.
    fn new_writer(file: Py<PyAny>) -> (PyFileWriter, Rc<RefCell<Option<PyErr>>>) {
        let err = Rc::new(RefCell::new(None));
        (
            PyFileWriter {
                file,
                err: Rc::clone(&err),
            },
            err,
        )
    }

    /// Generic message for a charting backend method that panicked upstream
    /// (e.g. an unsupported `SvgBackend` orientation, or a `node`/`structline`
    /// referencing a bidegree for which `node()` was never called, or more
    /// classes than the node patterns support). Contained with `catch_unwind`
    /// so it never crosses the FFI boundary as a panic.
    fn panic_msg() -> PyErr {
        PyRuntimeError::new_err(
            "charting backend method panicked: likely an unsupported orientation \
             (SvgBackend supports only Left/Below), a node()/structline() at a \
             bidegree where node() was not called, or too many classes for the \
             node patterns",
        )
    }

    /// Clone an upstream `Product` (which is not `Clone`) by rebuilding its
    /// `MultiIndexed` matrix store. Used to copy the products passed to
    /// `write_to_graph` into an owned `Vec` the upstream iterator can borrow.
    fn clone_product(p: &RsProduct) -> RsProduct {
        let matrices: MultiIndexed<2, RsMatrix> = MultiIndexed::new();
        for (coords, m) in p.matrices.iter() {
            let _ = matrices.try_insert(RsBidegree::from(coords), m.clone());
        }
        RsProduct {
            b: p.b,
            left: p.left,
            matrices,
        }
    }

    /// Drive `Sseq::write_to_graph` over a concrete bound backend.
    ///
    /// Takes (consumes) the backend's inner upstream value: `write_to_graph`
    /// owns its `T: Backend` and its `Drop` writes the closing
    /// `</svg>`/`\end{tikzpicture}`, so the chart is only complete once the
    /// backend is dropped at the end of the call. Subsequent manual method
    /// calls on the same pyclass therefore raise "already consumed".
    ///
    /// `header` is a Python callable invoked (after the grid is drawn) with a
    /// single `None` argument: the upstream `header` receives the live Rust
    /// `&mut T`, which has no Python representation, so the callback cannot
    /// write to the chart. All examples pass a no-op `lambda _: None`; richer
    /// header drawing is not supported (documented limitation).
    fn run_write_to_graph<T>(
        inner: &mut Option<T>,
        err: &Rc<RefCell<Option<PyErr>>>,
        sseq: &RsSseq,
        r: i32,
        differentials: bool,
        products: &[(String, RsProduct)],
        header: Py<PyAny>,
    ) -> PyResult<()>
    where
        T: RsBackend<Error = io::Error>,
    {
        let g = inner.take().ok_or_else(|| {
            PyRuntimeError::new_err("backend was already consumed by a previous write_to_graph")
        })?;

        let header_err = Rc::clone(err);
        let header_closure = move |_g: &mut T| -> io::Result<()> {
            Python::attach(|py| match header.call1(py, (py.None(),)) {
                Ok(_) => Ok(()),
                Err(e) => {
                    let mut slot = header_err.borrow_mut();
                    if slot.is_none() {
                        *slot = Some(e);
                    }
                    Err(io::Error::other("header callback raised"))
                }
            })
        };

        // `try_write_to_graph` checks the "minimum y-coordinate == 0" precondition
        // (the sseq's minimal filtration) up front and returns `Err(String)`
        // instead of panicking; map that to a clear error. The remaining,
        // genuinely-unguardable panics (e.g. too many classes for the node
        // patterns) are still contained with `catch_unwind`.
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            sseq.try_write_to_graph(g, r, differentials, products.iter(), header_closure)
        }))
        .map_err(|_| {
            PyRuntimeError::new_err(
                "write_to_graph panicked (e.g. too many classes for the node patterns)",
            )
        })?;
        let res = outcome.map_err(PyRuntimeError::new_err)?;
        raise_io(err, res)
    }

    /// Generate a charting backend pyclass wrapping the upstream
    /// `$Rs<PyFileWriter>`, with the flattened `Backend` trait methods.
    macro_rules! charting_backend {
        ($Name:ident, $Rs:ty, $ext:literal, $doc:literal, [$($extra:tt)*]) => {
            #[doc = $doc]
            ///
            /// # Storage
            ///
            /// Holds the upstream backend in an `Option` (plain owned value,
            /// not interior mutability): PyO3 hands out `&mut self` under its
            /// runtime borrow check, so the manual `Backend` methods just take
            /// `&mut self`. `write_to_graph` *consumes* the backend (its `Drop`
            /// emits the closing tag), so it `take()`s the `Option`, after which
            /// the backend is `None` and further calls raise. The shared `err`
            /// slot (cloned into the `PyFileWriter`) carries Python `.write`
            /// exceptions back across the upstream `io::Write` boundary.
            #[pyclass(unsendable)]
            pub struct $Name {
                inner: Option<$Rs>,
                err: Rc<RefCell<Option<PyErr>>>,
            }

            impl $Name {
                /// Run a `Backend` method on the live inner backend, guarding
                /// against a consumed backend (`RuntimeError`), containing any
                /// upstream panic (`catch_unwind` -> `RuntimeError`), and
                /// re-raising a recorded Python `.write` exception.
                fn with_inner<F>(&mut self, f: F) -> PyResult<()>
                where
                    F: FnOnce(&mut $Rs) -> io::Result<()>,
                {
                    let res = {
                        let inner = self.inner.as_mut().ok_or_else(|| {
                            PyRuntimeError::new_err(
                                "backend was already consumed by write_to_graph",
                            )
                        })?;
                        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(inner)))
                            .map_err(|_| panic_msg())?
                    };
                    raise_io(&self.err, res)
                }
            }

            #[pymethods]
            impl $Name {
                /// The file extension commonly used for this backend's output.
                #[classattr]
                #[allow(non_upper_case_globals)]
                const EXT: &'static str = $ext;

                /// Wrap a Python file-like object (anything with a `.write`
                /// accepting `str` or `bytes`, e.g. `io.StringIO`,
                /// `io.BytesIO`, an `open(...)` handle, or `sys.stdout`).
                #[new]
                fn py_new(file: Py<PyAny>) -> Self {
                    let (writer, err) = new_writer(file);
                    $Name {
                        inner: Some(<$Rs>::new(writer)),
                        err,
                    }
                }

                /// Write the chart header for a chart whose maximal bidegree is
                /// `max`.
                fn header(&mut self, max: &Bidegree) -> PyResult<()> {
                    self.with_inner(|g| g.header(max.0))
                }

                /// Draw the background grid and axis labels up to `max`
                /// (calls `header` then the grid lines/labels).
                fn init(&mut self, max: &Bidegree) -> PyResult<()> {
                    self.with_inner(|g| g.init(max.0))
                }

                /// Draw a line from `start` to `end` with CSS/TikZ class
                /// `style`.
                fn line(&mut self, start: &Bidegree, end: &Bidegree, style: &str) -> PyResult<()> {
                    self.with_inner(|g| g.line(start.0, end.0, style))
                }

                /// Draw `content` near bidegree `b` with the given
                /// `orientation`. `SvgBackend` supports only `Left`/`Below`
                /// (others raise via the panic guard).
                fn text(
                    &mut self,
                    b: &Bidegree,
                    content: String,
                    orientation: Orientation,
                ) -> PyResult<()> {
                    let orientation = RsOrientation::from(orientation);
                    self.with_inner(|g| g.text(b.0, content, orientation))
                }

                /// Draw `n` nodes (classes) at bidegree `b`. Must be called for
                /// a bidegree before any `structline` referencing it.
                fn node(&mut self, b: &Bidegree, n: usize) -> PyResult<()> {
                    self.with_inner(|g| g.node(b.0, n))
                }

                /// Draw a structure line between two basis generators, with an
                /// optional CSS/TikZ class.
                #[pyo3(signature = (source, target, style=None))]
                fn structline(
                    &mut self,
                    source: &BidegreeGenerator,
                    target: &BidegreeGenerator,
                    style: Option<&str>,
                ) -> PyResult<()> {
                    self.with_inner(|g| g.structline(source.0, target.0, style))
                }

                /// Draw the structure lines encoded by a matrix between the
                /// classes at `source` and `target` (`matrix[k][l] != 0` draws
                /// the line from source generator `k` to target generator `l`).
                #[pyo3(signature = (source, target, matrix, class_=None))]
                fn structline_matrix(
                    &mut self,
                    source: &Bidegree,
                    target: &Bidegree,
                    matrix: Vec<Vec<u32>>,
                    class_: Option<&str>,
                ) -> PyResult<()> {
                    self.with_inner(|g| g.structline_matrix(source.0, target.0, matrix, class_))
                }

                $($extra)*
            }
        };
    }

    charting_backend!(
        SvgBackend,
        RsSvgBackend<PyFileWriter>,
        "svg",
        "An SVG charting backend writing to a Python file-like object.",
        [
            /// Write the node-pattern legend SVG to `file`.
            #[staticmethod]
            fn legend(file: Py<PyAny>) -> PyResult<()> {
                let (writer, err) = new_writer(file);
                let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    RsSvgBackend::legend(writer)
                }))
                .map_err(|_| panic_msg())?;
                raise_io(&err, res)
            }
        ]
    );

    charting_backend!(
        TikzBackend,
        RsTikzBackend<PyFileWriter>,
        "tex",
        "A TikZ charting backend writing to a Python file-like object.",
        []
    );

    /// Register the charting backends.
    ///
    /// `SvgBackend`/`TikzBackend` are generated by the `charting_backend!`
    /// macro, so the `#[pymodule]` proc-macro (which scans for `#[pyclass]`
    /// items at expansion time, before the `macro_rules!` invocation is
    /// expanded) does not auto-collect them. Every other pyclass in this module
    /// is written out directly and auto-registers; these two are added by hand
    /// here. (`Orientation`, being written directly, is auto-registered.)
    #[pymodule_init]
    fn init_charting(m: &Bound<'_, PyModule>) -> PyResult<()> {
        m.add_class::<SvgBackend>()?;
        m.add_class::<TikzBackend>()?;
        Ok(())
    }
}
