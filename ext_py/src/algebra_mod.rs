use pyo3::prelude::*;

#[pymodule]
#[pyo3(name = "algebra")]
pub mod algebra_py {
    use algebra::Algebra;

    use super::*;

    #[pyclass] // This will be part of the module
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum AlgebraType {
        Adem,
        Milnor,
    }

    impl From<AlgebraType> for ::algebra::AlgebraType {
        fn from(value: AlgebraType) -> Self {
            match value {
                AlgebraType::Adem => ::algebra::AlgebraType::Adem,
                AlgebraType::Milnor => ::algebra::AlgebraType::Milnor,
            }
        }
    }

    #[pyclass]
    pub struct MilnorAlgebra(::algebra::MilnorAlgebra);

    #[pymethods]
    impl MilnorAlgebra {
        #[new]
        pub fn new(p: u32, unstable_enabled: bool) -> Self {
            MilnorAlgebra(::algebra::MilnorAlgebra::new(
                ::fp::prime::ValidPrime::new(p),
                unstable_enabled,
            ))
        }

        pub fn compute_basis(&mut self, degree: i32) {
            self.0.compute_basis(degree);
        }

        pub fn dimension(&self, degree: i32) -> usize {
            self.0.dimension(degree)
        }
    }

    #[pymodule_init]
    fn init(_m: &Bound<'_, PyModule>) -> PyResult<()> {
        // Arbitrary code to run at the module initialization
        // m.add("double2", m.getattr("double")?)
        Ok(())
    }
}
