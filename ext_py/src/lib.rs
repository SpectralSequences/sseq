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
    fn build(spec: Config, algorithm: Option<&str>) -> anyhow::Result<AnyResolution> {
        use ext::utils::{construct_nassau, construct_standard};

        let nassau = |spec| construct_nassau(spec, None).map(|r| AnyResolution::Nassau(Arc::new(r)));
        let standard = |spec| {
            construct_standard::<false, _, _>(spec, None)
                .map(|r| AnyResolution::Standard(Arc::new(r)))
        };

        match algorithm {
            Some("nassau") => nassau(spec),
            Some("standard") => standard(spec),
            None | Some("auto") => match nassau(spec.clone()) {
                Ok(res) => Ok(res),
                // Nassau validates eligibility before doing any work, so this probe is safe.
                Err(_) => standard(spec),
            },
            Some(other) => Err(anyhow::anyhow!(
                "Unknown algorithm {other:?}; expected \"auto\", \"nassau\", or \"standard\""
            )),
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
            build(config, algorithm)
                .map(Resolution)
                .map_err(|e: anyhow::Error| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
        }

        pub fn compute_through_stem(&self, max: sseq_py::Bidegree) {
            dispatch!(&self.0, r => r.compute_through_stem(max.0))
        }

        pub fn graded_dimension_string(&self) -> String {
            dispatch!(&self.0, r => r.graded_dimension_string())
        }
    }

    enum AnySecondary {
        Nassau(
            ext::secondary::SecondaryResolution<ext::nassau::Resolution<FDModule<MilnorAlgebra>>>,
        ),
        Standard(ext::secondary::SecondaryResolution<ext::resolution::Resolution<ext::CCC>>),
    }

    #[pyclass(frozen)]
    pub struct SecondaryResolution(AnySecondary);

    #[pymethods]
    impl SecondaryResolution {
        #[new]
        pub fn new(cc: &Resolution) -> Self {
            SecondaryResolution(match &cc.0 {
                AnyResolution::Nassau(r) => {
                    AnySecondary::Nassau(ext::secondary::SecondaryResolution::new(Arc::clone(r)))
                }
                AnyResolution::Standard(r) => {
                    AnySecondary::Standard(ext::secondary::SecondaryResolution::new(Arc::clone(r)))
                }
            })
        }

        pub fn extend_all(&self) {
            match &self.0 {
                AnySecondary::Nassau(s) => s.extend_all(),
                AnySecondary::Standard(s) => s.extend_all(),
            }
        }

        pub fn underlying(&self) -> Resolution {
            Resolution(match &self.0 {
                AnySecondary::Nassau(s) => AnyResolution::Nassau(Arc::clone(&s.underlying())),
                AnySecondary::Standard(s) => AnyResolution::Standard(Arc::clone(&s.underlying())),
            })
        }
    }

    #[pymodule_init]
    fn init(_m: &Bound<'_, PyModule>) -> PyResult<()> {
        ext::utils::init_logging()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(())
    }
}
