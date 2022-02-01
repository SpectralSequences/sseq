// #![allow(unused_macros)]

use derive_more::{Display};

use std::sync::Arc;

use algebra::{
    Algebra,
    AdemAlgebra as AdemAlgebraRust, 
    MilnorAlgebra as MilnorAlgebraRust,
    SteenrodAlgebraT as SteenrodAlgebraRustT,
    SteenrodAlgebraBorrow as SteenrodAlgebraBorrowRust,
    GeneratedAlgebra,
    JsonAlgebra
};

use crate::algebra::{
    AdemAlgebra, 
    MilnorAlgebra,
    PythonAlgebra,
    PythonAlgebraRust
};

use pyo3::{
    prelude::*,
    PyErr
};


// For some reason the enum_dispatch doesn't work right?
#[derive(Display)]
pub enum AlgebraRust {
    AdemAlgebraRust(AdemAlgebraRust),
    MilnorAlgebraRust(MilnorAlgebraRust),
    PythonAlgebraRust(PythonAlgebraRust)
}

impl SteenrodAlgebraRustT for AlgebraRust {
    fn steenrod_algebra(&self) -> SteenrodAlgebraBorrowRust {
        match self {
            AlgebraRust::AdemAlgebraRust(a) => SteenrodAlgebraBorrowRust::BorrowAdem(a),
            AlgebraRust::MilnorAlgebraRust(a) => SteenrodAlgebraBorrowRust::BorrowMilnor(a),
            _ => panic!("Invalid algebra type: to_steenrod_algebra requires a AdemAlgebra or a MilnorAlgebra.")
        }
    }
}

impl GeneratedAlgebra for AlgebraRust {
    fn generators(&self, x: i32) -> Vec<usize> { 
        match self {
            AlgebraRust::AdemAlgebraRust(a) => a.generators(x),
            AlgebraRust::MilnorAlgebraRust(a) => a.generators(x),
            _ => panic!("Invalid algebra type: generators requires a AdemAlgebra or a MilnorAlgebra.")
        }
    }
    fn string_to_generator<'a, 'b>(&'a self, input: &'b str) -> nom::IResult<&'b str, (i32, usize)> {
        match self {
            AlgebraRust::AdemAlgebraRust(a) => a.string_to_generator(input),
            AlgebraRust::MilnorAlgebraRust(a) => a.string_to_generator(input),
            _ => panic!("Invalid algebra type: string_to_generator requires a AdemAlgebra or a MilnorAlgebra.")
        }
    }
    fn decompose_basis_element(&self, x: i32, y: usize) -> Vec<(u32, (i32, usize), (i32, usize))> {
        match self {
            AlgebraRust::AdemAlgebraRust(a) => a.decompose_basis_element(x, y),
            AlgebraRust::MilnorAlgebraRust(a) => a.decompose_basis_element(x, y),
            _ => panic!("Invalid algebra type: decompose_basis_element requires a AdemAlgebra or a MilnorAlgebra.")
        }
    }
    fn generating_relations(&self, x: i32) -> Vec<Vec<(u32, (i32, usize), (i32, usize))>> {
        match self {
            AlgebraRust::AdemAlgebraRust(a) => a.generating_relations(x),
            AlgebraRust::MilnorAlgebraRust(a) => a.generating_relations(x),
            _ => panic!("Invalid algebra type: generating_relations requires a AdemAlgebra or a MilnorAlgebra.")
        }
    }
}

impl JsonAlgebra for AlgebraRust {
    fn prefix(&self) -> &str { todo!() }
    fn json_to_basis(&self, _: &serde_json::value::Value) -> anyhow::Result<(i32, usize)> { todo!() }
    fn json_from_basis(&self, _: i32, _: usize) -> serde_json::value::Value { todo!() }
}

impl AlgebraRust {
    pub fn into_py_object(algebra : Arc<AlgebraRust>) -> PyObject {
        let gil = Python::acquire_gil();
        let py = gil.python();
        match *algebra {
            AlgebraRust::AdemAlgebraRust(_) => AdemAlgebra::wrap_immutable(algebra).into_py(py),
            AlgebraRust::MilnorAlgebraRust(_) => MilnorAlgebra::wrap_immutable(algebra).into_py(py),
            AlgebraRust::PythonAlgebraRust(_) => PythonAlgebra::wrap_immutable(algebra).into_py(py),
        }
    }
    
    pub fn from_py_object(algebra : PyObject) -> PyResult<Arc<AlgebraRust>> {
        let gil = Python::acquire_gil();
        let py = gil.python();
        Err(python_utils::exception!(RuntimeError, "Dummy"))
            .or_else(|_err : PyErr| Ok((&algebra).extract::<AdemAlgebra>(py)?.to_arc()?))
            .or_else(|_err : PyErr| Ok((&algebra).extract::<MilnorAlgebra>(py)?.to_arc()?))
            .or_else(|_err : PyErr| Ok((&algebra).extract::<PythonAlgebra>(py)?.to_arc()?))
            .map( |a| a.clone())
            .map_err(|_err : PyErr| {
                python_utils::exception!(TypeError,
                    "Invalid algebra!"
                )
            })
    }
}


macro_rules! dispatch_algebra_rust {
    () => {};
    ($vis:vis fn $method:ident$(<$($lt:lifetime),+>)?(&$($lt2:lifetime)?self$(, $arg:ident: $ty:ty )*$(,)?) $(-> $ret:ty)?; $($tail:tt)*) => {
        $vis fn $method$(<$($lt),+>)?(&$($lt2)?self, $($arg: $ty),* ) $(-> $ret)* {
            match self {
                AlgebraRust::AdemAlgebraRust(alg) => alg.$method($($arg),*),
                AlgebraRust::MilnorAlgebraRust(alg) => alg.$method($($arg),*),
                AlgebraRust::PythonAlgebraRust(alg) => alg.$method($($arg),*)
            }
        }
        dispatch_algebra_rust!{$($tail)*}
    };
}

algebra::dispatch_algebra!{AlgebraRust, dispatch_algebra_rust}


// #[derive(Debug)]
// struct InvalidAlgebraError {
//     name : String
// }

// impl std::fmt::Display for InvalidAlgebraError {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "Invalid algebra: {}", &self.name)
//     }
// }

// impl Error for InvalidAlgebraError {
//     fn description(&self) -> &str {
//         "Invalid algebra supplied"
//     }
// }
