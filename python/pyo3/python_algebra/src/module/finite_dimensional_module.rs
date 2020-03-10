use pyo3::{
    prelude::*,
    exceptions::{
        IOError, 
        // ReferenceError, 
        // RuntimeError, 
        // TypeError,
        ValueError
    },
    PyObjectProtocol,
    // types::PyDict
};

use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use std::collections::HashMap;

use serde_json::{json, Value};

use python_utils::{
    self,
    py_repr, 
    rc_wrapper_type, 
    wrapper_type, 
    // immutable_wrapper_type,
    // get_from_kwargs
};

use bivec::BiVec;
use algebra::module::{FDModule as FDModuleRust, Module, BoundedModule};

use python_fp::vector::FpVector;
use crate::algebra::{AlgebraRust, algebra_into_py_object, algebra_from_py_object};


rc_wrapper_type!(FDModule, FDModuleRust<AlgebraRust>);

py_repr!(FDModule, "FreedFDModule", {
    Ok(format!(
        "FDModule({})",
        *inner.prime()
    ))
});

impl FDModule {
    fn from_json_inner(mut json: Value) -> PyResult<Self> {
        let algebra = AlgebraRust::from_json(&json, "adem".to_string())
            .map_err(|e| ValueError::py_err(format!("Failed to construct algebra: {}", e)))?;
        let algebra = Arc::new(algebra);
        let module = FDModuleRust::from_json(algebra, &mut json);
        // .map_err(|e| {
        //     ValueError::py_err(format!("Failed to construct module: {}", e))
        // })?;
        Ok(Self::box_and_wrap(module))
    }
}

#[pymethods]
impl FDModule {
    #[staticmethod]
    fn from_file(path: String) -> PyResult<Self> {
        let f =
            File::open(path).map_err(|e| IOError::py_err(format!("Failed to open file: {}", e)))?;

        let json = serde_json::from_reader(BufReader::new(f))
            .map_err(|e| ValueError::py_err(format!("Failed to parse json: {}", e)))?;

        Self::from_json_inner(json)
    }

    #[staticmethod]
    fn from_json(json: String) -> PyResult<Self> {
        let json = serde_json::from_str(&json)
            .map_err(|e| ValueError::py_err(format!("Failed to parse json: {}", e)))?;

        Self::from_json_inner(json)
    }

    fn to_json(&self) -> PyResult<String> {
        let mut json = json!({});
        let inner = self.inner()?;
        inner.algebra().to_json(&mut json);
        inner.to_json(&mut json);
        Ok(json.to_string())
    }

    fn dimension(&self, degree: i32) -> PyResult<usize> {
        Ok(self.inner()?.dimension(degree))
    }

    #[getter]
    fn get_min_degree(&self) -> PyResult<i32> {
        Ok(self.inner()?.min_degree())
    }

    #[getter]
    fn get_max_degree(&self) -> PyResult<i32> {
        Ok(self.inner()?.max_degree())
    }

    #[getter]
    fn get_algebra(&self) -> PyResult<PyObject> {
        Ok(algebra_into_py_object(self.inner()?.algebra()))
    }

    fn compute_basis(&self, degree: i32) -> PyResult<()>  {
        Ok(self.inner()?.compute_basis(degree))
    }

    fn basis_element_to_string(&self, degree: i32, idx: usize) -> PyResult<String> {
        Ok(self.inner()?.basis_element_to_string(degree, idx))
    }

    fn is_unit(&self) -> PyResult<bool> {
        Ok(self.inner()?.is_unit())
    }
}




wrapper_type!(FDModuleMutable, FDModuleRust<AlgebraRust>);

py_repr!(FDModuleMutable, "FreedFDModuleMutable", {
    Ok(format!(
        "FDModuleMutable({})",
        *inner.prime()
    ))
});

impl FDModuleMutable {
    fn from_json_inner(mut json: Value) -> PyResult<Self> {
        let algebra = AlgebraRust::from_json(&json, "adem".to_string())
            .map_err(|e| ValueError::py_err(format!("Failed to construct algebra: {}", e)))?;
        let algebra = Arc::new(algebra);
        let module = FDModuleRust::from_json(algebra, &mut json);
        // .map_err(|e| {
        //     ValueError::py_err(format!("Failed to construct module: {}", e))
        // })?;
        Ok(Self::box_and_wrap(module))
    }
}

#[pymethods]
impl FDModuleMutable {
    #[new]
    #[args(min_degree = 0)]
    pub fn new(algebra: PyObject, name: String, min_degree : i32) -> PyResult<Self> {
        let algebra = algebra_from_py_object(algebra)?;
        let empty_bivec = BiVec::new(min_degree);
        Ok(Self::box_and_wrap(FDModuleRust::new(algebra, name, empty_bivec)))
    }

    #[staticmethod]
    fn from_file(path: String) -> PyResult<Self> {
        let f =
            File::open(path).map_err(|e| IOError::py_err(format!("Failed to open file: {}", e)))?;

        let json = serde_json::from_reader(BufReader::new(f))
            .map_err(|e| ValueError::py_err(format!("Failed to parse json: {}", e)))?;

        Self::from_json_inner(json)
    }

    #[staticmethod]
    fn from_json(json: String) -> PyResult<Self> {
        let json = serde_json::from_str(&json)
            .map_err(|e| ValueError::py_err(format!("Failed to parse json: {}", e)))?;

        Self::from_json_inner(json)
    }

    fn to_json(&self) -> PyResult<String> {
        let mut json = json!({});
        let inner = self.inner()?;
        inner.algebra().to_json(&mut json);
        inner.to_json(&mut json);
        Ok(json.to_string())
    }

    fn dimension(&self, degree: i32) -> PyResult<usize> {
        Ok(self.inner()?.dimension(degree))
    }

    #[getter]
    fn get_min_degree(&self) -> PyResult<i32> {
        Ok(self.inner()?.min_degree())
    }

    #[getter]
    fn get_max_degree(&self) -> PyResult<i32> {
        Ok(self.inner()?.max_degree())
    }

    #[getter]
    fn get_algebra(&self) -> PyResult<PyObject> {
        Ok(algebra_into_py_object(self.inner()?.algebra()))
    }

    fn compute_basis(&self, degree: i32) -> PyResult<()>  {
        Ok(self.inner()?.compute_basis(degree))
    }

    fn basis_element_to_string(&self, degree: i32, idx: usize) -> PyResult<String> {
        Ok(self.inner()?.basis_element_to_string(degree, idx))
    }

    fn is_unit(&self) -> PyResult<bool> {
        Ok(self.inner()?.is_unit())
    }
}


#[pymethods]
impl FDModuleMutable {
    pub fn add_generator(&self, degree: i32, name: String) -> PyResult<()> {
        Ok(self.inner_mut()?.add_generator(degree, name))
    }

    pub fn set_action_vector(
        &self,
        operation_degree: i32,
        operation_idx: usize,
        input_degree: i32,
        input_idx: usize,
        output: &FpVector,
    ) -> PyResult<()> {
        self.inner_mut()?.set_action_vector(operation_degree, operation_idx, input_degree, input_idx, output.inner()?);
        Ok(())
    }

    pub fn action_mut(
        &self,
        operation_degree: i32,
        operation_idx: usize,
        input_degree: i32,
        input_idx: usize,
    ) -> PyResult<FpVector> {
        Ok(FpVector::wrap(self.inner_mut()?.action_mut(operation_degree, operation_idx, input_degree, input_idx), self.owner()))
    }

    pub fn parse_action(
        &self,
        gen_to_idx: PyObject,
        entry_: &str,
        overwrite: bool,
    ) -> PyResult<()> {
        // for i in self.get_min_degree()..self.get_max_degree() {

        // }
        // let gil = Python::acquire_gil();
        // let py = gil.python();
        // let gen_to_idx : HashMap<String, (i32, usize)> = gen_to_idx.extract(py)?;
        self.inner_mut()?.parse_action(&gen_to_idx, entry_, overwrite)
            .map_err(|err| ValueError::py_err(
                format!("Error parsing: {}", err)
            ))
    }


    pub fn extend_actions(&self, input_deg: i32, output_deg: i32) -> PyResult<()> {
        self.inner_mut()?.extend_actions(input_deg, output_deg);
        Ok(())
    }

    pub fn set_basis_element_name(&mut self, degree: i32, idx: usize, name: String) -> PyResult<()> {
        self.inner_mut()?.set_basis_element_name(degree, idx, name);
        Ok(())
    }

    pub fn check_validity(
        &self,
        input_deg: i32,
        output_deg: i32,
    ) -> PyResult<()> {
        self.inner()?.check_validity(input_deg, output_deg)
            .map_err(|err| ValueError::py_err(format!(
                "{}", err
            )))
    }


}