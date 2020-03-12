use pyo3::{
    prelude::*,
    PyObjectProtocol,
    exceptions,
    types::{PyDict, PyAny, },
};

use python_utils;
use python_utils::{
    py_repr, 
    rc_wrapper_type,
    // wrapper_type, 
    immutable_wrapper_type,
    get_from_kwargs,
};

use fp::vector::FpVectorT;
use python_fp::vector::FpVector;
use python_fp::prime::new_valid_prime;

use algebra::MilnorAlgebra as MilnorAlgebraRust;
use algebra::milnor_algebra::MilnorBasisElement as MilnorBasisElementRust;
use algebra::milnor_algebra::MilnorProfile as MilnorProfileRust;
use algebra::Algebra;


use crate::algebra_bindings::{ self, PVector };
use crate::algebra_rust::AlgebraRust;

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
        let q_part = algebra_bindings::bitmask_u32_from_py_object(qs, "qs")?;
        let p_part = algebra_bindings::vecu32_from_py_object(ps, "ps")?;

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
    pub fn get_qpart(&self) -> PyResult<PVector> {
        Ok(PVector::box_and_wrap(algebra_bindings::bitmask_u32_to_vec(self.inner()?.q_part)))
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
    self.name()
});


#[pymethods]
impl MilnorProfile {
    #[new]
    fn new(truncated : bool, qs : PyObject, ps : PyObject) -> PyResult<Self> {
        let q_part = algebra_bindings::bitmask_u32_from_py_object(qs, "qs")?;
        let p_part = algebra_bindings::vecu32_from_py_object(ps, "ps")?;
        Ok(Self::box_and_wrap(MilnorProfileRust {
            truncated,
            q_part,
            p_part
        }))
    }

    pub fn name(&self) -> PyResult<String> {
        let inner = self.inner()?;
        if inner.is_trivial() {
            return Ok("MilnorProfile()".to_string())
        }
        let mut p_part_str = "".to_string(); 
        if !inner.p_part.is_empty() {
            p_part_str = format!(", p_part={:?}", inner.p_part)
        } 
        let mut q_part_str = "".to_string(); 
        if inner.q_part != !0 {
            q_part_str = format!(", q_part={:?}", algebra_bindings::bitmask_u32_to_vec(inner.q_part))
        }
        let truncated_str = 
            if inner.truncated {
                "truncated=True"
            } else {
                "truncated=False"
            };
    
        Ok(format!(
            "MilnorProfile({}{}{})",
            truncated_str,
            p_part_str,
            q_part_str
        ))
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


crate::algebra_bindings!(MilnorAlgebra, MilnorAlgebraRust, MilnorElement, "MilnorElement");

py_algebra_repr!(MilnorAlgebra, "FreedMilnorAlgebra", {
    let p = *inner.prime();
    let mut generic_str = "";
    if inner.generic != (p!=2) {
        if inner.generic {
            generic_str = ", generic=True";
        } else {
            generic_str = ", generic=False";
        }
    }
    let mut profile_str = "".to_string();
    if !inner.profile.is_trivial() {
        profile_str = format!(", {}", 
            MilnorProfile::wrap(&inner.profile, self.owner()).name()?
        );
    }
    Ok(format!(
        "MilnorAlgebra(p={}{}{})",
        p,
        generic_str,
        profile_str
    ))
});

pub fn get_profile_from_kwargs(p : u32, kwargs : Option<&PyDict>) -> PyResult<MilnorProfileRust> {
    let truncated = get_from_kwargs(kwargs, "truncated", false)?;
    let mut q_part = !0;
    let p_part : Vec<u32>;
    if p == 2 {
        p_part = get_from_kwargs(kwargs, "profile", vec![])?;
    } else if let Some(x) = 
            kwargs.and_then(|dict| dict.get_item("profile"))
                  .map(|value| PyAny::extract::<Vec<Vec<u32>>>(value)) 
    {
        let profile = x?;
        if profile.len() != 2 {
            return Err(exceptions::ValueError::py_err(
                "For generic MilnorAlgebra profile argument should be a pair of lists [p_part, q_part]."
            ));
        }
        p_part = profile[0].clone();
        q_part = algebra_bindings::bitmask_u32_from_vec(&profile[1]);
    } else {
        p_part = vec![];
    }
    Ok(MilnorProfileRust {
        truncated,
        q_part,
        p_part
    })
}


#[pymethods]
impl MilnorAlgebra {
    #[new]
    #[args(kwargs="**")]
    pub fn new(p : u32, kwargs : Option<&PyDict>) -> PyResult<Self> {
        let mut algebra = MilnorAlgebraRust::new(new_valid_prime(p)?);
        let profile = get_profile_from_kwargs(p, kwargs)?;
        algebra.profile = profile;
        Ok(Self::box_and_wrap(AlgebraRust::MilnorAlgebraRust(algebra)))
    }

    #[getter]
    pub fn get_truncated(&self) -> PyResult<bool> {
        Ok(self.inner_algebra()?.profile.truncated)
    }

    #[getter]
    pub fn get_profile(&self) -> PyResult<MilnorProfile> {
        Ok(MilnorProfile::wrap(&self.inner_algebra()?.profile, self.owner()))
    }    

    pub fn basis_element_from_index(&self, degree : i32, idx : usize) -> PyResult<MilnorBasisElement> {
        self.check_not_null()?;
        self.check_degree(degree)?;
        self.check_index(degree, idx)?;
        Ok(MilnorBasisElement::wrap(self.inner_algebra_unchkd().basis_element_from_index(degree, idx), self.owner()))
    }

    pub fn basis_element_to_index(&self, elt: &MilnorBasisElement) -> PyResult<usize> {
        let mbe_inner = elt.inner()?;
        self.check_not_null()?;
        self.check_degree(mbe_inner.degree)?;
        self.inner_algebra_unchkd().try_basis_element_to_index(mbe_inner)
            .ok_or_else(|| 
                exceptions::ValueError::py_err(format!(
                    "MilnorBasisElement({}) is not a valid basis element.", 
                    mbe_inner
                ))
            )
    }
}
