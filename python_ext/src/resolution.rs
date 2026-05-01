//! Bindings for `ext::resolution::Resolution<CCC>` and entry points
//! `ext::utils::{construct, get_unit}`.

use std::path::PathBuf;
use std::sync::Arc;

use algebra::module::Module;
use ext::chain_complex::{ChainComplex, FreeChainComplex};
use ext::utils::QueryModuleResolution;
use pyo3::prelude::*;
use pyo3::types::PyAny;

use crate::coordinates::{Bidegree, BidegreeGenerator};
use crate::homomorphism::FreeModuleHomomorphism;
use crate::sseq_types::{Product, Sseq};

/// A handle to `ext::utils::QueryModuleResolution`, which is
/// `Resolution<FiniteChainComplex<SteenrodModule>>` for the (default,
/// non-nassau) feature configuration we build with.
#[pyclass(name = "Resolution", module = "sseq_ext")]
pub struct Resolution {
    pub inner: Arc<QueryModuleResolution>,
}

impl Resolution {
    pub fn arc(&self) -> Arc<QueryModuleResolution> {
        Arc::clone(&self.inner)
    }
}

#[pymethods]
impl Resolution {
    #[getter]
    fn name(&self) -> String {
        self.inner.name().to_owned()
    }

    fn prime(&self) -> crate::fp_types::ValidPrime {
        crate::fp_types::ValidPrime {
            inner: self.inner.prime(),
        }
    }

    fn min_degree(&self) -> i32 {
        self.inner.min_degree()
    }

    fn next_homological_degree(&self) -> i32 {
        self.inner.next_homological_degree()
    }

    fn has_computed_bidegree(&self, b: &Bidegree) -> bool {
        self.inner.has_computed_bidegree(b.inner)
    }

    /// Resolve up to `(s, t)` such that all bidegrees with `s' <= s` and
    /// `t' <= t` are computed.
    fn compute_through_bidegree(&self, b: &Bidegree, py: Python<'_>) {
        py.detach(|| self.inner.compute_through_bidegree(b.inner));
    }

    /// Resolve up to a given stem `n` and homological degree `s`.
    fn compute_through_stem(&self, b: &Bidegree, py: Python<'_>) {
        py.detach(|| self.inner.compute_through_stem(b.inner));
    }

    /// The module `F_s`. Returns a `FreeModule` handle.
    fn module(&self, homological_degree: i32) -> FreeModule {
        FreeModule {
            inner: Arc::clone(&self.inner.module(homological_degree)),
        }
    }

    /// The differential `d_s : F_s -> F_{s-1}`.
    fn differential(&self, s: i32) -> FreeModuleHomomorphism {
        FreeModuleHomomorphism {
            inner: self.inner.differential(s),
        }
    }

    /// The number of generators in bidegree `b`.
    fn number_of_gens_in_bidegree(&self, b: &Bidegree) -> usize {
        self.inner.number_of_gens_in_bidegree(b.inner)
    }

    /// Iterate through all defined bidegrees in increasing order of stem.
    /// Returns a Python list of `Bidegree` objects.
    fn iter_stem(&self) -> Vec<Bidegree> {
        self.inner.iter_stem().map(Bidegree::from).collect()
    }

    /// Iterate through all bidegrees that have at least one generator.
    fn iter_nonzero_stem(&self) -> Vec<Bidegree> {
        self.inner.iter_nonzero_stem().map(Bidegree::from).collect()
    }

    /// ASCII art summary, used by the `resolve` example.
    fn graded_dimension_string(&self) -> String {
        self.inner.graded_dimension_string()
    }

    /// Compute the boundary `d(g)` and return its string representation.
    fn boundary_string(&self, g: &BidegreeGenerator) -> String {
        self.inner.boundary_string(g.inner)
    }

    /// Convert to an `Sseq` for charting.
    fn to_sseq(&self) -> Sseq {
        Sseq {
            inner: self.inner.to_sseq(),
        }
    }

    /// All filtration-one operations supported by the underlying Steenrod
    /// algebra: a list of `(name, op_deg, op_idx)`.
    fn default_filtration_one_products(&self) -> Vec<(String, i32, usize)> {
        use algebra::Algebra;
        self.inner.algebra().default_filtration_one_products()
    }

    /// Return the filtration-one products with operation `(op_deg, op_idx)`,
    /// suitable for charting.
    fn filtration_one_products(&self, op_deg: i32, op_idx: usize) -> Product {
        Product {
            inner: self.inner.filtration_one_products(op_deg, op_idx),
        }
    }

    /// `(op_deg, op_idx, source)` -> `Some(matrix)` or `None` if not
    /// available. The matrix is a `list[list[u32]]`.
    fn filtration_one_product(
        &self,
        op_deg: i32,
        op_idx: usize,
        source: &Bidegree,
    ) -> Option<Vec<Vec<u32>>> {
        self.inner
            .filtration_one_product(op_deg, op_idx, source.inner)
    }
}

/// A free module `F_s` from a resolution.
#[pyclass(name = "FreeModule", module = "sseq_ext")]
pub struct FreeModule {
    pub inner: Arc<algebra::module::FreeModule<algebra::SteenrodAlgebra>>,
}

#[pymethods]
impl FreeModule {
    fn min_degree(&self) -> i32 {
        self.inner.min_degree()
    }

    fn max_computed_degree(&self) -> i32 {
        self.inner.max_computed_degree()
    }

    fn dimension(&self, degree: i32) -> usize {
        self.inner.dimension(degree)
    }

    fn number_of_gens_in_degree(&self, degree: i32) -> usize {
        self.inner.number_of_gens_in_degree(degree)
    }

    fn generator_offset(&self, degree: i32, gen_deg: i32, gen_idx: usize) -> usize {
        self.inner.generator_offset(degree, gen_deg, gen_idx)
    }
}

/// Construct a resolution.
///
/// # Arguments
///
/// * `module` - either a string of the form ``"S_2"`` / ``"Ceta@adem"`` / ``"S_2[2]"``,
///   or a Python ``dict``/``str`` containing the parsed JSON spec.
/// * `algebra` - optional, ``"adem"`` or ``"milnor"``. If supplied along with
///   a string ``module``, it overrides any ``@`` suffix (and forbids using
///   the wrong basis).
/// * `save_dir` - optional directory in which to save resolution data.
#[pyfunction]
#[pyo3(signature = (module, algebra=None, save_dir=None))]
pub fn construct(
    module: &Bound<'_, PyAny>,
    algebra: Option<&str>,
    save_dir: Option<PathBuf>,
) -> anyhow::Result<Resolution> {
    use ext::utils::Config;

    // Resolve the `Config` from the supplied Python object.
    let cfg: Config = if let Ok(s) = module.extract::<String>() {
        match algebra {
            None => Config::try_from(s.as_str())?,
            Some(a) => Config::try_from((s.as_str(), a))?,
        }
    } else {
        // Attempt: treat as a JSON-shaped object via `serde_json::Value`.
        // We go through a `repr`/`json.dumps` round-trip on the Python side
        // would be cleaner, but the simplest portable way is to ask Python to
        // dump it.
        let py = module.py();
        let json_module = py.import("json")?;
        let s: String = json_module.call_method1("dumps", (module,))?.extract()?;
        let value: serde_json::Value = serde_json::from_str(&s)?;
        Config::try_from((value, algebra.unwrap_or("milnor")))?
    };

    let resolution = ext::utils::construct(cfg, save_dir)?;
    Ok(Resolution {
        inner: Arc::new(resolution),
    })
}

/// Given a resolution, return `(is_unit, unit_resolution)`.
///
/// * `unit_save_dir` - optional save directory if a fresh unit resolution
///   needs to be created.
#[pyfunction]
#[pyo3(signature = (resolution, unit_save_dir=None))]
pub fn get_unit(
    resolution: &Resolution,
    unit_save_dir: Option<PathBuf>,
) -> anyhow::Result<(bool, Resolution)> {
    let resolution_arc = resolution.arc();
    let is_unit = {
        use ext::chain_complex::{AugmentedChainComplex, BoundedChainComplex};
        resolution_arc.target().max_s() == 1 && resolution_arc.target().module(0).is_unit()
    };

    if is_unit {
        return Ok((
            true,
            Resolution {
                inner: resolution_arc,
            },
        ));
    }

    // Build a fresh unit resolution (k -> k).
    let algebra = resolution_arc.algebra();
    let module = algebra::module::FDModule::new(
        algebra,
        "unit".to_owned(),
        bivec::BiVec::from_vec(0, vec![1]),
    );
    let cc = ext::chain_complex::FiniteChainComplex::ccdz(Arc::new(
        Box::new(module) as algebra::module::SteenrodModule
    ));
    let unit = ext::resolution::Resolution::new_with_save(Arc::new(cc), unit_save_dir)?;
    Ok((
        false,
        Resolution {
            inner: Arc::new(unit),
        },
    ))
}
