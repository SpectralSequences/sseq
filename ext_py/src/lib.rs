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
            steenrod_module, FDModule, Module, SteenrodModule as RsSteenrodModule,
        },
        Algebra,
    };
    use ext::{
        chain_complex::{
            AugmentedChainComplex, BoundedChainComplex, ChainComplex as RsChainComplex,
            ChainHomotopy as RsChainHomotopy,
            FiniteAugmentedChainComplex as RsFiniteAugmentedChainComplex,
            FiniteChainComplex as RsFiniteChainComplex, FreeChainComplex,
        },
        resolution_homomorphism::{
            ResolutionHomomorphism as RsResolutionHomomorphism,
            UnstableResolutionHomomorphism as RsUnstableResolutionHomomorphism,
        },
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

    /// Validate that a bidegree-like value is non-negative, evaluating to `$b`.
    ///
    /// Factors out the repeated negative-bidegree guard. On failure it `return`s an
    /// `Err(PyValueError)` from the *caller* (the s-only arm checks `s` only and drops
    /// `and t >= 0` from the message). `$what` is a string literal naming the value
    /// (e.g. `"target bidegree"`). The message is byte-for-byte identical to the
    /// hand-written guards it replaces, so the error text seen from Python is unchanged.
    macro_rules! require_nonneg {
        ($b:expr, $what:literal) => {{
            let b = $b;
            if b.s() < 0 || b.t() < 0 {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    concat!("invalid ", $what, " {}: require s >= 0 and t >= 0"),
                    b
                )));
            }
            b
        }};
        ($b:expr, $what:literal, s_only) => {{
            let b = $b;
            if b.s() < 0 {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    concat!("invalid ", $what, " {}: require s >= 0"),
                    b
                )));
            }
            b
        }};
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
        algebra_type: Option<algebra_py::AlgebraTypeArg>,
        save: bool,
    ) -> PyResult<Resolution> {
        ext::utils::query_module(algebra_type.map(algebra::AlgebraType::from), save)
            .map(|res| Resolution(AnyResolution::Standard(Arc::new(res))))
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    #[pyfunction]
    pub fn query_module_only(
        prompt: &str,
        algebra: Option<algebra_py::AlgebraTypeArg>,
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
    /// bad *argument*, so all are mapped to `ValueError`.
    ///
    /// The one previously-panicking path — a `[k]` shift suffix applied to a
    /// module JSON whose existing `"shift"` field is not an integer (upstream did
    /// `spec_shift.as_i64().unwrap()`) — now returns `anyhow::Err` with context
    /// instead of panicking, so the call is a plain `Result` mapped via `map_err`
    /// (no `catch_unwind` needed).
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

    /// Compile-time guard documenting that `ext::utils::QueryModuleResolution`
    /// (the type upstream `get_unit`/`construct` traffic in) is *exactly*
    /// `Resolution<CCC>`, the inner type of [`AnyResolution::Standard`]. This
    /// identity holds ONLY because `ext` depends on `ext` WITHOUT the `nassau`
    /// feature; if `ext/nassau` is ever enabled, `QueryModuleResolution` flips to
    /// `nassau::Resolution<FDModule<MilnorAlgebra>>` and the coercion below fails
    /// to COMPILE (a loud, type-level error — never silent unsoundness). The
    /// `get_unit` binding relies on this identity to return its constructed unit
    /// as an `AnyResolution::Standard`.
    #[allow(dead_code)]
    fn assert_query_module_is_standard(
        r: ext::utils::QueryModuleResolution,
    ) -> ext::resolution::Resolution<CCC> {
        r
    }

    /// Given a resolution, return `(is_unit, unit_resolution)`: a flag for
    /// whether the input already resolves the unit, and a resolution of the unit
    /// (the input itself when `is_unit` is true).
    ///
    /// Mirrors
    /// `ext::utils::get_unit(Arc<QueryModuleResolution>) -> anyhow::Result<(bool, Arc<QueryModuleResolution>)>`,
    /// **but with the interactive prompt removed**. Upstream's non-unit branch
    /// calls `query::optional("Unit save directory", …)` to obtain a save
    /// directory before constructing the unit resolution; `query::*` consumes
    /// process argv, blocks on stdin, and can `std::process::exit(1)`. None of
    /// that is permissible across the FFI boundary (the project invariant: all
    /// interactive I/O lives in the Python layer), so this binding NEVER calls
    /// `ext::utils::get_unit`. Instead it:
    ///   * computes the same `is_unit` predicate locally
    ///     (`target().max_s() == 1 && target().module(0).is_unit()` — cheap,
    ///     non-panicking reads), and
    ///   * when the input is NOT the unit, replicates upstream's non-unit
    ///     construction *non-interactively*, threading the Python-provided
    ///     `save_dir` (mirroring `construct`/`construct_unstable`) in place of the
    ///     prompted directory: it builds the one-dimensional `FDModule` "unit"
    ///     over the resolution's own algebra, wraps it in `FiniteChainComplex::ccdz`,
    ///     and calls `Resolution::new_with_save` exactly as upstream's
    ///     `#[cfg(not(feature = "nassau"))]` arm does.
    ///
    /// `ext` builds `ext` without the `nassau` feature, so
    /// `QueryModuleResolution = Resolution<CCC>` (see the feature-fragility note
    /// on [`assert_query_module_is_standard`]), which is exactly the inner type of
    /// [`AnyResolution::Standard`]; a Nassau-backed input cannot be passed and is
    /// rejected with `ValueError` (mirroring `chain_complex()` /
    /// `ResolutionHomomorphism`'s standard-only precedent).
    ///
    /// # Arguments
    ///  - `resolution`: the (standard-backend) resolution to find the unit of.
    ///  - `save_dir`: optional filesystem path for the freshly-constructed unit
    ///    resolution, used ONLY on the non-unit path (when `is_unit` is true the
    ///    input is returned as-is, a cheap shared-`Arc` with no construction and
    ///    no save dir). Behaves exactly as in [`construct`]: an existing path that
    ///    is not a directory is a `ValueError`; a non-existent path is created by
    ///    upstream `Resolution::new_with_save`.
    ///
    /// Error taxonomy: Nassau backend or save_dir-is-a-file -> `ValueError`;
    /// a genuine IO failure creating the unit resolution -> `RuntimeError`.
    /// Nothing panics, consumes argv, reads stdin, or exits across the FFI
    /// boundary.
    #[pyfunction]
    #[pyo3(signature = (resolution, save_dir=None))]
    pub fn get_unit(
        resolution: &Resolution,
        save_dir: Option<String>,
    ) -> PyResult<(bool, Resolution)> {
        let arc =
            match &resolution.0 {
                AnyResolution::Standard(r) => Arc::clone(r),
                AnyResolution::Nassau(_) => return Err(pyo3::exceptions::PyValueError::new_err(
                    "get_unit() is only available on the standard backend; the Nassau algorithm \
                     resolves over the concrete MilnorAlgebra and has no get_unit analogue here",
                )),
            };

        // Same predicate as upstream `ext::utils::get_unit`.
        let target = arc.target();
        let is_unit = target.max_s() == 1 && target.module(0).is_unit();

        if is_unit {
            // Cheap shared-Arc path: the input already resolves the unit. No
            // construction, no save_dir, no prompt.
            return Ok((true, Resolution(AnyResolution::Standard(arc))));
        }

        // Non-unit path: replicate upstream's `#[cfg(not(feature = "nassau"))]`
        // unit construction NON-interactively, using the Python-provided
        // `save_dir` in place of upstream's `query::optional(...)` prompt. No
        // `query::*` (and hence no argv/stdin/process::exit) is reachable.
        let save_dir = save_dir.map(PathBuf::from);
        if let Some(p) = &save_dir {
            // Mirror `construct`'s save_dir-is-a-file pre-check.
            if p.exists() && !p.is_dir() {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "save_dir {p:?} exists and is not a directory"
                )));
            }
        }

        let algebra = arc.algebra();
        let module = FDModule::new(
            algebra,
            String::from("unit"),
            bivec::BiVec::from_vec(0, vec![1]),
        );
        let cc = RsFiniteChainComplex::ccdz(Arc::new(steenrod_module::erase(module)));
        let unit = ext::resolution::Resolution::new_with_save(Arc::new(cc), save_dir)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        Ok((false, Resolution(AnyResolution::Standard(Arc::new(unit)))))
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
    ///
    /// # `spec` forms
    ///
    /// `spec` accepts the same forms the upstream [`Config`] does via its `TryFrom` impls:
    ///  - a **string** such as `"S_2"` or `"S_2@milnor"` (the `@`-suffix selects the algebra
    ///    basis, defaulting to Milnor);
    ///  - a **2-tuple** `(spec, algebra)` where `spec` is either a module-name string OR a
    ///    module-JSON `dict` (as produced by [`parse_module_name`]/[`load_module_json`]), and
    ///    `algebra` is an `algebra.AlgebraType` enum value or the string `"adem"`/`"milnor"`.
    ///
    /// A bare `dict` (with no accompanying algebra) is NOT a valid `Config` upstream and is
    /// rejected with a `TypeError` asking the caller to pass `(dict, algebra)`.
    ///
    /// `algorithm` chooses the resolution *algorithm* (`"auto"`/`"nassau"`/`"standard"`), NOT the
    /// algebra basis (which is the `@`-suffix or the tuple's second element).
    fn py_to_config(spec: &Bound<'_, PyAny>) -> PyResult<Config> {
        // String form: "S_2", "S_2@milnor", ...
        if let Ok(s) = spec.extract::<String>() {
            return Config::try_from(s.as_str()).map_err(|e: anyhow::Error| {
                pyo3::exceptions::PyValueError::new_err(e.to_string())
            });
        }
        // 2-tuple form: (spec, algebra), where spec is a string or a module-JSON dict.
        if let Ok((module_obj, alg_obj)) =
            spec.extract::<(Bound<'_, PyAny>, Bound<'_, PyAny>)>()
        {
            let alg: ::algebra::AlgebraType =
                alg_obj.extract::<algebra_py::AlgebraTypeArg>()?.into();
            if let Ok(s) = module_obj.extract::<String>() {
                return Config::try_from((s.as_str(), alg)).map_err(|e: anyhow::Error| {
                    pyo3::exceptions::PyValueError::new_err(e.to_string())
                });
            }
            let module_json = algebra_py::py_to_json(&module_obj)?;
            return Config::try_from((module_json, alg))
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("{e:?}")));
        }
        Err(pyo3::exceptions::PyTypeError::new_err(
            "construct() spec must be a string, or a (spec, algebra) tuple where spec is a string \
             or a module-JSON dict and algebra is an AlgebraType or 'adem'/'milnor'",
        ))
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
            if b.s() < 0 {
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
        /// Construct a [`Resolution`] of the module `spec`, optionally backed by an on-disk save
        /// directory, without any interactive prompting (all I/O lives in the Python layer).
        ///
        /// This is the non-interactive primitive the pure-Python `query_module*` helpers call
        /// after prompting for the spec and (optionally) the save directory.
        ///
        /// `spec` accepts the same forms the upstream [`Config`] does (see [`py_to_config`]): a
        /// string such as `"S_2"`/`"S_2@milnor"`, or a 2-tuple `(spec, algebra)` where `spec` is a
        /// module-name string or a module-JSON `dict` and `algebra` is an `algebra.AlgebraType`
        /// or the string `"adem"`/`"milnor"`.
        ///
        /// `save_dir` is an optional on-disk save directory; `algorithm` is `None`/`"auto"`
        /// (try Nassau, fall back to the general algorithm), `"nassau"`, or `"standard"`, and
        /// selects the resolution *algorithm* (NOT the algebra basis).
        ///
        /// Error taxonomy matches [`build`]: bad spec/eligibility/unknown-algorithm -> `ValueError`,
        /// genuine internal/IO failures -> `RuntimeError`. Nothing panics across FFI.
        #[staticmethod]
        #[pyo3(signature = (spec, save_dir=None, algorithm=None))]
        pub fn construct(
            spec: &Bound<'_, PyAny>,
            save_dir: Option<String>,
            algorithm: Option<&str>,
        ) -> PyResult<Resolution> {
            let config = py_to_config(spec)?;
            build(config, save_dir.map(PathBuf::from), algorithm).map(Resolution)
        }

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
            require_nonneg!(b, "target bidegree");
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
            require_nonneg!(b, "target bidegree");
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
            require_nonneg!(b, "target bidegree");
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
            require_nonneg!(b, "target bidegree");
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
        #[getter]
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
        /// `s` is rejected with a `ValueError` rather than wrapping to a huge
        /// `usize`; a negative internal degree `t` is legitimate (modules with
        /// negative `min_degree`, e.g. `RP_{-k}`, have generators in negative
        /// `t`) and simply returns `false` when out of the computed range.
        pub fn has_computed_bidegree(&self, b: sseq_py::Bidegree) -> PyResult<bool> {
            require_nonneg!(b.0, "bidegree", s_only);
            Ok(dispatch!(&self.0, r => r.has_computed_bidegree(b.0)))
        }

        /// The number of generators of the resolution at bidegree `b` (the
        /// dimension of `Ext` there). Returns 0 for any uncomputed or
        /// out-of-range bidegree (including `t < min_degree`); raises
        /// `ValueError` for negative `s`. A negative internal degree `t` is
        /// legitimate (modules with negative `min_degree`, e.g. `RP_{-k}`, have
        /// generators in negative `t`).
        ///
        /// Both backends' modules' generator tables (`OnceBiVec`s) panic when
        /// indexed out of range, so this is guarded; see `num_gens_at`.
        pub fn number_of_gens_in_bidegree(&self, b: sseq_py::Bidegree) -> PyResult<usize> {
            require_nonneg!(b.0, "bidegree", s_only);
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

        /// The chain-complex differential `d_s: R_s -> R_{s-1}` out of
        /// homological degree `s`, as a bound `algebra_py.FreeModuleHomomorphismToFree`
        /// sharing its `Arc`.
        ///
        /// Like `module()`, only the standard backend is supported. Its
        /// differentials are `FreeModuleHomomorphism<FreeModule<SteenrodAlgebra>>`
        /// (free module -> free module over the `SteenrodAlgebra`), exactly the
        /// inner type the `FreeModuleHomomorphismToFree` pyclass wraps. Nassau's
        /// differentials are over the concrete `MilnorAlgebra` (a distinct,
        /// non-interconvertible type parameter), so the pyclass cannot represent
        /// them; `differential()` rejects the Nassau backend with a `ValueError`,
        /// matching `module()`.
        ///
        /// Raises `ValueError` for negative `s` or `s` beyond the resolved range
        /// (`>= next_homological_degree()`); indexing the differentials `OnceVec`
        /// there would otherwise panic.
        ///
        /// WARNING: the returned homomorphism is a *live shared view* of this
        /// resolution's internal differential (the same `Arc`), not a copy. Treat
        /// it as read-only; calling its mutating methods is memory-safe but may
        /// logically corrupt the resolution.
        pub fn differential(&self, s: i32) -> PyResult<algebra_py::FreeModuleHomomorphismToFree> {
            if s < 0 {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "homological degree s must be non-negative",
                ));
            }
            match &self.0 {
                AnyResolution::Standard(r) => {
                    if s >= r.next_homological_degree() {
                        return Err(pyo3::exceptions::PyValueError::new_err(format!(
                            "differential index s = {s} is beyond the resolved range (next \
                             homological degree is {}); compute_through_bidegree / \
                             compute_through_stem first",
                            r.next_homological_degree()
                        )));
                    }
                    Ok(algebra_py::FreeModuleHomomorphismToFree::from_arc(
                        r.differential(s),
                    ))
                }
                AnyResolution::Nassau(_) => Err(pyo3::exceptions::PyValueError::new_err(
                    "differential() is only available on the standard backend; Nassau resolves \
                     over the concrete MilnorAlgebra, whose FreeModuleHomomorphism the \
                     algebra_py.FreeModuleHomomorphismToFree pyclass (over the SteenrodAlgebra \
                     union) cannot represent. Construct the Resolution with algorithm='standard'.",
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
            require_nonneg!(source.0, "source bidegree");
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
            require_nonneg!(gen, "generator");
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

        /// The resolution's Steenrod algebra as a `SteenrodAlgebra`.
        ///
        /// The standard backend resolves over the union `SteenrodAlgebra`
        /// directly, so its shared `Arc` is wrapped without copying. The Nassau
        /// backend resolves over a concrete `MilnorAlgebra` (not the union); that
        /// is rebuilt into the equivalent `SteenrodAlgebra::Milnor` variant (same
        /// prime/profile, so identical basis indexing). See
        /// `SteenrodAlgebra::from_milnor`.
        pub fn algebra(&self) -> algebra_py::SteenrodAlgebra {
            match &self.0 {
                AnyResolution::Standard(r) => algebra_py::SteenrodAlgebra::from_arc(r.algebra()),
                AnyResolution::Nassau(r) => algebra_py::SteenrodAlgebra::from_milnor(&r.algebra()),
            }
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
    ///    `FreeModule<SteenrodAlgebra> = MuFreeModule<false, _>`. This is now
    ///    bound via a dedicated read-only `algebra_py.UnstableFreeModule`
    ///    pyclass (see `module()` below).
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
        /// Construct an unstable resolution, optionally backed by an on-disk save
        /// directory. The unstable analogue of `Resolution(spec)`; there is no
        /// `algorithm` argument because the unstable family is general-algorithm
        /// only.
        ///
        /// The first argument is either:
        ///  - a module-specification **string** (e.g. `"S_2"`, `"Cnu@adem"`),
        ///    resolved exactly as the [`construct_unstable`] pyfunction does
        ///    (Steenrod-algebra basis via an `@adem`/`@milnor` suffix, default
        ///    Milnor); or
        ///  - a [`ChainComplex`] to resolve directly (the by-complex constructor,
        ///    mirroring upstream `MuResolution::new_with_save(complex, save_dir)`).
        ///    This is how `examples/resolve_unstable.py` &c. resolve a
        ///    `ChainComplex.ccdz(module)` they built themselves.
        ///
        /// `save_dir` behaves exactly as in [`construct_unstable`]: an existing
        /// path that is not a directory is a `ValueError`. A bad spec string is a
        /// `ValueError`; an internal/IO construction failure is a `RuntimeError`.
        #[new]
        #[pyo3(signature = (spec, save_dir=None))]
        pub fn new(spec: &Bound<'_, PyAny>, save_dir: Option<String>) -> PyResult<Self> {
            // By-complex constructor: resolve a caller-supplied ChainComplex.
            if let Ok(cc) = spec.extract::<PyRef<'_, ChainComplex>>() {
                let save_dir = save_dir.map(PathBuf::from);
                if let Some(p) = &save_dir {
                    if p.exists() && !p.is_dir() {
                        return Err(pyo3::exceptions::PyValueError::new_err(format!(
                            "save_dir {p:?} exists and is not a directory"
                        )));
                    }
                }
                let res = RsUnstableResolution::new_with_save(cc.0.clone(), save_dir)
                    .map_err(|e: anyhow::Error| {
                        pyo3::exceptions::PyRuntimeError::new_err(e.to_string())
                    })?;
                return Ok(UnstableResolution(Arc::new(res)));
            }
            // By-spec constructor: parse a module-specification string.
            let spec: &str = spec.extract().map_err(|_| {
                pyo3::exceptions::PyTypeError::new_err(
                    "UnstableResolution() expects a module-spec string or a ChainComplex \
                     as its first argument",
                )
            })?;
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
            require_nonneg!(b, "target bidegree");
            self.0.compute_through_stem(b);
            Ok(())
        }

        /// Resolve through the given target bidegree (fixed `t`). Validates
        /// `s >= 0`/`t >= 0`, raising `ValueError` rather than panicking.
        pub fn compute_through_bidegree(&self, max: sseq_py::Bidegree) -> PyResult<()> {
            let b = max.0;
            require_nonneg!(b, "target bidegree");
            self.0.compute_through_bidegree(b);
            Ok(())
        }

        /// The prime as a plain `int`.
        #[getter]
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
            require_nonneg!(b.0, "bidegree");
            Ok(self.0.has_computed_bidegree(b.0))
        }

        /// The number of generators of the unstable resolution at bidegree `b`
        /// (the dimension of unstable `Ext` there). Returns 0 for any uncomputed
        /// or out-of-range bidegree; raises `ValueError` for negative `s`/`t`.
        /// Guarded like the stable `number_of_gens_in_bidegree`; see
        /// `num_gens_at`.
        pub fn number_of_gens_in_bidegree(&self, b: sseq_py::Bidegree) -> PyResult<usize> {
            require_nonneg!(b.0, "bidegree");
            Ok(self.num_gens_at(b.0))
        }

        /// The `s`-th free module of the unstable resolution, as a bound
        /// `algebra_py.UnstableFreeModule` sharing the underlying `Arc` (a live
        /// read-only view). This closes the prior `module()` deferral: the
        /// unstable resolution's modules are `MuFreeModule<true, SteenrodAlgebra>`,
        /// which the stable `algebra_py.FreeModule` pyclass cannot represent, so a
        /// dedicated `UnstableFreeModule` pyclass is returned instead.
        ///
        /// Raises `IndexError` for `s` outside the defined range
        /// `[0, next_homological_degree)` (the internal `modules` `OnceBiVec`
        /// would otherwise panic). Resolve the resolution further to define more
        /// modules.
        pub fn module(&self, s: i32) -> PyResult<algebra_py::UnstableFreeModule> {
            if s < 0 || s >= self.0.next_homological_degree() {
                return Err(pyo3::exceptions::PyIndexError::new_err(format!(
                    "no module at homological degree s = {s}; defined range is [0, {})",
                    self.0.next_homological_degree()
                )));
            }
            Ok(algebra_py::UnstableFreeModule::from_arc(self.0.module(s)))
        }

        pub fn graded_dimension_string(&self) -> String {
            self.0.graded_dimension_string()
        }

        /// The unstable `E_2`-page as a bound `sseq_py.Sseq` (the unstable
        /// analogue of `Resolution.to_sseq`, i.e. `to_sseq` on the unstable free
        /// chain complex). Panic-free over the resolved range: upstream only
        /// queries bidegrees yielded by `iter_stem`, all in range.
        pub fn to_sseq(&self) -> sseq_py::Sseq {
            let p = self.0.prime();
            sseq_py::Sseq::from_rust(self.0.to_sseq(), p)
        }

        /// Backwards-compatible alias for [`to_sseq`](Self::to_sseq).
        pub fn to_unstable_sseq(&self) -> sseq_py::Sseq {
            self.to_sseq()
        }

        /// The filtration-one products induced by the algebra operation
        /// `(op_deg, op_idx)`, as a bound `sseq_py.Product`. The unstable
        /// analogue of `Resolution.filtration_one_products`.
        ///
        /// Validates `op_deg >= 0` (`ValueError`) and range-checks `op_idx`
        /// against the algebra's operation dimension in degree `op_deg`
        /// (`IndexError`). Unstable resolutions are `U = true`, so upstream does
        /// bounds-check `op_idx`; we keep the explicit pre-check for a clean
        /// `IndexError` message, mirroring the stable binding. An uncomputed
        /// resolution (`next_homological_degree() == 0`) has no modules, so the
        /// empty product is returned directly rather than panicking on
        /// `module(0)`.
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
            let alg = self.0.algebra();
            alg.compute_basis(op_deg);
            let dim = alg.dimension(op_deg);
            if op_idx >= dim {
                return Err(pyo3::exceptions::PyIndexError::new_err(format!(
                    "op_idx {op_idx} out of range for op_deg {op_deg} (algebra dimension {dim})"
                )));
            }
            // An uncomputed resolution has no modules; upstream would panic
            // indexing module(0). Return the empty product directly.
            if self.0.next_homological_degree() == 0 {
                return Ok(sseq_py::Product(::sseq::Product {
                    b: RsBidegree::x_y(op_deg - 1, 1),
                    left: true,
                    matrices: ::once::MultiIndexed::new(),
                }));
            }
            let product = self.0.filtration_one_products(op_deg, op_idx);
            Ok(sseq_py::Product(product))
        }

        /// A string representation of `d(g)` for the generator `g = (s, t, idx)`.
        /// Raises `ValueError` if `g` is outside the computed range or `idx`
        /// exceeds the generator count there (upstream would otherwise panic).
        pub fn boundary_string(&self, g: sseq_py::BidegreeGenerator) -> PyResult<String> {
            let gen = g.0;
            require_nonneg!(gen, "generator");
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
        /// share the same prime. Only `shift.s` must be non-negative (a negative
        /// homological-degree shift is nonsensical and is rejected rather than
        /// risking a wrapped index). `shift.t` may be negative, e.g. for a map
        /// out of a stunted projective space `RP_{-k}` (whose source module has
        /// negative `min_degree`).
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
            if shift.0.s() < 0 {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "invalid shift {}: require s >= 0 (t may be negative, e.g. for a map out of RP_{{-k}})",
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
        ///  - `shift.s` non-negative (`shift.t` may be negative, e.g. a map out
        ///    of a stunted projective space `RP_{-k}`);
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
            if b.s() < 0 {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "invalid shift {b}: require s >= 0 (t may be negative, e.g. for a map out of RP_{{-k}})"
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
        #[getter]
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
            require_nonneg!(b, "target bidegree");
            self.check_extend_range(b.s(), |_s| b.t())?;
            self.0.extend(b);
            Ok(())
        }

        /// Extend the chain map through the stem `max` (defined on every `(s, t)`
        /// with `s <= max.s` and `t - s <= max.n`). Guards the touched range as
        /// `extend` does.
        pub fn extend_through_stem(&self, max: sseq_py::Bidegree) -> PyResult<()> {
            let b = max.0;
            require_nonneg!(b, "target bidegree");
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

        /// Manually define the chain map on the single bidegree `input`,
        /// sending the `k`-th source generator there to the `k`-th vector of
        /// `extra_images` (or to zero when `extra_images is None`). This is the
        /// hook for defining the map where the source complex is not exact (it
        /// is what the `extend*` family drives internally for every bidegree);
        /// follow it with `extend_all` to fill in the rest by exactness.
        ///
        /// Returns the half-open range `(start, end)` of internal degrees the
        /// step touched (the upstream `Range<i32>`) as a 2-tuple.
        ///
        /// Guards the upstream debug `assert!`s in `extend_step_raw` so they
        /// raise a clean `ValueError`/`RuntimeError` rather than panicking
        /// across FFI:
        ///  - `input` non-negative in both `s` and `t`;
        ///  - `input.s >= shift.s` (the map cannot lower homological degree);
        ///  - the source computed at `input`, and the target computed at
        ///    `input - shift` (`has_computed_bidegree`).
        /// As defence-in-depth the upstream call itself is wrapped in
        /// `catch_unwind`, mapping any residual panic (e.g. an `extra_images`
        /// row whose length does not match the target dimension, or a
        /// non-`None` `extra_images` on an already-defined degree) to a
        /// `RuntimeError`.
        #[pyo3(signature = (input, extra_images=None))]
        pub fn extend_step_raw(
            &self,
            input: sseq_py::Bidegree,
            extra_images: Option<Vec<PyRef<'_, fp_py::PyFpVector>>>,
        ) -> PyResult<(i32, i32)> {
            let b = input.0;
            require_nonneg!(b, "input bidegree");
            let shift = self.0.shift;
            if b.s() < shift.s() {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "input homological degree s = {} is below the homomorphism's shift \
                     s = {} (the map cannot lower homological degree)",
                    b.s(),
                    shift.s()
                )));
            }
            if !self.0.source.has_computed_bidegree(b) {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "source not computed at bidegree (s={}, t={}); resolve it there first",
                    b.s(),
                    b.t()
                )));
            }
            let output = b - shift;
            if !self.0.target.has_computed_bidegree(output) {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "target not computed at bidegree (s={}, t={}) = input - shift; resolve \
                     it there first",
                    output.s(),
                    output.t()
                )));
            }
            let extra: Option<Vec<::fp::vector::FpVector>> =
                extra_images.map(|v| v.iter().map(|x| x.as_rust().clone()).collect());
            use std::panic::{catch_unwind, AssertUnwindSafe};
            match catch_unwind(AssertUnwindSafe(|| self.0.extend_step_raw(b, extra))) {
                Ok(range) => Ok((range.start, range.end)),
                Err(payload) => {
                    let detail = payload
                        .downcast_ref::<&str>()
                        .map(|s| (*s).to_owned())
                        .or_else(|| payload.downcast_ref::<String>().cloned())
                        .unwrap_or_else(|| "unknown panic".to_owned());
                    Err(pyo3::exceptions::PyRuntimeError::new_err(format!(
                        "extend_step_raw panicked (likely an extra_images row whose length \
                         does not match the target dimension, or a non-None extra_images on \
                         an already-defined degree); underlying panic: {detail}"
                    )))
                }
            }
        }

        /// Manually define the chain map on the single bidegree `input`, the
        /// higher-level companion of `extend_step_raw`. Rather than taking the
        /// images already lifted into the target resolution, `extra_images` is
        /// an `fp.Matrix` whose `k`-th row is the image of the `k`-th source
        /// generator (with `d(g) = 0`) in the target's *augmentation*; upstream
        /// `extend_step` lifts those rows through the target's quasi-inverse for
        /// you before delegating to `extend_step_raw`. Pass `None` to define the
        /// step on zero generators. Follow with `extend_all` to fill in the rest
        /// by exactness.
        ///
        /// Returns the half-open range `(start, end)` of internal degrees the
        /// step touched (the upstream `Range<i32>`) as a 2-tuple.
        ///
        /// Guards the upstream debug `assert!`s the same way `extend_step_raw`
        /// does so they raise a clean `ValueError` rather than panicking across
        /// FFI:
        ///  - `input` non-negative in both `s` and `t`;
        ///  - `input.s >= shift.s` (the map cannot lower homological degree);
        ///  - the source computed at `input`, and the target computed at
        ///    `input - shift` (`has_computed_bidegree`).
        /// As defence-in-depth the upstream call is wrapped in `catch_unwind`,
        /// mapping any residual panic (e.g. an `extra_images` whose row count or
        /// width does not match the source/target dimensions) to a
        /// `RuntimeError`.
        #[pyo3(signature = (input, extra_images=None))]
        pub fn extend_step(
            &self,
            input: sseq_py::Bidegree,
            extra_images: Option<PyRef<'_, fp_py::PyMatrix>>,
        ) -> PyResult<(i32, i32)> {
            let b = input.0;
            require_nonneg!(b, "input bidegree");
            let shift = self.0.shift;
            if b.s() < shift.s() {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "input homological degree s = {} is below the homomorphism's shift \
                     s = {} (the map cannot lower homological degree)",
                    b.s(),
                    shift.s()
                )));
            }
            if !self.0.source.has_computed_bidegree(b) {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "source not computed at bidegree (s={}, t={}); resolve it there first",
                    b.s(),
                    b.t()
                )));
            }
            let output = b - shift;
            if !self.0.target.has_computed_bidegree(output) {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "target not computed at bidegree (s={}, t={}) = input - shift; resolve \
                     it there first",
                    output.s(),
                    output.t()
                )));
            }
            use std::panic::{catch_unwind, AssertUnwindSafe};
            match catch_unwind(AssertUnwindSafe(|| {
                self.0
                    .extend_step(b, extra_images.as_ref().map(|m| m.as_rust()))
            })) {
                Ok(range) => Ok((range.start, range.end)),
                Err(payload) => {
                    let detail = payload
                        .downcast_ref::<&str>()
                        .map(|s| (*s).to_owned())
                        .or_else(|| payload.downcast_ref::<String>().cloned())
                        .unwrap_or_else(|| "unknown panic".to_owned());
                    Err(pyo3::exceptions::PyRuntimeError::new_err(format!(
                        "extend_step panicked (likely an extra_images matrix whose row count or \
                         width does not match the source/target dimensions); underlying panic: \
                         {detail}"
                    )))
                }
            }
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
        ///
        /// `result` may be a bound `fp.FpVector` or `fp.FpSliceMut`.
        pub fn act(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coef: u32,
            g: sseq_py::BidegreeGenerator,
        ) -> PyResult<()> {
            let gen = g.0;
            require_nonneg!(gen, "generator");
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
            let expected = map.source().number_of_gens_in_degree(src_t);
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
            fp_py::with_target_slice_mut(py, result, |slice| {
                if slice.prime().as_u32() != p {
                    return Err(pyo3::exceptions::PyValueError::new_err(format!(
                        "result vector prime {} != homomorphism prime {p}",
                        slice.prime().as_u32()
                    )));
                }
                if slice.as_slice().len() != expected {
                    return Err(pyo3::exceptions::PyValueError::new_err(format!(
                        "result vector has length {} but the source has {expected} generator(s) at \
                         bidegree (s={src_s}, t={src_t})",
                        slice.as_slice().len()
                    )));
                }
                self.0.act(slice, coef, gen);
                Ok(())
            })
        }
    }

    /// The concrete `ExtAlgebra` bound here: a bigraded-algebra view of a
    /// *standard*-backend resolution. `ExtAlgebra<CC>` is generic over a
    /// `FreeChainComplex` `CC`; we monomorphise it at `ext::resolution::Resolution<CCC>`,
    /// which is exactly `ext::utils::QueryModuleResolution` (see
    /// `assert_query_module_is_standard`) — the inner type of
    /// [`AnyResolution::Standard`]. This is the only instantiation whose
    /// resolutions the bound `Resolution` pyclass can represent: the Nassau
    /// backend resolves over the concrete `MilnorAlgebra` and is rejected with a
    /// `ValueError` (mirroring `ResolutionHomomorphism`/`Resolution.module`).
    ///
    /// Bound here in the top-level `ext` module (NOT in the `algebra_py`
    /// submodule) on purpose: despite its name, `ExtAlgebra` is **not** a
    /// Steenrod-`Algebra`-trait type like `MilnorAlgebra`/`SteenrodAlgebra`/`Field`
    /// — it does not implement `Algebra` and has no `(degree, index)` basis. It
    /// is an Ext-product abstraction *built from resolutions*, so it depends on
    /// the `Resolution`/`AnyResolution` types defined in this module and belongs
    /// next to `Resolution`/`ResolutionHomomorphism`.
    type RsExtAlgebra = ext::ext_algebra::ExtAlgebra<ext::resolution::Resolution<CCC>>;

    /// $\Ext(M, k)$ as a bigraded module over the bigraded algebra $\Ext(k, k)$,
    /// backed by a (standard-backend) resolution of `M` and a resolution of the
    /// base field `k` (the "unit"). When `M == k` (same resolution passed twice)
    /// this is the algebra $\Ext(k, k)$ itself.
    ///
    /// Held behind an `Arc`: every method takes `&self` upstream (the
    /// per-generator product-map cache is an interior-mutable `DashMap`), so a
    /// `frozen` pyclass works directly, and `Arc<RsExtAlgebra>` derefs to
    /// `RsExtAlgebra` so the `&self` methods reach the upstream methods
    /// unchanged. Sharing the `Arc` (with the secondary layer and
    /// `SecondaryExtAlgebra::ext_algebra`) gives a stable identity and a shared
    /// product cache; the interior `DashMap` does its own locking, so concurrent
    /// `&self` access (under the GIL) through the shared `Arc` is sound.
    #[pyclass(frozen)]
    pub struct ExtAlgebra(Arc<RsExtAlgebra>);

    /// Number of generators of a resolution at bidegree `b`, returning 0 (never
    /// panicking) outside the computed range. Mirrors `Resolution::num_gens_at`
    /// (upstream `ExtAlgebra::dimension` is `resolution.number_of_gens_in_bidegree(b)`,
    /// whose two `OnceBiVec` indexings both panic out of range).
    fn ext_algebra_num_gens(r: &ext::resolution::Resolution<CCC>, b: RsBidegree) -> usize {
        if b.s() < 0 || b.t() < 0 || b.s() >= r.next_homological_degree() {
            return 0;
        }
        let m = r.module(b.s());
        if b.t() < m.min_degree() || b.t() > m.max_computed_degree() {
            0
        } else {
            m.number_of_gens_in_degree(b.t())
        }
    }

    /// Run an `ExtAlgebra` product computation under `catch_unwind`, translating
    /// any residual upstream panic into a `ValueError` (the established
    /// `catch_unwind`→`ValueError` backstop; cf. `catch_yoneda_panic` /
    /// `catch_unstable_construct_panic`).
    ///
    /// The `multiply`/`try_multiply`/`multiply_into` family is pre-checked
    /// exhaustively below (operand bidegrees resolved, vector lengths matching
    /// the generator counts, primes matching, target degree not overflowing
    /// `i32`, the unit's `(0,0)` augmentation 1-dimensional). This wrapper exists
    /// only as a defence-in-depth net for any internal `assert!`/index in the
    /// `ResolutionHomomorphism::from_class` + `extend_all` + `hom_k` plumbing that
    /// `ExtAlgebra::multiply_into` drives and that the pre-checks do not reach.
    ///
    /// `AssertUnwindSafe` over `&self`: a panic mid-product can leave a partially
    /// built map in the interior `DashMap` cache, but the cache is rebuilt/
    /// re-extended idempotently on the next call (a later observer sees a valid,
    /// possibly-unextended map, never a broken invariant), matching the shared-
    /// state precedent of `catch_yoneda_panic`.
    fn catch_ext_algebra_panic<T, F: FnOnce() -> T>(f: F) -> PyResult<T> {
        use std::panic::{catch_unwind, AssertUnwindSafe};
        match catch_unwind(AssertUnwindSafe(f)) {
            Ok(v) => Ok(v),
            Err(payload) => {
                let detail = payload
                    .downcast_ref::<&str>()
                    .map(|s| (*s).to_owned())
                    .or_else(|| payload.downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| "unknown panic".to_owned());
                Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "ExtAlgebra product computation panicked (an out-of-range or unresolved \
                     bidegree that slipped past the pre-checks); underlying panic: {detail}"
                )))
            }
        }
    }

    impl ExtAlgebra {
        /// Validate that `x` is a well-formed, computed element of $\Ext(M, k)$
        /// (the resolution side / left operand): non-negative bidegree, the
        /// bidegree resolved, the coordinate vector over the algebra's prime and
        /// of length equal to the number of generators there. Mirrors the
        /// element-validity requirements `ExtAlgebra::multiply_into` assumes (it
        /// indexes `class[g.idx()]` for each nonzero coordinate of `x`).
        fn check_res_element(&self, x: &::sseq::coordinates::BidegreeElement) -> PyResult<()> {
            Self::check_element(
                "resolution",
                self.0.resolution(),
                x,
                self.0.prime().as_u32(),
            )
        }

        /// As [`check_res_element`], but for `y`, an element of $\Ext(k, k)$ (the
        /// unit side / right operand): `multiply_into`/`try_multiply` index the
        /// product matrix's rows by the nonzero coordinates of `y`, so `y` must
        /// match the unit's generator count at its bidegree.
        fn check_unit_element(&self, y: &::sseq::coordinates::BidegreeElement) -> PyResult<()> {
            Self::check_element("unit", self.0.unit(), y, self.0.prime().as_u32())
        }

        fn check_element(
            which: &str,
            r: &ext::resolution::Resolution<CCC>,
            e: &::sseq::coordinates::BidegreeElement,
            prime: u32,
        ) -> PyResult<()> {
            let b = e.degree();
            if b.s() < 0 || b.t() < 0 {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "invalid {which} element bidegree {b}: require s >= 0 and t >= 0"
                )));
            }
            if e.vec().prime().as_u32() != prime {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "{which} element is over prime {} but the ExtAlgebra is over prime {prime}",
                    e.vec().prime().as_u32()
                )));
            }
            if !r.has_computed_bidegree(b) {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "{which} not computed at the element's bidegree (s={}, t={}); resolve it \
                     there first (compute_through_stem / compute_through_bidegree)",
                    b.s(),
                    b.t()
                )));
            }
            let dim = ext_algebra_num_gens(r, b);
            if e.vec().len() != dim {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "{which} element has {} coordinate(s) but there are {dim} generator(s) at \
                     bidegree (s={}, t={})",
                    e.vec().len(),
                    b.s(),
                    b.t()
                )));
            }
            Ok(())
        }

        /// Pre-check the requirement `ExtAlgebra::multiply_into` inherits from
        /// `ResolutionHomomorphism::from_class`: when the left operand is
        /// nonzero, a per-generator product map is built, which maps the class
        /// through the unit's augmentation at `(0, 0)` and requires that
        /// augmentation to be computed and 1-dimensional (the unit/sphere case).
        /// Mirrors the `from_class` guard at the bound `ResolutionHomomorphism`.
        fn check_unit_augmentation(&self) -> PyResult<()> {
            let u = self.0.unit();
            let zero = RsBidegree::s_t(0, 0);
            if !u.has_computed_bidegree(zero) {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "unit resolution not computed at bidegree (0, 0); resolve it through (0, 0) \
                     first",
                ));
            }
            let aug_dim = u.target().module(0).dimension(0);
            if aug_dim != 1 {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "products require the unit resolution's augmentation to be 1-dimensional in \
                     degree 0 (a unit/sphere resolution); got dimension {aug_dim}"
                )));
            }
            Ok(())
        }

        /// Share this algebra's `Arc<RsExtAlgebra>` (a cheap `Arc::clone`, not a
        /// rebuild): the SAME `RsExtAlgebra` instance — including its
        /// interior-mutable per-generator product-map `DashMap` cache — is
        /// shared. Used to hand the user's actual `ExtAlgebra` `Arc` to
        /// `SecondaryExtAlgebra::new` (which needs `Arc<ExtAlgebra<CC>>`), so the
        /// secondary layer and the `ExtAlgebra` share one instance (ptr-identity,
        /// shared product cache).
        pub(crate) fn inner_arc(&self) -> Arc<RsExtAlgebra> {
            Arc::clone(&self.0)
        }

        /// Wrap a shared `Arc<RsExtAlgebra>` into the bound `ExtAlgebra` pyclass
        /// (a cheap `Arc::clone` of the same instance). Used by
        /// `SecondaryExtAlgebra::ext_algebra` to return the SAME `ExtAlgebra`
        /// instance it was built on (stable identity, shared product cache).
        pub(crate) fn from_arc(alg: &Arc<RsExtAlgebra>) -> ExtAlgebra {
            ExtAlgebra(Arc::clone(alg))
        }

        /// Reject the addition `a + b` overflowing `i32` (the product lands at
        /// `x.degree() + y.degree()`, whose coordinates index modules/`FpVector`s).
        fn checked_target(a: RsBidegree, b: RsBidegree) -> PyResult<()> {
            if a.s().checked_add(b.s()).is_none() || a.t().checked_add(b.t()).is_none() {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "target bidegree {a} + {b} overflows i32"
                )));
            }
            Ok(())
        }
    }

    #[pymethods]
    impl ExtAlgebra {
        /// Build an `ExtAlgebra` from an explicit `(resolution, unit)` pair: a
        /// resolution of `M` (products land in its Ext) and a resolution of the
        /// base field `k`. Pass the *same* `Resolution` twice for the algebra
        /// $\Ext(k, k)$ itself.
        ///
        /// Both must be standard-backend (a Nassau-backed `Resolution` raises
        /// `ValueError`, as for `ResolutionHomomorphism`) and over the same prime
        /// (upstream `ExtAlgebra::new` `assert_eq!`s the two primes; we pre-check
        /// and raise `ValueError` instead of panicking).
        ///
        /// The upstream `ExtAlgebra::from_resolution` single-argument constructor
        /// is intentionally **not** bound: it derives the unit via
        /// `ext::utils::get_unit`, which prompts on stdin for a save directory
        /// when `M != k` — interactive I/O that may not cross the FFI boundary
        /// (the project invariant; cf. the bound non-interactive `get_unit`
        /// pyfunction). Build the unit with `ext.get_unit(resolution)` (or
        /// `ext.construct`) in Python and pass it here.
        #[new]
        pub fn new(resolution: &Resolution, unit: &Resolution) -> PyResult<Self> {
            let r = ResolutionHomomorphism::standard_arc(resolution, "resolution")?;
            let u = ResolutionHomomorphism::standard_arc(unit, "unit")?;
            if r.prime().as_u32() != u.prime().as_u32() {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "resolution and unit are over different primes ({} != {})",
                    r.prime().as_u32(),
                    u.prime().as_u32()
                )));
            }
            Ok(ExtAlgebra(Arc::new(RsExtAlgebra::new(r, u))))
        }

        /// Build an `ExtAlgebra` for resolution-*intrinsic* operations that do
        /// not involve products (notably the secondary `d2` differential), using
        /// the resolution itself in place of a unit — upstream
        /// `ExtAlgebra::without_unit(resolution)` = `new(resolution, resolution)`,
        /// so `is_unit()` is `True`.
        ///
        /// This avoids the unit-resolution setup that `from_resolution` performs
        /// (and any associated prompt). The product methods (`multiply` etc.) are
        /// only meaningful here when `M == k`; for products with `M != k`, build
        /// with `ExtAlgebra(resolution, unit)` instead. This is the constructor
        /// the secondary (`d2`) layer uses to build an `ExtAlgebra` without a
        /// unit (`SecondaryExtAlgebra`).
        ///
        /// Standard-backend only (a Nassau-backed `Resolution` raises
        /// `ValueError`, as for `ExtAlgebra(...)`).
        #[staticmethod]
        pub fn without_unit(resolution: &Resolution) -> PyResult<Self> {
            let r = ResolutionHomomorphism::standard_arc(resolution, "resolution")?;
            Ok(ExtAlgebra(Arc::new(RsExtAlgebra::without_unit(r))))
        }

        /// The prime as a plain `int`.
        #[getter]
        pub fn prime(&self) -> u32 {
            self.0.prime().as_u32()
        }

        /// Whether the resolution already resolves the unit (i.e. `M == k`, the
        /// resolution and unit share the same `Arc`).
        pub fn is_unit(&self) -> bool {
            self.0.is_unit()
        }

        /// The resolution of `M` (shares the underlying `Arc`).
        pub fn resolution(&self) -> Resolution {
            Resolution(AnyResolution::Standard(Arc::clone(self.0.resolution())))
        }

        /// The resolution of the unit `k` (shares the underlying `Arc`).
        pub fn unit(&self) -> Resolution {
            Resolution(AnyResolution::Standard(Arc::clone(self.0.unit())))
        }

        /// Ensure both the resolution and the unit are computed through the given
        /// stem. Negative `s`/`t` is rejected with `ValueError` (the resolve loop
        /// panics on a negative target; cf. `Resolution.compute_through_stem`).
        pub fn compute_through_stem(&self, max: sseq_py::Bidegree) -> PyResult<()> {
            let b = max.0;
            require_nonneg!(b, "target bidegree");
            self.0.compute_through_stem(b);
            Ok(())
        }

        /// Ensure both the resolution and the unit are computed through the given
        /// bidegree. Negative `s`/`t` is rejected with `ValueError`.
        pub fn compute_through_bidegree(&self, max: sseq_py::Bidegree) -> PyResult<()> {
            let b = max.0;
            require_nonneg!(b, "target bidegree");
            self.0.compute_through_bidegree(b);
            Ok(())
        }

        /// The dimension of $\Ext^{s,t}(M, k)$ at bidegree `b` (the number of
        /// generators of the resolution there). Returns 0 for any uncomputed or
        /// out-of-range bidegree; raises `ValueError` for negative `s`/`t`.
        /// (Upstream `ExtAlgebra::dimension` indexes two `OnceBiVec`s and panics
        /// out of range; this is the guarded analogue.)
        pub fn dimension(&self, b: sseq_py::Bidegree) -> PyResult<usize> {
            require_nonneg!(b.0, "bidegree");
            Ok(ext_algebra_num_gens(self.0.resolution(), b.0))
        }

        /// The dimension of $\Ext^{s,t}(k, k)$ at bidegree `b` (the
        /// multiplicand/"scalar" side). Guarded as [`dimension`].
        pub fn unit_dimension(&self, b: sseq_py::Bidegree) -> PyResult<usize> {
            require_nonneg!(b.0, "bidegree");
            Ok(ext_algebra_num_gens(self.0.unit(), b.0))
        }

        /// The basis generators of $\Ext(M, k)$ at bidegree `b`, as a list of
        /// `sseq_py.BidegreeGenerator`. Empty for an uncomputed/out-of-range
        /// bidegree; raises `ValueError` for negative `s`/`t`.
        pub fn basis(&self, b: sseq_py::Bidegree) -> PyResult<Vec<sseq_py::BidegreeGenerator>> {
            let n = self.dimension(b)?;
            Ok((0..n)
                .map(|i| {
                    sseq_py::BidegreeGenerator(::sseq::coordinates::BidegreeGenerator::new(b.0, i))
                })
                .collect())
        }

        /// The basis generators of $\Ext(k, k)$ at bidegree `b`. Guarded as
        /// [`basis`].
        pub fn unit_basis(
            &self,
            b: sseq_py::Bidegree,
        ) -> PyResult<Vec<sseq_py::BidegreeGenerator>> {
            let n = self.unit_dimension(b)?;
            Ok((0..n)
                .map(|i| {
                    sseq_py::BidegreeGenerator(::sseq::coordinates::BidegreeGenerator::new(b.0, i))
                })
                .collect())
        }

        /// A single generator of $\Ext(M, k)$ as a class
        /// (`sseq_py.BidegreeElement`). Raises `ValueError` for negative `s`/`t`
        /// and `IndexError` if `g.idx()` is out of range at its bidegree (upstream
        /// `ExtAlgebra::generator` `assert!`s `dimension > idx`).
        pub fn generator(
            &self,
            g: sseq_py::BidegreeGenerator,
        ) -> PyResult<sseq_py::BidegreeElement> {
            let gen = g.0;
            require_nonneg!(gen, "generator");
            let dim = ext_algebra_num_gens(self.0.resolution(), gen.degree());
            if gen.idx() >= dim {
                return Err(pyo3::exceptions::PyIndexError::new_err(format!(
                    "generator index {} out of range at bidegree {} ({dim} generator(s), or the \
                     bidegree is uncomputed)",
                    gen.idx(),
                    gen.degree()
                )));
            }
            Ok(sseq_py::BidegreeElement(
                gen.into_element(self.0.prime(), dim),
            ))
        }

        /// A class in $\Ext(M, k)$ from its coordinates in the generator basis at
        /// bidegree `b`. Raises `ValueError` for negative `s`/`t`, an uncomputed
        /// `b`, or a `coords` length not matching the dimension there (upstream
        /// `ExtAlgebra::element` `assert_eq!`s the two).
        pub fn element(
            &self,
            b: sseq_py::Bidegree,
            coords: Vec<u32>,
        ) -> PyResult<sseq_py::BidegreeElement> {
            Self::make_element(
                self.0.resolution(),
                self.0.prime(),
                b.0,
                coords,
                "resolution",
            )
        }

        /// A class in $\Ext(k, k)$ (the unit side) from its coordinates. Guarded
        /// as [`element`].
        pub fn unit_element(
            &self,
            b: sseq_py::Bidegree,
            coords: Vec<u32>,
        ) -> PyResult<sseq_py::BidegreeElement> {
            Self::make_element(self.0.unit(), self.0.prime(), b.0, coords, "unit")
        }

        /// Left-multiplication by the class `x` (in $\Ext(M, k)$) applied to
        /// every basis generator of $\Ext(k, k)$ at bidegree `b`: a matrix with
        /// one row per generator of $\Ext(k, k)$ at `b`, row `j` being the product
        /// `x · g_j` in the generator basis of $\Ext(M, k)$ at `b + x.degree()`.
        ///
        /// Returns `None` when the product is out of the computed range (`b` or
        /// `b + x.degree()` unresolved), so an uncomputed product is never
        /// mistaken for a zero one (a computed-but-empty bidegree yields a valid
        /// zero-dimension matrix — an empty/`[]`-rows list — not `None`).
        ///
        /// `x` must be a valid, computed element (see the operand guards);
        /// negative `b`, a degree-sum overflow, or a non-unit augmentation when
        /// `x` is nonzero raise `ValueError`.
        pub fn multiply_into(
            &self,
            x: &sseq_py::BidegreeElement,
            b: sseq_py::Bidegree,
        ) -> PyResult<Option<Vec<Vec<u32>>>> {
            self.check_res_element(&x.0)?;
            let bb = b.0;
            require_nonneg!(bb, "bidegree");
            Self::checked_target(bb, x.0.degree())?;
            if !x.0.vec().is_zero() {
                self.check_unit_augmentation()?;
            }
            let matrix = catch_ext_algebra_panic(|| self.0.multiply_into(&x.0, bb))?;
            Ok(matrix.map(|m| {
                // Upstream `Matrix::to_vec` panics for a zero-column matrix
                // (chunking by a zero stride); a computed-but-empty target
                // bidegree is `m.rows()` empty rows. Mirrors `fp_py` PyMatrix.
                if m.columns() == 0 {
                    vec![Vec::new(); m.rows()]
                } else {
                    m.to_vec()
                }
            }))
        }

        /// The product `x · y` if it lies in the computed range, else `None`.
        /// `x ∈ Ext(M, k)`, `y ∈ Ext(k, k)`; the result lies in bidegree
        /// `x.degree() + y.degree()`. Both operands are validated (see the operand
        /// guards) — a malformed/uncomputed operand raises `ValueError` rather
        /// than returning a misleading `None`.
        pub fn try_multiply(
            &self,
            x: &sseq_py::BidegreeElement,
            y: &sseq_py::BidegreeElement,
        ) -> PyResult<Option<sseq_py::BidegreeElement>> {
            self.check_res_element(&x.0)?;
            self.check_unit_element(&y.0)?;
            Self::checked_target(x.0.degree(), y.0.degree())?;
            if !x.0.vec().is_zero() {
                self.check_unit_augmentation()?;
            }
            let res = catch_ext_algebra_panic(|| self.0.try_multiply(&x.0, &y.0))?;
            Ok(res.map(sseq_py::BidegreeElement))
        }

        /// The product `x · y`, where `x ∈ Ext(M, k)` and `y ∈ Ext(k, k)`. The
        /// result lies in bidegree `x.degree() + y.degree()`.
        ///
        /// Upstream `ExtAlgebra::multiply` `.expect()`s the product to be in the
        /// computed range; this binding instead raises `ValueError` (never
        /// panicking) when it is out of range — compute further or use
        /// [`try_multiply`]. Operands are validated as for [`try_multiply`].
        pub fn multiply(
            &self,
            x: &sseq_py::BidegreeElement,
            y: &sseq_py::BidegreeElement,
        ) -> PyResult<sseq_py::BidegreeElement> {
            self.check_res_element(&x.0)?;
            self.check_unit_element(&y.0)?;
            Self::checked_target(x.0.degree(), y.0.degree())?;
            if !x.0.vec().is_zero() {
                self.check_unit_augmentation()?;
            }
            match catch_ext_algebra_panic(|| self.0.try_multiply(&x.0, &y.0))? {
                Some(e) => Ok(sseq_py::BidegreeElement(e)),
                None => Err(pyo3::exceptions::PyValueError::new_err(
                    "product is out of the computed range; resolve the ExtAlgebra further \
                     (compute_through_stem / compute_through_bidegree) or use try_multiply",
                )),
            }
        }
    }

    impl ExtAlgebra {
        /// Shared helper for `element`/`unit_element`: build a class over `r`'s
        /// generator basis at `b` from `coords`, validating non-negativity, that
        /// `b` is resolved, and that `coords` matches the dimension there.
        fn make_element(
            r: &ext::resolution::Resolution<CCC>,
            prime: ::fp::prime::ValidPrime,
            b: RsBidegree,
            coords: Vec<u32>,
            which: &str,
        ) -> PyResult<sseq_py::BidegreeElement> {
            require_nonneg!(b, "bidegree");
            if !r.has_computed_bidegree(b) {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "{which} not computed at bidegree (s={}, t={}); resolve it there first",
                    b.s(),
                    b.t()
                )));
            }
            let dim = ext_algebra_num_gens(r, b);
            if coords.len() != dim {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "coords has length {} but there are {dim} generator(s) at bidegree \
                     (s={}, t={})",
                    coords.len(),
                    b.s(),
                    b.t()
                )));
            }
            Ok(sseq_py::BidegreeElement(
                ::sseq::coordinates::BidegreeElement::new(
                    b,
                    ::fp::vector::FpVector::from_slice(prime, &coords),
                ),
            ))
        }
    }

    /// The concrete *unstable* (`U = true`) resolution homomorphism bound here:
    /// a chain map between two `UnstableResolution`s of the default complex
    /// `CCC`. Both source and target are
    /// `ext::resolution::UnstableResolution<CCC> = MuResolution<true, CCC>` (the
    /// single concrete type the bound `UnstableResolution` pyclass holds — the
    /// unstable family is general-algorithm only, so there is no
    /// `AnyResolution`-style backend union and no Nassau rejection needed).
    ///
    /// Its `get_map(s)` returns an *unstable* free → free homomorphism
    /// `Arc<MuFreeModuleHomomorphism<true, MuFreeModule<true, SteenrodAlgebra>>>`,
    /// represented by the bound `algebra_py.UnstableFreeModuleHomomorphism`
    /// pyclass (NOT the stable `FreeModuleHomomorphismToFree`, whose inner type
    /// is the `U = false` variant).
    type RsUnstableResHom =
        RsUnstableResolutionHomomorphism<RsUnstableResolution, RsUnstableResolution>;

    /// A lifted chain map between two unstable resolutions — the unstable
    /// analogue of [`ResolutionHomomorphism`]. Mirrors the stable binding member
    /// for member (`new`/`from_class`/`get_map`/`extend*`/`act`/`name`/`shift`/
    /// `source`/`target`/`prime`/`algebra`/`next_homological_degree`/`save_dir`)
    /// with the unstable module/dimension where the math differs.
    ///
    /// Held behind an `Arc` like the stable one; every method takes `&self` and
    /// the internal `maps` table is interior-mutable (`OnceBiVec`).
    ///
    /// Note: `get_map(s)` returns an `algebra_py.UnstableFreeModuleHomomorphism`
    /// that shares the internal `Arc` of this homomorphism's `s`-th map — a live
    /// read-only view (its mutators are not bound, so it cannot corrupt the
    /// chain map).
    #[pyclass(frozen)]
    pub struct UnstableResolutionHomomorphism(Arc<RsUnstableResHom>);

    impl UnstableResolutionHomomorphism {
        /// Number of generators of the target resolution at bidegree `b`,
        /// returning 0 (never panicking) outside the computed range. Mirrors
        /// `ResolutionHomomorphism::target_num_gens`.
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

        /// Pre-flight guard for the `extend*` family, identical in shape to
        /// `ResolutionHomomorphism::check_extend_range`: it verifies the whole
        /// touched grid `{(s, t) : shift_s <= s <= max_s, min_t <= t <= t_max(s)}`
        /// is resolved in *both* the source and the shifted target, raising
        /// `ValueError` rather than letting an upstream `assert!`
        /// (`has_computed_bidegree` in `extend_step_raw`) panic across FFI.
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
    impl UnstableResolutionHomomorphism {
        /// Construct an (initially empty) unstable resolution homomorphism
        /// `source -> target` of the given bidegree `shift` and `name`. Mirrors
        /// `ResolutionHomomorphism.__init__`: both resolutions must share the
        /// same prime, and `shift` must be non-negative in both `s` and `t`.
        #[new]
        pub fn new(
            name: String,
            source: &UnstableResolution,
            target: &UnstableResolution,
            shift: sseq_py::Bidegree,
        ) -> PyResult<Self> {
            let s = Arc::clone(&source.0);
            let t = Arc::clone(&target.0);
            if s.prime().as_u32() != t.prime().as_u32() {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "source and target resolutions are over different primes ({} != {})",
                    s.prime().as_u32(),
                    t.prime().as_u32()
                )));
            }
            require_nonneg!(shift.0, "shift");
            Ok(UnstableResolutionHomomorphism(Arc::new(
                RsUnstableResHom::new(name, s, t, shift.0),
            )))
        }

        /// Build the unstable resolution homomorphism representing
        /// multiplication by the unstable `Ext` class `class` at bidegree
        /// `shift` in `source`. Mirrors `ResolutionHomomorphism.from_class` and
        /// ports all of its guards (same prime; `shift` non-negative; `source`
        /// computed at `shift` with `len(class)` matching its generator count;
        /// `target` computed at `(0, 0)` with a 1-dimensional augmentation).
        #[staticmethod]
        pub fn from_class(
            name: String,
            source: &UnstableResolution,
            target: &UnstableResolution,
            shift: sseq_py::Bidegree,
            class: Vec<u32>,
        ) -> PyResult<Self> {
            let s = Arc::clone(&source.0);
            let t = Arc::clone(&target.0);
            if s.prime().as_u32() != t.prime().as_u32() {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "source and target resolutions are over different primes ({} != {})",
                    s.prime().as_u32(),
                    t.prime().as_u32()
                )));
            }
            let b = shift.0;
            require_nonneg!(b, "shift");
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
            Ok(UnstableResolutionHomomorphism(Arc::new(
                RsUnstableResHom::from_class(name, s, t, b, &class),
            )))
        }

        /// The homomorphism's name (used in tracing/logging).
        pub fn name(&self) -> String {
            self.0.name().to_string()
        }

        /// The Steenrod algebra the (source) resolution is built over (an
        /// unstable-flagged `SteenrodAlgebra`).
        pub fn algebra(&self) -> algebra_py::SteenrodAlgebra {
            algebra_py::SteenrodAlgebra::from_arc(self.0.algebra())
        }

        /// The prime as a plain `int`.
        #[getter]
        pub fn prime(&self) -> u32 {
            self.0.source.prime().as_u32()
        }

        /// The shift bidegree of the homomorphism.
        pub fn shift(&self) -> sseq_py::Bidegree {
            sseq_py::Bidegree(self.0.shift)
        }

        /// The source unstable resolution (shares the underlying `Arc`).
        pub fn source(&self) -> UnstableResolution {
            UnstableResolution(Arc::clone(&self.0.source))
        }

        /// The target unstable resolution (shares the underlying `Arc`).
        pub fn target(&self) -> UnstableResolution {
            UnstableResolution(Arc::clone(&self.0.target))
        }

        /// The first homological degree `s` at which the chain map is not yet
        /// defined (the length of the internal `maps` table).
        pub fn next_homological_degree(&self) -> i32 {
            self.0.next_homological_degree()
        }

        /// The directory used to persist the chain map, or `None` if held purely
        /// in memory (the default).
        pub fn save_dir(&self) -> Option<String> {
            self.0.save_dir().read().map(|p| p.display().to_string())
        }

        /// The chain map on the `s`-th source module, as a bound
        /// `algebra_py.UnstableFreeModuleHomomorphism` sharing its `Arc`.
        ///
        /// Raises `IndexError` for `s` outside the defined range
        /// `[shift.s, next_homological_degree)`. Extend the homomorphism first
        /// to define more maps.
        ///
        /// WARNING: the returned homomorphism is a *live shared view* of this
        /// resolution homomorphism's internal map (the same `Arc`); treat it as
        /// read-only.
        pub fn get_map(&self, s: i32) -> PyResult<algebra_py::UnstableFreeModuleHomomorphism> {
            if s < self.0.shift.s() || s >= self.0.next_homological_degree() {
                return Err(pyo3::exceptions::PyIndexError::new_err(format!(
                    "no map defined at homological degree s = {s}; defined range is [{}, {})",
                    self.0.shift.s(),
                    self.0.next_homological_degree()
                )));
            }
            Ok(algebra_py::UnstableFreeModuleHomomorphism::from_arc(
                self.0.get_map(s),
            ))
        }

        /// Extend the chain map so it is defined on every bidegree `(s, t)` with
        /// `s <= max.s` and `t <= max.t`, lifting by exactness. Guards the
        /// touched range as the stable `extend` does.
        pub fn extend(&self, max: sseq_py::Bidegree) -> PyResult<()> {
            let b = max.0;
            require_nonneg!(b, "target bidegree");
            self.check_extend_range(b.s(), |_s| b.t())?;
            self.0.extend(b);
            Ok(())
        }

        /// Extend the chain map through the stem `max`. Guards the touched range
        /// as `extend` does.
        pub fn extend_through_stem(&self, max: sseq_py::Bidegree) -> PyResult<()> {
            let b = max.0;
            require_nonneg!(b, "target bidegree");
            let n = b.n();
            self.check_extend_range(b.s(), |s| n + s)?;
            self.0.extend_through_stem(b);
            Ok(())
        }

        /// Extend the chain map as far as the source and target are already
        /// resolved. Guards the degenerate empty-range case (mirrors the stable
        /// `extend_all`).
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

        /// Manually define the chain map on the single bidegree `input`,
        /// sending the `k`-th source generator there to the `k`-th vector of
        /// `extra_images` (or to zero when `extra_images is None`). The
        /// unstable analogue of `ResolutionHomomorphism.extend_step_raw`,
        /// porting all of its guards (this is the hook
        /// `examples/unstable_suspension.py` uses to seed the suspension map
        /// before `extend_all`).
        ///
        /// Returns the half-open range `(start, end)` of internal degrees the
        /// step touched (the upstream `Range<i32>`) as a 2-tuple.
        ///
        /// Guards the upstream debug `assert!`s in `extend_step_raw` so they
        /// raise a clean `ValueError`/`RuntimeError` rather than panicking
        /// across FFI:
        ///  - `input` non-negative in both `s` and `t`;
        ///  - `input.s >= shift.s` (the map cannot lower homological degree);
        ///  - the source computed at `input`, and the target computed at
        ///    `input - shift` (`has_computed_bidegree`).
        /// As defence-in-depth the upstream call itself is wrapped in
        /// `catch_unwind`, mapping any residual panic (e.g. an `extra_images`
        /// row whose length does not match the target dimension, or a
        /// non-`None` `extra_images` on an already-defined degree) to a
        /// `RuntimeError`.
        #[pyo3(signature = (input, extra_images=None))]
        pub fn extend_step_raw(
            &self,
            input: sseq_py::Bidegree,
            extra_images: Option<Vec<PyRef<'_, fp_py::PyFpVector>>>,
        ) -> PyResult<(i32, i32)> {
            let b = input.0;
            require_nonneg!(b, "input bidegree");
            let shift = self.0.shift;
            if b.s() < shift.s() {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "input homological degree s = {} is below the homomorphism's shift \
                     s = {} (the map cannot lower homological degree)",
                    b.s(),
                    shift.s()
                )));
            }
            if !self.0.source.has_computed_bidegree(b) {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "source not computed at bidegree (s={}, t={}); resolve it there first",
                    b.s(),
                    b.t()
                )));
            }
            let output = b - shift;
            if !self.0.target.has_computed_bidegree(output) {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "target not computed at bidegree (s={}, t={}) = input - shift; resolve \
                     it there first",
                    output.s(),
                    output.t()
                )));
            }
            let extra: Option<Vec<::fp::vector::FpVector>> =
                extra_images.map(|v| v.iter().map(|x| x.as_rust().clone()).collect());
            use std::panic::{catch_unwind, AssertUnwindSafe};
            match catch_unwind(AssertUnwindSafe(|| self.0.extend_step_raw(b, extra))) {
                Ok(range) => Ok((range.start, range.end)),
                Err(payload) => {
                    let detail = payload
                        .downcast_ref::<&str>()
                        .map(|s| (*s).to_owned())
                        .or_else(|| payload.downcast_ref::<String>().cloned())
                        .unwrap_or_else(|| "unknown panic".to_owned());
                    Err(pyo3::exceptions::PyRuntimeError::new_err(format!(
                        "extend_step_raw panicked (likely an extra_images row whose length \
                         does not match the target dimension, or a non-None extra_images on \
                         an already-defined degree); underlying panic: {detail}"
                    )))
                }
            }
        }

        /// Apply the dual map `Hom(f, k)` to the target-resolution generator
        /// `g`, accumulating `coef` times the result into `result` (a bound
        /// `fp.FpVector`). Ports every guard from the stable
        /// `ResolutionHomomorphism.act`.
        ///
        /// Note on the unstable `op_idx`/`dimension_unstable` concern: upstream
        /// `act` calls `target_module.operation_generator_to_index(0, 0, g.t(),
        /// g.idx())` with operation degree AND index both fixed at 0 (the
        /// identity operation). `dimension_unstable(0, g.t())` is always >= 1, so
        /// `op_idx = 0` is always in range — the unstable-specific `op_idx`
        /// bound (live in `UnstableFreeModule.operation_generator_to_index`) is
        /// not a reachable footgun *here*; the relevant guard is that `g` is a
        /// valid target generator (its `idx` in range, the target computed at
        /// `g.degree()`), exactly as in the stable binding.
        pub fn act(
            &self,
            mut result: PyRefMut<'_, fp_py::PyFpVector>,
            coef: u32,
            g: sseq_py::BidegreeGenerator,
        ) -> PyResult<()> {
            let gen = g.0;
            require_nonneg!(gen, "generator");
            let shift = self.0.shift;
            let src_s = gen.s().checked_add(shift.s()).ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err("source s = g.s + shift.s overflows i32")
            })?;
            let src_t = gen.t().checked_add(shift.t()).ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err("source t = g.t + shift.t overflows i32")
            })?;
            let source_b = RsBidegree::s_t(src_s, src_t);
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
            if gen.s() >= self.0.target.next_homological_degree() {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "target not resolved at homological degree s = {} (g.s)",
                    gen.s()
                )));
            }
            // The target's module basis must be computed through g.t() so that
            // `operation_generator_to_index(0, 0, g.t(), g.idx())` does not index
            // an unbuilt `generator_to_index` row; `has_computed_bidegree`
            // ensures the module is resolved (and its basis built) there.
            //
            // NOTE: this guard intentionally OVER-guards relative to the stable
            // `ResolutionHomomorphism.act` (which omits it): `has_computed_bidegree`
            // additionally requires the target *differential* at g.degree(), which
            // `act` never uses. It is a conservative, redundant superset of the
            // `src_t >= map.next_degree()` guard above — the hom cannot be extended
            // past the target's computed range, so that guard already fires first
            // and this branch is unreachable from Python (see the act-guard tests).
            // Kept deliberately as a belt-and-braces check; do NOT "fix" it away or
            // propagate it to the stable analogue.
            if !self.0.target.has_computed_bidegree(gen.degree()) {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "target not computed at bidegree (s={}, t={}) = g.degree()",
                    gen.s(),
                    gen.t()
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
        #[getter]
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
            require_nonneg!(b, "max_source bidegree");
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

        /// Allocate the internal homotopy table so `homotopy(s)` is defined for
        /// every `s` up to *but excluding* `max_source_s`, *without* lifting any
        /// maps. This lets a caller populate a non-zero bottom homotopy manually
        /// before `extend`/`extend_all` (which otherwise default the bottom-most
        /// homotopy to zero) — the setup needed for secondary Massey products.
        ///
        /// Upstream `initialize_homotopies` builds, for each newly added `s` in
        /// `[defined_range().end, max_source_s)`, a `FreeModuleHomomorphism`
        /// from `left.source.module(s)` to `right.target.module(s + 1 - shift.s)`;
        /// indexing a not-yet-created module would panic. We pre-check (via
        /// `next_homological_degree`, as `extend_all` does) that both resolutions
        /// are resolved far enough and raise a clean `ValueError` otherwise. If
        /// `max_source_s` does not exceed the currently defined range this is a
        /// no-op (matching upstream `OnceBiVec::extend`).
        pub fn initialize_homotopies(&self, max_source_s: i32) -> PyResult<()> {
            let left = self.0.left();
            let right = self.0.right();
            let shift = self.0.shift();
            let start = self.0.defined_range().end;
            for s in start..max_source_s {
                if s >= left.source.next_homological_degree() {
                    return Err(pyo3::exceptions::PyValueError::new_err(format!(
                        "the left source resolution has no module at homological degree s = {s} \
                         (next homological degree {}); resolve it further before initializing \
                         the homotopy table up to max_source_s = {max_source_s}",
                        left.source.next_homological_degree()
                    )));
                }
                let tgt_s = s + 1 - shift.s();
                if tgt_s >= right.target.next_homological_degree() {
                    return Err(pyo3::exceptions::PyValueError::new_err(format!(
                        "the right target resolution has no module at homological degree s = \
                         {tgt_s} (= {s} + 1 - shift.s; next homological degree {}); resolve it \
                         further before initializing the homotopy table up to max_source_s = \
                         {max_source_s}",
                        right.target.next_homological_degree()
                    )));
                }
            }
            self.0.initialize_homotopies(max_source_s);
            Ok(())
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

        /// The prime as a plain `int` (mirrors the `prime` getter on the other
        /// secondary pyclasses).
        #[getter]
        pub fn prime(&self) -> u32 {
            self.0.prime().as_u32()
        }

        /// The $E_3$-page of the resolution (the $E_2$-page of the underlying
        /// resolution with the secondary $d_2$ differentials added), as a bound
        /// `sseq_py.Sseq`.
        ///
        /// Call `extend_all()` first so the secondary homotopies are populated;
        /// upstream `e3_page` indexes `homotopy(b.s() + 2)` for every computed
        /// bidegree, so we run it under `catch_unwind` (-> `ValueError`) rather
        /// than let an unpopulated `OnceBiVec` index cross the FFI boundary.
        #[getter]
        pub fn e3_page(&self) -> PyResult<sseq_py::Sseq> {
            let p = self.0.prime();
            let sseq = catch_secondary_compute_panic(|| self.0.e3_page())?;
            Ok(sseq_py::Sseq::from_rust(sseq, p))
        }

        /// The secondary homotopy at homological degree `s` (a bound
        /// `SecondaryHomotopy`), a live shared view of this resolution's internal
        /// homotopy datum (shares the `Arc`).
        ///
        /// Raises `IndexError` for `s` outside the populated range of the
        /// internal homotopy table (`[min_degree, len)` of `homotopies()`), which
        /// would otherwise panic on the `OnceBiVec` index. Call `extend_all`
        /// first.
        pub fn homotopy(&self, s: i32) -> PyResult<SecondaryHomotopy> {
            let homotopies = self.0.homotopies();
            if s < homotopies.min_degree() || s >= homotopies.len() {
                return Err(pyo3::exceptions::PyIndexError::new_err(format!(
                    "no secondary homotopy defined at homological degree s = {s}; defined range is \
                     [{}, {}) (extend the secondary resolution first)",
                    homotopies.min_degree(),
                    homotopies.len()
                )));
            }
            Ok(SecondaryHomotopy {
                res: Arc::clone(&self.0),
                s,
            })
        }
    }

    /// A single secondary homotopy `h_s` of a [`SecondaryResolution`] — the bound
    /// view of the upstream `ext::secondary::SecondaryHomotopy` held at
    /// homological degree `s`. Produced by [`SecondaryResolution.homotopy`];
    /// never constructed directly from Python.
    ///
    /// Holds the parent secondary resolution's `Arc` together with the
    /// homological degree `s`, so it stays a live shared view (the upstream
    /// `homotopy(s)` hands back a borrow into the resolution's interior-mutable
    /// `OnceBiVec`, which cannot be held across the FFI boundary; re-deriving it
    /// from the `Arc` + `s` on each access is the safe equivalent). `s` was range
    /// -checked at construction.
    #[pyclass(frozen)]
    pub struct SecondaryHomotopy {
        res: Arc<RsSecRes>,
        s: i32,
    }

    #[pymethods]
    impl SecondaryHomotopy {
        /// The homological degree `s` this homotopy sits at.
        #[getter]
        pub fn s(&self) -> i32 {
            self.s
        }

        /// The homotopy's underlying free-module map (`homotopies` field of the
        /// upstream `SecondaryHomotopy`), as a bound `SecondaryHomotopyMap`
        /// exposing `hom_k`. A live shared view (shares the parent `Arc` + `s`).
        #[getter]
        pub fn homotopies(&self) -> SecondaryHomotopyMap {
            SecondaryHomotopyMap {
                res: Arc::clone(&self.res),
                s: self.s,
            }
        }
    }

    /// The free-module homomorphism underlying a [`SecondaryHomotopy`] (the
    /// `homotopies` field of the upstream `SecondaryHomotopy`). It exposes only
    /// `hom_k` (the dual map on generators), which is what the secondary
    /// examples read off a `d_2`.
    ///
    /// Like [`SecondaryHomotopy`], it holds the parent secondary resolution's
    /// `Arc` and the homological degree `s` rather than a borrow: the upstream
    /// map lives by value inside the resolution's interior-mutable `OnceBiVec`,
    /// so it is re-derived from the `Arc` + `s` on each access. `s` was range
    /// -checked when the parent `SecondaryHomotopy` was constructed.
    #[pyclass(frozen)]
    pub struct SecondaryHomotopyMap {
        res: Arc<RsSecRes>,
        s: i32,
    }

    #[pymethods]
    impl SecondaryHomotopyMap {
        /// The dual map on generators in source degree `t`: the matrix of the
        /// secondary homotopy `h_s` (rows indexed by the target's generators in
        /// degree `t`, columns by the source's generators in `t + degree_shift`).
        /// Returns an empty list when the target has no generators in degree `t`.
        ///
        /// Upstream `hom_k` indexes the source/target free modules' generator
        /// `OnceBiVec`s, which panic above the computed range, so it is run under
        /// `catch_unwind` (-> `ValueError`) as the defence-in-depth backstop.
        pub fn hom_k(&self, t: i32) -> PyResult<Vec<Vec<u32>>> {
            catch_secondary_compute_panic(|| self.res.homotopy(self.s).homotopies.hom_k(t))
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
        #[getter]
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

        /// Compute the induced map on $\Mod_{C\lambda^2}$ homotopy groups
        /// (upstream `SecondaryResolutionHomomorphism::hom_k`).
        ///
        /// For each input class in source bidegree `b` (an `FpVector`/`FpSlice`
        /// over the source's generators at `b`), the corresponding output is the
        /// image, written into the matching `outputs` entry (an `FpVector` or
        /// `FpSliceMut`). Each output spans the total dimension of
        /// `(b.s + shift.s - 1, b.t + shift.t - 1)` (its Ext part, the first
        /// chunk) and `(… + LAMBDA_BIDEGREE)` (its λ part, the second chunk); see
        /// the upstream docs. `sseq` records the `d₂` differentials and is used to
        /// reduce the λ part by the image of `d₂`.
        ///
        /// `inputs` and `outputs` must be the same length. Each output's length
        /// determines the span upstream writes into; outputs are *accumulated*
        /// into (matching upstream, which adds into the provided slices), so pass
        /// freshly-zeroed vectors for a plain image. All vectors must share this
        /// homomorphism's prime.
        ///
        /// Upstream indexes the source/target free modules' generator `OnceBiVec`s
        /// and the `sseq` page data, which panic outside the computed range, so the
        /// computation runs under `catch_unwind` (-> `ValueError`) as the
        /// defence-in-depth backstop.
        pub fn hom_k(
            &self,
            py: Python<'_>,
            sseq: &sseq_py::Sseq,
            b: &sseq_py::Bidegree,
            inputs: Vec<Bound<'_, PyAny>>,
            outputs: Vec<Bound<'_, PyAny>>,
        ) -> PyResult<()> {
            if inputs.len() != outputs.len() {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "hom_k expects matching numbers of inputs and outputs (got {} inputs and {} \
                     outputs)",
                    inputs.len(),
                    outputs.len()
                )));
            }

            let p = self.inner.prime();

            // Own the inputs (cloning the backing vector / slice), so we can hold
            // their slices for the duration of the upstream call without juggling
            // simultaneous Python borrows.
            let input_vecs: Vec<::fp::vector::FpVector> = inputs
                .iter()
                .map(|o| fp_py::extract_input_owned(py, o))
                .collect::<PyResult<_>>()?;

            // Allocate a zeroed scratch output per `outputs` entry, sized to the
            // entry's current span (and prime-checked). Upstream accumulates into
            // these; we add the result back into the real Python targets afterwards
            // (so the two distinct mutable borrows never overlap).
            let mut scratch: Vec<::fp::vector::FpVector> = Vec::with_capacity(outputs.len());
            for o in &outputs {
                let len = fp_py::with_target_slice_mut(py, o, |s| {
                    if s.as_slice().prime() != p {
                        return Err(pyo3::exceptions::PyValueError::new_err(format!(
                            "hom_k output vector has prime {} but this homomorphism is over prime {}",
                            s.as_slice().prime(),
                            p
                        )));
                    }
                    Ok(s.as_slice().len())
                })?;
                scratch.push(::fp::vector::FpVector::new(p, len));
            }

            catch_secondary_compute_panic(|| {
                self.inner.hom_k(
                    Some(sseq.as_rust()),
                    b.0,
                    input_vecs.iter().map(|v| v.as_slice()),
                    scratch.iter_mut().map(|v| v.as_slice_mut()),
                )
            })?;

            for (o, s) in outputs.iter().zip(scratch.iter()) {
                fp_py::with_target_slice_mut(py, o, |mut tgt| {
                    tgt.add(s.as_slice(), 1);
                    Ok(())
                })?;
            }

            Ok(())
        }

        /// Find the class whose `d₂` hits the λ part of the (null) product
        /// (upstream `SecondaryResolutionHomomorphism::product_nullhomotopy`).
        ///
        /// Given an element `class` (an `FpVector`/`FpSlice` over the target's
        /// generators at bidegree `b`, spanning its Ext and λ parts) whose
        /// product with this homomorphism is null, this returns the `FpVector`
        /// over the source's generators at `shift + b - (1, 0)` whose `d₂`
        /// witnesses the nullity — the datum used to seed the first homotopy of
        /// the secondary Massey product. `lambda_part` is the optional λ-part of
        /// this class's lift (the same `ResolutionHomomorphism` passed to
        /// `hom_k_with`); `sseq` is the source's spectral sequence, whose `d₂`
        /// quasi-inverse is applied to recover the result.
        ///
        /// `class` is cloned into an owned vector for the duration of the call
        /// and must share this homomorphism's prime. Upstream indexes the
        /// source/target free modules' generator `OnceBiVec`s, the underlying
        /// homomorphism's `hom_k` matrix, and the `sseq` differentials, all of
        /// which panic outside the computed range, so the computation runs under
        /// `catch_unwind` (-> `ValueError`) as the defence-in-depth backstop.
        pub fn product_nullhomotopy(
            &self,
            py: Python<'_>,
            lambda_part: Option<&ResolutionHomomorphism>,
            sseq: &sseq_py::Sseq,
            b: &sseq_py::Bidegree,
            class: Bound<'_, PyAny>,
        ) -> PyResult<fp_py::PyFpVector> {
            let p = self.inner.prime();

            let class_vec = fp_py::extract_input_owned(py, &class)?;
            if class_vec.prime() != p {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "product_nullhomotopy class vector has prime {} but this homomorphism is over \
                     prime {}",
                    class_vec.prime(),
                    p
                )));
            }

            let lambda = lambda_part.map(|l| l.0.as_ref());

            let result = catch_secondary_compute_panic(|| {
                self.inner.product_nullhomotopy(
                    lambda,
                    sseq.as_rust(),
                    b.0,
                    class_vec.as_slice(),
                )
            })?;

            Ok(fp_py::PyFpVector::from_rust(result))
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
        #[getter]
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

    /// Run a `SecondaryExtAlgebra` *query/compute* (`d2`/`survives`/`page_data`/
    /// `secondary_multiply_into`) under `catch_unwind`, translating any residual
    /// upstream panic into a `ValueError` (the established `catch_unwind` ->
    /// `ValueError` backstop; cf. `catch_ext_algebra_panic` /
    /// `catch_secondary_lift_panic`).
    ///
    /// The methods below pre-check what they can (`extend_all` was called,
    /// non-negative bidegrees, well-formed elements). This wrapper is the
    /// defence-in-depth net for the upstream indexing the pre-checks cannot
    /// reach without redoing the computation: `d2`'s `homotopy(b.s()+2).hom_k`
    /// `OnceBiVec`/matrix indexing, `page_data`'s `sseq.page_data(b)` (`data[b]`)
    /// indexing on an uncomputed bidegree (and the `d[d.len()-1]` underflow),
    /// and the `from_class`+`extend`+`hom_k` plumbing `secondary_multiply_into`
    /// drives.
    ///
    /// `AssertUnwindSafe` is sound for the same reason as
    /// `catch_secondary_lift_panic`/`catch_ext_algebra_panic`: a panic only
    /// leaves the `Arc`-shared, interior-mutable, append-only `OnceVec`/
    /// `OnceBiVec`/`DashMap`/`Mutex<Option>` tables in a valid-but-partial
    /// (memory-safe) state — no broken invariant for a later observer.
    fn catch_secondary_compute_panic<T, F: FnOnce() -> T>(f: F) -> PyResult<T> {
        use std::panic::{catch_unwind, AssertUnwindSafe};
        match catch_unwind(AssertUnwindSafe(f)) {
            Ok(v) => Ok(v),
            Err(payload) => {
                let detail = payload
                    .downcast_ref::<&str>()
                    .map(|s| (*s).to_owned())
                    .or_else(|| payload.downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| "unknown panic".to_owned());
                Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "secondary computation panicked (an out-of-range or unresolved bidegree, or \
                     unrealizable input, that slipped past the pre-checks); underlying panic: \
                     {detail}"
                )))
            }
        }
    }

    /// A single secondary product `x · y` in $\Mod_{C\lambda^2}$, where `y` is an
    /// $E_3$-surviving class — the bound, read-only (`frozen`) view of the
    /// upstream `ext::ext_algebra::SecondaryProduct`. Produced by
    /// [`SecondaryExtAlgebra.secondary_multiply_into`]; never constructed
    /// directly from Python.
    #[pyclass(frozen)]
    pub struct SecondaryProduct {
        source: ::sseq::coordinates::BidegreeElement,
        ext_part: ::fp::vector::FpVector,
        lambda_part: ::fp::vector::FpVector,
    }

    impl SecondaryProduct {
        /// Wrap an owned upstream `SecondaryProduct` into the bound pyclass.
        pub(crate) fn from_rust(p: ext::ext_algebra::SecondaryProduct) -> Self {
            SecondaryProduct {
                source: p.source,
                ext_part: p.ext_part,
                lambda_part: p.lambda_part,
            }
        }
    }

    #[pymethods]
    impl SecondaryProduct {
        /// The multiplicand: an $E_3$-surviving generator of the unit at the
        /// queried bidegree `b` (a `sseq_py.BidegreeElement`).
        #[getter]
        pub fn source(&self) -> sseq_py::BidegreeElement {
            sseq_py::BidegreeElement(self.source.clone())
        }

        /// The $\Ext$ part of the product, in bidegree `b + x.degree()` (an
        /// `fp_py.FpVector`).
        #[getter]
        pub fn ext_part(&self) -> fp_py::PyFpVector {
            fp_py::PyFpVector::from_rust(self.ext_part.clone())
        }

        /// The $\lambda$ part of the product, in bidegree
        /// `b + x.degree() + LAMBDA_BIDEGREE`, already reduced by the image of
        /// $d_2$ (an `fp_py.FpVector`).
        #[getter]
        pub fn lambda_part(&self) -> fp_py::PyFpVector {
            fp_py::PyFpVector::from_rust(self.lambda_part.clone())
        }

        pub fn __repr__(&self) -> String {
            format!(
                "SecondaryProduct(source={}, ext_part_dim={}, lambda_part_dim={})",
                self.source,
                self.ext_part.len(),
                self.lambda_part.len()
            )
        }
    }

    /// The concrete (standard-backend) `SecondaryExtAlgebra` monomorphisation.
    /// As with `RsExtAlgebra`/`RsSecRes`, only the standard backend is reachable:
    /// it is built from a bound `ExtAlgebra` (standard-only). `CCC::Algebra`
    /// (`= SteenrodAlgebra`) implements `PairAlgebra` — the same bound the
    /// already-bound `SecondaryResolution<Resolution<CCC>>` (`RsSecRes`) requires
    /// — so this monomorphisation type-checks.
    type RsSecondaryExtAlgebra =
        ext::ext_algebra::SecondaryExtAlgebra<ext::resolution::Resolution<CCC>>;

    /// The secondary ($d_2$) layer over an [`ExtAlgebra`]: the secondary
    /// differential `d2` (with the survival check `survives`), the $E_3$-page
    /// data (`page_data`/`unit_page_data`), and the $\Mod_{C\lambda^2}$ secondary
    /// product (`secondary_multiply_into`). Standard-backend only (it wraps a
    /// `SecondaryResolution`, which rejects Nassau — see `SecondaryResolution`).
    ///
    /// Construction is cheap; call [`extend_all`](Self::extend_all) (which
    /// computes the secondary resolutions and $E_3$ pages) before any query.
    /// Querying before `extend_all` raises `ValueError "call extend_all() first"`
    /// rather than letting the upstream `.expect()`/`OnceBiVec` index panic.
    ///
    /// Held by value with an interior `extended` flag (an `AtomicBool` set by
    /// `extend_all`): every upstream method takes `&self` (the secondary
    /// resolutions' homotopy tables and the $E_3$-page `Mutex<Option>` are
    /// interior-mutable), so a `frozen` pyclass works directly.
    #[pyclass(frozen)]
    pub struct SecondaryExtAlgebra {
        inner: RsSecondaryExtAlgebra,
        extended: std::sync::atomic::AtomicBool,
    }

    impl SecondaryExtAlgebra {
        /// Require that [`extend_all`](Self::extend_all) has completed, so the
        /// $E_3$-page `Mutex<Option>`s are populated and the secondary
        /// resolutions' homotopy `OnceBiVec`s are filled. Mirrors the upstream
        /// `.expect("call extend_all() first")` as a pre-check (a clean
        /// `ValueError`, never a panic). Also gates `d2`/`survives`, whose
        /// `homotopy(b.s()+2)` index would hit an empty `OnceBiVec` otherwise.
        fn require_extended(&self) -> PyResult<()> {
            if !self.extended.load(std::sync::atomic::Ordering::SeqCst) {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "call extend_all() first",
                ));
            }
            Ok(())
        }

        /// Validate that `x` is a well-formed, computed element of $\Ext(M, k)$
        /// (the resolution side): non-negative bidegree, the bidegree resolved,
        /// the coordinate vector over the algebra's prime and of length equal to
        /// the number of generators there. `d2`/`survives`/`secondary_multiply_into`
        /// index `x.vec()` against the generator count at its bidegree, so an
        /// over-long vector would otherwise index out of range. Reuses the
        /// `ExtAlgebra` element guard.
        fn check_res_element(&self, x: &::sseq::coordinates::BidegreeElement) -> PyResult<()> {
            let alg = self.inner.ext_algebra();
            ExtAlgebra::check_element("resolution", alg.resolution(), x, alg.prime().as_u32())
        }
    }

    #[pymethods]
    impl SecondaryExtAlgebra {
        /// Build the secondary layer over a bound `ExtAlgebra` (standard-backend;
        /// typically built with `ExtAlgebra.without_unit(res)` or
        /// `ExtAlgebra(res, res)` for the $d_2$ of the sphere). Construction is
        /// cheap — call [`extend_all`](Self::extend_all) to actually compute.
        ///
        /// Shares the SAME `ExtAlgebra` instance (see `ExtAlgebra.inner_arc`, a
        /// cheap `Arc::clone`): the secondary resolutions are built over exactly
        /// that algebra's resolution/unit, and `ext_algebra()` returns it back
        /// with stable identity and a shared product cache.
        #[new]
        pub fn new(alg: &ExtAlgebra) -> Self {
            SecondaryExtAlgebra {
                inner: RsSecondaryExtAlgebra::new(alg.inner_arc()),
                extended: std::sync::atomic::AtomicBool::new(false),
            }
        }

        /// The prime as a plain `int`.
        #[getter]
        pub fn prime(&self) -> u32 {
            self.inner.ext_algebra().prime().as_u32()
        }

        /// The primary `ExtAlgebra` this is built on: the SAME shared instance
        /// (stable identity, shared resolution/unit `Arc`s and product cache —
        /// see `ExtAlgebra.inner_arc`/`from_arc`).
        pub fn ext_algebra(&self) -> ExtAlgebra {
            ExtAlgebra::from_arc(self.inner.ext_algebra())
        }

        /// Extend the secondary resolutions as far as the underlying resolutions
        /// allow, then compute the $E_3$ pages. Must be called before `d2`,
        /// `survives`, `page_data`, `unit_page_data`, or `secondary_multiply_into`.
        ///
        /// A topologically invalid / non-realizable module can trip the inherent
        /// upstream lift-validity `assert!` ("secondary: Failed to lift …"),
        /// which is mathematical and cannot be pre-checked without performing the
        /// computation; it is contained (`catch_unwind` -> `ValueError`) so it
        /// never crosses the FFI boundary as a `PanicException`. On a clean run
        /// the interior `extended` flag is set, enabling the queries.
        pub fn extend_all(&self) -> PyResult<()> {
            catch_secondary_lift_panic(|| self.inner.extend_all())?;
            self.extended
                .store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }

        /// Sharding entry point: compute only the secondary resolution data for
        /// filtration `s` (mirrors the upstream `compute_partial`). Returns
        /// before any $E_3$ page is built, so it does *not* enable the queries —
        /// call [`extend_all`](Self::extend_all) for that. Requires `s >= 0`
        /// (`ValueError` otherwise).
        ///
        /// A non-realizable input can trip the lift-validity `assert!`; contained
        /// as in [`extend_all`](Self::extend_all).
        pub fn compute_partial(&self, s: i32) -> PyResult<()> {
            if s < 0 {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "invalid filtration s = {s}: require s >= 0"
                )));
            }
            catch_secondary_lift_panic(|| self.inner.compute_partial(s))
        }

        /// The secondary differential $d_2(x)$, a class in bidegree
        /// `(n - 1, s + 2)` (a `sseq_py.BidegreeElement`), or `None` if the
        /// target bidegree has not been computed (so $d_2$ is unknown). A
        /// computed-but-zero differential is a zero class, not `None`.
        ///
        /// Requires `extend_all()` first (`ValueError` otherwise). `x` must be a
        /// well-formed, computed element of $\Ext(M, k)$ (negative bidegree,
        /// uncomputed bidegree, prime mismatch, or wrong coordinate count raise
        /// `ValueError`).
        pub fn d2(
            &self,
            x: &sseq_py::BidegreeElement,
        ) -> PyResult<Option<sseq_py::BidegreeElement>> {
            self.require_extended()?;
            self.check_res_element(&x.0)?;
            let out = catch_secondary_compute_panic(|| self.inner.d2(&x.0))?;
            Ok(out.map(sseq_py::BidegreeElement))
        }

        /// Whether `x` is a $d_2$-cycle (a permanent class through $E_3$): `True`
        /// if `d2(x)` is the zero class, `False` if nonzero, `None` if the $d_2$
        /// target is uncomputed. Same guards as [`d2`](Self::d2).
        pub fn survives(&self, x: &sseq_py::BidegreeElement) -> PyResult<Option<bool>> {
            self.require_extended()?;
            self.check_res_element(&x.0)?;
            catch_secondary_compute_panic(|| self.inner.survives(&x.0))
        }

        /// The $E_3$-page subquotient of $\Ext(M, k)$ at bidegree `b` (an
        /// `fp_py.Subquotient`). Requires `extend_all()` first (`ValueError`
        /// otherwise). Negative `s`/`t` raises `ValueError`; an uncomputed
        /// bidegree raises `ValueError` (it would index an undefined spectral-
        /// sequence cell).
        pub fn page_data(&self, b: sseq_py::Bidegree) -> PyResult<fp_py::PySubquotient> {
            self.require_extended()?;
            require_nonneg!(b.0, "bidegree");
            let sq = catch_secondary_compute_panic(|| self.inner.page_data(b.0))?;
            Ok(fp_py::PySubquotient::from_rust(sq))
        }

        /// The $E_3$-page subquotient of the unit $\Ext(k, k)$ at bidegree `b`.
        /// Guarded as [`page_data`](Self::page_data).
        pub fn unit_page_data(&self, b: sseq_py::Bidegree) -> PyResult<fp_py::PySubquotient> {
            self.require_extended()?;
            require_nonneg!(b.0, "bidegree");
            let sq = catch_secondary_compute_panic(|| self.inner.unit_page_data(b.0))?;
            Ok(fp_py::PySubquotient::from_rust(sq))
        }

        /// The secondary product of `x` with every $E_3$-surviving class of the
        /// unit at bidegree `b`, computed in $\Mod_{C\lambda^2}$: one
        /// `SecondaryProduct` per surviving generator at `b` (empty list if none
        /// survive). The $\lambda$ part is already reduced by the image of $d_2$.
        ///
        /// Requires `extend_all()` first (`ValueError` otherwise), and both
        /// resolutions computed far enough. `x` must be a well-formed, computed
        /// element of $\Ext(M, k)$; `b` must be non-negative (otherwise
        /// `ValueError`). The product machinery (`from_class` + `extend` +
        /// `hom_k`) is run under `catch_unwind`, so an out-of-range/unresolved
        /// query surfaces as `ValueError` rather than a panic.
        pub fn secondary_multiply_into(
            &self,
            x: &sseq_py::BidegreeElement,
            b: sseq_py::Bidegree,
        ) -> PyResult<Vec<SecondaryProduct>> {
            self.require_extended()?;
            self.check_res_element(&x.0)?;
            require_nonneg!(b.0, "bidegree");
            let products =
                catch_secondary_compute_panic(|| self.inner.secondary_multiply_into(&x.0, b.0))?;
            Ok(products
                .into_iter()
                .map(SecondaryProduct::from_rust)
                .collect())
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
        /// module (then `compute_through_bidegree`, `module`, ...).
        ///
        /// `module` may be a [`SteenrodModule`] or a [`SuspensionModule`] (the
        /// latter is boxed into a `SteenrodModule` first, exactly as its
        /// `into_steenrod_module()` does); this lets the unstable examples feed a
        /// `SuspensionModule(module, shift)` straight in.
        #[staticmethod]
        pub fn ccdz(module: &Bound<'_, PyAny>) -> PyResult<Self> {
            let m = if let Ok(sm) = module.extract::<PyRef<'_, algebra_py::SteenrodModule>>() {
                sm.as_rust().clone()
            } else if let Ok(susp) =
                module.extract::<PyRef<'_, algebra_py::SuspensionModule>>()
            {
                susp.into_steenrod_module().as_rust().clone()
            } else {
                return Err(pyo3::exceptions::PyTypeError::new_err(
                    "ChainComplex.ccdz expects a SteenrodModule or a SuspensionModule",
                ));
            };
            Ok(ChainComplex(Arc::new(CCC::ccdz(Arc::new(m)))))
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
        #[getter]
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
            require_nonneg!(b.0, "bidegree");
            Ok(self.0.has_computed_bidegree(b.0))
        }

        /// Ensure every bidegree `<= b` has been computed. Like
        /// `Resolution.compute_through_stem`, a negative `s`/`t` is rejected with
        /// a `ValueError` rather than risking an internal panic.
        pub fn compute_through_bidegree(&self, b: sseq_py::Bidegree) -> PyResult<()> {
            require_nonneg!(b.0, "target bidegree");
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
        require_nonneg!(bd, "bidegree");

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
        #[getter]
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
            require_nonneg!(b.0, "bidegree");
            Ok(self.inner.has_computed_bidegree(b.0))
        }

        /// Ensure every bidegree `<= b` has been computed. Negative `s`/`t` is
        /// rejected with a `ValueError`.
        pub fn compute_through_bidegree(&self, b: sseq_py::Bidegree) -> PyResult<()> {
            require_nonneg!(b.0, "target bidegree");
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
