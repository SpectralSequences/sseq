// #![macro_use]

mod finite_dimensional_module;
mod free_module;
pub use finite_dimensional_module::*;
pub use free_module::*;


#[macro_export]
macro_rules! module_methods { ( $module:ident ) => {
    #[pymethods]
    impl $module {
        #[getter]
        pub fn get_algebra(&self) -> PyResult<PyObject> {
            Ok(algebra_into_py_object(self.inner()?.algebra()))
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
            Ok(self.inner()?.dimension(degree))
        }

        pub fn act_on_basis(
            &self,
            result: &FpVector,
            coeff: u32,
            op_degree: i32,
            op_index: usize,
            mod_degree: i32,
            mod_index: usize,
        ) -> PyResult<()>{
            self.inner()?.act_on_basis(result.inner_mut()?, coeff, op_degree, op_index, mod_degree, mod_index);
            Ok(())
        }

        pub fn basis_element_to_string(&self, degree: i32, idx: usize) -> PyResult<String> {
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
        //     mod_degree: i32,
        //     mod_index: usize,
        // ) -> PyResult<FpVector> {
        //     Ok(FpVector::wrap(self.inner()?.act_on_basis_borrow(op_degree, op_index, mod_degree, mod_index), self.owner()))
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
            self.inner()?.act_by_element(result.inner_mut()?, coeff, op_degree, op.inner()?, input_degree, input.inner()?);
            Ok(())
        }


        fn generator_list_string(&self, degree: i32) -> PyResult<String> {
            Ok(self.inner()?.generator_list_string(degree))
        }

        fn element_to_string(&self, degree: i32, element: &FpVector) -> PyResult<String> {
            Ok(self.inner()?.element_to_string(degree, element.inner()?))
        }


        // fn truncate_to_fd_module(self: Arc<Self>, max_deg: i32) -> FDModule<Self::Algebra> {}

    }
}}