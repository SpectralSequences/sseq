// Added core_intrinsics so that python_utils can display name of
// dropped type for debug purposes
// #![feature(core_intrinsics)]

// pub mod basis;
pub mod matrix;
pub mod prime;
pub mod vector;

use pyo3::prelude::*;

#[pymodule]
fn python_fp(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<vector::FpVector>()?;
    m.add_class::<matrix::PivotVecWrapper>()?;
    m.add_class::<matrix::Matrix>()?;
    m.add_class::<matrix::Subspace>()?;
    m.add_class::<matrix::QuasiInverse>()?;
    // m.add_class::<basis::Basis>()?;
    Ok(())
}
