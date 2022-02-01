use pyo3::{
    prelude::*,
    PyObjectProtocol,
    types::PyTuple
};

// use std::fs::File;
// use std::io::BufReader;
// use std::sync::Arc;

// use serde_json::{json, Value};

use python_utils;
use rustc_hash::FxHashMap as HashMap;


use bivec::BiVec;
use algebra::Algebra as AlgebraT;
use algebra::module::{FDModule as FDModuleRust, Module, BoundedModule};

use python_fp::vector::FpVector;
use crate::algebra::AlgebraRust;
use crate::module::module_rust::ModuleRust;

crate::module_bindings!(FDModule, FDModuleRust, FDModuleElement);

python_utils::py_repr!(FDModule, inner, "FreedFDModule", {
    Ok(format!(
        "FDModule(p={})",
        inner.prime(),
    ))
});


impl FDModule {
    // fn from_json_inner(mut json: Value) -> PyResult<Self> {
    //     let algebra = AlgebraRust::from_json(&json, "adem".to_string())
    //         .map_err(|e| 
    //             python_utils::exception!(ValueError, "Failed to construct algebra: {}", e)
    //         )?;
    //     let algebra = Arc::new(algebra);
    //     let module = FDModuleRust::from_json(algebra, &mut json)
    //         .map_err(|e| {
    //             python_utils::exception!(ValueError, "Failed to construct module: {}", e)
    //         })?;
    //     Ok(Self::box_and_wrap(module))
    // }
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

    // #[staticmethod]
    // fn from_file(path: String) -> PyResult<Self> {
    //     let f =
    //         File::open(path).map_err(|e| 
    //             python_utils::exception!(IOError, "Failed to open file: {}", e)
    //         )?;

    //     let json = serde_json::from_reader(BufReader::new(f))
    //         .map_err(|e| 
    //             python_utils::exception!(ValueError, "Failed to parse json: {}", e)
    //         )?;

    //     Self::from_json_inner(json)
    // }

    // #[staticmethod]
    // fn from_json(json: String) -> PyResult<Self> {
    //     let json = serde_json::from_str(&json)
    //         .map_err(|e| 
    //             python_utils::exception!(ValueError, "Failed to parse json: {}", e)
    //         )?;

    //     Self::from_json_inner(json)
    // }

    // fn to_json(&self) -> PyResult<String> {
    //     let mut json = json!({});
    //     let inner = self.inner()?;
    //     inner.algebra().to_json(&mut json);
    //     inner.to_json(&mut json);
    //     Ok(json.to_string())
    // }

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
            Err(python_utils::exception!(ValueError,
                "Degree {} is less than max_degree {}. Must add generators in increasing order of degree.",
                degree, inner.max_degree()
            ))
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

    
    #[args(overwrite=true, pyargs="*")]
    pub fn parse_action(
        &mut self,          
        entry_: &str,    // mandatory
        overwrite: bool, // default = True
        pyargs : &PyTuple// gen_to_idx_from_py : PyDict { str => (int, int) }
    ) -> PyResult<()> {
        python_utils::check_number_of_positional_arguments!("parse_action", 2, 4, 3+pyargs.len())?;
        let inner = self.inner_mut()?;
        let mut gen_to_idx : HashMap<String, (i32, usize)>;
        if pyargs.is_empty() {
            gen_to_idx = HashMap::default();
            for degree in inner.min_degree()..=inner.max_degree() {
                for idx in 0 .. inner.dimension(degree){
                    gen_to_idx.insert(
                        inner.basis_element_to_string(degree, idx),
                        (degree, idx)
                    );
                }
            }
        } else {
            gen_to_idx = pyargs.get_item(0).extract()
                        .map_err(|_err| python_utils::exception!(TypeError,
                            "gen_to_idx_from_py is expected to be a dictionary of type {{ str : (int, str) }}",
                        ))?;
        }
        inner.parse_action(&gen_to_idx, entry_, overwrite)
            .map_err(|err| python_utils::exception!(ValueError,
                "Error parsing: {}", err
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
            .map_err(|err| python_utils::exception!(ValueError,
                "{}", err
            ))
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
}