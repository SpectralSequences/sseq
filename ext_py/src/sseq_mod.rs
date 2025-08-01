use pyo3::prelude::*;

#[pymodule]
#[pyo3(name = "sseq")]
pub mod sseq_py {
    use super::*;

    #[pyclass] // This will be part of the module
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Bidegree(pub sseq::coordinates::Bidegree);

    impl From<Bidegree> for sseq::coordinates::Bidegree {
        fn from(value: Bidegree) -> Self {
            value.0
        }
    }

    #[pymethods]
    impl Bidegree {
        #[staticmethod]
        pub fn s_t(s: i32, t: i32) -> Self {
            Bidegree(sseq::coordinates::Bidegree::s_t(s, t))
        }

        #[staticmethod]
        pub fn n_s(n: i32, s: i32) -> Self {
            Bidegree(sseq::coordinates::Bidegree::n_s(n, s))
        }
    }
}
