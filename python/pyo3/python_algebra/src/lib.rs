
pub mod algebra;
pub mod module;


use pyo3::prelude::*;

#[pymodule]
fn python_algebra(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<algebra::PVector>()?;
    m.add_class::<algebra::AdemAlgebra>()?;
    m.add_class::<algebra::AdemBasisElement>()?;
    m.add_class::<algebra::AdemElement>()?;
    m.add_class::<algebra::MilnorAlgebra>()?;
    m.add_class::<algebra::MilnorBasisElement>()?;
    m.add_class::<algebra::MilnorElement>()?;
    m.add_class::<algebra::PythonAlgebra>()?;
    m.add_class::<algebra::PythonElement>()?;
    m.add_class::<module::FDModule>()?;
    // m.add_class::<module::FreeModule>()?;
    Ok(())
}