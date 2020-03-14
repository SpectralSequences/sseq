
mod algebra_bindings;
mod algebra_rust;
mod adem_algebra;
mod milnor_algebra;
mod python_algebra;
mod module;


use pyo3::prelude::*;

#[pymodule]
fn python_algebra(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<algebra_bindings::PVector>()?;
    m.add_class::<adem_algebra::AdemAlgebra>()?;
    m.add_class::<adem_algebra::AdemBasisElement>()?;
    m.add_class::<adem_algebra::AdemElement>()?;
    m.add_class::<milnor_algebra::MilnorAlgebra>()?;
    m.add_class::<milnor_algebra::MilnorBasisElement>()?;
    m.add_class::<milnor_algebra::MilnorElement>()?;
    m.add_class::<python_algebra::PythonAlgebra>()?;
    m.add_class::<python_algebra::PythonElement>()?;
    // m.add_class::<module::FDModule>()?;
    // m.add_class::<module::FreeModule>()?;
    Ok(())
}