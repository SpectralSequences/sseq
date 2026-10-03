use pyo3::prelude::*;

mod fp_mod;

pub use fp_mod::fp_py;

#[pymodule]
#[pyo3(name = "ext")]
mod ext_py {
    #[pymodule_export]
    use super::fp_py;
}
