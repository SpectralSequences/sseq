use pyo3::prelude::*;

#[pymodule]
#[pyo3(name = "fp")]
pub mod fp_py {
    use fp::field::{element::FieldElement, Field, Fp as RustFp, SmallFq as RustSmallFq};
    use fp::prime::{self, Binomial, Prime};
    use pyo3::exceptions::PyValueError;

    use super::*;

    const MAX_VALID_PRIME: u32 = 1 << 31;

    type DynFp = RustFp<prime::ValidPrime>;
    type DynSmallFq = RustSmallFq<prime::ValidPrime>;
    type DynFpElement = FieldElement<DynFp>;
    type DynSmallFqElement = FieldElement<DynSmallFq>;

    #[pyclass(name = "Fp", frozen, from_py_object)]
    #[derive(Clone, Copy)]
    pub struct PyFp(DynFp);

    #[pyclass(name = "SmallFq", frozen, from_py_object)]
    #[derive(Clone, Copy)]
    pub struct PySmallFq(DynSmallFq);

    #[derive(Clone, Copy)]
    enum FieldElementKind {
        Fp(DynFpElement),
        SmallFq(DynSmallFqElement),
    }

    #[pyclass(name = "FieldElement", frozen, from_py_object)]
    #[derive(Clone, Copy)]
    pub struct PyFieldElement(FieldElementKind);

    fn valid_prime(p: u32) -> PyResult<prime::ValidPrime> {
        if p < 2 || p >= MAX_VALID_PRIME {
            return Err(PyValueError::new_err(format!("{p} is not prime")));
        }
        prime::ValidPrime::try_from(p)
            .map_err(|_| PyValueError::new_err(format!("{p} is not prime")))
    }

    fn table_prime(p: u32) -> PyResult<prime::ValidPrime> {
        if fp::PRIMES.contains(&p) {
            valid_prime(p)
        } else {
            Err(PyValueError::new_err(format!(
                "{p} is not a supported table prime"
            )))
        }
    }

    fn small_fq(p: u32, degree: u32) -> PyResult<DynSmallFq> {
        let p = valid_prime(p)?;
        if degree <= 1 {
            return Err(PyValueError::new_err("degree must be greater than 1"));
        }
        if degree > 16 || p.as_u32().checked_pow(degree).is_none_or(|q| q >= 1 << 16) {
            return Err(PyValueError::new_err("field is too large"));
        }
        Ok(DynSmallFq::new(p, degree))
    }

    impl FieldElementKind {
        fn field_name(self) -> &'static str {
            match self {
                Self::Fp(_) => "Fp",
                Self::SmallFq(_) => "SmallFq",
            }
        }

        fn mismatched_field_error(lhs: Self, rhs: Self) -> PyErr {
            PyValueError::new_err(format!(
                "cannot combine {} element with {} element",
                lhs.field_name(),
                rhs.field_name()
            ))
        }
    }

    #[pymethods]
    impl PyFp {
        #[new]
        pub fn new(p: u32) -> PyResult<Self> {
            Ok(Self(DynFp::new(valid_prime(p)?)))
        }

        pub fn characteristic(&self) -> u32 {
            self.0.characteristic().as_u32()
        }

        pub fn degree(&self) -> u32 {
            self.0.degree()
        }

        pub fn zero(&self) -> PyFieldElement {
            PyFieldElement(FieldElementKind::Fp(self.0.zero()))
        }

        pub fn one(&self) -> PyFieldElement {
            PyFieldElement(FieldElementKind::Fp(self.0.one()))
        }

        pub fn element(&self, value: u32) -> PyFieldElement {
            PyFieldElement(FieldElementKind::Fp(self.0.element(value)))
        }

        pub fn __repr__(&self) -> String {
            format!("Fp({})", self.characteristic())
        }
    }

    #[pymethods]
    impl PySmallFq {
        #[new]
        pub fn new(p: u32, degree: u32) -> PyResult<Self> {
            Ok(Self(small_fq(p, degree)?))
        }

        pub fn p(&self) -> u32 {
            self.0.characteristic().as_u32()
        }

        pub fn degree(&self) -> u32 {
            self.0.degree()
        }

        pub fn a(&self) -> PyFieldElement {
            PyFieldElement(FieldElementKind::SmallFq(self.0.a()))
        }

        pub fn q(&self) -> u32 {
            self.0.q()
        }

        pub fn zero(&self) -> PyFieldElement {
            PyFieldElement(FieldElementKind::SmallFq(self.0.zero()))
        }

        pub fn one(&self) -> PyFieldElement {
            PyFieldElement(FieldElementKind::SmallFq(self.0.one()))
        }

        pub fn __repr__(&self) -> String {
            format!("SmallFq({}, {})", self.p(), self.degree())
        }
    }

    #[pymethods]
    impl PyFieldElement {
        pub fn inv(&self) -> Option<Self> {
            match self.0 {
                FieldElementKind::Fp(x) => x.inv().map(|x| Self(FieldElementKind::Fp(x))),
                FieldElementKind::SmallFq(x) => x.inv().map(|x| Self(FieldElementKind::SmallFq(x))),
            }
        }

        pub fn frobenius(&self) -> Self {
            match self.0 {
                FieldElementKind::Fp(x) => Self(FieldElementKind::Fp(x.frobenius())),
                FieldElementKind::SmallFq(x) => Self(FieldElementKind::SmallFq(x.frobenius())),
            }
        }

        pub fn field<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
            match self.0 {
                FieldElementKind::Fp(x) => {
                    Py::new(py, PyFp(x.field())).map(|x| x.into_bound(py).into_any())
                }
                FieldElementKind::SmallFq(x) => {
                    Py::new(py, PySmallFq(x.field())).map(|x| x.into_bound(py).into_any())
                }
            }
        }

        pub fn __add__(&self, rhs: Self) -> PyResult<Self> {
            match (self.0, rhs.0) {
                (FieldElementKind::Fp(a), FieldElementKind::Fp(b)) if a.field() == b.field() => {
                    Ok(Self(FieldElementKind::Fp(a + b)))
                }
                (FieldElementKind::SmallFq(a), FieldElementKind::SmallFq(b))
                    if a.field() == b.field() =>
                {
                    Ok(Self(FieldElementKind::SmallFq(a + b)))
                }
                (a, b) => Err(FieldElementKind::mismatched_field_error(a, b)),
            }
        }

        pub fn __sub__(&self, rhs: Self) -> PyResult<Self> {
            match (self.0, rhs.0) {
                (FieldElementKind::Fp(a), FieldElementKind::Fp(b)) if a.field() == b.field() => {
                    Ok(Self(FieldElementKind::Fp(a - b)))
                }
                (FieldElementKind::SmallFq(a), FieldElementKind::SmallFq(b))
                    if a.field() == b.field() =>
                {
                    Ok(Self(FieldElementKind::SmallFq(a - b)))
                }
                (a, b) => Err(FieldElementKind::mismatched_field_error(a, b)),
            }
        }

        pub fn __mul__(&self, rhs: Self) -> PyResult<Self> {
            match (self.0, rhs.0) {
                (FieldElementKind::Fp(a), FieldElementKind::Fp(b)) if a.field() == b.field() => {
                    Ok(Self(FieldElementKind::Fp(a * b)))
                }
                (FieldElementKind::SmallFq(a), FieldElementKind::SmallFq(b))
                    if a.field() == b.field() =>
                {
                    Ok(Self(FieldElementKind::SmallFq(a * b)))
                }
                (a, b) => Err(FieldElementKind::mismatched_field_error(a, b)),
            }
        }

        pub fn __truediv__(&self, rhs: Self) -> PyResult<Option<Self>> {
            match (self.0, rhs.0) {
                (FieldElementKind::Fp(a), FieldElementKind::Fp(b)) if a.field() == b.field() => {
                    Ok((a / b).map(|x| Self(FieldElementKind::Fp(x))))
                }
                (FieldElementKind::SmallFq(a), FieldElementKind::SmallFq(b))
                    if a.field() == b.field() =>
                {
                    Ok((a / b).map(|x| Self(FieldElementKind::SmallFq(x))))
                }
                (a, b) => Err(FieldElementKind::mismatched_field_error(a, b)),
            }
        }

        pub fn __neg__(&self) -> Self {
            match self.0 {
                FieldElementKind::Fp(x) => Self(FieldElementKind::Fp(-x)),
                FieldElementKind::SmallFq(x) => Self(FieldElementKind::SmallFq(-x)),
            }
        }

        pub fn __int__(&self) -> PyResult<u32> {
            match self.0 {
                FieldElementKind::Fp(x) => Ok(*x),
                FieldElementKind::SmallFq(_) => Err(PyValueError::new_err(
                    "SmallFq elements do not have a canonical integer value",
                )),
            }
        }

        pub fn __repr__(&self) -> String {
            match self.0 {
                FieldElementKind::Fp(x) => {
                    format!("FieldElement(Fp({}), {x})", x.field().characteristic())
                }
                FieldElementKind::SmallFq(x) => {
                    let f = x.field();
                    format!(
                        "FieldElement(SmallFq({}, {}), {x})",
                        f.characteristic(),
                        f.degree()
                    )
                }
            }
        }
    }

    #[pyfunction]
    pub fn power_mod(p: u32, b: u32, e: u32) -> PyResult<u32> {
        Ok(valid_prime(p)?.pow_mod(b, e))
    }

    #[pyfunction]
    pub fn log2(n: usize) -> usize {
        prime::log2(n)
    }

    #[pyfunction]
    pub fn logp(p: u32, n: u32) -> PyResult<u32> {
        Ok(prime::logp(valid_prime(p)?, n))
    }

    #[pyfunction]
    pub fn factor_pk(p: u32, n: u32) -> PyResult<(u32, u32)> {
        Ok(prime::factor_pk(valid_prime(p)?, n))
    }

    #[pyfunction]
    pub fn inverse(p: u32, k: u32) -> PyResult<u32> {
        Ok(prime::inverse(valid_prime(p)?, k))
    }

    #[pyfunction]
    pub fn minus_one_to_the_n(p: u32, i: i32) -> PyResult<u32> {
        Ok(prime::minus_one_to_the_n(valid_prime(p)?, i))
    }

    #[pyfunction]
    pub fn is_prime(p: u32) -> bool {
        valid_prime(p).is_ok()
    }

    #[pyfunction]
    pub fn binomial(p: u32, n: u32, k: u32) -> PyResult<u32> {
        Ok(u32::binomial(table_prime(p)?, n, k))
    }

    #[pyfunction]
    pub fn multinomial(p: u32, mut l: Vec<u32>) -> PyResult<u32> {
        Ok(u32::multinomial(table_prime(p)?, &mut l))
    }

    #[pyfunction]
    pub fn binomial_odd_is_zero(p: u32, n: u32, k: u32) -> PyResult<bool> {
        Ok(u32::binomial_odd_is_zero(table_prime(p)?, n, k))
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
        m.add("F2", PyFp(DynFp::new(prime::TWO)))?;
        m.add("F3", PyFp(DynFp::new(prime::P3.to_dyn())))?;
        m.add("F5", PyFp(DynFp::new(prime::P5.to_dyn())))?;
        m.add("F7", PyFp(DynFp::new(prime::P7.to_dyn())))?;
        m.add("TWO", prime::TWO.as_u32())?;
        m.add("PRIMES", fp::PRIMES.to_vec())?;
        m.add("NUM_PRIMES", fp::NUM_PRIMES)?;
        m.add("PRIME_TO_INDEX_MAP", fp::PRIME_TO_INDEX_MAP.to_vec())?;
        m.add("MAX_MULTINOMIAL_LEN", fp::MAX_MULTINOMIAL_LEN)?;
        m.add("ODD_PRIMES", fp::ODD_PRIMES)?;
        Ok(())
    }
}
