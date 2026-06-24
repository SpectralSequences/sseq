use pyo3::prelude::*;

mod algebra_mod;
mod fp_mod;
mod sseq_mod;

pub use algebra_mod::algebra_py;
pub use fp_mod::fp_py;
pub use sseq_mod::sseq_py;

#[pymodule]
#[pyo3(name = "ext")]
mod ext_py {
    use std::sync::Arc;

    use algebra::{
        milnor_algebra::MilnorAlgebra,
        module::{homomorphism::ModuleHomomorphism, FDModule, Module},
        Algebra,
    };
    use ext::{
        chain_complex::{AugmentedChainComplex, ChainComplex as RsChainComplex, FreeChainComplex},
        resolution_homomorphism::ResolutionHomomorphism as RsResolutionHomomorphism,
        secondary::SecondaryLift,
        utils::Config,
        CCC,
    };
    use fp::prime::Prime;
    use sseq::coordinates::Bidegree as RsBidegree;

    #[pymodule_export]
    use super::algebra_py;
    #[pymodule_export]
    use super::fp_py;
    #[pymodule_export]
    use super::sseq_py;
    use super::*;

    /// A monomorphized union of the two concrete resolution types. The two algorithms produce
    /// resolutions over different algebras (`MilnorAlgebra` vs `SteenrodAlgebra`), which are
    /// distinct associated types of `ChainComplex`, so they cannot share a `dyn` trait object.
    /// We therefore erase the difference with this enum and dispatch via `match`.
    enum AnyResolution {
        Nassau(Arc<ext::nassau::Resolution<FDModule<MilnorAlgebra>>>),
        Standard(Arc<ext::resolution::Resolution<ext::CCC>>),
    }

    /// Dispatch a `match` over both variants, binding the inner `Arc` to `$r` in each arm.
    macro_rules! dispatch {
        ($self:expr, $r:ident => $body:expr) => {
            match $self {
                AnyResolution::Nassau($r) => $body,
                AnyResolution::Standard($r) => $body,
            }
        };
    }

    /// Build a resolution, choosing Nassau's special algorithm or the general one at runtime.
    ///
    /// `algorithm` may be `None`/`"auto"` (try Nassau, fall back to the general algorithm),
    /// `"nassau"` (force Nassau, error if the module is ineligible), or `"standard"` (force the
    /// general algorithm).
    ///
    /// Error taxonomy (see fixes): bad-argument conditions map to `ValueError`, genuine
    /// internal/IO failures to `RuntimeError`.
    ///  - Unknown `algorithm` string -> `ValueError`.
    ///  - Forcing `"nassau"` on an ineligible module -> `ValueError`: with `save_dir = None`,
    ///    every `construct_nassau` failure is caused by the caller's input (algebra/profile/
    ///    prime/finite-dimensionality/cofiber eligibility checks, or malformed module JSON),
    ///    so the opaque `anyhow::Error` is reported as a bad argument.
    ///  - `"standard"`/`"auto"` build failures -> `RuntimeError` (may be internal/IO).
    fn build(spec: Config, algorithm: Option<&str>) -> PyResult<AnyResolution> {
        use ext::utils::{construct_nassau, construct_standard};

        let nassau =
            |spec| construct_nassau(spec, None).map(|r| AnyResolution::Nassau(Arc::new(r)));
        let standard = |spec| {
            construct_standard::<false, _, _>(spec, None)
                .map(|r| AnyResolution::Standard(Arc::new(r)))
        };
        let value_err = |e: anyhow::Error| pyo3::exceptions::PyValueError::new_err(e.to_string());
        let runtime_err =
            |e: anyhow::Error| pyo3::exceptions::PyRuntimeError::new_err(e.to_string());

        match algorithm {
            // Eligibility/bad-argument: report as ValueError.
            Some("nassau") => nassau(spec).map_err(value_err),
            Some("standard") => standard(spec).map_err(runtime_err),
            None | Some("auto") => match nassau(spec.clone()) {
                Ok(res) => Ok(res),
                // `auto` intentionally falls back to the general algorithm on ANY Nassau error,
                // not just eligibility errors. Nassau rejects ineligible modules up front, so in
                // practice the discarded error is an eligibility check; a genuinely malformed
                // module is surfaced by the general algorithm's own error below.
                Err(_) => standard(spec).map_err(runtime_err),
            },
            Some(other) => Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Unknown algorithm {other:?}; expected \"auto\", \"nassau\", or \"standard\""
            ))),
        }
    }

    #[pyfunction]
    pub fn query_module(
        algebra_type: Option<algebra_py::AlgebraType>,
        save: bool,
    ) -> PyResult<Resolution> {
        ext::utils::query_module(algebra_type.map(algebra::AlgebraType::from), save)
            .map(|res| Resolution(AnyResolution::Standard(Arc::new(res))))
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    #[pyfunction]
    pub fn query_module_only(
        prompt: &str,
        algebra: Option<algebra_py::AlgebraType>,
        load_quasi_inverse: bool,
    ) -> PyResult<Resolution> {
        ext::utils::query_module_only(
            prompt,
            algebra.map(algebra::AlgebraType::from),
            load_quasi_inverse,
        )
        .map(|res| Resolution(AnyResolution::Standard(Arc::new(res))))
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    impl AnyResolution {
        /// Clone the inner `Arc` (a cheap refcount bump), producing a second
        /// handle to the *same* resolution. Used to hand an owned
        /// `AnyResolution` to a `ResolutionStemIterator` so the iterator can
        /// live in a `#[pyclass]` without borrowing the `Resolution`.
        fn clone_ref(&self) -> AnyResolution {
            match self {
                AnyResolution::Nassau(r) => AnyResolution::Nassau(Arc::clone(r)),
                AnyResolution::Standard(r) => AnyResolution::Standard(Arc::clone(r)),
            }
        }
    }

    #[pyclass(frozen)]
    pub struct Resolution(AnyResolution);

    impl Resolution {
        /// Number of generators of the resolution at bidegree `b`, returning 0
        /// (never panicking) for any bidegree outside the computed range.
        ///
        /// Upstream `FreeChainComplex::number_of_gens_in_bidegree` is
        /// `self.module(b.s()).number_of_gens_in_degree(b.t())`, and BOTH
        /// indexing steps panic out of range: `module(s)` indexes the resolution's
        /// `modules` `OnceBiVec` (panicking for `s >= next_homological_degree()`),
        /// and `number_of_gens_in_degree(t)` indexes the module's `num_gens`
        /// `OnceBiVec` (panicking for `t > max_computed_degree()`). This mirrors
        /// the `algebra_py.FreeModule::num_gens_safe` guard: clamp both axes to
        /// the populated range and read 0 outside it.
        fn num_gens_at(&self, b: RsBidegree) -> usize {
            if b.s() < 0 || b.t() < 0 {
                return 0;
            }
            dispatch!(&self.0, r => {
                if b.s() >= r.next_homological_degree() {
                    0
                } else {
                    let m = r.module(b.s());
                    if b.t() < m.min_degree() || b.t() > m.max_computed_degree() {
                        0
                    } else {
                        m.number_of_gens_in_degree(b.t())
                    }
                }
            })
        }
    }

    #[pymethods]
    impl Resolution {
        /// Construct a resolution of the given module specification, dispatching to Nassau's
        /// algorithm or the general algorithm at runtime.
        #[new]
        #[pyo3(signature = (spec, algorithm=None))]
        pub fn new(spec: &str, algorithm: Option<&str>) -> PyResult<Self> {
            let config: Config = spec.try_into().map_err(|e: anyhow::Error| {
                pyo3::exceptions::PyValueError::new_err(e.to_string())
            })?;
            build(config, algorithm).map(Resolution)
        }

        /// Resolve through the given target bidegree.
        ///
        /// The target must be a non-negative bidegree: both algorithms allocate a
        /// `vec![..; max.s() + 1]` and kickstart from `t = -1`, so a negative `s` over-allocates
        /// (`max.s() as usize` wraps) and a negative `t`/`s` trips internal `assert!`/`panic!`s
        /// in the resolve loop. Validate the Python input here and raise a clean `ValueError`
        /// instead of panicking across the FFI boundary.
        pub fn compute_through_stem(&self, max: sseq_py::Bidegree) -> PyResult<()> {
            let b = max.0;
            if b.s() < 0 || b.t() < 0 {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "invalid target bidegree {b}: require s >= 0 and t >= 0"
                )));
            }
            dispatch!(&self.0, r => r.compute_through_stem(b));
            Ok(())
        }

        pub fn graded_dimension_string(&self) -> String {
            dispatch!(&self.0, r => r.graded_dimension_string())
        }

        /// The `E_2`-page of the resolution as a bound `sseq_py.Sseq`.
        ///
        /// This is a `FreeChainComplex` method (a resolution's modules are free,
        /// so it implements `FreeChainComplex`; the bare `ChainComplex` pyclass
        /// over `CCC` does not — its modules are arbitrary `SteenrodModule`s).
        /// Upstream `to_sseq` only ever queries bidegrees yielded by
        /// `iter_stem`, all of which lie in the computed range, so it is
        /// panic-free over the range resolved so far.
        pub fn to_sseq(&self) -> sseq_py::Sseq {
            let p = dispatch!(&self.0, r => r.prime());
            let sseq = dispatch!(&self.0, r => r.to_sseq());
            sseq_py::Sseq::from_rust(sseq, p)
        }

        /// The chain complex this resolution resolves, as a bound `ChainComplex`
        /// (`CCC`), sharing the same `Arc`.
        ///
        /// Only the standard backend resolves a `CCC`; Nassau's algorithm
        /// resolves a different (monomorphised) complex type that the
        /// `ChainComplex` pyclass cannot represent, so it is rejected with a
        /// `ValueError`.
        pub fn chain_complex(&self) -> PyResult<ChainComplex> {
            match &self.0 {
                AnyResolution::Standard(r) => Ok(ChainComplex(r.target())),
                AnyResolution::Nassau(_) => Err(pyo3::exceptions::PyValueError::new_err(
                    "chain_complex() is only available on the standard backend; Nassau resolves a \
                     different complex type that the ChainComplex pyclass (CCC) cannot represent. \
                     Construct the Resolution with algorithm='standard'.",
                )),
            }
        }

        /// Resolve through the given target bidegree (fixed `t`, as opposed to
        /// `compute_through_stem`'s fixed stem). Validates `s >= 0`/`t >= 0`,
        /// raising `ValueError` rather than risking an internal panic (cf.
        /// `compute_through_stem`).
        pub fn compute_through_bidegree(&self, max: sseq_py::Bidegree) -> PyResult<()> {
            let b = max.0;
            if b.s() < 0 || b.t() < 0 {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "invalid target bidegree {b}: require s >= 0 and t >= 0"
                )));
            }
            dispatch!(&self.0, r => r.compute_through_bidegree(b));
            Ok(())
        }

        /// As [`compute_through_bidegree`], but invoke `callback(bidegree)` once
        /// for each *newly* computed bidegree (matching the upstream
        /// `compute_through_bidegree_with_callback`, whose `cb: FnMut(Bidegree)`
        /// fires only when `new` is true). The callback receives an
        /// `sseq_py.Bidegree`.
        ///
        /// **Standard backend only.** The callback variants live on
        /// `ext::resolution::Resolution`; the Nassau algorithm exposes no
        /// per-bidegree callback hook (its `ChainComplex::compute_through_bidegree`
        /// is a plain double loop). A Nassau-backed resolution raises `ValueError`;
        /// use `compute_through_bidegree` (no callback) instead, or construct with
        /// `algorithm='standard'`.
        ///
        /// If the callback raises, the exception is captured and re-raised after
        /// the computation finishes (the in-flight upstream iteration cannot be
        /// unwound across the FFI boundary, so we record the first error, ignore
        /// later callbacks, and propagate it once control returns).
        ///
        /// **Do not re-enter this resolution from the callback.** Upstream runs
        /// the callback while holding the resolution's internal lock, so calling
        /// any `compute_through_*` on the *same* `Resolution` from within the
        /// callback deadlocks.
        pub fn compute_through_bidegree_with_callback(
            &self,
            py: Python<'_>,
            max: sseq_py::Bidegree,
            callback: Py<PyAny>,
        ) -> PyResult<()> {
            let b = max.0;
            if b.s() < 0 || b.t() < 0 {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "invalid target bidegree {b}: require s >= 0 and t >= 0"
                )));
            }
            let r = match &self.0 {
                AnyResolution::Standard(r) => r,
                AnyResolution::Nassau(_) => {
                    return Err(pyo3::exceptions::PyValueError::new_err(
                        "compute_through_bidegree_with_callback is only available on the standard \
                         backend; the Nassau algorithm has no per-bidegree callback hook. Use \
                         compute_through_bidegree (no callback) or algorithm='standard'.",
                    ));
                }
            };
            let mut err: Option<PyErr> = None;
            r.compute_through_bidegree_with_callback(b, |bd| {
                if err.is_some() {
                    return;
                }
                if let Err(e) = callback.call1(py, (sseq_py::Bidegree(bd),)) {
                    err = Some(e);
                }
            });
            match err {
                Some(e) => Err(e),
                None => Ok(()),
            }
        }

        /// As [`compute_through_stem`], but invoke `callback(bidegree)` once for
        /// each newly computed bidegree. See
        /// [`compute_through_bidegree_with_callback`] for the callback exception
        /// semantics, the standard-backend-only restriction, and the
        /// re-entrancy deadlock warning.
        pub fn compute_through_stem_with_callback(
            &self,
            py: Python<'_>,
            max: sseq_py::Bidegree,
            callback: Py<PyAny>,
        ) -> PyResult<()> {
            let b = max.0;
            if b.s() < 0 || b.t() < 0 {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "invalid target bidegree {b}: require s >= 0 and t >= 0"
                )));
            }
            let r = match &self.0 {
                AnyResolution::Standard(r) => r,
                AnyResolution::Nassau(_) => {
                    return Err(pyo3::exceptions::PyValueError::new_err(
                        "compute_through_stem_with_callback is only available on the standard \
                         backend; the Nassau algorithm has no per-bidegree callback hook. Use \
                         compute_through_stem (no callback) or algorithm='standard'.",
                    ));
                }
            };
            let mut err: Option<PyErr> = None;
            r.compute_through_stem_with_callback(b, |bd| {
                if err.is_some() {
                    return;
                }
                if let Err(e) = callback.call1(py, (sseq_py::Bidegree(bd),)) {
                    err = Some(e);
                }
            });
            match err {
                Some(e) => Err(e),
                None => Ok(()),
            }
        }

        /// The prime as a plain `int`.
        pub fn prime(&self) -> u32 {
            dispatch!(&self.0, r => r.prime().as_u32())
        }

        /// The minimum internal degree of the resolution's modules.
        pub fn min_degree(&self) -> i32 {
            dispatch!(&self.0, r => r.min_degree())
        }

        /// The first `s` for which `module(s)` is not yet defined (i.e. the
        /// number of homological degrees resolved so far).
        pub fn next_homological_degree(&self) -> i32 {
            dispatch!(&self.0, r => r.next_homological_degree())
        }

        /// Whether the resolution has been computed at bidegree `b`. Negative
        /// `s`/`t` is rejected with a `ValueError` rather than wrapping to a huge
        /// `usize`.
        pub fn has_computed_bidegree(&self, b: sseq_py::Bidegree) -> PyResult<bool> {
            if b.0.s() < 0 || b.0.t() < 0 {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "invalid bidegree {}: require s >= 0 and t >= 0",
                    b.0
                )));
            }
            Ok(dispatch!(&self.0, r => r.has_computed_bidegree(b.0)))
        }

        /// The number of generators of the resolution at bidegree `b` (the
        /// dimension of `Ext` there). Returns 0 for any uncomputed or
        /// out-of-range bidegree; raises `ValueError` for negative `s`/`t`.
        ///
        /// Both backends' modules' generator tables (`OnceBiVec`s) panic when
        /// indexed out of range, so this is guarded; see `num_gens_at`.
        pub fn number_of_gens_in_bidegree(&self, b: sseq_py::Bidegree) -> PyResult<usize> {
            if b.0.s() < 0 || b.0.t() < 0 {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "invalid bidegree {}: require s >= 0 and t >= 0",
                    b.0
                )));
            }
            Ok(self.num_gens_at(b.0))
        }

        /// The resolution's `s`-th free module, as a bound `algebra_py.FreeModule`
        /// sharing its `Arc`.
        ///
        /// Only the standard backend's modules are over the `SteenrodAlgebra`
        /// union the `FreeModule` pyclass wraps. Nassau's modules are over the
        /// concrete `MilnorAlgebra` (a distinct, non-interconvertible type
        /// parameter), so the pyclass cannot represent them; `module()` rejects
        /// the Nassau backend with a `ValueError`, matching `chain_complex()`.
        ///
        /// Raises `ValueError` for negative `s` or `s` beyond the resolved range
        /// (`>= next_homological_degree()`); indexing the modules `OnceBiVec`
        /// there would otherwise panic.
        pub fn module(&self, s: i32) -> PyResult<algebra_py::FreeModule> {
            if s < 0 {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "homological degree s must be non-negative",
                ));
            }
            match &self.0 {
                AnyResolution::Standard(r) => {
                    if s >= r.next_homological_degree() {
                        return Err(pyo3::exceptions::PyValueError::new_err(format!(
                            "module index s = {s} is beyond the resolved range (next homological \
                             degree is {}); compute_through_bidegree / compute_through_stem first",
                            r.next_homological_degree()
                        )));
                    }
                    Ok(algebra_py::FreeModule::from_arc(r.module(s)))
                }
                AnyResolution::Nassau(_) => Err(pyo3::exceptions::PyValueError::new_err(
                    "module() is only available on the standard backend; Nassau resolves over the \
                     concrete MilnorAlgebra, whose FreeModule the algebra_py.FreeModule pyclass \
                     (over the SteenrodAlgebra union) cannot represent. Construct the Resolution \
                     with algorithm='standard'.",
                )),
            }
        }

        /// The full filtration-one product (e.g. `h_0`, `h_1`, ...) given by the
        /// algebra operation of degree `op_deg` and index `op_idx`, as a bound
        /// `sseq_py.Product` over the range resolved so far.
        ///
        /// Upstream iterates only over `has_computed_bidegree` bidegrees, so it is
        /// panic-free over the computed range. `op_deg`/`op_idx` must index a
        /// valid algebra operation (e.g. `op_deg = 2^i`, `op_idx = 0` for `h_i`
        /// at the prime 2); `op_deg` is required to be non-negative.
        ///
        /// Two extra guards beyond the upstream method (which binds the stable,
        /// `U = false` resolutions here):
        ///  - Upstream evaluates `self.module(0).max_computed_degree()`
        ///    unconditionally, indexing the `modules` `OnceBiVec` at `0`. A
        ///    freshly constructed (never-resolved) resolution has no modules
        ///    (`next_homological_degree() == 0`), so this would panic; we instead
        ///    short-circuit to the empty/zero-length product (an uncomputed
        ///    resolution has no products).
        ///  - Upstream's `op_idx >= dimension` bounds check is gated behind
        ///    `if U`, so for these stable resolutions an out-of-range `op_idx`
        ///    flows into `FreeModule::operation_generator_to_index` and then
        ///    `FpVector::entry`, panicking for large `op_idx` or silently reading
        ///    a neighbouring generator's coefficient for moderate `op_idx`. We
        ///    pre-validate `op_idx` against the algebra's operation dimension in
        ///    degree `op_deg` (`IndexError`), mirroring `algebra_py`'s
        ///    `checked_op_index`.
        pub fn filtration_one_products(
            &self,
            op_deg: i32,
            op_idx: usize,
        ) -> PyResult<sseq_py::Product> {
            if op_deg < 0 {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "op_deg must be non-negative",
                ));
            }
            // Range-check op_idx against the number of algebra operations in
            // degree op_deg. compute_basis is idempotent and ensures dimension()
            // does not index its basis table out of range.
            let dim = dispatch!(&self.0, r => {
                let alg = r.algebra();
                alg.compute_basis(op_deg);
                alg.dimension(op_deg)
            });
            if op_idx >= dim {
                return Err(pyo3::exceptions::PyIndexError::new_err(format!(
                    "op_idx {op_idx} out of range for op_deg {op_deg} (algebra dimension {dim})"
                )));
            }
            // An uncomputed resolution has no modules; upstream would panic
            // indexing module(0). Return the empty product directly.
            if dispatch!(&self.0, r => r.next_homological_degree()) == 0 {
                return Ok(sseq_py::Product(::sseq::Product {
                    b: RsBidegree::x_y(op_deg - 1, 1),
                    left: true,
                    matrices: ::once::MultiIndexed::new(),
                }));
            }
            let product = dispatch!(&self.0, r => r.filtration_one_products(op_deg, op_idx));
            Ok(sseq_py::Product(product))
        }

        /// The single filtration-one product matrix out of `source`, as a list of
        /// rows (one per source generator) of `u32` entries, or `None` if the
        /// target bidegree `source + (1, op_deg)` has not been computed.
        ///
        /// Mirrors upstream `filtration_one_product`, which returns the nested
        /// `Vec<Vec<u32>>` directly and is guarded by its own
        /// `has_computed_bidegree` check. Raises `ValueError` for negative
        /// `source` coordinates or negative `op_deg`, and `IndexError` for an
        /// out-of-range `op_idx`.
        ///
        /// Upstream's `op_idx >= dimension` bounds check is gated behind `if U`,
        /// so for the stable (`U = false`) resolutions bound here an out-of-range
        /// `op_idx` would flow into `FreeModule::operation_generator_to_index`
        /// and then `FpVector::entry`, panicking for large `op_idx` or silently
        /// reading a neighbouring generator's coefficient for moderate `op_idx`.
        /// We delegate that check to upstream `try_filtration_one_product`, which
        /// errors (rather than panicking or misreading) for an out-of-range
        /// `op_idx` or an uncomputed target bidegree.
        pub fn filtration_one_product(
            &self,
            op_deg: i32,
            op_idx: usize,
            source: sseq_py::Bidegree,
        ) -> PyResult<Option<Vec<Vec<u32>>>> {
            if op_deg < 0 {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "op_deg must be non-negative",
                ));
            }
            if source.0.s() < 0 || source.0.t() < 0 {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "invalid source bidegree {}: require s >= 0 and t >= 0",
                    source.0
                )));
            }
            // Reject a source whose target degree `source.t() + op_deg` overflows
            // i32 before it can be used to index any module/FpVector.
            if source.0.t().checked_add(op_deg).is_none() {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "target degree source.t() + op_deg = {} + {op_deg} overflows i32",
                    source.0.t()
                )));
            }
            // `try_filtration_one_product` performs the op_idx range check that
            // previously needed an ad-hoc pre-check. Its error is either an
            // out-of-range `op_idx` (a caller error -> `IndexError`) or an
            // uncomputed target bidegree (documented here as `None`).
            match dispatch!(&self.0, r => r.try_filtration_one_product(op_deg, op_idx, source.0)) {
                Ok(products) => Ok(Some(products)),
                Err(e) => {
                    let msg = e.to_string();
                    if msg.contains("out of range") {
                        Err(pyo3::exceptions::PyIndexError::new_err(msg))
                    } else {
                        Ok(None)
                    }
                }
            }
        }

        /// A string representation of `d(g)`, the differential applied to the
        /// generator `g = (s, t, idx)`. Raises `ValueError` if `g` lies outside
        /// the computed range or `idx` exceeds the number of generators there
        /// (upstream would otherwise panic indexing the differential's output
        /// table).
        pub fn boundary_string(&self, g: sseq_py::BidegreeGenerator) -> PyResult<String> {
            let gen = g.0;
            if gen.s() < 0 || gen.t() < 0 {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "invalid generator {gen}: require s >= 0 and t >= 0"
                )));
            }
            let ngens = self.num_gens_at(gen.degree());
            if gen.idx() >= ngens {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "generator index {} out of range at bidegree {} ({ngens} generators, or the \
                     bidegree is uncomputed)",
                    gen.idx(),
                    gen.degree()
                )));
            }
            Ok(dispatch!(&self.0, r => r.boundary_string(gen)))
        }

        /// Iterate over the defined bidegrees in increasing order of stem. The
        /// iterator yields `sseq_py.Bidegree`s and holds its own `Arc` handle to
        /// the resolution (so the `Resolution` may be dropped while it is alive).
        ///
        /// The iteration is bounded by the resolved range (each module's
        /// `max_computed_degree` and `next_homological_degree`), so unlike
        /// `ChainComplex.iter_stem` it terminates; it is still exposed lazily.
        pub fn iter_stem(&self) -> ResolutionStemIterator {
            ResolutionStemIterator::new(self.0.clone_ref(), false)
        }

        /// As [`iter_stem`], but yield only bidegrees with a nonzero number of
        /// generators (the nonzero entries of the `Ext` chart).
        pub fn iter_nonzero_stem(&self) -> ResolutionStemIterator {
            ResolutionStemIterator::new(self.0.clone_ref(), true)
        }

        /// The resolution's name (used in tracing/logging). Both backends store a
        /// plain `String` name.
        ///
        /// The companion `set_name` is intentionally **not** bound: it takes
        /// `&mut self` upstream, but the `Resolution` pyclass is `frozen` and
        /// wraps the resolution in a (shareable) `Arc`, so no exclusive `&mut`
        /// reference is obtainable to mutate the name in place.
        pub fn name(&self) -> String {
            dispatch!(&self.0, r => r.name().to_string())
        }
    }

    /// The lazy iterator returned by [`Resolution::iter_stem`] /
    /// [`Resolution::iter_nonzero_stem`]. Holds an owned `AnyResolution` (a
    /// cloned `Arc`) and dispatches over both backends, re-implementing the
    /// upstream `chain_complex::StemIterator` walk so it can live in a
    /// `#[pyclass]` without borrowing the resolution. When `nonzero` is set it
    /// additionally skips bidegrees with no generators.
    #[pyclass]
    pub struct ResolutionStemIterator {
        res: AnyResolution,
        current: RsBidegree,
        max_s: i32,
        nonzero: bool,
    }

    impl ResolutionStemIterator {
        fn new(res: AnyResolution, nonzero: bool) -> Self {
            let min_degree = dispatch!(&res, r => r.min_degree());
            let max_s = dispatch!(&res, r => r.next_homological_degree());
            ResolutionStemIterator {
                res,
                current: RsBidegree::n_s(min_degree, 0),
                max_s,
                nonzero,
            }
        }

        /// The raw (unfiltered) stem walk, mirroring upstream `StemIterator`.
        fn raw_next(&mut self) -> Option<RsBidegree> {
            loop {
                if self.max_s == 0 {
                    return None;
                }
                let cur = self.current;
                if cur.s() == self.max_s {
                    self.current = RsBidegree::n_s(cur.n() + 1, 0);
                    continue;
                }
                let max_deg = dispatch!(&self.res, r => r.module(cur.s()).max_computed_degree());
                if cur.t() > max_deg {
                    if cur.s() == 0 {
                        return None;
                    } else {
                        self.current = RsBidegree::n_s(cur.n() + 1, 0);
                        continue;
                    }
                }
                self.current = cur + RsBidegree::n_s(0, 1);
                return Some(cur);
            }
        }
    }

    #[pymethods]
    impl ResolutionStemIterator {
        fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
            slf
        }

        fn __next__(&mut self) -> Option<sseq_py::Bidegree> {
            loop {
                let b = self.raw_next()?;
                if !self.nonzero {
                    return Some(sseq_py::Bidegree(b));
                }
                let n = dispatch!(&self.res, r => r.number_of_gens_in_bidegree(b));
                if n > 0 {
                    return Some(sseq_py::Bidegree(b));
                }
            }
        }
    }

    /// The concrete resolution homomorphism type bound here: a stable
    /// (`U = false`) chain map between two *standard*-backend resolutions of the
    /// default complex `CCC`. Both source and target are
    /// `ext::resolution::Resolution<CCC>` (the type held by
    /// `AnyResolution::Standard`).
    ///
    /// Only this Standard→Standard instantiation is bound. A
    /// `ResolutionHomomorphism` is generic over its source/target chain
    /// complexes; Nassau resolutions are over the concrete `MilnorAlgebra` (a
    /// distinct associated `Algebra` type), so a Nassau-backed source/target
    /// would be a *different* concrete `ResolutionHomomorphism<…>` whose
    /// `get_map` returns a `FreeModuleHomomorphism` over a `MilnorAlgebra`
    /// `FreeModule` that the bound `FreeModuleHomomorphismToFree` pyclass (over
    /// the `SteenrodAlgebra` union) cannot represent. We therefore reject
    /// Nassau-backed arguments with a `ValueError`, mirroring the standard-only
    /// precedent set by `Resolution.module` / `Resolution.chain_complex`.
    type RsResHom = RsResolutionHomomorphism<
        ext::resolution::Resolution<CCC>,
        ext::resolution::Resolution<CCC>,
    >;

    /// A lifted chain map between two (standard-backend) resolutions — i.e. a
    /// map of `Ext` modules realised on the level of free resolutions. Used to
    /// represent multiplication by an `Ext` class (`from_class`), and as a
    /// building block for products / Massey products.
    ///
    /// Held by value (not behind an extra `Arc`): every mutating method
    /// (`extend*`) takes `&self` upstream via the maps' interior-mutable
    /// `OnceBiVec`, so a `frozen` pyclass works directly; `source()`/`target()`
    /// hand back the resolution `Arc`s the homomorphism already stores.
    #[pyclass(frozen)]
    pub struct ResolutionHomomorphism(RsResHom);

    impl ResolutionHomomorphism {
        /// Extract the Standard-backend `Arc` from a bound `Resolution`, or
        /// raise `ValueError` for a Nassau-backed one (see `RsResHom`).
        fn standard_arc(
            res: &Resolution,
            which: &str,
        ) -> PyResult<Arc<ext::resolution::Resolution<CCC>>> {
            match &res.0 {
                AnyResolution::Standard(r) => Ok(Arc::clone(r)),
                AnyResolution::Nassau(_) => Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "ResolutionHomomorphism requires standard-backend resolutions; the {which} \
                     resolution is Nassau-backed (over the concrete MilnorAlgebra, whose maps the \
                     bound homomorphism pyclasses cannot represent). Construct it with \
                     algorithm='standard'."
                ))),
            }
        }

        /// Number of generators of the target resolution at bidegree `b`,
        /// returning 0 (never panicking) outside the computed range. Mirrors
        /// `Resolution::num_gens_at`.
        fn target_num_gens(&self, b: RsBidegree) -> usize {
            if b.s() < 0 || b.t() < 0 || b.s() >= self.0.target.next_homological_degree() {
                return 0;
            }
            let m = self.0.target.module(b.s());
            if b.t() < m.min_degree() || b.t() > m.max_computed_degree() {
                0
            } else {
                m.number_of_gens_in_degree(b.t())
            }
        }

        /// Pre-flight guard for the `extend*` family. `extend_profile` first
        /// calls `get_map_ensure_length(max_s)` — which builds the intermediate
        /// maps by indexing `source.module(s)` / `target.module(s - shift_s)`
        /// for every `s` in `shift_s..=max_s` (panicking if either module is
        /// undefined) — and then drives `iter_s_t`, whose `extend_step_raw`
        /// asserts `source.has_computed_bidegree(input)` and
        /// `target.has_computed_bidegree(input - shift)` for *every* touched
        /// bidegree. The touched set is exactly `{(s, t) : shift_s <= s <= max_s,
        /// min_t <= t <= t_max(s)}` (see `iter_s_t`/`BidegreeRange`), so we
        /// verify that whole grid is resolved up front, raising `ValueError`
        /// rather than letting an upstream `assert!` panic across FFI.
        fn check_extend_range(&self, max_s: i32, t_max: impl Fn(i32) -> i32) -> PyResult<()> {
            let shift = self.0.shift;
            if max_s < shift.s() {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "target homological degree s = {max_s} is below the homomorphism's shift \
                     s = {} (nothing to extend)",
                    shift.s()
                )));
            }
            if max_s >= self.0.source.next_homological_degree() {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "source not resolved through homological degree s = {max_s} (next homological \
                     degree is {}); resolve the source further first",
                    self.0.source.next_homological_degree()
                )));
            }
            if max_s - shift.s() >= self.0.target.next_homological_degree() {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "target not resolved through homological degree s = {} (next homological \
                     degree is {}); resolve the target further first",
                    max_s - shift.s(),
                    self.0.target.next_homological_degree()
                )));
            }
            let min_t = self.0.source.min_degree();
            for s in shift.s()..=max_s {
                let hi = t_max(s);
                for t in min_t..=hi {
                    let input = RsBidegree::s_t(s, t);
                    if !self.0.source.has_computed_bidegree(input) {
                        return Err(pyo3::exceptions::PyValueError::new_err(format!(
                            "source not computed at bidegree (s={s}, t={t}), which is required to \
                             extend the homomorphism over this range; resolve the source further"
                        )));
                    }
                    if !self.0.target.has_computed_bidegree(input - shift) {
                        let o = input - shift;
                        return Err(pyo3::exceptions::PyValueError::new_err(format!(
                            "target not computed at bidegree (s={}, t={}), which is required to \
                             extend the homomorphism over this range; resolve the target further",
                            o.s(),
                            o.t()
                        )));
                    }
                }
            }
            Ok(())
        }
    }

    #[pymethods]
    impl ResolutionHomomorphism {
        /// Construct an (initially empty) resolution homomorphism `source ->
        /// target` of the given bidegree `shift` and `name`. The map is defined
        /// on no generators yet; populate it with `from_class` / `extend_step`
        /// (not bound — see module notes) or call an `extend*` method to fill it
        /// in by exactness (yielding the zero map from an empty `new`).
        ///
        /// Both resolutions must be standard-backend (Nassau → `ValueError`) and
        /// share the same prime. `shift` must be non-negative in both `s` and
        /// `t` (a resolution homomorphism raises homological/internal degree; a
        /// negative shift is rejected rather than risking a wrapped index).
        #[new]
        pub fn new(
            name: String,
            source: &Resolution,
            target: &Resolution,
            shift: sseq_py::Bidegree,
        ) -> PyResult<Self> {
            let s = Self::standard_arc(source, "source")?;
            let t = Self::standard_arc(target, "target")?;
            if s.prime().as_u32() != t.prime().as_u32() {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "source and target resolutions are over different primes ({} != {})",
                    s.prime().as_u32(),
                    t.prime().as_u32()
                )));
            }
            if shift.0.s() < 0 || shift.0.t() < 0 {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "invalid shift {}: require s >= 0 and t >= 0",
                    shift.0
                )));
            }
            Ok(ResolutionHomomorphism(RsResHom::new(name, s, t, shift.0)))
        }

        /// Build the resolution homomorphism representing (multiplication by)
        /// the `Ext` class `class` living at bidegree `shift` in `source`: the
        /// map of `shift` sending the `k`-th generator at `shift` to
        /// `class[k]` times the fundamental class of `target`. This is the
        /// `from_class` constructor used to set up product / Massey-product
        /// computations.
        ///
        /// Validates (all `ValueError`/`IndexError`, never a panic):
        ///  - both resolutions standard-backend and same prime (as `new`);
        ///  - `shift` non-negative;
        ///  - `source` computed at `shift` (else indexing its module/generator
        ///    table would panic), and `len(class)` equals the number of source
        ///    generators there (upstream `assert_eq!`);
        ///  - `target` computed at bidegree `(0, 0)` — upstream maps the class
        ///    through the target's augmentation at `(0,0)` (`output = shift -
        ///    shift`); and that augmentation is 1-dimensional in degree 0 (the
        ///    unit/sphere case the single-column class matrix assumes), else the
        ///    quasi-inverse application would mismatch dimensions.
        #[staticmethod]
        pub fn from_class(
            name: String,
            source: &Resolution,
            target: &Resolution,
            shift: sseq_py::Bidegree,
            class: Vec<u32>,
        ) -> PyResult<Self> {
            let s = Self::standard_arc(source, "source")?;
            let t = Self::standard_arc(target, "target")?;
            if s.prime().as_u32() != t.prime().as_u32() {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "source and target resolutions are over different primes ({} != {})",
                    s.prime().as_u32(),
                    t.prime().as_u32()
                )));
            }
            let b = shift.0;
            if b.s() < 0 || b.t() < 0 {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "invalid shift {b}: require s >= 0 and t >= 0"
                )));
            }
            if !s.has_computed_bidegree(b) {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "source not computed at the class bidegree (s={}, t={}); resolve it there first",
                    b.s(),
                    b.t()
                )));
            }
            let num_gens = s.module(b.s()).number_of_gens_in_degree(b.t());
            if class.len() != num_gens {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "class has length {} but the source has {num_gens} generator(s) at bidegree \
                     (s={}, t={})",
                    class.len(),
                    b.s(),
                    b.t()
                )));
            }
            // Upstream maps the class through the target augmentation at (0,0)
            // with a single-column matrix; require that augmentation to be
            // computed and 1-dimensional in degree 0 (the unit/sphere case).
            let zero = RsBidegree::s_t(0, 0);
            if !t.has_computed_bidegree(zero) {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "target not computed at bidegree (0, 0); resolve it through (0, 0) first",
                ));
            }
            let aug_dim = t.target().module(0).dimension(0);
            if aug_dim != 1 {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "from_class requires the target's augmentation to be 1-dimensional in degree 0 \
                     (a unit/sphere resolution); got dimension {aug_dim}"
                )));
            }
            Ok(ResolutionHomomorphism(RsResHom::from_class(
                name, s, t, b, &class,
            )))
        }

        /// The homomorphism's name (used in tracing/logging).
        ///
        /// `set_name` is not bound: the upstream `name` field is private and has
        /// no `&self` setter (and this pyclass is `frozen`).
        pub fn name(&self) -> String {
            self.0.name().to_string()
        }

        /// The Steenrod algebra the (source) resolution is built over.
        pub fn algebra(&self) -> algebra_py::SteenrodAlgebra {
            algebra_py::SteenrodAlgebra::from_arc(self.0.algebra())
        }

        /// The prime as a plain `int`.
        pub fn prime(&self) -> u32 {
            self.0.source.prime().as_u32()
        }

        /// The shift bidegree of the homomorphism (`f` sends `source.module(s)`
        /// into `target.module(s - shift.s)` and raises internal degree by
        /// `shift.t`).
        pub fn shift(&self) -> sseq_py::Bidegree {
            sseq_py::Bidegree(self.0.shift)
        }

        /// The source resolution (shares the underlying `Arc`).
        pub fn source(&self) -> Resolution {
            Resolution(AnyResolution::Standard(Arc::clone(&self.0.source)))
        }

        /// The target resolution (shares the underlying `Arc`).
        pub fn target(&self) -> Resolution {
            Resolution(AnyResolution::Standard(Arc::clone(&self.0.target)))
        }

        /// The first homological degree `s` at which the chain map is not yet
        /// defined (the length of the internal `maps` table).
        pub fn next_homological_degree(&self) -> i32 {
            self.0.next_homological_degree()
        }

        /// The directory used to persist the chain map, or `None` if it is held
        /// purely in memory (the default — only set when the source resolution
        /// has a save directory and the homomorphism has a non-empty name).
        pub fn save_dir(&self) -> Option<String> {
            self.0.save_dir().read().map(|p| p.display().to_string())
        }

        /// The chain map on the `s`-th source module, as a bound
        /// `FreeModuleHomomorphismToFree` sharing its `Arc` (the standard
        /// resolution's modules are free over the `SteenrodAlgebra`, so its maps
        /// are `FreeModule -> FreeModule`).
        ///
        /// Raises `IndexError` for `s` outside the defined range
        /// `[shift.s, next_homological_degree)` (the internal `maps` `OnceBiVec`
        /// is indexed there and would otherwise panic). Extend the homomorphism
        /// first to define more maps.
        pub fn get_map(&self, s: i32) -> PyResult<algebra_py::FreeModuleHomomorphismToFree> {
            if s < self.0.shift.s() || s >= self.0.next_homological_degree() {
                return Err(pyo3::exceptions::PyIndexError::new_err(format!(
                    "no map defined at homological degree s = {s}; defined range is [{}, {})",
                    self.0.shift.s(),
                    self.0.next_homological_degree()
                )));
            }
            Ok(algebra_py::FreeModuleHomomorphismToFree::from_arc(
                self.0.get_map(s),
            ))
        }

        /// Extend the chain map so it is defined on every bidegree `(s, t)` with
        /// `s <= max.s` and `t <= max.t`, lifting by exactness. Both source and
        /// target must already be resolved over the touched range (see the
        /// guard); otherwise a clean `ValueError` is raised. Negative `max` is
        /// rejected.
        pub fn extend(&self, max: sseq_py::Bidegree) -> PyResult<()> {
            let b = max.0;
            if b.s() < 0 || b.t() < 0 {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "invalid target bidegree {b}: require s >= 0 and t >= 0"
                )));
            }
            self.check_extend_range(b.s(), |_s| b.t())?;
            self.0.extend(b);
            Ok(())
        }

        /// Extend the chain map through the stem `max` (defined on every `(s, t)`
        /// with `s <= max.s` and `t - s <= max.n`). Guards the touched range as
        /// `extend` does.
        pub fn extend_through_stem(&self, max: sseq_py::Bidegree) -> PyResult<()> {
            let b = max.0;
            if b.s() < 0 || b.t() < 0 {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "invalid target bidegree {b}: require s >= 0 and t >= 0"
                )));
            }
            let n = b.n();
            self.check_extend_range(b.s(), |s| n + s)?;
            self.0.extend_through_stem(b);
            Ok(())
        }

        /// Extend the chain map as far as the source and target are already
        /// resolved (the largest range for which lifting is possible). Does
        /// nothing useful if the source/target are not resolved past the shift.
        ///
        /// Guards the degenerate case where the computable range is empty (the
        /// source is not resolved past `shift.s`, or the target is unresolved):
        /// upstream would index `maps[-1]`/an empty module and panic, so we
        /// raise `ValueError` instead.
        pub fn extend_all(&self) -> PyResult<()> {
            let shift = self.0.shift;
            if self.0.source.next_homological_degree() <= shift.s()
                || self.0.target.next_homological_degree() <= 0
            {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "nothing to extend: resolve the source past the shift's homological degree \
                     and the target past s = 0 first",
                ));
            }
            self.0.extend_all();
            Ok(())
        }

        /// Apply the dual map `Hom(f, k)` to the target-resolution generator
        /// `g`, accumulating `coef` times the result into `result` (a bound
        /// `fp.FpVector`). This is how a `ResolutionHomomorphism` acts on `Ext`:
        /// `result` collects the coefficients on the source generators at
        /// bidegree `g.degree() + shift`.
        ///
        /// Every degree/index reaching an `OnceVec`/`num_gens`/`FpVector` access
        /// is pre-checked (`ValueError`/`IndexError`), so a bad `g`, an
        /// unextended map, an uncomputed bidegree, or a mismatched `result`
        /// length raises cleanly rather than panicking:
        ///  - `g` non-negative and `g.degree() + shift` not overflowing `i32`;
        ///  - the map defined at `(g.s + shift.s)` and extended through
        ///    `(g.t + shift.t)`;
        ///  - `result` over the same prime and of length equal to the number of
        ///    source generators at `g.degree() + shift`;
        ///  - `g` a valid generator of the target at `g.degree()`.
        pub fn act(
            &self,
            mut result: PyRefMut<'_, fp_py::PyFpVector>,
            coef: u32,
            g: sseq_py::BidegreeGenerator,
        ) -> PyResult<()> {
            let gen = g.0;
            if gen.s() < 0 || gen.t() < 0 {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "invalid generator {gen}: require s >= 0 and t >= 0"
                )));
            }
            let shift = self.0.shift;
            let src_s = gen.s().checked_add(shift.s()).ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err("source s = g.s + shift.s overflows i32")
            })?;
            let src_t = gen.t().checked_add(shift.t()).ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err("source t = g.t + shift.t overflows i32")
            })?;
            let source_b = RsBidegree::s_t(src_s, src_t);
            // The map at source_b.s() must exist and be extended through source_b.t().
            if src_s >= self.0.next_homological_degree() {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "the homomorphism is not defined at homological degree s = {src_s} (= g.s + \
                     shift.s); extend it first"
                )));
            }
            if !self.0.source.has_computed_bidegree(source_b) {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "source not computed at bidegree (s={src_s}, t={src_t}) = g.degree() + shift"
                )));
            }
            let map = self.0.get_map(src_s);
            if src_t >= map.next_degree() {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "the homomorphism is not extended through (s={src_s}, t={src_t}); extend it \
                     further first"
                )));
            }
            // result must match the source prime and the number of source
            // generators at source_b.
            let p = self.0.source.prime().as_u32();
            if result.as_rust().prime().as_u32() != p {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "result vector prime {} != homomorphism prime {p}",
                    result.as_rust().prime().as_u32()
                )));
            }
            let expected = map.source().number_of_gens_in_degree(src_t);
            if result.as_rust().len() != expected {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "result vector has length {} but the source has {expected} generator(s) at \
                     bidegree (s={src_s}, t={src_t})",
                    result.as_rust().len()
                )));
            }
            // g must be a valid generator of the target at g.degree().
            if gen.s() >= self.0.target.next_homological_degree() {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "target not resolved at homological degree s = {} (g.s)",
                    gen.s()
                )));
            }
            let tgt_gens = self.target_num_gens(gen.degree());
            if gen.idx() >= tgt_gens {
                return Err(pyo3::exceptions::PyIndexError::new_err(format!(
                    "generator index {} out of range at target bidegree (s={}, t={}) ({tgt_gens} \
                     generator(s), or the bidegree is uncomputed)",
                    gen.idx(),
                    gen.s(),
                    gen.t()
                )));
            }
            self.0.act(result.as_rust_mut().as_slice_mut(), coef, gen);
            Ok(())
        }
    }

    /// A secondary resolution is only supported over the standard backend. Nassau's algorithm
    /// stores its quasi-inverses on disk and returns them only when a save directory is present;
    /// without one, `apply_quasi_inverse` always reports failure and the secondary lift's internal
    /// `assert!` panics. Since the binding never gives Nassau a save directory, we reject the
    /// pairing up front rather than expose a guaranteed FFI panic.
    #[pyclass(frozen)]
    pub struct SecondaryResolution(
        ext::secondary::SecondaryResolution<ext::resolution::Resolution<ext::CCC>>,
    );

    #[pymethods]
    impl SecondaryResolution {
        #[new]
        pub fn new(cc: &Resolution) -> PyResult<Self> {
            match &cc.0 {
                AnyResolution::Standard(r) => Ok(SecondaryResolution(
                    ext::secondary::SecondaryResolution::new(Arc::clone(r)),
                )),
                AnyResolution::Nassau(_) => Err(pyo3::exceptions::PyValueError::new_err(
                    "SecondaryResolution requires the standard backend (Nassau resolutions store \
                     quasi-inverses on disk and need a save directory); construct the Resolution \
                     with algorithm='standard'",
                )),
            }
        }

        pub fn extend_all(&self) {
            self.0.extend_all()
        }

        pub fn underlying(&self) -> Resolution {
            Resolution(AnyResolution::Standard(Arc::clone(&self.0.underlying())))
        }
    }

    /// A finite chain complex of Steenrod modules: the crate's default
    /// `CCC = FiniteChainComplex<SteenrodModule>`, i.e. exactly the type
    /// `utils::construct` resolves over.
    ///
    /// Stored as a (possibly shared) `Arc<CCC>`. Most methods take `&self` and
    /// either read or compute into the modules' interior-mutable tables, mirror-
    /// ing the `Resolution` binding. Unlike `Resolution`, this pyclass is *not*
    /// `frozen`, because `pop` structurally mutates the complex and needs
    /// `&mut self`; `pop` additionally requires sole ownership of the `Arc`.
    ///
    /// Only the `ChainComplex` trait surface is bound here. The
    /// `FreeChainComplex` methods (`graded_dimension_string`, `to_sseq`,
    /// `filtration_one_product(s)`, `number_of_gens_in_bidegree`,
    /// `iter_nonzero_stem`, `boundary_string`) are **not** implemented for
    /// `CCC`: that trait requires `Module = FreeModule`, but a `CCC`'s modules
    /// are arbitrary `SteenrodModule`s (`Arc<dyn Module>`). Those methods live
    /// on `Resolution` instead (whose modules are free); `to_sseq` is bound
    /// there.
    #[pyclass]
    pub struct ChainComplex(Arc<CCC>);

    #[pymethods]
    impl ChainComplex {
        /// The "concentrated chain complex, degreewise zero differential" of a
        /// single module: the one-term complex `C_0 = module`, `C_s = 0`
        /// otherwise. This is the simplest way to obtain a `ChainComplex` from a
        /// `SteenrodModule` (then `compute_through_bidegree`, `module`, ...).
        #[staticmethod]
        pub fn ccdz(module: PyRef<'_, algebra_py::SteenrodModule>) -> Self {
            let m = module.as_rust().clone();
            ChainComplex(Arc::new(CCC::ccdz(Arc::new(m))))
        }

        /// Build a finite chain complex from an explicit list of `modules`
        /// (`C_0, C_1, ..., C_n`) and the `differentials` between consecutive
        /// ones (`differentials[i]: C_{i+1} -> C_i`). The augmentation
        /// `d_0: C_0 -> 0` and the boundary `0 -> C_n` are appended by the
        /// underlying constructor automatically, so the caller supplies only the
        /// `n` interior differentials.
        ///
        /// Upstream `FiniteChainComplex::new` stores `modules`/`differentials`
        /// verbatim with no structural checks, so the inputs are validated here
        /// before construction. Raises `ValueError` if:
        /// * `modules` is empty (the underlying constructor indexes `modules[0]`);
        /// * `differentials.len() != modules.len() - 1` (one interior
        ///   differential per consecutive pair `C_{i+1} -> C_i`);
        /// * the modules do not all share the same prime and algebra object;
        /// * a differential is not built over that same prime and algebra.
        ///
        /// The exact source/target *module* of each differential is **not**
        /// checked against the adjacent modules: there is no cheap structural
        /// equality on `dyn Module`, and `Arc::ptr_eq` would reject the common,
        /// legitimate case where the differential was built from separate
        /// (cloned) module handles. The prime + algebra checks reject the
        /// incoherent cases (mixed prime/algebra, wrong count) while never
        /// rejecting a consistently-built complex.
        #[staticmethod]
        pub fn new(
            py: Python<'_>,
            modules: Vec<Py<algebra_py::SteenrodModule>>,
            differentials: Vec<Py<algebra_py::FullModuleHomomorphism>>,
        ) -> PyResult<Self> {
            if modules.is_empty() {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "ChainComplex.new requires at least one module",
                ));
            }
            // For a complex C_0 <- C_1 <- ... <- C_n (modules.len() == n+1),
            // there are exactly n interior differentials (differentials[i] maps
            // C_{i+1} -> C_i). Cf. upstream `FiniteChainComplex::new`, which
            // prepends d_0 and appends the boundary map itself.
            let expected = modules.len() - 1;
            if differentials.len() != expected {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "ChainComplex.new expects exactly {expected} differential(s) for {} module(s) \
                     (differentials[i] is the map C_(i+1) -> C_i); got {}",
                    modules.len(),
                    differentials.len()
                )));
            }
            let modules: Vec<Arc<algebra::module::SteenrodModule>> = modules
                .iter()
                .map(|m| Arc::new(m.borrow(py).as_rust().clone()))
                .collect();
            // All modules must share the same prime AND the same algebra object
            // (the latter via `Arc::ptr_eq`, as TensorModule/homomorphism
            // constructors do). Otherwise `prime()`/`algebra()` would report
            // module 0's values while later modules disagree.
            let ref_algebra = modules[0].algebra();
            let p = modules[0].prime().as_u32();
            for (i, m) in modules.iter().enumerate() {
                let alg = m.algebra();
                if m.prime().as_u32() != p {
                    return Err(pyo3::exceptions::PyValueError::new_err(format!(
                        "all modules must share the same prime; module 0 is over p={p} but \
                         module {i} is over p={}",
                        m.prime().as_u32()
                    )));
                }
                if !Arc::ptr_eq(&alg, &ref_algebra) {
                    return Err(pyo3::exceptions::PyValueError::new_err(format!(
                        "all modules must be built over the same algebra object; module {i} is \
                         over a different algebra than module 0"
                    )));
                }
            }
            let differentials = differentials
                .iter()
                .enumerate()
                .map(|(i, d)| {
                    let d = d.borrow(py);
                    if d.prime() != p {
                        return Err(pyo3::exceptions::PyValueError::new_err(format!(
                            "differential {i} is over p={} but the complex is over p={p}",
                            d.prime()
                        )));
                    }
                    if !Arc::ptr_eq(&d.source_algebra(), &ref_algebra)
                        || !Arc::ptr_eq(&d.target_algebra(), &ref_algebra)
                    {
                        return Err(pyo3::exceptions::PyValueError::new_err(format!(
                            "differential {i} must be built over the same algebra object as the \
                             complex's modules"
                        )));
                    }
                    Ok(Arc::new(d.clone_rust()))
                })
                .collect::<PyResult<Vec<_>>>()?;
            Ok(ChainComplex(Arc::new(CCC::new(modules, differentials))))
        }

        /// Remove the top module (and its differentials) from the complex.
        ///
        /// Requires sole ownership of the underlying `Arc`; raises `RuntimeError`
        /// if the complex is shared (e.g. obtained from `Resolution.chain_complex`
        /// or aliased by another Python handle). A live `StemIterator` from
        /// `iter_stem` also holds a shared handle, so drop any such iterator
        /// before calling `pop`.
        pub fn pop(&mut self) -> PyResult<()> {
            let cc = Arc::get_mut(&mut self.0).ok_or_else(|| {
                pyo3::exceptions::PyRuntimeError::new_err(
                    "cannot pop a shared ChainComplex (the underlying complex is referenced \
                     elsewhere, e.g. by a Resolution)",
                )
            })?;
            cc.pop();
            Ok(())
        }

        /// The prime as a plain `int` (`ValidPrime` is never exposed).
        pub fn prime(&self) -> u32 {
            self.0.prime().as_u32()
        }

        /// The Steenrod algebra the complex is built over.
        pub fn algebra(&self) -> algebra_py::SteenrodAlgebra {
            algebra_py::SteenrodAlgebra::from_arc(self.0.algebra())
        }

        /// The minimum internal degree shared by every module.
        pub fn min_degree(&self) -> i32 {
            self.0.min_degree()
        }

        /// The first `s` for which `module(s)` is not defined. For a
        /// `FiniteChainComplex` this is `i32::MAX` (every `s` resolves to the
        /// zero module past the top), so `iter_stem` is *infinite*; see there.
        pub fn next_homological_degree(&self) -> i32 {
            self.0.next_homological_degree()
        }

        /// The zero module (the target/source of the boundary differentials).
        pub fn zero_module(&self) -> algebra_py::SteenrodModule {
            algebra_py::SteenrodModule::from_rust((*self.0.zero_module()).clone())
        }

        /// The `s`-th module `C_s`, sharing its `Arc`. Out-of-range `s` (`>=` the
        /// number of modules) returns the zero module, matching upstream.
        /// Raises `ValueError` for negative `s`.
        pub fn module(&self, s: i32) -> PyResult<algebra_py::SteenrodModule> {
            if s < 0 {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "homological degree s must be non-negative",
                ));
            }
            Ok(algebra_py::SteenrodModule::from_rust(
                (*self.0.module(s)).clone(),
            ))
        }

        /// The differential `C_s -> C_{s-1}`, as a bound `FullModuleHomomorphism`
        /// sharing its `Arc`. Out-of-range `s` returns a zero homomorphism.
        /// Raises `ValueError` for negative `s`.
        pub fn differential(&self, s: i32) -> PyResult<algebra_py::FullModuleHomomorphism> {
            if s < 0 {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "homological degree s must be non-negative",
                ));
            }
            Ok(algebra_py::FullModuleHomomorphism::from_rust(
                (*self.0.differential(s)).clone(),
            ))
        }

        /// Whether the complex has been computed at bidegree `b`. Like
        /// `module`/`differential`/`compute_through_bidegree`, a negative
        /// `s`/`t` is rejected with a `ValueError` rather than wrapping to a
        /// huge `usize`.
        pub fn has_computed_bidegree(&self, b: sseq_py::Bidegree) -> PyResult<bool> {
            if b.0.s() < 0 || b.0.t() < 0 {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "invalid bidegree {}: require s >= 0 and t >= 0",
                    b.0
                )));
            }
            Ok(self.0.has_computed_bidegree(b.0))
        }

        /// Ensure every bidegree `<= b` has been computed. Like
        /// `Resolution.compute_through_stem`, a negative `s`/`t` is rejected with
        /// a `ValueError` rather than risking an internal panic.
        pub fn compute_through_bidegree(&self, b: sseq_py::Bidegree) -> PyResult<()> {
            if b.0.s() < 0 || b.0.t() < 0 {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "invalid target bidegree {}: require s >= 0 and t >= 0",
                    b.0
                )));
            }
            self.0.compute_through_bidegree(b.0);
            Ok(())
        }

        /// Iterate over the defined bidegrees in increasing order of stem.
        ///
        /// WARNING: for a `FiniteChainComplex` whose modules report an unbounded
        /// `max_computed_degree` (as `FDModule` does), this iterator is
        /// *infinite* — `next_homological_degree` is `i32::MAX` and the
        /// per-stem cutoff never triggers. It is exposed faithfully as a lazy
        /// iterator (it will not hang unless fully materialised); slice it with
        /// `itertools.islice` rather than `list()`.
        ///
        /// The returned `StemIterator` holds a shared handle to the complex, so
        /// while one is alive `pop` will raise `RuntimeError`. Drop any
        /// `StemIterator` before calling `pop`.
        pub fn iter_stem(&self) -> StemIterator {
            StemIterator {
                cc: Arc::clone(&self.0),
                current: RsBidegree::n_s(self.0.min_degree(), 0),
                max_s: self.0.next_homological_degree(),
            }
        }

        /// The directory used to persist this complex, or `None` if it is purely
        /// in-memory (the default for `CCC`).
        pub fn save_dir(&self) -> Option<String> {
            self.0.save_dir().read().map(|p| p.display().to_string())
        }
    }

    /// The lazy iterator returned by [`ChainComplex::iter_stem`]. Re-implements
    /// the upstream `chain_complex::StemIterator` over an owned `Arc<CCC>` so it
    /// can live in a `#[pyclass]` without a borrow of the complex.
    #[pyclass]
    pub struct StemIterator {
        cc: Arc<CCC>,
        current: RsBidegree,
        max_s: i32,
    }

    #[pymethods]
    impl StemIterator {
        fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
            slf
        }

        fn __next__(&mut self) -> Option<sseq_py::Bidegree> {
            loop {
                if self.max_s == 0 {
                    return None;
                }
                let cur = self.current;
                if cur.s() == self.max_s {
                    self.current = RsBidegree::n_s(cur.n() + 1, 0);
                    continue;
                }
                if cur.t() > self.cc.module(cur.s()).max_computed_degree() {
                    if cur.s() == 0 {
                        return None;
                    } else {
                        self.current = RsBidegree::n_s(cur.n() + 1, 0);
                        continue;
                    }
                }
                self.current = cur + RsBidegree::n_s(0, 1);
                return Some(sseq_py::Bidegree(cur));
            }
        }
    }

    #[pymodule_init]
    fn init(_m: &Bound<'_, PyModule>) -> PyResult<()> {
        ext::utils::init_logging()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(())
    }
}
