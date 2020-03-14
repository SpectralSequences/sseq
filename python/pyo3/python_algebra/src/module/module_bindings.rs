#![macro_use]



#[macro_export]
macro_rules! module_methods {
    ($module : ty) => {

        #[allow(dead_code)]
        impl $module {
            fn check_degree(&self, degree : i32) -> PyResult<()> {
                let max_degree = self.max_computed_degree()?;
                if degree > max_degree {
                    Err(exceptions::IndexError::py_err(format!(
                        "Degree {} too large: maximum degree of module is {}.", 
                        degree, max_degree
                    )))
                } else {
                    Ok(())
                }
            }
    
            fn check_dimension(&self, degree : i32, vec : &FpVector) -> PyResult<()> {
                let what_the_dimension_should_be = self.inner_unchkd().dimension(degree);
                let the_dimension = vec.get_dimension()?;
                if the_dimension == what_the_dimension_should_be {
                    Ok(())
                } else {
                    Err(exceptions::ValueError::py_err(format!(
                        "Dimension of vector is {} but the dimension of the module in degree {} is {}.",
                        the_dimension,
                        degree,
                        what_the_dimension_should_be
                    )))
                }
            }
    
            pub fn check_index(&self, degree : i32, idx : usize) -> PyResult<()> {
                let dimension = self.inner_unchkd().dimension(degree);
                if idx < dimension {
                    Ok(())
                } else {
                    Err(exceptions::IndexError::py_err(format!(
                        "Index {} is larger than dimension {} of the module in degree {}.",
                        idx,
                        dimension,
                        degree,
                    )))                
                }
            }
    
            fn check_algebra_degree(&self, degree : i32) -> PyResult<()> {
                let max_degree = self.inner_unchkd().algebra().max_degree();
                if degree > max_degree {
                    Err(exceptions::IndexError::py_err(format!(
                        "Degree {} too large: maximum degree of algebra is {}. Run \"algebra.compute_basis({})\" first", 
                        degree, max_degree, degree
                    )))
                } else {
                    Ok(())
                }
            }
    
    
            fn check_algebra_dimension(&self, degree : i32, vec : &FpVector) -> PyResult<()> {
                let what_the_dimension_should_be = self.inner_unchkd().algebra().dimension(degree, 0);
                let the_dimension = vec.get_dimension()?;
                if the_dimension == what_the_dimension_should_be {
                    Ok(())
                } else {
                    Err(exceptions::ValueError::py_err(format!(
                        "Dimension of vector is {} but the dimension of the module in degree {} is {}.",
                        the_dimension,
                        degree,
                        what_the_dimension_should_be
                    )))
                }
            }
    
            pub fn check_algebra_index(&self, degree : i32, idx : usize) -> PyResult<()> {
                let dimension = self.inner_unchkd().algebra().dimension(degree, 0);
                if idx < dimension {
                    Ok(())
                } else {
                    Err(exceptions::IndexError::py_err(format!(
                        "Index {} is larger than dimension {} of the module in degree {}.",
                        idx,
                        dimension,
                        degree,
                    )))                
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
                Ok(*self.inner()?.prime())
            }
    
            #[getter]
            pub fn get_name(&self) -> PyResult<String> {
                Ok(self.inner()?.name())
            }
    
            #[getter]
            pub fn get_min_degree(&self) -> PyResult<i32> {
                Ok(self.inner()?.min_degree())
            }
    
            pub fn compute_basis(&self, max_degree : i32) -> PyResult<()> {
                self.inner()?.compute_basis(max_degree);
                Ok(())
            }
    
            pub fn dimension(&self, degree : i32) -> PyResult<usize> {
                self.check_not_null()?;
                self.check_degree(degree)?;
                Ok(self.inner_unchkd().dimension(degree))
            }
    
    
            pub fn act_on_basis(
                &self,
                result: &mut FpVector,
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
                self.inner_unchkd().act_on_basis(result.inner_mut_unchkd(), coeff, op_degree, op_index, mod_degree, mod_index);
                Ok(())
            }


            fn act(
                &self,
                result: &mut FpVector,
                coeff: u32,
                op_degree: i32,
                op_index: usize,
                input_degree: i32,
                input: &FpVector,
            ) -> PyResult<()> {
                self.check_not_null()?;
                self.check_degree(op_degree + input_degree)?;
                self.check_algebra_degree(op_degree)?;
                result.check_not_null()?;
                self.check_dimension(op_degree + input_degree, result)?;
                input.check_not_null()?;
                self.check_dimension(input_degree, input)?;
                self.check_algebra_index(op_degree, op_index)?;
                self.inner_unchkd().act(result.inner_mut_unchkd(), coeff, op_degree, op_index, input_degree, input.inner_unchkd());
                Ok(())
            }

            fn act_by_element(
                &self,
                result: &mut FpVector,
                coeff: u32,
                op_degree: i32,
                op: &FpVector,
                input_degree: i32,
                input: &FpVector,
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
                self.inner_unchkd().act_by_element(result.inner_mut_unchkd(), coeff, op_degree, op.inner_unchkd(), input_degree, input.inner_unchkd());
                Ok(())
            }

            fn generator_list_string(&self, degree: i32) -> PyResult<String> {
                Ok(self.inner()?.generator_list_string(degree))
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
            
        }
    }
}



#[macro_export]
macro_rules! module_bindings { ( $module : ident, $module_rust : ident) => {

    python_utils::rc_wrapper_type_inner!($module, ModuleRust);
    python_utils::wrapper_outer_defs_dispatch_to_enum_variant!($module, ModuleRust, $module, $module_rust<AlgebraRust>);

    module_methods!($module);

        // fn wrap_module_rust_frozen(inner : crate::module::module_bindings::ModuleRustFrozenWrapper) -> Self {
        //     Self {
        //         inner : $module_inner::ModuleFrozen(inner)
        //     }
        // }

        // fn wrap_module_rust_mutable(inner : crate::module::module_bindings::ModuleRustMutableWrapper) -> Self {
        //     Self {
        //         inner : $module_inner::ModuleMutable(inner)
        //     }
        // }


        // pub fn mutable_from_rust(module : $module_rust<AlgebraRust>) -> Self {
        //     Self::wrap_module_rust_mutable(crate::module::module_bindings::ModuleRustMutableWrapper::box_and_wrap(
        //         ModuleRust::$module(module)
        //     ))
        // }

        // pub fn immutable_from_rust(module : $module_rust<AlgebraRust>) -> Self {
        //     Self::wrap_module_rust_frozen(crate::module::module_bindings::ModuleRustFrozenWrapper::box_and_wrap(
        //         ModuleRust::$module(module)
        //     ))
        // }

        // pub fn immutable_from_arc(module : Arc<ModuleRust>) -> Self {
        //     Self::wrap_module_rust_frozen(crate::module::module_bindings::ModuleRustFrozenWrapper::wrap(module))
        // }


    // #[pyclass(dict)]
    // pub struct $element {
    //     module : $module,
    //     degree : i32,
    //     element : FpVector
    // }
    
    // #[pyproto]
    // impl PyObjectProtocol for $element {
    //     fn __repr__(&self) -> PyResult<String> {
    //         self.algebra.element_to_string(self.degree, &self.element)
    //     }
    // }

    // #[pyproto]
    // impl pyo3::PySequenceProtocol for $element {
    //     fn __len__(self) -> PyResult<usize> {
    //         self.element.get_dimension()
    //     }

    //     fn __getitem__(self, index : isize) -> PyResult<u32> {
    //         self.element.check_not_null()?;
    //         self.element.check_index(index)?;
    //         Ok(self.element.inner_unchkd().entry(index as usize))
    //     }

    //     fn __setitem__(mut self, index : isize, value : i32) -> PyResult<()> {
    //         self.element.check_not_null()?;
    //         self.element.check_index(index)?;
    //         self.element.inner_mut_unchkd().set_entry(index as usize, self.element.reduce_coefficient(value));
    //         Ok(())
    //     }
    // }
    
    // impl $element {
    //     fn obj_to_vec(obj : PyObject, argument_name : &str) -> PyResult<FpVector> {
    //         let gil = Python::acquire_gil();
    //         let py = gil.python();
    //         Ok(obj.extract::<&FpVector>(py).or_else(|_err| {
    //             Ok(&obj.extract::<&$element>(py)?.element)
    //         }).map_err(|_err : PyErr| {
    //             exceptions::ValueError::py_err(format!(
    //                 "Argument \"{}\" expected to be either an {} or an FpVector.",
    //                 $element_name,
    //                 argument_name
    //             ))
    //         })?.clone())
    //     }
    // }
    
    // #[pymethods]
    // impl $element {
    //     #[getter]
    //     fn get_vec(&self) -> FpVector {
    //         self.element.clone()
    //     }

    //     #[getter]
    //     fn get_module(&self) -> $module {
    //         self.module.clone()
    //     }

    //     #[getter]
    //     fn get_degree(&self) -> i32 {
    //         self.degree
    //     }

    //     #[getter]
    //     fn get_dimension(&self) -> PyResult<usize> {
    //         self.element.get_dimension()
    //     }
    
    //     #[args(c=1)]
    //     fn add(&mut self, other : PyObject, c : i32) -> PyResult<()> {
    //         self.element.add(&$element::obj_to_vec(other, "other")?, c)
    //     }
    
    //     #[args(coeff=1)]
    //     fn multiply_add(&mut self, left : &$element, right : &$element, coeff : i32) -> PyResult<()> {
    //         let coeff = python_utils::reduce_coefficient(self.algebra.get_prime()?, coeff);
    //         self.algebra.multiply_element_by_element(&mut self.element, coeff, 
    //             left.degree, &left.element, 
    //             -1
    //             right.degree, &right.element, 
    //         )
    //     }
    // }
    
    // #[pymethods]
    // impl $module {
    //     fn new_element(&self, degree : i32) -> PyResult<$element> {
    //         Ok($element {
    //             algebra : self.clone(),
    //             degree,
    //             element : FpVector::new(self.get_prime()?, self.dimension(degree, -1)?)?
    //         })
    //     }
    
    //     fn element_from_vec(&self, degree : i32, v : &FpVector) -> PyResult<$element> {
    //         Ok($element {
    //             algebra : self.clone(),
    //             degree,
    //             element : v.clone()
    //         })
    //     }
    // }
}}