//! Bindings for `FreeModuleHomomorphism`, `ResolutionHomomorphism`, and
//! `ChainHomotopy` instantiated for `QueryModuleResolution`.

use std::sync::Arc;

use algebra::SteenrodAlgebra;
use algebra::module::FreeModule;
use algebra::module::homomorphism::FreeModuleHomomorphism as InnerFMH;
use ext::chain_complex::{ChainComplex, ChainHomotopy as InnerCH};
use ext::resolution_homomorphism::ResolutionHomomorphism as InnerRH;
use ext::utils::QueryModuleResolution;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use crate::coordinates::Bidegree;
use crate::fp_types::{FpVector, Matrix};
use crate::resolution::Resolution;

// (FreeModuleHomomorphism::output constructs an owned FpVector via
// `FpVector::new_owned`; see fp_types.rs.)

/// `FreeModuleHomomorphism<MuFreeModule<false, SteenrodAlgebra>>` —
/// the differential of a resolution.
#[pyclass(name = "FreeModuleHomomorphism", module = "sseq_ext")]
pub struct FreeModuleHomomorphism {
    pub inner: Arc<InnerFMH<FreeModule<SteenrodAlgebra>>>,
}

#[pymethods]
impl FreeModuleHomomorphism {
    /// `output(t, idx)` returns the image vector of the `idx`-th generator in
    /// degree `t`. Returns an *owned* `FpVector` (not a view).
    fn output(&self, generator_degree: i32, generator_index: usize) -> FpVector {
        FpVector::new_owned(self.inner.output(generator_degree, generator_index).clone())
    }

    /// Compute `Hom(_, k)` of this map at internal degree `t`.
    /// Returns a `list[list[u32]]` of shape `(target_dim, source_dim)`.
    fn hom_k(&self, t: i32) -> Vec<Vec<u32>> {
        self.inner.hom_k(t)
    }

    fn degree_shift(&self) -> i32 {
        self.inner.degree_shift()
    }
}

/// `ResolutionHomomorphism<QueryModuleResolution, QueryModuleResolution>`.
#[pyclass(name = "ResolutionHomomorphism", module = "sseq_ext")]
pub struct ResolutionHomomorphism {
    pub inner: Arc<InnerRH<QueryModuleResolution, QueryModuleResolution>>,
}

impl ResolutionHomomorphism {
    pub fn arc(&self) -> Arc<InnerRH<QueryModuleResolution, QueryModuleResolution>> {
        Arc::clone(&self.inner)
    }
}

#[pymethods]
impl ResolutionHomomorphism {
    /// Construct an empty homomorphism with the given source/target/shift.
    /// Use `extend_step` and friends to fill it in.
    #[new]
    fn new(name: String, source: &Resolution, target: &Resolution, shift: &Bidegree) -> Self {
        Self {
            inner: Arc::new(InnerRH::new(name, source.arc(), target.arc(), shift.inner)),
        }
    }

    /// Construct a chain map representing the given Ext class.
    ///
    /// The class is supplied as a list of integers, of length
    /// ``source.module(shift.s).number_of_gens_in_degree(shift.t)``. Raises
    /// `ValueError` if `class` has the wrong length.
    #[staticmethod]
    fn from_class(
        name: String,
        source: &Resolution,
        target: &Resolution,
        shift: &Bidegree,
        class: Vec<u32>,
    ) -> PyResult<Self> {
        let num_gens = source
            .arc()
            .module(shift.inner.s())
            .number_of_gens_in_degree(shift.inner.t());
        if num_gens != class.len() {
            return Err(PyValueError::new_err(format!(
                "class has length {} but source has {num_gens} generators in \
                 bidegree (s={}, t={})",
                class.len(),
                shift.inner.s(),
                shift.inner.t(),
            )));
        }
        Ok(Self {
            inner: Arc::new(InnerRH::from_class(
                name,
                source.arc(),
                target.arc(),
                shift.inner,
                &class,
            )),
        })
    }

    #[getter]
    fn name(&self) -> String {
        self.inner.name().to_owned()
    }

    fn next_homological_degree(&self) -> i32 {
        self.inner.next_homological_degree()
    }

    fn shift(&self) -> Bidegree {
        Bidegree {
            inner: self.inner.shift,
        }
    }

    /// The chain map on the `s`-th source module.
    fn get_map(&self, input_s: i32) -> FreeModuleHomomorphism {
        FreeModuleHomomorphism {
            inner: self.inner.get_map(input_s),
        }
    }

    /// Extend the homomorphism so that it is defined at `input` (the
    /// "primary" step). If `extra_images` is supplied (a `Matrix`), use it for
    /// generators that don't have a unique lift.
    #[pyo3(signature = (input, extra_images=None))]
    fn extend_step(
        &self,
        input: &Bidegree,
        extra_images: Option<&Matrix>,
        py: Python<'_>,
    ) -> (i32, i32) {
        let m_ref = extra_images.map(|m| &m.inner);
        let r = py.detach(|| self.inner.extend_step(input.inner, m_ref));
        (r.start, r.end)
    }

    /// Extend so that the map is defined up to bidegree `(s, t)`.
    fn extend(&self, max: &Bidegree, py: Python<'_>) {
        py.detach(|| self.inner.extend(max.inner));
    }

    /// Extend so that the map is defined up to stem `n` and homological degree `s`.
    fn extend_through_stem(&self, max: &Bidegree, py: Python<'_>) {
        py.detach(|| self.inner.extend_through_stem(max.inner));
    }

    fn extend_all(&self, py: Python<'_>) {
        py.detach(|| self.inner.extend_all());
    }

    /// `act(result, coeff, generator)`: write `coeff * f^*(g)` into `result`.
    /// `result` is any `FpVector` that supports mutation (owned or
    /// `view_mut`); this lets you pass a row view of a `Matrix` /
    /// `AugmentedMatrix` directly. Adds to existing values; doesn't reset.
    fn act(
        &self,
        py: Python<'_>,
        result: &mut FpVector,
        coeff: u32,
        g: &crate::coordinates::BidegreeGenerator,
    ) -> PyResult<()> {
        result.with_slice_mut_pub(py, |s| self.inner.act(s, coeff, g.inner))
    }
}

/// `ChainHomotopy<S, T, U>` instantiated with `QueryModuleResolution` for all
/// three parameters (which is the case used by `massey.rs`).
#[pyclass(name = "ChainHomotopy", module = "sseq_ext")]
pub struct ChainHomotopy {
    pub inner:
        Arc<InnerCH<QueryModuleResolution, QueryModuleResolution, QueryModuleResolution>>,
}

#[pymethods]
impl ChainHomotopy {
    /// `ChainHomotopy::new(left, right)`. The two `ResolutionHomomorphism`s
    /// must be composable: `left.target` must be the same resolution as
    /// `right.source`. Raises `ValueError` otherwise.
    #[new]
    fn new(left: &ResolutionHomomorphism, right: &ResolutionHomomorphism) -> PyResult<Self> {
        if !Arc::ptr_eq(&left.inner.target, &right.inner.source) {
            return Err(PyValueError::new_err(
                "ChainHomotopy(left, right) requires left.target to be the \
                 same resolution as right.source",
            ));
        }
        Ok(Self {
            inner: Arc::new(InnerCH::new(left.arc(), right.arc())),
        })
    }

    fn shift(&self) -> Bidegree {
        Bidegree {
            inner: self.inner.shift(),
        }
    }

    /// Lift maps so that the chain homotopy is defined on `max_source`.
    fn extend(&self, max_source: &Bidegree, py: Python<'_>) {
        py.detach(|| self.inner.extend(max_source.inner));
    }

    fn extend_all(&self, py: Python<'_>) {
        py.detach(|| self.inner.extend_all());
    }

    fn initialize_homotopies(&self, max_source_s: i32) {
        self.inner.initialize_homotopies(max_source_s);
    }

    /// Return the homotopy at homological degree `source_s`.
    fn homotopy(&self, source_s: i32) -> FreeModuleHomomorphism {
        FreeModuleHomomorphism {
            inner: self.inner.homotopy(source_s),
        }
    }
}
