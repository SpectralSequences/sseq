
pub mod algebra;
pub mod module;


use pyo3::{
    prelude::*,
    wrap_pymodule
};

#[pymodule]
fn algebra(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<algebra::PVector>()?;
    
    m.add_class::<algebra::AdemAlgebra>()?;
    m.add_class::<algebra::AdemBasisElement>()?;
    m.add_class::<algebra::AdemElement>()?;

    m.add_class::<algebra::MilnorAlgebra>()?;
    m.add_class::<algebra::MilnorBasisElement>()?;
    m.add_class::<algebra::MilnorElement>()?;
    
    m.add_class::<algebra::PythonAlgebra>()?;
    m.add_class::<algebra::PythonElement>()?;
    Ok(())
}

#[pymodule]
fn module(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<module::FDModule>()?;
    m.add_class::<module::RealProjectiveSpace>()?;
    m.add_class::<module::FreeModule>()?;
    m.add_class::<module::FreeUnstableModule>()?;
    m.add_class::<module::KFpn>()?;
    m.add_class::<module::BCp>()?;
    m.add_class::<module::Dickson2>()?;
    Ok(())
}

#[pymodule]
fn python_algebra(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_wrapped(wrap_pymodule!(algebra))?;
    m.add_wrapped(wrap_pymodule!(module))?;
    Ok(())
}