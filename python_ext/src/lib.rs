//! Python bindings for the `ext` Rust crate.
//!
//! Scope: this module exposes enough of the API to translate the following
//! `ext/examples/*.rs` programs into Python:
//!
//! - `resolve.rs`
//! - `num_gens.rs`
//! - `chart.rs`
//! - `secondary.rs`
//! - `massey.rs`
//!
//! See `python_ext/README.md` for the design rationale.

#![allow(clippy::too_many_arguments)]

use pyo3::prelude::*;

mod algebra;
mod chart;
mod coordinates;
mod fp_types;
mod homomorphism;
mod resolution;
mod secondary;
mod sseq_types;

/// The native extension module.
#[pymodule]
fn _sseq_ext(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Initialize the global tracing subscriber. This may be called more than
    // once across processes; we ignore the error from the second call.
    m.add_function(wrap_pyfunction!(init_logging, m)?)?;
    m.add_function(wrap_pyfunction!(secondary_job, m)?)?;
    m.add_function(wrap_pyfunction!(resolution::construct, m)?)?;
    m.add_function(wrap_pyfunction!(resolution::get_unit, m)?)?;
    m.add_function(wrap_pyfunction!(chart::write_sseq_svg, m)?)?;

    m.add_class::<algebra::MilnorAlgebra>()?;
    m.add_class::<coordinates::Bidegree>()?;
    m.add_class::<coordinates::BidegreeGenerator>()?;
    m.add_class::<coordinates::BidegreeElement>()?;
    m.add_class::<fp_types::ValidPrime>()?;
    m.add_class::<fp_types::FpVector>()?;
    m.add_class::<fp_types::Matrix>()?;
    m.add_class::<fp_types::MatrixView>()?;
    m.add_class::<fp_types::MatrixViewMut>()?;
    m.add_class::<fp_types::AugmentedMatrix>()?;
    m.add_class::<fp_types::AugmentedMatrixView>()?;
    m.add_class::<fp_types::AugmentedMatrixViewMut>()?;
    m.add_class::<fp_types::Subspace>()?;
    m.add_class::<resolution::Resolution>()?;
    m.add_class::<homomorphism::FreeModuleHomomorphism>()?;
    m.add_class::<homomorphism::ResolutionHomomorphism>()?;
    m.add_class::<homomorphism::ChainHomotopy>()?;
    m.add_class::<secondary::SecondaryResolution>()?;
    m.add_class::<secondary::SecondaryHomotopy>()?;
    m.add_class::<sseq_types::Sseq>()?;
    m.add_class::<sseq_types::Product>()?;
    Ok(())
}

#[pyfunction]
fn init_logging() -> PyResult<()> {
    // Ignore "already set" errors so callers can call this freely from each
    // entry point.
    let _ = ext::utils::init_logging();
    Ok(())
}

#[pyfunction]
fn secondary_job() -> Option<i32> {
    ext::utils::secondary_job()
}
