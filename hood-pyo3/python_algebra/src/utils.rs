#![macro_use]

use pyo3::prelude::*;
use pyo3::{PySequenceProtocol, PyObjectProtocol};
use pyo3::exceptions;

// use fp::vector::FpVectorT;

use python_utils;
use python_utils::immutable_wrapper_type;

immutable_wrapper_type!(PVector, Vec<u32>);


python_utils::py_repr!(PVector, "FreedPVector", {
    Ok(format!(
        "PVector({:?})",
        inner
    ))
});


pub fn vecu32_from_py_object(obj : PyObject, argument_name : &str) -> PyResult<Vec<u32>> {
    let gil = Python::acquire_gil();
    let py = gil.python();
    obj.extract(py).or_else(|_err| {
        let result : &PVector = obj.extract(py)?;
        Ok(result.inner()?.clone())
    }).map_err(|_err : PyErr| {
        exceptions::ValueError::py_err(format!(
            "Argument \"{}\" expected to be either a list of integers or a PVector.",
            argument_name
        ))
    })
}

pub fn bitmask_u32_from_py_object(obj : PyObject, argument_name : &str) -> PyResult<u32> {
    let gil = Python::acquire_gil();
    let py = gil.python();
    obj.extract::<u32>(py).or_else(|_err| {
        let a : Vec<u32> = obj.extract(py)?;
        let mut result = 0;
        // TODO: make sure we get ordering correct here
        for (idx, b) in a.iter().enumerate() {
            result |= if *b != 0 { 1 << idx } else { 0 };
        }
        Ok(result)
    }).map_err(|_err : PyErr| {
        exceptions::ValueError::py_err(format!(
            "Argument \"{}\" expected to be either a single integer or a list of integers.",
            argument_name
        ))
    })
}


#[pymethods]
impl PVector {
    #[new]
    fn new(l : PyObject) -> PyResult<Self> {
        let gil = Python::acquire_gil();
        let py = gil.python();
        let vec : Vec<u32> = l.extract(py)?;
        Ok(Self::box_and_wrap(vec))
    }

    fn to_list(&self) -> PyResult<PyObject> {
        let gil = Python::acquire_gil();
        let py = gil.python();
        Ok(self.inner()?.clone().into_py(py))
    }

    fn check_index(&self, index : isize) -> PyResult<()> {
        python_utils::check_index(self.inner_unchkd().len(), index, "length", "PVector")
    }
}

#[pyproto]
impl PySequenceProtocol for PVector {
    fn __len__(self) -> PyResult<usize> {
        Ok(self.inner()?.len())
    }

    fn __getitem__(self, index : isize) -> PyResult<u32> {
        self.check_not_null()?;
        self.check_index(index)?;
        Ok(self.inner_unchkd()[index as usize])
    }
}


#[macro_export]
macro_rules! algebra_bindings { ( $algebra:ty, $element : ident, $element_name : expr ) => {

    impl $algebra {
        fn check_degree(&self, degree : i32) -> PyResult<()> {
            let next_degree = self.inner_unchkd().next_degree();
            if degree >= next_degree {
                Err(exceptions::IndexError::py_err(
                    format!("Degree {} too large: maximum degree of algebra is {}", degree, next_degree - 1)
                ))
            } else {
                Ok(())
            }
        }

        fn check_dimension(&self, degree : i32, vec : &FpVector) -> PyResult<()> {
            let what_the_dimension_should_be = self.inner_unchkd().dimension(degree, -1);
            let the_dimension = vec.dimension()?;
            if the_dimension == what_the_dimension_should_be {
                Ok(())
            } else {
                Err(exceptions::ValueError::py_err(format!(
                    "Dimension of vector is {} but the dimension of the algebra in degree {} is {}.",
                    the_dimension,
                    degree,
                    what_the_dimension_should_be
                )))
            }
        }

        fn check_index(&self, degree : i32, idx : usize) -> PyResult<()> {
            let dimension = self.inner_unchkd().dimension(degree, -1);
            if idx < dimension {
                Ok(())
            } else {
                Err(exceptions::IndexError::py_err(format!(
                    "Index {} is larger than dimension {} of the algebra in degree {}.",
                    idx,
                    dimension,
                    degree,
                )))                
            }
            
        }
    }

    #[pymethods]
    impl $algebra {
        pub fn algebra_type(&self) -> PyResult<&str> {
            Ok(self.inner()?.algebra_type())
        }

        pub fn prime(&self) -> PyResult<u32> {
            Ok(*self.inner()?.prime())
        }

        pub fn name(&self) -> PyResult<&str> {
            Ok(self.inner()?.name())
        }

        pub fn compute_basis(&self, max_degree : i32) -> PyResult<()> {
            self.inner()?.compute_basis(max_degree);
            Ok(())
        }

        pub fn dimension(&self, degree : i32, excess : i32) -> PyResult<usize> {
            self.check_not_null()?;
            self.check_degree(degree)?;
            Ok(self.inner_unchkd().dimension(degree, excess))
        }

        pub fn multiply_basis_elements(&self, 
            result : &mut FpVector, coeff : u32, 
            r_degree : i32, r_index : usize, 
            s_degree : i32, s_index : usize, excess : i32
        ) -> PyResult<()> {
            self.check_not_null()?;
            self.check_degree(r_degree + s_degree)?;
            self.check_index(r_degree, r_index)?;
            self.check_index(s_degree, s_index)?;
            self.inner_unchkd().multiply_basis_elements(result.inner_mut()?, coeff, r_degree, r_index, s_degree, s_index, excess);
            Ok(())
        }

        pub fn multiply_basis_element_by_element(&self, 
            result : &mut FpVector, coeff : u32, 
            r_degree : i32, r_index : usize, 
            s_degree : i32, s : &FpVector, 
            excess : i32
        ) -> PyResult<()>{
            self.check_not_null()?;
            self.check_degree(r_degree + s_degree)?;
            self.check_index(r_degree, r_index)?;
            self.check_dimension(s_degree, s)?;
            self.inner_unchkd().multiply_basis_element_by_element(
                result.inner_mut()?, coeff, 
                r_degree, r_index,
                s_degree, s.inner()?,
                excess
            );
            Ok(())
        }

        pub fn multiply_element_by_basis_element(&self, 
            result : &mut FpVector, coeff : u32, 
            r_degree : i32, r : &FpVector, 
            s_degree : i32, s_index : usize, 
            excess : i32
        ) -> PyResult<()> {
            self.check_not_null()?;
            self.check_degree(r_degree + s_degree)?;
            self.check_dimension(r_degree, r)?;
            self.check_index(s_degree, s_index)?;
            self.inner_unchkd().multiply_element_by_basis_element(
                result.inner_mut()?, coeff, 
                r_degree, r.inner()?,
                s_degree, s_index,
                excess
            );
            Ok(())
        }

        pub fn multiply_element_by_element(&self, 
            result : &mut FpVector, coeff : u32, 
            r_degree : i32, r : &FpVector, 
            s_degree : i32, s : &FpVector, 
            excess : i32
        ) -> PyResult<()> {
            self.check_not_null()?;
            self.check_degree(r_degree + s_degree)?;
            self.check_dimension(r_degree, r)?;
            self.check_dimension(s_degree, s)?;
            self.inner_unchkd().multiply_element_by_element(
                result.inner_mut()?, coeff, 
                r_degree, r.inner()?,
                s_degree, s.inner()?,
                excess
            );
            Ok(())
        }

        pub fn default_filtration_one_products(&self) -> PyResult<PyObject> {
            let gil = Python::acquire_gil();
            let py = gil.python();
            Ok(self.inner()?.default_filtration_one_products().into_py(py))
        }

        pub fn basis_element_to_string(&self, degree : i32, idx : usize) -> PyResult<String> {
            self.check_not_null()?;
            self.check_degree(degree)?;
            self.check_index(degree, idx)?;
            Ok(self.inner_unchkd().basis_element_to_string(degree, idx))
        }
        
        pub fn element_to_string(&self, degree : i32, element : &FpVector) -> PyResult<String> {
            self.check_not_null()?;
            self.check_degree(degree)?;
            element.check_not_null()?;
            self.check_dimension(degree, element)?;
            Ok(self.inner_unchkd().element_to_string(degree, element.inner_unchkd()))
        }
        
        // fn generator_to_string(&self, degree: i32, idx: usize) -> String
        // fn string_to_generator<'a, 'b>(&'a self, _input: &'b str) -> IResult<&'b str, (i32, usize)>

        pub fn decompose_basis_element(&self, degree : i32, idx : usize) -> PyResult<PyObject> {
            self.check_not_null()?;
            self.check_degree(degree)?;
            self.check_index(degree, idx)?;
            let gil = Python::acquire_gil();
            let py = gil.python();
            Ok(self.inner_unchkd().decompose_basis_element(degree, idx).into_py(py))
        }

        pub fn relations_to_check(&self, degree : i32) -> PyResult<PyObject> {
            self.check_not_null()?;
            self.check_degree(degree)?;            
            let gil = Python::acquire_gil();
            let py = gil.python();
            Ok(self.inner_unchkd().relations_to_check(degree).into_py(py))
        }
    }

    #[pyclass(dict)]
    pub struct $element {
        algebra : $algebra,
        degree : i32,
        element : FpVector
    }
    
    #[pyproto]
    impl PyObjectProtocol for $element {
        fn __repr__(&self) -> PyResult<String> {
            self.algebra.element_to_string(self.degree, &self.element)
        }
    }
    
    impl $element {
        fn obj_to_vec(obj : PyObject, argument_name : &str) -> PyResult<FpVector> {
            let gil = Python::acquire_gil();
            let py = gil.python();
            Ok(obj.extract::<&FpVector>(py).or_else(|_err| {
                Ok(&obj.extract::<&$element>(py)?.element)
            }).map_err(|_err : PyErr| {
                exceptions::ValueError::py_err(format!(
                    "Argument \"{}\" expected to be either an {} or an FpVector.",
                    $element_name,
                    argument_name
                ))
            })?.clone())
        }
    }
    
    #[pymethods]
    impl $element {
        #[getter]
        fn get_vec(&self) -> FpVector {
            self.element.clone()
        }
    
        fn add(&mut self, other : PyObject, c : i32) -> PyResult<()> {
            self.element.add(&$element::obj_to_vec(other, "other")?, c)
        }
    
        fn multiply_add(&mut self, left : &$element, right : &$element, coeff : i32) -> PyResult<()> {
            let coeff = python_utils::reduce_coefficient(self.algebra.prime()?, coeff);
            self.algebra.multiply_element_by_element(&mut self.element, coeff, 
                left.degree, &left.element, 
                right.degree, &right.element, 
                -1
            )
        }
    }
    
    #[pymethods]
    impl $algebra {
        fn new_element(&self, degree : i32) -> PyResult<$element> {
            Ok($element {
                algebra : self.clone(),
                degree,
                element : FpVector::new(self.prime()?, self.dimension(degree, -1)?)?
            })
        }
    
        fn element_from_vec(&self, degree : i32, v : &FpVector) -> PyResult<$element> {
            Ok($element {
                algebra : self.clone(),
                degree,
                element : v.clone()
            })
        }
    }    
}}