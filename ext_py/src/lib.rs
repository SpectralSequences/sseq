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

    use algebra::{milnor_algebra::MilnorAlgebra, module::FDModule};
    use ext::{chain_complex::FreeChainComplex, secondary::SecondaryLift, utils::Config};

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

    #[pymodule_init]
    fn init(_m: &Bound<'_, PyModule>) -> PyResult<()> {
        ext::utils::init_logging()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(())
    }
}
