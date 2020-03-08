#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]

use pyo3::{
    prelude::*,
    PyObjectProtocol,
    PyObject,
    exceptions,
    types::{PyDict, PyAny, },
};

use std::sync::Arc;

use fp::prime::ValidPrime;
use fp::vector::{FpVector as FpVectorRust, FpVectorT};
use algebra::Algebra;

use python_fp::prime::new_valid_prime;
use python_fp::vector::FpVector;


pub struct PythonAlgebraRust {
    prime : ValidPrime,
    compute_basis : PyObject,
    get_dimension : PyObject,
    multiply_basis_elements : PyObject
}

use python_utils::{
    self,
    py_repr, 
    rc_wrapper_type, 
    // wrapper_type, 
    // immutable_wrapper_type,
    // get_from_kwargs
};

rc_wrapper_type!(PythonAlgebra, PythonAlgebraRust);

py_repr!(PythonAlgebra, "FreedPythonAlgebra", {
    Ok(format!(
        "PythonAlgebra(p={})",
        *inner.prime
    ))
});

crate::algebra_bindings!(PythonAlgebra, PythonElement, "PythonElement");

#[pymethods]
impl PythonAlgebra {
    #[new]
    #[args(p, "*", compute_basis, get_dimension, multiply_basis_elements, basis_element_to_string)]
    fn new(p : u32, 
        compute_basis : PyObject,
        get_dimension : PyObject,
        multiply_basis_elements : PyObject,
        basis_element_to_string : PyObject
    ) -> PyResult<Self> {
        Ok(Self::box_and_wrap(PythonAlgebraRust {
            prime : new_valid_prime(p)?,
            compute_basis,
            get_dimension,
            multiply_basis_elements
        }))
    }
}

impl Algebra for PythonAlgebraRust {
    fn algebra_type(&self) -> &str {
        "PythonAlgebra"
    }

    fn prime(&self) -> ValidPrime {
        self.prime
    }
    
    fn compute_basis(&self, degree : i32) {
        let gil = Python::acquire_gil();
        let py = gil.python();
        drop(self.compute_basis.call1(py, (degree,)));
    }

    fn max_degree(&self) -> i32 {
        i32::max_value()
    }

    fn dimension(&self, degree : i32, excess : i32) -> usize {
        let gil = Python::acquire_gil();
        let py = gil.python();
        self.get_dimension.call1(py, (degree,excess)).unwrap()
            .extract(py).unwrap()
    }

    fn multiply_basis_elements(&self, result : &mut FpVectorRust, coeff : u32, r_degree : i32, r_idx : usize, s_degree: i32, s_idx : usize, excess : i32){
        let gil = Python::acquire_gil();
        let py = gil.python();
        let temp_arc = Arc::new(());
        self.multiply_basis_elements.call1(py,
                (FpVector::wrap(result, Arc::downgrade(&temp_arc)), coeff, r_degree, r_idx, s_degree, s_idx, excess)
            ).unwrap();
    }

    fn default_filtration_one_products(&self) -> Vec<(String, i32, usize)> { Vec::new() }

    fn basis_element_to_string(&self, degree : i32, idx : usize) -> String {
        format!("a_{{{}, {}}}", degree, idx)
    }

}