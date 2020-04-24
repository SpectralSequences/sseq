use pyo3::{
    prelude::*,
    PyObjectProtocol,
    types::PyDict
};

use python_utils::{
    self,
    py_repr, 
    // rc_wrapper_type, 
    // wrapper_type, 
    wrapper_type,
    get_from_kwargs
};

use fp::vector::FpVectorT;
use python_fp::vector::FpVector;
use python_fp::prime::new_valid_prime;

use algebra::AdemAlgebra as AdemAlgebraRust;
use algebra::adem_algebra::AdemBasisElement as AdemBasisElementRust;
use algebra::Algebra;

use crate::algebra::{
    algebra_bindings::{ self, PVector },
    algebra_rust::AlgebraRust
};

wrapper_type!(AdemBasisElement, AdemBasisElementRust);

py_repr!(AdemBasisElement, "FreedAdemBasisElement", {
    Ok(format!(
        "AdemBasisElement({})",
        inner
    ))
});


#[pymethods]
impl AdemBasisElement {
    #[new]
    fn new(degree : i32, excess : i32, bocksteins : PyObject, ps : PyObject) -> PyResult<Self> {
        let bs = algebra_bindings::bitmask_u32_from_py_object(bocksteins, "bocksteins")?;
        let ps_vec = algebra_bindings::vecu32_from_py_object(ps, "ps")?;

        Ok(Self::box_and_wrap(AdemBasisElementRust {
            degree,
            excess,
            bocksteins : bs,
            ps : ps_vec,
            p_or_sq : true
        }))
    }

    #[getter]
    pub fn get_degree(&self) -> PyResult<i32> {
        Ok(self.inner()?.degree)
    }

    #[getter]
    pub fn get_excess(&self) -> PyResult<i32> {
        Ok(self.inner()?.excess)
    }

    #[getter]
    pub fn get_bocksteins(&self) -> PyResult<u32> {
        Ok(self.inner()?.bocksteins)
    }

    #[getter]
    pub fn get_ps(&self) -> PyResult<PVector>{
        Ok(PVector::wrap_immutable(&self.inner()?.ps, self.owner()))
    }
    // pub fn to_python(&self) -> PyResult<PyObject> {

    // }
}


crate::algebra_bindings!(AdemAlgebra, AdemAlgebraRust, AdemElement, "AdemElement");

py_repr!(AdemAlgebra, "FreedAdemAlgebra", {
    let p = *inner.prime();
    let mut generic_str = "";    
    if inner.generic != (p!=2) {
        if inner.generic {
            generic_str = ", generic=True";
        } else {
            generic_str = ", generic=False";
        }
    }
    let mut unstable_str = "";
    if inner.unstable {
        unstable_str = ", unstable=True";
    }    
    Ok(format!(
        "AdemAlgebra(p={}{}{})",
        inner.prime(),
        generic_str,
        unstable_str
    ))
});



#[pymethods]
impl AdemAlgebra {
    #[new]
    #[args("*", unstable=false,  unstable_enabled=true, kwargs="**")]
    pub fn new(p : u32,  unstable : bool, mut unstable_enabled : bool, kwargs: Option<&PyDict>) -> PyResult<Self> {
        if unstable {
            unstable_enabled = true;
        }
        let generic : bool = get_from_kwargs(kwargs, "generic", p!=2)?;
        // if unstable {
        //     return Err(python_utils::exception!(NotImplementedError,
        //         "Unstable Adem algebras not yet implemented."
        //     ));
        // }
        let algebra = AdemAlgebraRust::new(new_valid_prime(p)?, generic, unstable, unstable_enabled);
        let mut result = Self::box_and_wrap(algebra);
        result.freeze().unwrap_or_else(|_err| unreachable!());
        Ok(result)
    }

    #[getter]
    pub fn get_generic(&self) -> PyResult<bool> {
        Ok(self.inner()?.generic)
    }

    #[getter]
    pub fn get_unstable(&self) -> PyResult<bool> {
        Ok(self.inner()?.unstable)
    }

    #[getter]
    pub fn get_unstable_enabled(&self) -> PyResult<bool> {
        Ok(self.inner()?.unstable_enabled)
    }

    pub fn dimension_unstable(&self, degree : i32, excess : i32) -> PyResult<usize> {
        Ok(self.inner()?.dimension_unstable(degree, excess))
    }

    pub fn multiply_basis_elements_unstable(&self, result : &mut FpVector, coeff : u32, 
        r_degree : i32, r_index : usize, 
        s_degree : i32, s_index : usize, excess : i32
    ) -> PyResult<()> {
        self.inner()?.multiply_basis_elements_unstable(result.inner_mut()?, coeff, r_degree, r_index, s_degree, s_index, excess, &|_,_|true);
        Ok(())
    }

    pub fn basis_element_from_index(&self, degree : i32, idx : usize) -> PyResult<AdemBasisElement> {
        self.check_not_null()?;
        self.check_degree(degree)?;
        self.check_index(degree, idx)?;
        Ok(AdemBasisElement::wrap_immutable(self.inner_unchkd().basis_element_from_index(degree, idx), self.owner()))
    }

    pub fn basis_element_to_index(&self, elt: &AdemBasisElement) -> PyResult<usize> {
        let abe_inner = elt.inner()?;
        self.check_not_null()?;
        self.check_degree(abe_inner.degree)?;
        self.inner_unchkd().try_basis_element_to_index(abe_inner)
            .ok_or_else(|| 
                python_utils::exception!(ValueError,
                    "AdemBasisElement({}) is not a valid basis element.", 
                    abe_inner
                )
            )
    }


    #[args(excess=65535, unstable=false)]
    pub fn make_mono_admissible(&self, result : &mut FpVector, coeff : u32,
        monomial : &mut AdemBasisElement, excess : i32, unstable : bool
    ) -> PyResult<()> {
        let mut monomial_inner = monomial.inner()?.clone();
        self.check_not_null()?;
        self.check_degree(monomial_inner.degree)?;
        // TODO: this is insufficient to prevent a panic: we would need validity checking on monomial.
        // What if it is lying about its degree?
        // Should add check_reduced_monomial() and check_not_necessarily_reduced_monomial()?
        self.inner_unchkd().make_mono_admissible(result.inner_mut()?, coeff, &mut monomial_inner, excess, unstable, &|_,_|true);
        Ok(())
    }
}
