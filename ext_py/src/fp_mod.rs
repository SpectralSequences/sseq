use pyo3::prelude::*;

#[pymodule]
#[pyo3(name = "fp")]
pub mod fp_py {
    use fp::prime::{self, Binomial, Prime};
    use pyo3::basic::CompareOp;

    use super::*;

    #[pyclass(frozen)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct ValidPrime(pub prime::ValidPrime);

    impl From<prime::ValidPrime> for ValidPrime {
        fn from(value: prime::ValidPrime) -> Self {
            Self(value)
        }
    }

    impl From<ValidPrime> for prime::ValidPrime {
        fn from(value: ValidPrime) -> Self {
            value.0
        }
    }

    #[pymethods]
    impl ValidPrime {
        #[new]
        pub fn new(p: u32) -> Self {
            prime::ValidPrime::new(p).into()
        }

        #[staticmethod]
        pub fn new_unchecked(p: u32) -> Self {
            prime::ValidPrime::new_unchecked(p).into()
        }

        pub fn as_i32(&self) -> i32 {
            self.0.as_i32()
        }

        pub fn as_u32(&self) -> u32 {
            self.0.as_u32()
        }

        pub fn as_usize(&self) -> usize {
            self.0.as_usize()
        }

        pub fn sum(&self, a: u32, b: u32) -> u32 {
            self.0.sum(a, b)
        }

        pub fn product(&self, a: u32, b: u32) -> u32 {
            self.0.product(a, b)
        }

        pub fn inverse(&self, k: u32) -> u32 {
            self.0.inverse(k)
        }

        pub fn pow(&self, exp: u32) -> u32 {
            self.0.pow(exp)
        }

        pub fn pow_mod(&self, b: u32, e: u32) -> u32 {
            self.0.pow_mod(b, e)
        }

        fn __int__(&self) -> u32 {
            self.0.as_u32()
        }

        fn __index__(&self) -> u32 {
            self.0.as_u32()
        }

        fn __hash__(&self) -> u64 {
            self.0.as_u32() as u64
        }

        fn __repr__(&self) -> String {
            format!("ValidPrime({})", self.0)
        }

        fn __richcmp__(&self, other: Self, op: CompareOp) -> bool {
            match op {
                CompareOp::Eq => self.0 == other.0,
                CompareOp::Ne => self.0 != other.0,
                _ => false,
            }
        }
    }

    #[pyfunction]
    pub fn power_mod(p: u32, b: u32, e: u32) -> u32 {
        prime::power_mod(p, b, e)
    }

    #[pyfunction]
    pub fn log2(n: usize) -> usize {
        prime::log2(n)
    }

    #[pyfunction]
    pub fn logp(p: ValidPrime, n: u32) -> u32 {
        prime::logp(p.0, n)
    }

    #[pyfunction]
    pub fn factor_pk(p: ValidPrime, n: u32) -> (u32, u32) {
        prime::factor_pk(p.0, n)
    }

    #[pyfunction]
    pub fn inverse(p: ValidPrime, k: u32) -> u32 {
        prime::inverse(p.0, k)
    }

    #[pyfunction]
    pub fn minus_one_to_the_n(p: ValidPrime, i: i32) -> u32 {
        prime::minus_one_to_the_n(p.0, i)
    }

    #[pyfunction]
    pub fn is_prime(p: u32) -> bool {
        prime::is_prime(p)
    }

    #[pyfunction]
    pub fn binomial(p: ValidPrime, n: u32, k: u32) -> u32 {
        u32::binomial(p.0, n, k)
    }

    #[pyfunction]
    pub fn multinomial(p: ValidPrime, mut l: Vec<u32>) -> u32 {
        u32::multinomial(p.0, &mut l)
    }

    #[pyfunction]
    pub fn binomial_odd_is_zero(p: ValidPrime, n: u32, k: u32) -> bool {
        u32::binomial_odd_is_zero(p.0, n, k)
    }

    #[pyfunction]
    pub fn binomial2(n: u32, k: u32) -> u32 {
        u32::binomial2(n, k)
    }

    #[pyfunction]
    pub fn multinomial2(l: Vec<u32>) -> u32 {
        u32::multinomial2(&l)
    }

    #[pyfunction]
    pub fn binomial4(n: u32, k: u32) -> u32 {
        u32::binomial4(n, k)
    }

    #[pyfunction]
    pub fn binomial4_rec(n: u32, k: u32) -> u32 {
        u32::binomial4_rec(n, k)
    }

    #[pymodule_init]
    fn init(m: &Bound<'_, PyModule>) -> PyResult<()> {
        m.add("TWO", ValidPrime(prime::TWO))?;
        m.add("PRIMES", fp::PRIMES.to_vec())?;
        m.add("NUM_PRIMES", fp::NUM_PRIMES)?;
        m.add("PRIME_TO_INDEX_MAP", fp::PRIME_TO_INDEX_MAP.to_vec())?;
        m.add("MAX_MULTINOMIAL_LEN", fp::MAX_MULTINOMIAL_LEN)?;
        m.add("ODD_PRIMES", fp::ODD_PRIMES)?;
        Ok(())
    }
}
