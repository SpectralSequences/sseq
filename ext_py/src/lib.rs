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
    use std::{path::PathBuf, sync::Arc};

    use algebra::{
        milnor_algebra::MilnorAlgebra,
        module::{
            homomorphism::{
                FullModuleHomomorphism as RsFullModuleHomomorphism, ModuleHomomorphism,
            },
            FDModule, Module, SteenrodModule as RsSteenrodModule,
        },
        Algebra,
    };
    use ext::{
        chain_complex::{
            AugmentedChainComplex, ChainComplex as RsChainComplex,
            ChainHomotopy as RsChainHomotopy,
            FiniteAugmentedChainComplex as RsFiniteAugmentedChainComplex, FreeChainComplex,
        },
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
    ///
    /// `save_dir` is the optional on-disk save directory threaded down to the upstream
    /// `construct_nassau`/`construct_standard` (which accept `impl Into<SaveDirectory>`, and
    /// `Option<PathBuf>: Into<SaveDirectory>`). When `Some`, the resolution is backed by that
    /// directory: any already-computed bidegrees are loaded from it and newly-computed ones are
    /// written back. We never prompt for it here (that is the Python I/O layer's job) and do not
    /// pre-create/validate the path beyond what upstream does (upstream handles dir creation).
    fn build(
        spec: Config,
        save_dir: Option<PathBuf>,
        algorithm: Option<&str>,
    ) -> PyResult<AnyResolution> {
        use ext::utils::{construct_nassau, construct_standard};

        // Classify a save-directory problem up front, on EVERY algorithm path. Without this,
        // a save-dir IO failure (e.g. the path is an existing *file*) would be reported as a
        // `ValueError` only on the forced-`"nassau"` path (whose blanket `value_err` lumps it in
        // with module-eligibility errors), giving a confusing eligibility-flavoured message. A
        // path that exists but is not a directory is a bad *argument*, so we report a clear
        // `ValueError` here consistently for nassau/standard/auto. A non-existent path is fine:
        // upstream `create_dir_all` creates it.
        if let Some(p) = &save_dir {
            if p.exists() && !p.is_dir() {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "save_dir {p:?} exists and is not a directory"
                )));
            }
        }

        let nassau = |spec, save_dir: Option<PathBuf>| {
            construct_nassau(spec, save_dir).map(|r| AnyResolution::Nassau(Arc::new(r)))
        };
        let standard = |spec, save_dir: Option<PathBuf>| {
            construct_standard::<false, _, _>(spec, save_dir)
                .map(|r| AnyResolution::Standard(Arc::new(r)))
        };
        let value_err = |e: anyhow::Error| pyo3::exceptions::PyValueError::new_err(e.to_string());
        let runtime_err =
            |e: anyhow::Error| pyo3::exceptions::PyRuntimeError::new_err(e.to_string());

        match algorithm {
            // Eligibility/bad-argument: report as ValueError.
            Some("nassau") => nassau(spec, save_dir).map_err(value_err),
            Some("standard") => standard(spec, save_dir).map_err(runtime_err),
            None | Some("auto") => match nassau(spec.clone(), save_dir.clone()) {
                Ok(res) => Ok(res),
                // `auto` intentionally falls back to the general algorithm on ANY Nassau error,
                // not just eligibility errors. Nassau rejects ineligible modules up front, so in
                // practice the discarded error is an eligibility check; a genuinely malformed
                // module is surfaced by the general algorithm's own error below.
                Err(_) => standard(spec, save_dir).map_err(runtime_err),
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

    /// Parse a module *name*/specification string into its JSON description, as
    /// a native Python `dict`.
    ///
    /// Mirrors `ext::utils::parse_module_name(name: &str) -> anyhow::Result<Value>`:
    /// it loads the bundled module JSON for the base name and applies any
    /// `[shift]` suffix. The resulting `serde_json::Value` is converted to a
    /// Python object via the shared [`algebra_py::json_to_py`] bridge (the same
    /// helper pair used by `SteenrodAlgebra.from_json`'s `py_to_json`), so the
    /// caller gets a plain `dict` rather than an opaque handle.
    ///
    /// Every failure upstream returns `anyhow::Err` (unknown module name,
    /// unterminated/`non-integer` shift, missing bundled file); the path is a
    /// bad *argument*, so all are mapped to `ValueError`. The upstream code is
    /// pure parsing plus a `std::fs::read_to_string`; it does not panic, so no
    /// `catch_unwind` is required.
    #[pyfunction]
    pub fn parse_module_name(py: Python<'_>, name: &str) -> PyResult<Py<PyAny>> {
        let value = ext::utils::parse_module_name(name)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        algebra_py::json_to_py(py, &value)
    }

    /// Load a bundled module JSON file by name (without the `.json` extension)
    /// as a native Python `dict`.
    ///
    /// Mirrors `ext::utils::load_module_json(name: &str) -> anyhow::Result<Value>`:
    /// it searches the current directory, `./steenrod_modules`, and the
    /// compiled-in `STATIC_MODULES_PATH` (`ext/steenrod_modules`) for
    /// `<name>.json`. The found `serde_json::Value` is converted via the shared
    /// [`algebra_py::json_to_py`] bridge.
    ///
    /// An unknown/missing module name is a bad *argument* -> `ValueError`
    /// (matching `parse_module_name`); a present-but-malformed JSON file is a
    /// genuine read/parse failure -> `RuntimeError`. Upstream returns a typed
    /// [`LoadModuleError`](ext::utils::LoadModuleError) whose `NotFound`/`Read`
    /// variants make this distinction without matching on the error string.
    #[pyfunction]
    pub fn load_module_json(py: Python<'_>, name: &str) -> PyResult<Py<PyAny>> {
        let value = ext::utils::load_module_json(name).map_err(|e| {
            let msg = e.to_string();
            match e {
                ext::utils::LoadModuleError::NotFound(_) => {
                    pyo3::exceptions::PyValueError::new_err(msg)
                }
                ext::utils::LoadModuleError::Read(_) => {
                    pyo3::exceptions::PyRuntimeError::new_err(msg)
                }
            }
        })?;
        algebra_py::json_to_py(py, &value)
    }

    /// The lambda-algebra bidegree constant `ext::secondary::LAMBDA_BIDEGREE`
    /// (`Bidegree::n_s(0, 1)`), exposed as a bound [`sseq_py::Bidegree`].
    ///
    /// Sourced directly from the Rust constant so it cannot drift from upstream;
    /// the Python package binds `ext.LAMBDA_BIDEGREE = lambda_bidegree()` as a
    /// module-level value (the examples use it as a value, e.g.
    /// `shift + ext.LAMBDA_BIDEGREE`).
    #[pyfunction]
    pub fn lambda_bidegree() -> sseq_py::Bidegree {
        ext::secondary::LAMBDA_BIDEGREE.into()
    }

    /// Given a resolution, return `(is_unit, unit_resolution)`: a flag for
    /// whether the input already resolves the unit, and a resolution of the unit
    /// (the input itself when `is_unit` is true).
    ///
    /// Mirrors
    /// `ext::utils::get_unit(Arc<QueryModuleResolution>) -> anyhow::Result<(bool, Arc<QueryModuleResolution>)>`.
    /// `ext` builds `ext` without the `nassau` feature, so
    /// `QueryModuleResolution = Resolution<CCC>`, which is exactly the inner type
    /// of [`AnyResolution::Standard`]; a Nassau-backed input therefore cannot be
    /// passed and is rejected with `ValueError` (mirroring `chain_complex()` /
    /// `ResolutionHomomorphism`'s standard-only precedent).
    ///
    /// The `is_unit` check reads `target().max_s()` and `module(0).is_unit()`
    /// (cheap, non-panicking reads). NOTE: when the input is NOT the unit,
    /// upstream `get_unit` interactively prompts (`query::optional`) for a unit
    /// save directory and then constructs a fresh unit resolution; that prompt is
    /// upstream behavior, so callers in a non-interactive context should only
    /// pass a unit resolution (e.g. `S_2`, the typical `massey.py` input).
    /// Construction failures (IO on the save directory) map to `RuntimeError`.
    #[pyfunction]
    pub fn get_unit(resolution: &Resolution) -> PyResult<(bool, Resolution)> {
        let arc =
            match &resolution.0 {
                AnyResolution::Standard(r) => Arc::clone(r),
                AnyResolution::Nassau(_) => return Err(pyo3::exceptions::PyValueError::new_err(
                    "get_unit() is only available on the standard backend; the Nassau algorithm \
                     resolves over the concrete MilnorAlgebra and has no get_unit analogue here",
                )),
            };
        let (is_unit, unit) = ext::utils::get_unit(arc)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok((is_unit, Resolution(AnyResolution::Standard(unit))))
    }

    /// Construct a [`Resolution`] of the module `spec`, optionally backed by an on-disk save
    /// directory, without any interactive prompting (all I/O lives in the Python layer).
    ///
    /// This is the non-interactive primitive the pure-Python `query_module*` helpers call after
    /// they have prompted the user for the spec and (optionally) the save directory.
    ///
    /// # Arguments
    ///  - `spec`: the module specification, parsed into a [`Config`] exactly as
    ///    [`Resolution::new`] does. The Steenrod-algebra basis (Adem vs Milnor) is selected by an
    ///    `@adem`/`@milnor` suffix on the spec (e.g. `"S_2@milnor"`); there is no separate algebra
    ///    enum argument here.
    ///  - `save_dir`: optional filesystem path. When given, the resolution loads any previously
    ///    saved bidegrees from it and writes newly-computed ones back. Not pre-created/validated
    ///    here; upstream `construct` handles directory creation.
    ///  - `algorithm`: `None`/`"auto"` (try Nassau, fall back to the general algorithm),
    ///    `"nassau"` (force Nassau), or `"standard"` (force the general algorithm). This selects
    ///    the resolution *algorithm*, NOT the algebra basis (which is the `@`-suffix above).
    ///
    /// Error taxonomy matches [`build`]: bad spec/eligibility/unknown-algorithm -> `ValueError`,
    /// genuine internal/IO failures -> `RuntimeError`. Nothing panics across FFI.
    #[pyfunction]
    #[pyo3(signature = (spec, save_dir=None, algorithm=None))]
    pub fn construct(
        spec: &str,
        save_dir: Option<String>,
        algorithm: Option<&str>,
    ) -> PyResult<Resolution> {
        let config: Config = spec
            .try_into()
            .map_err(|e: anyhow::Error| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        build(config, save_dir.map(PathBuf::from), algorithm).map(Resolution)
    }

    /// The concrete unstable resolution type bound here: an `U = true`
    /// [`MuResolution`] over the default complex `CCC`
    /// (`UnstableResolution<CCC> = MuResolution<true, CCC>`). The unstable family
    /// is *general-algorithm only*: Nassau's algorithm has no unstable variant
    /// (it special-cases the stable, `U = false`, mod-2 sphere), so there is no
    /// `AnyResolution`-style backend union here — a single concrete type.
    type RsUnstableResolution = ext::resolution::UnstableResolution<CCC>;

    /// Construct an unstable resolution of `spec` via the general algorithm with
    /// `U = true`, threading `save_dir` exactly as the stable [`build`] does
    /// (including the "save_dir is an existing file" pre-check). Unstable
    /// construction monomorphises `construct_standard::<true, _, _>` — the same
    /// entry point the stable standard path uses with `U = false` — building the
    /// Steenrod algebra with `unstable = true` and resolving over it.
    ///
    /// Error taxonomy matches the stable standard path: a malformed spec is a
    /// `ValueError` (raised at the `Config` conversion in [`construct_unstable`]),
    /// and an internal/IO construction failure is a `RuntimeError`.
    ///
    /// # Cofiber specs
    ///
    /// A module spec whose JSON carries a non-null `cofiber` field (e.g. `C9`,
    /// `C4`, `Ceta2`, `C2v14`, `C3v1b1`) is supported by the *stable* standard
    /// path but NOT by the unstable one. Upstream `construct_standard` now
    /// returns `Err("Cofiber not supported for unstable resolution")` for a
    /// `cofiber`-bearing spec with `U = true` (rather than asserting/panicking),
    /// so we surface that as a `ValueError` via `map_err` and leave genuine
    /// internal/IO failures as `RuntimeError`.
    fn build_unstable(
        spec: Config,
        save_dir: Option<PathBuf>,
    ) -> PyResult<Arc<RsUnstableResolution>> {
        if let Some(p) = &save_dir {
            if p.exists() && !p.is_dir() {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "save_dir {p:?} exists and is not a directory"
                )));
            }
        }
        ext::utils::construct_standard::<true, _, _>(spec, save_dir)
            .map(Arc::new)
            .map_err(|e: anyhow::Error| {
                let msg = e.to_string();
                if msg.contains("Cofiber") || msg.contains("cofiber") {
                    pyo3::exceptions::PyValueError::new_err(format!(
                        "unstable resolution does not support cofiber modules (the spec's \
                         module JSON has a non-null `cofiber` field, which is stable-only): {msg}"
                    ))
                } else {
                    pyo3::exceptions::PyRuntimeError::new_err(msg)
                }
            })
    }

    /// Construct an [`UnstableResolution`] of the module `spec`, optionally backed
    /// by an on-disk save directory, without any interactive prompting (the
    /// unstable analogue of [`construct`]).
    ///
    /// Unstable resolutions are computed by the general algorithm only (there is
    /// no Nassau analogue), so unlike [`construct`] there is no `algorithm`
    /// argument: the `U = true` instantiation of `construct_standard` is always
    /// used. The Steenrod-algebra basis (Adem vs Milnor) is still selected by an
    /// `@adem`/`@milnor` suffix on the spec; the default is Milnor.
    ///
    /// `save_dir` behaves exactly as in [`construct`]: when given, previously
    /// saved bidegrees are loaded and new ones written back; an existing path
    /// that is not a directory is a `ValueError`; a non-existent path is created
    /// by upstream. Error taxonomy: bad spec -> `ValueError`, internal/IO ->
    /// `RuntimeError`. Nothing panics across FFI.
    #[pyfunction]
    #[pyo3(signature = (spec, save_dir=None))]
    pub fn construct_unstable(
        spec: &str,
        save_dir: Option<String>,
    ) -> PyResult<UnstableResolution> {
        let config: Config = spec
            .try_into()
            .map_err(|e: anyhow::Error| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        build_unstable(config, save_dir.map(PathBuf::from)).map(UnstableResolution)
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
        ///
        /// `save_dir` is an optional on-disk save directory (added as a third optional argument so
        /// `Resolution(spec)` and `Resolution(spec, algorithm)` keep working unchanged). When
        /// given, the resolution loads any previously-saved bidegrees and writes new ones back;
        /// see [`construct`] for the full description. No prompting happens here.
        #[new]
        #[pyo3(signature = (spec, algorithm=None, save_dir=None))]
        pub fn new(
            spec: &str,
            algorithm: Option<&str>,
            save_dir: Option<String>,
        ) -> PyResult<Self> {
            let config: Config = spec.try_into().map_err(|e: anyhow::Error| {
                pyo3::exceptions::PyValueError::new_err(e.to_string())
            })?;
            build(config, save_dir.map(PathBuf::from), algorithm).map(Resolution)
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

    /// An unstable minimal resolution (`U = true`), the unstable analogue of
    /// the stable [`Resolution`].
    ///
    /// This is a **separate** pyclass rather than a new variant of
    /// `AnyResolution`/the stable `Resolution`. The two were deliberately kept
    /// apart:
    ///  - the unstable resolution is a distinct monomorphisation
    ///    (`MuResolution<true, CCC>`), so it cannot share `AnyResolution`'s
    ///    `match` arms (its modules are `MuFreeModule<true, _>`, a different type
    ///    from the stable `MuFreeModule<false, _>`);
    ///  - there is no Nassau unstable algorithm, so the backend-dispatch
    ///    machinery (`dispatch!`, the standard-only callback methods) has no
    ///    unstable counterpart;
    ///  - mixing the two through one pyclass would reintroduce exactly the
    ///    stable/unstable footguns the upstream `if U` branches guard against.
    ///
    /// Holds the resolution behind an `Arc` (mirroring `AnyResolution`'s
    /// variants) so a [`UnstableResolutionStemIterator`] can own a cheap second
    /// handle. `frozen`: every method takes `&self` and the resolution's tables
    /// are interior-mutable.
    ///
    /// Deferred members (with concrete reasons):
    ///  - `module(s)`: the unstable resolution's modules are
    ///    `MuFreeModule<true, SteenrodAlgebra>`, a *different* type from the
    ///    bound `algebra_py.FreeModule` pyclass's inner
    ///    `FreeModule<SteenrodAlgebra> = MuFreeModule<false, _>`. The pyclass
    ///    cannot represent it, so `module()` is omitted (mirroring how Nassau's
    ///    `module()` was rejected).
    ///  - `new_with_save(chain_complex)`: upstream's by-complex constructor is
    ///    NOT bound because it cannot be made panic-safe at the FFI boundary. An
    ///    unstable resolution requires its algebra to have been built with
    ///    `unstable = true` (so the `dimension_unstable` basis tables exist);
    ///    resolving over a `ChainComplex` whose algebra was built stably panics
    ///    deep in upstream `once.rs` on the first `compute_through_*`, and there
    ///    is no public accessor to detect the algebra's unstable flag up front.
    ///    The spec-based [`construct_unstable`] / `UnstableResolution(spec)` path
    ///    always builds the algebra with `unstable = true`, so it is safe.
    ///  - the `*_with_callback` / `chain_complex` / `filtration_one_product(s)`
    ///    methods are not bound here (the callback hooks and standard-complex
    ///    accessors carry no unstable-specific value for the primary deliverable;
    ///    filtration-one is unstable-conditional and can return `None`).
    #[pyclass(frozen)]
    pub struct UnstableResolution(Arc<RsUnstableResolution>);

    impl UnstableResolution {
        /// Number of generators of the unstable resolution at bidegree `b`,
        /// returning 0 (never panicking) for any bidegree outside the computed
        /// range. Mirrors `Resolution::num_gens_at`: both indexing steps
        /// (`module(s)` and `number_of_gens_in_degree(t)`) panic out of range,
        /// so both axes are clamped to the populated range.
        fn num_gens_at(&self, b: RsBidegree) -> usize {
            if b.s() < 0 || b.t() < 0 || b.s() >= self.0.next_homological_degree() {
                return 0;
            }
            let m = self.0.module(b.s());
            if b.t() < m.min_degree() || b.t() > m.max_computed_degree() {
                0
            } else {
                m.number_of_gens_in_degree(b.t())
            }
        }
    }

    #[pymethods]
    impl UnstableResolution {
        /// Construct an unstable resolution of the module specification `spec`
        /// (the unstable analogue of `Resolution(spec)`), optionally backed by an
        /// on-disk save directory. Equivalent to the [`construct_unstable`]
        /// pyfunction; see it for the full description. There is no `algorithm`
        /// argument because the unstable family is general-algorithm only.
        #[new]
        #[pyo3(signature = (spec, save_dir=None))]
        pub fn new(spec: &str, save_dir: Option<String>) -> PyResult<Self> {
            let config: Config = spec.try_into().map_err(|e: anyhow::Error| {
                pyo3::exceptions::PyValueError::new_err(e.to_string())
            })?;
            build_unstable(config, save_dir.map(PathBuf::from)).map(UnstableResolution)
        }

        /// Resolve through the given target stem. Validates `s >= 0`/`t >= 0`
        /// (a negative target trips the same internal `assert!`/over-allocation
        /// as the stable path), raising `ValueError` rather than panicking.
        pub fn compute_through_stem(&self, max: sseq_py::Bidegree) -> PyResult<()> {
            let b = max.0;
            if b.s() < 0 || b.t() < 0 {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "invalid target bidegree {b}: require s >= 0 and t >= 0"
                )));
            }
            self.0.compute_through_stem(b);
            Ok(())
        }

        /// Resolve through the given target bidegree (fixed `t`). Validates
        /// `s >= 0`/`t >= 0`, raising `ValueError` rather than panicking.
        pub fn compute_through_bidegree(&self, max: sseq_py::Bidegree) -> PyResult<()> {
            let b = max.0;
            if b.s() < 0 || b.t() < 0 {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "invalid target bidegree {b}: require s >= 0 and t >= 0"
                )));
            }
            self.0.compute_through_bidegree(b);
            Ok(())
        }

        /// The prime as a plain `int`.
        pub fn prime(&self) -> u32 {
            self.0.prime().as_u32()
        }

        /// The minimum internal degree of the resolution's modules.
        pub fn min_degree(&self) -> i32 {
            self.0.min_degree()
        }

        /// The first `s` for which `module(s)` is not yet defined.
        pub fn next_homological_degree(&self) -> i32 {
            self.0.next_homological_degree()
        }

        /// Whether the resolution has been computed at bidegree `b`. Negative
        /// `s`/`t` is rejected with a `ValueError`.
        pub fn has_computed_bidegree(&self, b: sseq_py::Bidegree) -> PyResult<bool> {
            if b.0.s() < 0 || b.0.t() < 0 {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "invalid bidegree {}: require s >= 0 and t >= 0",
                    b.0
                )));
            }
            Ok(self.0.has_computed_bidegree(b.0))
        }

        /// The number of generators of the unstable resolution at bidegree `b`
        /// (the dimension of unstable `Ext` there). Returns 0 for any uncomputed
        /// or out-of-range bidegree; raises `ValueError` for negative `s`/`t`.
        /// Guarded like the stable `number_of_gens_in_bidegree`; see
        /// `num_gens_at`.
        pub fn number_of_gens_in_bidegree(&self, b: sseq_py::Bidegree) -> PyResult<usize> {
            if b.0.s() < 0 || b.0.t() < 0 {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "invalid bidegree {}: require s >= 0 and t >= 0",
                    b.0
                )));
            }
            Ok(self.num_gens_at(b.0))
        }

        pub fn graded_dimension_string(&self) -> String {
            self.0.graded_dimension_string()
        }

        /// The unstable `E_2`-page as a bound `sseq_py.Sseq` (the unstable
        /// analogue of `Resolution.to_sseq`, i.e. `to_sseq` on the unstable free
        /// chain complex). Panic-free over the resolved range: upstream only
        /// queries bidegrees yielded by `iter_stem`, all in range.
        pub fn to_unstable_sseq(&self) -> sseq_py::Sseq {
            let p = self.0.prime();
            sseq_py::Sseq::from_rust(self.0.to_sseq(), p)
        }

        /// A string representation of `d(g)` for the generator `g = (s, t, idx)`.
        /// Raises `ValueError` if `g` is outside the computed range or `idx`
        /// exceeds the generator count there (upstream would otherwise panic).
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
            Ok(self.0.boundary_string(gen))
        }

        /// The resolution's name (used in tracing/logging). `set_name` is not
        /// bound for the same reason as on `Resolution` (it takes `&mut self`,
        /// but this pyclass is `frozen` and wraps the resolution in an `Arc`).
        pub fn name(&self) -> String {
            self.0.name().to_string()
        }

        /// The directory used to persist the resolution, or `None` if it is held
        /// purely in memory (the default).
        pub fn save_dir(&self) -> Option<String> {
            self.0.save_dir().read().map(|p| p.display().to_string())
        }

        /// Iterate over the defined bidegrees in increasing order of stem. The
        /// iterator yields `sseq_py.Bidegree`s and holds its own `Arc` handle to
        /// the resolution. Bounded by the resolved range (terminates), exposed
        /// lazily.
        pub fn iter_stem(&self) -> UnstableResolutionStemIterator {
            UnstableResolutionStemIterator::new(Arc::clone(&self.0), false)
        }

        /// As [`iter_stem`], but yield only bidegrees with a nonzero number of
        /// generators (the nonzero entries of the unstable `Ext` chart).
        pub fn iter_nonzero_stem(&self) -> UnstableResolutionStemIterator {
            UnstableResolutionStemIterator::new(Arc::clone(&self.0), true)
        }
    }

    /// The lazy iterator returned by [`UnstableResolution::iter_stem`] /
    /// [`UnstableResolution::iter_nonzero_stem`]. Mirrors
    /// `ResolutionStemIterator` over the single concrete unstable resolution
    /// type (no backend dispatch), re-implementing the upstream stem walk so it
    /// can live in a `#[pyclass]` without borrowing the resolution.
    #[pyclass]
    pub struct UnstableResolutionStemIterator {
        res: Arc<RsUnstableResolution>,
        current: RsBidegree,
        max_s: i32,
        nonzero: bool,
    }

    impl UnstableResolutionStemIterator {
        fn new(res: Arc<RsUnstableResolution>, nonzero: bool) -> Self {
            let min_degree = res.min_degree();
            let max_s = res.next_homological_degree();
            UnstableResolutionStemIterator {
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
                let max_deg = self.res.module(cur.s()).max_computed_degree();
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
    impl UnstableResolutionStemIterator {
        fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
            slf
        }

        fn __next__(&mut self) -> Option<sseq_py::Bidegree> {
            loop {
                let b = self.raw_next()?;
                if !self.nonzero {
                    return Some(sseq_py::Bidegree(b));
                }
                if self.res.number_of_gens_in_bidegree(b) > 0 {
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
    ///
    /// Note: `get_map(s)` returns a `FreeModuleHomomorphismToFree` that shares
    /// the internal `Arc` of this homomorphism's `s`-th map. It is a *live
    /// view* and should be treated as read-only; calling its mutating methods
    /// (`add_generators_from_rows`, `set_quasi_inverse`, `extend_by_zero`, …)
    /// is memory-safe but can logically corrupt the chain map.
    ///
    /// Held behind an `Arc` (not by value) so it can be shared into a
    /// [`ChainHomotopy`] via `Arc::clone`: `ChainHomotopy::new` takes
    /// `Arc<ResolutionHomomorphism<…>>`, and sharing the same `Arc` (rather
    /// than a clone) means any further `extend*` of this homomorphism is visible
    /// to the homotopy built from it (and vice versa), matching the upstream
    /// `examples/massey.rs` usage. Every method takes `&self` and the inner
    /// state is interior-mutable (`OnceBiVec`), so the `Arc` adds no friction.
    #[pyclass(frozen)]
    pub struct ResolutionHomomorphism(Arc<RsResHom>);

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
        ///
        /// Note: if the `source` resolution is backed by a save directory *and*
        /// `name` is non-empty, upstream `new` creates a `products/{name}`
        /// subdirectory on disk (filesystem I/O, which can error). The default
        /// in-memory resolutions built here have no save directory, so this
        /// path is not exercised by the bound API.
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
            Ok(ResolutionHomomorphism(Arc::new(RsResHom::new(
                name, s, t, shift.0,
            ))))
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
        ///
        /// Note: as with `new`, constructing a *named* homomorphism against a
        /// `source` resolution backed by a save directory performs filesystem
        /// I/O (it creates a `products/{name}` directory and can error). The
        /// default in-memory resolutions built here have no save directory, so
        /// this path is not exercised by the bound API.
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
            Ok(ResolutionHomomorphism(Arc::new(RsResHom::from_class(
                name, s, t, b, &class,
            ))))
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
        ///
        /// WARNING: the returned homomorphism is a *live shared view* of this
        /// resolution homomorphism's internal map (the same `Arc`), not a copy.
        /// Treat it as read-only: calling its mutating methods
        /// (`add_generators_from_rows`, `set_quasi_inverse`, `extend_by_zero`, …)
        /// is memory-safe but may logically corrupt the chain map.
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
        ///
        /// Note: the touched range is the *full strip* — the source (and the
        /// shifted target) must be resolved over every `t` in
        /// `[min_degree, max.t]` for each `s` in `[shift.s, max.s]`, not merely
        /// at the corner `max`. A "thin"/partial computation that does not cover
        /// the whole strip is rejected rather than risking an upstream panic.
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

    /// The concrete `ChainHomotopy` monomorphisation bound here. Following the
    /// `ResolutionHomomorphism` precedent, only the standard-backend instantiation
    /// is reachable: all three chain-complex type parameters are
    /// `ext::resolution::Resolution<CCC>` (the type held by
    /// `AnyResolution::Standard`), because the inputs are two
    /// `ResolutionHomomorphism`s and those are standard→standard only. The
    /// homotopy maps are then `FreeModuleHomomorphism<FreeModule<SteenrodAlgebra>>`,
    /// exactly the inner type of the bound `FreeModuleHomomorphismToFree` pyclass.
    type RsCH = RsChainHomotopy<
        ext::resolution::Resolution<CCC>,
        ext::resolution::Resolution<CCC>,
        ext::resolution::Resolution<CCC>,
    >;

    /// A chain homotopy between two chain maps `left: S -> T` and `right: T -> U`
    /// (equivalently, a null-homotopy of their difference), the primitive used to
    /// assemble (triple) Massey products — see `examples/massey.rs`. Built from
    /// two `ResolutionHomomorphism`s `left` and `right` for which
    /// `left.target()` is the *same* resolution object as `right.source()`.
    ///
    /// Every method takes `&self` and the homotopy table is interior-mutable
    /// (`OnceBiVec`). The `num_chain` count is not needed (the homotopy table's
    /// populated range is queried upstream via `defined_range`).
    ///
    /// Only the standard backend is supported (see `RsCH`); the input
    /// `ResolutionHomomorphism`s already enforce this, so no extra backend check
    /// is needed here.
    ///
    /// Held behind an `Arc` (not by value) so the *same* instance can be shared
    /// into a [`SecondaryChainHomotopy`] via `Arc::clone`:
    /// `SecondaryChainHomotopy::new` takes `Arc<ChainHomotopy<…>>`, and sharing
    /// the same `Arc` (rather than a clone) means any further `extend*` of this
    /// homotopy is visible to the secondary lift built from it (mirroring the
    /// upstream `examples/secondary_massey.rs` usage). Every method takes `&self`
    /// and the homotopy table is interior-mutable (`OnceBiVec`), so the `Arc`
    /// adds no friction; `SecondaryChainHomotopy.underlying()` hands this `Arc`
    /// back.
    #[pyclass(frozen)]
    pub struct ChainHomotopy(Arc<RsCH>);

    impl ChainHomotopy {
        /// Pre-flight guard for the `extend*` family, mirroring
        /// `ResolutionHomomorphism::check_extend_range`. `extend`/`extend_all`
        /// drive `iter_s_t` over a profile grid; for every touched source
        /// bidegree `(s, t)` the upstream `extend_step` indexes (and would panic
        /// out of range):
        ///  - `left.source.module(s).number_of_gens_in_degree(t)` and
        ///    `right.target.module(s + 1 - shift.s).dimension(t - shift.t)` (plus
        ///    that target differential's quasi-inverse), so both resolutions must
        ///    be computed over the grid; and
        ///  - for the non-trivial lifts (`s >= shift.s`), `left.get_map(s)` and
        ///    `right.get_map(s - left.shift.s)`, extended through `t` and
        ///    `t - left.shift.t` respectively, so both chain maps must have been
        ///    extended over the grid (call `ResolutionHomomorphism.extend*`
        ///    first).
        ///
        /// We verify the whole profile grid up front (a conservative
        /// over-approximation — `extend_step` may skip some bidegrees via its
        /// zero-homotopy early return — but never an under-approximation), raising
        /// `ValueError` rather than letting an upstream index/`assert!` panic
        /// across FFI. `max_s` is the inclusive top homological degree;
        /// `t_hi(s)` the inclusive top internal degree of row `s`.
        fn check_extend_range(&self, max_s: i32, t_hi: impl Fn(i32) -> i32) -> PyResult<()> {
            let left = self.0.left();
            let right = self.0.right();
            let shift = self.0.shift();
            let left_shift = left.shift;
            let base_s = shift.s() - 1;
            if max_s < base_s {
                // Nothing is touched (upstream `extend_profile` returns early).
                return Ok(());
            }
            let min_t = std::cmp::min(
                left.source.min_degree(),
                right.target.min_degree() + shift.t(),
            );
            for s in base_s..=max_s {
                let hi = t_hi(s);
                if hi < min_t {
                    continue;
                }
                // Resolution coverage (needed for every touched source bidegree,
                // including the s = shift.s - 1 bottom row). has_computed_bidegree
                // is monotone in t, so the row corner suffices.
                let src_b = RsBidegree::s_t(s, hi);
                if !left.source.has_computed_bidegree(src_b) {
                    return Err(pyo3::exceptions::PyValueError::new_err(format!(
                        "the left source resolution is not computed at bidegree (s={s}, t={hi}), \
                         which is required to extend the homotopy over this range; resolve it \
                         further"
                    )));
                }
                let tgt_b = src_b + RsBidegree::s_t(1, 0) - shift;
                if !right.target.has_computed_bidegree(tgt_b) {
                    return Err(pyo3::exceptions::PyValueError::new_err(format!(
                        "the right target resolution is not computed at bidegree (s={}, t={}), \
                         which is required to extend the homotopy over this range; resolve it \
                         further",
                        tgt_b.s(),
                        tgt_b.t()
                    )));
                }
                // Chain-map coverage for the non-trivial lifts (s >= shift.s).
                if s >= shift.s() {
                    if s >= left.next_homological_degree() {
                        return Err(pyo3::exceptions::PyValueError::new_err(format!(
                            "the left chain map is not defined at homological degree s = {s}; \
                             extend it (ResolutionHomomorphism.extend*) first"
                        )));
                    }
                    if left.get_map(s).next_degree() <= hi {
                        return Err(pyo3::exceptions::PyValueError::new_err(format!(
                            "the left chain map is not extended through (s={s}, t={hi}); extend it \
                             further first"
                        )));
                    }
                    let rs = s - left_shift.s();
                    let rt = hi - left_shift.t();
                    if rs >= right.next_homological_degree() {
                        return Err(pyo3::exceptions::PyValueError::new_err(format!(
                            "the right chain map is not defined at homological degree s = {rs} \
                             (= {s} - left.shift.s); extend it first"
                        )));
                    }
                    if right.get_map(rs).next_degree() <= rt {
                        return Err(pyo3::exceptions::PyValueError::new_err(format!(
                            "the right chain map is not extended through (s={rs}, t={rt}); extend \
                             it further first"
                        )));
                    }
                }
            }
            Ok(())
        }
    }

    #[pymethods]
    impl ChainHomotopy {
        /// Construct the chain homotopy from two `ResolutionHomomorphism`s
        /// `left: S -> T` and `right: T -> U`. Upstream `ChainHomotopy::new`
        /// asserts `Arc::ptr_eq(&left.target, &right.source)` — i.e. `left`'s
        /// target resolution must be *the very same object* as `right`'s source
        /// resolution (a shared `Arc`, as produced by passing the same Python
        /// `Resolution` to both). We pre-check this and raise `ValueError`
        /// rather than letting the `assert!` panic across FFI.
        ///
        /// Both homomorphisms are standard-backend (the `ResolutionHomomorphism`
        /// pyclass is standard-only), and the shared middle resolution forces a
        /// common prime/algebra across the whole zig-zag, so no further coherence
        /// check is needed.
        ///
        /// Note: if `left`'s source resolution has a save directory *and* both
        /// homomorphisms have non-empty names, upstream `new` creates a
        /// `massey/{left},{right}/` directory on disk. The default in-memory
        /// resolutions built here have no save directory, so this path is not
        /// exercised.
        #[new]
        pub fn new(
            left: &ResolutionHomomorphism,
            right: &ResolutionHomomorphism,
        ) -> PyResult<Self> {
            if !Arc::ptr_eq(&left.0.target, &right.0.source) {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "ChainHomotopy requires left.target() to be the same resolution object as \
                     right.source(); pass the same Resolution handle to both ResolutionHomomorphisms",
                ));
            }
            Ok(ChainHomotopy(Arc::new(RsCH::new(
                Arc::clone(&left.0),
                Arc::clone(&right.0),
            ))))
        }

        /// The prime as a plain `int`.
        pub fn prime(&self) -> u32 {
            self.0.prime().as_u32()
        }

        /// The total shift bidegree `left.shift + right.shift`.
        pub fn shift(&self) -> sseq_py::Bidegree {
            sseq_py::Bidegree(self.0.shift())
        }

        /// The left homomorphism `S -> T` (shares the underlying `Arc`).
        pub fn left(&self) -> ResolutionHomomorphism {
            ResolutionHomomorphism(self.0.left())
        }

        /// The right homomorphism `T -> U` (shares the underlying `Arc`).
        pub fn right(&self) -> ResolutionHomomorphism {
            ResolutionHomomorphism(self.0.right())
        }

        /// Lift the maps so the chain homotopy is defined on every source
        /// bidegree `(s, t)` with `s <= max_source.s` and `t - s <= max_source.n`
        /// (a stem-shaped profile, matching upstream). Both underlying
        /// resolutions must be computed, and both chain maps extended, over the
        /// touched grid (see the guard); otherwise a clean `ValueError` is
        /// raised. Negative `max_source` is rejected.
        pub fn extend(&self, max_source: sseq_py::Bidegree) -> PyResult<()> {
            let b = max_source.0;
            if b.s() < 0 || b.t() < 0 {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "invalid max_source bidegree {b}: require s >= 0 and t >= 0"
                )));
            }
            // Upstream profile (exclusive): max_s = b.s() + 1, t_max(s) =
            // b.t() - b.s() + s + 1. Inclusive form below.
            self.check_extend_range(b.s(), |s| b.t() - b.s() + s)?;
            self.0.extend(b);
            Ok(())
        }

        /// Lift the maps as far as both resolutions are already resolved and
        /// both chain maps extended.
        ///
        /// Upstream `extend_all` computes its own profile by indexing
        /// `right.target.module(s + 1 - shift.s)` up to `s = max_s - 1`, where
        /// `max_s = min(left.source.next_homological_degree(),
        /// right.target.next_homological_degree() + shift.s)`. When the source is
        /// resolved at least as far as the (shifted) target — i.e.
        /// `left.source.next_homological_degree() >= right.target.next_homological_degree()
        /// + shift.s` — that profile would index the target's module at its
        /// `next_homological_degree` and panic. We reject this configuration with
        /// a `ValueError` (resolve the right target further, or use the bounded
        /// `extend(max_source)` instead, which is what the Massey workflow uses).
        pub fn extend_all(&self) -> PyResult<()> {
            let left = self.0.left();
            let right = self.0.right();
            let shift = self.0.shift();
            let n_left = left.source.next_homological_degree();
            let n_right = right.target.next_homological_degree();
            // Safe iff the source is the *strict* binding limit; otherwise the
            // upstream profile indexes right.target.module(n_right) and panics.
            if n_left >= n_right + shift.s() {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "cannot extend_all: the left source resolution (next homological degree {n_left}) \
                     is resolved at least as far as the shifted right target (next homological \
                     degree {n_right} + shift.s {}); resolve the right target further or use \
                     extend(max_source) instead",
                    shift.s()
                )));
            }
            // Inclusive top s = n_left - 1; the upstream t-profile, inclusive.
            let max_s = n_left - 1;
            let t_hi = |s: i32| {
                std::cmp::min(
                    left.source.module(s).max_computed_degree(),
                    right.target.module(s + 1 - shift.s()).max_computed_degree() + shift.t(),
                )
            };
            self.check_extend_range(max_s, t_hi)?;
            self.0.extend_all();
            Ok(())
        }

        /// The `s`-th homotopy map (`h_s: C_s -> D_{s + 1 - shift.s}`), as a bound
        /// `FreeModuleHomomorphismToFree` sharing its `Arc`.
        ///
        /// Raises `IndexError` for `s` outside the range for which the homotopy
        /// is currently defined (the populated range of the internal homotopy
        /// table, queried via upstream `defined_range`), which would otherwise
        /// panic on the `OnceBiVec` index. Call `extend`/`extend_all` first.
        ///
        /// WARNING: the returned homomorphism is a *live shared view* of this
        /// homotopy's internal map (the same `Arc`), not a copy. Treat it as
        /// read-only; calling its mutating methods is memory-safe but may
        /// logically corrupt the homotopy.
        pub fn homotopy(&self, s: i32) -> PyResult<algebra_py::FreeModuleHomomorphismToFree> {
            let range = self.0.defined_range();
            if s < range.start || s >= range.end {
                return Err(pyo3::exceptions::PyIndexError::new_err(format!(
                    "no homotopy defined at homological degree s = {s}; defined range is [{}, {}) \
                     (extend the homotopy first)",
                    range.start, range.end
                )));
            }
            Ok(algebra_py::FreeModuleHomomorphismToFree::from_arc(
                self.0.homotopy(s),
            ))
        }
    }

    /// Run a secondary `extend_all` under `catch_unwind`, translating an
    /// inherent upstream panic into a `ValueError`.
    ///
    /// The secondary lift's only remaining panic (after the pre-flight coverage
    /// guards) is the mathematical lift-validity `assert!` in upstream
    /// `compute_homotopy_step` — "secondary: Failed to lift …": it fires on a
    /// topologically invalid / non-realizable module (e.g. the cofiber of `h4`)
    /// and cannot be pre-checked without performing the computation. Per the
    /// project policy of containing a panic *only* when upstream offers no
    /// non-panicking path (matching the `from_json`/`from_string` bindings in
    /// `algebra_mod`), we catch it here and surface a `ValueError` rather than
    /// let a `PanicException` (a `BaseException`, uncaught by `except Exception`)
    /// cross the FFI boundary.
    ///
    /// `AssertUnwindSafe` is sound: `f` only appends to the `Arc`-shared,
    /// interior-mutable, append-only `OnceVec`/`OnceBiVec` homotopy tables, so a
    /// panic mid-`extend` leaves them in a valid-but-partial (memory-safe) state
    /// — there is no broken invariant for a later observer to witness.
    fn catch_secondary_lift_panic<F: FnOnce()>(f: F) -> PyResult<()> {
        use std::panic::{catch_unwind, AssertUnwindSafe};
        match catch_unwind(AssertUnwindSafe(f)) {
            Ok(()) => Ok(()),
            Err(payload) => {
                let detail = payload
                    .downcast_ref::<&str>()
                    .map(|s| (*s).to_owned())
                    .or_else(|| payload.downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| "unknown panic".to_owned());
                Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "secondary computation failed to lift (input may not be a \
                     valid/realizable module); underlying panic: {detail}"
                )))
            }
        }
    }

    /// The concrete (standard-backend) `SecondaryResolution` monomorphisation.
    type RsSecRes = ext::secondary::SecondaryResolution<ext::resolution::Resolution<ext::CCC>>;

    /// A secondary resolution is only supported over the standard backend. Nassau's algorithm
    /// stores its quasi-inverses on disk and returns them only when a save directory is present;
    /// without one, `apply_quasi_inverse` always reports failure and the secondary lift's internal
    /// `assert!` panics. Since the binding never gives Nassau a save directory, we reject the
    /// pairing up front rather than expose a guaranteed FFI panic.
    ///
    /// Held behind an `Arc` (not by value) so the *same* instance can be shared
    /// into a [`SecondaryResolutionHomomorphism`] via `Arc::clone`:
    /// `SecondaryResolutionHomomorphism::new` takes
    /// `Arc<SecondaryResolution<…>>` and `assert!`s the source/target secondary
    /// resolutions are pointer-equal to the underlying homomorphism's
    /// source/target resolutions. Every method takes `&self` (the homotopy
    /// tables are interior-mutable `OnceBiVec`s), so the `Arc` adds no friction.
    #[pyclass(frozen)]
    pub struct SecondaryResolution(Arc<RsSecRes>);

    #[pymethods]
    impl SecondaryResolution {
        #[new]
        pub fn new(cc: &Resolution) -> PyResult<Self> {
            match &cc.0 {
                AnyResolution::Standard(r) => Ok(SecondaryResolution(Arc::new(
                    ext::secondary::SecondaryResolution::new(Arc::clone(r)),
                ))),
                AnyResolution::Nassau(_) => Err(pyo3::exceptions::PyValueError::new_err(
                    "SecondaryResolution requires the standard backend (Nassau resolutions store \
                     quasi-inverses on disk and need a save directory); construct the Resolution \
                     with algorithm='standard'",
                )),
            }
        }

        /// Compute the secondary homotopies as far as the underlying resolution
        /// is resolved (the upstream `SecondaryLift::extend_all`).
        ///
        /// A topologically invalid / non-realizable module can trip the inherent
        /// upstream lift-validity `assert!` ("secondary: Failed to lift …"),
        /// which is mathematical and cannot be pre-checked without performing the
        /// computation. We contain it (`catch_unwind` -> `ValueError`) so it
        /// never crosses the FFI boundary as a `PanicException`.
        pub fn extend_all(&self) -> PyResult<()> {
            catch_secondary_lift_panic(|| self.0.extend_all())
        }

        pub fn underlying(&self) -> Resolution {
            Resolution(AnyResolution::Standard(Arc::clone(&self.0.underlying())))
        }
    }

    /// The concrete (standard→standard) `SecondaryResolutionHomomorphism`
    /// monomorphisation. As with `RsResHom`/`RsCH`, only this Standard-backend
    /// instantiation is reachable: it is built from two `SecondaryResolution`s
    /// (standard-only — Nassau is rejected at their construction) and a
    /// `ResolutionHomomorphism` (also standard-only), so all chain-complex type
    /// parameters are `ext::resolution::Resolution<CCC>`.
    type RsSecResHom = ext::secondary::SecondaryResolutionHomomorphism<
        ext::resolution::Resolution<CCC>,
        ext::resolution::Resolution<CCC>,
    >;

    /// The concrete (standard) `SecondaryChainHomotopy` monomorphisation. All
    /// three chain-complex type parameters are `ext::resolution::Resolution<CCC>`
    /// for the same reason as `RsSecResHom`.
    type RsSecCH = ext::secondary::SecondaryChainHomotopy<
        ext::resolution::Resolution<CCC>,
        ext::resolution::Resolution<CCC>,
        ext::resolution::Resolution<CCC>,
    >;

    /// The secondary (`Mod_{Cλ²}`) lift of a `ResolutionHomomorphism`: the
    /// datum that promotes a chain map of resolutions to a map respecting the
    /// secondary (`d₂`) structure, used to compute secondary products (see
    /// `examples/secondary_product.rs`).
    ///
    /// Built from a `source` and `target` `SecondaryResolution` and the
    /// `underlying` `ResolutionHomomorphism` between their underlying
    /// resolutions. Upstream `new` `assert!`s that `underlying.source` /
    /// `underlying.target` are the *same* resolution objects (`Arc::ptr_eq`) as
    /// the source/target secondary resolutions' underlying resolutions; we
    /// pre-check this and raise `ValueError` rather than let the assert panic
    /// across FFI (mirroring `ChainHomotopy::new`).
    ///
    /// Held behind an `Arc` so it can be shared into a [`SecondaryChainHomotopy`]
    /// (whose `new` takes `Arc<SecondaryResolutionHomomorphism<…>>`). The source
    /// and target secondary-resolution `Arc`s are stored alongside so
    /// `extend_all` can verify (through public `homotopies()` ranges) that they
    /// have been extended far enough, raising `ValueError` rather than indexing
    /// an unpopulated `OnceBiVec`.
    ///
    /// Standard backend only (see `RsSecResHom`).
    #[pyclass(frozen)]
    pub struct SecondaryResolutionHomomorphism {
        inner: Arc<RsSecResHom>,
        source: Arc<RsSecRes>,
        target: Arc<RsSecRes>,
    }

    impl SecondaryResolutionHomomorphism {
        /// Pre-flight guard for `extend_all`. `extend_all` drives
        /// `compute_composites`, which iterates the homotopy `OnceBiVec` over
        /// `[shift.s, max_s)` (where `max_s = underlying.next_homological_degree()`,
        /// the eager part of `max()`), and for each `s` evaluates the `max()`
        /// closure, which *indexes* `source.homotopies[s]` (and, for
        /// `s > shift.s`, `target.homotopies[s + 1 - shift.s]`). Indexing an
        /// unpopulated `OnceBiVec` panics, so we require the source/target
        /// secondary resolutions to be extended far enough up front.
        ///
        /// The internal degree of each step is itself clamped by `max()` to
        /// `underlying.get_map(s).next_degree()`, so the underlying
        /// homomorphism need not be extended any further than it already is —
        /// `extend_all` simply does less work — and no extra `t`-grid check is
        /// needed here. (The inherent mathematical lift-validity `assert!` in
        /// `compute_homotopy_step` — "secondary: Failed to lift …" — fires only
        /// on topologically invalid input and cannot be pre-checked without
        /// performing the computation; upstream itself treats it as a panic.)
        fn check_extend_all(&self) -> PyResult<()> {
            let max_s = self.inner.max().s();
            let shift_s = self.inner.shift().s();
            if max_s <= shift_s {
                // Empty touched range: extend_all is a safe no-op.
                return Ok(());
            }
            let src_h = self.source.homotopies();
            // source.homotopies[s] is read for every s in [shift_s, max_s - 1].
            if src_h.min_degree() > shift_s || src_h.len() < max_s {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "the source SecondaryResolution is not extended far enough (its secondary \
                     homotopies cover [{}, {}), but extending this homomorphism reads s in [{}, \
                     {}]); call source.extend_all() first",
                    src_h.min_degree(),
                    src_h.len(),
                    shift_s,
                    max_s - 1
                )));
            }
            // target.homotopies[s'] is read for s' = s + 1 - shift_s with
            // s in [shift_s + 1, max_s - 1], i.e. s' in [2, max_s - shift_s].
            let tgt_top = max_s - shift_s;
            if tgt_top >= 2 {
                let tgt_h = self.target.homotopies();
                if tgt_h.min_degree() > 2 || tgt_h.len() <= tgt_top {
                    return Err(pyo3::exceptions::PyValueError::new_err(format!(
                        "the target SecondaryResolution is not extended far enough (its secondary \
                         homotopies cover [{}, {}), but extending this homomorphism reads s up to \
                         {tgt_top}); call target.extend_all() first",
                        tgt_h.min_degree(),
                        tgt_h.len()
                    )));
                }
            }
            Ok(())
        }
    }

    #[pymethods]
    impl SecondaryResolutionHomomorphism {
        /// Construct the secondary lift of `underlying` over `source`/`target`.
        ///
        /// `underlying` must be the `ResolutionHomomorphism` between exactly the
        /// `source` and `target` secondary resolutions' underlying resolutions:
        /// upstream `assert!`s `Arc::ptr_eq(&underlying.source, &source.underlying)`
        /// and `Arc::ptr_eq(&underlying.target, &target.underlying)`. We pre-check
        /// both and raise `ValueError` (never panic). All three objects are
        /// standard-backend (enforced at their own construction), so the shared
        /// resolutions force a common prime/algebra and no further coherence
        /// check is needed.
        ///
        /// Construction does not require any of the three to be computed/extended
        /// yet (only `extend_all` does — see its guard).
        #[new]
        pub fn new(
            source: &SecondaryResolution,
            target: &SecondaryResolution,
            underlying: &ResolutionHomomorphism,
        ) -> PyResult<Self> {
            if !Arc::ptr_eq(&underlying.0.source, &source.0.underlying()) {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "SecondaryResolutionHomomorphism requires the underlying homomorphism's source \
                     to be the same resolution object as the source SecondaryResolution's \
                     underlying resolution; build the homomorphism from source.underlying()",
                ));
            }
            if !Arc::ptr_eq(&underlying.0.target, &target.0.underlying()) {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "SecondaryResolutionHomomorphism requires the underlying homomorphism's target \
                     to be the same resolution object as the target SecondaryResolution's \
                     underlying resolution; build the homomorphism from target.underlying()",
                ));
            }
            Ok(SecondaryResolutionHomomorphism {
                inner: Arc::new(RsSecResHom::new(
                    Arc::clone(&source.0),
                    Arc::clone(&target.0),
                    Arc::clone(&underlying.0),
                )),
                source: Arc::clone(&source.0),
                target: Arc::clone(&target.0),
            })
        }

        /// The homomorphism's name, bracketed (`[name]`) to mark it as the
        /// secondary lift (matching upstream `name()`).
        pub fn name(&self) -> String {
            self.inner.name()
        }

        /// The prime as a plain `int`.
        pub fn prime(&self) -> u32 {
            self.inner.prime().as_u32()
        }

        /// The Steenrod algebra the resolutions are built over.
        pub fn algebra(&self) -> algebra_py::SteenrodAlgebra {
            algebra_py::SteenrodAlgebra::from_arc(self.inner.algebra())
        }

        /// The shift bidegree of the secondary lift (`underlying.shift + (1, 0)`).
        pub fn shift(&self) -> sseq_py::Bidegree {
            sseq_py::Bidegree(self.inner.shift())
        }

        /// The source resolution (the *underlying* resolution of the source
        /// secondary resolution; shares its `Arc`).
        pub fn source(&self) -> Resolution {
            Resolution(AnyResolution::Standard(self.inner.source()))
        }

        /// The target resolution (the *underlying* resolution of the target
        /// secondary resolution; shares its `Arc`).
        pub fn target(&self) -> Resolution {
            Resolution(AnyResolution::Standard(self.inner.target()))
        }

        /// The underlying `ResolutionHomomorphism` (shares its `Arc`; a live
        /// shared view — extending it is visible here and vice versa).
        pub fn underlying(&self) -> ResolutionHomomorphism {
            ResolutionHomomorphism(self.inner.underlying())
        }

        /// The directory used to persist the lift, or `None` if held in memory
        /// (the default for the in-memory resolutions built here).
        pub fn save_dir(&self) -> Option<String> {
            self.inner
                .save_dir()
                .read()
                .map(|p| p.display().to_string())
        }

        /// Compute the secondary homotopies as far as the source/target
        /// secondary resolutions and the underlying homomorphism are computed
        /// (the upstream `SecondaryLift::extend_all`).
        ///
        /// The source and target `SecondaryResolution`s must have been
        /// `extend_all`-ed far enough first (see the guard); otherwise a clean
        /// `ValueError` is raised rather than indexing an unpopulated
        /// `OnceBiVec`. A topologically invalid input can still trip the
        /// inherent upstream lift-validity `assert!`; that condition is
        /// mathematical and cannot be pre-checked without performing the
        /// computation, so it is contained (`catch_unwind` -> `ValueError`)
        /// rather than crossing the FFI boundary as a `PanicException`.
        pub fn extend_all(&self) -> PyResult<()> {
            self.check_extend_all()?;
            catch_secondary_lift_panic(|| self.inner.extend_all())
        }
    }

    /// The secondary (`Mod_{Cλ²}`) lift of a `ChainHomotopy`: the datum used to
    /// assemble secondary Massey products (see `examples/secondary_massey.rs`).
    ///
    /// Built from the secondary lifts `left`/`right` of the two homomorphisms
    /// the underlying `ChainHomotopy` is a null-homotopy of, optional λ-parts
    /// `left_lambda`/`right_lambda` (the non-standard-lift part of each class),
    /// and the `underlying` `ChainHomotopy`. Upstream `new` `assert!`s a chain
    /// of `Arc::ptr_eq` structural preconditions (and shift relations for the
    /// λ-parts); we pre-check every one and raise `ValueError` rather than let
    /// an assert panic across FFI.
    ///
    /// Standard backend only (see `RsSecCH`). This class is bound for
    /// construction and structural inspection; its homotopy *computation*
    /// (`extend_all`/`compute_partial`) is deferred — see the module notes.
    #[pyclass(frozen)]
    pub struct SecondaryChainHomotopy(Arc<RsSecCH>);

    #[pymethods]
    impl SecondaryChainHomotopy {
        /// Construct the secondary lift of `underlying` from the secondary
        /// homomorphism lifts `left`/`right` and optional λ-parts.
        ///
        /// Pre-checks (all `ValueError`, never a panic), mirroring upstream's
        /// `assert!`s:
        ///  - `underlying.left()` is the same homomorphism object (`Arc::ptr_eq`)
        ///    as `left`'s underlying homomorphism, and likewise for `right`;
        ///  - if `left_lambda` is given, its `source`/`target` are the same
        ///    objects as `underlying.left()`'s, and its shift equals
        ///    `underlying.left().shift + LAMBDA_BIDEGREE`; likewise `right_lambda`.
        ///
        /// `left_lambda`/`right_lambda` default to `None` (standard lifts).
        #[new]
        #[pyo3(signature = (left, right, underlying, left_lambda=None, right_lambda=None))]
        pub fn new(
            left: &SecondaryResolutionHomomorphism,
            right: &SecondaryResolutionHomomorphism,
            underlying: &ChainHomotopy,
            left_lambda: Option<&ResolutionHomomorphism>,
            right_lambda: Option<&ResolutionHomomorphism>,
        ) -> PyResult<Self> {
            let u_left = underlying.0.left();
            let u_right = underlying.0.right();
            if !Arc::ptr_eq(&u_left, &left.inner.underlying()) {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "SecondaryChainHomotopy requires underlying.left() to be the same \
                     homomorphism object as left's underlying ResolutionHomomorphism",
                ));
            }
            if !Arc::ptr_eq(&u_right, &right.inner.underlying()) {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "SecondaryChainHomotopy requires underlying.right() to be the same \
                     homomorphism object as right's underlying ResolutionHomomorphism",
                ));
            }
            let lambda = ext::secondary::LAMBDA_BIDEGREE;
            if let Some(ll) = left_lambda {
                if !Arc::ptr_eq(&ll.0.source, &u_left.source)
                    || !Arc::ptr_eq(&ll.0.target, &u_left.target)
                {
                    return Err(pyo3::exceptions::PyValueError::new_err(
                        "left_lambda must have the same source/target resolutions as \
                         underlying.left()",
                    ));
                }
                if ll.0.shift != u_left.shift + lambda {
                    return Err(pyo3::exceptions::PyValueError::new_err(format!(
                        "left_lambda shift {} must equal underlying.left().shift + LAMBDA_BIDEGREE \
                         ({})",
                        ll.0.shift,
                        u_left.shift + lambda
                    )));
                }
            }
            if let Some(rl) = right_lambda {
                if !Arc::ptr_eq(&rl.0.source, &u_right.source)
                    || !Arc::ptr_eq(&rl.0.target, &u_right.target)
                {
                    return Err(pyo3::exceptions::PyValueError::new_err(
                        "right_lambda must have the same source/target resolutions as \
                         underlying.right()",
                    ));
                }
                if rl.0.shift != u_right.shift + lambda {
                    return Err(pyo3::exceptions::PyValueError::new_err(format!(
                        "right_lambda shift {} must equal underlying.right().shift + \
                         LAMBDA_BIDEGREE ({})",
                        rl.0.shift,
                        u_right.shift + lambda
                    )));
                }
            }
            Ok(SecondaryChainHomotopy(Arc::new(RsSecCH::new(
                Arc::clone(&left.inner),
                Arc::clone(&right.inner),
                left_lambda.map(|x| Arc::clone(&x.0)),
                right_lambda.map(|x| Arc::clone(&x.0)),
                Arc::clone(&underlying.0),
            ))))
        }

        /// The prime as a plain `int`.
        pub fn prime(&self) -> u32 {
            self.0.prime().as_u32()
        }

        /// The Steenrod algebra the resolutions are built over.
        pub fn algebra(&self) -> algebra_py::SteenrodAlgebra {
            algebra_py::SteenrodAlgebra::from_arc(self.0.algebra())
        }

        /// The total shift bidegree of the secondary chain homotopy.
        pub fn shift(&self) -> sseq_py::Bidegree {
            sseq_py::Bidegree(self.0.shift())
        }

        /// The source resolution (`left`'s source; shares its `Arc`).
        pub fn source(&self) -> Resolution {
            Resolution(AnyResolution::Standard(self.0.source()))
        }

        /// The target resolution (`right`'s target; shares its `Arc`).
        pub fn target(&self) -> Resolution {
            Resolution(AnyResolution::Standard(self.0.target()))
        }

        /// The underlying `ChainHomotopy` (shares its `Arc`; a live shared view).
        pub fn underlying(&self) -> ChainHomotopy {
            ChainHomotopy(self.0.underlying())
        }

        /// The directory used to persist the lift, or `None` if held in memory.
        pub fn save_dir(&self) -> Option<String> {
            self.0.save_dir().read().map(|p| p.display().to_string())
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

        /// The `s`-th module `C_s`, as a *clone* (snapshot) of the underlying
        /// module — not a shared `Arc` view, since `SteenrodModule` wraps the
        /// value by clone. Out-of-range `s` (`>=` the number of modules) returns
        /// the zero module, matching upstream. Raises `ValueError` for negative
        /// `s`.
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

    /// The inner monomorphisation of a `FullModuleHomomorphism` between boxed
    /// dynamic modules — the differential/chain-map type of `CCC` and the
    /// element type the bound `FullModuleHomomorphism` pyclass holds (matching
    /// `algebra_py`'s `FullModuleHomomorphismInner`).
    type FullHomInner = RsFullModuleHomomorphism<RsSteenrodModule, RsSteenrodModule>;

    /// The augmented finite chain complex bound here: a `CCC` (a
    /// `FiniteChainComplex<SteenrodModule>`) together with an augmentation chain
    /// map to a target `CCC`. Both the interior differentials and the
    /// augmentation maps are `FullModuleHomomorphism<SteenrodModule>`, exactly
    /// the bound `FullModuleHomomorphism` pyclass's inner type.
    type FACC = RsFiniteAugmentedChainComplex<RsSteenrodModule, FullHomInner, FullHomInner, CCC>;

    /// An augmented finite chain complex `C -> D`: a finite chain complex `C`
    /// (the `cc`) plus an augmentation chain map to a target complex `D` (the
    /// `target`). This is the structure `utils::construct` and `yoneda`
    /// produce; here it is constructible directly from explicit modules,
    /// differentials, a target complex, and one augmentation map per module.
    ///
    /// Stored as an `Arc<FACC>` plus the number of augmentation maps (so
    /// `chain_map(s)` can be range-guarded — upstream exposes no length
    /// accessor and `chain_map` panics out of range). `frozen`: every method
    /// takes `&self` and reads interior-mutable module tables.
    ///
    /// The `ChainComplex`/`FreeChainComplex` query surface mirrors the
    /// `ChainComplex` pyclass (the underlying `cc` is a `CCC`), so only the
    /// genuinely new augmented surface (`target`, `chain_map`) plus the shared
    /// `ChainComplex` accessors are bound; the free-module-only methods are
    /// absent for the same reason as on `ChainComplex` (the modules are
    /// arbitrary `SteenrodModule`s).
    #[pyclass(frozen)]
    pub struct FiniteAugmentedChainComplex {
        inner: Arc<FACC>,
        num_chain_maps: usize,
    }

    impl FiniteAugmentedChainComplex {
        /// Wrap an upstream `FACC` (e.g. the result of a yoneda computation,
        /// after its `FDModule` modules have been erased to `RsSteenrodModule`)
        /// in the bound pyclass. Takes ownership of the `Arc`-able value.
        ///
        /// `num_chain_maps` is the number of augmentation maps, which for a
        /// `FiniteChainComplex::augment` is exactly the number of modules; the
        /// bounded-complex `max_s()` returns `modules.len()` (see upstream
        /// `FiniteChainComplex::max_s`), so it is the correct
        /// `chain_map(s)`-guard bound.
        pub(crate) fn from_rust(inner: FACC) -> Self {
            use ext::chain_complex::BoundedChainComplex;
            let num_chain_maps = inner.max_s() as usize;
            FiniteAugmentedChainComplex {
                inner: Arc::new(inner),
                num_chain_maps,
            }
        }
    }

    /// Compute a Yoneda representative of an Ext class.
    ///
    /// Given a (standard-backend) `resolution`, a bidegree `b`, and an Ext class
    /// `class` (a `list[int]` of length `number_of_gens_in_bidegree(b)`, the
    /// coordinates of the class in the generator basis at `b`), this returns a
    /// `FiniteAugmentedChainComplex` — a quasi-isomorphic finite quotient of the
    /// resolution that the cohomology class factors through, i.e. the geometric
    /// Yoneda representative (see upstream `ext::yoneda::yoneda_representative_element`
    /// and `examples/yoneda.rs`).
    ///
    /// **Standard backend only.** Yoneda operates on a
    /// `resolution::Resolution<CCC>` (its modules are `FreeModule`s over the
    /// `SteenrodAlgebra` and its target is a `CCC`); a Nassau-backed `Resolution`
    /// resolves over the concrete `MilnorAlgebra` and a different complex type, so
    /// it is rejected with a `ValueError`, mirroring `ResolutionHomomorphism` /
    /// `SecondaryResolution` / `chain_complex()`.
    ///
    /// Raises:
    /// * `ValueError` if `resolution` is Nassau-backed;
    /// * `ValueError` if `b` has a negative `s` or `t`;
    /// * `ValueError` if any `class[i] >= p` (the prime), since each entry is
    ///   written into an `FpVector` over `p`;
    /// * `ValueError` if upstream `try_yoneda_representative_element` reports an
    ///   error: the bidegree is uncomputed, `len(class)` does not match the
    ///   generator count, or an internal sanity check (Euler characteristic /
    ///   lift round-trip) fails. These are surfaced as a `Result` rather than
    ///   panicking across the FFI boundary.
    ///
    /// The returned complex's modules are independently-owned `FDModule`s (erased
    /// to `SteenrodModule`); only its augmentation `target()` shares an `Arc` with
    /// the input resolution's target complex (treat that as a read-only live view).
    #[pyfunction]
    pub fn yoneda_representative_element(
        resolution: &Resolution,
        b: sseq_py::Bidegree,
        class: Vec<u32>,
    ) -> PyResult<FiniteAugmentedChainComplex> {
        // Backend: standard only (Nassau resolves over a different algebra/complex).
        let res = match &resolution.0 {
            AnyResolution::Standard(r) => Arc::clone(r),
            AnyResolution::Nassau(_) => {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "yoneda_representative_element requires the standard backend; the resolution \
                     is Nassau-backed (over the concrete MilnorAlgebra and a different complex \
                     type). Construct the Resolution with algorithm='standard'.",
                ));
            }
        };

        let bd = b.0;
        if bd.s() < 0 || bd.t() < 0 {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "invalid bidegree {bd}: require s >= 0 and t >= 0"
            )));
        }

        // Each entry is written into an FpVector over the prime; reject out-of-range.
        let p = res.prime().as_u32();
        for (i, &v) in class.iter().enumerate() {
            if v >= p {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "class[{i}] = {v} is out of range for prime p = {p}; entries must be in [0, p)"
                )));
            }
        }

        // `try_yoneda_representative_element` validates the remaining
        // preconditions (the bidegree is computed, `class` has one coordinate per
        // generator) and surfaces its internal sanity checks (Euler characteristic
        // / lift round-trip) as an error rather than panicking.
        let result = ext::yoneda::try_yoneda_representative_element(res, bd, &class)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

        // The yoneda result's modules are `FDModule`s; erase them to the dynamic
        // `SteenrodModule` the bound `FiniteAugmentedChainComplex` (a `FACC`) holds.
        // This mirrors `utils.rs`'s `yoneda.map(|m| steenrod_module::erase(m.clone()))`.
        let erased = result.map(|m| algebra::module::steenrod_module::erase(m.clone()));
        Ok(FiniteAugmentedChainComplex::from_rust(erased))
    }

    #[pymethods]
    impl FiniteAugmentedChainComplex {
        /// Build an augmented finite chain complex from an explicit list of
        /// `modules` (`C_0, ..., C_n`), the interior `differentials`
        /// (`differentials[i]: C_{i+1} -> C_i`), a `target` chain complex `D`,
        /// and the augmentation `chain_maps` (`chain_maps[s]: C_s -> D_s`, one
        /// per module). Mirrors `ChainComplex.new` for the `C`-side validation
        /// and additionally validates the augmentation.
        ///
        /// Raises `ValueError` if (all checks mirror upstream conventions):
        /// * `modules` is empty;
        /// * `differentials.len() != modules.len() - 1` (one interior
        ///   differential per consecutive pair `C_{i+1} -> C_i`);
        /// * `chain_maps.len() != modules.len()` — the augmentation has exactly
        ///   one map per source module (`chain_map(s)` returns `chain_maps[s]`);
        /// * the modules / differentials / chain maps / target do not all share
        ///   the same prime and algebra object (`Arc::ptr_eq`).
        ///
        /// As in `ChainComplex.new`, the exact *source/target module identity*
        /// of each differential and chain map is **not** checked (there is no
        /// cheap structural equality on `dyn Module`, and `Arc::ptr_eq` would
        /// reject legitimate cloned handles); the prime+algebra checks reject the
        /// incoherent cases without rejecting a consistently-built complex.
        #[new]
        pub fn new(
            py: Python<'_>,
            modules: Vec<Py<algebra_py::SteenrodModule>>,
            differentials: Vec<Py<algebra_py::FullModuleHomomorphism>>,
            target: &ChainComplex,
            chain_maps: Vec<Py<algebra_py::FullModuleHomomorphism>>,
        ) -> PyResult<Self> {
            if modules.is_empty() {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "FiniteAugmentedChainComplex.new requires at least one module",
                ));
            }
            let n_modules = modules.len();
            let expected_diffs = n_modules - 1;
            if differentials.len() != expected_diffs {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "FiniteAugmentedChainComplex.new expects exactly {expected_diffs} \
                     differential(s) for {n_modules} module(s) (differentials[i] is the map \
                     C_(i+1) -> C_i); got {}",
                    differentials.len()
                )));
            }
            if chain_maps.len() != n_modules {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "FiniteAugmentedChainComplex.new expects exactly {n_modules} augmentation \
                     chain map(s) (one per module, chain_maps[s]: C_s -> D_s); got {}",
                    chain_maps.len()
                )));
            }
            let modules: Vec<Arc<RsSteenrodModule>> = modules
                .iter()
                .map(|m| Arc::new(m.borrow(py).as_rust().clone()))
                .collect();
            let ref_algebra = modules[0].algebra();
            let p = modules[0].prime().as_u32();
            for (i, m) in modules.iter().enumerate() {
                if m.prime().as_u32() != p {
                    return Err(pyo3::exceptions::PyValueError::new_err(format!(
                        "all modules must share the same prime; module 0 is over p={p} but \
                         module {i} is over p={}",
                        m.prime().as_u32()
                    )));
                }
                if !Arc::ptr_eq(&m.algebra(), &ref_algebra) {
                    return Err(pyo3::exceptions::PyValueError::new_err(format!(
                        "all modules must be built over the same algebra object; module {i} is \
                         over a different algebra than module 0"
                    )));
                }
            }
            // The target complex must be over the same prime + algebra.
            if target.0.prime().as_u32() != p {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "the target complex is over p={} but the complex is over p={p}",
                    target.0.prime().as_u32()
                )));
            }
            if !Arc::ptr_eq(&target.0.algebra(), &ref_algebra) {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "the target complex must be built over the same algebra object as the modules",
                ));
            }
            let check_hom = |i: usize, d: &algebra_py::FullModuleHomomorphism, kind: &str| {
                if d.prime() != p {
                    return Err(pyo3::exceptions::PyValueError::new_err(format!(
                        "{kind} {i} is over p={} but the complex is over p={p}",
                        d.prime()
                    )));
                }
                if !Arc::ptr_eq(&d.source_algebra(), &ref_algebra)
                    || !Arc::ptr_eq(&d.target_algebra(), &ref_algebra)
                {
                    return Err(pyo3::exceptions::PyValueError::new_err(format!(
                        "{kind} {i} must be built over the same algebra object as the complex's \
                         modules"
                    )));
                }
                Ok(())
            };
            let differentials = differentials
                .iter()
                .enumerate()
                .map(|(i, d)| {
                    let d = d.borrow(py);
                    check_hom(i, &d, "differential")?;
                    Ok(Arc::new(d.clone_rust()))
                })
                .collect::<PyResult<Vec<_>>>()?;
            let chain_maps_rust = chain_maps
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    let c = c.borrow(py);
                    check_hom(i, &c, "chain map")?;
                    Ok(Arc::new(c.clone_rust()))
                })
                .collect::<PyResult<Vec<_>>>()?;
            let cc = CCC::new(modules, differentials);
            let augmented = cc.augment(Arc::clone(&target.0), chain_maps_rust);
            Ok(FiniteAugmentedChainComplex {
                inner: Arc::new(augmented),
                num_chain_maps: n_modules,
            })
        }

        /// The prime as a plain `int`.
        pub fn prime(&self) -> u32 {
            self.inner.prime().as_u32()
        }

        /// The Steenrod algebra the complex is built over.
        pub fn algebra(&self) -> algebra_py::SteenrodAlgebra {
            algebra_py::SteenrodAlgebra::from_arc(self.inner.algebra())
        }

        /// The minimum internal degree shared by every module.
        pub fn min_degree(&self) -> i32 {
            self.inner.min_degree()
        }

        /// The first `s` for which `module(s)` is not defined (`i32::MAX` for a
        /// finite complex; `iter`-style helpers are therefore not bound here,
        /// matching `ChainComplex`).
        pub fn next_homological_degree(&self) -> i32 {
            self.inner.next_homological_degree()
        }

        /// The zero module.
        pub fn zero_module(&self) -> algebra_py::SteenrodModule {
            algebra_py::SteenrodModule::from_rust((*self.inner.zero_module()).clone())
        }

        /// The `s`-th module `C_s`, as a *clone* (snapshot) of the underlying
        /// module — not a shared `Arc` view, since `SteenrodModule` wraps the
        /// value by clone. Out-of-range `s` returns the zero module (matching
        /// upstream). Raises `ValueError` for negative `s`.
        pub fn module(&self, s: i32) -> PyResult<algebra_py::SteenrodModule> {
            if s < 0 {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "homological degree s must be non-negative",
                ));
            }
            Ok(algebra_py::SteenrodModule::from_rust(
                (*self.inner.module(s)).clone(),
            ))
        }

        /// The differential `C_s -> C_{s-1}`, as a bound `FullModuleHomomorphism`.
        /// Out-of-range `s` returns a zero homomorphism. Raises `ValueError` for
        /// negative `s`.
        pub fn differential(&self, s: i32) -> PyResult<algebra_py::FullModuleHomomorphism> {
            if s < 0 {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "homological degree s must be non-negative",
                ));
            }
            Ok(algebra_py::FullModuleHomomorphism::from_rust(
                (*self.inner.differential(s)).clone(),
            ))
        }

        /// Whether the complex has been computed at bidegree `b`. Negative
        /// `s`/`t` is rejected with a `ValueError`.
        pub fn has_computed_bidegree(&self, b: sseq_py::Bidegree) -> PyResult<bool> {
            if b.0.s() < 0 || b.0.t() < 0 {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "invalid bidegree {}: require s >= 0 and t >= 0",
                    b.0
                )));
            }
            Ok(self.inner.has_computed_bidegree(b.0))
        }

        /// Ensure every bidegree `<= b` has been computed. Negative `s`/`t` is
        /// rejected with a `ValueError`.
        pub fn compute_through_bidegree(&self, b: sseq_py::Bidegree) -> PyResult<()> {
            if b.0.s() < 0 || b.0.t() < 0 {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "invalid target bidegree {}: require s >= 0 and t >= 0",
                    b.0
                )));
            }
            self.inner.compute_through_bidegree(b.0);
            Ok(())
        }

        /// The augmentation target complex `D`, as a bound `ChainComplex`
        /// sharing the underlying `Arc`. (Because it shares the `Arc`, the
        /// returned complex cannot be `pop`-ped — `pop` requires sole ownership.)
        pub fn target(&self) -> ChainComplex {
            ChainComplex(self.inner.target())
        }

        /// The `s`-th augmentation chain map `C_s -> D_s`, as a bound
        /// `FullModuleHomomorphism` (a clone of the shared map, mirroring
        /// `differential`). Raises `IndexError` for `s` outside
        /// `[0, len(chain_maps))` (upstream `chain_map` indexes a `Vec` and would
        /// otherwise panic).
        pub fn chain_map(&self, s: i32) -> PyResult<algebra_py::FullModuleHomomorphism> {
            if s < 0 || s as usize >= self.num_chain_maps {
                return Err(pyo3::exceptions::PyIndexError::new_err(format!(
                    "no augmentation chain map at homological degree s = {s}; defined range is \
                     [0, {})",
                    self.num_chain_maps
                )));
            }
            Ok(algebra_py::FullModuleHomomorphism::from_rust(
                (*self.inner.chain_map(s)).clone(),
            ))
        }

        /// The maximum homological degree `s` with `C_s != 0` (the bounded-complex
        /// `max_s`).
        pub fn max_s(&self) -> i32 {
            use ext::chain_complex::BoundedChainComplex;
            self.inner.max_s()
        }
    }

    #[pymodule_init]
    fn init(_m: &Bound<'_, PyModule>) -> PyResult<()> {
        ext::utils::init_logging()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(())
    }
}
