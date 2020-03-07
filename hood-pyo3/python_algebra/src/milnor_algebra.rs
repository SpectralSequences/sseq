use pyo3::prelude::*;
use pyo3::PyObjectProtocol;
use pyo3::exceptions;

use python_utils;
use python_utils::{py_repr, wrapper_type, immutable_wrapper_type};

use python_fp::vector::FpVector;
use python_fp::prime::new_valid_prime;

use algebra::MilnorAlgebra as MilnorAlgebraRust;
use algebra::milnor_algebra::MilnorBasisElement as MilnorBasisElementRust;
use algebra::milnor_algebra::MilnorProfile as MilnorProfileRust;
use algebra::Algebra;

use crate::utils::{ self, PVector };


immutable_wrapper_type!(MilnorBasisElement, MilnorBasisElementRust);

py_repr!(MilnorBasisElement, "FreedMilnorBasisElement", {
    Ok(format!(
        "MilnorBasisElement({})",
        inner
    ))
});


#[pymethods]
impl MilnorBasisElement {
    #[new]
    fn new(degree : i32, qs : PyObject, ps : PyObject) -> PyResult<Self> {
        let q_part = utils::bitmask_u32_from_py_object(qs, "qs")?;
        let p_part = utils::vecu32_from_py_object(ps, "ps")?;

        Ok(Self::box_and_wrap(MilnorBasisElementRust {
            degree,
            q_part,
            p_part
        }))
    }

    #[getter]
    pub fn get_degree(&self) -> PyResult<i32> {
        Ok(self.inner()?.degree)
    }

    #[getter]
    pub fn get_qpart(&self) -> PyResult<u32> {
        Ok(self.inner()?.q_part)
    }

    #[getter]
    pub fn get_ppart(&self) -> PyResult<PVector>{
        Ok(PVector::wrap(&self.inner()?.p_part, self.owner()))
    }

    // pub fn to_python(&self) -> PyResult<PyObject> {

    // }
}

immutable_wrapper_type!(MilnorProfile, MilnorProfileRust);

py_repr!(MilnorProfile, "FreedMilnorProfile", {
    Ok(format!(
        "MilnorBasisElement()",
        // inner
    ))
});


#[pymethods]
impl MilnorProfile {
    #[new]
    fn new(truncated : bool, qs : PyObject, ps : PyObject) -> PyResult<Self> {
        let q_part = utils::bitmask_u32_from_py_object(qs, "qs")?;
        let p_part = utils::vecu32_from_py_object(ps, "ps")?;
        Ok(Self::box_and_wrap(MilnorProfileRust {
            truncated,
            q_part,
            p_part
        }))
    }

    #[getter]
    pub fn get_truncated(&self) -> PyResult<bool> {
        Ok(self.inner()?.truncated)
    }

    #[getter]
    pub fn get_qpart(&self) -> PyResult<u32> {
        Ok(self.inner()?.q_part)
    }

    #[getter]
    pub fn get_ppart(&self) -> PyResult<PVector>{
        Ok(PVector::wrap(&self.inner()?.p_part, self.owner()))
    }
}

wrapper_type!(MilnorAlgebra, MilnorAlgebraRust);

py_repr!(MilnorAlgebra, "FreedMilnorAlgebra", {
    Ok(format!(
        "{}",
        inner.name()
    ))
});


#[pymethods]
impl MilnorAlgebra {
    #[new]
    pub fn new(p : u32) -> PyResult<Self> {
        Ok(Self::box_and_wrap(MilnorAlgebraRust::new(new_valid_prime(p)?)))
    }

    pub fn basis_element_from_index(&self, degree : i32, idx : usize) -> PyResult<MilnorBasisElement> {
        self.check_not_null()?;
        self.check_degree(degree)?;
        self.check_index(degree, idx)?;
        Ok(MilnorBasisElement::wrap(self.inner_unchkd().basis_element_from_index(degree, idx), self.owner()))
    }

    pub fn basis_element_to_index(&self, elt: &MilnorBasisElement) -> PyResult<usize> {
        let mbe_inner = elt.inner()?;
        self.check_not_null()?;
        self.check_degree(mbe_inner.degree)?;
        self.inner_unchkd().try_basis_element_to_index(mbe_inner)
            .ok_or_else(|| 
                exceptions::ValueError::py_err(format!(
                    "MilnorBasisElement({}) is not a valid basis element.", 
                    mbe_inner
                ))
            )
    }
}

crate::algebra_bindings!(MilnorAlgebra, MilnorElement, "MilnorElement");