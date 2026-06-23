use pyo3::prelude::*;

#[pymodule]
#[pyo3(name = "algebra")]
pub mod algebra_py {
    use ::algebra::{Algebra, Bialgebra, GeneratedAlgebra};
    use ::fp::prime::{self, Prime};
    use pyo3::basic::CompareOp;
    use pyo3::exceptions::{PyIndexError, PyValueError};

    use super::*;

    /// Upper bound on accepted primes, mirroring `fp_py::valid_prime`.
    const MAX_VALID_PRIME: u32 = 1 << 31;

    /// Convert a plain `int` prime from Python into a `ValidPrime`, raising
    /// `ValueError` (never panicking) for a non-prime. `ValidPrime` itself is
    /// never exposed to Python. Mirrors the `valid_prime` helper in `fp_mod`.
    fn valid_prime(p: u32) -> PyResult<prime::ValidPrime> {
        if p < 2 || p >= MAX_VALID_PRIME {
            return Err(PyValueError::new_err(format!("{p} is not prime")));
        }
        prime::ValidPrime::try_from(p)
            .map_err(|_| PyValueError::new_err(format!("{p} is not prime")))
    }

    fn checked_same_prime(lhs: u32, rhs: u32) -> PyResult<()> {
        if lhs == rhs {
            Ok(())
        } else {
            Err(PyValueError::new_err(format!(
                "prime mismatch: {lhs} != {rhs}"
            )))
        }
    }

    fn checked_equal_len(lhs: usize, rhs: usize) -> PyResult<()> {
        if lhs == rhs {
            Ok(())
        } else {
            Err(PyValueError::new_err(format!(
                "length mismatch: {lhs} != {rhs}"
            )))
        }
    }

    /// Ensure a result slice is long enough to receive a product landing in a
    /// space of dimension `dim`, raising `ValueError` rather than letting an
    /// upstream `add_basis_element` index panic.
    fn checked_result_len(len: usize, dim: usize) -> PyResult<()> {
        if len >= dim {
            Ok(())
        } else {
            Err(PyValueError::new_err(format!(
                "result has length {len} but the target degree has dimension {dim}"
            )))
        }
    }

    fn non_negative_degree(degree: i32) -> PyResult<()> {
        if degree >= 0 {
            Ok(())
        } else {
            Err(PyIndexError::new_err(format!(
                "degree {degree} is negative"
            )))
        }
    }

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

    /// A basis element of the Milnor algebra: a product of exterior generators
    /// `Q_k` (encoded as the bitmask `q_part`) and a polynomial part `P(p_part)`.
    #[pyclass(name = "MilnorBasisElement", skip_from_py_object)]
    #[derive(Clone)]
    pub struct MilnorBasisElement(::algebra::milnor_algebra::MilnorBasisElement);

    #[pymethods]
    impl MilnorBasisElement {
        #[new]
        #[pyo3(signature = (p_part, q_part = 0, degree = 0))]
        pub fn new(p_part: Vec<u32>, q_part: u32, degree: i32) -> Self {
            MilnorBasisElement(::algebra::milnor_algebra::MilnorBasisElement {
                q_part,
                p_part,
                degree,
            })
        }

        #[getter]
        pub fn q_part(&self) -> u32 {
            self.0.q_part
        }

        #[setter]
        pub fn set_q_part(&mut self, value: u32) {
            self.0.q_part = value;
        }

        #[getter]
        pub fn p_part(&self) -> Vec<u32> {
            self.0.p_part.clone()
        }

        #[setter]
        pub fn set_p_part(&mut self, value: Vec<u32>) {
            self.0.p_part = value;
        }

        #[getter]
        pub fn degree(&self) -> i32 {
            self.0.degree
        }

        #[setter]
        pub fn set_degree(&mut self, value: i32) {
            self.0.degree = value;
        }

        /// Recompute the `degree` field from the `p_part`/`q_part` at prime `p`.
        pub fn compute_degree(&mut self, p: u32) -> PyResult<()> {
            self.0.compute_degree(valid_prime(p)?);
            Ok(())
        }

        pub fn __repr__(&self) -> String {
            format!(
                "MilnorBasisElement(p_part={:?}, q_part={}, degree={})",
                self.0.p_part, self.0.q_part, self.0.degree
            )
        }

        pub fn __str__(&self) -> String {
            format!("{}", self.0)
        }

        pub fn __richcmp__(&self, other: &Bound<'_, PyAny>, op: CompareOp) -> bool {
            let eq = other
                .extract::<PyRef<Self>>()
                .is_ok_and(|other| self.0 == other.0);
            match op {
                CompareOp::Eq => eq,
                CompareOp::Ne => !eq,
                _ => false,
            }
        }
    }

    /// A Milnor profile function, describing a sub-Hopf-algebra of the Steenrod
    /// algebra.
    #[pyclass(name = "MilnorProfile")]
    pub struct MilnorProfile(::algebra::milnor_algebra::MilnorProfile);

    impl MilnorProfile {
        /// `MilnorProfile` upstream is intentionally not `Clone`; reconstruct a
        /// fresh copy from its public fields when we need to hand one to the
        /// algebra constructor or return one to Python.
        fn to_rust(&self) -> ::algebra::milnor_algebra::MilnorProfile {
            ::algebra::milnor_algebra::MilnorProfile {
                truncated: self.0.truncated,
                q_part: self.0.q_part,
                p_part: self.0.p_part.clone(),
            }
        }
    }

    #[pymethods]
    impl MilnorProfile {
        #[new]
        #[pyo3(signature = (truncated = false, q_part = u32::MAX, p_part = Vec::new()))]
        pub fn new(truncated: bool, q_part: u32, p_part: Vec<u32>) -> Self {
            MilnorProfile(::algebra::milnor_algebra::MilnorProfile {
                truncated,
                q_part,
                p_part,
            })
        }

        #[getter]
        pub fn truncated(&self) -> bool {
            self.0.truncated
        }

        #[setter]
        pub fn set_truncated(&mut self, value: bool) {
            self.0.truncated = value;
        }

        #[getter]
        pub fn q_part(&self) -> u32 {
            self.0.q_part
        }

        #[setter]
        pub fn set_q_part(&mut self, value: u32) {
            self.0.q_part = value;
        }

        #[getter(p_part)]
        pub fn profile_p_part(&self) -> Vec<u32> {
            self.0.p_part.clone()
        }

        #[setter(p_part)]
        pub fn set_p_part(&mut self, value: Vec<u32>) {
            self.0.p_part = value;
        }

        pub fn is_trivial(&self) -> bool {
            self.0.is_trivial()
        }

        pub fn get_p_part(&self, i: usize) -> u32 {
            self.0.get_p_part(i)
        }

        pub fn is_valid(&self) -> bool {
            self.0.is_valid()
        }

        pub fn is_an(&self, generic: bool) -> bool {
            self.0.is_an(generic)
        }

        pub fn __repr__(&self) -> String {
            format!(
                "MilnorProfile(truncated={}, q_part={}, p_part={:?})",
                self.0.truncated, self.0.q_part, self.0.p_part
            )
        }
    }

    #[pyclass]
    pub struct MilnorAlgebra(::algebra::MilnorAlgebra);

    impl MilnorAlgebra {
        /// Lazily compute book-keeping up to `degree`. The Milnor algebra is
        /// infinite-dimensional and its internal `OnceVec` tables panic when
        /// indexed past the computed range, so every degree-indexed Python
        /// method funnels through here first. `compute_basis` is idempotent and
        /// cheap to re-call, so this is a safe (if slightly eager) way to avoid
        /// cross-boundary panics; it is a no-op for negative degrees.
        fn ensure_basis(&self, degree: i32) {
            if degree >= 0 {
                self.0.compute_basis(degree);
            }
        }

        /// Validate two factor degrees and compute the (basis-populated) target
        /// degree of their product.
        fn product_target(&self, r_degree: i32, s_degree: i32) -> PyResult<i32> {
            non_negative_degree(r_degree)?;
            non_negative_degree(s_degree)?;
            let target = r_degree
                .checked_add(s_degree)
                .ok_or_else(|| PyValueError::new_err("product degree overflows i32"))?;
            self.ensure_basis(target);
            Ok(target)
        }

        fn checked_basis_index(&self, degree: i32, idx: usize) -> PyResult<()> {
            let dim = self.0.dimension(degree);
            if idx < dim {
                Ok(())
            } else {
                Err(PyIndexError::new_err(format!(
                    "index {idx} out of range for degree {degree} (dimension {dim})"
                )))
            }
        }
    }

    #[pymethods]
    impl MilnorAlgebra {
        #[new]
        #[pyo3(signature = (p, unstable_enabled = false))]
        pub fn new(p: u32, unstable_enabled: bool) -> PyResult<Self> {
            Ok(MilnorAlgebra(::algebra::MilnorAlgebra::new(
                valid_prime(p)?,
                unstable_enabled,
            )))
        }

        /// Construct a Milnor algebra restricted to the given profile. Raises
        /// `ValueError` for an invalid profile rather than panicking (upstream
        /// `new_with_profile` asserts validity).
        #[staticmethod]
        #[pyo3(signature = (p, profile, unstable_enabled = false))]
        pub fn new_with_profile(
            p: u32,
            profile: PyRef<'_, MilnorProfile>,
            unstable_enabled: bool,
        ) -> PyResult<Self> {
            let p = valid_prime(p)?;
            let profile = profile.to_rust();
            if !profile.is_valid() {
                return Err(PyValueError::new_err("invalid Milnor profile"));
            }
            Ok(MilnorAlgebra(::algebra::MilnorAlgebra::new_with_profile(
                p,
                profile,
                unstable_enabled,
            )))
        }

        // --- Algebra trait surface --------------------------------------------

        /// The prime as a plain `int` (`ValidPrime` is never exposed).
        pub fn prime(&self) -> u32 {
            self.0.prime().as_u32()
        }

        pub fn compute_basis(&self, degree: i32) {
            self.ensure_basis(degree);
        }

        pub fn dimension(&self, degree: i32) -> usize {
            if degree < 0 {
                return 0;
            }
            self.ensure_basis(degree);
            self.0.dimension(degree)
        }

        pub fn basis_element_to_string(&self, degree: i32, idx: usize) -> PyResult<String> {
            non_negative_degree(degree)?;
            self.ensure_basis(degree);
            self.checked_basis_index(degree, idx)?;
            Ok(self.0.basis_element_to_string(degree, idx))
        }

        /// Parse a basis element, returning `(degree, index)`. Raises
        /// `ValueError` if the string does not parse.
        pub fn basis_element_from_string(&self, elt: &str) -> PyResult<(i32, usize)> {
            self.0.basis_element_from_string(elt).ok_or_else(|| {
                PyValueError::new_err(format!("could not parse basis element: {elt}"))
            })
        }

        pub fn element_to_string(
            &self,
            py: Python<'_>,
            degree: i32,
            element: &Bound<'_, PyAny>,
        ) -> PyResult<String> {
            non_negative_degree(degree)?;
            self.ensure_basis(degree);
            let element = crate::fp_py::extract_input_owned(py, element)?;
            checked_same_prime(element.prime().as_u32(), self.0.prime().as_u32())?;
            checked_equal_len(element.len(), self.0.dimension(degree))?;
            Ok(self.0.element_to_string(degree, element.as_slice()))
        }

        pub fn multiply_basis_elements(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            r_degree: i32,
            r_idx: usize,
            s_degree: i32,
            s_idx: usize,
        ) -> PyResult<()> {
            let p = self.0.prime().as_u32();
            let target = self.product_target(r_degree, s_degree)?;
            let dim = self.0.dimension(target);
            self.checked_basis_index(r_degree, r_idx)?;
            self.checked_basis_index(s_degree, s_idx)?;
            crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                checked_same_prime(res.prime().as_u32(), p)?;
                checked_result_len(res.as_slice().len(), dim)?;
                self.0
                    .multiply_basis_elements(res.copy(), coeff, r_degree, r_idx, s_degree, s_idx);
                Ok(())
            })
        }

        pub fn multiply_basis_element_by_element(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            r_degree: i32,
            r_idx: usize,
            s_degree: i32,
            s: &Bound<'_, PyAny>,
        ) -> PyResult<()> {
            let p = self.0.prime().as_u32();
            let target = self.product_target(r_degree, s_degree)?;
            let dim = self.0.dimension(target);
            self.checked_basis_index(r_degree, r_idx)?;
            let s = crate::fp_py::extract_input_owned(py, s)?;
            checked_same_prime(s.prime().as_u32(), p)?;
            checked_equal_len(s.len(), self.0.dimension(s_degree))?;
            crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                checked_same_prime(res.prime().as_u32(), p)?;
                checked_result_len(res.as_slice().len(), dim)?;
                self.0.multiply_basis_element_by_element(
                    res.copy(),
                    coeff,
                    r_degree,
                    r_idx,
                    s_degree,
                    s.as_slice(),
                );
                Ok(())
            })
        }

        pub fn multiply_element_by_basis_element(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            r_degree: i32,
            r: &Bound<'_, PyAny>,
            s_degree: i32,
            s_idx: usize,
        ) -> PyResult<()> {
            let p = self.0.prime().as_u32();
            let target = self.product_target(r_degree, s_degree)?;
            let dim = self.0.dimension(target);
            self.checked_basis_index(s_degree, s_idx)?;
            let r = crate::fp_py::extract_input_owned(py, r)?;
            checked_same_prime(r.prime().as_u32(), p)?;
            checked_equal_len(r.len(), self.0.dimension(r_degree))?;
            crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                checked_same_prime(res.prime().as_u32(), p)?;
                checked_result_len(res.as_slice().len(), dim)?;
                self.0.multiply_element_by_basis_element(
                    res.copy(),
                    coeff,
                    r_degree,
                    r.as_slice(),
                    s_degree,
                    s_idx,
                );
                Ok(())
            })
        }

        pub fn multiply_element_by_element(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            r_degree: i32,
            r: &Bound<'_, PyAny>,
            s_degree: i32,
            s: &Bound<'_, PyAny>,
        ) -> PyResult<()> {
            let p = self.0.prime().as_u32();
            let target = self.product_target(r_degree, s_degree)?;
            let dim = self.0.dimension(target);
            let r = crate::fp_py::extract_input_owned(py, r)?;
            let s = crate::fp_py::extract_input_owned(py, s)?;
            checked_same_prime(r.prime().as_u32(), p)?;
            checked_same_prime(s.prime().as_u32(), p)?;
            checked_equal_len(r.len(), self.0.dimension(r_degree))?;
            checked_equal_len(s.len(), self.0.dimension(s_degree))?;
            crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                checked_same_prime(res.prime().as_u32(), p)?;
                checked_result_len(res.as_slice().len(), dim)?;
                self.0.multiply_element_by_element(
                    res.copy(),
                    coeff,
                    r_degree,
                    r.as_slice(),
                    s_degree,
                    s.as_slice(),
                );
                Ok(())
            })
        }

        pub fn default_filtration_one_products(&self) -> Vec<(String, i32, usize)> {
            self.0.default_filtration_one_products()
        }

        // --- GeneratedAlgebra trait surface -----------------------------------

        pub fn generators(&self, degree: i32) -> PyResult<Vec<usize>> {
            if degree < 0 {
                return Ok(Vec::new());
            }
            self.ensure_basis(degree);
            Ok(self.0.generators(degree))
        }

        pub fn generator_to_string(&self, degree: i32, idx: usize) -> PyResult<String> {
            non_negative_degree(degree)?;
            self.ensure_basis(degree);
            self.checked_basis_index(degree, idx)?;
            Ok(self.0.generator_to_string(degree, idx))
        }

        pub fn decompose_basis_element(
            &self,
            degree: i32,
            idx: usize,
        ) -> PyResult<Vec<(u32, (i32, usize), (i32, usize))>> {
            non_negative_degree(degree)?;
            self.ensure_basis(degree);
            self.checked_basis_index(degree, idx)?;
            Ok(self.0.decompose_basis_element(degree, idx))
        }

        pub fn generating_relations(
            &self,
            degree: i32,
        ) -> PyResult<Vec<Vec<(u32, (i32, usize), (i32, usize))>>> {
            if degree < 0 {
                return Ok(Vec::new());
            }
            self.ensure_basis(degree);
            Ok(self.0.generating_relations(degree))
        }

        // --- Bialgebra trait surface ------------------------------------------

        /// Compute a coproduct. Only supported at `p = 2` upstream; raises
        /// `ValueError` at odd primes rather than panicking on the assertion.
        pub fn coproduct(
            &self,
            degree: i32,
            idx: usize,
        ) -> PyResult<Vec<(i32, usize, i32, usize)>> {
            if self.0.prime().as_u32() != 2 {
                return Err(PyValueError::new_err(
                    "coproduct is only supported at p = 2",
                ));
            }
            non_negative_degree(degree)?;
            self.ensure_basis(degree);
            self.checked_basis_index(degree, idx)?;
            Ok(self.0.coproduct(degree, idx))
        }

        pub fn decompose(&self, degree: i32, idx: usize) -> PyResult<Vec<(i32, usize)>> {
            non_negative_degree(degree)?;
            self.ensure_basis(degree);
            self.checked_basis_index(degree, idx)?;
            Ok(self.0.decompose(degree, idx))
        }

        // --- Milnor-specific methods ------------------------------------------

        pub fn generic(&self) -> bool {
            self.0.generic()
        }

        pub fn q(&self) -> i32 {
            self.0.q()
        }

        pub fn profile(&self) -> MilnorProfile {
            let profile = self.0.profile();
            MilnorProfile(::algebra::milnor_algebra::MilnorProfile {
                truncated: profile.truncated,
                q_part: profile.q_part,
                p_part: profile.p_part.clone(),
            })
        }

        pub fn basis_element_from_index(
            &self,
            degree: i32,
            idx: usize,
        ) -> PyResult<MilnorBasisElement> {
            non_negative_degree(degree)?;
            self.ensure_basis(degree);
            self.checked_basis_index(degree, idx)?;
            Ok(MilnorBasisElement(
                self.0.basis_element_from_index(degree, idx).clone(),
            ))
        }

        pub fn try_basis_element_to_index(
            &self,
            elt: PyRef<'_, MilnorBasisElement>,
        ) -> Option<usize> {
            if elt.0.degree < 0 {
                return None;
            }
            self.ensure_basis(elt.0.degree);
            self.0.try_basis_element_to_index(&elt.0)
        }

        /// Like `try_basis_element_to_index`, but raises `ValueError` if the
        /// element is not in the algebra (upstream panics).
        pub fn basis_element_to_index(
            &self,
            elt: PyRef<'_, MilnorBasisElement>,
        ) -> PyResult<usize> {
            non_negative_degree(elt.0.degree)?;
            self.ensure_basis(elt.0.degree);
            self.0
                .try_basis_element_to_index(&elt.0)
                .ok_or_else(|| PyValueError::new_err(format!("element not in algebra: {}", elt.0)))
        }

        /// The list of `P(R)` partitions in degree `t`.
        pub fn ppart_table(&self, t: i32) -> PyResult<Vec<Vec<u32>>> {
            non_negative_degree(t)?;
            // The internal table is indexed by `degree / q`, so compute enough
            // book-keeping that index `t` is in range at every prime.
            let needed = t
                .checked_mul(self.0.q())
                .ok_or_else(|| PyValueError::new_err("degree overflows i32"))?;
            self.ensure_basis(needed);
            Ok(self.0.ppart_table(t).to_vec())
        }

        /// The degree and index of `Q_1^e P(x)`. Raises `ValueError` if that
        /// element is not in the (profiled) algebra (upstream panics).
        pub fn beps_pn(&self, e: u32, x: u32) -> PyResult<(i32, usize)> {
            let q = self.0.q() as u32;
            let degree = q
                .checked_mul(x)
                .and_then(|v| v.checked_add(e))
                .ok_or_else(|| PyValueError::new_err("degree overflows"))?
                as i32;
            self.ensure_basis(degree);
            let elt = ::algebra::milnor_algebra::MilnorBasisElement {
                degree,
                q_part: e,
                p_part: vec![x],
            };
            self.0
                .try_basis_element_to_index(&elt)
                .map(|idx| (degree, idx))
                .ok_or_else(|| {
                    PyValueError::new_err(format!("Q_1^{e} P({x}) is not in the algebra"))
                })
        }

        /// Multiply two `MilnorBasisElement`s, accumulating into `result`.
        pub fn multiply(
            &self,
            py: Python<'_>,
            result: &Bound<'_, PyAny>,
            coeff: u32,
            m1: PyRef<'_, MilnorBasisElement>,
            m2: PyRef<'_, MilnorBasisElement>,
        ) -> PyResult<()> {
            let p = self.0.prime().as_u32();
            let target = self.product_target(m1.0.degree, m2.0.degree)?;
            let dim = self.0.dimension(target);
            // Reject elements that are not genuine basis elements of this
            // algebra up front, since the inner multiply panics if an
            // intermediate term cannot be indexed.
            self.ensure_basis(m1.0.degree);
            self.ensure_basis(m2.0.degree);
            if self.0.try_basis_element_to_index(&m1.0).is_none() {
                return Err(PyValueError::new_err(format!(
                    "left factor is not a basis element of this algebra: {}",
                    m1.0
                )));
            }
            if self.0.try_basis_element_to_index(&m2.0).is_none() {
                return Err(PyValueError::new_err(format!(
                    "right factor is not a basis element of this algebra: {}",
                    m2.0
                )));
            }
            crate::fp_py::with_target_slice_mut(py, result, |mut res| {
                checked_same_prime(res.prime().as_u32(), p)?;
                checked_result_len(res.as_slice().len(), dim)?;
                self.0.multiply(res.copy(), coeff, &m1.0, &m2.0);
                Ok(())
            })
        }

        pub fn __repr__(&self) -> String {
            format!("{}", self.0)
        }
    }

    #[pymodule_init]
    fn init(_m: &Bound<'_, PyModule>) -> PyResult<()> {
        // Arbitrary code to run at the module initialization
        // m.add("double2", m.getattr("double")?)
        Ok(())
    }
}
