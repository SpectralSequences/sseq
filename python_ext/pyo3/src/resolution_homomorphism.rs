
use ext::resolution_homomorphism::{
    ResolutionHomomorphism as ResolutionHomomorphismRust, 
};

use pyo3::prelude::*;

use python_fp::vector::FpVector;
use python_fp::matrix::Matrix;
use crate::resolution::{CCRust, Resolution};

#[pymethods]
impl ResolutionHomomorphism {
    #[new]
    pub fn new(
        name : String,
        source : &Resolution, target : &Resolution,
        homological_degree_shift : u32, internal_degree_shift : i32
    ) -> PyResult<Self> {
        Ok(ResolutionHomomorphism::box_and_wrap(ResolutionHomomorphismRust::new(
            name, 
            source.to_arc(), 
            target.to_arc(),
            homological_degree_shift,
            internal_degree_shift
        )))
    }

    // pub fn get_map(&self, output_homological_degree : u32) -> &FreeModuleHomomorphism {
    //     Ok(FreeModuleHomomorphism:: self.inner()?.get_map(output_homological_degree))
    // }

    pub fn extend(&self, source_homological_degree : u32, source_degree : i32) -> PyResult<()> {
        let self_inner = self.inner()?;
        python_utils::release_gil!(self_inner.extend(source_homological_degree, source_degree));
        Ok(())
    }

    pub fn extend_step(&self, source_homological_degree : u32, source_degree : i32, extra_images : PyObject) -> PyResult<()> {
        let temp;
        let extra_images_rust = 
            if extra_images.is_none() {
                None
            } else {
                let gil = Python::acquire_gil();
                let py = gil.python();
                temp = extra_images.extract::<Matrix>(py)
                    .map_err(|_err : PyErr| {
                        python_utils::exception!(TypeError,
                            "Type error!"
                        )
                    })?;
                Some(
                    temp.inner()?
                )
            };
        let self_inner = self.inner()?;
        python_utils::release_gil!(self_inner.extend_step(source_homological_degree, source_degree, extra_images_rust));
        Ok(())
    }

    pub fn act(&self, result: &mut FpVector, s: u32, t: i32, idx: usize)  -> PyResult<()> {
        self.inner()?.act(result.inner_mut()?, s, t, idx);
        Ok(())
    }
}