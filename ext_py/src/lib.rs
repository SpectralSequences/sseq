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
        module::{FDModule, Module},
    };
    use ext::{
        chain_complex::{AugmentedChainComplex, ChainComplex as RsChainComplex, FreeChainComplex},
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

    #[pyclass(frozen)]
    pub struct Resolution(AnyResolution);

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
        /// (`C_0, C_1, ...`) and the `differentials` between consecutive ones
        /// (`differentials[i]: C_{i+1} -> C_i`). Zero homomorphisms are appended
        /// at both ends automatically. Raises `ValueError` if `modules` is empty
        /// (the underlying constructor indexes `modules[0]`).
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
            let modules: Vec<Arc<algebra::module::SteenrodModule>> = modules
                .iter()
                .map(|m| Arc::new(m.borrow(py).as_rust().clone()))
                .collect();
            let differentials = differentials
                .iter()
                .map(|d| Arc::new(d.borrow(py).clone_rust()))
                .collect();
            Ok(ChainComplex(Arc::new(CCC::new(modules, differentials))))
        }

        /// Remove the top module (and its differentials) from the complex.
        ///
        /// Requires sole ownership of the underlying `Arc`; raises `RuntimeError`
        /// if the complex is shared (e.g. obtained from `Resolution.chain_complex`
        /// or aliased by another Python handle).
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

        /// Whether the complex has been computed at bidegree `b`.
        pub fn has_computed_bidegree(&self, b: sseq_py::Bidegree) -> bool {
            self.0.has_computed_bidegree(b.0)
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
