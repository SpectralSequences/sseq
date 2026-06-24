use pyo3::prelude::*;

#[pymodule]
#[pyo3(name = "algebra")]
pub mod algebra_py {
    use std::sync::Arc;

    use ::algebra::module::{
        block_structure::BlockStructure as RsBlockStructure, steenrod_module,
        FDModule as RsFDModule, FPModule as RsFPModule, FreeModule as RsFreeModule,
        HomModule as RsHomModule, Module, OperationGeneratorPair as RsOperationGeneratorPair,
        QuotientModule as RsQuotientModule, RealProjectiveSpace as RsRealProjectiveSpace,
        SteenrodModule as RsSteenrodModule, SuspensionModule as RsSuspensionModule,
        TensorModule as RsTensorModule,
    };
    // Imported on its own line (not folded into the multi-item `module` import
    // above) so that later commits extending that import block do not conflict.
    use ::algebra::module::ActError;
    use ::algebra::{Algebra, Bialgebra, Field as RsField, GeneratedAlgebra};
    use ::fp::prime::{self, Prime};
    use pyo3::basic::CompareOp;
    use pyo3::exceptions::{PyIndexError, PyRuntimeError, PyValueError};

    use super::*;

    /// The concrete monomorphisations the §5.3 module bindings are built over.
    /// Every concrete module the proposal exposes is taken over the
    /// `SteenrodAlgebra` union (see `SteenrodModule` below), so we never need a
    /// generic-over-algebra binding.
    type RsSteenrodAlgebra = ::algebra::SteenrodAlgebra;
    type FDModuleInner = RsFDModule<RsSteenrodAlgebra>;
    type FreeModuleInner = RsFreeModule<RsSteenrodAlgebra>;
    /// The derived modules are monomorphised over `RsSteenrodModule`
    /// (`Arc<dyn Module>`), the boxed dynamic module. The `Module` trait carries
    /// `#[auto_impl(Arc, Box)]`, so `Arc<dyn Module>` itself implements `Module`
    /// (and is `Sized`, unlike `dyn Module`, which the upstream
    /// `TensorModule<M>`/`SuspensionModule<M>` type parameters require). The
    /// factors are therefore accepted as the bound `SteenrodModule` pyclass and
    /// the upstream `new` is given `Arc<RsSteenrodModule>`. Both
    /// `TensorModule<RsSteenrodModule, RsSteenrodModule>` and
    /// `SuspensionModule<RsSteenrodModule>` implement `Module`, so
    /// `into_steenrod_module()` unsizes an `Arc` of either directly.
    type TensorModuleInner = RsTensorModule<RsSteenrodModule, RsSteenrodModule>;
    type SuspensionModuleInner = RsSuspensionModule<RsSteenrodModule>;
    /// The quotient module is monomorphised over the boxed dynamic module
    /// (`RsSteenrodModule = Arc<dyn Module>`), exactly like Tensor/Suspension:
    /// upstream `QuotientModule<M>::new` takes `Arc<M>`, so the inner module is
    /// accepted as the bound `SteenrodModule` pyclass and wrapped once more in
    /// an `Arc`. `QuotientModule<RsSteenrodModule>::Algebra` is
    /// `SteenrodAlgebra`, so `into_steenrod_module()` unsizes the stored `Arc`
    /// directly into a `SteenrodModule`.
    type QuotientModuleInner = RsQuotientModule<RsSteenrodModule>;
    /// The Hom module is monomorphised the same way over `RsSteenrodModule` for
    /// its *target*; its *source* is the concrete `FreeModule<SteenrodAlgebra>`
    /// upstream requires. Crucially `HomModule<M>::Algebra` is `Field` (the
    /// ground field), *not* `SteenrodAlgebra`: the module is the graded
    /// vector space `Hom(source, target)`, only acted on by scalars. It is
    /// therefore *not* a `SteenrodModule` and exposes no
    /// `into_steenrod_module()`/`algebra()` (see the binding for why those are
    /// deferred). The flattened `Module` method set is still shared via the
    /// algebra-generic `module_*` helpers above.
    type HomModuleInner = RsHomModule<RsSteenrodModule>;
    type RpInner = RsRealProjectiveSpace<RsSteenrodAlgebra>;
    /// The finitely presented module is monomorphised over the concrete
    /// `SteenrodAlgebra` (like `FreeModule`/`FDModule`), since upstream
    /// `FinitelyPresentedModule::new` takes `Arc<A>` and the module's own
    /// generators/relations are concrete `FreeModule<SteenrodAlgebra>`s. The
    /// inner module is held in an `Arc` so `into_steenrod_module()` can unsize
    /// it directly (the `FreeModule` Arc-unsizing pattern) and so the mutating
    /// `add_generators`/`add_relations` can take `&mut` via `Arc::get_mut` (the
    /// `QuotientModule` pattern: mutation fails while a box shares the `Arc`).
    type FPModuleInner = RsFPModule<RsSteenrodAlgebra>;
    /// A borrowed trait object over the algebra union. The flattened `Module`
    /// method set is implemented once against this type and shared by every
    /// concrete module pyclass and by `SteenrodModule` via dynamic dispatch.
    type DynModule = dyn Module<Algebra = RsSteenrodAlgebra>;

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

    fn checked_same_prime(lhs: u32, rhs: u32) -> PyResult<()> {
        if lhs == rhs {
            Ok(())
        } else {
            Err(PyValueError::new_err(format!(
                "prime mismatch: {lhs} != {rhs}"
            )))
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

    /// Ensure a result slice is long enough to receive a product landing in a
    /// space of dimension `dim`, raising `ValueError` rather than letting an
    /// upstream `add_basis_element` index panic.
    fn checked_result_len(len: usize, dim: usize) -> PyResult<()> {
        if len >= dim {
            Ok(())
        } else {
            Err(PyValueError::new_err(format!(
                "result has length {len} but the target degree has dimension {dim}"
            )))
        }
    }

    fn non_negative_degree(degree: i32) -> PyResult<()> {
        if degree >= 0 {
            Ok(())
        } else {
            Err(PyIndexError::new_err(format!(
                "degree {degree} is negative"
            )))
        }
    }

    /// Convert a Python value (`dict`/`list`/`int`/`float`/`str`/`bool`/`None`)
    /// into a `serde_json::Value`. This is the minimal hand-rolled half of the
    /// `serde_json::Value` <-> Python bridge described in API_PROPOSAL §2.6
    /// (we have no `pythonize` dependency); only the directions exercised by
    /// `SteenrodAlgebra.from_json` are implemented. Booleans are checked before
    /// integers because Python `bool` is a subclass of `int`. Raises
    /// `ValueError` for unsupported types or non-finite floats rather than
    /// panicking.
    fn py_to_json(value: &Bound<'_, PyAny>) -> PyResult<serde_json::Value> {
        use pyo3::types::{PyBool, PyDict, PyFloat, PyInt, PyList, PyString, PyTuple};
        if value.is_none() {
            return Ok(serde_json::Value::Null);
        }
        if let Ok(b) = value.cast::<PyBool>() {
            return Ok(serde_json::Value::Bool(b.is_true()));
        }
        if let Ok(i) = value.cast::<PyInt>() {
            // Accept the full `[i64::MIN, u64::MAX]` range JSON numbers can
            // represent. Try signed first, then unsigned for the
            // `(i64::MAX, u64::MAX]` tail; anything outside that range raises
            // `ValueError` (the taxonomy) rather than leaking `OverflowError`.
            if let Ok(n) = i.extract::<i64>() {
                return Ok(serde_json::Value::from(n));
            }
            if let Ok(n) = i.extract::<u64>() {
                return Ok(serde_json::Value::from(n));
            }
            return Err(PyValueError::new_err(
                "integer out of range for JSON (must fit in i64 or u64)",
            ));
        }
        if let Ok(f) = value.cast::<PyFloat>() {
            let f: f64 = f.extract()?;
            return serde_json::Number::from_f64(f)
                .map(serde_json::Value::Number)
                .ok_or_else(|| PyValueError::new_err("cannot represent non-finite float as JSON"));
        }
        if let Ok(s) = value.cast::<PyString>() {
            return Ok(serde_json::Value::String(s.extract()?));
        }
        if let Ok(dict) = value.cast::<PyDict>() {
            let mut map = serde_json::Map::with_capacity(dict.len());
            for (k, v) in dict.iter() {
                let key: String = k
                    .cast::<PyString>()
                    .map_err(|_| PyValueError::new_err("JSON object keys must be strings"))?
                    .extract()?;
                map.insert(key, py_to_json(&v)?);
            }
            return Ok(serde_json::Value::Object(map));
        }
        if let Ok(list) = value.cast::<PyList>() {
            let mut arr = Vec::with_capacity(list.len());
            for item in list.iter() {
                arr.push(py_to_json(&item)?);
            }
            return Ok(serde_json::Value::Array(arr));
        }
        if let Ok(tuple) = value.cast::<PyTuple>() {
            let mut arr = Vec::with_capacity(tuple.len());
            for item in tuple.iter() {
                arr.push(py_to_json(&item)?);
            }
            return Ok(serde_json::Value::Array(arr));
        }
        Err(PyValueError::new_err(format!(
            "cannot convert {} to JSON",
            value.get_type().name()?
        )))
    }

    #[pyclass] // This will be part of the module
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum AlgebraType {
        Adem,
        Milnor,
    }

    impl From<AlgebraType> for ::algebra::AlgebraType {
        fn from(value: AlgebraType) -> Self {
            match value {
                AlgebraType::Adem => ::algebra::AlgebraType::Adem,
                AlgebraType::Milnor => ::algebra::AlgebraType::Milnor,
            }
        }
    }

    /// A basis element of the Milnor algebra: a product of exterior generators
    /// `Q_k` (encoded as the bitmask `q_part`) and a polynomial part `P(p_part)`.
    #[pyclass(name = "MilnorBasisElement", skip_from_py_object)]
    #[derive(Clone)]
    pub struct MilnorBasisElement(::algebra::milnor_algebra::MilnorBasisElement);

    #[pymethods]
    impl MilnorBasisElement {
        #[new]
        #[pyo3(signature = (p_part, q_part = 0, degree = 0))]
        pub fn new(p_part: Vec<u32>, q_part: u32, degree: i32) -> Self {
            MilnorBasisElement(::algebra::milnor_algebra::MilnorBasisElement {
                q_part,
                p_part,
                degree,
            })
        }

        #[getter]
        pub fn q_part(&self) -> u32 {
            self.0.q_part
        }

        #[setter]
        pub fn set_q_part(&mut self, value: u32) {
            self.0.q_part = value;
        }

        #[getter]
        pub fn p_part(&self) -> Vec<u32> {
            self.0.p_part.clone()
        }

        #[setter]
        pub fn set_p_part(&mut self, value: Vec<u32>) {
            self.0.p_part = value;
        }

        #[getter]
        pub fn degree(&self) -> i32 {
            self.0.degree
        }

        #[setter]
        pub fn set_degree(&mut self, value: i32) {
            self.0.degree = value;
        }

        /// Recompute the `degree` field from the `p_part`/`q_part` at prime `p`.
        pub fn compute_degree(&mut self, p: u32) -> PyResult<()> {
            self.0.compute_degree(valid_prime(p)?);
            Ok(())
        }

        pub fn __repr__(&self) -> String {
            format!(
                "MilnorBasisElement(p_part={:?}, q_part={}, degree={})",
                self.0.p_part, self.0.q_part, self.0.degree
            )
        }

        pub fn __str__(&self) -> String {
            format!("{}", self.0)
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
    }

    /// A Milnor profile function, describing a sub-Hopf-algebra of the Steenrod
    /// algebra.
    #[pyclass(name = "MilnorProfile")]
    pub struct MilnorProfile(::algebra::milnor_algebra::MilnorProfile);

    impl MilnorProfile {
        /// `MilnorProfile` upstream is intentionally not `Clone`; reconstruct a
        /// fresh copy from its public fields when we need to hand one to the
        /// algebra constructor or return one to Python.
        fn to_rust(&self) -> ::algebra::milnor_algebra::MilnorProfile {
            ::algebra::milnor_algebra::MilnorProfile {
                truncated: self.0.truncated,
                q_part: self.0.q_part,
                p_part: self.0.p_part.clone(),
            }
        }
    }

    #[pymethods]
    impl MilnorProfile {
        #[new]
        #[pyo3(signature = (truncated = false, q_part = u32::MAX, p_part = Vec::new()))]
        pub fn new(truncated: bool, q_part: u32, p_part: Vec<u32>) -> Self {
            MilnorProfile(::algebra::milnor_algebra::MilnorProfile {
                truncated,
                q_part,
                p_part,
            })
        }

        #[getter]
        pub fn truncated(&self) -> bool {
            self.0.truncated
        }

        #[setter]
        pub fn set_truncated(&mut self, value: bool) {
            self.0.truncated = value;
        }

        #[getter]
        pub fn q_part(&self) -> u32 {
            self.0.q_part
        }

        #[setter]
        pub fn set_q_part(&mut self, value: u32) {
            self.0.q_part = value;
        }

        #[getter(p_part)]
        pub fn profile_p_part(&self) -> Vec<u32> {
            self.0.p_part.clone()
        }

        #[setter(p_part)]
        pub fn set_p_part(&mut self, value: Vec<u32>) {
            self.0.p_part = value;
        }

        pub fn is_trivial(&self) -> bool {
            self.0.is_trivial()
        }

        pub fn get_p_part(&self, i: usize) -> u32 {
            self.0.get_p_part(i)
        }

        pub fn is_valid(&self) -> bool {
            self.0.is_valid()
        }

        pub fn is_an(&self, generic: bool) -> bool {
            self.0.is_an(generic)
        }

        pub fn __repr__(&self) -> String {
            format!(
                "MilnorProfile(truncated={}, q_part={}, p_part={:?})",
                self.0.truncated, self.0.q_part, self.0.p_part
            )
        }
    }

    #[pyclass]
    pub struct MilnorAlgebra(::algebra::MilnorAlgebra);

    impl MilnorAlgebra {
        /// Lazily compute book-keeping up to `degree`. The Milnor algebra is
        /// infinite-dimensional and its internal `OnceVec` tables panic when
        /// indexed past the computed range, so every degree-indexed Python
        /// method funnels through here first. `compute_basis` is idempotent and
        /// cheap to re-call, so this is a safe (if slightly eager) way to avoid
        /// cross-boundary panics; it is a no-op for negative degrees.
        fn ensure_basis(&self, degree: i32) {
            if degree >= 0 {
                self.0.compute_basis(degree);
            }
        }

        /// Validate two factor degrees and compute the (basis-populated) target
        /// degree of their product.
        fn product_target(&self, r_degree: i32, s_degree: i32) -> PyResult<i32> {
            non_negative_degree(r_degree)?;
            non_negative_degree(s_degree)?;
            let target = r_degree
                .checked_add(s_degree)
                .ok_or_else(|| PyValueError::new_err("product degree overflows i32"))?;
            self.ensure_basis(target);
            Ok(target)
        }

        fn checked_basis_index(&self, degree: i32, idx: usize) -> PyResult<()> {
            let dim = self.0.dimension(degree);
            if idx < dim {
                Ok(())
            } else {
                Err(PyIndexError::new_err(format!(
                    "index {idx} out of range for degree {degree} (dimension {dim})"
                )))
            }
        }
    }

    #[pymethods]
    impl MilnorAlgebra {
        #[new]
        #[pyo3(signature = (p, unstable_enabled = false))]
        pub fn new(p: u32, unstable_enabled: bool) -> PyResult<Self> {
            Ok(MilnorAlgebra(::algebra::MilnorAlgebra::new(
                valid_prime(p)?,
                unstable_enabled,
            )))
        }

        /// Construct a Milnor algebra restricted to the given profile. Raises
        /// `ValueError` for an invalid profile rather than panicking (upstream
        /// `new_with_profile` asserts validity).
        #[staticmethod]
        #[pyo3(signature = (p, profile, unstable_enabled = false))]
        pub fn new_with_profile(
            p: u32,
            profile: PyRef<'_, MilnorProfile>,
            unstable_enabled: bool,
        ) -> PyResult<Self> {
            let p = valid_prime(p)?;
            let profile = profile.to_rust();
            if !profile.is_valid() {
                return Err(PyValueError::new_err("invalid Milnor profile"));
            }
            Ok(MilnorAlgebra(::algebra::MilnorAlgebra::new_with_profile(
                p,
                profile,
                unstable_enabled,
            )))
        }

        // --- Algebra trait surface --------------------------------------------

        /// The prime as a plain `int` (`ValidPrime` is never exposed).
        pub fn prime(&self) -> u32 {
            self.0.prime().as_u32()
        }

        pub fn compute_basis(&self, degree: i32) {
            self.ensure_basis(degree);
        }

        pub fn dimension(&self, degree: i32) -> usize {
            if degree < 0 {
                return 0;
            }
            self.ensure_basis(degree);
            self.0.dimension(degree)
        }

        pub fn basis_element_to_string(&self, degree: i32, idx: usize) -> PyResult<String> {
            self.0.try_basis_element_to_string(degree, idx).ok_or_else(|| {
                PyIndexError::new_err(format!(
                    "no basis element at degree {degree} index {idx}"
                ))
            })
        }

        /// Parse a basis element, returning `(degree, index)`. Raises
        /// `ValueError` if the string does not parse, or if it names an element
        /// that is not present in this (possibly profiled) algebra.
        ///
        /// Upstream's `basis_element_from_string` is now total: a parseable but
        /// absent/out-of-profile name (e.g. `"Sq0"`, `"P0"`, `"Q_5"`) returns
        /// `None` rather than panicking. We map that `None` to `ValueError`.
        pub fn basis_element_from_string(&self, elt: &str) -> PyResult<(i32, usize)> {
            self.0.basis_element_from_string(elt).ok_or_else(|| {
                PyValueError::new_err(format!(
                    "{elt} does not name a basis element of this algebra"
                ))
            })
        }

        pub fn element_to_string(
            &self,
            py: Python<'_>,
            degree: i32,
            element: &Bound<'_, PyAny>,
        ) -> PyResult<String> {
            non_negative_degree(degree)?;
            self.ensure_basis(degree);
            let element = crate::fp_py::extract_input_owned(py, element)?;
            checked_same_prime(element.prime().as_u32(), self.0.prime().as_u32())?;
            checked_equal_len(element.len(), self.0.dimension(degree))?;
            Ok(self.0.element_to_string(degree, element.as_slice()))
        }

        pub fn multiply_basis_elements(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            r_degree: i32,
            r_idx: usize,
            s_degree: i32,
            s_idx: usize,
        ) -> PyResult<()> {
            let p = self.0.prime().as_u32();
            // Reduce the coefficient mod p before handing it to upstream, which
            // computes `coeff * v` (milnor_algebra.rs ~555) before reducing and
            // would overflow (panicking in debug, wrapping in release) for large
            // `coeff`. The algebra is over F_p, so this is mathematically
            // equivalent.
            let coeff = coeff % p;
            let target = self.product_target(r_degree, s_degree)?;
            let dim = self.0.dimension(target);
            self.checked_basis_index(r_degree, r_idx)?;
            self.checked_basis_index(s_degree, s_idx)?;
            crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                checked_same_prime(res.prime().as_u32(), p)?;
                checked_result_len(res.as_slice().len(), dim)?;
                self.0
                    .multiply_basis_elements(res.copy(), coeff, r_degree, r_idx, s_degree, s_idx);
                Ok(())
            })
        }

        pub fn multiply_basis_element_by_element(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            r_degree: i32,
            r_idx: usize,
            s_degree: i32,
            s: &Bound<'_, PyAny>,
        ) -> PyResult<()> {
            let p = self.0.prime().as_u32();
            // See `multiply_basis_elements`: reduce mod p to avoid the upstream
            // `coeff * v` overflow.
            let coeff = coeff % p;
            let target = self.product_target(r_degree, s_degree)?;
            let dim = self.0.dimension(target);
            self.checked_basis_index(r_degree, r_idx)?;
            let s = crate::fp_py::extract_input_owned(py, s)?;
            checked_same_prime(s.prime().as_u32(), p)?;
            checked_equal_len(s.len(), self.0.dimension(s_degree))?;
            crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                checked_same_prime(res.prime().as_u32(), p)?;
                checked_result_len(res.as_slice().len(), dim)?;
                self.0.multiply_basis_element_by_element(
                    res.copy(),
                    coeff,
                    r_degree,
                    r_idx,
                    s_degree,
                    s.as_slice(),
                );
                Ok(())
            })
        }

        pub fn multiply_element_by_basis_element(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            r_degree: i32,
            r: &Bound<'_, PyAny>,
            s_degree: i32,
            s_idx: usize,
        ) -> PyResult<()> {
            let p = self.0.prime().as_u32();
            // See `multiply_basis_elements`: reduce mod p to avoid the upstream
            // `coeff * v` overflow.
            let coeff = coeff % p;
            let target = self.product_target(r_degree, s_degree)?;
            let dim = self.0.dimension(target);
            self.checked_basis_index(s_degree, s_idx)?;
            let r = crate::fp_py::extract_input_owned(py, r)?;
            checked_same_prime(r.prime().as_u32(), p)?;
            checked_equal_len(r.len(), self.0.dimension(r_degree))?;
            crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                checked_same_prime(res.prime().as_u32(), p)?;
                checked_result_len(res.as_slice().len(), dim)?;
                self.0.multiply_element_by_basis_element(
                    res.copy(),
                    coeff,
                    r_degree,
                    r.as_slice(),
                    s_degree,
                    s_idx,
                );
                Ok(())
            })
        }

        pub fn multiply_element_by_element(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            r_degree: i32,
            r: &Bound<'_, PyAny>,
            s_degree: i32,
            s: &Bound<'_, PyAny>,
        ) -> PyResult<()> {
            let p = self.0.prime().as_u32();
            // See `multiply_basis_elements`: reduce mod p to avoid the upstream
            // `coeff * v` overflow.
            let coeff = coeff % p;
            let target = self.product_target(r_degree, s_degree)?;
            let dim = self.0.dimension(target);
            let r = crate::fp_py::extract_input_owned(py, r)?;
            let s = crate::fp_py::extract_input_owned(py, s)?;
            checked_same_prime(r.prime().as_u32(), p)?;
            checked_same_prime(s.prime().as_u32(), p)?;
            checked_equal_len(r.len(), self.0.dimension(r_degree))?;
            checked_equal_len(s.len(), self.0.dimension(s_degree))?;
            crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                checked_same_prime(res.prime().as_u32(), p)?;
                checked_result_len(res.as_slice().len(), dim)?;
                self.0.multiply_element_by_element(
                    res.copy(),
                    coeff,
                    r_degree,
                    r.as_slice(),
                    s_degree,
                    s.as_slice(),
                );
                Ok(())
            })
        }

        pub fn default_filtration_one_products(&self) -> Vec<(String, i32, usize)> {
            self.0.default_filtration_one_products()
        }

        // --- GeneratedAlgebra trait surface -----------------------------------

        pub fn generators(&self, degree: i32) -> PyResult<Vec<usize>> {
            if degree < 0 {
                return Ok(Vec::new());
            }
            self.ensure_basis(degree);
            Ok(self.0.generators(degree))
        }

        pub fn generator_to_string(&self, degree: i32, idx: usize) -> PyResult<String> {
            non_negative_degree(degree)?;
            self.ensure_basis(degree);
            self.checked_basis_index(degree, idx)?;
            Ok(self.0.generator_to_string(degree, idx))
        }

        pub fn decompose_basis_element(
            &self,
            degree: i32,
            idx: usize,
        ) -> PyResult<Vec<(u32, (i32, usize), (i32, usize))>> {
            non_negative_degree(degree)?;
            self.ensure_basis(degree);
            self.checked_basis_index(degree, idx)?;
            // Decomposition is only defined for non-generators. Upstream has two
            // underflow panic paths, both of which hit precisely the
            // indecomposable elements reported by `generators`:
            //   * `decompose_basis_element_ppart` (q_part == 0) computes
            //     `p_part[0..len - 1]`; with `len == 0` this underflows
            //     (milnor_algebra.rs ~1607). An empty `p_part` with
            //     `q_part == 0` can only be the degree-0 unit.
            //   * `decompose_basis_element_qpart` (q_part != 0) computes
            //     `prime().pow(i - 1)` with `i = q_part.trailing_zeros()`; for
            //     `Q_0` (`q_part == 1`) `i == 0`, so `i - 1` underflows
            //     (milnor_algebra.rs ~1533-1536). `Q_0` lives in degree 1 and
            //     is `generators(1) == [0]`.
            // The generators-based guard (matching the Adem branch) therefore
            // covers both panic preconditions; it also rejects ordinary
            // generators such as `P(p^k)`, keeping the two variants consistent.
            if degree == 0 || self.0.generators(degree).contains(&idx) {
                return Err(PyValueError::new_err(
                    "the unit and algebra generators are indecomposable",
                ));
            }
            Ok(self.0.decompose_basis_element(degree, idx))
        }

        pub fn generating_relations(
            &self,
            degree: i32,
        ) -> PyResult<Vec<Vec<(u32, (i32, usize), (i32, usize))>>> {
            if degree < 0 {
                return Ok(Vec::new());
            }
            self.ensure_basis(degree);
            Ok(self.0.generating_relations(degree))
        }

        // --- Bialgebra trait surface ------------------------------------------

        /// Compute a coproduct. Only supported at `p = 2` upstream; raises
        /// `ValueError` at odd primes rather than panicking on the assertion.
        pub fn coproduct(
            &self,
            degree: i32,
            idx: usize,
        ) -> PyResult<Vec<(i32, usize, i32, usize)>> {
            if self.0.prime().as_u32() != 2 {
                return Err(PyValueError::new_err(
                    "coproduct is only supported at p = 2",
                ));
            }
            non_negative_degree(degree)?;
            self.ensure_basis(degree);
            self.checked_basis_index(degree, idx)?;
            Ok(self.0.coproduct(degree, idx))
        }

        pub fn decompose(&self, degree: i32, idx: usize) -> PyResult<Vec<(i32, usize)>> {
            non_negative_degree(degree)?;
            self.ensure_basis(degree);
            self.checked_basis_index(degree, idx)?;
            Ok(self.0.decompose(degree, idx))
        }

        // --- Milnor-specific methods ------------------------------------------

        pub fn generic(&self) -> bool {
            self.0.generic()
        }

        pub fn q(&self) -> i32 {
            self.0.q()
        }

        pub fn profile(&self) -> MilnorProfile {
            let profile = self.0.profile();
            MilnorProfile(::algebra::milnor_algebra::MilnorProfile {
                truncated: profile.truncated,
                q_part: profile.q_part,
                p_part: profile.p_part.clone(),
            })
        }

        pub fn basis_element_from_index(
            &self,
            degree: i32,
            idx: usize,
        ) -> PyResult<MilnorBasisElement> {
            non_negative_degree(degree)?;
            self.ensure_basis(degree);
            self.checked_basis_index(degree, idx)?;
            Ok(MilnorBasisElement(
                self.0.basis_element_from_index(degree, idx).clone(),
            ))
        }

        pub fn try_basis_element_to_index(
            &self,
            elt: PyRef<'_, MilnorBasisElement>,
        ) -> Option<usize> {
            if elt.0.degree < 0 {
                return None;
            }
            self.ensure_basis(elt.0.degree);
            self.0.try_basis_element_to_index(&elt.0)
        }

        /// Like `try_basis_element_to_index`, but raises `ValueError` if the
        /// element is not in the algebra (upstream panics).
        pub fn basis_element_to_index(
            &self,
            elt: PyRef<'_, MilnorBasisElement>,
        ) -> PyResult<usize> {
            non_negative_degree(elt.0.degree)?;
            self.ensure_basis(elt.0.degree);
            self.0
                .try_basis_element_to_index(&elt.0)
                .ok_or_else(|| PyValueError::new_err(format!("element not in algebra: {}", elt.0)))
        }

        /// The list of `P(R)` partitions in degree `t`.
        pub fn ppart_table(&self, t: i32) -> PyResult<Vec<Vec<u32>>> {
            non_negative_degree(t)?;
            // The internal table is indexed by `degree / q`, so compute enough
            // book-keeping that index `t` is in range at every prime.
            let needed = t
                .checked_mul(self.0.q())
                .ok_or_else(|| PyValueError::new_err("degree overflows i32"))?;
            self.ensure_basis(needed);
            Ok(self.0.ppart_table(t).to_vec())
        }

        /// The degree and index of `Q_1^e P(x)`. Raises `ValueError` if that
        /// element is not in the (profiled) algebra (upstream's non-panicking
        /// `try_beps_pn` returns `None`).
        pub fn beps_pn(&self, e: u32, x: u32) -> PyResult<(i32, usize)> {
            self.0.try_beps_pn(e, x).ok_or_else(|| {
                PyValueError::new_err(format!("Q_1^{e} P({x}) is not in the algebra"))
            })
        }

        /// Multiply two `MilnorBasisElement`s, accumulating into `result`.
        pub fn multiply(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            m1: PyRef<'_, MilnorBasisElement>,
            m2: PyRef<'_, MilnorBasisElement>,
        ) -> PyResult<()> {
            let p = self.0.prime().as_u32();
            // See `multiply_basis_elements`: reduce mod p to avoid the upstream
            // `coeff * v` overflow.
            let coeff = coeff % p;
            let target = self.product_target(m1.0.degree, m2.0.degree)?;
            let dim = self.0.dimension(target);
            // Reject elements that are not genuine basis elements of this
            // algebra up front, since the inner multiply panics if an
            // intermediate term cannot be indexed.
            self.ensure_basis(m1.0.degree);
            self.ensure_basis(m2.0.degree);
            if self.0.try_basis_element_to_index(&m1.0).is_none() {
                return Err(PyValueError::new_err(format!(
                    "left factor is not a basis element of this algebra: {}",
                    m1.0
                )));
            }
            if self.0.try_basis_element_to_index(&m2.0).is_none() {
                return Err(PyValueError::new_err(format!(
                    "right factor is not a basis element of this algebra: {}",
                    m2.0
                )));
            }
            crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                checked_same_prime(res.prime().as_u32(), p)?;
                checked_result_len(res.as_slice().len(), dim)?;
                self.0.multiply(res.copy(), coeff, &m1.0, &m2.0);
                Ok(())
            })
        }

        pub fn __repr__(&self) -> String {
            format!("{}", self.0)
        }
    }

    /// A Steenrod power `P^i`, or a Bockstein `b^e`. Mirrors upstream's
    /// `PorBockstein` enum (the pieces of an Adem basis element's
    /// decomposition).
    #[pyclass(name = "PorBockstein")]
    #[derive(Clone, Debug)]
    pub enum PorBockstein {
        P(u32),
        Bockstein(bool),
    }

    /// An Adem basis element of the Steenrod algebra: a sequence of Steenrod
    /// powers `ps` interleaved with Bocksteins encoded in the bitmask
    /// `bocksteins`.
    #[pyclass(name = "AdemBasisElement", skip_from_py_object)]
    #[derive(Clone)]
    pub struct AdemBasisElement(::algebra::adem_algebra::AdemBasisElement);

    #[pymethods]
    impl AdemBasisElement {
        #[new]
        #[pyo3(signature = (ps, bocksteins = 0, degree = 0, p_or_sq = false))]
        pub fn new(ps: Vec<u32>, bocksteins: u32, degree: i32, p_or_sq: bool) -> Self {
            AdemBasisElement(::algebra::adem_algebra::AdemBasisElement {
                degree,
                bocksteins,
                ps,
                p_or_sq,
            })
        }

        #[getter]
        pub fn degree(&self) -> i32 {
            self.0.degree
        }

        #[setter]
        pub fn set_degree(&mut self, value: i32) {
            self.0.degree = value;
        }

        #[getter]
        pub fn bocksteins(&self) -> u32 {
            self.0.bocksteins
        }

        #[setter]
        pub fn set_bocksteins(&mut self, value: u32) {
            self.0.bocksteins = value;
        }

        #[getter]
        pub fn ps(&self) -> Vec<u32> {
            self.0.ps.clone()
        }

        #[setter]
        pub fn set_ps(&mut self, value: Vec<u32>) {
            self.0.ps = value;
        }

        #[getter]
        pub fn p_or_sq(&self) -> bool {
            self.0.p_or_sq
        }

        #[setter]
        pub fn set_p_or_sq(&mut self, value: bool) {
            self.0.p_or_sq = value;
        }

        /// The decomposition into alternating Bocksteins and Steenrod powers,
        /// dropping trivial (`b^0`) Bocksteins. Mirrors the upstream private
        /// `iter_filtered`.
        pub fn iter_filtered(&self) -> Vec<PorBockstein> {
            let bocksteins: Vec<bool> =
                ::fp::prime::iter::BitflagIterator::new(self.0.bocksteins as u64).collect();
            let n = bocksteins.len().max(self.0.ps.len());
            let mut out = Vec::new();
            for i in 0..n {
                if let Some(&b) = bocksteins.get(i) {
                    if b {
                        out.push(PorBockstein::Bockstein(true));
                    }
                }
                if let Some(&p) = self.0.ps.get(i) {
                    out.push(PorBockstein::P(p));
                }
            }
            out
        }

        pub fn __repr__(&self) -> String {
            format!(
                "AdemBasisElement(ps={:?}, bocksteins={}, degree={}, p_or_sq={})",
                self.0.ps, self.0.bocksteins, self.0.degree, self.0.p_or_sq
            )
        }

        pub fn __str__(&self) -> String {
            format!("{}", self.0)
        }

        pub fn __richcmp__(&self, other: &Bound<'_, PyAny>, op: CompareOp) -> bool {
            // Upstream equality compares only `ps` and `bocksteins`.
            let eq = other
                .extract::<PyRef<Self>>()
                .is_ok_and(|other| self.0 == other.0);
            match op {
                CompareOp::Eq => eq,
                CompareOp::Ne => !eq,
                _ => false,
            }
        }
    }

    #[pyclass]
    pub struct AdemAlgebra(::algebra::AdemAlgebra);

    impl AdemAlgebra {
        /// Lazily compute book-keeping up to `degree`. Like `MilnorAlgebra`,
        /// the Adem algebra is infinite-dimensional and its internal `OnceVec`
        /// tables panic when indexed past the computed range, so every
        /// degree-indexed Python method funnels through here first. A no-op for
        /// negative degrees.
        fn ensure_basis(&self, degree: i32) {
            if degree >= 0 {
                self.0.compute_basis(degree);
            }
        }

        fn product_target(&self, r_degree: i32, s_degree: i32) -> PyResult<i32> {
            non_negative_degree(r_degree)?;
            non_negative_degree(s_degree)?;
            let target = r_degree
                .checked_add(s_degree)
                .ok_or_else(|| PyValueError::new_err("product degree overflows i32"))?;
            self.ensure_basis(target);
            Ok(target)
        }

        fn checked_basis_index(&self, degree: i32, idx: usize) -> PyResult<()> {
            let dim = self.0.dimension(degree);
            if idx < dim {
                Ok(())
            } else {
                Err(PyIndexError::new_err(format!(
                    "index {idx} out of range for degree {degree} (dimension {dim})"
                )))
            }
        }
    }

    #[pymethods]
    impl AdemAlgebra {
        #[new]
        #[pyo3(signature = (p, unstable_enabled = false))]
        pub fn new(p: u32, unstable_enabled: bool) -> PyResult<Self> {
            // `generic` is not a constructor flag upstream: it is derived as
            // `p != 2`.
            Ok(AdemAlgebra(::algebra::AdemAlgebra::new(
                valid_prime(p)?,
                unstable_enabled,
            )))
        }

        // --- Algebra trait surface --------------------------------------------

        /// The prime as a plain `int` (`ValidPrime` is never exposed).
        pub fn prime(&self) -> u32 {
            self.0.prime().as_u32()
        }

        pub fn compute_basis(&self, degree: i32) {
            self.ensure_basis(degree);
        }

        pub fn dimension(&self, degree: i32) -> usize {
            if degree < 0 {
                return 0;
            }
            self.ensure_basis(degree);
            self.0.dimension(degree)
        }

        pub fn basis_element_to_string(&self, degree: i32, idx: usize) -> PyResult<String> {
            self.0.try_basis_element_to_string(degree, idx).ok_or_else(|| {
                PyIndexError::new_err(format!(
                    "no basis element at degree {degree} index {idx}"
                ))
            })
        }

        /// Parse a basis element, returning `(degree, index)`. Raises
        /// `ValueError` if the string does not parse, or if it names an element
        /// that is not present in this algebra.
        ///
        /// Upstream's `basis_element_from_string` is now total: a parseable but
        /// absent/inadmissible name (e.g. `"Sq0"`, `"Sq1 Sq1"`) returns `None`
        /// rather than panicking. We map that `None` to `ValueError`.
        pub fn basis_element_from_string(&self, elt: &str) -> PyResult<(i32, usize)> {
            self.0.basis_element_from_string(elt).ok_or_else(|| {
                PyValueError::new_err(format!(
                    "{elt} does not name a basis element of this algebra"
                ))
            })
        }

        pub fn element_to_string(
            &self,
            py: Python<'_>,
            degree: i32,
            element: &Bound<'_, PyAny>,
        ) -> PyResult<String> {
            non_negative_degree(degree)?;
            self.ensure_basis(degree);
            let element = crate::fp_py::extract_input_owned(py, element)?;
            checked_same_prime(element.prime().as_u32(), self.0.prime().as_u32())?;
            checked_equal_len(element.len(), self.0.dimension(degree))?;
            Ok(self.0.element_to_string(degree, element.as_slice()))
        }

        pub fn multiply_basis_elements(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            r_degree: i32,
            r_idx: usize,
            s_degree: i32,
            s_idx: usize,
        ) -> PyResult<()> {
            let p = self.0.prime().as_u32();
            // Reduce the coefficient mod p before handing it to upstream, which
            // computes `coeff * value` (e.g. adem_algebra.rs ~1161) before
            // reducing and would overflow for large `coeff`. The algebra is
            // over F_p, so this is mathematically equivalent.
            let coeff = coeff % p;
            let target = self.product_target(r_degree, s_degree)?;
            let dim = self.0.dimension(target);
            self.checked_basis_index(r_degree, r_idx)?;
            self.checked_basis_index(s_degree, s_idx)?;
            crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                checked_same_prime(res.prime().as_u32(), p)?;
                checked_result_len(res.as_slice().len(), dim)?;
                self.0
                    .multiply_basis_elements(res.copy(), coeff, r_degree, r_idx, s_degree, s_idx);
                Ok(())
            })
        }

        pub fn multiply_basis_element_by_element(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            r_degree: i32,
            r_idx: usize,
            s_degree: i32,
            s: &Bound<'_, PyAny>,
        ) -> PyResult<()> {
            let p = self.0.prime().as_u32();
            let coeff = coeff % p;
            let target = self.product_target(r_degree, s_degree)?;
            let dim = self.0.dimension(target);
            self.checked_basis_index(r_degree, r_idx)?;
            let s = crate::fp_py::extract_input_owned(py, s)?;
            checked_same_prime(s.prime().as_u32(), p)?;
            checked_equal_len(s.len(), self.0.dimension(s_degree))?;
            crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                checked_same_prime(res.prime().as_u32(), p)?;
                checked_result_len(res.as_slice().len(), dim)?;
                self.0.multiply_basis_element_by_element(
                    res.copy(),
                    coeff,
                    r_degree,
                    r_idx,
                    s_degree,
                    s.as_slice(),
                );
                Ok(())
            })
        }

        pub fn multiply_element_by_basis_element(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            r_degree: i32,
            r: &Bound<'_, PyAny>,
            s_degree: i32,
            s_idx: usize,
        ) -> PyResult<()> {
            let p = self.0.prime().as_u32();
            let coeff = coeff % p;
            let target = self.product_target(r_degree, s_degree)?;
            let dim = self.0.dimension(target);
            self.checked_basis_index(s_degree, s_idx)?;
            let r = crate::fp_py::extract_input_owned(py, r)?;
            checked_same_prime(r.prime().as_u32(), p)?;
            checked_equal_len(r.len(), self.0.dimension(r_degree))?;
            crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                checked_same_prime(res.prime().as_u32(), p)?;
                checked_result_len(res.as_slice().len(), dim)?;
                self.0.multiply_element_by_basis_element(
                    res.copy(),
                    coeff,
                    r_degree,
                    r.as_slice(),
                    s_degree,
                    s_idx,
                );
                Ok(())
            })
        }

        pub fn multiply_element_by_element(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            r_degree: i32,
            r: &Bound<'_, PyAny>,
            s_degree: i32,
            s: &Bound<'_, PyAny>,
        ) -> PyResult<()> {
            let p = self.0.prime().as_u32();
            let coeff = coeff % p;
            let target = self.product_target(r_degree, s_degree)?;
            let dim = self.0.dimension(target);
            let r = crate::fp_py::extract_input_owned(py, r)?;
            let s = crate::fp_py::extract_input_owned(py, s)?;
            checked_same_prime(r.prime().as_u32(), p)?;
            checked_same_prime(s.prime().as_u32(), p)?;
            checked_equal_len(r.len(), self.0.dimension(r_degree))?;
            checked_equal_len(s.len(), self.0.dimension(s_degree))?;
            crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                checked_same_prime(res.prime().as_u32(), p)?;
                checked_result_len(res.as_slice().len(), dim)?;
                self.0.multiply_element_by_element(
                    res.copy(),
                    coeff,
                    r_degree,
                    r.as_slice(),
                    s_degree,
                    s.as_slice(),
                );
                Ok(())
            })
        }

        pub fn default_filtration_one_products(&self) -> Vec<(String, i32, usize)> {
            self.0.default_filtration_one_products()
        }

        // --- GeneratedAlgebra trait surface -----------------------------------

        pub fn generators(&self, degree: i32) -> PyResult<Vec<usize>> {
            if degree < 0 {
                return Ok(Vec::new());
            }
            self.ensure_basis(degree);
            Ok(self.0.generators(degree))
        }

        pub fn generator_to_string(&self, degree: i32, idx: usize) -> PyResult<String> {
            non_negative_degree(degree)?;
            self.ensure_basis(degree);
            self.checked_basis_index(degree, idx)?;
            Ok(self.0.generator_to_string(degree, idx))
        }

        pub fn decompose_basis_element(
            &self,
            degree: i32,
            idx: usize,
        ) -> PyResult<Vec<(u32, (i32, usize), (i32, usize))>> {
            non_negative_degree(degree)?;
            self.ensure_basis(degree);
            self.checked_basis_index(degree, idx)?;
            // Decomposition is only defined for non-generators. The degree-0
            // unit has an empty `ps`, so upstream's `b.ps[0]` indexes out of
            // bounds (adem_algebra.rs ~1195/1270); a generator like `Sq^2`
            // decomposes into a factor of degree 0 whose `AdemBasisElement` is
            // not in the basis, so `basis_element_to_index` panics. Both are
            // indecomposable by definition, so we surface a `ValueError` rather
            // than aborting. (Upstream's own test skips generators before
            // calling `decompose_basis_element`.)
            if degree == 0 || self.0.generators(degree).contains(&idx) {
                return Err(PyValueError::new_err(
                    "the unit and algebra generators are indecomposable",
                ));
            }
            Ok(self.0.decompose_basis_element(degree, idx))
        }

        pub fn generating_relations(
            &self,
            degree: i32,
        ) -> PyResult<Vec<Vec<(u32, (i32, usize), (i32, usize))>>> {
            if degree < 0 {
                return Ok(Vec::new());
            }
            self.ensure_basis(degree);
            Ok(self.0.generating_relations(degree))
        }

        // --- Bialgebra trait surface ------------------------------------------

        /// Compute a coproduct. Raises `ValueError` for inputs that would trip
        /// an upstream assertion: a non-`q`-divisible degree in the generic
        /// case, or a nonzero index in the `p = 2` case (adem_algebra.rs
        /// ~1398/1409).
        pub fn coproduct(
            &self,
            degree: i32,
            idx: usize,
        ) -> PyResult<Vec<(i32, usize, i32, usize)>> {
            non_negative_degree(degree)?;
            self.ensure_basis(degree);
            self.checked_basis_index(degree, idx)?;
            if self.0.generic() {
                if degree != 1 {
                    let q = 2 * self.0.prime().as_u32() - 2;
                    if (degree as u32) % q != 0 {
                        return Err(PyValueError::new_err(format!(
                            "coproduct expects a degree divisible by {q}, got {degree}"
                        )));
                    }
                }
            } else if idx != 0 {
                return Err(PyValueError::new_err(
                    "at p = 2 the coproduct expects index 0",
                ));
            }
            Ok(self.0.coproduct(degree, idx))
        }

        pub fn decompose(&self, degree: i32, idx: usize) -> PyResult<Vec<(i32, usize)>> {
            non_negative_degree(degree)?;
            self.ensure_basis(degree);
            self.checked_basis_index(degree, idx)?;
            Ok(self.0.decompose(degree, idx))
        }

        // --- Adem-specific methods --------------------------------------------

        pub fn generic(&self) -> bool {
            self.0.generic()
        }

        pub fn q(&self) -> i32 {
            self.0.q()
        }

        pub fn basis_element_from_index(
            &self,
            degree: i32,
            idx: usize,
        ) -> PyResult<AdemBasisElement> {
            non_negative_degree(degree)?;
            self.ensure_basis(degree);
            self.checked_basis_index(degree, idx)?;
            Ok(AdemBasisElement(
                self.0.basis_element_from_index(degree, idx).clone(),
            ))
        }

        pub fn try_basis_element_to_index(
            &self,
            elt: PyRef<'_, AdemBasisElement>,
        ) -> Option<usize> {
            if elt.0.degree < 0 {
                return None;
            }
            self.ensure_basis(elt.0.degree);
            self.0.try_basis_element_to_index(&elt.0)
        }

        /// Like `try_basis_element_to_index`, but raises `ValueError` if the
        /// element is not in the algebra (upstream panics).
        pub fn basis_element_to_index(&self, elt: PyRef<'_, AdemBasisElement>) -> PyResult<usize> {
            non_negative_degree(elt.0.degree)?;
            self.ensure_basis(elt.0.degree);
            self.0
                .try_basis_element_to_index(&elt.0)
                .ok_or_else(|| PyValueError::new_err(format!("element not in algebra: {}", elt.0)))
        }

        /// The degree and index of `b^e P^x`. Raises `ValueError` if that
        /// element is not in the algebra (upstream's non-panicking `try_beps_pn`
        /// returns `None`).
        pub fn beps_pn(&self, e: u32, x: u32) -> PyResult<(i32, usize)> {
            self.0.try_beps_pn(e, x).ok_or_else(|| {
                PyValueError::new_err(format!("b^{e} P^{x} is not in the algebra"))
            })
        }

        pub fn __repr__(&self) -> String {
            format!("{}", self.0)
        }
    }

    /// The `enum_dispatch` union of the Adem and Milnor Steenrod algebras
    /// (`::algebra::SteenrodAlgebra`). A single value is *either* Adem or Milnor
    /// at runtime; every `Algebra`/`GeneratedAlgebra`/`Bialgebra` method
    /// dispatches to the active variant. This is one pyclass that wraps the
    /// union and dispatches; it does not inherit from `MilnorAlgebra`/
    /// `AdemAlgebra`.
    #[pyclass]
    pub struct SteenrodAlgebra(Arc<::algebra::SteenrodAlgebra>);

    impl SteenrodAlgebra {
        /// Wrap an already-shared algebra (e.g. the `Arc` a module hands back
        /// from `Module::algebra`) into the bound pyclass without cloning the
        /// underlying algebra. This is how a module's `algebra()` accessor
        /// returns a `SteenrodAlgebra` to Python.
        pub(crate) fn from_arc(algebra: Arc<::algebra::SteenrodAlgebra>) -> Self {
            SteenrodAlgebra(algebra)
        }

        /// A cheap clone of the shared algebra handle, for feeding module
        /// constructors that take `Arc<SteenrodAlgebra>` upstream.
        pub(crate) fn arc(&self) -> Arc<::algebra::SteenrodAlgebra> {
            Arc::clone(&self.0)
        }

        /// Lazily compute book-keeping up to `degree`. Both underlying algebras
        /// are infinite-dimensional with `OnceVec` tables that panic when
        /// indexed past the computed range, so every degree-indexed Python
        /// method funnels through here first (idempotent; no-op for negative
        /// degrees). The dispatch is identical for either variant.
        fn ensure_basis(&self, degree: i32) {
            if degree >= 0 {
                self.0.compute_basis(degree);
            }
        }

        fn product_target(&self, r_degree: i32, s_degree: i32) -> PyResult<i32> {
            non_negative_degree(r_degree)?;
            non_negative_degree(s_degree)?;
            let target = r_degree
                .checked_add(s_degree)
                .ok_or_else(|| PyValueError::new_err("product degree overflows i32"))?;
            self.ensure_basis(target);
            Ok(target)
        }

        fn checked_basis_index(&self, degree: i32, idx: usize) -> PyResult<()> {
            let dim = self.0.dimension(degree);
            if idx < dim {
                Ok(())
            } else {
                Err(PyIndexError::new_err(format!(
                    "index {idx} out of range for degree {degree} (dimension {dim})"
                )))
            }
        }
    }

    #[pymethods]
    impl SteenrodAlgebra {
        // --- §5.2 constructors ------------------------------------------------

        /// Construct a `SteenrodAlgebra` from a module-spec `dict` (the JSON the
        /// crate reads from a module file), the desired `AlgebraType`, and the
        /// `unstable` flag. Mirrors `::algebra::SteenrodAlgebra::from_json`,
        /// which reads `{"p": <int>, "algebra": [..]?, "profile": {..}?}`. If
        /// the spec's `algebra` list does not contain the requested type, the
        /// upstream falls back to the first listed type. Upstream returns an
        /// `anyhow::Error` for every failure (bad prime, malformed spec, parse
        /// error) without distinguishing them, so all `from_json` failures map
        /// to `RuntimeError`. (Type conversion of the Python value itself, in
        /// `py_to_json`, still raises `ValueError` before upstream is called.)
        #[staticmethod]
        #[pyo3(signature = (value, ty, unstable = false))]
        pub fn from_json(
            value: &Bound<'_, PyAny>,
            ty: AlgebraType,
            unstable: bool,
        ) -> PyResult<Self> {
            let json = py_to_json(value)?;
            ::algebra::SteenrodAlgebra::from_json(&json, ty.into(), unstable)
                .map(|a| SteenrodAlgebra(Arc::new(a)))
                .map_err(|e| {
                    use pyo3::exceptions::PyRuntimeError;
                    PyRuntimeError::new_err(e.to_string())
                })
        }

        /// Construct the Adem variant at prime `p`. Validates the prime ->
        /// `ValueError`.
        #[staticmethod]
        #[pyo3(signature = (p, unstable = false))]
        pub fn adem(p: u32, unstable: bool) -> PyResult<Self> {
            let p = valid_prime(p)?;
            Ok(SteenrodAlgebra(Arc::new(
                ::algebra::SteenrodAlgebra::AdemAlgebra(::algebra::AdemAlgebra::new(p, unstable)),
            )))
        }

        /// Construct the Milnor variant at prime `p`. Validates the prime ->
        /// `ValueError`.
        #[staticmethod]
        #[pyo3(signature = (p, unstable = false))]
        pub fn milnor(p: u32, unstable: bool) -> PyResult<Self> {
            let p = valid_prime(p)?;
            Ok(SteenrodAlgebra(Arc::new(
                ::algebra::SteenrodAlgebra::MilnorAlgebra(::algebra::MilnorAlgebra::new(
                    p, unstable,
                )),
            )))
        }

        /// Which variant this value is (`AlgebraType.ADEM`/`MILNOR`).
        pub fn algebra_type(&self) -> AlgebraType {
            match self.0.as_ref() {
                ::algebra::SteenrodAlgebra::AdemAlgebra(_) => AlgebraType::Adem,
                ::algebra::SteenrodAlgebra::MilnorAlgebra(_) => AlgebraType::Milnor,
            }
        }

        // --- Algebra trait surface --------------------------------------------

        /// The prime as a plain `int` (`ValidPrime` is never exposed).
        pub fn prime(&self) -> u32 {
            self.0.prime().as_u32()
        }

        pub fn compute_basis(&self, degree: i32) {
            self.ensure_basis(degree);
        }

        pub fn dimension(&self, degree: i32) -> usize {
            if degree < 0 {
                return 0;
            }
            self.ensure_basis(degree);
            self.0.dimension(degree)
        }

        pub fn basis_element_to_string(&self, degree: i32, idx: usize) -> PyResult<String> {
            self.0.try_basis_element_to_string(degree, idx).ok_or_else(|| {
                PyIndexError::new_err(format!(
                    "no basis element at degree {degree} index {idx}"
                ))
            })
        }

        /// Parse a basis element, returning `(degree, index)`. Raises
        /// `ValueError` if the string does not parse or names an element not in
        /// this algebra.
        ///
        /// The union dispatches straight to the active variant's now-total
        /// `basis_element_from_string`: a parseable but absent/inadmissible name
        /// (e.g. `"Sq0"`) returns `None` rather than panicking. We map that
        /// `None` to `ValueError`.
        pub fn basis_element_from_string(&self, elt: &str) -> PyResult<(i32, usize)> {
            self.0.basis_element_from_string(elt).ok_or_else(|| {
                PyValueError::new_err(format!(
                    "{elt} does not name a basis element of this algebra"
                ))
            })
        }

        pub fn element_to_string(
            &self,
            py: Python<'_>,
            degree: i32,
            element: &Bound<'_, PyAny>,
        ) -> PyResult<String> {
            non_negative_degree(degree)?;
            self.ensure_basis(degree);
            let element = crate::fp_py::extract_input_owned(py, element)?;
            checked_same_prime(element.prime().as_u32(), self.0.prime().as_u32())?;
            checked_equal_len(element.len(), self.0.dimension(degree))?;
            Ok(self.0.element_to_string(degree, element.as_slice()))
        }

        pub fn multiply_basis_elements(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            r_degree: i32,
            r_idx: usize,
            s_degree: i32,
            s_idx: usize,
        ) -> PyResult<()> {
            let p = self.0.prime().as_u32();
            // Reduce mod p before handing to upstream, which computes
            // `coeff * value` before reducing and would overflow for large
            // `coeff`. The algebra is over F_p, so this is equivalent.
            let coeff = coeff % p;
            let target = self.product_target(r_degree, s_degree)?;
            let dim = self.0.dimension(target);
            self.checked_basis_index(r_degree, r_idx)?;
            self.checked_basis_index(s_degree, s_idx)?;
            crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                checked_same_prime(res.prime().as_u32(), p)?;
                checked_result_len(res.as_slice().len(), dim)?;
                self.0
                    .multiply_basis_elements(res.copy(), coeff, r_degree, r_idx, s_degree, s_idx);
                Ok(())
            })
        }

        pub fn multiply_basis_element_by_element(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            r_degree: i32,
            r_idx: usize,
            s_degree: i32,
            s: &Bound<'_, PyAny>,
        ) -> PyResult<()> {
            let p = self.0.prime().as_u32();
            let coeff = coeff % p;
            let target = self.product_target(r_degree, s_degree)?;
            let dim = self.0.dimension(target);
            self.checked_basis_index(r_degree, r_idx)?;
            let s = crate::fp_py::extract_input_owned(py, s)?;
            checked_same_prime(s.prime().as_u32(), p)?;
            checked_equal_len(s.len(), self.0.dimension(s_degree))?;
            crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                checked_same_prime(res.prime().as_u32(), p)?;
                checked_result_len(res.as_slice().len(), dim)?;
                self.0.multiply_basis_element_by_element(
                    res.copy(),
                    coeff,
                    r_degree,
                    r_idx,
                    s_degree,
                    s.as_slice(),
                );
                Ok(())
            })
        }

        pub fn multiply_element_by_basis_element(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            r_degree: i32,
            r: &Bound<'_, PyAny>,
            s_degree: i32,
            s_idx: usize,
        ) -> PyResult<()> {
            let p = self.0.prime().as_u32();
            let coeff = coeff % p;
            let target = self.product_target(r_degree, s_degree)?;
            let dim = self.0.dimension(target);
            self.checked_basis_index(s_degree, s_idx)?;
            let r = crate::fp_py::extract_input_owned(py, r)?;
            checked_same_prime(r.prime().as_u32(), p)?;
            checked_equal_len(r.len(), self.0.dimension(r_degree))?;
            crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                checked_same_prime(res.prime().as_u32(), p)?;
                checked_result_len(res.as_slice().len(), dim)?;
                self.0.multiply_element_by_basis_element(
                    res.copy(),
                    coeff,
                    r_degree,
                    r.as_slice(),
                    s_degree,
                    s_idx,
                );
                Ok(())
            })
        }

        pub fn multiply_element_by_element(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            r_degree: i32,
            r: &Bound<'_, PyAny>,
            s_degree: i32,
            s: &Bound<'_, PyAny>,
        ) -> PyResult<()> {
            let p = self.0.prime().as_u32();
            let coeff = coeff % p;
            let target = self.product_target(r_degree, s_degree)?;
            let dim = self.0.dimension(target);
            let r = crate::fp_py::extract_input_owned(py, r)?;
            let s = crate::fp_py::extract_input_owned(py, s)?;
            checked_same_prime(r.prime().as_u32(), p)?;
            checked_same_prime(s.prime().as_u32(), p)?;
            checked_equal_len(r.len(), self.0.dimension(r_degree))?;
            checked_equal_len(s.len(), self.0.dimension(s_degree))?;
            crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                checked_same_prime(res.prime().as_u32(), p)?;
                checked_result_len(res.as_slice().len(), dim)?;
                self.0.multiply_element_by_element(
                    res.copy(),
                    coeff,
                    r_degree,
                    r.as_slice(),
                    s_degree,
                    s.as_slice(),
                );
                Ok(())
            })
        }

        pub fn default_filtration_one_products(&self) -> Vec<(String, i32, usize)> {
            self.0.default_filtration_one_products()
        }

        // --- GeneratedAlgebra trait surface -----------------------------------

        pub fn generators(&self, degree: i32) -> PyResult<Vec<usize>> {
            if degree < 0 {
                return Ok(Vec::new());
            }
            self.ensure_basis(degree);
            Ok(self.0.generators(degree))
        }

        pub fn generator_to_string(&self, degree: i32, idx: usize) -> PyResult<String> {
            non_negative_degree(degree)?;
            self.ensure_basis(degree);
            self.checked_basis_index(degree, idx)?;
            Ok(self.0.generator_to_string(degree, idx))
        }

        pub fn decompose_basis_element(
            &self,
            degree: i32,
            idx: usize,
        ) -> PyResult<Vec<(u32, (i32, usize), (i32, usize))>> {
            non_negative_degree(degree)?;
            self.ensure_basis(degree);
            self.checked_basis_index(degree, idx)?;
            // Decomposition is invalid for indecomposables. The union dispatches
            // to the active variant's (panicking) implementation. Both variants
            // panic on the unit and on algebra generators (Milnor underflows in
            // `decompose_basis_element_ppart`/`_qpart` on the degree-0 unit and
            // on `Q_0`; Adem indexes out of bounds / hits a panicking
            // `basis_element_to_index`). In every case the panicking elements
            // are exactly those reported by `generators`, so the same
            // generators-based guard applies uniformly to both variants.
            if degree == 0 || self.0.generators(degree).contains(&idx) {
                return Err(PyValueError::new_err(
                    "the unit and algebra generators are indecomposable",
                ));
            }
            Ok(self.0.decompose_basis_element(degree, idx))
        }

        pub fn generating_relations(
            &self,
            degree: i32,
        ) -> PyResult<Vec<Vec<(u32, (i32, usize), (i32, usize))>>> {
            if degree < 0 {
                return Ok(Vec::new());
            }
            self.ensure_basis(degree);
            Ok(self.0.generating_relations(degree))
        }

        // --- Bialgebra trait surface ------------------------------------------

        /// Compute a coproduct. The underlying assertions differ by variant, so
        /// we apply the same guards the concrete bindings use: Milnor only
        /// supports `p = 2`; generic Adem expects a degree divisible by
        /// `q = 2p - 2` (except degree 1), and `p = 2` Adem expects index 0.
        pub fn coproduct(
            &self,
            degree: i32,
            idx: usize,
        ) -> PyResult<Vec<(i32, usize, i32, usize)>> {
            non_negative_degree(degree)?;
            self.ensure_basis(degree);
            self.checked_basis_index(degree, idx)?;
            match self.0.as_ref() {
                ::algebra::SteenrodAlgebra::MilnorAlgebra(_) => {
                    if self.0.prime().as_u32() != 2 {
                        return Err(PyValueError::new_err(
                            "coproduct is only supported at p = 2",
                        ));
                    }
                }
                ::algebra::SteenrodAlgebra::AdemAlgebra(a) => {
                    if a.generic() {
                        if degree != 1 {
                            let q = 2 * self.0.prime().as_u32() - 2;
                            if (degree as u32) % q != 0 {
                                return Err(PyValueError::new_err(format!(
                                    "coproduct expects a degree divisible by {q}, got {degree}"
                                )));
                            }
                        }
                    } else if idx != 0 {
                        return Err(PyValueError::new_err(
                            "at p = 2 the coproduct expects index 0",
                        ));
                    }
                }
            }
            Ok(self.0.coproduct(degree, idx))
        }

        pub fn decompose(&self, degree: i32, idx: usize) -> PyResult<Vec<(i32, usize)>> {
            non_negative_degree(degree)?;
            self.ensure_basis(degree);
            self.checked_basis_index(degree, idx)?;
            Ok(self.0.decompose(degree, idx))
        }

        pub fn __repr__(&self) -> String {
            format!("{}", self.0)
        }
    }

    // =========================================================================
    // §5.3 modules over the Steenrod algebra
    //
    // All modules are taken over the `SteenrodAlgebra` union. A module holds its
    // algebra as `Arc<SteenrodAlgebra>` upstream; the bound algebra pyclass also
    // holds an `Arc<SteenrodAlgebra>`, so module constructors take a
    // `SteenrodAlgebra` pyclass and clone its `Arc` (`SteenrodAlgebra::arc`),
    // while a module's `algebra()` accessor re-wraps the `Arc` upstream hands
    // back (`SteenrodAlgebra::from_arc`) -- no algebra is ever deep-copied.
    //
    // The flattened `Module` method set is shared by every concrete module and
    // by `SteenrodModule` through the `&DynModule` helpers below, which apply a
    // uniform panic-guard before each degree-indexed read. The upstream
    // `OnceVec`/`BiVec` tables panic when indexed past the computed range, and
    // `act*`/`basis_element_to_string` assert on out-of-range op/module indices,
    // so we always `compute_basis` (idempotent) and range-check first.
    // =========================================================================

    /// Compute book-keeping so that degree-`degree` data of `m` (and the algebra
    /// operations landing there) can be queried. Idempotent; a no-op below the
    /// module's `min_degree`. Both the algebra and the module are advanced,
    /// because a `FreeModule`'s own `compute_basis` reads (but does not extend)
    /// the algebra's tables.
    fn module_ensure<A: Algebra>(m: &dyn Module<Algebra = A>, degree: i32) {
        if degree >= m.min_degree() {
            // op degrees landing in `degree` are at most `degree - min_degree`.
            m.algebra().compute_basis(degree - m.min_degree());
            m.compute_basis(degree);
        }
    }

    /// Dimension of `m` in `degree`, guarded so the `FreeModule` `OnceVec`
    /// length assertion can never fire across the boundary. Degrees below
    /// `min_degree` are empty.
    fn module_dimension<A: Algebra>(m: &dyn Module<Algebra = A>, degree: i32) -> usize {
        if degree < m.min_degree() {
            return 0;
        }
        module_ensure(m, degree);
        m.dimension(degree)
    }

    fn module_basis_element_to_string<A: Algebra>(
        m: &dyn Module<Algebra = A>,
        degree: i32,
        idx: usize,
    ) -> PyResult<String> {
        let dim = module_dimension(m, degree);
        if idx >= dim {
            return Err(PyIndexError::new_err(format!(
                "index {idx} out of range for degree {degree} (dimension {dim})"
            )));
        }
        Ok(m.basis_element_to_string(degree, idx))
    }

    fn module_element_to_string<A: Algebra>(
        m: &dyn Module<Algebra = A>,
        py: Python<'_>,
        degree: i32,
        element: &Bound<'_, PyAny>,
    ) -> PyResult<String> {
        let dim = module_dimension(m, degree);
        let element = crate::fp_py::extract_input_owned(py, element)?;
        checked_same_prime(element.prime().as_u32(), m.prime().as_u32())?;
        checked_equal_len(element.len(), dim)?;
        Ok(m.element_to_string(degree, element.as_slice()))
    }

    /// Validate the output degree of an action and ensure every degree it
    /// touches is computed. Returns `(prime, reduced_coeff, output_degree)`.
    fn action_target<A: Algebra>(
        m: &dyn Module<Algebra = A>,
        coeff: u32,
        op_degree: i32,
        mod_degree: i32,
    ) -> PyResult<i32> {
        non_negative_degree(op_degree)?;
        let _ = coeff;
        let output_degree = mod_degree
            .checked_add(op_degree)
            .ok_or_else(|| PyValueError::new_err("output degree overflows i32"))?;
        module_ensure(m, output_degree);
        // The op degree must be computed in the algebra to range-check op_index.
        m.algebra().compute_basis(op_degree);
        Ok(output_degree)
    }

    fn checked_op_index<A: Algebra>(
        m: &dyn Module<Algebra = A>,
        op_degree: i32,
        op_index: usize,
    ) -> PyResult<()> {
        let dim = m.algebra().dimension(op_degree);
        if op_index < dim {
            Ok(())
        } else {
            Err(PyIndexError::new_err(format!(
                "operation index {op_index} out of range for degree {op_degree} (algebra \
                 dimension {dim})"
            )))
        }
    }

    fn checked_mod_index<A: Algebra>(
        m: &dyn Module<Algebra = A>,
        mod_degree: i32,
        mod_index: usize,
    ) -> PyResult<()> {
        let dim = module_dimension(m, mod_degree);
        if mod_index < dim {
            Ok(())
        } else {
            Err(PyIndexError::new_err(format!(
                "module index {mod_index} out of range for degree {mod_degree} (dimension {dim})"
            )))
        }
    }

    /// Translate the typed [`ActError`] from `Module::try_act`/`try_act_on_basis`
    /// into the matching Python exception: an out-of-range degree/index is an
    /// `IndexError`, an over-long input vector is a `ValueError`.
    fn act_error_to_py(e: ActError) -> PyErr {
        match e {
            ActError::IndexOutOfRange(m) => PyIndexError::new_err(m),
            ActError::InvalidInput(m) => PyValueError::new_err(m),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn module_act_on_basis<A: Algebra>(
        m: &dyn Module<Algebra = A>,
        py: Python<'_>,
        result: &Bound<'_, PyAny>,
        coeff: u32,
        op_degree: i32,
        op_index: usize,
        mod_degree: i32,
        mod_index: usize,
    ) -> PyResult<()> {
        let p = m.prime().as_u32();
        let coeff = coeff % p;
        // `action_target` validates the op degree and computes the output degree
        // (and hence the required `result` length); `try_act_on_basis` performs
        // the op/module index range checks that previously needed `checked_*`.
        let output_degree = action_target(m, coeff, op_degree, mod_degree)?;
        let out_dim = module_dimension(m, output_degree);
        crate::fp_py::with_target_slice_mut(py, result, |mut res| {
            checked_same_prime(res.prime().as_u32(), p)?;
            checked_result_len(res.as_slice().len(), out_dim)?;
            m.try_act_on_basis(
                res.copy(),
                coeff,
                op_degree,
                op_index,
                mod_degree,
                mod_index,
            )
            .map_err(act_error_to_py)?;
            Ok(())
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn module_act<A: Algebra>(
        m: &dyn Module<Algebra = A>,
        py: Python<'_>,
        result: &Bound<'_, PyAny>,
        coeff: u32,
        op_degree: i32,
        op_index: usize,
        input_degree: i32,
        input: &Bound<'_, PyAny>,
    ) -> PyResult<()> {
        let p = m.prime().as_u32();
        let coeff = coeff % p;
        // `action_target` validates the op degree and computes the output degree
        // (hence the required `result` length); `try_act` performs the op-index
        // range check and the `input.len() <= dimension(input_degree)` check that
        // previously needed `checked_op_index` and the manual length guard.
        let output_degree = action_target(m, coeff, op_degree, input_degree)?;
        let out_dim = module_dimension(m, output_degree);
        // Own the input before taking the mutable borrow of `result`.
        let input = crate::fp_py::extract_input_owned(py, input)?;
        checked_same_prime(input.prime().as_u32(), p)?;
        crate::fp_py::with_target_slice_mut(py, result, |mut res| {
            checked_same_prime(res.prime().as_u32(), p)?;
            checked_result_len(res.as_slice().len(), out_dim)?;
            m.try_act(
                res.copy(),
                coeff,
                op_degree,
                op_index,
                input_degree,
                input.as_slice(),
            )
            .map_err(act_error_to_py)?;
            Ok(())
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn module_act_by_element<A: Algebra>(
        m: &dyn Module<Algebra = A>,
        py: Python<'_>,
        result: &Bound<'_, PyAny>,
        coeff: u32,
        op_degree: i32,
        op: &Bound<'_, PyAny>,
        input_degree: i32,
        input: &Bound<'_, PyAny>,
    ) -> PyResult<()> {
        let p = m.prime().as_u32();
        let coeff = coeff % p;
        let output_degree = action_target(m, coeff, op_degree, input_degree)?;
        let in_dim = module_dimension(m, input_degree);
        let out_dim = module_dimension(m, output_degree);
        let op_dim = m.algebra().dimension(op_degree);
        // Own both inputs before the mutable borrow of `result`.
        let op = crate::fp_py::extract_input_owned(py, op)?;
        let input = crate::fp_py::extract_input_owned(py, input)?;
        checked_same_prime(op.prime().as_u32(), p)?;
        checked_same_prime(input.prime().as_u32(), p)?;
        // Upstream `act_by_element` asserts both lengths exactly.
        checked_equal_len(op.len(), op_dim)?;
        checked_equal_len(input.len(), in_dim)?;
        crate::fp_py::with_target_slice_mut(py, result, |mut res| {
            checked_same_prime(res.prime().as_u32(), p)?;
            checked_result_len(res.as_slice().len(), out_dim)?;
            m.act_by_element(
                res.copy(),
                coeff,
                op_degree,
                op.as_slice(),
                input_degree,
                input.as_slice(),
            );
            Ok(())
        })
    }

    fn module_total_dimension<A: Algebra>(m: &dyn Module<Algebra = A>) -> PyResult<usize> {
        match m.max_degree() {
            Some(max) => {
                module_ensure(m, max);
                Ok(m.total_dimension())
            }
            None => Err(PyValueError::new_err(
                "total_dimension requires the module to be bounded above",
            )),
        }
    }

    /// The boxed (`Arc`'d) dynamic module accepted downstream by chain complexes
    /// and resolutions. Wraps `::algebra::module::SteenrodModule`, i.e.
    /// `Arc<dyn Module<Algebra = SteenrodAlgebra>>`. This is the type
    /// `into_steenrod_module()` produces; every flattened `Module` method
    /// dispatches dynamically to the underlying concrete module.
    #[pyclass(name = "SteenrodModule")]
    pub struct SteenrodModule(RsSteenrodModule);

    #[pymethods]
    impl SteenrodModule {
        pub fn algebra(&self) -> SteenrodAlgebra {
            SteenrodAlgebra::from_arc(self.0.algebra())
        }

        pub fn min_degree(&self) -> i32 {
            self.0.min_degree()
        }

        pub fn max_computed_degree(&self) -> i32 {
            self.0.max_computed_degree()
        }

        pub fn max_degree(&self) -> Option<i32> {
            self.0.max_degree()
        }

        /// The prime as a plain `int` (`ValidPrime` is never exposed).
        pub fn prime(&self) -> u32 {
            self.0.prime().as_u32()
        }

        pub fn compute_basis(&self, degree: i32) {
            module_ensure(&*self.0, degree);
        }

        pub fn dimension(&self, degree: i32) -> usize {
            module_dimension(&*self.0, degree)
        }

        pub fn total_dimension(&self) -> PyResult<usize> {
            module_total_dimension(&*self.0)
        }

        pub fn is_unit(&self) -> bool {
            self.0.is_unit()
        }

        pub fn basis_element_to_string(&self, degree: i32, idx: usize) -> PyResult<String> {
            module_basis_element_to_string(&*self.0, degree, idx)
        }

        pub fn element_to_string(
            &self,
            py: Python<'_>,
            degree: i32,
            element: &Bound<'_, PyAny>,
        ) -> PyResult<String> {
            module_element_to_string(&*self.0, py, degree, element)
        }

        #[allow(clippy::too_many_arguments)]
        pub fn act_on_basis(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            op_degree: i32,
            op_index: usize,
            mod_degree: i32,
            mod_index: usize,
        ) -> PyResult<()> {
            module_act_on_basis(
                &*self.0, py, result, coeff, op_degree, op_index, mod_degree, mod_index,
            )
        }

        #[allow(clippy::too_many_arguments)]
        pub fn act(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            op_degree: i32,
            op_index: usize,
            input_degree: i32,
            input: &Bound<'_, PyAny>,
        ) -> PyResult<()> {
            module_act(
                &*self.0,
                py,
                result,
                coeff,
                op_degree,
                op_index,
                input_degree,
                input,
            )
        }

        #[allow(clippy::too_many_arguments)]
        pub fn act_by_element(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            op_degree: i32,
            op: &Bound<'_, PyAny>,
            input_degree: i32,
            input: &Bound<'_, PyAny>,
        ) -> PyResult<()> {
            module_act_by_element(
                &*self.0,
                py,
                result,
                coeff,
                op_degree,
                op,
                input_degree,
                input,
            )
        }

        pub fn __repr__(&self) -> String {
            format!("SteenrodModule({})", self.0)
        }
    }

    /// A pair `(operation, generator)` indexing a basis element of a
    /// `FreeModule`: the basis element is `operation * generator`. Mirrors
    /// upstream `OperationGeneratorPair`'s four integer fields.
    #[pyclass(name = "OperationGeneratorPair", skip_from_py_object)]
    #[derive(Clone)]
    pub struct OperationGeneratorPair(RsOperationGeneratorPair);

    #[pymethods]
    impl OperationGeneratorPair {
        #[getter]
        pub fn operation_degree(&self) -> i32 {
            self.0.operation_degree
        }

        #[getter]
        pub fn operation_index(&self) -> usize {
            self.0.operation_index
        }

        #[getter]
        pub fn generator_degree(&self) -> i32 {
            self.0.generator_degree
        }

        #[getter]
        pub fn generator_index(&self) -> usize {
            self.0.generator_index
        }

        pub fn __repr__(&self) -> String {
            format!(
                "OperationGeneratorPair(operation_degree={}, operation_index={}, \
                 generator_degree={}, generator_index={})",
                self.0.operation_degree,
                self.0.operation_index,
                self.0.generator_degree,
                self.0.generator_index
            )
        }
    }

    /// A finite-dimensional module over the Steenrod algebra. The graded
    /// dimensions are given as a `list[int]` starting at `min_degree`.
    #[pyclass(name = "FDModule")]
    pub struct FDModule(FDModuleInner);

    impl FDModule {
        fn as_dyn(&self) -> &DynModule {
            &self.0
        }
    }

    #[pymethods]
    impl FDModule {
        /// Build a finite-dimensional module with `graded_dims[i]` generators in
        /// degree `min_degree + i`. All actions are initialised to zero; use
        /// `add_generator`/`set_action`/`extend_actions` to populate them, or
        /// build from JSON via `steenrod_module_from_json`.
        #[new]
        #[pyo3(signature = (algebra, name, graded_dims, min_degree = 0))]
        pub fn new(
            algebra: PyRef<'_, SteenrodAlgebra>,
            name: String,
            graded_dims: Vec<usize>,
            min_degree: i32,
        ) -> Self {
            let graded_dimension = ::bivec::BiVec::from_vec(min_degree, graded_dims);
            FDModule(FDModuleInner::new(algebra.arc(), name, graded_dimension))
        }

        // --- flattened Module method set --------------------------------------

        pub fn algebra(&self) -> SteenrodAlgebra {
            SteenrodAlgebra::from_arc(self.0.algebra())
        }

        pub fn min_degree(&self) -> i32 {
            self.0.min_degree()
        }

        pub fn max_computed_degree(&self) -> i32 {
            self.0.max_computed_degree()
        }

        pub fn max_degree(&self) -> Option<i32> {
            self.0.max_degree()
        }

        pub fn prime(&self) -> u32 {
            self.0.prime().as_u32()
        }

        pub fn compute_basis(&self, degree: i32) {
            module_ensure(self.as_dyn(), degree);
        }

        pub fn dimension(&self, degree: i32) -> usize {
            module_dimension(self.as_dyn(), degree)
        }

        pub fn total_dimension(&self) -> PyResult<usize> {
            module_total_dimension(self.as_dyn())
        }

        pub fn is_unit(&self) -> bool {
            self.0.is_unit()
        }

        pub fn basis_element_to_string(&self, degree: i32, idx: usize) -> PyResult<String> {
            module_basis_element_to_string(self.as_dyn(), degree, idx)
        }

        pub fn element_to_string(
            &self,
            py: Python<'_>,
            degree: i32,
            element: &Bound<'_, PyAny>,
        ) -> PyResult<String> {
            module_element_to_string(self.as_dyn(), py, degree, element)
        }

        #[allow(clippy::too_many_arguments)]
        pub fn act_on_basis(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            op_degree: i32,
            op_index: usize,
            mod_degree: i32,
            mod_index: usize,
        ) -> PyResult<()> {
            module_act_on_basis(
                self.as_dyn(),
                py,
                result,
                coeff,
                op_degree,
                op_index,
                mod_degree,
                mod_index,
            )
        }

        #[allow(clippy::too_many_arguments)]
        pub fn act(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            op_degree: i32,
            op_index: usize,
            input_degree: i32,
            input: &Bound<'_, PyAny>,
        ) -> PyResult<()> {
            module_act(
                self.as_dyn(),
                py,
                result,
                coeff,
                op_degree,
                op_index,
                input_degree,
                input,
            )
        }

        #[allow(clippy::too_many_arguments)]
        pub fn act_by_element(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            op_degree: i32,
            op: &Bound<'_, PyAny>,
            input_degree: i32,
            input: &Bound<'_, PyAny>,
        ) -> PyResult<()> {
            module_act_by_element(
                self.as_dyn(),
                py,
                result,
                coeff,
                op_degree,
                op,
                input_degree,
                input,
            )
        }

        // --- FDModule-specific (thin) -----------------------------------------

        /// Rename a basis element. Raises `IndexError` if `(degree, idx)` is not
        /// a basis element (upstream indexes `gen_names` and would panic).
        pub fn set_basis_element_name(
            &mut self,
            degree: i32,
            idx: usize,
            name: String,
        ) -> PyResult<()> {
            checked_mod_index(&self.0, degree, idx)?;
            self.0.set_basis_element_name(degree, idx, name);
            Ok(())
        }

        /// Append a new generator in `degree`, returning its index.
        pub fn add_generator(&mut self, degree: i32, name: String) {
            self.0.add_generator(degree, name);
        }

        /// Set the action `op * x = output`, where `op = (op_degree, op_index)`
        /// and `x = (input_degree, input_index)`. `output` is a coefficient
        /// vector in degree `input_degree + op_degree`. Raises `IndexError`/
        /// `ValueError` rather than letting an upstream assertion/`copy_from_slice`
        /// length-mismatch panic.
        #[allow(clippy::too_many_arguments)]
        pub fn set_action(
            &mut self,
            op_degree: i32,
            op_index: usize,
            input_degree: i32,
            input_index: usize,
            output: Vec<u32>,
        ) -> PyResult<()> {
            non_negative_degree(op_degree)?;
            self.0.algebra().compute_basis(op_degree);
            checked_op_index(&self.0, op_degree, op_index)?;
            checked_mod_index(&self.0, input_degree, input_index)?;
            let output_degree = input_degree
                .checked_add(op_degree)
                .ok_or_else(|| PyValueError::new_err("output degree overflows i32"))?;
            // Upstream indexes `actions[input_degree][output_degree]`, whose
            // `BiVec::Index` panics when `output_degree` is outside the module's
            // graded range (e.g. above `max_degree`). An empty `output` with an
            // empty (out-of-range) `output_degree` passes the length check but
            // would then panic, so reject it the same way the `action` getter
            // does: an empty output degree is a `ValueError`.
            let out_dim = module_dimension(&self.0, output_degree);
            if out_dim == 0 {
                return Err(PyValueError::new_err(format!(
                    "output degree {output_degree} is empty"
                )));
            }
            checked_equal_len(output.len(), out_dim)?;
            let p = self.0.prime().as_u32();
            for v in &output {
                if *v >= p {
                    return Err(PyValueError::new_err(format!(
                        "coefficient {v} is not reduced mod {p}"
                    )));
                }
            }
            self.0
                .set_action(op_degree, op_index, input_degree, input_index, &output);
            Ok(())
        }

        /// The stored action `op * x` as a coefficient vector. Raises rather
        /// than panicking for out-of-range indices or an empty output degree.
        pub fn action(
            &self,
            op_degree: i32,
            op_index: usize,
            input_degree: i32,
            input_index: usize,
        ) -> PyResult<Vec<u32>> {
            non_negative_degree(op_degree)?;
            self.0.algebra().compute_basis(op_degree);
            checked_op_index(&self.0, op_degree, op_index)?;
            checked_mod_index(&self.0, input_degree, input_index)?;
            let output_degree = input_degree
                .checked_add(op_degree)
                .ok_or_else(|| PyValueError::new_err("output degree overflows i32"))?;
            if module_dimension(&self.0, output_degree) == 0 {
                return Err(PyValueError::new_err(format!(
                    "output degree {output_degree} is empty"
                )));
            }
            let vec = self
                .0
                .action(op_degree, op_index, input_degree, input_index);
            Ok(vec.iter().collect())
        }

        /// Fill in actions of decomposable operations in the given bidegree from
        /// the actions of the algebra generators. Raises if `output_deg <=
        /// input_deg` (upstream asserts).
        pub fn extend_actions(&mut self, input_degree: i32, output_degree: i32) -> PyResult<()> {
            if output_degree <= input_degree {
                return Err(PyValueError::new_err(
                    "output_degree must be strictly greater than input_degree",
                ));
            }
            self.0.algebra().compute_basis(output_degree - input_degree);
            self.0.extend_actions(input_degree, output_degree);
            Ok(())
        }

        /// Check that the stored actions satisfy the algebra's relations in the
        /// given bidegree. Raises `ValueError` (with the failing relation) if a
        /// relation fails, or if `output_deg <= input_deg`.
        pub fn check_validity(&self, input_degree: i32, output_degree: i32) -> PyResult<()> {
            if output_degree <= input_degree {
                return Err(PyValueError::new_err(
                    "output_degree must be strictly greater than input_degree",
                ));
            }
            self.0.algebra().compute_basis(output_degree - input_degree);
            self.0
                .check_validity(input_degree, output_degree)
                .map_err(|e| PyValueError::new_err(e.to_string()))
        }

        /// Look up a basis element by its name, returning `(degree, index)` or
        /// `None`.
        pub fn string_to_basis_element(&self, string: &str) -> Option<(i32, usize)> {
            self.0.string_to_basis_element(string)
        }

        /// Box this module into a `SteenrodModule` for downstream use.
        ///
        /// This returns an independent snapshot: the `FDModule` is deep-cloned
        /// into the boxed `SteenrodModule`, so later `set_action`/`add_generator`
        /// calls on this `FDModule` do *not* propagate to the returned module.
        /// (`FreeModule.into_steenrod_module`, by contrast, shares state via an
        /// `Arc`.)
        pub fn into_steenrod_module(&self) -> SteenrodModule {
            SteenrodModule(steenrod_module::erase(self.0.clone()))
        }

        pub fn __repr__(&self) -> String {
            format!("FDModule({})", self.0)
        }
    }

    /// A free module over the Steenrod algebra, determined by its list of
    /// generators (added in increasing degree).
    #[pyclass(name = "FreeModule")]
    pub struct FreeModule(Arc<FreeModuleInner>);

    impl FreeModule {
        fn as_dyn(&self) -> &DynModule {
            &*self.0
        }

        /// The number of generators in `degree`, returning 0 (never panicking)
        /// for degrees outside the populated `num_gens` range. Upstream
        /// `number_of_gens_in_degree` only guards `degree < min_degree` and then
        /// indexes `num_gens[degree]`, whose `OnceBiVec::Index` asserts
        /// `degree < num_gens.len()`. `num_gens` is extended only by
        /// `add_generators`/`extend_by_zero` (not by `compute_basis`), and its
        /// populated upper bound is exactly `max_computed_degree()` (defined
        /// upstream as `num_gens.max_degree() == num_gens.len() - 1`). So a
        /// degree `>= min_degree` but `> max_computed_degree()` has no
        /// generators added yet and must read as 0 rather than panic.
        fn num_gens_safe(&self, degree: i32) -> usize {
            if degree < self.0.min_degree() || degree > self.0.max_computed_degree() {
                return 0;
            }
            self.0.number_of_gens_in_degree(degree)
        }
    }

    #[pymethods]
    impl FreeModule {
        #[new]
        #[pyo3(signature = (algebra, name, min_degree = 0))]
        pub fn new(algebra: PyRef<'_, SteenrodAlgebra>, name: String, min_degree: i32) -> Self {
            FreeModule(Arc::new(FreeModuleInner::new(
                algebra.arc(),
                name,
                min_degree,
            )))
        }

        // --- flattened Module method set --------------------------------------

        pub fn algebra(&self) -> SteenrodAlgebra {
            SteenrodAlgebra::from_arc(self.0.algebra())
        }

        pub fn min_degree(&self) -> i32 {
            self.0.min_degree()
        }

        pub fn max_computed_degree(&self) -> i32 {
            self.0.max_computed_degree()
        }

        pub fn max_degree(&self) -> Option<i32> {
            self.0.max_degree()
        }

        pub fn prime(&self) -> u32 {
            self.0.prime().as_u32()
        }

        pub fn compute_basis(&self, degree: i32) {
            module_ensure(self.as_dyn(), degree);
        }

        pub fn dimension(&self, degree: i32) -> usize {
            module_dimension(self.as_dyn(), degree)
        }

        pub fn total_dimension(&self) -> PyResult<usize> {
            module_total_dimension(self.as_dyn())
        }

        pub fn is_unit(&self) -> bool {
            self.0.is_unit()
        }

        pub fn basis_element_to_string(&self, degree: i32, idx: usize) -> PyResult<String> {
            module_basis_element_to_string(self.as_dyn(), degree, idx)
        }

        pub fn element_to_string(
            &self,
            py: Python<'_>,
            degree: i32,
            element: &Bound<'_, PyAny>,
        ) -> PyResult<String> {
            module_element_to_string(self.as_dyn(), py, degree, element)
        }

        #[allow(clippy::too_many_arguments)]
        pub fn act_on_basis(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            op_degree: i32,
            op_index: usize,
            mod_degree: i32,
            mod_index: usize,
        ) -> PyResult<()> {
            module_act_on_basis(
                self.as_dyn(),
                py,
                result,
                coeff,
                op_degree,
                op_index,
                mod_degree,
                mod_index,
            )
        }

        #[allow(clippy::too_many_arguments)]
        pub fn act(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            op_degree: i32,
            op_index: usize,
            input_degree: i32,
            input: &Bound<'_, PyAny>,
        ) -> PyResult<()> {
            module_act(
                self.as_dyn(),
                py,
                result,
                coeff,
                op_degree,
                op_index,
                input_degree,
                input,
            )
        }

        #[allow(clippy::too_many_arguments)]
        pub fn act_by_element(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            op_degree: i32,
            op: &Bound<'_, PyAny>,
            input_degree: i32,
            input: &Bound<'_, PyAny>,
        ) -> PyResult<()> {
            module_act_by_element(
                self.as_dyn(),
                py,
                result,
                coeff,
                op_degree,
                op,
                input_degree,
                input,
            )
        }

        // --- FreeModule-specific (thin) ---------------------------------------

        /// Add `num_gens` generators in `degree`, optionally naming them.
        /// Generators must be added at exactly the next consecutive degree:
        /// upstream `add_generators` does `num_gens.push_checked(.., degree)`,
        /// whose `OnceBiVec::push_checked` asserts the appended index equals
        /// `degree`, i.e. `degree == num_gens.len()`. `num_gens.len()` is
        /// `max_computed_degree() + 1` (upstream `max_computed_degree` returns
        /// `num_gens.max_degree() == num_gens.len() - 1`). Raises `ValueError`
        /// for `degree < min_degree`, for a non-consecutive degree (a gap must
        /// be filled with `extend_by_zero` first), or for re-adding a degree.
        #[pyo3(signature = (degree, num_gens, names = None))]
        pub fn add_generators(
            &self,
            degree: i32,
            num_gens: usize,
            names: Option<Vec<String>>,
        ) -> PyResult<()> {
            if degree < self.0.min_degree() {
                return Err(PyValueError::new_err(format!(
                    "degree {degree} is below the module's min_degree {}",
                    self.0.min_degree()
                )));
            }
            let next_expected = self.0.max_computed_degree() + 1;
            if degree != next_expected {
                return Err(PyValueError::new_err(format!(
                    "generators must be added at the next consecutive degree \
                     {next_expected}, got {degree}; use extend_by_zero to fill gaps"
                )));
            }
            if let Some(names) = &names {
                checked_equal_len(names.len(), num_gens)?;
            }
            // `add_generators` reads the algebra/opgen tables up to the current
            // computed degree, so make sure they are populated through `degree`.
            module_ensure(self.as_dyn(), degree);
            self.0.add_generators(degree, num_gens, names);
            Ok(())
        }

        /// The number of generators in `degree`. Returns 0 for degrees that
        /// have not had generators added yet (including a fresh module or any
        /// degree above the highest generator degree), rather than panicking on
        /// the upstream `num_gens[degree]` index assertion.
        pub fn number_of_gens_in_degree(&self, degree: i32) -> usize {
            self.num_gens_safe(degree)
        }

        /// The generator names up to the maximum computed generator degree, as a
        /// list (indexed from `min_degree`) of lists.
        pub fn gen_names(&self) -> Vec<Vec<String>> {
            self.0.gen_names().iter().map(|(_, v)| v.clone()).collect()
        }

        /// The offset in `degree` of the first basis element coming from the
        /// generator `(gen_degree, gen_index)`.
        pub fn generator_offset(
            &self,
            degree: i32,
            gen_degree: i32,
            gen_index: usize,
        ) -> PyResult<usize> {
            if gen_degree < self.0.min_degree() {
                return Err(PyValueError::new_err(format!(
                    "generator degree {gen_degree} is below min_degree {}",
                    self.0.min_degree()
                )));
            }
            if gen_index >= self.num_gens_safe(gen_degree) {
                return Err(PyIndexError::new_err(format!(
                    "generator index {gen_index} out of range in degree {gen_degree}"
                )));
            }
            module_ensure(self.as_dyn(), degree);
            Ok(self.0.generator_offset(degree, gen_degree, gen_index))
        }

        /// The offset in `degree` of the first basis element coming from the
        /// generator with internal index `internal_gen_idx`.
        pub fn internal_generator_offset(
            &self,
            degree: i32,
            internal_gen_idx: usize,
        ) -> PyResult<usize> {
            module_ensure(self.as_dyn(), degree);
            let dim = module_dimension(self.as_dyn(), degree);
            // `generator_to_index[degree]` has one entry per generator with a
            // basis element in `degree`; guard against an out-of-range internal
            // index to avoid the upstream `OnceVec` panic.
            if degree < self.0.min_degree() {
                return Err(PyIndexError::new_err(format!(
                    "degree {degree} is below min_degree {}",
                    self.0.min_degree()
                )));
            }
            let _ = dim;
            let count = self.0.iter_gens(degree).count();
            if internal_gen_idx >= count {
                return Err(PyIndexError::new_err(format!(
                    "internal generator index {internal_gen_idx} out of range (only {count} \
                     generators up to degree {degree})"
                )));
            }
            Ok(self.0.internal_generator_offset(degree, internal_gen_idx))
        }

        /// The basis index of `op * gen`, where `op = (op_degree, op_index)` and
        /// `gen = (gen_degree, gen_index)`.
        #[allow(clippy::too_many_arguments)]
        pub fn operation_generator_to_index(
            &self,
            op_degree: i32,
            op_index: usize,
            gen_degree: i32,
            gen_index: usize,
        ) -> PyResult<usize> {
            non_negative_degree(op_degree)?;
            if gen_degree < self.0.min_degree() {
                return Err(PyValueError::new_err(format!(
                    "generator degree {gen_degree} is below min_degree {}",
                    self.0.min_degree()
                )));
            }
            if gen_index >= self.num_gens_safe(gen_degree) {
                return Err(PyIndexError::new_err(format!(
                    "generator index {gen_index} out of range in degree {gen_degree}"
                )));
            }
            let output_degree = op_degree
                .checked_add(gen_degree)
                .ok_or_else(|| PyValueError::new_err("degree overflows i32"))?;
            module_ensure(self.as_dyn(), output_degree);
            self.0.algebra().compute_basis(op_degree);
            checked_op_index(self.as_dyn(), op_degree, op_index)?;
            Ok(self
                .0
                .operation_generator_to_index(op_degree, op_index, gen_degree, gen_index))
        }

        /// The `(operation, generator)` pair for the basis element at
        /// `(degree, index)`.
        pub fn index_to_op_gen(
            &self,
            degree: i32,
            index: usize,
        ) -> PyResult<OperationGeneratorPair> {
            checked_mod_index(self.as_dyn(), degree, index)?;
            Ok(OperationGeneratorPair(
                self.0.index_to_op_gen(degree, index).clone(),
            ))
        }

        /// Add zero generators in every degree up to (and including) `degree`.
        pub fn extend_by_zero(&self, degree: i32) {
            self.0.extend_by_zero(degree);
        }

        /// Iterate the `(degree, index)` of every generator up to `degree`.
        /// Returns an empty list for `degree < min_degree`: upstream computes
        /// `take((degree - min_degree + 1) as usize)`, which for a negative
        /// difference wraps to a huge `usize` and would otherwise yield *all*
        /// generators.
        pub fn iter_gens(&self, degree: i32) -> Vec<(i32, usize)> {
            if degree < self.0.min_degree() {
                return Vec::new();
            }
            self.0.iter_gens(degree).collect()
        }

        /// Box this module into a `SteenrodModule` for downstream use.
        pub fn into_steenrod_module(&self) -> SteenrodModule {
            // `Arc<FreeModule>` unsizes directly to `Arc<dyn Module>`.
            SteenrodModule(Arc::clone(&self.0) as RsSteenrodModule)
        }

        pub fn __repr__(&self) -> String {
            format!("FreeModule({})", self.0)
        }
    }

    // =========================================================================
    // Derived / standalone modules (§5.3)
    //
    // Each holds its concrete module in an `Arc`, both so the flattened `Module`
    // method set can dispatch through `&DynModule` (the shared guard helpers)
    // and so `into_steenrod_module()` can unsize the `Arc` directly into a
    // `SteenrodModule` (the `FreeModule` Arc-unsizing pattern), sharing state
    // rather than deep-cloning. The two derived modules (`TensorModule`,
    // `SuspensionModule`) accept their factor(s) as the already-boxed
    // `SteenrodModule` trait object; callers box concrete modules first with
    // `.into_steenrod_module()`.
    // =========================================================================

    /// The tensor product `left (x) right` of two modules over the Steenrod
    /// algebra. The factors are passed as `SteenrodModule`s (box concrete
    /// modules with `.into_steenrod_module()` first).
    #[pyclass(name = "TensorModule")]
    pub struct TensorModule(Arc<TensorModuleInner>);

    impl TensorModule {
        fn as_dyn(&self) -> &DynModule {
            &*self.0
        }
    }

    #[pymethods]
    impl TensorModule {
        /// Build `left (x) right`. Both factors must be built from the *same*
        /// `SteenrodAlgebra` Python object: the same-algebra check uses
        /// `Arc::ptr_eq` (there is no cheap structural equality on
        /// `SteenrodAlgebra`, so we cannot accept distinct-but-equal algebras),
        /// meaning a distinct-but-equal algebra object is rejected with
        /// `ValueError`. The requirement exists because upstream takes the
        /// coproduct from `left`'s algebra and applies it to `right`'s basis,
        /// so a prime mismatch would panic on a length/prime mismatch inside
        /// the `FpVector` action and an algebra mismatch would silently compute
        /// the wrong answer. We therefore reject both up front with
        /// `ValueError` (upstream `new` does no such check).
        #[new]
        pub fn new(
            left: PyRef<'_, SteenrodModule>,
            right: PyRef<'_, SteenrodModule>,
        ) -> PyResult<Self> {
            let left_alg = left.0.algebra();
            let right_alg = right.0.algebra();
            checked_same_prime(left_alg.prime().as_u32(), right_alg.prime().as_u32())?;
            if !Arc::ptr_eq(&left_alg, &right_alg) {
                return Err(PyValueError::new_err(
                    "tensor factors must be built over the same algebra",
                ));
            }
            Ok(TensorModule(Arc::new(TensorModuleInner::new(
                Arc::new(Arc::clone(&left.0)),
                Arc::new(Arc::clone(&right.0)),
            ))))
        }

        // --- flattened Module method set --------------------------------------

        pub fn algebra(&self) -> SteenrodAlgebra {
            SteenrodAlgebra::from_arc(self.0.algebra())
        }

        pub fn min_degree(&self) -> i32 {
            self.0.min_degree()
        }

        pub fn max_computed_degree(&self) -> i32 {
            self.0.max_computed_degree()
        }

        pub fn max_degree(&self) -> Option<i32> {
            self.0.max_degree()
        }

        pub fn prime(&self) -> u32 {
            self.0.prime().as_u32()
        }

        pub fn compute_basis(&self, degree: i32) {
            module_ensure(self.as_dyn(), degree);
        }

        pub fn dimension(&self, degree: i32) -> usize {
            module_dimension(self.as_dyn(), degree)
        }

        pub fn total_dimension(&self) -> PyResult<usize> {
            module_total_dimension(self.as_dyn())
        }

        pub fn is_unit(&self) -> bool {
            self.0.is_unit()
        }

        pub fn basis_element_to_string(&self, degree: i32, idx: usize) -> PyResult<String> {
            module_basis_element_to_string(self.as_dyn(), degree, idx)
        }

        pub fn element_to_string(
            &self,
            py: Python<'_>,
            degree: i32,
            element: &Bound<'_, PyAny>,
        ) -> PyResult<String> {
            module_element_to_string(self.as_dyn(), py, degree, element)
        }

        #[allow(clippy::too_many_arguments)]
        pub fn act_on_basis(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            op_degree: i32,
            op_index: usize,
            mod_degree: i32,
            mod_index: usize,
        ) -> PyResult<()> {
            module_act_on_basis(
                self.as_dyn(),
                py,
                result,
                coeff,
                op_degree,
                op_index,
                mod_degree,
                mod_index,
            )
        }

        #[allow(clippy::too_many_arguments)]
        pub fn act(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            op_degree: i32,
            op_index: usize,
            input_degree: i32,
            input: &Bound<'_, PyAny>,
        ) -> PyResult<()> {
            module_act(
                self.as_dyn(),
                py,
                result,
                coeff,
                op_degree,
                op_index,
                input_degree,
                input,
            )
        }

        #[allow(clippy::too_many_arguments)]
        pub fn act_by_element(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            op_degree: i32,
            op: &Bound<'_, PyAny>,
            input_degree: i32,
            input: &Bound<'_, PyAny>,
        ) -> PyResult<()> {
            module_act_by_element(
                self.as_dyn(),
                py,
                result,
                coeff,
                op_degree,
                op,
                input_degree,
                input,
            )
        }

        // --- TensorModule-specific (thin) -------------------------------------

        /// The degree of the left tensor factor of basis element `index` in
        /// total degree `degree`. Raises `IndexError` rather than panicking on
        /// an out-of-range basis index (upstream indexes the block structure).
        pub fn seek_module_num(&self, degree: i32, index: usize) -> PyResult<i32> {
            checked_mod_index(self.as_dyn(), degree, index)?;
            Ok(self.0.seek_module_num(degree, index))
        }

        /// The offset, within total degree `degree`, of the block of basis
        /// elements whose left factor lives in `left_degree`. Raises
        /// `IndexError`/`ValueError` rather than letting the block structure
        /// index out of range.
        pub fn offset(&self, degree: i32, left_degree: i32) -> PyResult<usize> {
            // The block structure is indexed by total degree; ensure it is
            // computed and `degree` is a populated degree of the tensor module.
            module_ensure(self.as_dyn(), degree);
            if degree < self.0.min_degree() || degree > self.0.max_computed_degree() {
                return Err(PyIndexError::new_err(format!(
                    "degree {degree} is outside the computed range of the tensor module"
                )));
            }
            // `left_degree` must index a left block: the left factor's degree
            // ranges over `[left.min_degree(), degree - right.min_degree()]`.
            let left_min = self.0.left.min_degree();
            let left_max = degree - self.0.right.min_degree();
            if left_degree < left_min || left_degree > left_max {
                return Err(PyIndexError::new_err(format!(
                    "left_degree {left_degree} out of range [{left_min}, {left_max}] for total \
                     degree {degree}"
                )));
            }
            // A `left_degree` inside the accepted range can still address an
            // empty block when the left factor has dimension 0 there (an
            // internal degree gap, e.g. graded dims `[1, 0, 1]`). Upstream
            // `offset` would then index `blocks[left_degree][0]` out of bounds,
            // so reject it explicitly. `&**self.0.left` reaches the left factor
            // as a `DynModule` for the shared dimension helper.
            if module_dimension(&**self.0.left, left_degree) == 0 {
                return Err(PyIndexError::new_err(format!(
                    "left_degree {left_degree} addresses an empty block (the left factor has \
                     dimension 0 there); the offset of an empty block is undefined"
                )));
            }
            Ok(self.0.offset(degree, left_degree))
        }

        /// Box this module into a `SteenrodModule` for downstream use. Shares
        /// state with this `TensorModule` via an `Arc` (the `FreeModule`
        /// pattern), so the boxed module sees the same computed basis.
        pub fn into_steenrod_module(&self) -> SteenrodModule {
            SteenrodModule(Arc::clone(&self.0) as RsSteenrodModule)
        }

        pub fn __repr__(&self) -> String {
            format!("TensorModule({})", self.0)
        }
    }

    /// A degree shift of a module: `SuspensionModule(inner, shift)` is `inner`
    /// with every degree raised by `shift`. The inner module is passed as a
    /// `SteenrodModule` (box concrete modules with `.into_steenrod_module()`).
    #[pyclass(name = "SuspensionModule")]
    pub struct SuspensionModule {
        inner: Arc<SuspensionModuleInner>,
        // The `shift` field is private upstream with no accessor, so we keep our
        // own copy to back the `shift()` getter.
        shift: i32,
    }

    impl SuspensionModule {
        fn as_dyn(&self) -> &DynModule {
            &*self.inner
        }
    }

    #[pymethods]
    impl SuspensionModule {
        #[new]
        pub fn new(inner: PyRef<'_, SteenrodModule>, shift: i32) -> Self {
            SuspensionModule {
                inner: Arc::new(SuspensionModuleInner::new(
                    Arc::new(Arc::clone(&inner.0)),
                    shift,
                )),
                shift,
            }
        }

        // --- flattened Module method set --------------------------------------

        pub fn algebra(&self) -> SteenrodAlgebra {
            SteenrodAlgebra::from_arc(self.inner.algebra())
        }

        pub fn min_degree(&self) -> i32 {
            self.inner.min_degree()
        }

        pub fn max_computed_degree(&self) -> i32 {
            self.inner.max_computed_degree()
        }

        pub fn max_degree(&self) -> Option<i32> {
            self.inner.max_degree()
        }

        pub fn prime(&self) -> u32 {
            self.inner.prime().as_u32()
        }

        pub fn compute_basis(&self, degree: i32) {
            module_ensure(self.as_dyn(), degree);
        }

        pub fn dimension(&self, degree: i32) -> usize {
            module_dimension(self.as_dyn(), degree)
        }

        pub fn total_dimension(&self) -> PyResult<usize> {
            module_total_dimension(self.as_dyn())
        }

        pub fn is_unit(&self) -> bool {
            self.inner.is_unit()
        }

        pub fn basis_element_to_string(&self, degree: i32, idx: usize) -> PyResult<String> {
            module_basis_element_to_string(self.as_dyn(), degree, idx)
        }

        pub fn element_to_string(
            &self,
            py: Python<'_>,
            degree: i32,
            element: &Bound<'_, PyAny>,
        ) -> PyResult<String> {
            module_element_to_string(self.as_dyn(), py, degree, element)
        }

        #[allow(clippy::too_many_arguments)]
        pub fn act_on_basis(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            op_degree: i32,
            op_index: usize,
            mod_degree: i32,
            mod_index: usize,
        ) -> PyResult<()> {
            module_act_on_basis(
                self.as_dyn(),
                py,
                result,
                coeff,
                op_degree,
                op_index,
                mod_degree,
                mod_index,
            )
        }

        #[allow(clippy::too_many_arguments)]
        pub fn act(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            op_degree: i32,
            op_index: usize,
            input_degree: i32,
            input: &Bound<'_, PyAny>,
        ) -> PyResult<()> {
            module_act(
                self.as_dyn(),
                py,
                result,
                coeff,
                op_degree,
                op_index,
                input_degree,
                input,
            )
        }

        #[allow(clippy::too_many_arguments)]
        pub fn act_by_element(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            op_degree: i32,
            op: &Bound<'_, PyAny>,
            input_degree: i32,
            input: &Bound<'_, PyAny>,
        ) -> PyResult<()> {
            module_act_by_element(
                self.as_dyn(),
                py,
                result,
                coeff,
                op_degree,
                op,
                input_degree,
                input,
            )
        }

        // --- SuspensionModule-specific (thin) ---------------------------------

        /// The degree shift this suspension applies. (Upstream's `shift` field
        /// is private with no accessor, so we report our stored copy.)
        pub fn shift(&self) -> i32 {
            self.shift
        }

        /// Box this module into a `SteenrodModule` for downstream use. Shares
        /// state with this `SuspensionModule` via an `Arc`.
        pub fn into_steenrod_module(&self) -> SteenrodModule {
            SteenrodModule(Arc::clone(&self.inner) as RsSteenrodModule)
        }

        pub fn __repr__(&self) -> String {
            format!("SuspensionModule({})", self.inner)
        }
    }

    /// The zero module over the Steenrod algebra with the given `min_degree`
    /// (an empty finite-dimensional module). Dimension 0 in every degree.
    #[pyclass(name = "ZeroModule")]
    pub struct ZeroModule(Arc<FDModuleInner>);

    impl ZeroModule {
        fn as_dyn(&self) -> &DynModule {
            &*self.0
        }
    }

    #[pymethods]
    impl ZeroModule {
        /// Build the zero module. Mirrors upstream
        /// `FDModule::zero_module(algebra, min_degree)`, i.e. an `FDModule` with
        /// an empty graded dimension starting at `min_degree`.
        #[new]
        #[pyo3(signature = (algebra, min_degree = 0))]
        pub fn new(algebra: PyRef<'_, SteenrodAlgebra>, min_degree: i32) -> Self {
            let graded_dimension = ::bivec::BiVec::new(min_degree);
            ZeroModule(Arc::new(FDModuleInner::new(
                algebra.arc(),
                "zero".to_string(),
                graded_dimension,
            )))
        }

        // --- flattened Module method set --------------------------------------

        pub fn algebra(&self) -> SteenrodAlgebra {
            SteenrodAlgebra::from_arc(self.0.algebra())
        }

        pub fn min_degree(&self) -> i32 {
            self.0.min_degree()
        }

        pub fn max_computed_degree(&self) -> i32 {
            self.0.max_computed_degree()
        }

        pub fn max_degree(&self) -> Option<i32> {
            self.0.max_degree()
        }

        pub fn prime(&self) -> u32 {
            self.0.prime().as_u32()
        }

        pub fn compute_basis(&self, degree: i32) {
            module_ensure(self.as_dyn(), degree);
        }

        pub fn dimension(&self, degree: i32) -> usize {
            module_dimension(self.as_dyn(), degree)
        }

        pub fn total_dimension(&self) -> PyResult<usize> {
            module_total_dimension(self.as_dyn())
        }

        pub fn is_unit(&self) -> bool {
            self.0.is_unit()
        }

        pub fn basis_element_to_string(&self, degree: i32, idx: usize) -> PyResult<String> {
            module_basis_element_to_string(self.as_dyn(), degree, idx)
        }

        pub fn element_to_string(
            &self,
            py: Python<'_>,
            degree: i32,
            element: &Bound<'_, PyAny>,
        ) -> PyResult<String> {
            module_element_to_string(self.as_dyn(), py, degree, element)
        }

        #[allow(clippy::too_many_arguments)]
        pub fn act_on_basis(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            op_degree: i32,
            op_index: usize,
            mod_degree: i32,
            mod_index: usize,
        ) -> PyResult<()> {
            module_act_on_basis(
                self.as_dyn(),
                py,
                result,
                coeff,
                op_degree,
                op_index,
                mod_degree,
                mod_index,
            )
        }

        #[allow(clippy::too_many_arguments)]
        pub fn act(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            op_degree: i32,
            op_index: usize,
            input_degree: i32,
            input: &Bound<'_, PyAny>,
        ) -> PyResult<()> {
            module_act(
                self.as_dyn(),
                py,
                result,
                coeff,
                op_degree,
                op_index,
                input_degree,
                input,
            )
        }

        #[allow(clippy::too_many_arguments)]
        pub fn act_by_element(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            op_degree: i32,
            op: &Bound<'_, PyAny>,
            input_degree: i32,
            input: &Bound<'_, PyAny>,
        ) -> PyResult<()> {
            module_act_by_element(
                self.as_dyn(),
                py,
                result,
                coeff,
                op_degree,
                op,
                input_degree,
                input,
            )
        }

        /// Box this module into a `SteenrodModule` for downstream use. Shares
        /// state with this `ZeroModule` via an `Arc`.
        pub fn into_steenrod_module(&self) -> SteenrodModule {
            SteenrodModule(Arc::clone(&self.0) as RsSteenrodModule)
        }

        pub fn __repr__(&self) -> String {
            format!("ZeroModule({})", self.0)
        }
    }

    /// The real projective space module
    /// `RP_min^max` over the Steenrod algebra at `p = 2`. `max = None` gives
    /// `RP_min^oo`. `clear_bottom` mods out the `A(2)`-submodule generated below
    /// `min` (see the upstream docs); note it always shifts `min` to `-1 mod 8`.
    #[pyclass(name = "RealProjectiveSpace")]
    pub struct RealProjectiveSpace(Arc<RpInner>);

    impl RealProjectiveSpace {
        fn as_dyn(&self) -> &DynModule {
            &*self.0
        }
    }

    #[pymethods]
    impl RealProjectiveSpace {
        /// Build `RP_min^max`. Raises `ValueError` for a non-`p = 2` algebra or
        /// `max < min` (upstream `new` asserts both).
        #[new]
        #[pyo3(signature = (algebra, min, max = None, clear_bottom = false))]
        pub fn new(
            algebra: PyRef<'_, SteenrodAlgebra>,
            min: i32,
            max: Option<i32>,
            clear_bottom: bool,
        ) -> PyResult<Self> {
            if algebra.prime() != 2 {
                return Err(PyValueError::new_err(
                    "RealProjectiveSpace is only defined at p = 2",
                ));
            }
            if let Some(max) = max {
                if max < min {
                    return Err(PyValueError::new_err(format!(
                        "max {max} must be at least min {min}"
                    )));
                }
            }
            Ok(RealProjectiveSpace(Arc::new(RpInner::new(
                algebra.arc(),
                min,
                max,
                clear_bottom,
            ))))
        }

        // --- flattened Module method set --------------------------------------

        pub fn algebra(&self) -> SteenrodAlgebra {
            SteenrodAlgebra::from_arc(self.0.algebra())
        }

        pub fn min_degree(&self) -> i32 {
            self.0.min_degree()
        }

        pub fn max_computed_degree(&self) -> i32 {
            self.0.max_computed_degree()
        }

        pub fn max_degree(&self) -> Option<i32> {
            self.0.max_degree()
        }

        pub fn prime(&self) -> u32 {
            self.0.prime().as_u32()
        }

        pub fn compute_basis(&self, degree: i32) {
            module_ensure(self.as_dyn(), degree);
        }

        pub fn dimension(&self, degree: i32) -> usize {
            module_dimension(self.as_dyn(), degree)
        }

        pub fn total_dimension(&self) -> PyResult<usize> {
            module_total_dimension(self.as_dyn())
        }

        pub fn is_unit(&self) -> bool {
            self.0.is_unit()
        }

        pub fn basis_element_to_string(&self, degree: i32, idx: usize) -> PyResult<String> {
            module_basis_element_to_string(self.as_dyn(), degree, idx)
        }

        pub fn element_to_string(
            &self,
            py: Python<'_>,
            degree: i32,
            element: &Bound<'_, PyAny>,
        ) -> PyResult<String> {
            module_element_to_string(self.as_dyn(), py, degree, element)
        }

        #[allow(clippy::too_many_arguments)]
        pub fn act_on_basis(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            op_degree: i32,
            op_index: usize,
            mod_degree: i32,
            mod_index: usize,
        ) -> PyResult<()> {
            module_act_on_basis(
                self.as_dyn(),
                py,
                result,
                coeff,
                op_degree,
                op_index,
                mod_degree,
                mod_index,
            )
        }

        #[allow(clippy::too_many_arguments)]
        pub fn act(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            op_degree: i32,
            op_index: usize,
            input_degree: i32,
            input: &Bound<'_, PyAny>,
        ) -> PyResult<()> {
            module_act(
                self.as_dyn(),
                py,
                result,
                coeff,
                op_degree,
                op_index,
                input_degree,
                input,
            )
        }

        #[allow(clippy::too_many_arguments)]
        pub fn act_by_element(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            op_degree: i32,
            op: &Bound<'_, PyAny>,
            input_degree: i32,
            input: &Bound<'_, PyAny>,
        ) -> PyResult<()> {
            module_act_by_element(
                self.as_dyn(),
                py,
                result,
                coeff,
                op_degree,
                op,
                input_degree,
                input,
            )
        }

        // --- RealProjectiveSpace-specific (thin) ------------------------------

        #[getter]
        pub fn min(&self) -> i32 {
            self.0.min
        }

        #[getter]
        pub fn max(&self) -> Option<i32> {
            self.0.max
        }

        #[getter]
        pub fn clear_bottom(&self) -> bool {
            self.0.clear_bottom
        }

        /// Box this module into a `SteenrodModule` for downstream use. Shares
        /// state via an `Arc`.
        pub fn into_steenrod_module(&self) -> SteenrodModule {
            SteenrodModule(Arc::clone(&self.0) as RsSteenrodModule)
        }

        pub fn __repr__(&self) -> String {
            format!("RealProjectiveSpace({})", self.0)
        }
    }

    /// A quotient `module / W` of a module over the Steenrod algebra, truncated
    /// above `truncation`: every degree `> truncation` is quotiented to zero,
    /// and in each degree `<= truncation` a subspace `W` (built up with the
    /// `quotient*` methods) is divided out. The inner module is passed as a
    /// `SteenrodModule` (box concrete modules with `.into_steenrod_module()`).
    ///
    /// The `quotient*` methods mutate the subspace and therefore require unique
    /// ownership of the inner `Arc`; while a boxed `SteenrodModule` produced
    /// from this module (via `into_steenrod_module()`) is still alive it shares
    /// the `Arc`, so mutation raises `RuntimeError`. Dropping every such box
    /// restores unique ownership and mutation works again. Build up the
    /// quotient first, then box it.
    #[pyclass(name = "QuotientModule")]
    pub struct QuotientModule(Arc<QuotientModuleInner>);

    impl QuotientModule {
        fn as_dyn(&self) -> &DynModule {
            &*self.0
        }

        /// Mutable access to the inner module for the `quotient*` setters.
        /// Fails while the `Arc` is shared (i.e. while a boxed `SteenrodModule`
        /// produced via `into_steenrod_module()` is still alive), since that
        /// box observes the same state and a mutation would be unsound. Once
        /// every such box is dropped, unique ownership is restored and mutation
        /// succeeds again.
        fn inner_mut(&mut self) -> PyResult<&mut QuotientModuleInner> {
            Arc::get_mut(&mut self.0).ok_or_else(|| {
                PyRuntimeError::new_err(
                    "cannot mutate a QuotientModule after it has been boxed into a SteenrodModule",
                )
            })
        }

        /// Validate that `degree` indexes a populated subspace, i.e. lies in
        /// `[min_degree, truncation]`. Below `min_degree` or above `truncation`
        /// the `subspaces`/`basis_list` `BiVec`s have no entry and upstream
        /// would index-panic.
        fn checked_subspace_degree(&self, degree: i32) -> PyResult<()> {
            let min = self.0.min_degree();
            let trunc = self.0.truncation;
            if degree < min || degree > trunc {
                Err(PyIndexError::new_err(format!(
                    "degree {degree} is outside the quotient's range [{min}, {trunc}]"
                )))
            } else {
                Ok(())
            }
        }
    }

    #[pymethods]
    impl QuotientModule {
        /// Build the quotient of `module` truncated above `truncation`. Raises
        /// `ValueError` if `truncation` is below `min_degree - 1` (upstream
        /// builds `BiVec`s spanning `[min_degree, truncation]` and would
        /// allocate a negative-length capacity / `debug_assert`), or if
        /// `truncation + 1` overflows `i32`.
        #[new]
        pub fn new(module: PyRef<'_, SteenrodModule>, truncation: i32) -> PyResult<Self> {
            let min_degree = module.0.min_degree();
            truncation
                .checked_add(1)
                .ok_or_else(|| PyValueError::new_err("truncation is too large"))?;
            if truncation < min_degree - 1 {
                return Err(PyValueError::new_err(format!(
                    "truncation {truncation} is below the module's min_degree {min_degree}"
                )));
            }
            // Upstream `QuotientModuleInner::new` calls `module.compute_basis(truncation)`,
            // which for a `FreeModule` inner reads `algebra.dimension_unstable(..)` *without*
            // extending the algebra and `OnceVec`-panics if the algebra is not computed
            // through `truncation`. Pre-extend the inner module (and its algebra) here.
            module_ensure(&*module.0, truncation);
            Ok(QuotientModule(Arc::new(QuotientModuleInner::new(
                Arc::new(Arc::clone(&module.0)),
                truncation,
            ))))
        }

        // --- flattened Module method set --------------------------------------

        pub fn algebra(&self) -> SteenrodAlgebra {
            SteenrodAlgebra::from_arc(self.0.algebra())
        }

        pub fn min_degree(&self) -> i32 {
            self.0.min_degree()
        }

        pub fn max_computed_degree(&self) -> i32 {
            self.0.max_computed_degree()
        }

        pub fn max_degree(&self) -> Option<i32> {
            self.0.max_degree()
        }

        pub fn prime(&self) -> u32 {
            self.0.prime().as_u32()
        }

        pub fn compute_basis(&self, degree: i32) {
            module_ensure(self.as_dyn(), degree);
        }

        pub fn dimension(&self, degree: i32) -> usize {
            module_dimension(self.as_dyn(), degree)
        }

        pub fn total_dimension(&self) -> PyResult<usize> {
            module_total_dimension(self.as_dyn())
        }

        pub fn is_unit(&self) -> bool {
            self.0.is_unit()
        }

        pub fn basis_element_to_string(&self, degree: i32, idx: usize) -> PyResult<String> {
            module_basis_element_to_string(self.as_dyn(), degree, idx)
        }

        pub fn element_to_string(
            &self,
            py: Python<'_>,
            degree: i32,
            element: &Bound<'_, PyAny>,
        ) -> PyResult<String> {
            module_element_to_string(self.as_dyn(), py, degree, element)
        }

        #[allow(clippy::too_many_arguments)]
        pub fn act_on_basis(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            op_degree: i32,
            op_index: usize,
            mod_degree: i32,
            mod_index: usize,
        ) -> PyResult<()> {
            module_act_on_basis(
                self.as_dyn(),
                py,
                result,
                coeff,
                op_degree,
                op_index,
                mod_degree,
                mod_index,
            )
        }

        #[allow(clippy::too_many_arguments)]
        pub fn act(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            op_degree: i32,
            op_index: usize,
            input_degree: i32,
            input: &Bound<'_, PyAny>,
        ) -> PyResult<()> {
            module_act(
                self.as_dyn(),
                py,
                result,
                coeff,
                op_degree,
                op_index,
                input_degree,
                input,
            )
        }

        #[allow(clippy::too_many_arguments)]
        pub fn act_by_element(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            op_degree: i32,
            op: &Bound<'_, PyAny>,
            input_degree: i32,
            input: &Bound<'_, PyAny>,
        ) -> PyResult<()> {
            module_act_by_element(
                self.as_dyn(),
                py,
                result,
                coeff,
                op_degree,
                op,
                input_degree,
                input,
            )
        }

        // --- QuotientModule-specific (thin) -----------------------------------

        /// The degree above which everything is quotiented out.
        #[getter]
        pub fn truncation(&self) -> i32 {
            self.0.truncation
        }

        /// Quotient out the subspace spanned (additionally) by `element` in
        /// `degree`. `element` is a coefficient vector of length equal to the
        /// *original* module's dimension in `degree`. A `degree > truncation`
        /// is a no-op upstream; we still require a valid in-range `degree`
        /// (`[min_degree, truncation]`) for the subspace it indexes, the right
        /// prime, and the right length, raising rather than letting
        /// `Subspace::add_vector`/the `BiVec` index panic.
        pub fn quotient(
            &mut self,
            py: Python<'_>,
            degree: i32,
            element: &Bound<'_, PyAny>,
        ) -> PyResult<()> {
            self.checked_subspace_degree(degree)?;
            let p = self.0.prime().as_u32();
            let orig_dim = module_dimension(&**self.0.module, degree);
            let element = crate::fp_py::extract_input_owned(py, element)?;
            checked_same_prime(element.prime().as_u32(), p)?;
            checked_equal_len(element.len(), orig_dim)?;
            self.inner_mut()?.quotient(degree, element.as_slice());
            Ok(())
        }

        /// Quotient out the original basis elements at the given `indices` in
        /// `degree`. Each index must be a valid basis index of the *original*
        /// module in `degree` (`Subspace::add_basis_elements` would otherwise
        /// `set_entry` out of bounds), and `degree` must be in
        /// `[min_degree, truncation]`.
        pub fn quotient_basis_elements(
            &mut self,
            degree: i32,
            indices: Vec<usize>,
        ) -> PyResult<()> {
            self.checked_subspace_degree(degree)?;
            let orig_dim = module_dimension(&**self.0.module, degree);
            for &idx in &indices {
                if idx >= orig_dim {
                    return Err(PyIndexError::new_err(format!(
                        "basis index {idx} out of range for degree {degree} (original dimension \
                         {orig_dim})"
                    )));
                }
            }
            self.inner_mut()?
                .quotient_basis_elements(degree, indices.into_iter());
            Ok(())
        }

        /// Quotient out the entire degree `degree` (set it to zero in the
        /// quotient). `degree` must be in `[min_degree, truncation]`.
        pub fn quotient_all(&mut self, degree: i32) -> PyResult<()> {
            self.checked_subspace_degree(degree)?;
            self.inner_mut()?.quotient_all(degree);
            Ok(())
        }

        /// Reduce `vec` modulo the quotient subspace in `degree`, in place.
        /// For `degree > truncation` this zeroes `vec` (any length is fine);
        /// for `degree` in `[min_degree, truncation]`, `vec` must have length
        /// equal to the *original* module's dimension there (the subspace's
        /// ambient dimension), which `Subspace::reduce` asserts. A
        /// `degree < min_degree` raises `IndexError`.
        pub fn reduce(&self, py: Python<'_>, degree: i32, vec: &Bound<'_, PyAny>) -> PyResult<()> {
            let p = self.0.prime().as_u32();
            if degree < self.0.min_degree() {
                return Err(PyIndexError::new_err(format!(
                    "degree {degree} is below the module's min_degree {}",
                    self.0.min_degree()
                )));
            }
            if degree <= self.0.truncation {
                let orig_dim = module_dimension(&**self.0.module, degree);
                crate::fp_py::with_target_slice_mut(py, vec, |res| {
                    checked_same_prime(res.prime().as_u32(), p)?;
                    checked_equal_len(res.as_slice().len(), orig_dim)?;
                    self.0.reduce(degree, res);
                    Ok(())
                })
            } else {
                crate::fp_py::with_target_slice_mut(py, vec, |res| {
                    checked_same_prime(res.prime().as_u32(), p)?;
                    self.0.reduce(degree, res);
                    Ok(())
                })
            }
        }

        /// Re-express an element written in the *original* module's basis as an
        /// element of the quotient's basis, accumulating into `new`. `old` must
        /// have length equal to the original dimension in `degree`, and `new`
        /// must have length at least the quotient dimension there. `degree`
        /// must be in `[min_degree, truncation]`.
        pub fn old_basis_to_new(
            &self,
            py: Python<'_>,
            degree: i32,
            new: &Bound<'_, PyAny>,
            old: &Bound<'_, PyAny>,
        ) -> PyResult<()> {
            self.checked_subspace_degree(degree)?;
            let p = self.0.prime().as_u32();
            let orig_dim = module_dimension(&**self.0.module, degree);
            let quot_dim = module_dimension(self.as_dyn(), degree);
            let old = crate::fp_py::extract_input_owned(py, old)?;
            checked_same_prime(old.prime().as_u32(), p)?;
            checked_equal_len(old.len(), orig_dim)?;
            crate::fp_py::with_target_slice_mut(py, new, |res| {
                checked_same_prime(res.prime().as_u32(), p)?;
                checked_result_len(res.as_slice().len(), quot_dim)?;
                self.0.old_basis_to_new(degree, res, old.as_slice());
                Ok(())
            })
        }

        /// Box this module into a `SteenrodModule` for downstream use. Shares
        /// state with this `QuotientModule` via an `Arc` (the `FreeModule`
        /// pattern); while a boxed `SteenrodModule` from this module is alive
        /// the `quotient*` setters raise `RuntimeError`, and they work again
        /// once every such box is dropped.
        pub fn into_steenrod_module(&self) -> SteenrodModule {
            SteenrodModule(Arc::clone(&self.0) as RsSteenrodModule)
        }

        pub fn __repr__(&self) -> String {
            format!("QuotientModule({})", self.0)
        }
    }

    /// The Hom module `Hom(source, target)` over the Steenrod algebra, where
    /// `source` is a `FreeModule` and `target` is a *bounded* module. This is
    /// the graded vector space of degree-shifting maps, graded *opposite* to
    /// the usual grading so that it is bounded below; it is a module over the
    /// ground field `F_p` (acted on only by scalars), **not** over the Steenrod
    /// algebra. Consequently it is not a `SteenrodModule` and exposes neither
    /// `algebra()` (the ground-field algebra pyclass `Field` is a separate
    /// §5.2 binding, not yet available) nor `into_steenrod_module()`; both are
    /// deferred for this reason.
    #[pyclass(name = "HomModule")]
    pub struct HomModule(Arc<HomModuleInner>);

    impl HomModule {
        fn as_dyn(&self) -> &dyn Module<Algebra = RsField> {
            &*self.0
        }

        /// Populate book-keeping so that degree-`degree` data can be queried.
        ///
        /// Unlike the other modules, a `HomModule`'s `algebra()` is the ground
        /// field, *not* the Steenrod algebra its source/target are built over.
        /// The generic `module_ensure` therefore cannot extend the right
        /// algebra: `HomModule::compute_basis(degree)` internally runs
        /// `source.compute_basis(degree + target.max_degree())`, which reads
        /// (but does not extend) the source's *Steenrod* algebra tables and
        /// would `OnceVec`-panic if they are not computed far enough. So we
        /// extend the source's Steenrod algebra here, then call the upstream
        /// `compute_basis` (idempotent).
        ///
        /// Returns `true` when degree-`degree` data is (or already was)
        /// computable, and `false` *without computing anything* when `degree`
        /// is so large that the upstream `compute_basis` — which itself adds
        /// `degree + target.max_degree()` (the same sum guarded below) — would
        /// overflow `i32`. Such a degree is not a reachable module degree, so
        /// callers short-circuit to a clean error / zero dimension rather than
        /// panic. A no-op (returning `true`) below `min_degree`, where the
        /// guarded `module_*` helpers already treat the degree as empty.
        fn ensure(&self, degree: i32) -> bool {
            if degree < self.0.min_degree() {
                return true;
            }
            // `target.max_degree()` is `Some` (checked in `new`).
            let tmax = self.0.target().max_degree().unwrap();
            let Some(src_deg) = degree.checked_add(tmax) else {
                // Upstream `HomModule::compute_basis(degree)` recomputes this
                // same `degree + tmax`; bail before it overflows.
                return false;
            };
            let source = self.0.source();
            source
                .algebra()
                .compute_basis(src_deg - source.min_degree());
            self.0.compute_basis(degree);
            true
        }
    }

    #[pymethods]
    impl HomModule {
        /// Build `Hom(source, target)`. `source` must be a `FreeModule` and
        /// `target` any (boxed) `SteenrodModule`. Both must be built from the
        /// *same* `SteenrodAlgebra` Python object: the same-algebra check uses
        /// `Arc::ptr_eq` (there is no cheap structural equality on
        /// `SteenrodAlgebra`), so a distinct-but-equal algebra object is
        /// rejected with `ValueError`. A prime mismatch is rejected first.
        /// `target` must be bounded above (`max_degree()` is not `None`);
        /// otherwise upstream `new` panics, so we raise `ValueError`.
        #[new]
        pub fn new(
            source: PyRef<'_, FreeModule>,
            target: PyRef<'_, SteenrodModule>,
        ) -> PyResult<Self> {
            let source_arc = Arc::clone(&source.0);
            let source_alg = source_arc.algebra();
            let target_alg = target.0.algebra();
            checked_same_prime(source_alg.prime().as_u32(), target_alg.prime().as_u32())?;
            if !Arc::ptr_eq(&source_alg, &target_alg) {
                return Err(PyValueError::new_err(
                    "Hom source and target must be built over the same algebra",
                ));
            }
            if target.0.max_degree().is_none() {
                return Err(PyValueError::new_err(
                    "HomModule requires the target module to be bounded above",
                ));
            }
            Ok(HomModule(Arc::new(HomModuleInner::new(
                source_arc,
                Arc::new(Arc::clone(&target.0)),
            ))))
        }

        // --- flattened Module method set (algebra() deferred, see docstring) --

        pub fn min_degree(&self) -> i32 {
            self.0.min_degree()
        }

        pub fn max_computed_degree(&self) -> i32 {
            self.0.max_computed_degree()
        }

        pub fn max_degree(&self) -> Option<i32> {
            self.0.max_degree()
        }

        pub fn prime(&self) -> u32 {
            self.0.prime().as_u32()
        }

        pub fn compute_basis(&self, degree: i32) {
            self.ensure(degree);
        }

        pub fn dimension(&self, degree: i32) -> usize {
            // An uncomputable (overflowing) degree is not a reachable module
            // degree; report a 0 dimension instead of letting the upstream
            // `compute_basis` re-add `degree + tmax` and overflow-panic.
            if !self.ensure(degree) {
                return 0;
            }
            module_dimension(self.as_dyn(), degree)
        }

        pub fn total_dimension(&self) -> PyResult<usize> {
            // `HomModule` is unbounded above (over a free source), so
            // `max_degree()` is `None` and this raises without computing.
            module_total_dimension(self.as_dyn())
        }

        pub fn is_unit(&self) -> bool {
            self.0.is_unit()
        }

        pub fn basis_element_to_string(&self, degree: i32, idx: usize) -> PyResult<String> {
            // Uncomputable degree -> dimension 0, so any index is out of range.
            if !self.ensure(degree) {
                return Err(PyIndexError::new_err(format!(
                    "index {idx} out of range for degree {degree} (dimension 0)"
                )));
            }
            module_basis_element_to_string(self.as_dyn(), degree, idx)
        }

        pub fn element_to_string(
            &self,
            py: Python<'_>,
            degree: i32,
            element: &Bound<'_, PyAny>,
        ) -> PyResult<String> {
            if !self.ensure(degree) {
                return Err(PyValueError::new_err(format!(
                    "degree {degree} is too large to compute"
                )));
            }
            module_element_to_string(self.as_dyn(), py, degree, element)
        }

        #[allow(clippy::too_many_arguments)]
        pub fn act_on_basis(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            op_degree: i32,
            op_index: usize,
            mod_degree: i32,
            mod_index: usize,
        ) -> PyResult<()> {
            // Pre-extend the source algebra for every degree the guard helper
            // will touch (`mod_degree` and the output `mod_degree + op_degree`).
            // An uncomputable (overflowing) degree has dimension 0, so the
            // basis index cannot exist; bail with a clean `IndexError` before
            // the upstream `compute_basis` overflow-panics.
            if !self.ensure(mod_degree) {
                return Err(PyIndexError::new_err(format!(
                    "module index {mod_index} out of range for degree {mod_degree} (dimension 0)"
                )));
            }
            if op_degree >= 0 {
                if let Some(out) = mod_degree.checked_add(op_degree) {
                    if !self.ensure(out) {
                        return Err(PyValueError::new_err(
                            "output degree is too large to compute",
                        ));
                    }
                }
            }
            module_act_on_basis(
                self.as_dyn(),
                py,
                result,
                coeff,
                op_degree,
                op_index,
                mod_degree,
                mod_index,
            )
        }

        #[allow(clippy::too_many_arguments)]
        pub fn act(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            op_degree: i32,
            op_index: usize,
            input_degree: i32,
            input: &Bound<'_, PyAny>,
        ) -> PyResult<()> {
            // Uncomputable (overflowing) input/output degrees are unreachable
            // module degrees; raise cleanly before the upstream `compute_basis`
            // overflow-panics.
            if !self.ensure(input_degree) {
                return Err(PyValueError::new_err(format!(
                    "degree {input_degree} is too large to compute"
                )));
            }
            if op_degree >= 0 {
                if let Some(out) = input_degree.checked_add(op_degree) {
                    if !self.ensure(out) {
                        return Err(PyValueError::new_err(
                            "output degree is too large to compute",
                        ));
                    }
                }
            }
            module_act(
                self.as_dyn(),
                py,
                result,
                coeff,
                op_degree,
                op_index,
                input_degree,
                input,
            )
        }

        #[allow(clippy::too_many_arguments)]
        pub fn act_by_element(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            op_degree: i32,
            op: &Bound<'_, PyAny>,
            input_degree: i32,
            input: &Bound<'_, PyAny>,
        ) -> PyResult<()> {
            // See `act`: bail cleanly on uncomputable (overflowing) degrees.
            if !self.ensure(input_degree) {
                return Err(PyValueError::new_err(format!(
                    "degree {input_degree} is too large to compute"
                )));
            }
            if op_degree >= 0 {
                if let Some(out) = input_degree.checked_add(op_degree) {
                    if !self.ensure(out) {
                        return Err(PyValueError::new_err(
                            "output degree is too large to compute",
                        ));
                    }
                }
            }
            module_act_by_element(
                self.as_dyn(),
                py,
                result,
                coeff,
                op_degree,
                op,
                input_degree,
                input,
            )
        }

        // --- HomModule-specific (thin) ----------------------------------------

        /// The source `FreeModule` (shares state via an `Arc`).
        pub fn source(&self) -> FreeModule {
            FreeModule(self.0.source())
        }

        /// The target module, as a `SteenrodModule` (shares state via an
        /// `Arc`).
        pub fn target(&self) -> SteenrodModule {
            SteenrodModule((*self.0.target()).clone())
        }

        pub fn __repr__(&self) -> String {
            format!("HomModule({})", self.0)
        }
    }

    /// A finitely presented module over the Steenrod algebra: the quotient of a
    /// `FreeModule` (the *generators*) by the sub-`FreeModule` spanned by a set
    /// of *relations*. Build it by adding generators (in consecutive degrees)
    /// and then relations, or all at once with `from_json`.
    ///
    /// The inner module is held in an `Arc`. `into_steenrod_module()` shares
    /// that `Arc` (the `FreeModule` pattern); while a boxed `SteenrodModule`
    /// from this module is alive the mutating `add_generators`/`add_relations`
    /// raise `RuntimeError` (the `QuotientModule` pattern), since the box
    /// observes the same state. Drop every such box to mutate again.
    #[pyclass(name = "FPModule")]
    pub struct FPModule {
        inner: Arc<FPModuleInner>,
        /// The degree at which the next batch of relations must be added.
        /// Upstream pushes relations into an `OnceBiVec` starting at
        /// `min_degree` via `push_checked`, which asserts the appended index is
        /// exactly the next one; we track that next degree here so we can raise
        /// `ValueError` instead of letting the assertion fire. Mutated only by
        /// `add_relations` (which needs `&mut self` anyway).
        next_relation_degree: i32,
    }

    impl FPModule {
        fn as_dyn(&self) -> &DynModule {
            &*self.inner
        }

        /// Mutable access for `add_generators`/`add_relations`, which upstream
        /// take `&mut self`. Fails while the `Arc` is shared (a boxed
        /// `SteenrodModule` from `into_steenrod_module()` is still alive), since
        /// that box observes the same state.
        fn inner_mut(&mut self) -> PyResult<&mut FPModuleInner> {
            Arc::get_mut(&mut self.inner).ok_or_else(|| {
                PyRuntimeError::new_err(
                    "cannot mutate an FPModule after it has been boxed into a SteenrodModule",
                )
            })
        }
    }

    #[pymethods]
    impl FPModule {
        /// Build an empty finitely presented module over `algebra`, named
        /// `name`, with generators living in degrees `>= min_degree`.
        #[new]
        #[pyo3(signature = (algebra, name, min_degree = 0))]
        pub fn new(algebra: PyRef<'_, SteenrodAlgebra>, name: String, min_degree: i32) -> Self {
            FPModule {
                inner: Arc::new(FPModuleInner::new(algebra.arc(), name, min_degree)),
                next_relation_degree: min_degree,
            }
        }

        // --- flattened Module method set --------------------------------------

        pub fn algebra(&self) -> SteenrodAlgebra {
            SteenrodAlgebra::from_arc(self.inner.algebra())
        }

        pub fn min_degree(&self) -> i32 {
            self.inner.min_degree()
        }

        pub fn max_computed_degree(&self) -> i32 {
            self.inner.max_computed_degree()
        }

        pub fn max_degree(&self) -> Option<i32> {
            self.inner.max_degree()
        }

        pub fn prime(&self) -> u32 {
            self.inner.prime().as_u32()
        }

        pub fn compute_basis(&self, degree: i32) {
            module_ensure(self.as_dyn(), degree);
        }

        pub fn dimension(&self, degree: i32) -> usize {
            module_dimension(self.as_dyn(), degree)
        }

        pub fn total_dimension(&self) -> PyResult<usize> {
            module_total_dimension(self.as_dyn())
        }

        pub fn is_unit(&self) -> bool {
            self.inner.is_unit()
        }

        pub fn basis_element_to_string(&self, degree: i32, idx: usize) -> PyResult<String> {
            module_basis_element_to_string(self.as_dyn(), degree, idx)
        }

        pub fn element_to_string(
            &self,
            py: Python<'_>,
            degree: i32,
            element: &Bound<'_, PyAny>,
        ) -> PyResult<String> {
            module_element_to_string(self.as_dyn(), py, degree, element)
        }

        #[allow(clippy::too_many_arguments)]
        pub fn act_on_basis(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            op_degree: i32,
            op_index: usize,
            mod_degree: i32,
            mod_index: usize,
        ) -> PyResult<()> {
            module_act_on_basis(
                self.as_dyn(),
                py,
                result,
                coeff,
                op_degree,
                op_index,
                mod_degree,
                mod_index,
            )
        }

        #[allow(clippy::too_many_arguments)]
        pub fn act(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            op_degree: i32,
            op_index: usize,
            input_degree: i32,
            input: &Bound<'_, PyAny>,
        ) -> PyResult<()> {
            module_act(
                self.as_dyn(),
                py,
                result,
                coeff,
                op_degree,
                op_index,
                input_degree,
                input,
            )
        }

        #[allow(clippy::too_many_arguments)]
        pub fn act_by_element(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            op_degree: i32,
            op: &Bound<'_, PyAny>,
            input_degree: i32,
            input: &Bound<'_, PyAny>,
        ) -> PyResult<()> {
            module_act_by_element(
                self.as_dyn(),
                py,
                result,
                coeff,
                op_degree,
                op,
                input_degree,
                input,
            )
        }

        // --- FPModule-specific (thin) -----------------------------------------

        /// The underlying generators `FreeModule` (shares state via an `Arc`).
        /// A general element of the FP module is a homogeneous sum of operations
        /// on these generators, modulo the relations.
        pub fn generators(&self) -> FreeModule {
            FreeModule(self.inner.generators())
        }

        /// Add generators in `degree`, one per name in `gen_names`. Generators
        /// must be added at the next consecutive degree (mirroring
        /// `FreeModule.add_generators`, which `push_checked`s into an
        /// `OnceBiVec` keyed by degree): `degree` must equal
        /// `generators().max_computed_degree() + 1` and be `>= min_degree`.
        /// Raises `ValueError` (never panics) otherwise.
        pub fn add_generators(&mut self, degree: i32, gen_names: Vec<String>) -> PyResult<()> {
            let min_degree = self.inner.min_degree();
            if degree < min_degree {
                return Err(PyValueError::new_err(format!(
                    "degree {degree} is below the module's min_degree {min_degree}"
                )));
            }
            let next_expected = self.inner.generators().max_computed_degree() + 1;
            if degree != next_expected {
                return Err(PyValueError::new_err(format!(
                    "generators must be added at the next consecutive degree {next_expected}, got \
                     {degree}"
                )));
            }
            // `add_generators` reads the algebra/opgen tables up to the current
            // computed degree, so make sure they are populated through `degree`.
            module_ensure(&*self.inner.generators(), degree);
            self.inner_mut()?.add_generators(degree, gen_names);
            Ok(())
        }

        /// Add relations in `degree`: each relation is a coefficient vector over
        /// the generators' basis in `degree` (length
        /// `generators().dimension(degree)`, same prime as the module). Pass an
        /// empty list to register a degree with no relations.
        ///
        /// Relations are stored in an `OnceBiVec` starting at `min_degree` and
        /// pushed with `push_checked`, so they must be added at consecutive
        /// degrees starting from `min_degree`: `degree` must equal the next
        /// pending relation degree. Fill intervening degrees with empty lists.
        /// Raises `ValueError` for a wrong degree, prime, or length (never
        /// panics).
        pub fn add_relations(
            &mut self,
            py: Python<'_>,
            degree: i32,
            relations: Vec<Bound<'_, PyAny>>,
        ) -> PyResult<()> {
            let min_degree = self.inner.min_degree();
            if degree < min_degree {
                return Err(PyValueError::new_err(format!(
                    "degree {degree} is below the module's min_degree {min_degree}"
                )));
            }
            if degree != self.next_relation_degree {
                return Err(PyValueError::new_err(format!(
                    "relations must be added at consecutive degrees starting from min_degree; \
                     expected degree {} but got {degree} (fill gaps with empty relation lists)",
                    self.next_relation_degree
                )));
            }
            let p = self.inner.prime().as_u32();
            // The relation vectors live in the generators' space in `degree`;
            // make sure it is computed, then validate every vector's prime and
            // length before handing them to the upstream (which pushes them
            // verbatim and would only panic much later, in `compute_basis`).
            let gens = self.inner.generators();
            module_ensure(&*gens, degree);
            let gen_dim = module_dimension(&*gens, degree);
            let mut rows = Vec::with_capacity(relations.len());
            for reln in &relations {
                let v = crate::fp_py::extract_input_owned(py, reln)?;
                checked_same_prime(v.prime().as_u32(), p)?;
                checked_equal_len(v.len(), gen_dim)?;
                rows.push(v);
            }
            self.inner_mut()?.add_relations(degree, rows);
            self.next_relation_degree = degree + 1;
            Ok(())
        }

        /// Map a generator basis index `idx` in `degree` to its index in the FP
        /// module's basis, or `-1` if that generator is killed by a relation.
        /// Raises `IndexError` for an out-of-range `idx` or a `degree` below
        /// `min_degree` (the degree's data is computed first).
        pub fn gen_idx_to_fp_idx(&self, degree: i32, idx: usize) -> PyResult<isize> {
            if degree < self.inner.min_degree() {
                return Err(PyIndexError::new_err(format!(
                    "degree {degree} is below the module's min_degree {}",
                    self.inner.min_degree()
                )));
            }
            module_ensure(self.as_dyn(), degree);
            // The `gen_idx_to_fp_idx` table has one entry per generator basis
            // element in `degree`, i.e. `generators().dimension(degree)`.
            let gen_dim = module_dimension(&*self.inner.generators(), degree);
            if idx >= gen_dim {
                return Err(PyIndexError::new_err(format!(
                    "generator index {idx} out of range for degree {degree} (generator dimension \
                     {gen_dim})"
                )));
            }
            Ok(self.inner.gen_idx_to_fp_idx(degree, idx))
        }

        /// Map an FP module basis index `idx` in `degree` to the generator basis
        /// index it represents. Raises `IndexError` for an out-of-range `idx`
        /// or a `degree` below `min_degree`.
        pub fn fp_idx_to_gen_idx(&self, degree: i32, idx: usize) -> PyResult<usize> {
            if degree < self.inner.min_degree() {
                return Err(PyIndexError::new_err(format!(
                    "degree {degree} is below the module's min_degree {}",
                    self.inner.min_degree()
                )));
            }
            module_ensure(self.as_dyn(), degree);
            // The `fp_idx_to_gen_idx` table has one entry per FP-module basis
            // element in `degree`, i.e. `self.dimension(degree)`.
            let dim = module_dimension(self.as_dyn(), degree);
            if idx >= dim {
                return Err(PyIndexError::new_err(format!(
                    "fp index {idx} out of range for degree {degree} (dimension {dim})"
                )));
            }
            Ok(self.inner.fp_idx_to_gen_idx(degree, idx))
        }

        /// Build a finitely presented module from a module-spec `dict` over
        /// `algebra`. Mirrors `FinitelyPresentedModule::from_json`, which reads
        /// the generators (`"gens"`) and the `<prefix>_relations` list. All
        /// failures map to `ValueError`.
        ///
        /// Two panic hazards are guarded, exactly as `steenrod_module_from_json`.
        /// First, upstream does not check the spec's prime against `algebra`; a
        /// mismatch (or wrong-prefix relations) makes the relation parser
        /// compute the wrong degree and index out of bounds, so we reject a
        /// spec `p` that disagrees with `algebra.prime()` up front. Second, the
        /// upstream `from_json` `unwrap()`s the `<prefix>_relations` array and
        /// other fields, so we wrap the call in `catch_unwind` to surface a
        /// malformed spec as `ValueError` rather than aborting across the FFI
        /// boundary.
        #[staticmethod]
        pub fn from_json(
            algebra: PyRef<'_, SteenrodAlgebra>,
            value: &Bound<'_, PyAny>,
        ) -> PyResult<Self> {
            use std::panic::{catch_unwind, AssertUnwindSafe};
            let json = py_to_json(value)?;
            if let Some(spec_p) = json["p"].as_u64() {
                let algebra_p = algebra.prime() as u64;
                if spec_p != algebra_p {
                    return Err(PyValueError::new_err(format!(
                        "module spec is over p = {spec_p} but the algebra is over p = {algebra_p}"
                    )));
                }
            }
            let arc = algebra.arc();
            match catch_unwind(AssertUnwindSafe(|| FPModuleInner::from_json(arc, &json))) {
                Ok(Ok(module)) => {
                    let next_relation_degree = module.max_computed_degree() + 1;
                    Ok(FPModule {
                        inner: Arc::new(module),
                        next_relation_degree,
                    })
                }
                Ok(Err(e)) => Err(PyValueError::new_err(e.to_string())),
                Err(_) => Err(PyValueError::new_err(
                    "failed to build FPModule from JSON (malformed spec)",
                )),
            }
        }

        /// Box this module into a `SteenrodModule` for downstream use. Shares
        /// state with this `FPModule` via an `Arc` (the `FreeModule` pattern);
        /// while a boxed `SteenrodModule` from this module is alive the
        /// `add_generators`/`add_relations` setters raise `RuntimeError`, and
        /// they work again once every such box is dropped.
        pub fn into_steenrod_module(&self) -> SteenrodModule {
            SteenrodModule(Arc::clone(&self.inner) as RsSteenrodModule)
        }

        pub fn __repr__(&self) -> String {
            format!("FPModule({})", self.inner)
        }
    }

    /// One basis element of a [`BlockStructure`]: the `basis_index`-th basis
    /// element of the block belonging to generator `(generator_degree,
    /// generator_index)`. Mirrors upstream `GeneratorBasisEltPair`'s three
    /// public fields.
    #[pyclass(name = "GeneratorBasisEltPair")]
    #[derive(Clone)]
    pub struct GeneratorBasisEltPair {
        #[pyo3(get)]
        pub generator_degree: i32,
        #[pyo3(get)]
        pub generator_index: usize,
        #[pyo3(get)]
        pub basis_index: usize,
    }

    #[pymethods]
    impl GeneratorBasisEltPair {
        #[new]
        pub fn new(generator_degree: i32, generator_index: usize, basis_index: usize) -> Self {
            GeneratorBasisEltPair {
                generator_degree,
                generator_index,
                basis_index,
            }
        }

        pub fn __repr__(&self) -> String {
            format!(
                "GeneratorBasisEltPair(generator_degree={}, generator_index={}, basis_index={})",
                self.generator_degree, self.generator_index, self.basis_index
            )
        }
    }

    /// A book-keeping structure mapping between an index into a direct sum of
    /// vector spaces (one "block" per generator) and the individual block
    /// coordinates. Used internally by `FreeModule`/`TensorModule`; exposed as a
    /// standalone helper.
    ///
    /// Construct from `min_degree` and `block_sizes`, a list (indexed from
    /// `min_degree`) of lists giving the size of each generator's block in that
    /// degree.
    #[pyclass(name = "BlockStructure")]
    pub struct BlockStructure {
        inner: RsBlockStructure,
        min_degree: i32,
        /// The block sizes the structure was built from, kept so the query
        /// methods can bounds-check (degree, generator, basis element) against
        /// the private `BiVec`/`Vec` upstream indexes, which would otherwise
        /// panic on out-of-range input.
        block_sizes: Vec<Vec<usize>>,
    }

    impl BlockStructure {
        /// The block sizes for `gen_deg`, or `None` (raising `IndexError`) if
        /// `gen_deg` is outside the populated degree range.
        fn sizes_in_degree(&self, gen_deg: i32) -> PyResult<&Vec<usize>> {
            if gen_deg < self.min_degree {
                return Err(PyIndexError::new_err(format!(
                    "generator degree {gen_deg} is below min_degree {}",
                    self.min_degree
                )));
            }
            let i = (gen_deg - self.min_degree) as usize;
            self.block_sizes.get(i).ok_or_else(|| {
                PyIndexError::new_err(format!(
                    "generator degree {gen_deg} is above the maximum degree {}",
                    self.min_degree + self.block_sizes.len() as i32 - 1
                ))
            })
        }

        fn checked_generator(&self, gen_deg: i32, gen_idx: usize) -> PyResult<usize> {
            let sizes = self.sizes_in_degree(gen_deg)?;
            sizes.get(gen_idx).copied().ok_or_else(|| {
                PyIndexError::new_err(format!(
                    "generator index {gen_idx} out of range in degree {gen_deg} ({} generators)",
                    sizes.len()
                ))
            })
        }
    }

    #[pymethods]
    impl BlockStructure {
        #[new]
        pub fn new(min_degree: i32, block_sizes: Vec<Vec<usize>>) -> Self {
            let bivec = ::bivec::BiVec::from_vec(min_degree, block_sizes.clone());
            BlockStructure {
                inner: RsBlockStructure::new(&bivec),
                min_degree,
                block_sizes,
            }
        }

        /// The half-open index range `(start, end)` of the block belonging to
        /// generator `(gen_deg, gen_idx)`.
        pub fn generator_to_block(&self, gen_deg: i32, gen_idx: usize) -> PyResult<(usize, usize)> {
            self.checked_generator(gen_deg, gen_idx)?;
            let range = self.inner.generator_to_block(gen_deg, gen_idx);
            Ok((range.start, range.end))
        }

        /// The index in the direct sum of the `basis_elt`-th element of the
        /// block belonging to generator `(gen_deg, gen_idx)`.
        pub fn generator_basis_elt_to_index(
            &self,
            gen_deg: i32,
            gen_idx: usize,
            basis_elt: usize,
        ) -> PyResult<usize> {
            let size = self.checked_generator(gen_deg, gen_idx)?;
            if basis_elt >= size {
                return Err(PyIndexError::new_err(format!(
                    "basis element {basis_elt} out of range for the block of generator \
                     ({gen_deg}, {gen_idx}) (block size {size})"
                )));
            }
            Ok(self
                .inner
                .generator_basis_elt_to_index(gen_deg, gen_idx, basis_elt))
        }

        /// The `(generator, basis element)` pair corresponding to index `idx`
        /// of the direct sum.
        pub fn index_to_generator_basis_elt(&self, idx: usize) -> PyResult<GeneratorBasisEltPair> {
            let total = self.inner.total_dimension();
            if idx >= total {
                return Err(PyIndexError::new_err(format!(
                    "index {idx} out of range (total dimension {total})"
                )));
            }
            let pair = self.inner.index_to_generator_basis_elt(idx);
            Ok(GeneratorBasisEltPair {
                generator_degree: pair.generator_degree,
                generator_index: pair.generator_index,
                basis_index: pair.basis_index,
            })
        }

        /// The total dimension of the direct sum (the sum of all block sizes).
        pub fn total_dimension(&self) -> usize {
            self.inner.total_dimension()
        }

        /// Add `coeff * source` into the block of `target` belonging to
        /// generator `(gen_deg, gen_idx)`. `source` must have length equal to
        /// that block's size and `target` must be long enough to cover the
        /// block; both must share the same prime. Raises `ValueError`/
        /// `IndexError` (never panics) on a mismatch.
        pub fn add_block(
            &self,
            py: Python<'_>,
            target: &Bound<'_, PyAny>,
            coeff: u32,
            gen_deg: i32,
            gen_idx: usize,
            source: &Bound<'_, PyAny>,
        ) -> PyResult<()> {
            let size = self.checked_generator(gen_deg, gen_idx)?;
            let range = self.inner.generator_to_block(gen_deg, gen_idx);
            let source = crate::fp_py::extract_input_owned(py, source)?;
            let p = source.prime().as_u32();
            let coeff = coeff % p;
            checked_equal_len(source.len(), size)?;
            crate::fp_py::with_target_slice_mut(py, target, |res| {
                checked_same_prime(res.prime().as_u32(), p)?;
                // `add_block` writes into `target[range.start..range.end]`.
                if res.as_slice().len() < range.end {
                    return Err(PyValueError::new_err(format!(
                        "target has length {} but the block ends at index {}",
                        res.as_slice().len(),
                        range.end
                    )));
                }
                self.inner
                    .add_block(res, coeff, gen_deg, gen_idx, source.as_slice());
                Ok(())
            })
        }

        pub fn __repr__(&self) -> String {
            format!(
                "BlockStructure(total_dimension={})",
                self.inner.total_dimension()
            )
        }
    }

    /// Build a `SteenrodModule` from a module-spec `dict` (the JSON the crate
    /// reads from a module file) over the given `algebra`. Mirrors
    /// `::algebra::module::steenrod_module::from_json`, which dispatches on the
    /// spec's `"type"` field (finite dimensional / finitely presented / real
    /// projective space). Upstream returns an `anyhow::Error` for every failure
    /// (unknown/missing type, malformed spec, parse error) without
    /// distinguishing them, so all `from_json` failures map to `ValueError`.
    /// (Type conversion of the Python value, in `py_to_json`, also raises
    /// `ValueError`.)
    ///
    /// Two panic hazards are guarded explicitly. First, upstream `from_json`
    /// does *not* check the spec's prime against the supplied algebra: a
    /// mismatch makes the action parser compute the wrong output degree and
    /// index `actions` out of bounds (finite_dimensional_module.rs ~396), so we
    /// reject a `p` that disagrees with `algebra.prime()` up front. Second, we
    /// still wrap the upstream call in `catch_unwind` (as the `from_string`
    /// bindings do) so that any remaining internal `unwrap`/index panic on a
    /// malformed spec surfaces as a `ValueError` rather than aborting across the
    /// FFI boundary.
    #[pyfunction]
    pub fn steenrod_module_from_json(
        algebra: PyRef<'_, SteenrodAlgebra>,
        value: &Bound<'_, PyAny>,
    ) -> PyResult<SteenrodModule> {
        use std::panic::{catch_unwind, AssertUnwindSafe};
        let json = py_to_json(value)?;
        if let Some(spec_p) = json["p"].as_u64() {
            let algebra_p = algebra.prime() as u64;
            if spec_p != algebra_p {
                return Err(PyValueError::new_err(format!(
                    "module spec is over p = {spec_p} but the algebra is over p = {algebra_p}"
                )));
            }
        }
        let arc = algebra.arc();
        match catch_unwind(AssertUnwindSafe(|| steenrod_module::from_json(arc, &json))) {
            Ok(Ok(module)) => Ok(SteenrodModule(module)),
            Ok(Err(e)) => Err(PyValueError::new_err(e.to_string())),
            Err(_) => Err(PyValueError::new_err(
                "failed to build module from JSON (malformed spec)",
            )),
        }
    }

    #[pymodule_init]
    fn init(_m: &Bound<'_, PyModule>) -> PyResult<()> {
        // Arbitrary code to run at the module initialization
        // m.add("double2", m.getattr("double")?)
        Ok(())
    }
}
