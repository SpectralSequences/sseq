use enum_dispatch::enum_dispatch;
use nom::IResult;
use std::sync::Arc;

use fp::vector::FpVector;
use algebra::{
    Algebra,
    AdemAlgebra as AdemAlgebraRust, 
    MilnorAlgebra as MilnorAlgebraRust
};

use crate::{adem_algebra::AdemAlgebra, milnor_algebra::MilnorAlgebra, python_algebra::PythonAlgebra};

use pyo3::{prelude::*, exceptions, PyErr};

use crate::python_algebra::PythonAlgebraRust;

// For some reason the enum_dispatch doesn't work right?
#[enum_dispatch(Algebra)]
pub enum AlgebraRust {
    AdemAlgebraRust,
    MilnorAlgebraRust,
    PythonAlgebraRust
}

pub fn algebra_into_py_object(algebra : Arc<AlgebraRust>) -> PyObject {
    let gil = Python::acquire_gil();
    let py = gil.python();
    match *algebra {
        AlgebraRust::AdemAlgebraRust(_) => AdemAlgebra::wrap(algebra).into_py(py),
        AlgebraRust::MilnorAlgebraRust(_) => MilnorAlgebra::wrap(algebra).into_py(py),
        AlgebraRust::PythonAlgebraRust(_) => PythonAlgebra::wrap(algebra).into_py(py),
    }
}

pub fn algebra_from_py_object(algebra : PyObject) -> PyResult<Arc<AlgebraRust>> {
    let gil = Python::acquire_gil();
    let py = gil.python();
    algebra.extract::<&AdemAlgebra>(py).and_then(|a| a.inner())
            .or_else(|_err : PyErr| Ok(algebra.extract::<&MilnorAlgebra>(py)?.inner()?))
            .or_else(|_err : PyErr| Ok(algebra.extract::<&PythonAlgebra>(py)?.inner()?))
            .map( |a| a.clone())
            .map_err(|_err : PyErr| {
                exceptions::ValueError::py_err(format!(
                    "Invalid algebra!"
                ))
            })
}

macro_rules! because_enum_dispatch_doesnt_work_for_me {
    ($method : ident, $self_ : expr, $( $args : ident ),*) => {
        match $self_ {
            AlgebraRust::AdemAlgebraRust(alg) => alg.$method($($args),*),
            AlgebraRust::MilnorAlgebraRust(alg) => alg.$method($($args),*),
            AlgebraRust::PythonAlgebraRust(alg) => alg.$method($($args),*)
        }
    };
}

impl Algebra for AlgebraRust {
    fn algebra_type(&self) -> &str {
        because_enum_dispatch_doesnt_work_for_me!(algebra_type, self,)
    }

    fn prime(&self) -> fp::prime::ValidPrime {
        because_enum_dispatch_doesnt_work_for_me!(prime, self, )
    }

    fn compute_basis(&self, degree : i32) {
        because_enum_dispatch_doesnt_work_for_me!(compute_basis, self, degree)
    }

    fn max_degree(&self) -> i32 {
        because_enum_dispatch_doesnt_work_for_me!(max_degree, self, )
    }

    fn dimension(&self, degree : i32, excess : i32) -> usize {
        because_enum_dispatch_doesnt_work_for_me!(dimension, self, degree, excess)
    }

    fn multiply_basis_elements(&self, result : &mut FpVector, coeff : u32, 
        r_deg : i32, r_idx : usize,
        s_deg : i32, s_idx : usize,
        excess : i32
    ){
        because_enum_dispatch_doesnt_work_for_me!(multiply_basis_elements, self, result, coeff, r_deg, r_idx, s_deg, s_idx, excess)
    }

    fn json_to_basis(&self, json : Value) -> (i32, usize) {
        because_enum_dispatch_doesnt_work_for_me!(json_to_basis, self, json)
    }

    fn json_from_basis(&self, degree : i32, idx : usize) -> Value {
        because_enum_dispatch_doesnt_work_for_me!(json_from_basis, self, degree, idx)
    }

    fn basis_element_to_string(&self, degree : i32, idx : usize) -> String {
        because_enum_dispatch_doesnt_work_for_me!(basis_element_to_string, self, degree, idx)
    }

    fn generators(&self, degree : i32) -> Vec<usize> { 
        because_enum_dispatch_doesnt_work_for_me!(generators, self, degree)
    }

    fn string_to_generator<'a, 'b>(&'a self, input: &'b str) -> IResult<&'b str, (i32, usize)> { 
        because_enum_dispatch_doesnt_work_for_me!(string_to_generator, self, input)
    }

    fn decompose_basis_element(&self, degree : i32, idx : usize) -> Vec<(u32, (i32, usize), (i32, usize))> {
        because_enum_dispatch_doesnt_work_for_me!(decompose_basis_element, self, degree, idx)
    }

    
    fn relations_to_check(&self, degree : i32) -> Vec<Vec<(u32, (i32, usize), (i32, usize))>> { 
        because_enum_dispatch_doesnt_work_for_me!(relations_to_check, self, degree)
    }

    
}

use serde::Deserialize;
use serde_json::Value;
use std::error::Error;

use fp::prime::ValidPrime;

#[derive(Deserialize, Debug)]
struct MilnorProfileOption {
    truncated : Option<bool>,
    q_part : Option<u32>,
    p_part : Option<Vec<u32>>
}

#[derive(Deserialize, Debug)]
struct AlgebraSpec {
    p : u32,
    algebra : Option<Vec<String>>,
    profile : Option<MilnorProfileOption>
}

impl AlgebraRust {
    pub fn from_json(json : &Value, mut algebra_name : String) -> Result<Self, Box<dyn Error>> {
        let spec : AlgebraSpec = serde_json::from_value(json.clone())?;
    
        let p = ValidPrime::new(spec.p);
        if let Some(mut list) = spec.algebra {
            if !list.contains(&algebra_name) {
                println!("Module does not support algebra {}", algebra_name);
                println!("Using {} instead", list[0]);
                algebra_name = list.remove(0);
            }
        }
    
        let algebra : AlgebraRust;
        match algebra_name.as_ref() {
            "adem" => algebra = AlgebraRust::AdemAlgebraRust(AdemAlgebraRust::new(p, *p != 2, false)),
            "milnor" => {
                let mut algebra_inner = MilnorAlgebraRust::new(p);
                if let Some(profile) = spec.profile {
                    if let Some(truncated) = profile.truncated {
                        algebra_inner.profile.truncated = truncated;
                    }
                    if let Some(q_part) = profile.q_part {
                        algebra_inner.profile.q_part = q_part;
                    }
                    if let Some(p_part) = profile.p_part {
                        algebra_inner.profile.p_part = p_part;
                    }
                }
                algebra = AlgebraRust::MilnorAlgebraRust(algebra_inner);
            }
            _ => { return Err(Box::new(InvalidAlgebraError { name : algebra_name })); }
        };
        Ok(algebra)
    }
    
    pub fn to_json(&self, json: &mut Value) {
        match self {
            AlgebraRust::MilnorAlgebraRust(a) => {
                json["p"] = Value::from(*a.prime());
                json["generic"] = Value::from(a.generic);
    
                if !a.profile.is_trivial() {
                    json["algebra"] = Value::from(vec!["milnor"]);
                    json["profile"] = Value::Object(serde_json::map::Map::with_capacity(3));
                    if a.profile.truncated {
                        json["profile"]["truncated"] = Value::Bool(true);
                    }
                    if a.profile.q_part != !0 {
                        json["profile"]["q_part"] = Value::from(a.profile.q_part);
                    }
                    if !a.profile.p_part.is_empty() {
                        json["profile"]["p_part"] = Value::from(a.profile.p_part.clone());
                    }
                }
            }
            AlgebraRust::AdemAlgebraRust(a) => {
                json["p"] = Value::from(*a.prime());
                json["generic"] = Value::Bool(a.generic);
            }
            _ => panic!()
        }
    }
}

#[derive(Debug)]
struct InvalidAlgebraError {
    name : String
}

impl std::fmt::Display for InvalidAlgebraError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid algebra: {}", &self.name)
    }
}

impl Error for InvalidAlgebraError {
    fn description(&self) -> &str {
        "Invalid algebra supplied"
    }
}
