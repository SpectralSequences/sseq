// Added core_intrinsics so that python_utils can display name of 
// dropped type for debug purposes
// #![feature(core_intrinsics)]

mod utils;
mod adem_algebra;
mod milnor_algebra;
mod python_algebra;

use pyo3::prelude::*;

#[pymodule]
fn python_algebra(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<utils::PVector>()?;
    m.add_class::<adem_algebra::AdemAlgebra>()?;
    m.add_class::<adem_algebra::AdemBasisElement>()?;
    m.add_class::<adem_algebra::AdemElement>()?;
    m.add_class::<milnor_algebra::MilnorAlgebra>()?;
    m.add_class::<milnor_algebra::MilnorBasisElement>()?;
    m.add_class::<milnor_algebra::MilnorElement>()?;
    m.add_class::<python_algebra::PythonAlgebra>()?;
    Ok(())
}