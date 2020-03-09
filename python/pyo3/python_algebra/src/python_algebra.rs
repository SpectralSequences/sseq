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
use crate::algebra::AlgebraRust;


pub struct PythonAlgebraRust {
    pub prime : ValidPrime,
    pub compute_basis : PyObject,
    pub get_dimension : PyObject,
    pub multiply_basis_elements : PyObject,
    pub basis_element_to_string : PyObject
}

use python_utils::{
    self,
    py_repr, 
    rc_wrapper_type, 
    // wrapper_type, 
    // immutable_wrapper_type,
    // get_from_kwargs
};

py_algebra_repr!(PythonAlgebra, "FreedPythonAlgebra", {
    Ok(format!(
        "PythonAlgebra(p={})",
        *inner.prime
    ))
});

crate::algebra_bindings!(PythonAlgebra, PythonAlgebraRust, PythonElement, "PythonElement");

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
        let algebra = PythonAlgebraRust {
            prime : new_valid_prime(p)?,
            compute_basis,
            get_dimension,
            multiply_basis_elements,
            basis_element_to_string
        };
        Ok(Self::box_and_wrap(AlgebraRust::PythonAlgebraRust(algebra)))
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
        if self.compute_basis.is_none(){
            return;
        } else {
            let gil = Python::acquire_gil();
            let py = gil.python();            
            let result = self.compute_basis.call1(py, (degree,));
            if let Err(e) = result {
                eprintln!("Error occurred in call compute_basis({}):",
                    degree
                );
                e.print(py);
                panic!();
            }
        }
    }

    fn max_degree(&self) -> i32 {
        i32::max_value()
    }

    fn dimension(&self, degree : i32, excess : i32) -> usize {
        let gil = Python::acquire_gil();
        let py = gil.python();
        let result = self.get_dimension.call1(py, (degree,excess)).unwrap()
            .extract(py);
        match result {
            Err(e) => {
                eprintln!("Error occurred in call dimension({}, {}):",
                    degree, excess
                );
                e.print(py);
                panic!();
            },
            Ok(value) => value
        }
    }

    fn multiply_basis_elements(&self, result : &mut FpVectorRust, coeff : u32, r_degree : i32, r_idx : usize, s_degree: i32, s_idx : usize, excess : i32){
        let gil = Python::acquire_gil();
        let py = gil.python();
        let temp_arc = Arc::new(());
        let result = self.multiply_basis_elements.call1(py,
                (FpVector::wrap(result, Arc::downgrade(&temp_arc)), coeff, r_degree, r_idx, s_degree, s_idx, excess)
            );
        if let Err(e) = result {
            eprintln!("Error occurred in call multiply_basis_elements(result, {}, {}, {}, {}, {}, {}):",
                coeff, r_degree, r_idx, s_degree, s_idx, excess
            );
            e.print(py);
            panic!();
        }
    }

    fn default_filtration_one_products(&self) -> Vec<(String, i32, usize)> { Vec::new() }

    fn basis_element_to_string(&self, degree : i32, idx : usize) -> String {
        let gil = Python::acquire_gil();
        let py = gil.python();
        let result = self.basis_element_to_string.call1(py, (degree, idx))
            .and_then(|r| r.extract::<String>(py));
        match result {
            Err(e) => {
                eprintln!("Error occurred in call basis_element_to_string({}, {}):",
                    degree, idx
                );
                e.print(py);
                panic!();
            },
            Ok(value) => value
        }
    }

}