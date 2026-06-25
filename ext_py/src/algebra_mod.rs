use pyo3::prelude::*;

#[pymodule]
#[pyo3(name = "algebra")]
pub mod algebra_py {
    use std::sync::Arc;

    use ::algebra::module::{
        block_structure::BlockStructure as RsBlockStructure,
        homomorphism::{
            FreeModuleHomomorphism as RsFreeModuleHomomorphism,
            FullModuleHomomorphism as RsFullModuleHomomorphism,
            GenericZeroHomomorphism as RsGenericZeroHomomorphism, HomPullback as RsHomPullback,
            IdentityHomomorphism, ModuleHomomorphism,
            QuotientHomomorphism as RsQuotientHomomorphism,
            QuotientHomomorphismSource as RsQuotientHomomorphismSource, ZeroHomomorphism,
        },
        steenrod_module, FDModule as RsFDModule, FPModule as RsFPModule,
        FreeModule as RsFreeModule, HomModule as RsHomModule, Module,
        OperationGeneratorPair as RsOperationGeneratorPair, QuotientModule as RsQuotientModule,
        RealProjectiveSpace as RsRealProjectiveSpace, SteenrodModule as RsSteenrodModule,
        SuspensionModule as RsSuspensionModule, TensorModule as RsTensorModule,
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
    /// vector space `Hom(source, target)`, only acted on by scalars. Its
    /// `algebra()` is therefore the bound ground-field `Field` pyclass (sharing
    /// the module's `Arc<Field>`), *not* a `SteenrodAlgebra`. Because its
    /// algebra is not `SteenrodAlgebra`, it is *not* a `SteenrodModule` and
    /// exposes no `into_steenrod_module()` (see the binding). The flattened
    /// `Module` method set is still shared via the algebra-generic `module_*`
    /// helpers above.
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
    /// A `FreeModuleHomomorphism` whose *target* is the boxed dynamic module
    /// `RsSteenrodModule` (`Arc<dyn Module>`), mirroring the Tensor/Suspension/
    /// Quotient monomorphisation. Upstream's
    /// `FreeModuleHomomorphism<M>::Source` is then
    /// `FreeModule<M::Algebra> = FreeModule<SteenrodAlgebra> = FreeModuleInner`
    /// (since `RsSteenrodModule::Algebra = SteenrodAlgebra`), so the source is
    /// exactly the bound `FreeModule` pyclass and the target is the bound
    /// `SteenrodModule` pyclass; both share their `Arc`-held state with the
    /// homomorphism. All of its mutators use interior mutability (`OnceVec`/
    /// `OnceBiVec`) and take `&self`, so the pyclass holds the value directly
    /// (no `Consumable`/`Arc::get_mut` dance is required).
    type FreeModuleHomomorphismInner = RsFreeModuleHomomorphism<RsSteenrodModule>;
    /// A `FreeModuleHomomorphism` whose *target* is itself the concrete
    /// `FreeModule<SteenrodAlgebra> = FreeModuleInner` (rather than the boxed
    /// dynamic `RsSteenrodModule`). Upstream's `FreeModuleHomomorphism<M>::Source`
    /// is `FreeModule<M::Algebra>`; with `M = FreeModuleInner` (whose `Algebra`
    /// is `SteenrodAlgebra`) *both* `Source` and `Target` are `FreeModuleInner`,
    /// so the `source()`/`target()` accessors each hand back the bound
    /// `FreeModule` pyclass (sharing the `Arc`). This free → free monomorphisation
    /// is exactly the `map` type `HomPullback::new` requires, and — unlike the
    /// free → dynamic variant — it additionally exposes `hom_k` (the dual map on
    /// cohomology), which upstream gates on `M = FreeModule` (see
    /// `free_module_homomorphism.rs`, the `impl … MuFreeModuleHomomorphism<U,
    /// MuFreeModule<U, A>>` block). The pyclass holds the value behind an `Arc`
    /// (not by value, unlike the free → dynamic variant) so the *same*
    /// homomorphism can be shared into a `HomPullback`; all of its mutators take
    /// `&self` via interior mutability, so the `Arc` needs no `get_mut`.
    type FreeModuleHomToFreeInner = RsFreeModuleHomomorphism<FreeModuleInner>;
    /// A `FullModuleHomomorphism` whose *source* and *target* are both the
    /// boxed dynamic module `RsSteenrodModule` (`Arc<dyn Module>`). Upstream
    /// `FullModuleHomomorphism<S, T>` records the matrix of the map in every
    /// degree, so unlike `FreeModuleHomomorphism` it does not need its source
    /// to be a concrete `FreeModule`; the symmetric `<RsSteenrodModule,
    /// RsSteenrodModule>` monomorphisation lets both factors be the bound
    /// `SteenrodModule` pyclass and is the only choice for which the
    /// `IdentityHomomorphism` impl (which requires `Source == Target`) is
    /// reachable. Since `RsSteenrodModule::Algebra = SteenrodAlgebra`, both the
    /// source and target accessors hand back the same `Arc`-shared
    /// `SteenrodModule`. Upstream `new`/`from_matrices` take `Arc<S>`/`Arc<T>`,
    /// i.e. `Arc<RsSteenrodModule> = Arc<Arc<dyn Module>>`, so each factor is
    /// wrapped once more in an `Arc`. All of its tables use interior mutability
    /// (`OnceBiVec`) and take `&self`, so the pyclass holds the value directly.
    type FullModuleHomomorphismInner = RsFullModuleHomomorphism<RsSteenrodModule, RsSteenrodModule>;
    /// The induced map on quotients, monomorphised so its underlying
    /// homomorphism `F` is exactly the bound `FullModuleHomomorphism` (whose
    /// `Source` and `Target` are both the boxed dynamic module
    /// `RsSteenrodModule`). With `F = FullModuleHomomorphismInner` we have
    /// `QuotientHomomorphism::Source = QuotientModule<F::Source> =
    /// QuotientModule<RsSteenrodModule> = QuotientModuleInner` and likewise for
    /// `Target`, so both `source()` and `target()` hand back the bound
    /// `QuotientModule` pyclass (sharing the same `Arc`). This is the only
    /// monomorphisation for which both quotients are the already-bound
    /// `QuotientModule` type. Upstream `new(f, s, t)` takes `Arc<F>` plus the two
    /// quotient `Arc`s; the binding clones the `FullModuleHomomorphism`'s inner
    /// value into a fresh `Arc` (it is `Clone`) and shares the quotients'
    /// `Arc`s.
    type QuotientHomomorphismInner = RsQuotientHomomorphism<FullModuleHomomorphismInner>;
    /// The source-side quotient map `QuotientModule<F::Source> -> F::Target`,
    /// monomorphised the same way over `F = FullModuleHomomorphismInner`. Its
    /// `Source` is `QuotientModuleInner` (the bound `QuotientModule`) and its
    /// `Target` is `F::Target = RsSteenrodModule` (the bound `SteenrodModule`).
    /// Upstream `new(f, s)` takes `Arc<F>` and the source quotient `Arc`.
    type QuotientHomomorphismSourceInner =
        RsQuotientHomomorphismSource<FullModuleHomomorphismInner>;
    /// The generic zero map between two boxed dynamic modules, monomorphised
    /// `<RsSteenrodModule, RsSteenrodModule>` so both `source()` and `target()`
    /// hand back the bound `SteenrodModule` pyclass. Upstream `new(source,
    /// target, degree_shift)` takes `Arc<S>`/`Arc<T>`, i.e.
    /// `Arc<RsSteenrodModule> = Arc<Arc<dyn Module>>`, so each factor is wrapped
    /// once more in an `Arc`. `apply_to_basis_element` is a no-op upstream and
    /// the map carries no auxiliary data (kernel/image/quasi_inverse are always
    /// `None`).
    type GenericZeroHomomorphismInner =
        RsGenericZeroHomomorphism<RsSteenrodModule, RsSteenrodModule>;
    /// The induced pullback map `Hom(B, X) -> Hom(A, X)` of a free → free map
    /// `A -> B`, monomorphised over `M = RsSteenrodModule` (the boxed dynamic
    /// `X`). Upstream `HomPullback<M>` has
    /// `Source = Target = HomModule<M> = HomModuleInner` (so both `source()` and
    /// `target()` hand back the bound `HomModule` pyclass) and requires
    /// `map: Arc<FreeModuleHomomorphism<FreeModule<M::Algebra>>>`, i.e. exactly
    /// `Arc<FreeModuleHomToFreeInner>` (since `RsSteenrodModule::Algebra =
    /// SteenrodAlgebra`). All of its auxiliary-data tables use interior
    /// mutability (`OnceBiVec`) and take `&self`, so the pyclass holds the value
    /// directly; it also keeps an `Arc` clone of the `map` so the binding can
    /// guard the map's outputs (the upstream `map` field is private).
    type HomPullbackInner = RsHomPullback<RsSteenrodModule>;
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

    /// Like `non_negative_degree`, but raises `ValueError` rather than
    /// `IndexError`. The combinatorics free functions (`inadmissible_pairs`)
    /// treat a negative degree as malformed *input*, not an out-of-range index,
    /// so the `ValueError` taxonomy that the other combinatorics guards use
    /// applies. `non_negative_degree` itself is left unchanged because its other
    /// callers use the degree as an index and rely on `IndexError`.
    fn non_negative_degree_value(degree: i32) -> PyResult<()> {
        if degree >= 0 {
            Ok(())
        } else {
            Err(PyValueError::new_err(format!(
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
    pub(crate) fn py_to_json(value: &Bound<'_, PyAny>) -> PyResult<serde_json::Value> {
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

    /// Convert a `serde_json::Value` into a native Python object
    /// (`None`/`bool`/`int`/`float`/`str`/`list`/`dict`). This is the reverse
    /// direction of [`py_to_json`], completing the single `serde_json::Value`
    /// <-> Python bridge described in API_PROPOSAL §2.6 (we have no `pythonize`
    /// dependency). It is total over `serde_json::Value` and never panics:
    /// every number fits in `i64`/`u64`/`f64` by construction (serde_json's own
    /// invariant), and object/array recursion mirrors the input structure.
    pub(crate) fn json_to_py(py: Python<'_>, value: &serde_json::Value) -> PyResult<Py<PyAny>> {
        use pyo3::types::{PyDict, PyList};
        use serde_json::Value;
        match value {
            Value::Null => Ok(py.None()),
            Value::Bool(b) => Ok(b.into_pyobject(py)?.to_owned().into_any().unbind()),
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(i.into_pyobject(py)?.into_any().unbind())
                } else if let Some(u) = n.as_u64() {
                    Ok(u.into_pyobject(py)?.into_any().unbind())
                } else {
                    // serde_json guarantees a non-integer number round-trips
                    // through f64.
                    let f = n.as_f64().ok_or_else(|| {
                        PyValueError::new_err("JSON number is not representable as f64")
                    })?;
                    Ok(f.into_pyobject(py)?.into_any().unbind())
                }
            }
            Value::String(s) => Ok(s.into_pyobject(py)?.into_any().unbind()),
            Value::Array(arr) => {
                let list = PyList::empty(py);
                for item in arr {
                    list.append(json_to_py(py, item)?)?;
                }
                Ok(list.into_any().unbind())
            }
            Value::Object(map) => {
                let dict = PyDict::new(py);
                for (k, v) in map {
                    dict.set_item(k, json_to_py(py, v)?)?;
                }
                Ok(dict.into_any().unbind())
            }
        }
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
            crate::fp_py::with_input_slice(py, element, |slice| {
                checked_same_prime(slice.prime().as_u32(), self.0.prime().as_u32())?;
                checked_equal_len(slice.len(), self.0.dimension(degree))?;
                Ok(self.0.element_to_string(degree, slice))
            })
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
            crate::fp_py::with_input_slice(py, s, |s_slice| {
                checked_same_prime(s_slice.prime().as_u32(), p)?;
                checked_equal_len(s_slice.len(), self.0.dimension(s_degree))?;
                crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                    checked_same_prime(res.prime().as_u32(), p)?;
                    checked_result_len(res.as_slice().len(), dim)?;
                    self.0.multiply_basis_element_by_element(
                        res.copy(),
                        coeff,
                        r_degree,
                        r_idx,
                        s_degree,
                        s_slice,
                    );
                    Ok(())
                })
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
            crate::fp_py::with_input_slice(py, r, |r_slice| {
                checked_same_prime(r_slice.prime().as_u32(), p)?;
                checked_equal_len(r_slice.len(), self.0.dimension(r_degree))?;
                crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                    checked_same_prime(res.prime().as_u32(), p)?;
                    checked_result_len(res.as_slice().len(), dim)?;
                    self.0.multiply_element_by_basis_element(
                        res.copy(),
                        coeff,
                        r_degree,
                        r_slice,
                        s_degree,
                        s_idx,
                    );
                    Ok(())
                })
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
            crate::fp_py::with_input_slice(py, r, |r_slice| {
                checked_same_prime(r_slice.prime().as_u32(), p)?;
                checked_equal_len(r_slice.len(), self.0.dimension(r_degree))?;
                crate::fp_py::with_input_slice(py, s, |s_slice| {
                    checked_same_prime(s_slice.prime().as_u32(), p)?;
                    checked_equal_len(s_slice.len(), self.0.dimension(s_degree))?;
                    crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                        checked_same_prime(res.prime().as_u32(), p)?;
                        checked_result_len(res.as_slice().len(), dim)?;
                        self.0.multiply_element_by_element(
                            res.copy(),
                            coeff,
                            r_degree,
                            r_slice,
                            s_degree,
                            s_slice,
                        );
                        Ok(())
                    })
                })
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
            crate::fp_py::with_input_slice(py, element, |slice| {
                checked_same_prime(slice.prime().as_u32(), self.0.prime().as_u32())?;
                checked_equal_len(slice.len(), self.0.dimension(degree))?;
                Ok(self.0.element_to_string(degree, slice))
            })
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
            crate::fp_py::with_input_slice(py, s, |s_slice| {
                checked_same_prime(s_slice.prime().as_u32(), p)?;
                checked_equal_len(s_slice.len(), self.0.dimension(s_degree))?;
                crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                    checked_same_prime(res.prime().as_u32(), p)?;
                    checked_result_len(res.as_slice().len(), dim)?;
                    self.0.multiply_basis_element_by_element(
                        res.copy(),
                        coeff,
                        r_degree,
                        r_idx,
                        s_degree,
                        s_slice,
                    );
                    Ok(())
                })
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
            crate::fp_py::with_input_slice(py, r, |r_slice| {
                checked_same_prime(r_slice.prime().as_u32(), p)?;
                checked_equal_len(r_slice.len(), self.0.dimension(r_degree))?;
                crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                    checked_same_prime(res.prime().as_u32(), p)?;
                    checked_result_len(res.as_slice().len(), dim)?;
                    self.0.multiply_element_by_basis_element(
                        res.copy(),
                        coeff,
                        r_degree,
                        r_slice,
                        s_degree,
                        s_idx,
                    );
                    Ok(())
                })
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
            crate::fp_py::with_input_slice(py, r, |r_slice| {
                checked_same_prime(r_slice.prime().as_u32(), p)?;
                checked_equal_len(r_slice.len(), self.0.dimension(r_degree))?;
                crate::fp_py::with_input_slice(py, s, |s_slice| {
                    checked_same_prime(s_slice.prime().as_u32(), p)?;
                    checked_equal_len(s_slice.len(), self.0.dimension(s_degree))?;
                    crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                        checked_same_prime(res.prime().as_u32(), p)?;
                        checked_result_len(res.as_slice().len(), dim)?;
                        self.0.multiply_element_by_element(
                            res.copy(),
                            coeff,
                            r_degree,
                            r_slice,
                            s_degree,
                            s_slice,
                        );
                        Ok(())
                    })
                })
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
            crate::fp_py::with_input_slice(py, element, |slice| {
                checked_same_prime(slice.prime().as_u32(), self.0.prime().as_u32())?;
                checked_equal_len(slice.len(), self.0.dimension(degree))?;
                Ok(self.0.element_to_string(degree, slice))
            })
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
            crate::fp_py::with_input_slice(py, s, |s_slice| {
                checked_same_prime(s_slice.prime().as_u32(), p)?;
                checked_equal_len(s_slice.len(), self.0.dimension(s_degree))?;
                crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                    checked_same_prime(res.prime().as_u32(), p)?;
                    checked_result_len(res.as_slice().len(), dim)?;
                    self.0.multiply_basis_element_by_element(
                        res.copy(),
                        coeff,
                        r_degree,
                        r_idx,
                        s_degree,
                        s_slice,
                    );
                    Ok(())
                })
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
            crate::fp_py::with_input_slice(py, r, |r_slice| {
                checked_same_prime(r_slice.prime().as_u32(), p)?;
                checked_equal_len(r_slice.len(), self.0.dimension(r_degree))?;
                crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                    checked_same_prime(res.prime().as_u32(), p)?;
                    checked_result_len(res.as_slice().len(), dim)?;
                    self.0.multiply_element_by_basis_element(
                        res.copy(),
                        coeff,
                        r_degree,
                        r_slice,
                        s_degree,
                        s_idx,
                    );
                    Ok(())
                })
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
            crate::fp_py::with_input_slice(py, r, |r_slice| {
                checked_same_prime(r_slice.prime().as_u32(), p)?;
                checked_equal_len(r_slice.len(), self.0.dimension(r_degree))?;
                crate::fp_py::with_input_slice(py, s, |s_slice| {
                    checked_same_prime(s_slice.prime().as_u32(), p)?;
                    checked_equal_len(s_slice.len(), self.0.dimension(s_degree))?;
                    crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                        checked_same_prime(res.prime().as_u32(), p)?;
                        checked_result_len(res.as_slice().len(), dim)?;
                        self.0.multiply_element_by_element(
                            res.copy(),
                            coeff,
                            r_degree,
                            r_slice,
                            s_degree,
                            s_slice,
                        );
                        Ok(())
                    })
                })
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

    /// The ground field $\mathbb{F}_p$ viewed as a (graded) **algebra over
    /// itself** — the *trivial* 1-dimensional algebra concentrated in degree 0,
    /// with single basis element `1` (the unit). This is `algebra::Field`.
    ///
    /// Do **not** confuse this with `fp_py.Fp`: `fp_py.Fp` is the *field type*
    /// (the scalars $\mathbb{F}_p$ themselves, used to build `FpVector`s),
    /// whereas `algebra_py.Field` is that field re-packaged as an `Algebra` so
    /// it can be the coefficient algebra of a graded module. Concretely
    /// `dimension(0) == 1`, `dimension(d) == 0` for `d != 0`, and
    /// `multiply_basis_elements` is just the field multiplication on the unit.
    ///
    /// `Field` is the `algebra()` of a `HomModule` (which is the graded vector
    /// space `Hom(source, target)`, acted on only by scalars), so a freshly
    /// constructed `Field` shares the module's `Arc<Field>` storage there.
    ///
    /// Like the other algebra bindings the prime is exposed as a plain `int`
    /// (`ValidPrime` is never surfaced); an invalid prime raises `ValueError`.
    #[pyclass(name = "Field")]
    pub struct Field(Arc<RsField>);

    impl Field {
        /// Re-wrap an `Arc<Field>` handed back by a module's `algebra()` (the
        /// `SteenrodAlgebra::from_arc` pattern) so the same ground field is
        /// shared with Python rather than deep-copied.
        pub(crate) fn from_arc(algebra: Arc<RsField>) -> Self {
            Field(algebra)
        }

        /// Range-check a basis index. The field is 1-dimensional concentrated
        /// in degree 0, so the only valid `(degree, idx)` is `(0, 0)`; every
        /// other pair is `IndexError`. This also forces `degree == 0` wherever
        /// it is applied, which is exactly the precondition the upstream
        /// `basis_element_to_string`/`element_to_string` `assert!(degree == 0)`
        /// guards rely on.
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
    impl Field {
        /// Construct the ground-field algebra `F_p`. Raises `ValueError` for a
        /// non-prime `p` (see `algebra_py.Field` vs `fp_py.Fp` above).
        #[new]
        pub fn new(p: u32) -> PyResult<Self> {
            Ok(Field(Arc::new(RsField::new(valid_prime(p)?))))
        }

        // --- Algebra trait surface --------------------------------------------

        /// The prime as a plain `int` (`ValidPrime` is never exposed).
        pub fn prime(&self) -> u32 {
            self.0.prime().as_u32()
        }

        /// A no-op upstream (the field is finite-dimensional and needs no
        /// book-keeping), kept for parity with the other algebra bindings.
        pub fn compute_basis(&self, _degree: i32) {}

        /// `1` in degree 0, `0` everywhere else (including negative degrees).
        pub fn dimension(&self, degree: i32) -> usize {
            self.0.dimension(degree)
        }

        pub fn basis_element_to_string(&self, degree: i32, idx: usize) -> PyResult<String> {
            // `try_basis_element_to_string` returns `None` for a negative degree
            // or an out-of-range index (the field is 1-dimensional in degree 0),
            // i.e. exactly the upstream `assert!(degree == 0)` precondition.
            self.0.try_basis_element_to_string(degree, idx).ok_or_else(|| {
                PyIndexError::new_err(format!(
                    "no basis element at degree {degree} index {idx}"
                ))
            })
        }

        /// Parse a basis element, returning `(degree, index)`. The field has the
        /// single basis element `1`, so upstream returns `(0, 0)` for *any*
        /// input; we surface that total behaviour unchanged.
        pub fn basis_element_from_string(&self, elt: &str) -> Option<(i32, usize)> {
            self.0.basis_element_from_string(elt)
        }

        pub fn element_to_string(
            &self,
            py: Python<'_>,
            degree: i32,
            element: &Bound<'_, PyAny>,
        ) -> PyResult<String> {
            non_negative_degree(degree)?;
            // Upstream `element_to_string` asserts `degree == 0` and reads
            // `element.entry(0)`, so the element must live in degree 0 (where
            // the dimension is 1).
            checked_equal_len(self.0.dimension(degree), 1)?;
            crate::fp_py::with_input_slice(py, element, |slice| {
                checked_same_prime(slice.prime().as_u32(), self.0.prime().as_u32())?;
                checked_equal_len(slice.len(), self.0.dimension(degree))?;
                Ok(self.0.element_to_string(degree, slice))
            })
        }

        /// Multiply two basis elements, accumulating `coeff * (r * s)` into
        /// `result`. The only basis element is the unit `1` in degree 0, so a
        /// valid product requires `r` and `s` to both be `(0, 0)`; the index
        /// guards reject anything else. Upstream simply adds `coeff` into
        /// component 0, so `result` must have length at least 1.
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
            // Reduce mod p for parity with the other bindings (harmless here:
            // upstream only forwards `coeff` to `add_basis_element`).
            let coeff = coeff % p;
            // Both factors must be the unit `(0, 0)`; this also pins both
            // degrees to 0, so the product lands in degree 0 (dimension 1).
            self.checked_basis_index(r_degree, r_idx)?;
            self.checked_basis_index(s_degree, s_idx)?;
            let dim = self.0.dimension(0);
            crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                checked_same_prime(res.prime().as_u32(), p)?;
                checked_result_len(res.as_slice().len(), dim)?;
                self.0
                    .multiply_basis_elements(res.copy(), coeff, r_degree, r_idx, s_degree, s_idx);
                Ok(())
            })
        }

        pub fn default_filtration_one_products(&self) -> Vec<(String, i32, usize)> {
            self.0.default_filtration_one_products()
        }

        // --- Bialgebra trait surface ------------------------------------------
        //
        // `Field` is a `Bialgebra` (trivial diagonal comultiplication), so
        // `coproduct`/`decompose` are bound. It does *not* implement
        // `GeneratedAlgebra` (it has no generators/relations — it is the unit
        // algebra), so `generators`/`generator_to_string`/
        // `decompose_basis_element`/`generating_relations` are intentionally
        // *not* bound (unlike `MilnorAlgebra`/`AdemAlgebra`). The provided
        // `multiply_*_by_*` element-level helpers are likewise omitted: every
        // product reduces to scaling the single unit basis element, fully
        // exercised by `multiply_basis_elements`.

        /// The (trivial) coproduct of the unit. Only `(0, 0)` is a basis
        /// element, so any other `(degree, idx)` raises rather than describing a
        /// nonexistent element.
        pub fn coproduct(
            &self,
            degree: i32,
            idx: usize,
        ) -> PyResult<Vec<(i32, usize, i32, usize)>> {
            non_negative_degree(degree)?;
            self.checked_basis_index(degree, idx)?;
            Ok(self.0.coproduct(degree, idx))
        }

        pub fn decompose(&self, degree: i32, idx: usize) -> PyResult<Vec<(i32, usize)>> {
            non_negative_degree(degree)?;
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

    /// Number of generators of a concrete `FreeModule` in `degree`, reading 0
    /// (never panicking) outside the populated generator range — the free
    /// function mirror of `FreeModule::num_gens_safe`/
    /// `FreeModuleHomomorphism::source_num_gens`, used where only an
    /// `&FreeModuleInner` is in hand.
    fn fm_num_gens_safe(m: &FreeModuleInner, degree: i32) -> usize {
        if degree < m.min_degree() || degree > m.max_computed_degree() {
            return 0;
        }
        m.number_of_gens_in_degree(degree)
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
        crate::fp_py::with_input_slice(py, element, |slice| {
            checked_same_prime(slice.prime().as_u32(), m.prime().as_u32())?;
            checked_equal_len(slice.len(), dim)?;
            Ok(m.element_to_string(degree, slice))
        })
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
        // Borrow the input transiently rather than cloning it. If the same
        // object is passed as both `input` and `result`, the nested
        // shared+mutable borrows raise `RuntimeError` (PyO3 borrow conflict)
        // rather than UB. `try_act` performs the op-index range check and the
        // `input.len() <= dimension(input_degree)` check internally.
        crate::fp_py::with_input_slice(py, input, |input_slice| {
            checked_same_prime(input_slice.prime().as_u32(), p)?;
            crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                checked_same_prime(res.prime().as_u32(), p)?;
                checked_result_len(res.as_slice().len(), out_dim)?;
                m.try_act(
                    res.copy(),
                    coeff,
                    op_degree,
                    op_index,
                    input_degree,
                    input_slice,
                )
                .map_err(act_error_to_py)?;
                Ok(())
            })
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
        // Borrow both inputs transiently rather than cloning. Aliasing with the
        // mutable `result` surfaces as a `RuntimeError` (PyO3 borrow conflict).
        crate::fp_py::with_input_slice(py, op, |op_slice| {
            checked_same_prime(op_slice.prime().as_u32(), p)?;
            // Upstream `act_by_element` asserts both lengths exactly.
            checked_equal_len(op_slice.len(), op_dim)?;
            crate::fp_py::with_input_slice(py, input, |input_slice| {
                checked_same_prime(input_slice.prime().as_u32(), p)?;
                checked_equal_len(input_slice.len(), in_dim)?;
                crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                    checked_same_prime(res.prime().as_u32(), p)?;
                    checked_result_len(res.as_slice().len(), out_dim)?;
                    m.act_by_element(
                        res.copy(),
                        coeff,
                        op_degree,
                        op_slice,
                        input_degree,
                        input_slice,
                    );
                    Ok(())
                })
            })
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

    impl SteenrodModule {
        /// Wrap an upstream boxed dynamic module (`Arc<dyn Module>`). Used by the
        /// `ext` chain-complex bindings to hand back the modules of a `CCC`
        /// while sharing the same `Arc`.
        pub(crate) fn from_rust(module: RsSteenrodModule) -> Self {
            SteenrodModule(module)
        }

        /// Borrow the underlying `Arc<dyn Module>` (shares interior-mutable state).
        pub(crate) fn as_rust(&self) -> &RsSteenrodModule {
            &self.0
        }
    }

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

    /// A mutable builder for a finite-dimensional module over the Steenrod
    /// algebra. The graded dimensions are given as a `list[int]` starting at
    /// `min_degree`. Populate the actions with
    /// `add_generator`/`set_action`/`extend_actions`/`set_basis_element_name`,
    /// then call [`FDModuleBuilder::build`] to obtain an immutable
    /// `SteenrodModule`.
    ///
    /// The inner module is held in an `Arc` (like `FreeModule`) so that
    /// `build()` can unsize that `Arc` directly into a `SteenrodModule`,
    /// sharing state rather than deep-cloning. `build()` flips a `built` flag;
    /// once it is set, every mutating method raises `RuntimeError` (checked
    /// first, before any other validation, so the error is deterministic and
    /// never a `ValueError`/panic). The `Arc::get_mut` guard in `inner_mut`
    /// remains as a backstop. Read-only query methods stay available for
    /// inspection during construction.
    #[pyclass(name = "FDModuleBuilder")]
    pub struct FDModuleBuilder {
        inner: Arc<FDModuleInner>,
        /// Set by `build()`; once set, all mutators raise `RuntimeError`.
        built: bool,
    }

    impl FDModuleBuilder {
        fn as_dyn(&self) -> &DynModule {
            &*self.inner
        }

        /// Mutable access for the action/generator setters. Fails with
        /// `RuntimeError` (rather than panicking or diverging) once `build()`
        /// has been called: either the `built` flag is set, or the shared `Arc`
        /// makes `Arc::get_mut` return `None`.
        fn inner_mut(&mut self) -> PyResult<&mut FDModuleInner> {
            if self.built {
                return Err(PyRuntimeError::new_err(
                    "cannot mutate an FDModuleBuilder after build()",
                ));
            }
            Arc::get_mut(&mut self.inner).ok_or_else(|| {
                PyRuntimeError::new_err("cannot mutate an FDModuleBuilder after build()")
            })
        }
    }

    #[pymethods]
    impl FDModuleBuilder {
        /// Build an in-progress finite-dimensional module with `graded_dims[i]`
        /// generators in degree `min_degree + i`. All actions are initialised to
        /// zero; use `add_generator`/`set_action`/`extend_actions` to populate
        /// them, then call `build()` to obtain the immutable `SteenrodModule`.
        #[new]
        #[pyo3(signature = (algebra, name, graded_dims, min_degree = 0))]
        pub fn new(
            algebra: PyRef<'_, SteenrodAlgebra>,
            name: String,
            graded_dims: Vec<usize>,
            min_degree: i32,
        ) -> Self {
            let graded_dimension = ::bivec::BiVec::from_vec(min_degree, graded_dims);
            FDModuleBuilder {
                inner: Arc::new(FDModuleInner::new(algebra.arc(), name, graded_dimension)),
                built: false,
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

        // --- FDModuleBuilder-specific (thin) ----------------------------------

        /// Rename a basis element. Raises `RuntimeError` after `build()`
        /// (checked first), or `IndexError` if `(degree, idx)` is not a basis
        /// element (upstream indexes `gen_names` and would panic).
        pub fn set_basis_element_name(
            &mut self,
            degree: i32,
            idx: usize,
            name: String,
        ) -> PyResult<()> {
            if self.built {
                return Err(PyRuntimeError::new_err(
                    "cannot mutate an FDModuleBuilder after build()",
                ));
            }
            checked_mod_index(self.as_dyn(), degree, idx)?;
            self.inner_mut()?.set_basis_element_name(degree, idx, name);
            Ok(())
        }

        /// Append a new generator in `degree`. Raises `RuntimeError` after
        /// `build()`.
        pub fn add_generator(&mut self, degree: i32, name: String) -> PyResult<()> {
            if self.built {
                return Err(PyRuntimeError::new_err(
                    "cannot mutate an FDModuleBuilder after build()",
                ));
            }
            self.inner_mut()?.add_generator(degree, name);
            Ok(())
        }

        /// Set the action `op * x = output`, where `op = (op_degree, op_index)`
        /// and `x = (input_degree, input_index)`. `output` is a coefficient
        /// vector in degree `input_degree + op_degree`. Raises `RuntimeError`
        /// after `build()` (checked first), otherwise `IndexError`/`ValueError`
        /// rather than letting an upstream assertion/`copy_from_slice`
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
            if self.built {
                return Err(PyRuntimeError::new_err(
                    "cannot mutate an FDModuleBuilder after build()",
                ));
            }
            non_negative_degree(op_degree)?;
            self.inner.algebra().compute_basis(op_degree);
            checked_op_index(self.as_dyn(), op_degree, op_index)?;
            checked_mod_index(self.as_dyn(), input_degree, input_index)?;
            let output_degree = input_degree
                .checked_add(op_degree)
                .ok_or_else(|| PyValueError::new_err("output degree overflows i32"))?;
            // Upstream indexes `actions[input_degree][output_degree]`, whose
            // `BiVec::Index` panics when `output_degree` is outside the module's
            // graded range (e.g. above `max_degree`). An empty `output` with an
            // empty (out-of-range) `output_degree` passes the length check but
            // would then panic, so reject it the same way the `action` getter
            // does: an empty output degree is a `ValueError`.
            let out_dim = module_dimension(self.as_dyn(), output_degree);
            if out_dim == 0 {
                return Err(PyValueError::new_err(format!(
                    "output degree {output_degree} is empty"
                )));
            }
            checked_equal_len(output.len(), out_dim)?;
            let p = self.inner.prime().as_u32();
            for v in &output {
                if *v >= p {
                    return Err(PyValueError::new_err(format!(
                        "coefficient {v} is not reduced mod {p}"
                    )));
                }
            }
            self.inner_mut()?
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
            self.inner.algebra().compute_basis(op_degree);
            checked_op_index(self.as_dyn(), op_degree, op_index)?;
            checked_mod_index(self.as_dyn(), input_degree, input_index)?;
            let output_degree = input_degree
                .checked_add(op_degree)
                .ok_or_else(|| PyValueError::new_err("output degree overflows i32"))?;
            if module_dimension(self.as_dyn(), output_degree) == 0 {
                return Err(PyValueError::new_err(format!(
                    "output degree {output_degree} is empty"
                )));
            }
            let vec = self
                .inner
                .action(op_degree, op_index, input_degree, input_index);
            Ok(vec.iter().collect())
        }

        /// Fill in actions of decomposable operations in the given bidegree from
        /// the actions of the algebra generators. Raises if `output_deg <=
        /// input_deg` (upstream asserts).
        pub fn extend_actions(&mut self, input_degree: i32, output_degree: i32) -> PyResult<()> {
            if self.built {
                return Err(PyRuntimeError::new_err(
                    "cannot mutate an FDModuleBuilder after build()",
                ));
            }
            if output_degree <= input_degree {
                return Err(PyValueError::new_err(
                    "output_degree must be strictly greater than input_degree",
                ));
            }
            self.inner
                .algebra()
                .compute_basis(output_degree - input_degree);
            self.inner_mut()?
                .extend_actions(input_degree, output_degree);
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
            self.inner
                .algebra()
                .compute_basis(output_degree - input_degree);
            self.inner
                .check_validity(input_degree, output_degree)
                .map_err(|e| PyValueError::new_err(e.to_string()))
        }

        /// Look up a basis element by its name, returning `(degree, index)` or
        /// `None`.
        pub fn string_to_basis_element(&self, string: &str) -> Option<(i32, usize)> {
            self.inner.string_to_basis_element(string)
        }

        /// Finalize the builder and return the immutable `SteenrodModule` it has
        /// constructed. This is the only producer of the finished module.
        ///
        /// The returned `SteenrodModule` **shares state** with this builder via
        /// an `Arc` (the `FreeModule.into_steenrod_module` pattern): no deep
        /// clone is made, so any pre-build mutation is reflected in the built
        /// module. `build()` flips a `built` flag; afterwards every mutating
        /// method (`set_action`/`add_generator`/`set_basis_element_name`/
        /// `extend_actions`) raises `RuntimeError` (checked first, never a
        /// `ValueError`/panic). `build()` may be called multiple times to obtain
        /// additional handles to the same shared module.
        pub fn build(&mut self) -> SteenrodModule {
            self.built = true;
            // `Arc<FDModuleInner>` unsizes directly to `Arc<dyn Module>`.
            SteenrodModule(Arc::clone(&self.inner) as RsSteenrodModule)
        }

        pub fn __repr__(&self) -> String {
            format!("FDModuleBuilder({})", self.inner)
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

        /// Wrap an existing `Arc<FreeModule<SteenrodAlgebra>>`, sharing the
        /// `Arc` rather than deep-copying. Used by `ext_py::Resolution::module`
        /// to expose a resolution's free modules (which live behind an `Arc`)
        /// without cloning their generator tables.
        pub(crate) fn from_arc(module: Arc<FreeModuleInner>) -> Self {
            FreeModule(module)
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
        //
        // `FreeModule` is intentionally query-only from Python: it has no
        // mutating methods (no `add_generators`/`extend_by_zero`). A populated
        // `FreeModule` is only ever obtained from a path that owns its
        // generators (e.g. `FPModule.generators()` or a resolution), so a
        // handed-out `FreeModule` can never desync the state of whatever module
        // produced it. Construction of generators happens through the owning
        // module's builder (`FPModuleBuilder`) or upstream Rust APIs.

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
            crate::fp_py::with_input_slice(py, element, |slice| {
                checked_same_prime(slice.prime().as_u32(), p)?;
                checked_equal_len(slice.len(), orig_dim)?;
                self.inner_mut()?.quotient(degree, slice);
                Ok(())
            })
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
            crate::fp_py::with_input_slice(py, old, |old_slice| {
                checked_same_prime(old_slice.prime().as_u32(), p)?;
                checked_equal_len(old_slice.len(), orig_dim)?;
                crate::fp_py::with_target_slice_mut(py, new, |res| {
                    checked_same_prime(res.prime().as_u32(), p)?;
                    checked_result_len(res.as_slice().len(), quot_dim)?;
                    self.0.old_basis_to_new(degree, res, old_slice);
                    Ok(())
                })
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
    /// algebra. Its `algebra()` therefore returns the bound ground-field `Field`
    /// pyclass (sharing the module's `Arc<Field>`), not a `SteenrodAlgebra`.
    ///
    /// It is still *not* a `SteenrodModule`, so it exposes no
    /// `into_steenrod_module()`: a `SteenrodModule` is an
    /// `Arc<dyn Module<Algebra = SteenrodAlgebra>>`, whereas a `HomModule`'s
    /// `Algebra` is `Field` — there is no unsizing coercion between
    /// `dyn Module<Algebra = Field>` and `dyn Module<Algebra = SteenrodAlgebra>`
    /// (the associated `Algebra` types differ), so boxing it as a
    /// `SteenrodModule` is a type error, not merely unimplemented. It is left
    /// unbound for that reason.
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

        // --- flattened Module method set --------------------------------------

        /// The ground-field algebra `F_p` this Hom space is a module over,
        /// as the bound `Field` pyclass. Shares the module's `Arc<Field>` (no
        /// `ValidPrime` is exposed, and the prime matches the source/target).
        pub fn algebra(&self) -> Field {
            Field::from_arc(self.0.algebra())
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

        /// Build another `HomModule` `Hom(new_source, X)` over the *same* target
        /// module `X` as this one, sharing `X`'s exact `Arc` storage (not just an
        /// equal module).
        ///
        /// This is needed to build a compatible `(source, target)` pair for
        /// `HomPullback`: its upstream constructor asserts the two Hom modules
        /// share the identical `X` `Arc` (`Arc::ptr_eq`). Because the dynamic
        /// monomorphisation stores `X` behind a *per-instance* outer `Arc`, two
        /// independent `HomModule(f, X)` constructions each wrap `X` afresh and
        /// would fail that identity check; building the second Hom module with
        /// `with_source` reuses the first's outer `Arc` so the check passes.
        ///
        /// `new_source` must be over the same algebra as `X` (checked by prime
        /// and `Arc` identity, like `new`).
        pub fn with_source(&self, new_source: PyRef<'_, FreeModule>) -> PyResult<HomModule> {
            let source_arc = Arc::clone(&new_source.0);
            let source_alg = source_arc.algebra();
            let x = self.0.target();
            let target_alg = x.algebra();
            checked_same_prime(source_alg.prime().as_u32(), target_alg.prime().as_u32())?;
            if !Arc::ptr_eq(&source_alg, &target_alg) {
                return Err(PyValueError::new_err(
                    "Hom source and target must be built over the same algebra",
                ));
            }
            // `X` was already checked bounded above when `self` was built.
            Ok(HomModule(Arc::new(HomModuleInner::new(
                source_arc,
                Arc::clone(&x),
            ))))
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
    /// `FPModule` is an *immutable* view: it has no mutating methods and is not
    /// directly constructible from Python (no `#[new]`). Obtain one from
    /// `FPModuleBuilder.build()` or `FPModule.from_json(...)`. The inner module
    /// is held in an `Arc`; `into_steenrod_module()` shares that `Arc` (the
    /// `FreeModule` pattern). Because `FPModule` exposes no `add_relations`/
    /// `add_generators`, the relation-counter desync that a mutable
    /// `from_json` result could exhibit is impossible by construction.
    #[pyclass(name = "FPModule")]
    pub struct FPModule {
        inner: Arc<FPModuleInner>,
    }

    impl FPModule {
        fn as_dyn(&self) -> &DynModule {
            &*self.inner
        }
    }

    #[pymethods]
    impl FPModule {
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
                Ok(Ok(module)) => Ok(FPModule {
                    inner: Arc::new(module),
                }),
                Ok(Err(e)) => Err(PyValueError::new_err(e.to_string())),
                Err(_) => Err(PyValueError::new_err(
                    "failed to build FPModule from JSON (malformed spec)",
                )),
            }
        }

        /// Box this immutable module into a `SteenrodModule` for downstream
        /// use. Shares state with this `FPModule` via an `Arc` (the
        /// `FreeModule` pattern).
        pub fn into_steenrod_module(&self) -> SteenrodModule {
            SteenrodModule(Arc::clone(&self.inner) as RsSteenrodModule)
        }

        pub fn __repr__(&self) -> String {
            format!("FPModule({})", self.inner)
        }
    }

    /// A mutable builder for a finitely presented module. Add generators (in
    /// consecutive degrees starting at `min_degree`) and then relations, then
    /// call [`FPModuleBuilder::build`] to obtain an immutable [`FPModule`].
    ///
    /// The builder owns the in-progress module in an `Arc` that is unique while
    /// building, so the mutating upstream `add_generators`/`add_relations`
    /// (which take `&mut self`) reach it via `Arc::get_mut`. `build()` clones
    /// that `Arc` into the returned `FPModule` and flips a `built` flag; any
    /// further mutation then raises `RuntimeError` (it never panics). The
    /// builder is built incrementally from empty, so the `next_relation_degree`
    /// counter is always correct — there is no `from_json` path into the
    /// builder, which is why the builder cannot exhibit the relation-counter
    /// desync.
    #[pyclass(name = "FPModuleBuilder")]
    pub struct FPModuleBuilder {
        inner: Arc<FPModuleInner>,
        /// The degree at which the next batch of relations must be added.
        /// Upstream pushes relations into an `OnceBiVec` starting at
        /// `min_degree` via `push_checked`, which asserts the appended index is
        /// exactly the next one; we track that next degree here so we can raise
        /// `ValueError` instead of letting the assertion fire.
        next_relation_degree: i32,
        /// Set by `build()`; once set, all mutators raise `RuntimeError`.
        built: bool,
    }

    impl FPModuleBuilder {
        /// Mutable access for `add_generators`/`add_relations`. Fails (rather
        /// than panicking) once `build()` has been called: either the `built`
        /// flag is set, or the shared `Arc` makes `Arc::get_mut` return `None`.
        fn inner_mut(&mut self) -> PyResult<&mut FPModuleInner> {
            if self.built {
                return Err(PyRuntimeError::new_err(
                    "cannot mutate an FPModuleBuilder after build()",
                ));
            }
            Arc::get_mut(&mut self.inner).ok_or_else(|| {
                PyRuntimeError::new_err("cannot mutate an FPModuleBuilder after build()")
            })
        }
    }

    #[pymethods]
    impl FPModuleBuilder {
        /// Build an empty finitely presented module over `algebra`, named
        /// `name`, with generators living in degrees `>= min_degree`.
        #[new]
        #[pyo3(signature = (algebra, name, min_degree = 0))]
        pub fn new(algebra: PyRef<'_, SteenrodAlgebra>, name: String, min_degree: i32) -> Self {
            FPModuleBuilder {
                inner: Arc::new(FPModuleInner::new(algebra.arc(), name, min_degree)),
                next_relation_degree: min_degree,
                built: false,
            }
        }

        pub fn prime(&self) -> u32 {
            self.inner.prime().as_u32()
        }

        pub fn min_degree(&self) -> i32 {
            self.inner.min_degree()
        }

        /// Add generators in `degree`, one per name in `gen_names`. Generators
        /// must be added at the next consecutive degree (mirroring upstream
        /// `FreeModule::add_generators`, which `push_checked`s into an
        /// `OnceBiVec` keyed by degree): `degree` must equal
        /// `generators().max_computed_degree() + 1` and be `>= min_degree`.
        /// Raises `ValueError` (never panics) otherwise, or `RuntimeError`
        /// after `build()`.
        pub fn add_generators(&mut self, degree: i32, gen_names: Vec<String>) -> PyResult<()> {
            if self.built {
                return Err(PyRuntimeError::new_err(
                    "cannot mutate an FPModuleBuilder after build()",
                ));
            }
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
        /// panics), or `RuntimeError` after `build()`.
        pub fn add_relations(
            &mut self,
            py: Python<'_>,
            degree: i32,
            relations: Vec<Bound<'_, PyAny>>,
        ) -> PyResult<()> {
            if self.built {
                return Err(PyRuntimeError::new_err(
                    "cannot mutate an FPModuleBuilder after build()",
                ));
            }
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

        /// Finalize the builder and return an immutable [`FPModule`] sharing the
        /// underlying module via an `Arc`. After `build()`, any further
        /// mutation on this builder raises `RuntimeError` (never panics).
        /// `build()` may be called again to obtain another handle to the same
        /// immutable module.
        pub fn build(&mut self) -> FPModule {
            self.built = true;
            FPModule {
                inner: Arc::clone(&self.inner),
            }
        }

        pub fn __repr__(&self) -> String {
            format!("FPModuleBuilder({})", self.inner)
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
            crate::fp_py::with_input_slice(py, source, |source_slice| {
                let p = source_slice.prime().as_u32();
                let coeff = coeff % p;
                checked_equal_len(source_slice.len(), size)?;
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
                        .add_block(res, coeff, gen_deg, gen_idx, source_slice);
                    Ok(())
                })
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

    /// A homomorphism `f: F -> M` out of a `FreeModule` `F` (the *source*) into
    /// an arbitrary module `M` (the *target*), with `output_degree =
    /// input_degree - degree_shift`. This is the workhorse of resolution
    /// machinery: a differential is a `FreeModuleHomomorphism` built up
    /// degree-by-degree by specifying the image of each new generator.
    ///
    /// The source is the bound `FreeModule` pyclass and the target is the bound
    /// `SteenrodModule` pyclass; both share their `Arc`-held state with this
    /// homomorphism (the `source()`/`target()` accessors hand back the same
    /// underlying module, not a copy). Box a concrete target module with
    /// `.into_steenrod_module()` first.
    ///
    /// Every degree-indexed access is pre-checked so that an uncomputed degree,
    /// out-of-range index, prime/length mismatch, or non-consecutive mutation
    /// raises `ValueError`/`IndexError` rather than panicking across the FFI
    /// boundary. The internal `outputs`/`images`/`kernels`/`quasi_inverses`
    /// tables use interior mutability (`OnceBiVec`), so every method takes
    /// `&self`.
    #[pyclass(name = "FreeModuleHomomorphism")]
    pub struct FreeModuleHomomorphism(FreeModuleHomomorphismInner);

    impl FreeModuleHomomorphism {
        /// Dimension of the source `FreeModule` in `degree` (guarded; computes
        /// the basis first and reads 0 below `min_degree`).
        fn source_dim(&self, degree: i32) -> usize {
            module_dimension(&*self.0.source() as &DynModule, degree)
        }

        /// Dimension of the target module in `degree` (guarded).
        fn target_dim(&self, degree: i32) -> usize {
            module_dimension(&**self.0.target() as &DynModule, degree)
        }

        /// Ensure both the source basis through `input_degree` and the target
        /// basis through `output_degree` are computed.
        fn ensure_through(&self, input_degree: i32, output_degree: i32) {
            module_ensure(&*self.0.source() as &DynModule, input_degree);
            module_ensure(&**self.0.target() as &DynModule, output_degree);
        }

        /// `input_degree - degree_shift`, raising `ValueError` on overflow.
        fn output_degree(&self, input_degree: i32) -> PyResult<i32> {
            input_degree
                .checked_sub(self.0.degree_shift())
                .ok_or_else(|| PyValueError::new_err("output degree overflows i32"))
        }

        /// Number of generators of the source in `degree`, reading 0 (never
        /// panicking) outside the populated generator range — mirrors
        /// `FreeModule::num_gens_safe`.
        fn source_num_gens(&self, degree: i32) -> usize {
            let source = self.0.source();
            if degree < source.min_degree() || degree > source.max_computed_degree() {
                return 0;
            }
            source.number_of_gens_in_degree(degree)
        }

        /// Verify the outputs are defined on every source generator that can
        /// appear in a basis element of degree `<= hi`. `apply_to_basis_element`
        /// reads `outputs[generator_degree]` (panicking if `generator_degree >=
        /// next_degree()`), so a basis element built on a generator whose output
        /// is not yet set would abort across the boundary. A generator in degree
        /// `d` only exists once it has been added to the source, i.e. for `d <=
        /// source.max_computed_degree()`. We therefore reject if any such
        /// generator degree in `[next_degree(), hi]` carries generators. (Used
        /// by the methods that touch *every* basis element in a degree —
        /// `get_partial_matrix`/`compute_auxiliary_data_through_degree`.)
        fn check_outputs_cover(&self, hi: i32) -> PyResult<()> {
            let source = self.0.source();
            let lo = self.0.next_degree().max(source.min_degree());
            let hi = hi.min(source.max_computed_degree());
            for d in lo..=hi {
                if source.number_of_gens_in_degree(d) > 0 {
                    return Err(PyValueError::new_err(format!(
                        "the homomorphism's outputs are not defined on the source generators in \
                         degree {d}; define them (extend_by_zero / add_generators_from_rows) up to \
                         degree {hi} first"
                    )));
                }
            }
            Ok(())
        }

        /// Verify that the single basis element `(input_degree, input_idx)` only
        /// involves a generator whose output is defined. Returns `Ok` for a
        /// generator below `min_degree()` (which `apply_to_basis_element` treats
        /// as contributing zero) and for any generator degree `< next_degree()`;
        /// rejects a generator degree `>= next_degree()` (which would index the
        /// `outputs` table out of range). The caller must have already computed
        /// the source basis through `input_degree` and bounds-checked
        /// `input_idx`.
        fn check_basis_element_defined(&self, input_degree: i32, input_idx: usize) -> PyResult<()> {
            let source = self.0.source();
            let generator_degree = source
                .index_to_op_gen(input_degree, input_idx)
                .generator_degree;
            if generator_degree >= self.0.next_degree() {
                return Err(PyValueError::new_err(format!(
                    "the homomorphism's output is not defined on the source generator in degree \
                     {generator_degree} (define it with add_generators_from_rows / extend_by_zero \
                     first)"
                )));
            }
            Ok(())
        }
    }

    #[pymethods]
    impl FreeModuleHomomorphism {
        /// Build the zero homomorphism `source -> target` with the given
        /// `degree_shift` (`output_degree = input_degree - degree_shift`). The
        /// outputs on generators are all unset; populate them with
        /// `add_generators_from_rows`/`add_generators_from_matrix_rows`/
        /// `extend_by_zero`. The factors must be built over the *same* algebra
        /// object (checked by prime and `Arc` identity, like `TensorModule`),
        /// since the homomorphism applies the source's algebra action on the
        /// target.
        #[new]
        #[pyo3(signature = (source, target, degree_shift = 0))]
        pub fn new(
            source: PyRef<'_, FreeModule>,
            target: PyRef<'_, SteenrodModule>,
            degree_shift: i32,
        ) -> PyResult<Self> {
            let source_alg = source.0.algebra();
            let target_alg = target.0.algebra();
            checked_same_prime(source_alg.prime().as_u32(), target_alg.prime().as_u32())?;
            if !Arc::ptr_eq(&source_alg, &target_alg) {
                return Err(PyValueError::new_err(
                    "source and target must be built over the same algebra",
                ));
            }
            Ok(FreeModuleHomomorphism(FreeModuleHomomorphismInner::new(
                Arc::clone(&source.0),
                Arc::new(Arc::clone(&target.0)),
                degree_shift,
            )))
        }

        // --- flattened ModuleHomomorphism method set --------------------------

        /// The source `FreeModule` (shares state via `Arc`).
        pub fn source(&self) -> FreeModule {
            FreeModule(self.0.source())
        }

        /// The target module, boxed as a `SteenrodModule` (shares state via
        /// `Arc`).
        pub fn target(&self) -> SteenrodModule {
            SteenrodModule((*self.0.target()).clone())
        }

        /// The degree shift: `output_degree = input_degree - degree_shift`.
        pub fn degree_shift(&self) -> i32 {
            self.0.degree_shift()
        }

        /// The smallest input degree the homomorphism is defined on,
        /// `max(source.min_degree(), target.min_degree() + degree_shift)`.
        pub fn min_degree(&self) -> i32 {
            self.0.min_degree()
        }

        /// The prime as a plain `int` (`ValidPrime` is never exposed).
        pub fn prime(&self) -> u32 {
            self.0.prime().as_u32()
        }

        /// Apply the homomorphism to the basis element `input_idx` in
        /// `input_degree`, adding `coeff` times its image into `result` (a
        /// vector of length `target.dimension(input_degree - degree_shift)`).
        pub fn apply_to_basis_element(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            input_degree: i32,
            input_idx: usize,
        ) -> PyResult<()> {
            let p = self.0.prime().as_u32();
            let coeff = coeff % p;
            if input_degree < self.0.source().min_degree() {
                return Err(PyIndexError::new_err(format!(
                    "input degree {input_degree} is below the source min_degree {}",
                    self.0.source().min_degree()
                )));
            }
            let output_degree = self.output_degree(input_degree)?;
            self.ensure_through(input_degree, output_degree);
            let src_dim = self.source_dim(input_degree);
            if input_idx >= src_dim {
                return Err(PyIndexError::new_err(format!(
                    "input index {input_idx} out of range for source degree {input_degree} \
                     (dimension {src_dim})"
                )));
            }
            self.check_basis_element_defined(input_degree, input_idx)?;
            let out_dim = self.target_dim(output_degree);
            crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                checked_same_prime(res.prime().as_u32(), p)?;
                checked_equal_len(res.as_slice().len(), out_dim)?;
                self.0
                    .apply_to_basis_element(res.copy(), coeff, input_degree, input_idx);
                Ok(())
            })
        }

        /// Apply the homomorphism to a general `input` element of
        /// `source` in `input_degree` (length `source.dimension(input_degree)`),
        /// adding `coeff` times its image into `result`.
        pub fn apply(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            input_degree: i32,
            input: &Bound<'_, PyAny>,
        ) -> PyResult<()> {
            let p = self.0.prime().as_u32();
            let coeff = coeff % p;
            if input_degree < self.0.source().min_degree() {
                return Err(PyIndexError::new_err(format!(
                    "input degree {input_degree} is below the source min_degree {}",
                    self.0.source().min_degree()
                )));
            }
            let output_degree = self.output_degree(input_degree)?;
            self.ensure_through(input_degree, output_degree);
            let src_dim = self.source_dim(input_degree);
            let out_dim = self.target_dim(output_degree);
            crate::fp_py::with_input_slice(py, input, |in_slice| {
                checked_same_prime(in_slice.prime().as_u32(), p)?;
                checked_equal_len(in_slice.len(), src_dim)?;
                // Every basis element with a nonzero coefficient must be built
                // on a generator whose output is defined, else
                // `apply_to_basis_element` would index the `outputs` table out
                // of range. Check precisely so a partially-defined map can still
                // be applied to inputs that only touch the defined part.
                for (i, _) in in_slice.iter_nonzero() {
                    self.check_basis_element_defined(input_degree, i)?;
                }
                crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                    checked_same_prime(res.prime().as_u32(), p)?;
                    checked_equal_len(res.as_slice().len(), out_dim)?;
                    self.0.apply(res.copy(), coeff, input_degree, in_slice);
                    Ok(())
                })
            })
        }

        /// The kernel of the homomorphism in `degree`, if it has been computed
        /// (via `compute_auxiliary_data_through_degree` or `set_kernel`).
        /// Returns `None` otherwise (never panics).
        pub fn kernel(&self, degree: i32) -> Option<crate::fp_py::PySubspace> {
            self.0
                .kernel(degree)
                .map(|s| crate::fp_py::PySubspace::from_rust(s.clone()))
        }

        /// The image of the homomorphism in `degree`, if it has been computed.
        pub fn image(&self, degree: i32) -> Option<crate::fp_py::PySubspace> {
            self.0
                .image(degree)
                .map(|s| crate::fp_py::PySubspace::from_rust(s.clone()))
        }

        /// The quasi-inverse of the homomorphism in `degree`, if it has been
        /// computed.
        pub fn quasi_inverse(&self, degree: i32) -> Option<crate::fp_py::PyQuasiInverse> {
            self.0
                .quasi_inverse(degree)
                .map(|qi| crate::fp_py::PyQuasiInverse::from_rust(qi.clone()))
        }

        /// Compute (and cache) the image, kernel and quasi-inverse at every
        /// input degree up to `degree`. Requires the outputs on generators to be
        /// defined through `degree` (otherwise raises `ValueError`); also raises
        /// `ValueError` if a previous manual `set_image`/`set_kernel`/
        /// `set_quasi_inverse` has left the three auxiliary tables out of sync
        /// (which would otherwise panic on a non-consecutive insert).
        pub fn compute_auxiliary_data_through_degree(&self, degree: i32) -> PyResult<()> {
            let kernels_len = self.0.kernels.len();
            // The auxiliary data is only computed for degrees `>= kernels_len`;
            // each such degree's matrix touches every basis element, so the
            // outputs must be defined on every source generator up to `degree`.
            if degree >= kernels_len {
                self.check_outputs_cover(degree)?;
            }
            if self.0.images.len() != kernels_len || self.0.quasi_inverses.len() != kernels_len {
                return Err(PyValueError::new_err(
                    "auxiliary data tables are out of sync (a prior set_image/set_kernel/\
                     set_quasi_inverse advanced them unequally); cannot compute",
                ));
            }
            self.0.compute_auxiliary_data_through_degree(degree);
            Ok(())
        }

        /// The matrix whose rows are the images of the source basis elements
        /// `inputs` in `degree`. Columns index `target.dimension(degree)`.
        ///
        /// Note: upstream sizes the matrix columns by `target.dimension(degree)`
        /// but the per-row application lands in `target.dimension(degree -
        /// degree_shift)`; the two agree (so the call is well-defined) exactly
        /// when those dimensions coincide — always the case for `degree_shift ==
        /// 0`. We pre-check that equality and raise `ValueError` otherwise rather
        /// than letting the dimension assertion panic.
        pub fn get_partial_matrix(
            &self,
            degree: i32,
            inputs: Vec<usize>,
        ) -> PyResult<crate::fp_py::PyMatrix> {
            if degree < self.0.source().min_degree() {
                return Err(PyIndexError::new_err(format!(
                    "degree {degree} is below the source min_degree {}",
                    self.0.source().min_degree()
                )));
            }
            let output_degree = self.output_degree(degree)?;
            self.ensure_through(degree, output_degree);
            let src_dim = self.source_dim(degree);
            for &i in &inputs {
                if i >= src_dim {
                    return Err(PyIndexError::new_err(format!(
                        "input index {i} out of range for source degree {degree} (dimension \
                         {src_dim})"
                    )));
                }
            }
            self.check_outputs_cover(degree)?;
            if self.target_dim(degree) != self.target_dim(output_degree) {
                return Err(PyValueError::new_err(
                    "get_partial_matrix is only well-defined when target.dimension(degree) == \
                     target.dimension(degree - degree_shift) (e.g. degree_shift == 0)",
                ));
            }
            Ok(crate::fp_py::PyMatrix::from_rust(
                self.0.get_partial_matrix(degree, &inputs),
            ))
        }

        /// Apply the quasi-inverse at `degree` to `input`, adding the result
        /// into `result`. Returns `True` if the quasi-inverse was available (and
        /// applied), `False` otherwise. `input` has length
        /// `target.dimension(degree - degree_shift)` and `result` has length
        /// `source.dimension(degree)`.
        pub fn apply_quasi_inverse(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            degree: i32,
            input: &Bound<'_, PyAny>,
        ) -> PyResult<bool> {
            let p = self.0.prime().as_u32();
            let Some(qi) = self.0.quasi_inverse(degree) else {
                return Ok(false);
            };
            let source_dim = qi.source_dimension();
            let target_dim = qi.target_dimension();
            crate::fp_py::with_input_slice(py, input, |in_slice| {
                checked_same_prime(in_slice.prime().as_u32(), p)?;
                checked_equal_len(in_slice.len(), target_dim)?;
                crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                    checked_same_prime(res.prime().as_u32(), p)?;
                    checked_equal_len(res.as_slice().len(), source_dim)?;
                    qi.apply(res.copy(), 1, in_slice);
                    Ok(())
                })
            })?;
            Ok(true)
        }

        // --- FreeModuleHomomorphism-specific methods --------------------------

        /// The first input degree whose outputs on generators have *not* yet
        /// been defined (i.e. the length of the `outputs` table).
        pub fn next_degree(&self) -> i32 {
            self.0.next_degree()
        }

        /// The image of the generator `(generator_degree, generator_index)`, a
        /// vector of length `target.dimension(generator_degree - degree_shift)`.
        pub fn output(
            &self,
            generator_degree: i32,
            generator_index: usize,
        ) -> PyResult<crate::fp_py::PyFpVector> {
            if generator_degree < self.0.min_degree() {
                return Err(PyIndexError::new_err(format!(
                    "generator degree {generator_degree} is below min_degree {}",
                    self.0.min_degree()
                )));
            }
            if generator_degree >= self.0.next_degree() {
                return Err(PyValueError::new_err(format!(
                    "outputs are only defined through degree {} (add generators / extend_by_zero \
                     first)",
                    self.0.next_degree() - 1
                )));
            }
            let num_gens = self.source_num_gens(generator_degree);
            if generator_index >= num_gens {
                return Err(PyIndexError::new_err(format!(
                    "generator index {generator_index} out of range in degree {generator_degree} \
                     ({num_gens} generators)"
                )));
            }
            Ok(crate::fp_py::PyFpVector::from_rust(
                self.0.output(generator_degree, generator_index).clone(),
            ))
        }

        /// Apply the homomorphism to the generator `idx` in `degree`, adding
        /// `coeff` times its image (`output(degree, idx)`) into `result`.
        pub fn apply_to_generator(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            degree: i32,
            idx: usize,
        ) -> PyResult<()> {
            let p = self.0.prime().as_u32();
            let coeff = coeff % p;
            if degree < self.0.min_degree() {
                return Err(PyIndexError::new_err(format!(
                    "generator degree {degree} is below min_degree {}",
                    self.0.min_degree()
                )));
            }
            if degree >= self.0.next_degree() {
                return Err(PyValueError::new_err(format!(
                    "outputs are only defined through degree {} (add generators / extend_by_zero \
                     first)",
                    self.0.next_degree() - 1
                )));
            }
            let num_gens = self.source_num_gens(degree);
            if idx >= num_gens {
                return Err(PyIndexError::new_err(format!(
                    "generator index {idx} out of range in degree {degree} ({num_gens} generators)"
                )));
            }
            let output_degree = self.output_degree(degree)?;
            let out_dim = self.target_dim(output_degree);
            crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                checked_same_prime(res.prime().as_u32(), p)?;
                checked_equal_len(res.as_slice().len(), out_dim)?;
                res.add(self.0.output(degree, idx).as_slice(), coeff);
                Ok(())
            })
        }

        /// Set the outputs on the generators in `degree` to zero, extending the
        /// `outputs` table up to `degree`. Requires the source's generator
        /// counts to be defined through `degree`.
        pub fn extend_by_zero(&self, degree: i32) -> PyResult<()> {
            if degree >= self.0.next_degree() && degree > self.0.source().max_computed_degree() {
                return Err(PyValueError::new_err(format!(
                    "source generators are only defined through degree {} (cannot extend \
                     outputs to degree {degree})",
                    self.0.source().max_computed_degree()
                )));
            }
            let output_degree = self.output_degree(degree)?;
            self.ensure_through(degree, output_degree);
            self.0.extend_by_zero(degree);
            Ok(())
        }

        /// Define the outputs on the generators in `degree` from `rows`, one
        /// vector per generator (each of length `target.dimension(degree -
        /// degree_shift)`). `degree` must be the next undefined degree
        /// (`next_degree()`), consistent with the consecutive `OnceVec` push.
        pub fn add_generators_from_rows(
            &self,
            py: Python<'_>,
            degree: i32,
            rows: Vec<Bound<'_, PyAny>>,
        ) -> PyResult<()> {
            let p = self.0.prime().as_u32();
            if degree != self.0.next_degree() {
                return Err(PyValueError::new_err(format!(
                    "generators must be added consecutively: expected degree {}, got {degree}",
                    self.0.next_degree()
                )));
            }
            if degree > self.0.source().max_computed_degree() {
                return Err(PyValueError::new_err(format!(
                    "source generators are only defined through degree {}",
                    self.0.source().max_computed_degree()
                )));
            }
            let num_gens = self.source_num_gens(degree);
            if rows.len() != num_gens {
                return Err(PyValueError::new_err(format!(
                    "expected {num_gens} rows (one per generator in degree {degree}), got {}",
                    rows.len()
                )));
            }
            let output_degree = self.output_degree(degree)?;
            self.ensure_through(degree, output_degree);
            let out_dim = self.target_dim(output_degree);
            let mut owned: Vec<::fp::vector::FpVector> = Vec::with_capacity(rows.len());
            for row in &rows {
                let vec = crate::fp_py::extract_input_owned(py, row)?;
                checked_same_prime(vec.prime().as_u32(), p)?;
                checked_equal_len(vec.len(), out_dim)?;
                owned.push(vec);
            }
            self.0.add_generators_from_rows(degree, owned);
            Ok(())
        }

        /// Define the outputs on the generators in `degree` from the rows of
        /// `matrix` (the first `num_gens` rows are used). `degree` must be the
        /// next undefined degree. The matrix must have at least `num_gens` rows
        /// and exactly `target.dimension(degree - degree_shift)` columns.
        pub fn add_generators_from_matrix_rows(
            &self,
            degree: i32,
            matrix: PyRef<'_, crate::fp_py::PyMatrix>,
        ) -> PyResult<()> {
            let p = self.0.prime().as_u32();
            if degree != self.0.next_degree() {
                return Err(PyValueError::new_err(format!(
                    "generators must be added consecutively: expected degree {}, got {degree}",
                    self.0.next_degree()
                )));
            }
            if degree > self.0.source().max_computed_degree() {
                return Err(PyValueError::new_err(format!(
                    "source generators are only defined through degree {}",
                    self.0.source().max_computed_degree()
                )));
            }
            let num_gens = self.source_num_gens(degree);
            let output_degree = self.output_degree(degree)?;
            self.ensure_through(degree, output_degree);
            let out_dim = self.target_dim(output_degree);
            let m = matrix.as_rust();
            checked_same_prime(m.prime().as_u32(), p)?;
            if m.rows() < num_gens {
                return Err(PyValueError::new_err(format!(
                    "matrix has {} rows but {num_gens} generators in degree {degree}",
                    m.rows()
                )));
            }
            if out_dim != 0 && m.columns() != out_dim {
                return Err(PyValueError::new_err(format!(
                    "matrix has {} columns but the target degree has dimension {out_dim}",
                    m.columns()
                )));
            }
            let mut owned = m.clone();
            self.0
                .add_generators_from_matrix_rows(degree, owned.as_slice_mut());
            Ok(())
        }

        /// The average density (fraction of nonzero entries) of the output
        /// vectors on the generators in `degree`. Returns `nan` if there are no
        /// generators in `degree`. Requires the outputs in `degree` to be
        /// defined.
        pub fn differential_density(&self, degree: i32) -> PyResult<f32> {
            if degree < self.0.min_degree() || degree >= self.0.next_degree() {
                return Err(PyValueError::new_err(format!(
                    "outputs are not defined in degree {degree} (defined for {}..{})",
                    self.0.min_degree(),
                    self.0.next_degree()
                )));
            }
            Ok(self.0.differential_density(degree))
        }

        /// Manually set the cached image in `degree`. `degree` must be the next
        /// undefined image degree (consecutive `OnceVec` push).
        pub fn set_image(
            &self,
            degree: i32,
            image: Option<PyRef<'_, crate::fp_py::PySubspace>>,
        ) -> PyResult<()> {
            if degree != self.0.images.len() {
                return Err(PyValueError::new_err(format!(
                    "image must be set consecutively: expected degree {}, got {degree}",
                    self.0.images.len()
                )));
            }
            self.0.set_image(degree, image.map(|s| s.as_rust().clone()));
            Ok(())
        }

        /// Manually set the cached kernel in `degree`. `degree` must be the next
        /// undefined kernel degree.
        pub fn set_kernel(
            &self,
            degree: i32,
            kernel: Option<PyRef<'_, crate::fp_py::PySubspace>>,
        ) -> PyResult<()> {
            if degree != self.0.kernels.len() {
                return Err(PyValueError::new_err(format!(
                    "kernel must be set consecutively: expected degree {}, got {degree}",
                    self.0.kernels.len()
                )));
            }
            self.0
                .set_kernel(degree, kernel.map(|s| s.as_rust().clone()));
            Ok(())
        }

        /// Manually set the cached quasi-inverse in `degree`. `degree` must be
        /// the next undefined quasi-inverse degree.
        pub fn set_quasi_inverse(
            &self,
            degree: i32,
            quasi_inverse: Option<PyRef<'_, crate::fp_py::PyQuasiInverse>>,
        ) -> PyResult<()> {
            if degree != self.0.quasi_inverses.len() {
                return Err(PyValueError::new_err(format!(
                    "quasi-inverse must be set consecutively: expected degree {}, got {degree}",
                    self.0.quasi_inverses.len()
                )));
            }
            self.0
                .set_quasi_inverse(degree, quasi_inverse.map(|qi| qi.as_rust().clone()));
            Ok(())
        }

        pub fn __repr__(&self) -> String {
            format!(
                "FreeModuleHomomorphism(source={}, target={}, degree_shift={})",
                self.0.source(),
                self.0.target(),
                self.0.degree_shift()
            )
        }
    }

    /// A `FreeModuleHomomorphism` whose target is itself a concrete `FreeModule`
    /// (the free → free variant), i.e. a map `F -> G` between two free modules
    /// over the same Steenrod algebra. This is the variant `HomPullback` needs;
    /// it is distinct from `FreeModuleHomomorphism` (whose target is an arbitrary
    /// boxed `SteenrodModule`).
    ///
    /// Both the source and target are the bound `FreeModule` pyclass and share
    /// their `Arc`-held state with this homomorphism (the `source()`/`target()`
    /// accessors hand back the same underlying module, not a copy).
    ///
    /// Because the target is a concrete `FreeModule`, this variant additionally
    /// exposes `hom_k` (the dual map on the generators / cohomology), which
    /// upstream gates on a `FreeModule` target.
    ///
    /// Every degree-indexed access is pre-checked so that an uncomputed degree,
    /// out-of-range index, prime/length mismatch, or non-consecutive mutation
    /// raises `ValueError`/`IndexError` rather than panicking across the FFI
    /// boundary. The internal `outputs`/`images`/`kernels`/`quasi_inverses`
    /// tables use interior mutability (`OnceBiVec`), so every method takes
    /// `&self`; the homomorphism itself is held behind an `Arc` so the *same*
    /// instance can be shared into a `HomPullback`.
    #[pyclass(name = "FreeModuleHomomorphismToFree")]
    pub struct FreeModuleHomomorphismToFree(Arc<FreeModuleHomToFreeInner>);

    impl FreeModuleHomomorphismToFree {
        /// Wrap an existing `Arc<FreeModuleHomomorphism<FreeModule>>`, sharing
        /// the *same* underlying homomorphism (a cheap refcount bump). Exposed
        /// `pub(crate)` so sibling binding modules can hand back maps they hold
        /// behind an `Arc` without copying — in particular
        /// `ResolutionHomomorphism.get_map`, whose upstream `get_map(s)` returns
        /// exactly `Arc<MuFreeModuleHomomorphism<false, FreeModule>>`. Mirrors
        /// the `FreeModule::from_arc` Arc-sharing precedent.
        pub(crate) fn from_arc(inner: Arc<FreeModuleHomToFreeInner>) -> Self {
            FreeModuleHomomorphismToFree(inner)
        }

        /// Dimension of the source `FreeModule` in `degree` (guarded; computes
        /// the basis first and reads 0 below `min_degree`).
        fn source_dim(&self, degree: i32) -> usize {
            module_dimension(&*self.0.source() as &DynModule, degree)
        }

        /// Dimension of the target `FreeModule` in `degree` (guarded).
        fn target_dim(&self, degree: i32) -> usize {
            module_dimension(&*self.0.target() as &DynModule, degree)
        }

        /// Ensure both the source basis through `input_degree` and the target
        /// basis through `output_degree` are computed.
        fn ensure_through(&self, input_degree: i32, output_degree: i32) {
            module_ensure(&*self.0.source() as &DynModule, input_degree);
            module_ensure(&*self.0.target() as &DynModule, output_degree);
        }

        /// `input_degree - degree_shift`, raising `ValueError` on overflow.
        fn output_degree(&self, input_degree: i32) -> PyResult<i32> {
            input_degree
                .checked_sub(self.0.degree_shift())
                .ok_or_else(|| PyValueError::new_err("output degree overflows i32"))
        }

        /// Number of generators of the source in `degree`, reading 0 (never
        /// panicking) outside the populated generator range.
        fn source_num_gens(&self, degree: i32) -> usize {
            fm_num_gens_safe(&self.0.source(), degree)
        }

        /// See `FreeModuleHomomorphism::check_outputs_cover`.
        fn check_outputs_cover(&self, hi: i32) -> PyResult<()> {
            let source = self.0.source();
            let lo = self.0.next_degree().max(source.min_degree());
            let hi = hi.min(source.max_computed_degree());
            for d in lo..=hi {
                if source.number_of_gens_in_degree(d) > 0 {
                    return Err(PyValueError::new_err(format!(
                        "the homomorphism's outputs are not defined on the source generators in \
                         degree {d}; define them (extend_by_zero / add_generators_from_rows) up to \
                         degree {hi} first"
                    )));
                }
            }
            Ok(())
        }

        /// See `FreeModuleHomomorphism::check_basis_element_defined`.
        fn check_basis_element_defined(&self, input_degree: i32, input_idx: usize) -> PyResult<()> {
            let source = self.0.source();
            let generator_degree = source
                .index_to_op_gen(input_degree, input_idx)
                .generator_degree;
            if generator_degree >= self.0.next_degree() {
                return Err(PyValueError::new_err(format!(
                    "the homomorphism's output is not defined on the source generator in degree \
                     {generator_degree} (define it with add_generators_from_rows / extend_by_zero \
                     first)"
                )));
            }
            Ok(())
        }
    }

    #[pymethods]
    impl FreeModuleHomomorphismToFree {
        /// Build the zero homomorphism `source -> target` (both `FreeModule`s)
        /// with the given `degree_shift` (`output_degree = input_degree -
        /// degree_shift`). The outputs on generators are all unset; populate them
        /// with `add_generators_from_rows`/`add_generators_from_matrix_rows`/
        /// `extend_by_zero`. The factors must be built over the *same* algebra
        /// object (checked by prime and `Arc` identity).
        #[new]
        #[pyo3(signature = (source, target, degree_shift = 0))]
        pub fn new(
            source: PyRef<'_, FreeModule>,
            target: PyRef<'_, FreeModule>,
            degree_shift: i32,
        ) -> PyResult<Self> {
            let source_alg = source.0.algebra();
            let target_alg = target.0.algebra();
            checked_same_prime(source_alg.prime().as_u32(), target_alg.prime().as_u32())?;
            if !Arc::ptr_eq(&source_alg, &target_alg) {
                return Err(PyValueError::new_err(
                    "source and target must be built over the same algebra",
                ));
            }
            Ok(FreeModuleHomomorphismToFree(Arc::new(
                FreeModuleHomToFreeInner::new(
                    Arc::clone(&source.0),
                    Arc::clone(&target.0),
                    degree_shift,
                ),
            )))
        }

        // --- flattened ModuleHomomorphism method set --------------------------

        /// The source `FreeModule` (shares state via `Arc`).
        pub fn source(&self) -> FreeModule {
            FreeModule(self.0.source())
        }

        /// The target `FreeModule` (shares state via `Arc`).
        pub fn target(&self) -> FreeModule {
            FreeModule(self.0.target())
        }

        /// The degree shift: `output_degree = input_degree - degree_shift`.
        pub fn degree_shift(&self) -> i32 {
            self.0.degree_shift()
        }

        /// The smallest input degree the homomorphism is defined on,
        /// `max(source.min_degree(), target.min_degree() + degree_shift)`.
        pub fn min_degree(&self) -> i32 {
            self.0.min_degree()
        }

        /// The prime as a plain `int` (`ValidPrime` is never exposed).
        pub fn prime(&self) -> u32 {
            self.0.prime().as_u32()
        }

        /// Apply the homomorphism to the basis element `input_idx` in
        /// `input_degree`, adding `coeff` times its image into `result` (a
        /// vector of length `target.dimension(input_degree - degree_shift)`).
        pub fn apply_to_basis_element(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            input_degree: i32,
            input_idx: usize,
        ) -> PyResult<()> {
            let p = self.0.prime().as_u32();
            let coeff = coeff % p;
            if input_degree < self.0.source().min_degree() {
                return Err(PyIndexError::new_err(format!(
                    "input degree {input_degree} is below the source min_degree {}",
                    self.0.source().min_degree()
                )));
            }
            let output_degree = self.output_degree(input_degree)?;
            self.ensure_through(input_degree, output_degree);
            let src_dim = self.source_dim(input_degree);
            if input_idx >= src_dim {
                return Err(PyIndexError::new_err(format!(
                    "input index {input_idx} out of range for source degree {input_degree} \
                     (dimension {src_dim})"
                )));
            }
            self.check_basis_element_defined(input_degree, input_idx)?;
            let out_dim = self.target_dim(output_degree);
            crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                checked_same_prime(res.prime().as_u32(), p)?;
                checked_equal_len(res.as_slice().len(), out_dim)?;
                self.0
                    .apply_to_basis_element(res.copy(), coeff, input_degree, input_idx);
                Ok(())
            })
        }

        /// Apply the homomorphism to a general `input` element of `source` in
        /// `input_degree` (length `source.dimension(input_degree)`), adding
        /// `coeff` times its image into `result`. Aliasing the same vector as
        /// both `input` and `result` raises `RuntimeError` (the borrow
        /// conflict).
        pub fn apply(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            input_degree: i32,
            input: &Bound<'_, PyAny>,
        ) -> PyResult<()> {
            let p = self.0.prime().as_u32();
            let coeff = coeff % p;
            if input_degree < self.0.source().min_degree() {
                return Err(PyIndexError::new_err(format!(
                    "input degree {input_degree} is below the source min_degree {}",
                    self.0.source().min_degree()
                )));
            }
            let output_degree = self.output_degree(input_degree)?;
            self.ensure_through(input_degree, output_degree);
            let src_dim = self.source_dim(input_degree);
            let out_dim = self.target_dim(output_degree);
            crate::fp_py::with_input_slice(py, input, |in_slice| {
                checked_same_prime(in_slice.prime().as_u32(), p)?;
                checked_equal_len(in_slice.len(), src_dim)?;
                for (i, _) in in_slice.iter_nonzero() {
                    self.check_basis_element_defined(input_degree, i)?;
                }
                crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                    checked_same_prime(res.prime().as_u32(), p)?;
                    checked_equal_len(res.as_slice().len(), out_dim)?;
                    self.0.apply(res.copy(), coeff, input_degree, in_slice);
                    Ok(())
                })
            })
        }

        /// The kernel of the homomorphism in `degree`, if it has been computed
        /// (via `compute_auxiliary_data_through_degree` or `set_kernel`).
        /// Returns `None` otherwise (never panics).
        pub fn kernel(&self, degree: i32) -> Option<crate::fp_py::PySubspace> {
            self.0
                .kernel(degree)
                .map(|s| crate::fp_py::PySubspace::from_rust(s.clone()))
        }

        /// The image of the homomorphism in `degree`, if it has been computed.
        pub fn image(&self, degree: i32) -> Option<crate::fp_py::PySubspace> {
            self.0
                .image(degree)
                .map(|s| crate::fp_py::PySubspace::from_rust(s.clone()))
        }

        /// The quasi-inverse of the homomorphism in `degree`, if it has been
        /// computed.
        pub fn quasi_inverse(&self, degree: i32) -> Option<crate::fp_py::PyQuasiInverse> {
            self.0
                .quasi_inverse(degree)
                .map(|qi| crate::fp_py::PyQuasiInverse::from_rust(qi.clone()))
        }

        /// Compute (and cache) the image, kernel and quasi-inverse at every
        /// input degree up to `degree`. Requires the outputs on generators to be
        /// defined through `degree` (otherwise raises `ValueError`); also raises
        /// `ValueError` if a previous manual `set_image`/`set_kernel`/
        /// `set_quasi_inverse` has left the three auxiliary tables out of sync.
        pub fn compute_auxiliary_data_through_degree(&self, degree: i32) -> PyResult<()> {
            let kernels_len = self.0.kernels.len();
            if degree >= kernels_len {
                self.check_outputs_cover(degree)?;
            }
            if self.0.images.len() != kernels_len || self.0.quasi_inverses.len() != kernels_len {
                return Err(PyValueError::new_err(
                    "auxiliary data tables are out of sync (a prior set_image/set_kernel/\
                     set_quasi_inverse advanced them unequally); cannot compute",
                ));
            }
            self.0.compute_auxiliary_data_through_degree(degree);
            Ok(())
        }

        /// The matrix whose rows are the images of the source basis elements
        /// `inputs` in `degree`. Columns index `target.dimension(degree)`.
        ///
        /// As with `FreeModuleHomomorphism`, this is only well-defined when
        /// `target.dimension(degree) == target.dimension(degree - degree_shift)`
        /// (always so for `degree_shift == 0`); otherwise it raises `ValueError`.
        /// An out-of-range / uncomputed target degree reads as dimension 0 and
        /// yields the empty (`len(inputs) x 0`) matrix instead of panicking.
        pub fn get_partial_matrix(
            &self,
            degree: i32,
            inputs: Vec<usize>,
        ) -> PyResult<crate::fp_py::PyMatrix> {
            if degree < self.0.source().min_degree() {
                return Err(PyIndexError::new_err(format!(
                    "degree {degree} is below the source min_degree {}",
                    self.0.source().min_degree()
                )));
            }
            let output_degree = self.output_degree(degree)?;
            self.ensure_through(degree, output_degree);
            let src_dim = self.source_dim(degree);
            for &i in &inputs {
                if i >= src_dim {
                    return Err(PyIndexError::new_err(format!(
                        "input index {i} out of range for source degree {degree} (dimension \
                         {src_dim})"
                    )));
                }
            }
            self.check_outputs_cover(degree)?;
            let tgt_dim = self.target_dim(degree);
            if tgt_dim == 0 {
                return Ok(crate::fp_py::PyMatrix::from_rust(fp::matrix::Matrix::new(
                    self.0.prime(),
                    inputs.len(),
                    0,
                )));
            }
            if tgt_dim != self.target_dim(output_degree) {
                return Err(PyValueError::new_err(
                    "get_partial_matrix is only well-defined when target.dimension(degree) == \
                     target.dimension(degree - degree_shift) (e.g. degree_shift == 0)",
                ));
            }
            Ok(crate::fp_py::PyMatrix::from_rust(
                self.0.get_partial_matrix(degree, &inputs),
            ))
        }

        /// Apply the quasi-inverse at `degree` to `input`, adding the result
        /// into `result`. Returns `True` if the quasi-inverse was available (and
        /// applied), `False` otherwise.
        pub fn apply_quasi_inverse(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            degree: i32,
            input: &Bound<'_, PyAny>,
        ) -> PyResult<bool> {
            let p = self.0.prime().as_u32();
            let Some(qi) = self.0.quasi_inverse(degree) else {
                return Ok(false);
            };
            let source_dim = qi.source_dimension();
            let target_dim = qi.target_dimension();
            crate::fp_py::with_input_slice(py, input, |in_slice| {
                checked_same_prime(in_slice.prime().as_u32(), p)?;
                checked_equal_len(in_slice.len(), target_dim)?;
                crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                    checked_same_prime(res.prime().as_u32(), p)?;
                    checked_equal_len(res.as_slice().len(), source_dim)?;
                    qi.apply(res.copy(), 1, in_slice);
                    Ok(())
                })
            })?;
            Ok(true)
        }

        // --- FreeModuleHomomorphism-specific methods --------------------------

        /// The first input degree whose outputs on generators have *not* yet
        /// been defined (i.e. the length of the `outputs` table).
        pub fn next_degree(&self) -> i32 {
            self.0.next_degree()
        }

        /// The image of the generator `(generator_degree, generator_index)`, a
        /// vector of length `target.dimension(generator_degree - degree_shift)`.
        pub fn output(
            &self,
            generator_degree: i32,
            generator_index: usize,
        ) -> PyResult<crate::fp_py::PyFpVector> {
            if generator_degree < self.0.min_degree() {
                return Err(PyIndexError::new_err(format!(
                    "generator degree {generator_degree} is below min_degree {}",
                    self.0.min_degree()
                )));
            }
            if generator_degree >= self.0.next_degree() {
                return Err(PyValueError::new_err(format!(
                    "outputs are only defined through degree {} (add generators / extend_by_zero \
                     first)",
                    self.0.next_degree() - 1
                )));
            }
            let num_gens = self.source_num_gens(generator_degree);
            if generator_index >= num_gens {
                return Err(PyIndexError::new_err(format!(
                    "generator index {generator_index} out of range in degree {generator_degree} \
                     ({num_gens} generators)"
                )));
            }
            Ok(crate::fp_py::PyFpVector::from_rust(
                self.0.output(generator_degree, generator_index).clone(),
            ))
        }

        /// Apply the homomorphism to the generator `idx` in `degree`, adding
        /// `coeff` times its image (`output(degree, idx)`) into `result`.
        pub fn apply_to_generator(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            degree: i32,
            idx: usize,
        ) -> PyResult<()> {
            let p = self.0.prime().as_u32();
            let coeff = coeff % p;
            if degree < self.0.min_degree() {
                return Err(PyIndexError::new_err(format!(
                    "generator degree {degree} is below min_degree {}",
                    self.0.min_degree()
                )));
            }
            if degree >= self.0.next_degree() {
                return Err(PyValueError::new_err(format!(
                    "outputs are only defined through degree {} (add generators / extend_by_zero \
                     first)",
                    self.0.next_degree() - 1
                )));
            }
            let num_gens = self.source_num_gens(degree);
            if idx >= num_gens {
                return Err(PyIndexError::new_err(format!(
                    "generator index {idx} out of range in degree {degree} ({num_gens} generators)"
                )));
            }
            let output_degree = self.output_degree(degree)?;
            let out_dim = self.target_dim(output_degree);
            crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                checked_same_prime(res.prime().as_u32(), p)?;
                checked_equal_len(res.as_slice().len(), out_dim)?;
                res.add(self.0.output(degree, idx).as_slice(), coeff);
                Ok(())
            })
        }

        /// Set the outputs on the generators in `degree` to zero, extending the
        /// `outputs` table up to `degree`.
        pub fn extend_by_zero(&self, degree: i32) -> PyResult<()> {
            if degree >= self.0.next_degree() && degree > self.0.source().max_computed_degree() {
                return Err(PyValueError::new_err(format!(
                    "source generators are only defined through degree {} (cannot extend \
                     outputs to degree {degree})",
                    self.0.source().max_computed_degree()
                )));
            }
            let output_degree = self.output_degree(degree)?;
            self.ensure_through(degree, output_degree);
            self.0.extend_by_zero(degree);
            Ok(())
        }

        /// Define the outputs on the generators in `degree` from `rows`, one
        /// vector per generator (each of length `target.dimension(degree -
        /// degree_shift)`). `degree` must be the next undefined degree.
        pub fn add_generators_from_rows(
            &self,
            py: Python<'_>,
            degree: i32,
            rows: Vec<Bound<'_, PyAny>>,
        ) -> PyResult<()> {
            let p = self.0.prime().as_u32();
            if degree != self.0.next_degree() {
                return Err(PyValueError::new_err(format!(
                    "generators must be added consecutively: expected degree {}, got {degree}",
                    self.0.next_degree()
                )));
            }
            if degree > self.0.source().max_computed_degree() {
                return Err(PyValueError::new_err(format!(
                    "source generators are only defined through degree {}",
                    self.0.source().max_computed_degree()
                )));
            }
            let num_gens = self.source_num_gens(degree);
            if rows.len() != num_gens {
                return Err(PyValueError::new_err(format!(
                    "expected {num_gens} rows (one per generator in degree {degree}), got {}",
                    rows.len()
                )));
            }
            let output_degree = self.output_degree(degree)?;
            self.ensure_through(degree, output_degree);
            let out_dim = self.target_dim(output_degree);
            let mut owned: Vec<::fp::vector::FpVector> = Vec::with_capacity(rows.len());
            for row in &rows {
                let vec = crate::fp_py::extract_input_owned(py, row)?;
                checked_same_prime(vec.prime().as_u32(), p)?;
                checked_equal_len(vec.len(), out_dim)?;
                owned.push(vec);
            }
            self.0.add_generators_from_rows(degree, owned);
            Ok(())
        }

        /// Define the outputs on the generators in `degree` from the rows of
        /// `matrix` (the first `num_gens` rows are used). `degree` must be the
        /// next undefined degree.
        pub fn add_generators_from_matrix_rows(
            &self,
            degree: i32,
            matrix: PyRef<'_, crate::fp_py::PyMatrix>,
        ) -> PyResult<()> {
            let p = self.0.prime().as_u32();
            if degree != self.0.next_degree() {
                return Err(PyValueError::new_err(format!(
                    "generators must be added consecutively: expected degree {}, got {degree}",
                    self.0.next_degree()
                )));
            }
            if degree > self.0.source().max_computed_degree() {
                return Err(PyValueError::new_err(format!(
                    "source generators are only defined through degree {}",
                    self.0.source().max_computed_degree()
                )));
            }
            let num_gens = self.source_num_gens(degree);
            let output_degree = self.output_degree(degree)?;
            self.ensure_through(degree, output_degree);
            let out_dim = self.target_dim(output_degree);
            let m = matrix.as_rust();
            checked_same_prime(m.prime().as_u32(), p)?;
            if m.rows() < num_gens {
                return Err(PyValueError::new_err(format!(
                    "matrix has {} rows but {num_gens} generators in degree {degree}",
                    m.rows()
                )));
            }
            if out_dim != 0 && m.columns() != out_dim {
                return Err(PyValueError::new_err(format!(
                    "matrix has {} columns but the target degree has dimension {out_dim}",
                    m.columns()
                )));
            }
            let mut owned = m.clone();
            self.0
                .add_generators_from_matrix_rows(degree, owned.as_slice_mut());
            Ok(())
        }

        /// The average density (fraction of nonzero entries) of the output
        /// vectors on the generators in `degree`. Returns `nan` if there are no
        /// generators in `degree`. Requires the outputs in `degree` to be
        /// defined.
        pub fn differential_density(&self, degree: i32) -> PyResult<f32> {
            if degree < self.0.min_degree() || degree >= self.0.next_degree() {
                return Err(PyValueError::new_err(format!(
                    "outputs are not defined in degree {degree} (defined for {}..{})",
                    self.0.min_degree(),
                    self.0.next_degree()
                )));
            }
            Ok(self.0.differential_density(degree))
        }

        /// Manually set the cached image in `degree` (consecutive `OnceVec` push).
        pub fn set_image(
            &self,
            degree: i32,
            image: Option<PyRef<'_, crate::fp_py::PySubspace>>,
        ) -> PyResult<()> {
            if degree != self.0.images.len() {
                return Err(PyValueError::new_err(format!(
                    "image must be set consecutively: expected degree {}, got {degree}",
                    self.0.images.len()
                )));
            }
            self.0.set_image(degree, image.map(|s| s.as_rust().clone()));
            Ok(())
        }

        /// Manually set the cached kernel in `degree` (consecutive `OnceVec` push).
        pub fn set_kernel(
            &self,
            degree: i32,
            kernel: Option<PyRef<'_, crate::fp_py::PySubspace>>,
        ) -> PyResult<()> {
            if degree != self.0.kernels.len() {
                return Err(PyValueError::new_err(format!(
                    "kernel must be set consecutively: expected degree {}, got {degree}",
                    self.0.kernels.len()
                )));
            }
            self.0
                .set_kernel(degree, kernel.map(|s| s.as_rust().clone()));
            Ok(())
        }

        /// Manually set the cached quasi-inverse in `degree` (consecutive
        /// `OnceVec` push).
        pub fn set_quasi_inverse(
            &self,
            degree: i32,
            quasi_inverse: Option<PyRef<'_, crate::fp_py::PyQuasiInverse>>,
        ) -> PyResult<()> {
            if degree != self.0.quasi_inverses.len() {
                return Err(PyValueError::new_err(format!(
                    "quasi-inverse must be set consecutively: expected degree {}, got {degree}",
                    self.0.quasi_inverses.len()
                )));
            }
            self.0
                .set_quasi_inverse(degree, quasi_inverse.map(|qi| qi.as_rust().clone()));
            Ok(())
        }

        /// The dual map on generators in source degree `t`: given `f: F -> G`,
        /// computes `f*: Hom(G, k) -> Hom(F, k)` as the matrix (rows indexed by
        /// `G`'s generators in degree `t`, columns by `F`'s generators in degree
        /// `t + degree_shift`). Only available on this free → free variant
        /// (upstream gates `hom_k` on a `FreeModule` target). Returns an empty
        /// list when the target has no generators in degree `t` (including when
        /// `t` is above the target's computed range, which morally has 0
        /// generators). When the source has no generators in degree
        /// `t + degree_shift` (including degrees above the source's computed
        /// range), the dual matrix has `target_dim` rows of length 0, matching
        /// upstream's `vec![vec![0; source_dim]; target_dim]` with `source_dim ==
        /// 0`.
        ///
        /// Guards: the relevant source/target generator degrees must have their
        /// bases computed (done here) and, when the source has generators in
        /// degree `t + degree_shift`, their outputs must be defined (otherwise
        /// `ValueError`). Out-of-computed-range degrees never panic.
        pub fn hom_k(&self, t: i32) -> PyResult<Vec<Vec<u32>>> {
            let degree_shift = self.0.degree_shift();
            let gen_degree = t
                .checked_add(degree_shift)
                .ok_or_else(|| PyValueError::new_err("input degree overflows i32"))?;
            let source = self.0.source();
            let target = self.0.target();
            module_ensure(&*source as &DynModule, gen_degree);
            module_ensure(&*target as &DynModule, t);
            // Upstream `hom_k` reads `target.number_of_gens_in_degree(t)` before
            // any early return. That read PANICS (OnceBiVec index) when
            // `t > target.max_computed_degree()`; `fm_num_gens_safe` returns 0
            // there instead, and upstream returns `vec![]` when the target has no
            // generators in degree `t`. An uncomputed target degree morally has 0
            // generators, so `vec![]` is the correct (and safe) result.
            let target_dim = fm_num_gens_safe(&target, t);
            if target_dim == 0 {
                return Ok(vec![]);
            }
            // Upstream then reads `source.number_of_gens_in_degree(gen_degree)`,
            // which likewise PANICS for `gen_degree > source.max_computed_degree()`
            // (and returns 0 below `source.min_degree()`). In either case the
            // source has no generators in `gen_degree`, so `source_dim` is morally
            // 0 and upstream's result `vec![vec![0; source_dim]; target_dim]` is
            // `target_dim` empty rows. Return that directly to match upstream
            // without tripping the out-of-bounds index.
            if gen_degree < source.min_degree() || gen_degree > source.max_computed_degree() {
                return Ok(vec![Vec::new(); target_dim]);
            }
            let source_dim = fm_num_gens_safe(&source, gen_degree);
            if source_dim > 0
                && (gen_degree < self.0.min_degree() || gen_degree >= self.0.next_degree())
            {
                return Err(PyValueError::new_err(format!(
                    "the homomorphism's outputs are not defined on the source generators in degree \
                     {gen_degree}; define them (add_generators_from_rows / extend_by_zero) first"
                )));
            }
            Ok(self.0.hom_k(t))
        }

        pub fn __repr__(&self) -> String {
            format!(
                "FreeModuleHomomorphismToFree(source={}, target={}, degree_shift={})",
                self.0.source(),
                self.0.target(),
                self.0.degree_shift()
            )
        }
    }

    /// A `ModuleHomomorphism` `f: S -> M` that simply records its matrix in
    /// every degree (`output_degree = input_degree - degree_shift`). Both the
    /// source and target are arbitrary modules, accepted (and returned) as the
    /// bound `SteenrodModule` pyclass; both share their `Arc`-held state with
    /// this homomorphism. Box a concrete module with `.into_steenrod_module()`
    /// first.
    ///
    /// Unlike `FreeModuleHomomorphism`, an unspecified matrix is treated as
    /// zero, so applying the map to any degree never requires the outputs to be
    /// "defined" — undefined degrees simply contribute nothing. Every
    /// degree-indexed access is still pre-checked so that an out-of-range index,
    /// prime/length mismatch, or unbounded `identity`/`from_matrices` request
    /// raises `ValueError`/`IndexError` rather than panicking across the FFI
    /// boundary. The internal `matrices`/`images`/`kernels`/`quasi_inverses`
    /// tables use interior mutability (`OnceBiVec`), so every method takes
    /// `&self`.
    ///
    /// NOTE (deferred to later §5.4 tasks): `FullModuleHomomorphism::from`,
    /// `replace_source` and `replace_target` are *not* bound here. `from<F>`
    /// converts another `ModuleHomomorphism` with the *same* `Source`/`Target`
    /// type parameters into a `FullModuleHomomorphism`; with the single
    /// `<RsSteenrodModule, RsSteenrodModule>` monomorphisation the only
    /// reachable conversion is from another `FullModuleHomomorphism` (i.e. a
    /// plain clone), while the useful conversions (e.g. from a
    /// `FreeModuleHomomorphism`, whose `Source` is a concrete `FreeModule`)
    /// require additional monomorphisations not bound in this task.
    /// `replace_source`/`replace_target` only change a *type parameter* (not the
    /// mathematical module) and consume `self` by value; with one
    /// monomorphisation there is no distinct type to replace into, so they are
    /// likewise deferred until those other module-typed homomorphisms exist.
    #[pyclass(name = "FullModuleHomomorphism")]
    pub struct FullModuleHomomorphism(FullModuleHomomorphismInner);

    impl FullModuleHomomorphism {
        /// Wrap an upstream `FullModuleHomomorphism<SteenrodModule>` (the
        /// differential type of `CCC`). Used by the `ext` chain-complex
        /// bindings; the inner value is cloned out of its `Arc` (cheap: the
        /// recorded matrices are `Arc`-shared).
        pub(crate) fn from_rust(inner: FullModuleHomomorphismInner) -> Self {
            FullModuleHomomorphism(inner)
        }

        /// Clone the underlying upstream homomorphism out of the pyclass (cheap:
        /// the recorded matrices are `Arc`-shared). Used by `ChainComplex.new`.
        pub(crate) fn clone_rust(&self) -> FullModuleHomomorphismInner {
            self.0.clone()
        }

        /// The algebra of the source module (`Arc`-shared). Used by
        /// `ChainComplex.new` to check a differential is built over the same
        /// algebra as the complex's modules (via `Arc::ptr_eq`).
        pub(crate) fn source_algebra(&self) -> Arc<RsSteenrodAlgebra> {
            self.0.source().algebra()
        }

        /// The algebra of the target module (`Arc`-shared); see
        /// [`Self::source_algebra`].
        pub(crate) fn target_algebra(&self) -> Arc<RsSteenrodAlgebra> {
            self.0.target().algebra()
        }

        /// `min_degree()` of the source module (the smallest input degree).
        fn source_min_degree(&self) -> i32 {
            self.0.source().min_degree()
        }

        /// Dimension of the source module in `degree` (guarded; computes the
        /// basis first and reads 0 below `min_degree`).
        fn source_dim(&self, degree: i32) -> usize {
            module_dimension(&**self.0.source() as &DynModule, degree)
        }

        /// Dimension of the target module in `degree` (guarded).
        fn target_dim(&self, degree: i32) -> usize {
            module_dimension(&**self.0.target() as &DynModule, degree)
        }

        /// Ensure both the source basis through `input_degree` and the target
        /// basis through `output_degree` are computed (algebra + module).
        fn ensure_through(&self, input_degree: i32, output_degree: i32) {
            module_ensure(&**self.0.source() as &DynModule, input_degree);
            module_ensure(&**self.0.target() as &DynModule, output_degree);
        }

        /// `input_degree - degree_shift`, raising `ValueError` on overflow.
        fn output_degree(&self, input_degree: i32) -> PyResult<i32> {
            input_degree
                .checked_sub(self.0.degree_shift())
                .ok_or_else(|| PyValueError::new_err("output degree overflows i32"))
        }

        /// Validate that `source` and `target` are built over the *same* algebra
        /// object (checked by prime and `Arc` identity, like `TensorModule`),
        /// raising `ValueError` otherwise.
        fn check_same_algebra(
            source: &RsSteenrodModule,
            target: &RsSteenrodModule,
        ) -> PyResult<()> {
            let source_alg = source.algebra();
            let target_alg = target.algebra();
            checked_same_prime(source_alg.prime().as_u32(), target_alg.prime().as_u32())?;
            if !Arc::ptr_eq(&source_alg, &target_alg) {
                return Err(PyValueError::new_err(
                    "source and target must be built over the same algebra",
                ));
            }
            Ok(())
        }
    }

    #[pymethods]
    impl FullModuleHomomorphism {
        /// Build the zero homomorphism `source -> target` with the given
        /// `degree_shift` (every recorded matrix is absent, i.e. zero). The
        /// factors must be built over the *same* algebra object.
        #[new]
        #[pyo3(signature = (source, target, degree_shift = 0))]
        pub fn new(
            source: PyRef<'_, SteenrodModule>,
            target: PyRef<'_, SteenrodModule>,
            degree_shift: i32,
        ) -> PyResult<Self> {
            Self::check_same_algebra(&source.0, &target.0)?;
            Ok(FullModuleHomomorphism(FullModuleHomomorphismInner::new(
                Arc::new(source.0.clone()),
                Arc::new(target.0.clone()),
                degree_shift,
            )))
        }

        /// Build a `FullModuleHomomorphism` from explicit per-degree matrices.
        /// `matrices[i]` is the matrix in *output* degree `min_degree + i`
        /// (defaulting `min_degree` to `target.min_degree()`); its rows index
        /// the source basis in degree `min_degree + i + degree_shift` and its
        /// columns index the target basis in degree `min_degree + i`. Each
        /// matrix's prime and both dimensions are validated against the modules
        /// (raising `ValueError`) so that later `apply`/auxiliary-data
        /// computations can never index a row/column out of range.
        #[staticmethod]
        #[pyo3(signature = (source, target, matrices, degree_shift = 0, min_degree = None))]
        pub fn from_matrices(
            source: PyRef<'_, SteenrodModule>,
            target: PyRef<'_, SteenrodModule>,
            matrices: Vec<PyRef<'_, crate::fp_py::PyMatrix>>,
            degree_shift: i32,
            min_degree: Option<i32>,
        ) -> PyResult<Self> {
            Self::check_same_algebra(&source.0, &target.0)?;
            let p = source.0.prime().as_u32();
            let target_min = target.0.min_degree();
            let min_degree = min_degree.unwrap_or(target_min);
            // Upstream `FullModuleHomomorphism::from_matrices` always builds the
            // kernels/images/quasi_inverses `OnceBiVec`s starting at
            // `target.min_degree()`, regardless of the `matrices` BiVec's min.
            // A `min_degree` below `target.min_degree()` would therefore record
            // matrices whose auxiliary data is never computed — a silent
            // correctness surprise. Reject it up front.
            if min_degree < target_min {
                return Err(PyValueError::new_err(format!(
                    "min_degree {min_degree} is below the target min_degree {target_min}; \
                     auxiliary data is only computed at and above target.min_degree()"
                )));
            }
            let mut bivec = ::bivec::BiVec::new(min_degree);
            for (offset, m) in matrices.iter().enumerate() {
                let offset = i32::try_from(offset)
                    .map_err(|_| PyValueError::new_err("too many matrices"))?;
                let output_degree = min_degree
                    .checked_add(offset)
                    .ok_or_else(|| PyValueError::new_err("output degree overflows i32"))?;
                let input_degree = output_degree
                    .checked_add(degree_shift)
                    .ok_or_else(|| PyValueError::new_err("input degree overflows i32"))?;
                module_ensure(&*source.0 as &DynModule, input_degree);
                module_ensure(&*target.0 as &DynModule, output_degree);
                let src_dim = module_dimension(&*source.0 as &DynModule, input_degree);
                let tgt_dim = module_dimension(&*target.0 as &DynModule, output_degree);
                let m = m.as_rust();
                checked_same_prime(m.prime().as_u32(), p)?;
                if m.rows() != src_dim {
                    return Err(PyValueError::new_err(format!(
                        "matrix for output degree {output_degree} has {} rows but the source \
                         degree {input_degree} has dimension {src_dim}",
                        m.rows()
                    )));
                }
                if m.columns() != tgt_dim {
                    return Err(PyValueError::new_err(format!(
                        "matrix for output degree {output_degree} has {} columns but the target \
                         degree {output_degree} has dimension {tgt_dim}",
                        m.columns()
                    )));
                }
                bivec.push(m.clone());
            }
            Ok(FullModuleHomomorphism(
                FullModuleHomomorphismInner::from_matrices(
                    Arc::new(source.0.clone()),
                    Arc::new(target.0.clone()),
                    degree_shift,
                    bivec,
                ),
            ))
        }

        /// The zero homomorphism `source -> target` with the given
        /// `degree_shift` (the `ZeroHomomorphism` constructor). Identical to the
        /// `new` constructor for `FullModuleHomomorphism`, exposed separately to
        /// mirror the upstream trait surface.
        #[staticmethod]
        #[pyo3(signature = (source, target, degree_shift = 0))]
        pub fn zero(
            source: PyRef<'_, SteenrodModule>,
            target: PyRef<'_, SteenrodModule>,
            degree_shift: i32,
        ) -> PyResult<Self> {
            Self::check_same_algebra(&source.0, &target.0)?;
            Ok(FullModuleHomomorphism(
                <FullModuleHomomorphismInner as ZeroHomomorphism<
                    RsSteenrodModule,
                    RsSteenrodModule,
                >>::zero_homomorphism(
                    Arc::new(source.0.clone()),
                    Arc::new(target.0.clone()),
                    degree_shift,
                ),
            ))
        }

        /// The identity homomorphism on `module` (the `IdentityHomomorphism`
        /// constructor): `degree_shift = 0` and the identity matrix in every
        /// degree. Its source and target are the *same* module, so source ==
        /// target holds by construction. Requires the module to be bounded
        /// above (`max_degree()` is `Some`); raises `ValueError` otherwise
        /// rather than letting the upstream `expect` panic.
        #[staticmethod]
        pub fn identity(module: PyRef<'_, SteenrodModule>) -> PyResult<Self> {
            let Some(max) = module.0.max_degree() else {
                return Err(PyValueError::new_err(
                    "identity requires a module that is bounded above",
                ));
            };
            // Populate the module (and algebra) basis through `max` so the
            // upstream loop's `dimension(i)` reads never index past the computed
            // range.
            module_ensure(&*module.0 as &DynModule, max);
            Ok(
                FullModuleHomomorphism(<FullModuleHomomorphismInner as IdentityHomomorphism<
                    RsSteenrodModule,
                >>::identity_homomorphism(Arc::new(
                    module.0.clone(),
                ))),
            )
        }

        // --- flattened ModuleHomomorphism method set --------------------------

        /// The source module (shares state via `Arc`).
        pub fn source(&self) -> SteenrodModule {
            SteenrodModule((*self.0.source()).clone())
        }

        /// The target module (shares state via `Arc`).
        pub fn target(&self) -> SteenrodModule {
            SteenrodModule((*self.0.target()).clone())
        }

        /// The degree shift: `output_degree = input_degree - degree_shift`.
        pub fn degree_shift(&self) -> i32 {
            self.0.degree_shift()
        }

        /// The smallest input degree the homomorphism is defined on
        /// (`source.min_degree()`).
        pub fn min_degree(&self) -> i32 {
            self.0.min_degree()
        }

        /// The prime as a plain `int` (`ValidPrime` is never exposed).
        pub fn prime(&self) -> u32 {
            self.0.prime().as_u32()
        }

        /// Apply the homomorphism to the basis element `input_idx` in
        /// `input_degree`, adding `coeff` times its image into `result` (a
        /// vector of length `target.dimension(input_degree - degree_shift)`).
        pub fn apply_to_basis_element(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            input_degree: i32,
            input_idx: usize,
        ) -> PyResult<()> {
            let p = self.0.prime().as_u32();
            let coeff = coeff % p;
            let src_min = self.source_min_degree();
            if input_degree < src_min {
                return Err(PyIndexError::new_err(format!(
                    "input degree {input_degree} is below the source min_degree {src_min}"
                )));
            }
            let output_degree = self.output_degree(input_degree)?;
            self.ensure_through(input_degree, output_degree);
            let src_dim = self.source_dim(input_degree);
            if input_idx >= src_dim {
                return Err(PyIndexError::new_err(format!(
                    "input index {input_idx} out of range for source degree {input_degree} \
                     (dimension {src_dim})"
                )));
            }
            let out_dim = self.target_dim(output_degree);
            crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                checked_same_prime(res.prime().as_u32(), p)?;
                checked_equal_len(res.as_slice().len(), out_dim)?;
                self.0
                    .apply_to_basis_element(res.copy(), coeff, input_degree, input_idx);
                Ok(())
            })
        }

        /// Apply the homomorphism to a general `input` element of `source` in
        /// `input_degree` (length `source.dimension(input_degree)`), adding
        /// `coeff` times its image into `result`. Aliasing the same vector as
        /// both `input` and `result` raises `RuntimeError` (the borrow
        /// conflict).
        pub fn apply(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            input_degree: i32,
            input: &Bound<'_, PyAny>,
        ) -> PyResult<()> {
            let p = self.0.prime().as_u32();
            let coeff = coeff % p;
            let src_min = self.source_min_degree();
            if input_degree < src_min {
                return Err(PyIndexError::new_err(format!(
                    "input degree {input_degree} is below the source min_degree {src_min}"
                )));
            }
            let output_degree = self.output_degree(input_degree)?;
            self.ensure_through(input_degree, output_degree);
            let src_dim = self.source_dim(input_degree);
            let out_dim = self.target_dim(output_degree);
            crate::fp_py::with_input_slice(py, input, |in_slice| {
                checked_same_prime(in_slice.prime().as_u32(), p)?;
                checked_equal_len(in_slice.len(), src_dim)?;
                crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                    checked_same_prime(res.prime().as_u32(), p)?;
                    checked_equal_len(res.as_slice().len(), out_dim)?;
                    self.0.apply(res.copy(), coeff, input_degree, in_slice);
                    Ok(())
                })
            })
        }

        /// The kernel of the homomorphism in `degree`, if it has been computed
        /// (via `compute_auxiliary_data_through_degree`). Returns `None`
        /// otherwise (never panics).
        pub fn kernel(&self, degree: i32) -> Option<crate::fp_py::PySubspace> {
            self.0
                .kernel(degree)
                .map(|s| crate::fp_py::PySubspace::from_rust(s.clone()))
        }

        /// The image of the homomorphism in `degree`, if it has been computed.
        pub fn image(&self, degree: i32) -> Option<crate::fp_py::PySubspace> {
            self.0
                .image(degree)
                .map(|s| crate::fp_py::PySubspace::from_rust(s.clone()))
        }

        /// The quasi-inverse of the homomorphism in `degree`, if it has been
        /// computed.
        pub fn quasi_inverse(&self, degree: i32) -> Option<crate::fp_py::PyQuasiInverse> {
            self.0
                .quasi_inverse(degree)
                .map(|qi| crate::fp_py::PyQuasiInverse::from_rust(qi.clone()))
        }

        /// Compute (and cache) the image, kernel and quasi-inverse at every
        /// input degree up to `degree`. Upstream clamps the work to the range of
        /// recorded matrices, so degrees beyond the recorded matrices are a
        /// no-op (the zero homomorphism built by `new`/`zero` records no
        /// matrices, hence computes nothing). The source/target bases (and the
        /// algebra) are computed first so the per-degree matrix reductions never
        /// index past the computed range.
        pub fn compute_auxiliary_data_through_degree(&self, degree: i32) -> PyResult<()> {
            let output_degree = self.output_degree(degree)?;
            self.ensure_through(degree, output_degree);
            self.0.compute_auxiliary_data_through_degree(degree);
            Ok(())
        }

        /// The matrix whose rows are the images of the source basis elements
        /// `inputs` in `degree`. Columns index `target.dimension(degree)`.
        ///
        /// As with `FreeModuleHomomorphism`, the per-row application lands in
        /// `target.dimension(degree - degree_shift)`, so the call is only
        /// well-defined when that equals `target.dimension(degree)` (always the
        /// case for `degree_shift == 0`); otherwise this raises `ValueError`
        /// rather than letting the dimension mismatch panic.
        pub fn get_partial_matrix(
            &self,
            degree: i32,
            inputs: Vec<usize>,
        ) -> PyResult<crate::fp_py::PyMatrix> {
            let src_min = self.source_min_degree();
            if degree < src_min {
                return Err(PyIndexError::new_err(format!(
                    "degree {degree} is below the source min_degree {src_min}"
                )));
            }
            let output_degree = self.output_degree(degree)?;
            self.ensure_through(degree, output_degree);
            let src_dim = self.source_dim(degree);
            for &i in &inputs {
                if i >= src_dim {
                    return Err(PyIndexError::new_err(format!(
                        "input index {i} out of range for source degree {degree} (dimension \
                         {src_dim})"
                    )));
                }
            }
            if self.target_dim(degree) != self.target_dim(output_degree) {
                return Err(PyValueError::new_err(
                    "get_partial_matrix is only well-defined when target.dimension(degree) == \
                     target.dimension(degree - degree_shift) (e.g. degree_shift == 0)",
                ));
            }
            Ok(crate::fp_py::PyMatrix::from_rust(
                self.0.get_partial_matrix(degree, &inputs),
            ))
        }

        /// Apply the quasi-inverse at `degree` to `input`, adding the result
        /// into `result`. Returns `True` if the quasi-inverse was available (and
        /// applied), `False` otherwise. `input` has length
        /// `target.dimension(degree - degree_shift)` and `result` has length
        /// `source.dimension(degree)`.
        pub fn apply_quasi_inverse(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            degree: i32,
            input: &Bound<'_, PyAny>,
        ) -> PyResult<bool> {
            let p = self.0.prime().as_u32();
            let Some(qi) = self.0.quasi_inverse(degree) else {
                return Ok(false);
            };
            let source_dim = qi.source_dimension();
            let target_dim = qi.target_dimension();
            crate::fp_py::with_input_slice(py, input, |in_slice| {
                checked_same_prime(in_slice.prime().as_u32(), p)?;
                checked_equal_len(in_slice.len(), target_dim)?;
                crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                    checked_same_prime(res.prime().as_u32(), p)?;
                    checked_equal_len(res.as_slice().len(), source_dim)?;
                    qi.apply(res.copy(), 1, in_slice);
                    Ok(())
                })
            })?;
            Ok(true)
        }

        pub fn __repr__(&self) -> String {
            format!(
                "FullModuleHomomorphism(degree_shift={}, min_degree={}, prime={})",
                self.0.degree_shift(),
                self.0.min_degree(),
                self.0.prime().as_u32()
            )
        }
    }

    /// The homomorphism induced on quotient modules by an underlying
    /// `FullModuleHomomorphism` `f`: given quotients `s` of `f.source()` and `t`
    /// of `f.target()`, this is the map `s -> t` sending the class of a basis
    /// element to the class of its image. Both source and target are the bound
    /// `QuotientModule` pyclass and share their `Arc`-held state with the inputs.
    ///
    /// The two quotients must genuinely be quotients of `f`'s source and target
    /// modules (checked by `Arc` identity at construction); otherwise the
    /// basis-index translation `s.basis_list[..]` would index a foreign module
    /// and panic. Every degree-indexed access is pre-checked so that an
    /// out-of-range index, prime/length mismatch, or an output degree outside
    /// the target quotient's range raises `ValueError`/`IndexError` rather than
    /// panicking across the FFI boundary.
    ///
    /// NOTE: upstream this homomorphism overrides only `apply_to_basis_element`;
    /// it carries no auxiliary data, so `kernel`/`image`/`quasi_inverse` always
    /// return `None`, `compute_auxiliary_data_through_degree` is a no-op, and
    /// `apply_quasi_inverse` always returns `False`. They are bound anyway so the
    /// flattened `ModuleHomomorphism` surface is uniform across homomorphisms.
    #[pyclass(name = "QuotientHomomorphism")]
    pub struct QuotientHomomorphism(QuotientHomomorphismInner);

    impl QuotientHomomorphism {
        /// `min_degree()` of the (quotient) source module.
        fn source_min_degree(&self) -> i32 {
            self.0.source().min_degree()
        }

        /// Dimension of the (quotient) source module in `degree` (guarded).
        fn source_dim(&self, degree: i32) -> usize {
            module_dimension(&*self.0.source() as &DynModule, degree)
        }

        /// Dimension of the (quotient) target module in `degree` (guarded).
        fn target_dim(&self, degree: i32) -> usize {
            module_dimension(&*self.0.target() as &DynModule, degree)
        }

        /// Ensure both quotients' bases (and algebra) are computed through the
        /// relevant degrees. (Quotient `compute_basis` is a no-op upstream; the
        /// underlying modules are already computed through each quotient's
        /// truncation at construction.)
        fn ensure_through(&self, input_degree: i32, output_degree: i32) {
            module_ensure(&*self.0.source() as &DynModule, input_degree);
            module_ensure(&*self.0.target() as &DynModule, output_degree);
        }

        /// `input_degree - degree_shift`, raising `ValueError` on overflow.
        fn output_degree(&self, input_degree: i32) -> PyResult<i32> {
            input_degree
                .checked_sub(self.0.degree_shift())
                .ok_or_else(|| PyValueError::new_err("output degree overflows i32"))
        }
    }

    #[pymethods]
    impl QuotientHomomorphism {
        /// Build the induced map `source -> target` from the underlying
        /// `FullModuleHomomorphism` `f`. `source` must be a quotient of
        /// `f.source()` and `target` a quotient of `f.target()` (checked by
        /// `Arc` identity); otherwise raises `ValueError`.
        #[new]
        pub fn new(
            f: PyRef<'_, FullModuleHomomorphism>,
            source: PyRef<'_, QuotientModule>,
            target: PyRef<'_, QuotientModule>,
        ) -> PyResult<Self> {
            if !Arc::ptr_eq(&*f.0.source(), &*source.0.module) {
                return Err(PyValueError::new_err(
                    "source must be a quotient of the homomorphism's source module",
                ));
            }
            if !Arc::ptr_eq(&*f.0.target(), &*target.0.module) {
                return Err(PyValueError::new_err(
                    "target must be a quotient of the homomorphism's target module",
                ));
            }
            Ok(QuotientHomomorphism(QuotientHomomorphismInner::new(
                Arc::new(f.0.clone()),
                Arc::clone(&source.0),
                Arc::clone(&target.0),
            )))
        }

        // --- flattened ModuleHomomorphism method set --------------------------

        /// The (quotient) source module (shares state via `Arc`).
        pub fn source(&self) -> QuotientModule {
            QuotientModule(self.0.source())
        }

        /// The (quotient) target module (shares state via `Arc`).
        pub fn target(&self) -> QuotientModule {
            QuotientModule(self.0.target())
        }

        /// The degree shift: `output_degree = input_degree - degree_shift`.
        pub fn degree_shift(&self) -> i32 {
            self.0.degree_shift()
        }

        /// The smallest input degree the homomorphism is defined on.
        pub fn min_degree(&self) -> i32 {
            self.0.min_degree()
        }

        /// The prime as a plain `int` (`ValidPrime` is never exposed).
        pub fn prime(&self) -> u32 {
            self.0.prime().as_u32()
        }

        /// Apply the homomorphism to the basis element `input_idx` in
        /// `input_degree`, adding `coeff` times its image into `result` (a vector
        /// of length `target.dimension(input_degree - degree_shift)`).
        pub fn apply_to_basis_element(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            input_degree: i32,
            input_idx: usize,
        ) -> PyResult<()> {
            let p = self.0.prime().as_u32();
            let coeff = coeff % p;
            let src_min = self.source_min_degree();
            if input_degree < src_min {
                return Err(PyIndexError::new_err(format!(
                    "input degree {input_degree} is below the source min_degree {src_min}"
                )));
            }
            let output_degree = self.output_degree(input_degree)?;
            self.ensure_through(input_degree, output_degree);
            let src_dim = self.source_dim(input_degree);
            if input_idx >= src_dim {
                return Err(PyIndexError::new_err(format!(
                    "input index {input_idx} out of range for source degree {input_degree} \
                     (dimension {src_dim})"
                )));
            }
            let out_dim = self.target_dim(output_degree);
            crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                checked_same_prime(res.prime().as_u32(), p)?;
                checked_equal_len(res.as_slice().len(), out_dim)?;
                // When the target quotient is zero in the output degree (the
                // output degree is above the target truncation or below its min
                // degree) the image is zero; the upstream call would index the
                // target's `basis_list`/underlying-module dimension out of range,
                // so skip it. `out_dim == 0` already forces `res` to length 0.
                if out_dim != 0 {
                    self.0
                        .apply_to_basis_element(res.copy(), coeff, input_degree, input_idx);
                }
                Ok(())
            })
        }

        /// Apply the homomorphism to a general `input` element of the (quotient)
        /// source in `input_degree` (length `source.dimension(input_degree)`),
        /// adding `coeff` times its image into `result`. Aliasing the same vector
        /// as both `input` and `result` raises `RuntimeError`.
        pub fn apply(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            input_degree: i32,
            input: &Bound<'_, PyAny>,
        ) -> PyResult<()> {
            let p = self.0.prime().as_u32();
            let coeff = coeff % p;
            let src_min = self.source_min_degree();
            if input_degree < src_min {
                return Err(PyIndexError::new_err(format!(
                    "input degree {input_degree} is below the source min_degree {src_min}"
                )));
            }
            let output_degree = self.output_degree(input_degree)?;
            self.ensure_through(input_degree, output_degree);
            let src_dim = self.source_dim(input_degree);
            let out_dim = self.target_dim(output_degree);
            crate::fp_py::with_input_slice(py, input, |in_slice| {
                checked_same_prime(in_slice.prime().as_u32(), p)?;
                checked_equal_len(in_slice.len(), src_dim)?;
                crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                    checked_same_prime(res.prime().as_u32(), p)?;
                    checked_equal_len(res.as_slice().len(), out_dim)?;
                    if out_dim != 0 {
                        self.0.apply(res.copy(), coeff, input_degree, in_slice);
                    }
                    Ok(())
                })
            })
        }

        /// The kernel in `degree`. Always `None`: this homomorphism stores no
        /// auxiliary data upstream.
        pub fn kernel(&self, degree: i32) -> Option<crate::fp_py::PySubspace> {
            self.0
                .kernel(degree)
                .map(|s| crate::fp_py::PySubspace::from_rust(s.clone()))
        }

        /// The image in `degree`. Always `None` (see `kernel`).
        pub fn image(&self, degree: i32) -> Option<crate::fp_py::PySubspace> {
            self.0
                .image(degree)
                .map(|s| crate::fp_py::PySubspace::from_rust(s.clone()))
        }

        /// The quasi-inverse in `degree`. Always `None` (see `kernel`).
        pub fn quasi_inverse(&self, degree: i32) -> Option<crate::fp_py::PyQuasiInverse> {
            self.0
                .quasi_inverse(degree)
                .map(|qi| crate::fp_py::PyQuasiInverse::from_rust(qi.clone()))
        }

        /// No-op upstream (this homomorphism stores no auxiliary data); bound for
        /// surface uniformity. Still validates the output degree against `i32`
        /// overflow.
        pub fn compute_auxiliary_data_through_degree(&self, degree: i32) -> PyResult<()> {
            let output_degree = self.output_degree(degree)?;
            self.ensure_through(degree, output_degree);
            self.0.compute_auxiliary_data_through_degree(degree);
            Ok(())
        }

        /// The matrix whose rows are the images of the (quotient) source basis
        /// elements `inputs` in `degree`. Columns index `target.dimension(degree)`.
        ///
        /// The per-row application lands in `target.dimension(degree -
        /// degree_shift)`, so the call is only well-defined when that equals
        /// `target.dimension(degree)` (always so for `degree_shift == 0`);
        /// otherwise this raises `ValueError` rather than panicking.
        pub fn get_partial_matrix(
            &self,
            degree: i32,
            inputs: Vec<usize>,
        ) -> PyResult<crate::fp_py::PyMatrix> {
            let src_min = self.source_min_degree();
            if degree < src_min {
                return Err(PyIndexError::new_err(format!(
                    "degree {degree} is below the source min_degree {src_min}"
                )));
            }
            let output_degree = self.output_degree(degree)?;
            self.ensure_through(degree, output_degree);
            let src_dim = self.source_dim(degree);
            for &i in &inputs {
                if i >= src_dim {
                    return Err(PyIndexError::new_err(format!(
                        "input index {i} out of range for source degree {degree} (dimension \
                         {src_dim})"
                    )));
                }
            }
            // The trait-default builds a `target.dimension(degree)`-column
            // matrix and returns early when that is 0. But evaluating
            // `target.dimension(degree)` upstream indexes `QuotientModule`'s
            // `subspaces` BiVec directly, which panics when `degree` is outside
            // the target quotient's defined range (below its min degree or above
            // its truncation) — reachable even with `degree_shift == 0` when the
            // source/target quotients have asymmetric min degrees. Use the safe
            // dimension wrapper and return the empty (`len(inputs) x 0`) matrix
            // directly in that case.
            let tgt_dim = self.target_dim(degree);
            if tgt_dim == 0 {
                return Ok(crate::fp_py::PyMatrix::from_rust(fp::matrix::Matrix::new(
                    self.0.prime(),
                    inputs.len(),
                    0,
                )));
            }
            if tgt_dim != self.target_dim(output_degree) {
                return Err(PyValueError::new_err(
                    "get_partial_matrix is only well-defined when target.dimension(degree) == \
                     target.dimension(degree - degree_shift) (e.g. degree_shift == 0)",
                ));
            }
            Ok(crate::fp_py::PyMatrix::from_rust(
                self.0.get_partial_matrix(degree, &inputs),
            ))
        }

        /// Apply the quasi-inverse at `degree` to `input`. Always returns `False`
        /// (no quasi-inverse is ever stored) and therefore does NOT validate
        /// `input`/`result` (no prime/length/index/aliasing checks run).
        pub fn apply_quasi_inverse(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            degree: i32,
            input: &Bound<'_, PyAny>,
        ) -> PyResult<bool> {
            let p = self.0.prime().as_u32();
            let Some(qi) = self.0.quasi_inverse(degree) else {
                return Ok(false);
            };
            let source_dim = qi.source_dimension();
            let target_dim = qi.target_dimension();
            crate::fp_py::with_input_slice(py, input, |in_slice| {
                checked_same_prime(in_slice.prime().as_u32(), p)?;
                checked_equal_len(in_slice.len(), target_dim)?;
                crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                    checked_same_prime(res.prime().as_u32(), p)?;
                    checked_equal_len(res.as_slice().len(), source_dim)?;
                    qi.apply(res.copy(), 1, in_slice);
                    Ok(())
                })
            })?;
            Ok(true)
        }

        pub fn __repr__(&self) -> String {
            format!(
                "QuotientHomomorphism(degree_shift={}, min_degree={}, prime={})",
                self.0.degree_shift(),
                self.0.min_degree(),
                self.0.prime().as_u32()
            )
        }
    }

    /// The source-side quotient map `s -> f.target()` induced by a
    /// `FullModuleHomomorphism` `f` and a quotient `s` of `f.source()`: it sends
    /// the class of a basis element to its image in the (un-quotiented) target.
    /// Its source is the bound `QuotientModule` pyclass and its target is the
    /// bound `SteenrodModule` pyclass; both share their `Arc`-held state.
    ///
    /// `s` must genuinely be a quotient of `f.source()` (checked by `Arc`
    /// identity at construction). As with `QuotientHomomorphism`, this carries no
    /// auxiliary data: `kernel`/`image`/`quasi_inverse` are always `None`,
    /// `compute_auxiliary_data_through_degree` is a no-op and
    /// `apply_quasi_inverse` always returns `False`.
    #[pyclass(name = "QuotientHomomorphismSource")]
    pub struct QuotientHomomorphismSource(QuotientHomomorphismSourceInner);

    impl QuotientHomomorphismSource {
        /// `min_degree()` of the (quotient) source module.
        fn source_min_degree(&self) -> i32 {
            self.0.source().min_degree()
        }

        /// Dimension of the (quotient) source module in `degree` (guarded).
        fn source_dim(&self, degree: i32) -> usize {
            module_dimension(&*self.0.source() as &DynModule, degree)
        }

        /// Dimension of the (plain) target module in `degree` (guarded).
        fn target_dim(&self, degree: i32) -> usize {
            module_dimension(&**self.0.target() as &DynModule, degree)
        }

        /// Ensure the (quotient) source basis and the (plain) target basis are
        /// computed through the relevant degrees (the latter is a genuine module
        /// whose basis must be extended).
        fn ensure_through(&self, input_degree: i32, output_degree: i32) {
            module_ensure(&*self.0.source() as &DynModule, input_degree);
            module_ensure(&**self.0.target() as &DynModule, output_degree);
        }

        /// `input_degree - degree_shift`, raising `ValueError` on overflow.
        fn output_degree(&self, input_degree: i32) -> PyResult<i32> {
            input_degree
                .checked_sub(self.0.degree_shift())
                .ok_or_else(|| PyValueError::new_err("output degree overflows i32"))
        }
    }

    #[pymethods]
    impl QuotientHomomorphismSource {
        /// Build the source-side quotient map from the underlying
        /// `FullModuleHomomorphism` `f` and a quotient `source` of `f.source()`
        /// (checked by `Arc` identity; otherwise raises `ValueError`).
        #[new]
        pub fn new(
            f: PyRef<'_, FullModuleHomomorphism>,
            source: PyRef<'_, QuotientModule>,
        ) -> PyResult<Self> {
            if !Arc::ptr_eq(&*f.0.source(), &*source.0.module) {
                return Err(PyValueError::new_err(
                    "source must be a quotient of the homomorphism's source module",
                ));
            }
            Ok(QuotientHomomorphismSource(
                QuotientHomomorphismSourceInner::new(Arc::new(f.0.clone()), Arc::clone(&source.0)),
            ))
        }

        // --- flattened ModuleHomomorphism method set --------------------------

        /// The (quotient) source module (shares state via `Arc`).
        pub fn source(&self) -> QuotientModule {
            QuotientModule(self.0.source())
        }

        /// The (plain) target module, boxed as a `SteenrodModule` (shares state
        /// via `Arc`).
        pub fn target(&self) -> SteenrodModule {
            SteenrodModule((*self.0.target()).clone())
        }

        /// The degree shift: `output_degree = input_degree - degree_shift`.
        pub fn degree_shift(&self) -> i32 {
            self.0.degree_shift()
        }

        /// The smallest input degree the homomorphism is defined on.
        pub fn min_degree(&self) -> i32 {
            self.0.min_degree()
        }

        /// The prime as a plain `int` (`ValidPrime` is never exposed).
        pub fn prime(&self) -> u32 {
            self.0.prime().as_u32()
        }

        /// Apply the homomorphism to the basis element `input_idx` in
        /// `input_degree`, adding `coeff` times its image into `result` (a vector
        /// of length `target.dimension(input_degree - degree_shift)`).
        pub fn apply_to_basis_element(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            input_degree: i32,
            input_idx: usize,
        ) -> PyResult<()> {
            let p = self.0.prime().as_u32();
            let coeff = coeff % p;
            let src_min = self.source_min_degree();
            if input_degree < src_min {
                return Err(PyIndexError::new_err(format!(
                    "input degree {input_degree} is below the source min_degree {src_min}"
                )));
            }
            let output_degree = self.output_degree(input_degree)?;
            self.ensure_through(input_degree, output_degree);
            let src_dim = self.source_dim(input_degree);
            if input_idx >= src_dim {
                return Err(PyIndexError::new_err(format!(
                    "input index {input_idx} out of range for source degree {input_degree} \
                     (dimension {src_dim})"
                )));
            }
            let out_dim = self.target_dim(output_degree);
            crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                checked_same_prime(res.prime().as_u32(), p)?;
                checked_equal_len(res.as_slice().len(), out_dim)?;
                self.0
                    .apply_to_basis_element(res.copy(), coeff, input_degree, input_idx);
                Ok(())
            })
        }

        /// Apply the homomorphism to a general `input` element of the (quotient)
        /// source in `input_degree` (length `source.dimension(input_degree)`),
        /// adding `coeff` times its image into `result`. Aliasing the same vector
        /// as both `input` and `result` raises `RuntimeError`.
        pub fn apply(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            input_degree: i32,
            input: &Bound<'_, PyAny>,
        ) -> PyResult<()> {
            let p = self.0.prime().as_u32();
            let coeff = coeff % p;
            let src_min = self.source_min_degree();
            if input_degree < src_min {
                return Err(PyIndexError::new_err(format!(
                    "input degree {input_degree} is below the source min_degree {src_min}"
                )));
            }
            let output_degree = self.output_degree(input_degree)?;
            self.ensure_through(input_degree, output_degree);
            let src_dim = self.source_dim(input_degree);
            let out_dim = self.target_dim(output_degree);
            crate::fp_py::with_input_slice(py, input, |in_slice| {
                checked_same_prime(in_slice.prime().as_u32(), p)?;
                checked_equal_len(in_slice.len(), src_dim)?;
                crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                    checked_same_prime(res.prime().as_u32(), p)?;
                    checked_equal_len(res.as_slice().len(), out_dim)?;
                    self.0.apply(res.copy(), coeff, input_degree, in_slice);
                    Ok(())
                })
            })
        }

        /// The kernel in `degree`. Always `None` (no auxiliary data upstream).
        pub fn kernel(&self, degree: i32) -> Option<crate::fp_py::PySubspace> {
            self.0
                .kernel(degree)
                .map(|s| crate::fp_py::PySubspace::from_rust(s.clone()))
        }

        /// The image in `degree`. Always `None` (see `kernel`).
        pub fn image(&self, degree: i32) -> Option<crate::fp_py::PySubspace> {
            self.0
                .image(degree)
                .map(|s| crate::fp_py::PySubspace::from_rust(s.clone()))
        }

        /// The quasi-inverse in `degree`. Always `None` (see `kernel`).
        pub fn quasi_inverse(&self, degree: i32) -> Option<crate::fp_py::PyQuasiInverse> {
            self.0
                .quasi_inverse(degree)
                .map(|qi| crate::fp_py::PyQuasiInverse::from_rust(qi.clone()))
        }

        /// No-op upstream (no auxiliary data); validates output-degree overflow.
        pub fn compute_auxiliary_data_through_degree(&self, degree: i32) -> PyResult<()> {
            let output_degree = self.output_degree(degree)?;
            self.ensure_through(degree, output_degree);
            self.0.compute_auxiliary_data_through_degree(degree);
            Ok(())
        }

        /// The matrix whose rows are the images of the (quotient) source basis
        /// elements `inputs` in `degree`. Columns index `target.dimension(degree)`.
        /// Only well-defined when `target.dimension(degree) == target.dimension(
        /// degree - degree_shift)` (e.g. `degree_shift == 0`); otherwise raises
        /// `ValueError`.
        pub fn get_partial_matrix(
            &self,
            degree: i32,
            inputs: Vec<usize>,
        ) -> PyResult<crate::fp_py::PyMatrix> {
            let src_min = self.source_min_degree();
            if degree < src_min {
                return Err(PyIndexError::new_err(format!(
                    "degree {degree} is below the source min_degree {src_min}"
                )));
            }
            let output_degree = self.output_degree(degree)?;
            self.ensure_through(degree, output_degree);
            let src_dim = self.source_dim(degree);
            for &i in &inputs {
                if i >= src_dim {
                    return Err(PyIndexError::new_err(format!(
                        "input index {i} out of range for source degree {degree} (dimension \
                         {src_dim})"
                    )));
                }
            }
            if self.target_dim(degree) != self.target_dim(output_degree) {
                return Err(PyValueError::new_err(
                    "get_partial_matrix is only well-defined when target.dimension(degree) == \
                     target.dimension(degree - degree_shift) (e.g. degree_shift == 0)",
                ));
            }
            Ok(crate::fp_py::PyMatrix::from_rust(
                self.0.get_partial_matrix(degree, &inputs),
            ))
        }

        /// Apply the quasi-inverse at `degree`. Always returns `False` (no
        /// quasi-inverse is ever stored) and therefore does NOT validate
        /// `input`/`result` (no prime/length/index/aliasing checks run).
        pub fn apply_quasi_inverse(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            degree: i32,
            input: &Bound<'_, PyAny>,
        ) -> PyResult<bool> {
            let p = self.0.prime().as_u32();
            let Some(qi) = self.0.quasi_inverse(degree) else {
                return Ok(false);
            };
            let source_dim = qi.source_dimension();
            let target_dim = qi.target_dimension();
            crate::fp_py::with_input_slice(py, input, |in_slice| {
                checked_same_prime(in_slice.prime().as_u32(), p)?;
                checked_equal_len(in_slice.len(), target_dim)?;
                crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                    checked_same_prime(res.prime().as_u32(), p)?;
                    checked_equal_len(res.as_slice().len(), source_dim)?;
                    qi.apply(res.copy(), 1, in_slice);
                    Ok(())
                })
            })?;
            Ok(true)
        }

        pub fn __repr__(&self) -> String {
            format!(
                "QuotientHomomorphismSource(degree_shift={}, min_degree={}, prime={})",
                self.0.degree_shift(),
                self.0.min_degree(),
                self.0.prime().as_u32()
            )
        }
    }

    /// The generic zero homomorphism `source -> target` with a given
    /// `degree_shift`: it maps every element to `0`. Both source and target are
    /// the bound `SteenrodModule` pyclass and share their `Arc`-held state.
    ///
    /// Upstream `apply_to_basis_element` is a no-op, so `apply` never changes
    /// `result`. The map carries no auxiliary data:
    /// `kernel`/`image`/`quasi_inverse` are always `None`,
    /// `compute_auxiliary_data_through_degree` is a no-op and
    /// `apply_quasi_inverse` always returns `False` (and, because there is never
    /// a quasi-inverse, does not validate its inputs). The `apply`/
    /// `apply_to_basis_element` paths still validate their inputs
    /// (prime/length/index/aliasing) so misuse raises `ValueError`/`IndexError`/
    /// `RuntimeError` rather than silently succeeding.
    #[pyclass(name = "GenericZeroHomomorphism")]
    pub struct GenericZeroHomomorphism(GenericZeroHomomorphismInner);

    impl GenericZeroHomomorphism {
        fn source_min_degree(&self) -> i32 {
            self.0.source().min_degree()
        }

        fn source_dim(&self, degree: i32) -> usize {
            module_dimension(&**self.0.source() as &DynModule, degree)
        }

        fn target_dim(&self, degree: i32) -> usize {
            module_dimension(&**self.0.target() as &DynModule, degree)
        }

        fn ensure_through(&self, input_degree: i32, output_degree: i32) {
            module_ensure(&**self.0.source() as &DynModule, input_degree);
            module_ensure(&**self.0.target() as &DynModule, output_degree);
        }

        fn output_degree(&self, input_degree: i32) -> PyResult<i32> {
            input_degree
                .checked_sub(self.0.degree_shift())
                .ok_or_else(|| PyValueError::new_err("output degree overflows i32"))
        }
    }

    #[pymethods]
    impl GenericZeroHomomorphism {
        /// Build the zero homomorphism `source -> target` with the given
        /// `degree_shift`. The factors must be built over the *same* algebra
        /// object (checked by prime and `Arc` identity); otherwise raises
        /// `ValueError`.
        #[new]
        #[pyo3(signature = (source, target, degree_shift = 0))]
        pub fn new(
            source: PyRef<'_, SteenrodModule>,
            target: PyRef<'_, SteenrodModule>,
            degree_shift: i32,
        ) -> PyResult<Self> {
            let source_alg = source.0.algebra();
            let target_alg = target.0.algebra();
            checked_same_prime(source_alg.prime().as_u32(), target_alg.prime().as_u32())?;
            if !Arc::ptr_eq(&source_alg, &target_alg) {
                return Err(PyValueError::new_err(
                    "source and target must be built over the same algebra",
                ));
            }
            Ok(GenericZeroHomomorphism(GenericZeroHomomorphismInner::new(
                Arc::new(source.0.clone()),
                Arc::new(target.0.clone()),
                degree_shift,
            )))
        }

        // --- flattened ModuleHomomorphism method set --------------------------

        /// The source module (shares state via `Arc`).
        pub fn source(&self) -> SteenrodModule {
            SteenrodModule((*self.0.source()).clone())
        }

        /// The target module (shares state via `Arc`).
        pub fn target(&self) -> SteenrodModule {
            SteenrodModule((*self.0.target()).clone())
        }

        /// The degree shift: `output_degree = input_degree - degree_shift`.
        pub fn degree_shift(&self) -> i32 {
            self.0.degree_shift()
        }

        /// The smallest input degree the homomorphism is defined on.
        pub fn min_degree(&self) -> i32 {
            self.0.min_degree()
        }

        /// The prime as a plain `int` (`ValidPrime` is never exposed).
        pub fn prime(&self) -> u32 {
            self.0.prime().as_u32()
        }

        /// Apply the homomorphism to the basis element `input_idx` in
        /// `input_degree`. A no-op (the zero map), but still validates the
        /// inputs. `result` has length `target.dimension(input_degree -
        /// degree_shift)`.
        pub fn apply_to_basis_element(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            input_degree: i32,
            input_idx: usize,
        ) -> PyResult<()> {
            let p = self.0.prime().as_u32();
            let coeff = coeff % p;
            let src_min = self.source_min_degree();
            if input_degree < src_min {
                return Err(PyIndexError::new_err(format!(
                    "input degree {input_degree} is below the source min_degree {src_min}"
                )));
            }
            let output_degree = self.output_degree(input_degree)?;
            self.ensure_through(input_degree, output_degree);
            let src_dim = self.source_dim(input_degree);
            if input_idx >= src_dim {
                return Err(PyIndexError::new_err(format!(
                    "input index {input_idx} out of range for source degree {input_degree} \
                     (dimension {src_dim})"
                )));
            }
            let out_dim = self.target_dim(output_degree);
            crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                checked_same_prime(res.prime().as_u32(), p)?;
                checked_equal_len(res.as_slice().len(), out_dim)?;
                self.0
                    .apply_to_basis_element(res.copy(), coeff, input_degree, input_idx);
                Ok(())
            })
        }

        /// Apply the homomorphism to a general `input` element in `input_degree`.
        /// A no-op (the zero map), but still validates the inputs. Aliasing the
        /// same vector as both `input` and `result` raises `RuntimeError`.
        pub fn apply(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            input_degree: i32,
            input: &Bound<'_, PyAny>,
        ) -> PyResult<()> {
            let p = self.0.prime().as_u32();
            let coeff = coeff % p;
            let src_min = self.source_min_degree();
            if input_degree < src_min {
                return Err(PyIndexError::new_err(format!(
                    "input degree {input_degree} is below the source min_degree {src_min}"
                )));
            }
            let output_degree = self.output_degree(input_degree)?;
            self.ensure_through(input_degree, output_degree);
            let src_dim = self.source_dim(input_degree);
            let out_dim = self.target_dim(output_degree);
            crate::fp_py::with_input_slice(py, input, |in_slice| {
                checked_same_prime(in_slice.prime().as_u32(), p)?;
                checked_equal_len(in_slice.len(), src_dim)?;
                crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                    checked_same_prime(res.prime().as_u32(), p)?;
                    checked_equal_len(res.as_slice().len(), out_dim)?;
                    self.0.apply(res.copy(), coeff, input_degree, in_slice);
                    Ok(())
                })
            })
        }

        /// The kernel in `degree`. Always `None` (no auxiliary data upstream).
        pub fn kernel(&self, degree: i32) -> Option<crate::fp_py::PySubspace> {
            self.0
                .kernel(degree)
                .map(|s| crate::fp_py::PySubspace::from_rust(s.clone()))
        }

        /// The image in `degree`. Always `None` (see `kernel`).
        pub fn image(&self, degree: i32) -> Option<crate::fp_py::PySubspace> {
            self.0
                .image(degree)
                .map(|s| crate::fp_py::PySubspace::from_rust(s.clone()))
        }

        /// The quasi-inverse in `degree`. Always `None` (see `kernel`).
        pub fn quasi_inverse(&self, degree: i32) -> Option<crate::fp_py::PyQuasiInverse> {
            self.0
                .quasi_inverse(degree)
                .map(|qi| crate::fp_py::PyQuasiInverse::from_rust(qi.clone()))
        }

        /// No-op upstream (no auxiliary data); validates output-degree overflow.
        pub fn compute_auxiliary_data_through_degree(&self, degree: i32) -> PyResult<()> {
            let output_degree = self.output_degree(degree)?;
            self.ensure_through(degree, output_degree);
            self.0.compute_auxiliary_data_through_degree(degree);
            Ok(())
        }

        /// The matrix whose rows are the images of the source basis elements
        /// `inputs` in `degree` — always the zero matrix of shape `len(inputs) x
        /// target.dimension(degree)`. Validates the input indices.
        pub fn get_partial_matrix(
            &self,
            degree: i32,
            inputs: Vec<usize>,
        ) -> PyResult<crate::fp_py::PyMatrix> {
            let src_min = self.source_min_degree();
            if degree < src_min {
                return Err(PyIndexError::new_err(format!(
                    "degree {degree} is below the source min_degree {src_min}"
                )));
            }
            let output_degree = self.output_degree(degree)?;
            self.ensure_through(degree, output_degree);
            let src_dim = self.source_dim(degree);
            for &i in &inputs {
                if i >= src_dim {
                    return Err(PyIndexError::new_err(format!(
                        "input index {i} out of range for source degree {degree} (dimension \
                         {src_dim})"
                    )));
                }
            }
            // The trait-default builds a `target.dimension(degree)`-column matrix
            // and returns early when that is 0. `ensure_through` above only
            // computes the target through `output_degree`, so for
            // `degree_shift > 0` the upstream `target.dimension(degree)` query
            // would reach beyond the target's computed range (or below its min)
            // and trip the `OnceVec`/BiVec assertion. Calling the safe wrapper
            // ensures the target through `degree` (returning 0 when it is out of
            // range) and makes the subsequent upstream call safe; when it is 0 we
            // build the empty (`len(inputs) x 0`) matrix directly.
            let tgt_dim = self.target_dim(degree);
            if tgt_dim == 0 {
                return Ok(crate::fp_py::PyMatrix::from_rust(fp::matrix::Matrix::new(
                    self.0.prime(),
                    inputs.len(),
                    0,
                )));
            }
            Ok(crate::fp_py::PyMatrix::from_rust(
                self.0.get_partial_matrix(degree, &inputs),
            ))
        }

        /// Apply the quasi-inverse at `degree`. Always returns `False` (no
        /// quasi-inverse is ever stored) and therefore does NOT validate
        /// `input`/`result` (no prime/length/index/aliasing checks run).
        pub fn apply_quasi_inverse(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            degree: i32,
            input: &Bound<'_, PyAny>,
        ) -> PyResult<bool> {
            let p = self.0.prime().as_u32();
            let Some(qi) = self.0.quasi_inverse(degree) else {
                return Ok(false);
            };
            let source_dim = qi.source_dimension();
            let target_dim = qi.target_dimension();
            crate::fp_py::with_input_slice(py, input, |in_slice| {
                checked_same_prime(in_slice.prime().as_u32(), p)?;
                checked_equal_len(in_slice.len(), target_dim)?;
                crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                    checked_same_prime(res.prime().as_u32(), p)?;
                    checked_equal_len(res.as_slice().len(), source_dim)?;
                    qi.apply(res.copy(), 1, in_slice);
                    Ok(())
                })
            })?;
            Ok(true)
        }

        pub fn __repr__(&self) -> String {
            format!(
                "GenericZeroHomomorphism(degree_shift={}, min_degree={}, prime={})",
                self.0.degree_shift(),
                self.0.min_degree(),
                self.0.prime().as_u32()
            )
        }
    }

    /// The induced pullback map `Hom(B, X) -> Hom(A, X)` of a free → free map
    /// `map: A -> B`, where `A`, `B` are `FreeModule`s and `X` is a (boxed)
    /// `SteenrodModule`. Its `source` is `Hom(B, X)` and its `target` is
    /// `Hom(A, X)`, both the bound `HomModule` pyclass (sharing their `Arc`-held
    /// state). The `map` is the bound `FreeModuleHomomorphismToFree`.
    ///
    /// `HomModule`'s algebra is the ground `Field` (it is *not* a
    /// `SteenrodModule`), so the binding drives basis computation through
    /// `HomModule::ensure` — which extends the underlying source's *Steenrod*
    /// algebra and is the same machinery the `HomModule` pyclass uses.
    ///
    /// Construction enforces the three upstream `assert!`s as `ValueError`s (not
    /// panics): `source.source() == map.target()`, `target.source() ==
    /// map.source()` and `source.target() == target.target()` (all compared by
    /// `Arc::ptr_eq` on the underlying `FreeModule`/`SteenrodModule`).
    ///
    /// Upstream `HomPullback` overrides `apply_to_basis_element`,
    /// `compute_auxiliary_data_through_degree`, `kernel`, `image`,
    /// `quasi_inverse`, `source`, `target`, `degree_shift` and `min_degree`; the
    /// remaining `ModuleHomomorphism` surface (`apply`, `get_matrix`/
    /// `get_partial_matrix`, `auxiliary_data`, `apply_quasi_inverse`) uses the
    /// trait defaults. Unlike `QuotientHomomorphism`, the auxiliary data is
    /// genuinely computed and stored (`kernel`/`image`/`quasi_inverse` return
    /// real subspaces once `compute_auxiliary_data_through_degree` runs).
    ///
    /// Every degree-indexed access is pre-checked: an uncomputed/out-of-range
    /// degree reads as dimension 0 (yielding a zero matrix / skipped apply), an
    /// out-of-range index, prime/length mismatch or aliasing raises
    /// `IndexError`/`ValueError`/`RuntimeError`, and a `map` whose outputs are
    /// not defined far enough raises `ValueError` rather than panicking. The
    /// pyclass keeps an `Arc` clone of the `map` so these guards can inspect its
    /// outputs (the upstream `map` field is private).
    #[pyclass(name = "HomPullback")]
    pub struct HomPullback {
        inner: HomPullbackInner,
        map: Arc<FreeModuleHomToFreeInner>,
    }

    impl HomPullback {
        /// The source `Hom(B, X)` module as the bound `HomModule` pyclass
        /// (sharing the `Arc`).
        fn src_hom(&self) -> HomModule {
            HomModule(self.inner.source())
        }

        /// The target `Hom(A, X)` module as the bound `HomModule` pyclass.
        fn tgt_hom(&self) -> HomModule {
            HomModule(self.inner.target())
        }

        /// Dimension of the source `HomModule` in `degree` (guarded; reuses
        /// `HomModule::dimension`, which short-circuits to 0 for an
        /// out-of-range / uncomputable degree and never panics).
        fn source_dim(&self, degree: i32) -> usize {
            self.src_hom().dimension(degree)
        }

        /// Dimension of the target `HomModule` in `degree` (guarded).
        fn target_dim(&self, degree: i32) -> usize {
            self.tgt_hom().dimension(degree)
        }

        /// `input_degree - degree_shift`, raising `ValueError` on overflow.
        /// (`HomPullback::degree_shift() == -map.degree_shift()`, so the output
        /// degree is `input_degree + map.degree_shift()`.)
        fn output_degree(&self, input_degree: i32) -> PyResult<i32> {
            input_degree
                .checked_sub(self.inner.degree_shift())
                .ok_or_else(|| PyValueError::new_err("output degree overflows i32"))
        }

        /// Compute every basis (both `HomModule`s, and their underlying Steenrod
        /// algebra) the upstream `apply_to_basis_element` touches at input degree
        /// `fn_degree`, and verify the `map`'s outputs cover every target
        /// free-module generator it reads. Returns `Ok` once it is safe to apply
        /// the pullback to *any* basis element of `fn_degree`.
        ///
        /// Upstream iterates `map.source()`'s generators up to `max_degree =
        /// fn_degree + map.degree_shift() + X.max_degree() = output_degree +
        /// X.max_degree()`, calling `map.output(..)` on each, which panics if the
        /// outputs are not yet defined there; we replicate
        /// `FreeModuleHomomorphism::check_outputs_cover` against the `map`.
        ///
        /// `map.output(..)` also asserts `target_gen_deg >= map.min_degree()`
        /// (`free_module_homomorphism.rs:150`). This is unreachable: the upstream
        /// per-call filter (`hom_pullback.rs:86`) keeps only generators with
        /// `target_gen_deg >= max(generator_degree + degree_shift, ..)`, where
        /// `generator_degree` is a generator degree of `B = map.target()`, hence
        /// `>= B.min_degree()`. So the filter's lower bound is
        /// `>= B.min_degree() + degree_shift`. Since `map.min_degree() ==
        /// max(A.min_degree(), B.min_degree() + degree_shift)`, every admitted
        /// generator satisfies `target_gen_deg >= map.min_degree()` whether the
        /// max is attained by `A` (the iterated generators all live `>=
        /// A.min_degree()`) or by `B` (the filter bound dominates). No guard is
        /// needed for the `min_degree` assert; only the outputs-cover check
        /// above is required.
        fn ensure_apply(&self, fn_degree: i32, output_degree: i32) -> PyResult<()> {
            // Computing the source HomModule through `fn_degree` and the target
            // HomModule through `output_degree` also computes (via
            // `HomModule::compute_basis`) the underlying free modules through the
            // degrees upstream reads, plus the shared module `X`.
            self.src_hom().ensure(fn_degree);
            self.tgt_hom().ensure(output_degree);
            let tmax = self.inner.source().target().max_degree().ok_or_else(|| {
                PyValueError::new_err("the common module X must be bounded above")
            })?;
            let max_degree = output_degree
                .checked_add(tmax)
                .ok_or_else(|| PyValueError::new_err("degree overflows i32"))?;
            let a = self.map.source();
            let lo = self.map.next_degree().max(a.min_degree());
            let hi = max_degree.min(a.max_computed_degree());
            for d in lo..=hi {
                if a.number_of_gens_in_degree(d) > 0 {
                    return Err(PyValueError::new_err(format!(
                        "the pullback map's outputs are not defined on its source generators in \
                         degree {d}; extend the map (add_generators_from_rows / extend_by_zero) up \
                         to degree {max_degree} first"
                    )));
                }
            }
            Ok(())
        }
    }

    #[pymethods]
    impl HomPullback {
        /// Build the pullback `source = Hom(B, X) -> target = Hom(A, X)` of the
        /// free → free `map: A -> B`. The three upstream identities are checked
        /// by `Arc::ptr_eq` and raise `ValueError` on mismatch:
        ///   * `source.source()` (the free module `B`) `== map.target()`,
        ///   * `target.source()` (the free module `A`) `== map.source()`,
        ///   * `source.target() == target.target()` (the common module `X`).
        #[new]
        pub fn new(
            source: PyRef<'_, HomModule>,
            target: PyRef<'_, HomModule>,
            map: PyRef<'_, FreeModuleHomomorphismToFree>,
        ) -> PyResult<Self> {
            let map_arc = Arc::clone(&map.0);
            if !Arc::ptr_eq(&source.0.source(), &map_arc.target()) {
                return Err(PyValueError::new_err(
                    "source.source() must equal map.target() (source must be Hom(B, X) for \
                     map: A -> B)",
                ));
            }
            if !Arc::ptr_eq(&target.0.source(), &map_arc.source()) {
                return Err(PyValueError::new_err(
                    "target.source() must equal map.source() (target must be Hom(A, X) for \
                     map: A -> B)",
                ));
            }
            if !Arc::ptr_eq(&source.0.target(), &target.0.target()) {
                return Err(PyValueError::new_err(
                    "source.target() must equal target.target() (both Hom modules must share the \
                     same module X)",
                ));
            }
            let inner = HomPullbackInner::new(
                Arc::clone(&source.0),
                Arc::clone(&target.0),
                Arc::clone(&map_arc),
            );
            Ok(HomPullback {
                inner,
                map: map_arc,
            })
        }

        // --- flattened ModuleHomomorphism method set --------------------------

        /// The source `Hom(B, X)` module (shares state via `Arc`).
        pub fn source(&self) -> HomModule {
            self.src_hom()
        }

        /// The target `Hom(A, X)` module (shares state via `Arc`).
        pub fn target(&self) -> HomModule {
            self.tgt_hom()
        }

        /// The degree shift: `output_degree = input_degree - degree_shift`.
        /// Upstream this is `-map.degree_shift()`.
        pub fn degree_shift(&self) -> i32 {
            self.inner.degree_shift()
        }

        /// The smallest input degree the homomorphism is defined on
        /// (`source.min_degree()`).
        pub fn min_degree(&self) -> i32 {
            self.inner.min_degree()
        }

        /// The prime as a plain `int` (`ValidPrime` is never exposed).
        pub fn prime(&self) -> u32 {
            self.inner.prime().as_u32()
        }

        /// Apply the pullback to the basis element `input_idx` in `input_degree`,
        /// adding `coeff` times its image into `result` (a vector of length
        /// `target.dimension(input_degree - degree_shift)`).
        pub fn apply_to_basis_element(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            input_degree: i32,
            input_idx: usize,
        ) -> PyResult<()> {
            let p = self.inner.prime().as_u32();
            let coeff = coeff % p;
            let src_min = self.inner.min_degree();
            if input_degree < src_min {
                return Err(PyIndexError::new_err(format!(
                    "input degree {input_degree} is below the source min_degree {src_min}"
                )));
            }
            let output_degree = self.output_degree(input_degree)?;
            self.ensure_apply(input_degree, output_degree)?;
            let src_dim = self.source_dim(input_degree);
            if input_idx >= src_dim {
                return Err(PyIndexError::new_err(format!(
                    "input index {input_idx} out of range for source degree {input_degree} \
                     (dimension {src_dim})"
                )));
            }
            let out_dim = self.target_dim(output_degree);
            crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                checked_same_prime(res.prime().as_u32(), p)?;
                checked_equal_len(res.as_slice().len(), out_dim)?;
                // When the target Hom module is zero in the output degree the
                // image is zero; the upstream call would index the target's
                // block structure out of range, so skip it. `out_dim == 0`
                // already forces `res` to length 0.
                if out_dim != 0 {
                    self.inner
                        .apply_to_basis_element(res.copy(), coeff, input_degree, input_idx);
                }
                Ok(())
            })
        }

        /// Apply the pullback to a general `input` element of `source` in
        /// `input_degree` (length `source.dimension(input_degree)`), adding
        /// `coeff` times its image into `result`. Aliasing the same vector as
        /// both `input` and `result` raises `RuntimeError`.
        pub fn apply(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            input_degree: i32,
            input: &Bound<'_, PyAny>,
        ) -> PyResult<()> {
            let p = self.inner.prime().as_u32();
            let coeff = coeff % p;
            let src_min = self.inner.min_degree();
            if input_degree < src_min {
                return Err(PyIndexError::new_err(format!(
                    "input degree {input_degree} is below the source min_degree {src_min}"
                )));
            }
            let output_degree = self.output_degree(input_degree)?;
            self.ensure_apply(input_degree, output_degree)?;
            let src_dim = self.source_dim(input_degree);
            let out_dim = self.target_dim(output_degree);
            crate::fp_py::with_input_slice(py, input, |in_slice| {
                checked_same_prime(in_slice.prime().as_u32(), p)?;
                checked_equal_len(in_slice.len(), src_dim)?;
                crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                    checked_same_prime(res.prime().as_u32(), p)?;
                    checked_equal_len(res.as_slice().len(), out_dim)?;
                    if out_dim != 0 {
                        self.inner.apply(res.copy(), coeff, input_degree, in_slice);
                    }
                    Ok(())
                })
            })
        }

        /// The kernel of the pullback in `degree`, if it has been computed (via
        /// `compute_auxiliary_data_through_degree`). Returns `None` otherwise.
        pub fn kernel(&self, degree: i32) -> Option<crate::fp_py::PySubspace> {
            self.inner
                .kernel(degree)
                .map(|s| crate::fp_py::PySubspace::from_rust(s.clone()))
        }

        /// The image of the pullback in `degree`, if it has been computed.
        pub fn image(&self, degree: i32) -> Option<crate::fp_py::PySubspace> {
            self.inner
                .image(degree)
                .map(|s| crate::fp_py::PySubspace::from_rust(s.clone()))
        }

        /// The quasi-inverse of the pullback in `degree`, if it has been
        /// computed.
        pub fn quasi_inverse(&self, degree: i32) -> Option<crate::fp_py::PyQuasiInverse> {
            self.inner
                .quasi_inverse(degree)
                .map(|qi| crate::fp_py::PyQuasiInverse::from_rust(qi.clone()))
        }

        /// Compute (and cache) the image, kernel and quasi-inverse at every
        /// input degree up to `degree`. Requires the `map`'s outputs to be
        /// defined far enough (else `ValueError`); computing the top degree's
        /// bases also computes every lower degree's (the bases are cumulative).
        pub fn compute_auxiliary_data_through_degree(&self, degree: i32) -> PyResult<()> {
            if degree >= self.inner.min_degree() {
                let output_degree = self.output_degree(degree)?;
                self.ensure_apply(degree, output_degree)?;
            }
            self.inner.compute_auxiliary_data_through_degree(degree);
            Ok(())
        }

        /// The matrix whose rows are the images of the source basis elements
        /// `inputs` in `degree`. Columns index `target.dimension(degree)`.
        ///
        /// Only well-defined when `target.dimension(degree) ==
        /// target.dimension(degree - degree_shift)` (always so for
        /// `degree_shift == 0`); otherwise raises `ValueError`. An out-of-range /
        /// uncomputed target degree reads as dimension 0 and yields the empty
        /// (`len(inputs) x 0`) matrix instead of panicking.
        pub fn get_partial_matrix(
            &self,
            degree: i32,
            inputs: Vec<usize>,
        ) -> PyResult<crate::fp_py::PyMatrix> {
            let src_min = self.inner.min_degree();
            if degree < src_min {
                return Err(PyIndexError::new_err(format!(
                    "degree {degree} is below the source min_degree {src_min}"
                )));
            }
            let output_degree = self.output_degree(degree)?;
            self.ensure_apply(degree, output_degree)?;
            let src_dim = self.source_dim(degree);
            for &i in &inputs {
                if i >= src_dim {
                    return Err(PyIndexError::new_err(format!(
                        "input index {i} out of range for source degree {degree} (dimension \
                         {src_dim})"
                    )));
                }
            }
            let tgt_dim = self.target_dim(degree);
            if tgt_dim == 0 {
                return Ok(crate::fp_py::PyMatrix::from_rust(fp::matrix::Matrix::new(
                    self.inner.prime(),
                    inputs.len(),
                    0,
                )));
            }
            if tgt_dim != self.target_dim(output_degree) {
                return Err(PyValueError::new_err(
                    "get_partial_matrix is only well-defined when target.dimension(degree) == \
                     target.dimension(degree - degree_shift) (e.g. degree_shift == 0)",
                ));
            }
            Ok(crate::fp_py::PyMatrix::from_rust(
                self.inner.get_partial_matrix(degree, &inputs),
            ))
        }

        /// Apply the quasi-inverse at `degree` to `input`, adding the result into
        /// `result`. Returns `True` if the quasi-inverse was available (and
        /// applied), `False` otherwise. `input` has length
        /// `target.dimension(degree - degree_shift)` and `result` has length
        /// `source.dimension(degree)`.
        pub fn apply_quasi_inverse(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            degree: i32,
            input: &Bound<'_, PyAny>,
        ) -> PyResult<bool> {
            let p = self.inner.prime().as_u32();
            let Some(qi) = self.inner.quasi_inverse(degree) else {
                return Ok(false);
            };
            let source_dim = qi.source_dimension();
            let target_dim = qi.target_dimension();
            crate::fp_py::with_input_slice(py, input, |in_slice| {
                checked_same_prime(in_slice.prime().as_u32(), p)?;
                checked_equal_len(in_slice.len(), target_dim)?;
                crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                    checked_same_prime(res.prime().as_u32(), p)?;
                    checked_equal_len(res.as_slice().len(), source_dim)?;
                    qi.apply(res.copy(), 1, in_slice);
                    Ok(())
                })
            })?;
            Ok(true)
        }

        pub fn __repr__(&self) -> String {
            format!(
                "HomPullback(source={}, target={}, degree_shift={})",
                self.inner.source(),
                self.inner.target(),
                self.inner.degree_shift()
            )
        }
    }

    // === §5.5 Steenrod evaluator / parser ====================================

    /// A single factor of an admissible (`A(..)`) list in a parsed Steenrod
    /// expression: either a Bockstein `b` or a Steenrod power `Sq^n`/`P^n`.
    /// Mirrors upstream's `steenrod_parser::BocksteinOrSq`. This is a faithful
    /// (complete) binding of the upstream enum; the upstream
    /// `to_adem_basis_elt` helper is `pub(crate)` and intentionally not exposed.
    #[pyclass(name = "BocksteinOrSq")]
    #[derive(Clone, Debug)]
    pub enum BocksteinOrSq {
        Bockstein {},
        Sq(u32),
    }

    impl From<::algebra::steenrod_parser::BocksteinOrSq> for BocksteinOrSq {
        fn from(value: ::algebra::steenrod_parser::BocksteinOrSq) -> Self {
            match value {
                ::algebra::steenrod_parser::BocksteinOrSq::Bockstein => Self::Bockstein {},
                ::algebra::steenrod_parser::BocksteinOrSq::Sq(x) => Self::Sq(x),
            }
        }
    }

    /// A basis element appearing in a parsed Steenrod expression. Mirrors
    /// upstream's `steenrod_parser::AlgebraBasisElt`, which is a (non-recursive)
    /// enum with four shapes. Rather than a PyO3 complex enum (one variant,
    /// `AList`, carries a `Vec<BocksteinOrSq>` of bound pyclasses) we wrap the
    /// upstream value and expose a `kind()` discriminator plus per-shape
    /// accessors, each of which raises `ValueError` when called on the wrong
    /// shape. This is a faithful, fully-inspectable binding: every field of
    /// every variant is reachable.
    #[pyclass(name = "AlgebraBasisElt")]
    #[derive(Clone)]
    pub struct AlgebraBasisElt(::algebra::steenrod_parser::AlgebraBasisElt);

    #[pymethods]
    impl AlgebraBasisElt {
        /// One of `"AList"`, `"PList"`, `"P"`, `"Q"`.
        pub fn kind(&self) -> &'static str {
            use ::algebra::steenrod_parser::AlgebraBasisElt::*;
            match self.0 {
                AList(_) => "AList",
                PList(_) => "PList",
                P(_) => "P",
                Q(_) => "Q",
            }
        }

        /// The admissible list, for an `AList` element. Raises `ValueError`
        /// otherwise.
        pub fn a_list(&self) -> PyResult<Vec<BocksteinOrSq>> {
            use ::algebra::steenrod_parser::AlgebraBasisElt::*;
            match &self.0 {
                AList(list) => Ok(list.iter().map(|&x| x.into()).collect()),
                _ => Err(PyValueError::new_err("not an AList basis element")),
            }
        }

        /// The `P(R)` partition, for a `PList` element. Raises `ValueError`
        /// otherwise.
        pub fn p_list(&self) -> PyResult<Vec<u32>> {
            use ::algebra::steenrod_parser::AlgebraBasisElt::*;
            match &self.0 {
                PList(p_part) => Ok(p_part.clone()),
                _ => Err(PyValueError::new_err("not a PList basis element")),
            }
        }

        /// The exponent `n`, for a `P` (i.e. `P^n`/`Sq^n`) element. Raises
        /// `ValueError` otherwise.
        pub fn p(&self) -> PyResult<u32> {
            use ::algebra::steenrod_parser::AlgebraBasisElt::*;
            match self.0 {
                P(x) => Ok(x),
                _ => Err(PyValueError::new_err("not a P basis element")),
            }
        }

        /// The index `k`, for a `Q` (Milnor `Q_k`) element. Raises `ValueError`
        /// otherwise.
        pub fn q(&self) -> PyResult<u32> {
            use ::algebra::steenrod_parser::AlgebraBasisElt::*;
            match self.0 {
                Q(x) => Ok(x),
                _ => Err(PyValueError::new_err("not a Q basis element")),
            }
        }

        pub fn __repr__(&self) -> String {
            format!("{:?}", self.0)
        }
    }

    /// A node of a parsed algebra expression tree. Mirrors upstream's recursive
    /// `steenrod_parser::AlgebraNode` enum (`Product`/`Sum`/`BasisElt`/`Scalar`).
    /// Because the upstream enum is recursive (`Box<Self>` children), we wrap it
    /// and expose a `kind()` discriminator plus accessors that hand back the
    /// child `AlgebraNode`s (for `Product`/`Sum`), the `AlgebraBasisElt` (for
    /// `BasisElt`), or the `int` scalar (for `Scalar`). A Python user can fully
    /// walk the tree; each accessor raises `ValueError` on the wrong shape.
    #[pyclass(name = "AlgebraNode")]
    #[derive(Clone)]
    pub struct AlgebraNode(::algebra::steenrod_parser::AlgebraNode);

    #[pymethods]
    impl AlgebraNode {
        /// One of `"Product"`, `"Sum"`, `"BasisElt"`, `"Scalar"`.
        pub fn kind(&self) -> &'static str {
            use ::algebra::steenrod_parser::AlgebraNode::*;
            match self.0 {
                Product(..) => "Product",
                Sum(..) => "Sum",
                BasisElt(_) => "BasisElt",
                Scalar(_) => "Scalar",
            }
        }

        /// The left child of a `Product`/`Sum` node. Raises `ValueError`
        /// otherwise.
        pub fn left(&self) -> PyResult<AlgebraNode> {
            use ::algebra::steenrod_parser::AlgebraNode::*;
            match &self.0 {
                Product(l, _) | Sum(l, _) => Ok(AlgebraNode((**l).clone())),
                _ => Err(PyValueError::new_err("node has no left child")),
            }
        }

        /// The right child of a `Product`/`Sum` node. Raises `ValueError`
        /// otherwise.
        pub fn right(&self) -> PyResult<AlgebraNode> {
            use ::algebra::steenrod_parser::AlgebraNode::*;
            match &self.0 {
                Product(_, r) | Sum(_, r) => Ok(AlgebraNode((**r).clone())),
                _ => Err(PyValueError::new_err("node has no right child")),
            }
        }

        /// The basis element of a `BasisElt` node. Raises `ValueError`
        /// otherwise.
        pub fn basis_element(&self) -> PyResult<AlgebraBasisElt> {
            use ::algebra::steenrod_parser::AlgebraNode::*;
            match &self.0 {
                BasisElt(b) => Ok(AlgebraBasisElt(b.clone())),
                _ => Err(PyValueError::new_err("not a BasisElt node")),
            }
        }

        /// The integer of a `Scalar` node. Raises `ValueError` otherwise.
        pub fn scalar(&self) -> PyResult<i32> {
            use ::algebra::steenrod_parser::AlgebraNode::*;
            match self.0 {
                Scalar(x) => Ok(x),
                _ => Err(PyValueError::new_err("not a Scalar node")),
            }
        }

        pub fn __repr__(&self) -> String {
            format!("{:?}", self.0)
        }
    }

    /// Parse an algebra expression string into an `AlgebraNode` tree. Raises
    /// `ValueError` on any parse failure (upstream returns `anyhow::Error`;
    /// `parse_algebra` itself never panics).
    #[pyfunction]
    pub fn parse_algebra(input: &str) -> PyResult<AlgebraNode> {
        ::algebra::steenrod_parser::parse_algebra(input)
            .map(AlgebraNode)
            .map_err(|e| PyValueError::new_err(format!("{e:#}")))
    }

    /// Parse a module expression string into the upstream `ModuleNode`, a list
    /// of `(AlgebraNode, generator_name)` pairs. Raises `ValueError` on any
    /// parse failure (upstream returns `anyhow::Error`; `parse_module` itself
    /// never panics).
    #[pyfunction]
    pub fn parse_module(input: &str) -> PyResult<Vec<(AlgebraNode, String)>> {
        ::algebra::steenrod_parser::parse_module(input)
            .map(|tree| {
                tree.into_iter()
                    .map(|(node, g)| (AlgebraNode(node), g))
                    .collect()
            })
            .map_err(|e| PyValueError::new_err(format!("{e:#}")))
    }

    // ------------------------------------------------------------------------
    // §5.2 standalone algebra-crate items: module generator parsing and
    // combinatorics free functions.
    // ------------------------------------------------------------------------

    /// The largest prime for which the `fp` crate precomputes its index map and
    /// binomial/degree tables (`MAX_PRIME` upstream). `tau_degrees`,
    /// `xi_degrees`, and `adem_relation_coefficient` look up
    /// `PRIME_TO_INDEX_MAP[p]` (and, for the latter, the binomial table) by
    /// indexing arrays of length `MAX_PRIME + 1` / `NUM_PRIMES`, so a prime
    /// above this bound — though accepted by `valid_prime` — would index out of
    /// bounds and panic. These functions therefore validate against this
    /// tighter bound and raise `ValueError`.
    const MAX_TABLE_PRIME: u32 = 251;

    /// Upper bound on the magnitude of any degree (and on the span between the
    /// smallest and largest degree) accepted by `module_gens_from_json`. Real
    /// module specifications have tiny degrees (well under a few hundred), so
    /// this cap is far above any realistic spec. It is also far below the point
    /// where upstream's `BiVec::with_capacity(min_degree, max_degree + 1)`
    /// (which eagerly allocates one `usize` *and* one `Vec<String>` for every
    /// degree in the whole `[min, max]` span) would exhaust memory and abort
    /// the process — an allocation failure that `catch_unwind` cannot catch. A
    /// cap of 1_000_000 also keeps `max_degree + 1` (computed upstream as
    /// `i32`) comfortably clear of the `i32::MAX` overflow.
    const MAX_MODULE_DEGREE: i32 = 1_000_000;

    /// Upper bound on the `degree` accepted by `inadmissible_pairs`. Upstream
    /// loops roughly `p * degree / (q * (p + 1))` times, pushing a
    /// `(u32, u32, u32)` triple each iteration, so an unbounded huge degree
    /// would allocate a multi-gigabyte `Vec` and abort the process (an OOM that
    /// `catch_unwind` cannot catch). Resolutions in practice use degrees in the
    /// hundreds, so this cap is far above realistic use; it also keeps the
    /// internal `p * (degree / q)` arithmetic well within `u32` (with the
    /// degree capped, `degree / q` shrinks as `p` grows), so the computation
    /// cannot overflow in release either.
    const MAX_INADMISSIBLE_DEGREE: i32 = 100_000;

    /// Upper bound on the magnitude of the `x`, `y`, `j`, `e1`, `e2` arguments
    /// to `adem_relation_coefficient`. Upstream casts these to `i32` and forms
    /// `(y - j) * (p - 1) + e1 - 1` and `x - p * j - e2` with `p <= 251`; with
    /// each argument capped at this bound those products stay well below
    /// `i32::MAX` (`251 * 1_000_000 < 2.6e8`), so the result is well-defined in
    /// BOTH debug (no overflow panic) and release (no silent wrap). Real Adem
    /// inputs are tiny (degrees in the hundreds), so this cap is far above
    /// realistic use.
    const MAX_ADEM_ARG: u32 = 1_000_000;

    /// Validate a prime that will be used to index the `fp` precomputed tables.
    /// Raises `ValueError` for a non-prime (via `valid_prime`) or for a prime
    /// larger than `MAX_TABLE_PRIME` (which would index out of bounds upstream).
    fn table_prime(p: u32) -> PyResult<prime::ValidPrime> {
        let prime = valid_prime(p)?;
        if p > MAX_TABLE_PRIME {
            return Err(PyValueError::new_err(format!(
                "p = {p} exceeds the largest precomputed prime ({MAX_TABLE_PRIME})"
            )));
        }
        Ok(prime)
    }

    /// Parse a module's generator specification (a JSON object mapping each
    /// generator name to its integer degree) into its graded structure.
    ///
    /// Returns `(graded_dims, names)` where
    ///   * `graded_dims` is a `dict[int, int]` mapping each degree to the number
    ///     of generators in that degree, and
    ///   * `names` is a `dict[int, list[str]]` mapping each degree to the names
    ///     of its generators (in the order they index that degree's basis).
    ///
    /// Upstream `module_gens_from_json` returns a third element: a name-lookup
    /// *closure* `&str -> Result<(i32, usize)>`. Per API_PROPOSAL §8 ("Closures
    /// returned from Rust"), returned Rust closures cannot be wrapped thinly and
    /// are intentionally dropped; the same information is recoverable from
    /// `names` (the index of a name within `names[degree]` is its basis index).
    ///
    /// The upstream function uses `unwrap`/`as_i64` and panics on a value that is
    /// not a JSON object or whose degrees are not integers, so we validate the
    /// shape up front and raise `ValueError` instead of letting it panic across
    /// the FFI boundary. (Type conversion of the Python value, in `py_to_json`,
    /// also raises `ValueError`.)
    #[pyfunction]
    pub fn module_gens_from_json(
        value: &Bound<'_, PyAny>,
    ) -> PyResult<(
        std::collections::BTreeMap<i32, usize>,
        std::collections::BTreeMap<i32, Vec<String>>,
    )> {
        let json = py_to_json(value)?;
        let obj = json.as_object().ok_or_else(|| {
            PyValueError::new_err(
                "module generator spec must be a JSON object mapping names to degrees",
            )
        })?;
        let mut min_degree: Option<i64> = None;
        let mut max_degree: Option<i64> = None;
        for (name, degree) in obj {
            let Some(degree) = degree.as_i64() else {
                return Err(PyValueError::new_err(format!(
                    "generator {name:?} must have an integer degree"
                )));
            };
            // Reject any single degree whose magnitude is so large that
            // upstream's `BiVec::with_capacity(min, max + 1)` would over-allocate
            // (or `max + 1` would overflow `i32`). See `MAX_MODULE_DEGREE`.
            if degree < i64::from(-MAX_MODULE_DEGREE) || degree > i64::from(MAX_MODULE_DEGREE) {
                return Err(PyValueError::new_err(format!(
                    "generator {name:?} has degree {degree} outside the supported \
                     range [-{MAX_MODULE_DEGREE}, {MAX_MODULE_DEGREE}]"
                )));
            }
            min_degree = Some(min_degree.map_or(degree, |m| m.min(degree)));
            max_degree = Some(max_degree.map_or(degree, |m| m.max(degree)));
        }
        // Reject an oversized degree *span*: upstream allocates the full
        // `[min, max]` range, so a spec like `{"a": -1e6, "b": 1e6}` would still
        // over-allocate even though each individual degree is within bounds.
        if let (Some(min), Some(max)) = (min_degree, max_degree) {
            if max - min > i64::from(MAX_MODULE_DEGREE) {
                return Err(PyValueError::new_err(format!(
                    "module generator degrees span {} ({min}..={max}), exceeding the \
                     supported span of {MAX_MODULE_DEGREE}",
                    max - min
                )));
            }
        }
        // Validated above, so the upstream `unwrap`/`as_i64` calls cannot panic
        // and the bounded degree span cannot over-allocate or overflow.
        let (graded_dimension, gen_names, _name_lookup) = ::algebra::module_gens_from_json(&json);
        let dims = graded_dimension
            .iter_enum()
            .map(|(degree, &dim)| (degree, dim))
            .collect();
        let names = gen_names
            .iter_enum()
            .map(|(degree, names)| (degree, names.clone()))
            .collect();
        Ok((dims, names))
    }

    /// The Adem relation coefficient for the (in)admissible pair encoded by
    /// `(x, y, j, e1, e2)` at the prime `p`, reduced mod `p`. Mirrors upstream
    /// `combinatorics::adem_relation_coefficient`.
    ///
    /// Upstream takes a `ValidPrime` and indexes the `fp` binomial table by
    /// `PRIME_TO_INDEX_MAP[p]`, so `p` is validated against `MAX_TABLE_PRIME`
    /// (raising `ValueError` otherwise). The intermediate degree arithmetic is
    /// `i32` and could overflow (panicking in a debug build) for pathologically
    /// large inputs, so the computation is run under `catch_unwind` and any such
    /// overflow is reported as `ValueError` rather than aborting across the FFI
    /// boundary.
    #[pyfunction]
    pub fn adem_relation_coefficient(
        p: u32,
        x: u32,
        y: u32,
        j: u32,
        e1: u32,
        e2: u32,
    ) -> PyResult<u32> {
        use std::panic::catch_unwind;
        let prime = table_prime(p)?;
        // Range pre-check: cap each argument so the internal `i32` degree
        // arithmetic cannot overflow for accepted inputs. Without this the
        // overflow is a silent wrap in release (overflow-checks off) and only a
        // panic in debug, so the result would otherwise be ill-defined. See
        // `MAX_ADEM_ARG`. `catch_unwind` is kept below purely as a backstop.
        for (label, arg) in [("x", x), ("y", y), ("j", j), ("e1", e1), ("e2", e2)] {
            if arg > MAX_ADEM_ARG {
                return Err(PyValueError::new_err(format!(
                    "argument {label} = {arg} exceeds the supported maximum of {MAX_ADEM_ARG}"
                )));
            }
        }
        catch_unwind(|| ::algebra::combinatorics::adem_relation_coefficient(prime, x, y, j, e1, e2))
            .map_err(|_| PyValueError::new_err("degree arithmetic overflowed for these inputs"))
    }

    /// The inadmissible `(P^i, b, P^j)` pairs in the given `degree` at the prime
    /// `p` (with `generic` selecting the odd-primary/generic relations). Each
    /// triple `(i, b, j)` denotes `P^i P^j` when `b == 0` and `P^i β P^j` when
    /// `b == 1`. Mirrors upstream `combinatorics::inadmissible_pairs`.
    ///
    /// Upstream casts `degree` to `u32` (so a negative degree would wrap to a
    /// huge value) and performs `u32` degree arithmetic that could overflow
    /// (panicking in a debug build) for pathological inputs. We require a
    /// non-negative degree and run the computation under `catch_unwind`,
    /// reporting any overflow as `ValueError`.
    #[pyfunction]
    pub fn inadmissible_pairs(
        p: u32,
        generic: bool,
        degree: i32,
    ) -> PyResult<Vec<(u32, u32, u32)>> {
        use std::panic::catch_unwind;
        let prime = valid_prime(p)?;
        // A negative degree is malformed input for this combinatorics function,
        // so raise `ValueError` (not `IndexError`). Upstream would otherwise
        // cast it to a huge `u32`.
        non_negative_degree_value(degree)?;
        // Magnitude pre-check: a huge degree makes upstream push a multi-GB
        // `Vec`, an OOM abort that `catch_unwind` cannot catch. The cap also
        // bounds the internal `u32` arithmetic, so it cannot overflow in
        // release. See `MAX_INADMISSIBLE_DEGREE`.
        if degree > MAX_INADMISSIBLE_DEGREE {
            return Err(PyValueError::new_err(format!(
                "degree {degree} exceeds the supported maximum of {MAX_INADMISSIBLE_DEGREE}"
            )));
        }
        catch_unwind(|| ::algebra::combinatorics::inadmissible_pairs(prime, generic, degree))
            .map_err(|_| PyValueError::new_err("degree arithmetic overflowed for these inputs"))
    }

    /// The degrees of the exterior generators `τ_i` of the dual Steenrod algebra
    /// at the prime `p` (the values are meaningless at `p = 2`). Mirrors
    /// upstream `combinatorics::tau_degrees`, returning the precomputed slice as
    /// a Python `list[int]`. `p` is validated against `MAX_TABLE_PRIME` since
    /// upstream indexes `PRIME_TO_INDEX_MAP[p]`.
    #[pyfunction]
    pub fn tau_degrees(p: u32) -> PyResult<Vec<i32>> {
        let prime = table_prime(p)?;
        Ok(::algebra::combinatorics::tau_degrees(prime).to_vec())
    }

    /// The degrees (divided by `q = 2p - 2`, or `1` at `p = 2`) of the
    /// polynomial generators `ξ_i` of the dual Steenrod algebra at the prime
    /// `p`. Mirrors upstream `combinatorics::xi_degrees`, returning the
    /// precomputed slice as a Python `list[int]`. `p` is validated against
    /// `MAX_TABLE_PRIME` since upstream indexes `PRIME_TO_INDEX_MAP[p]`.
    #[pyfunction]
    pub fn xi_degrees(p: u32) -> PyResult<Vec<i32>> {
        let prime = table_prime(p)?;
        Ok(::algebra::combinatorics::xi_degrees(prime).to_vec())
    }

    /// An evaluator for Steenrod algebra expressions. Wraps upstream's
    /// `steenrod_evaluator::SteenrodEvaluator`, which holds an `AdemAlgebra` and
    /// a `MilnorAlgebra` at a fixed prime and can parse + evaluate expression
    /// strings into elements, as well as change basis between the Adem and
    /// Milnor bases.
    ///
    /// `adem_element_to_string`/`milnor_element_to_string` are *not* re-bound
    /// here: they are reachable via the already-bound `AdemAlgebra` /
    /// `MilnorAlgebra` `element_to_string`. The upstream `PairAlgebra` /
    /// `pair_algebra` element type is deferred (low priority; only used by
    /// `SecondaryResolution` internals).
    #[pyclass(name = "SteenrodEvaluator")]
    pub struct SteenrodEvaluator(::algebra::steenrod_evaluator::SteenrodEvaluator);

    impl SteenrodEvaluator {
        /// Run an evaluation closure, translating both the upstream
        /// `anyhow::Error` (parse / degree-mismatch errors) and any deeper
        /// `panic!`/`unwrap` (e.g. an out-of-range `Q_k`, an inadmissible list,
        /// or a `P(R)` not present in the algebra — the evaluator reaches the
        /// panicking `basis_element_to_index`/index paths buried in the Adem and
        /// Milnor algebras for such inputs) into a clean `ValueError`. The panic
        /// is contained with `catch_unwind`: it always originates from a failed
        /// lookup, never a half-finished mutation of shared state, so no
        /// inconsistent state survives the unwind.
        fn eval(
            &self,
            f: impl FnOnce(
                &::algebra::steenrod_evaluator::SteenrodEvaluator,
            ) -> anyhow::Result<(i32, ::fp::vector::FpVector)>,
        ) -> PyResult<(i32, crate::fp_py::PyFpVector)> {
            use std::panic::{catch_unwind, AssertUnwindSafe};
            match catch_unwind(AssertUnwindSafe(|| f(&self.0))) {
                Ok(Ok((degree, vec))) => Ok((degree, crate::fp_py::PyFpVector::from_rust(vec))),
                Ok(Err(e)) => Err(PyValueError::new_err(format!("{e:#}"))),
                Err(_) => Err(PyValueError::new_err(
                    "could not evaluate Steenrod expression",
                )),
            }
        }
    }

    #[pymethods]
    impl SteenrodEvaluator {
        /// Construct an evaluator at prime `p`. Validates the prime ->
        /// `ValueError` (`ValidPrime` is never exposed).
        #[new]
        pub fn new(p: u32) -> PyResult<Self> {
            Ok(SteenrodEvaluator(
                ::algebra::steenrod_evaluator::SteenrodEvaluator::new(valid_prime(p)?),
            ))
        }

        /// The prime as a plain `int`.
        pub fn prime(&self) -> u32 {
            self.0.adem.prime().as_u32()
        }

        /// Parse and evaluate `input` in the Adem basis, returning
        /// `(degree, FpVector)`. Raises `ValueError` on a parse error, a degree
        /// mismatch, or an otherwise-invalid expression.
        pub fn evaluate_algebra_adem(
            &self,
            input: &str,
        ) -> PyResult<(i32, crate::fp_py::PyFpVector)> {
            self.eval(|ev| ev.evaluate_algebra_adem(input))
        }

        /// Parse and evaluate `input` in the Milnor basis, returning
        /// `(degree, FpVector)`. Raises `ValueError` on a parse error, a degree
        /// mismatch, or an otherwise-invalid expression.
        pub fn evaluate_algebra_milnor(
            &self,
            input: &str,
        ) -> PyResult<(i32, crate::fp_py::PyFpVector)> {
            self.eval(|ev| ev.evaluate_algebra_milnor(input))
        }

        /// Parse and evaluate a module expression `input` in the Adem basis,
        /// returning a `dict` mapping each generator name to its
        /// `(degree, FpVector)` coefficient. Raises `ValueError` on a parse
        /// error or an otherwise-invalid expression.
        ///
        /// (Upstream has only an Adem variant of `evaluate_module_*`; there is
        /// no `evaluate_module_milnor`, so none is bound.)
        pub fn evaluate_module_adem(
            &self,
            input: &str,
        ) -> PyResult<std::collections::BTreeMap<String, (i32, crate::fp_py::PyFpVector)>> {
            use std::panic::{catch_unwind, AssertUnwindSafe};
            match catch_unwind(AssertUnwindSafe(|| self.0.evaluate_module_adem(input))) {
                Ok(Ok(map)) => Ok(map
                    .into_iter()
                    .map(|(g, (degree, vec))| {
                        (g, (degree, crate::fp_py::PyFpVector::from_rust(vec)))
                    })
                    .collect()),
                Ok(Err(e)) => Err(PyValueError::new_err(format!("{e:#}"))),
                Err(_) => Err(PyValueError::new_err(
                    "could not evaluate Steenrod module expression",
                )),
            }
        }

        /// Convert an element given in the Adem basis (in degree `degree`) to
        /// the Milnor basis, returning a freshly-allocated `FpVector`. Validates
        /// the degree (non-negative), the prime, and the input length against
        /// the dimension of `degree`.
        pub fn adem_to_milnor(
            &self,
            py: Python<'_>,
            degree: i32,
            input: &Bound<'_, PyAny>,
        ) -> PyResult<crate::fp_py::PyFpVector> {
            self.change_basis(py, degree, input, true)
        }

        /// Convert an element given in the Milnor basis (in degree `degree`) to
        /// the Adem basis, returning a freshly-allocated `FpVector`. Validates
        /// the degree (non-negative), the prime, and the input length against
        /// the dimension of `degree`.
        pub fn milnor_to_adem(
            &self,
            py: Python<'_>,
            degree: i32,
            input: &Bound<'_, PyAny>,
        ) -> PyResult<crate::fp_py::PyFpVector> {
            self.change_basis(py, degree, input, false)
        }

        pub fn __repr__(&self) -> String {
            format!("SteenrodEvaluator(p={})", self.0.adem.prime().as_u32())
        }
    }

    impl SteenrodEvaluator {
        /// Shared own-output change-of-basis helper for
        /// `adem_to_milnor`/`milnor_to_adem`. Both upstream methods take a
        /// `&mut FpVector` result of the *same* dimension as the input (the Adem
        /// and Milnor bases agree dimension-wise in every degree), so we
        /// allocate the result, copy the input into an owned `FpVector`, and run
        /// upstream with `coeff = 1`.
        fn change_basis(
            &self,
            py: Python<'_>,
            degree: i32,
            input: &Bound<'_, PyAny>,
            adem_to_milnor: bool,
        ) -> PyResult<crate::fp_py::PyFpVector> {
            non_negative_degree(degree)?;
            // Populate both algebras' book-keeping so the dimension read and the
            // internal index lookups are in range.
            self.0.adem.compute_basis(degree);
            self.0.milnor.compute_basis(degree);
            let p = self.0.adem.prime();
            let dim = self.0.adem.dimension(degree);
            crate::fp_py::with_input_slice(py, input, |slice| {
                checked_same_prime(slice.prime().as_u32(), p.as_u32())?;
                checked_equal_len(slice.len(), dim)?;
                let mut owned = ::fp::vector::FpVector::new(p, dim);
                owned.as_slice_mut().assign(slice);
                let mut result = ::fp::vector::FpVector::new(p, dim);
                if adem_to_milnor {
                    self.0.adem_to_milnor(&mut result, 1, degree, &owned);
                } else {
                    self.0.milnor_to_adem(&mut result, 1, degree, &owned);
                }
                Ok(crate::fp_py::PyFpVector::from_rust(result))
            })
        }
    }

    #[pymodule_init]
    fn init(_m: &Bound<'_, PyModule>) -> PyResult<()> {
        // Arbitrary code to run at the module initialization
        // m.add("double2", m.getattr("double")?)
        Ok(())
    }
}
