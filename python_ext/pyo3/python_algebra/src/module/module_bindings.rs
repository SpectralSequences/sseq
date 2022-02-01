#![macro_use]


#[macro_export]
macro_rules! module_methods {
    ($module : ty) => {

        #[allow(dead_code)]
        impl $module {
            fn check_degree(&self, degree : i32) -> PyResult<()> {
                let max_degree = Module::max_computed_degree(self.inner()?);
                if degree > max_degree {
                    Err(python_utils::exception!(IndexError,
                        "Degree {} too large: maximum degree of module is {}.", 
                        degree, max_degree
                    ))
                } else {
                    Ok(())
                }
            }
    
            fn check_dimension(&self, degree : i32, vec : &python_fp::vector::FpVector) -> PyResult<()> {
                let what_the_dimension_should_be = Module::dimension(self.inner_unchkd(), degree);
                let the_dimension = vec.get_dimension()?;
                if the_dimension == what_the_dimension_should_be {
                    Ok(())
                } else {
                    Err(python_utils::exception!(ValueError,
                        "Dimension of vector is {} but the dimension of the module in degree {} is {}.",
                        the_dimension,
                        degree,
                        what_the_dimension_should_be
                    ))
                }
            }
    
            pub fn check_index(&self, degree : i32, idx : usize) -> PyResult<()> {
                let dimension = Module::dimension(self.inner_unchkd(), degree);
                if idx < dimension {
                    Ok(())
                } else {
                    Err(python_utils::exception!(IndexError,
                        "Index {} is larger than dimension {} of the module in degree {}.",
                        idx,
                        dimension,
                        degree,
                    ))
                }
            }
    
            fn check_algebra_degree(&self, degree : i32) -> PyResult<()> {
                // let max_degree = self.inner_unchkd().algebra().max_computed_degree();
                let max_degree = i32::max_value(); // TODO: fix me.
                if degree > max_degree {
                    Err(python_utils::exception!(IndexError,
                        "Degree {} too large: maximum degree of algebra is {}. Run \"algebra.compute_basis({})\" first", 
                        degree, max_degree, degree
                    ))
                } else {
                    Ok(())
                }
            }
    
    
            fn check_algebra_dimension(&self, degree : i32, vec : &python_fp::vector::FpVector) -> PyResult<()> {
                let what_the_dimension_should_be = self.inner_unchkd().algebra().dimension(degree);
                let the_dimension = vec.get_dimension()?;
                if the_dimension == what_the_dimension_should_be {
                    Ok(())
                } else {
                    Err(python_utils::exception!(ValueError,
                        "Dimension of vector is {} but the dimension of the module in degree {} is {}.",
                        the_dimension,
                        degree,
                        what_the_dimension_should_be
                    ))
                }
            }
    
            pub fn check_algebra_index(&self, degree : i32, idx : usize) -> PyResult<()> {
                let dimension = self.inner_unchkd().algebra().dimension(degree);
                if idx < dimension {
                    Ok(())
                } else {
                    Err(python_utils::exception!(IndexError,
                        "Index {} is larger than dimension {} of the module in degree {}.",
                        idx,
                        dimension,
                        degree,
                    ))
                }
                
            }        
        }

        #[pymethods]
        impl $module {
            #[getter]
            fn get_algebra(&self) -> PyResult<PyObject> {
                Ok(AlgebraRust::into_py_object(self.inner()?.algebra()))
            }
    
            #[getter]
            pub fn get_prime(&self) -> PyResult<u32> {
                Ok(*Module::prime(self.inner()?))
            }
    
            #[getter]
            pub fn get_min_degree(&self) -> PyResult<i32> {
                Ok(self.inner()?.min_degree())
            }
    
            pub fn compute_basis(&self, max_degree : i32) -> PyResult<()> {
                Module::compute_basis(self.inner()?, max_degree);
                Ok(())
            }
    
            pub fn dimension(&self, degree : i32) -> PyResult<usize> {
                self.check_not_null()?;
                self.check_degree(degree)?;
                Ok(Module::dimension(self.inner_unchkd(), degree))
            }
    
            pub fn act_on_basis(
                &self,
                result: &mut python_fp::vector::FpVector,
                coeff: u32,
                op_degree: i32,
                op_index: usize,
                mod_degree: i32,
                mod_index: usize,
            ) -> PyResult<()> {
                self.check_not_null()?;
                self.check_degree(op_degree + mod_degree)?;
                self.check_algebra_degree(op_degree)?;
                result.check_not_null()?;
                self.check_dimension(op_degree + mod_degree, result)?;
                self.check_index(mod_degree, mod_index)?;
                self.check_algebra_index(op_degree, op_index)?;
                self.inner_unchkd().act_on_basis(result.inner_mut_unchkd().as_slice_mut(), coeff, op_degree, op_index, mod_degree, mod_index);
                Ok(())
            }


            fn act(
                &self,
                result: &mut python_fp::vector::FpVector,
                coeff: u32,
                op_degree: i32,
                op_index: usize,
                input_degree: i32,
                input: &python_fp::vector::FpVector,
            ) -> PyResult<()> {
                self.check_not_null()?;
                self.check_degree(op_degree + input_degree)?;
                self.check_algebra_degree(op_degree)?;
                result.check_not_null()?;
                self.check_dimension(op_degree + input_degree, result)?;
                input.check_not_null()?;
                self.check_dimension(input_degree, input)?;
                self.check_algebra_index(op_degree, op_index)?;
                self.inner_unchkd().act(result.inner_mut_unchkd().as_slice_mut(), coeff, op_degree, op_index, input_degree, input.inner_unchkd().as_slice());
                Ok(())
            }

            fn act_by_element(
                &self,
                result: &mut python_fp::vector::FpVector,
                coeff: u32,
                op_degree: i32,
                op: &python_fp::vector::FpVector,
                input_degree: i32,
                input: &python_fp::vector::FpVector,
            ) -> PyResult<()> {
                self.check_not_null()?;
                self.check_degree(op_degree + input_degree)?;
                self.check_algebra_degree(op_degree)?;
                result.check_not_null()?;
                self.check_dimension(op_degree + input_degree, result)?;
                input.check_not_null()?;
                self.check_dimension(input_degree, input)?;
                op.check_not_null()?;
                self.check_algebra_dimension(op_degree, op)?;
                self.inner_unchkd().act_by_element(result.inner_mut_unchkd().as_slice_mut(), coeff, op_degree, op.inner_unchkd().as_slice(), input_degree, input.inner_unchkd().as_slice());
                Ok(())
            }

            fn basis_string_list(&self, degree: i32) -> PyResult<Vec<String>> {
                self.check_not_null()?;
                self.check_degree(degree)?;                
                Ok(self.inner()?.basis_string_list(degree))
            }

            pub fn basis_element_to_string(&self, degree : i32, idx : usize) -> PyResult<String> {
                self.check_not_null()?;
                self.check_degree(degree)?;
                self.check_index(degree, idx)?;
                Ok(Module::basis_element_to_string(self.inner_unchkd(), degree, idx))
            }
            
            pub fn element_to_string(&self, degree : i32, element : &python_fp::vector::FpVector) -> PyResult<String> {
                self.check_not_null()?;
                self.check_degree(degree)?;
                element.check_not_null()?;
                self.check_dimension(degree, element)?;
                Ok(Module::element_to_string(self.inner_unchkd(), degree, element.inner_unchkd().as_slice()))
            }

            pub fn check_relation(&self,
                outer_op_degree : i32, outer_op_index : usize, 
                inner_op_degree : i32, inner_op_index : usize,
                module_degree : i32, module_index : usize
            ) -> PyResult<python_fp::vector::FpVector> {
                self.check_not_null()?;
                self.check_degree(outer_op_degree + inner_op_degree + module_degree)?;
                self.check_algebra_degree(outer_op_degree + inner_op_degree)?;
                let p = *self.inner_unchkd().algebra().prime();
                let result = python_fp::vector::FpVector::new(p, 0)?;
                let scratch = python_fp::vector::FpVector::new(p, 0)?;
                self.inner_unchkd().check_relation(
                    result.inner_mut_unchkd(), scratch.inner_mut_unchkd(),
                    outer_op_degree, outer_op_index, 
                    inner_op_degree, inner_op_index,
                    module_degree, module_index
                );
                Ok(result)
            }
            
        }
    }
}



#[macro_export]
macro_rules! module_bindings { ( $module : ident, $module_rust : ident, $element : ident) => {

    python_utils::rc_wrapper_type_inner!($module, ModuleRust);
    python_utils::wrapper_outer_defs_dispatch_to_enum_variant!($module, ModuleRust, $module, $module_rust<AlgebraRust>);

    module_methods!($module);

    #[pyclass(dict)]
    #[derive(Clone)]
    pub struct $element {
        module : $module,
        degree : i32,
        element : FpVector
    }
    
    #[pyproto]
    impl PyObjectProtocol for $element {
        fn __repr__(&self) -> PyResult<String> {
            self.module.element_to_string(self.degree, &self.element)
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
            Ok(obj.extract::<FpVector>(py).or_else(|_err| {
                Ok(obj.extract::<$element>(py)?.element)
            }).map_err(|_err : PyErr| {
                python_utils::exception!(TypeError,
                    "Argument \"{}\" expected to be either an {} or an FpVector.",
                    stringify!($element),
                    argument_name
                )
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
        fn get_module(&self) -> $module {
            self.module.clone()
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
    
        // #[args(coeff=1)]
        // fn multiply_add(&mut self, left : &$element, right : &$element, coeff : i32) -> PyResult<()> {
        //     let coeff = python_utils::reduce_coefficient(self.algebra.get_prime()?, coeff);
        //     self.algebra.multiply_element_by_element(&mut self.element, coeff, 
        //         left.degree, &left.element, 
        //         right.degree, &right.element, 
        //     )
        // }
    }
    
    #[pymethods]
    impl $module {
        fn new_element(&self, degree : i32) -> PyResult<$element> {
            Ok($element {
                module : self.clone(),
                degree,
                element : FpVector::new(self.get_prime()?, self.dimension(degree)?)?
            })
        }
    
        fn element_from_vec(&self, degree : i32, v : &FpVector) -> PyResult<$element> {
            Ok($element {
                module : self.clone(),
                degree,
                element : v.clone()
            })
        }
    }
}}