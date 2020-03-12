// #![macro_use]

mod module_rust;
mod finite_dimensional_module;
mod free_module;
pub use finite_dimensional_module::*;
pub use free_module::*;
pub use module_rust::ModuleRust;

#[macro_export]
macro_rules! module_methods { ( $module:ident ) => {    
    impl $module {
        fn check_degree(&self, degree : i32) -> PyResult<()> {
            let algebra_max_degree = self.get_rust_algebra()?.max_degree();
            if degree > algebra_max_degree {
                return Err(exceptions::IndexError::py_err(format!(
                    "Degree {} too large: maximum degree of algebra is {}. \
                     Run algebra.compute_basis({}) first.",
                    degree, algebra_max_degree, degree                    
                )))
            }
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

        pub fn check_index(&self, degree : i32, index : usize) -> PyResult<()> {
            let dimension = self.inner()?.dimension(degree);
            if index < dimension {
                Ok(())
            } else {
                Err(exceptions::IndexError::py_err(format!(
                    "Used index {} but module is only {} dimensional in degree {}.",
                    index,
                    dimension,
                    degree
                )))
            }
        }

        pub fn get_rust_algebra(&self) -> PyResult<std::sync::Arc<AlgebraRust>>{
            Ok(self.inner()?.algebra())
        }

        pub fn check_algebra_index(&self, degree : i32, idx : usize) -> PyResult<()> {
            let dimension = self.get_rust_algebra()?.dimension(degree, -1);
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

        fn check_dimension(&self, degree : i32, vec : &FpVector) -> PyResult<()> {
            let what_the_dimension_should_be = self.inner()?.dimension(degree);
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

        fn check_algebra_dimension(&self, degree : i32, vec : &FpVector) -> PyResult<()> {
            let what_the_dimension_should_be = self.get_rust_algebra()?.dimension(degree, -1);
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
    }

    #[pymethods]
    impl $module {
        #[getter]
        pub fn get_algebra(&self) -> PyResult<PyObject> {
            Ok(AlgebraRust::algebra_into_py_object(self.inner()?.algebra()))
        }

        #[getter]
        pub fn get_name(&self) -> PyResult<String> {
            Ok(self.inner()?.name())
        }

        #[getter]
        pub fn get_min_degree(&self) -> PyResult<i32> {
            Ok(self.inner()?.min_degree())
        }

        pub fn compute_basis(&self, degree: i32) -> PyResult<()>  {
            Ok(self.inner()?.compute_basis(degree))
        }

        pub fn dimension(&self, degree: i32) -> PyResult<usize> {
            self.check_degree(degree)?;
            Ok(self.inner()?.dimension(degree))
        }

        pub fn act_on_basis(
            &self,
            result: &FpVector,
            coeff: u32,
            op_degree: i32,
            op_index: usize,
            input_degree: i32,
            mod_index: usize,
        ) -> PyResult<()>{
            let inner = self.inner()?;
            self.check_degree(op_degree + input_degree)?;
            self.check_algebra_index(op_degree, op_index)?;
            self.check_index(input_degree, mod_index)?;
            self.check_dimension(op_degree + input_degree, result)?;
            inner.act_on_basis(result.inner_mut()?, coeff, op_degree, op_index, input_degree, mod_index);
            Ok(())
        }

        pub fn basis_element_to_string(&self, degree: i32, idx: usize) -> PyResult<String> {
            self.check_degree(degree)?;
            self.check_index(degree, idx)?;
            Ok(self.inner()?.basis_element_to_string(degree, idx))
        }

        fn is_unit(&self) -> PyResult<bool> {
            Ok(self.inner()?.is_unit())
        }

        fn prime(&self) -> PyResult<u32> {
            Ok(*self.inner()?.prime())
        }

        // fn borrow_output(&self) -> PyResult<bool> {
        //     Ok(self.inner()?.borrow_output())
        // }

        // fn act_on_basis_borrow(
        //     &self,
        //     op_degree: i32,
        //     op_index: usize,
        //     input_degree: i32,
        //     mod_index: usize,
        // ) -> PyResult<FpVector> {
        //     Ok(FpVector::wrap(self.inner()?.act_on_basis_borrow(op_degree, op_index, input_degree, mod_index), self.owner()))
        // }

        fn act(
            &self,
            result: &mut FpVector,
            coeff: u32,
            op_degree: i32,
            op_index: usize,
            input_degree: i32,
            input: &FpVector,
        ) -> PyResult<()> {
            self.check_degree(op_degree + input_degree)?;
            self.check_algebra_index(op_degree, op_index)?;
            self.check_dimension(input_degree, input)?;
            self.check_dimension(op_degree + input_degree, result)?;
            self.inner()?.act(result.inner_mut()?, coeff, op_degree, op_index, input_degree, input.inner()?);
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
            self.check_degree(op_degree + input_degree)?;
            self.check_dimension(op_degree + input_degree, result)?;
            self.check_algebra_dimension(op_degree, op)?;
            self.check_dimension(input_degree, input)?;
            self.inner()?.act_by_element(result.inner_mut()?, coeff, op_degree, op.inner()?, input_degree, input.inner()?);
            Ok(())
        }


        fn generator_list_string(&self, degree: i32) -> PyResult<String> {
            self.check_degree(degree)?;
            Ok(self.inner()?.generator_list_string(degree))
        }

        fn element_to_string(&self, degree: i32, element: &FpVector) -> PyResult<String> {
            self.check_degree(degree)?;
            self.check_dimension(degree, element)?;
            Ok(self.inner()?.element_to_string(degree, element.inner()?))
        }


        // fn truncate_to_fd_module(self: Arc<Self>, max_deg: i32) -> FDModule<Self::Algebra> {}

    }
}}