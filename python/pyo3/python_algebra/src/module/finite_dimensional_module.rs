#![allow(unused_imports)]

use pyo3::{
    prelude::*,
    exceptions,
    PyObjectProtocol,
    // types::PyDict
};

use std::fs::File;
use std::io::BufReader;
use std::sync::{Arc, Weak};
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

use crate::module_methods;

use bivec::BiVec;
use algebra::Algebra as AlgebraT;
use algebra::module::{FDModule as FDModuleRust, Module, BoundedModule};

use python_fp::vector::FpVector;
use crate::algebra_rust::AlgebraRust;
use crate::module::module_rust::ModuleRust;

crate::module_bindings!(FDModule, FDModuleRust);

impl FDModule {
    fn max_computed_degree(&self) -> PyResult<i32> {
        self.check_degree(20)?;
        Ok(i32::max_value())
        // Ok(self.inner()?.max_degree())
    }
}


impl FDModule {
    fn from_json_inner(mut json: Value) -> PyResult<Self> {
        let algebra = AlgebraRust::from_json(&json, "adem".to_string())
            .map_err(|e| exceptions::ValueError::py_err(format!("Failed to construct algebra: {}", e)))?;
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
    #[new]
    #[args(min_degree=0)]
    fn new(algebra: PyObject, name: String, min_degree : i32) -> PyResult<Self> {
        let graded_dimension = BiVec::new(min_degree);
        Ok(Self::box_and_wrap(
            FDModuleRust::new(AlgebraRust::from_py_object(algebra)?, name, graded_dimension)
        ))
    }

    #[staticmethod]
    fn from_file(path: String) -> PyResult<Self> {
        let f =
            File::open(path).map_err(|e| exceptions::IOError::py_err(format!("Failed to open file: {}", e)))?;

        let json = serde_json::from_reader(BufReader::new(f))
            .map_err(|e| exceptions::ValueError::py_err(format!("Failed to parse json: {}", e)))?;

        Self::from_json_inner(json)
    }

    #[staticmethod]
    fn from_json(json: String) -> PyResult<Self> {
        let json = serde_json::from_str(&json)
            .map_err(|e| exceptions::ValueError::py_err(format!("Failed to parse json: {}", e)))?;

        Self::from_json_inner(json)
    }

    fn to_json(&self) -> PyResult<String> {
        let mut json = json!({});
        let inner = self.inner()?;
        inner.algebra().to_json(&mut json);
        inner.to_json(&mut json);
        Ok(json.to_string())
    }

    #[getter]
    fn get_max_degree(&self) -> PyResult<i32> {
        Ok(self.inner()?.max_degree())
    }
}


#[pymethods]
impl FDModule {
    pub fn add_generator(&mut self, degree: i32, name: String) -> PyResult<()> {
        self.check_degree(degree)?;
        let inner = self.inner_mut()?; 
        if degree < inner.max_degree() {
            Err(exceptions::ValueError::py_err(format!(
                "Degree {} is less than max_degree {}. Must add generators in increasing order of degree.",
                degree, inner.max_degree()
            )))
        } else {
            Ok(inner.add_generator(degree, name))
        }
    }

    pub fn set_action_vector(
        &mut self,
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
        &mut self,
        operation_degree: i32,
        operation_idx: usize,
        input_degree: i32,
        input_idx: usize,
    ) -> PyResult<FpVector> {
        let owner = self.owner();
        Ok(FpVector::wrap(self.inner_mut()?.action_mut(operation_degree, operation_idx, input_degree, input_idx), owner))
    }

    #[args(overwrite=true)]
    pub fn parse_action(
        &mut self,
        entry_: &str,
        gen_to_idx_from_py : PyObject, // How can I make this default to PyNone?        
        overwrite: bool,
    ) -> PyResult<()> {
        let inner = self.inner_mut()?;
        let mut gen_to_idx : HashMap<String, (i32, usize)>;
        if gen_to_idx_from_py.is_none() {
            gen_to_idx = HashMap::new();
            for degree in inner.min_degree()..=inner.max_degree() {
                for idx in 0 .. inner.dimension(degree){
                    gen_to_idx.insert(
                        inner.basis_element_to_string(degree, idx),
                        (degree, idx)
                    );
                }
            }
        } else {
            let gil = Python::acquire_gil();
            let py = gil.python();
            gen_to_idx = gen_to_idx_from_py.extract(py)?;
        }
        inner.parse_action(&gen_to_idx, entry_, overwrite)
            .map_err(|err| exceptions::ValueError::py_err(
                format!("Error parsing: {}", err)
            ))
    }

    pub fn set_basis_element_name(&mut self, degree: i32, idx: usize, name: String) -> PyResult<()> {
        self.inner_mut()?.set_basis_element_name(degree, idx, name);
        Ok(())
    }

    pub fn check_validity_in_degree(
        &self,
        input_deg: i32,
        output_deg: i32,
    ) -> PyResult<()> {
        self.inner()?.check_validity(input_deg, output_deg)
            .map_err(|err| exceptions::ValueError::py_err(format!(
                "{}", err
            )))
    }

    pub fn check_validity(&self) -> PyResult<()> {
        let inner = self.inner()?;
        for input_degree in inner.min_degree() .. inner.max_degree() {
            for output_degree in input_degree + 1 ..= inner.max_degree(){
                self.check_validity_in_degree(input_degree, output_degree)?;
            }
        }
        Ok(())
    }

    pub fn extend_actions_in_degree(&mut self, input_deg: i32, output_deg: i32) -> PyResult<()> {
        self.inner_mut()?.extend_actions(input_deg, output_deg);
        Ok(())
    }

    pub fn extend_actions(&mut self) -> PyResult<()> {
        let inner = self.inner_mut()?;
        for input_degree in inner.min_degree() .. inner.max_degree() {
            for output_degree in input_degree + 1 ..= inner.max_degree(){
                inner.extend_actions(input_degree, output_degree);
            }
        }
        Ok(())
    }
    
    pub fn freeze(&mut self) -> PyResult<()> {
        self.ensure_immutable()
    }
}