#![macro_use]

#[macro_export]
macro_rules! module_bindings { ( $algebra:ident, $algebra_rust:ident, $element : ident, $element_name : expr ) => {


    #[pyclass(dict)]
    #[derive(Clone)]
    #[repr(transparent)]
    pub struct $algebra {
        inner : Option<std::sync::Arc<AlgebraRust>>
    }

    impl $algebra {
        // type Inner = $inner; // ==> "associated types are not yet supported in inherent imples" =(

        #![allow(dead_code)]
        pub fn inner_rc(&self) -> PyResult<&std::sync::Arc<AlgebraRust>> {
            self.inner.as_ref().ok_or_else(
                || python_utils::null_ptr_exception()
            )
        }
    
        pub fn inner_rc_unchkd(&self) -> &std::sync::Arc<$inner> {
            self.inner().unwrap()
        }

        pub fn inner(&self) -> PyResult<&$algebra_rust> {
            match &**self.inner()? {
                AlgebraRust::$algebra_rust(alg) => Ok(&alg),
                _ => panic!()
            }
        }

        pub fn inner_unchkd(&self) -> &$algebra_rust {
            match &**self.inner_unchkd() {
                AlgebraRust::$algebra_rust(alg) => &alg,
                _ => panic!()
            }
        }        
    
        pub fn box_and_wrap(inner : $algebra_rust) -> Self {
            Self {
                inner : Some(std::sync::Arc::new(AlgebraRust::$algebra_rust(inner)))
            }
        }

        pub fn owner(&self) -> std::sync::Weak<()> {
            self.inner.as_ref().map(|ptr| 
                python_utils::weak_ptr_to_final(std::sync::Arc::downgrade(ptr))
            ).unwrap_or_else(|| std::sync::Weak::new()) 
                // TODO: this else behavior may not be right...
        }

        pub fn is_null(&self) -> bool {
            self.inner.is_none()
        }

        pub fn check_not_null(&self) -> PyResult<()> {
            python_utils::null_ptr_exception_if_none(self.inner.as_ref())
        }
    
        pub fn is_owned(&self) -> bool {
            true
        }

        pub fn check_owned(&self) -> PyResult<()>{
            Ok(())
        }
    }

    #[pymethods]
    impl $algebra {
        pub fn free(&mut self) -> PyResult<()> {
            python_utils::null_ptr_exception_if_none(self.inner.take())?;
            Ok(())
        }

        #[getter]
        pub fn get_owned(&self) -> bool {
            true
        }
    }

    impl $algebra {
        // type Inner = $inner; // ==> "associated types are not yet supported in inherent imples" =(

        pub fn wrap(inner : std::sync::Arc<AlgebraRust>) -> Self {
            Self {
                inner : Some(inner),
            }
        }

        pub fn take_box(&mut self) -> PyResult<std::sync::Arc<AlgebraRust>> {
            self.inner.take().ok_or_else(|| python_utils::null_ptr_exception())
        }
    }


    impl $algebra {
        fn check_degree(&self, degree : i32) -> PyResult<()> {
            let max_degree = self. inner_unchkd().max_degree();
            if degree > max_degree {
                Err(exceptions::IndexError::py_err(format!(
                    "Degree {} too large: maximum degree of algebra is {}. Run algebra.compute_basis({}) first.", 
                    degree, max_degree, degree
                )))
            } else {
                Ok(())
            }
        }

        fn check_dimension(&self, degree : i32, vec : &FpVector) -> PyResult<()> {
            let what_the_dimension_should_be = self. inner_unchkd().dimension(degree, -1);
            let the_dimension = vec.get_dimension()?;
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

        pub fn check_index(&self, degree : i32, idx : usize) -> PyResult<()> {
            let dimension = self. inner_unchkd().dimension(degree, -1);
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
        pub fn algebra_type(&self) -> PyResult<String> {
            Ok(self.inner()?.algebra_type().to_string())
        }

        #[getter]
        pub fn get_prime(&self) -> PyResult<u32> {
            Ok(*self.inner()?.prime())
        }

        pub fn compute_basis(&self, max_degree : i32) -> PyResult<()> {
            self.inner()?.compute_basis(max_degree);
            Ok(())
        }

        #[args(excess=0)]
        pub fn dimension(&self, degree : i32, excess : i32) -> PyResult<usize> {
            self.check_not_null()?;
            self.check_degree(degree)?;
            Ok(self.inner_unchkd().dimension(degree, excess))
        }

        #[args(excess=0)]
        pub fn multiply_basis_elements(&self, 
            result : &mut FpVector, coeff : u32, 
            r_degree : i32, r_index : usize, 
            s_degree : i32, s_index : usize, excess : i32
        ) -> PyResult<()> {
            self.check_not_null()?;
            self.check_degree(r_degree + s_degree)?;
            self.check_index(r_degree, r_index)?;
            self.check_index(s_degree, s_index)?;
            self.check_dimension(r_degree + s_degree, result)?;
            self.inner_unchkd().multiply_basis_elements(result.inner_mut()?, coeff, r_degree, r_index, s_degree, s_index, excess);
            Ok(())
        }

        #[args(excess=0)]
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
            self.check_dimension(r_degree + s_degree, result)?;
            self.inner_unchkd().multiply_basis_element_by_element(
                result.inner_mut()?, coeff, 
                r_degree, r_index,
                s_degree, s.inner()?,
                excess
            );
            Ok(())
        }

        #[args(excess=0)]
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
            self.check_dimension(r_degree + s_degree, result)?;
            self.inner_unchkd().multiply_element_by_basis_element(
                result.inner_mut()?, coeff, 
                r_degree, r.inner()?,
                s_degree, s_index,
                excess
            );
            Ok(())
        }

        #[args(excess=0)]
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
            self.check_dimension(r_degree + s_degree, result)?;
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

    #[pyproto]
    impl pyo3::PySequenceProtocol for $element {
        fn __len__(self) -> PyResult<usize> {
            self.element.get_dimension()
        }

        fn __getitem__(self, index : isize) -> PyResult<u32> {
            self.element.check_not_null()?;
            self.element.check_index(index)?;
            Ok(self.element.inner_unchkd().entry(index as usize))
        }

        fn __setitem__(mut self, index : isize, value : i32) -> PyResult<()> {
            self.element.check_not_null()?;
            self.element.check_index(index)?;
            self.element.inner_mut_unchkd().set_entry(index as usize, self.element.reduce_coefficient(value));
            Ok(())
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

        #[getter]
        fn get_algebra(&self) -> $algebra {
            self.algebra.clone()
        }

        #[getter]
        fn get_degree(&self) -> i32 {
            self.degree
        }

        #[getter]
        fn get_dimension(&self) -> PyResult<usize> {
            self.element.get_dimension()
        }
    
        #[args(c=1)]
        fn add(&mut self, other : PyObject, c : i32) -> PyResult<()> {
            self.element.add(&$element::obj_to_vec(other, "other")?, c)
        }
    
        #[args(coeff=1)]
        fn multiply_add(&mut self, left : &$element, right : &$element, coeff : i32) -> PyResult<()> {
            let coeff = python_utils::reduce_coefficient(self.algebra.get_prime()?, coeff);
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
                element : FpVector::new(self.get_prime()?, self.dimension(degree, -1)?)?
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