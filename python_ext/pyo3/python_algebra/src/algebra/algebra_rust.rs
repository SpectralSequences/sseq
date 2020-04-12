// #![allow(unused_macros)]

use enum_dispatch::enum_dispatch;
use std::sync::Arc;

use fp::vector::FpVector;

use algebra::{
    // Algebra,
    AdemAlgebra as AdemAlgebraRust, 
    MilnorAlgebra as MilnorAlgebraRust,
    SteenrodAlgebraT as SteenrodAlgebraRustT,
    SteenrodAlgebraBorrow as SteenrodAlgebraBorrowRust
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

use error;

// For some reason the enum_dispatch doesn't work right?
#[enum_dispatch(Algebra)]
pub enum AlgebraRust {
    AdemAlgebraRust,
    MilnorAlgebraRust,
    PythonAlgebraRust
}

impl SteenrodAlgebraRustT for AlgebraRust {
    fn steenrod_algebra(&self) -> SteenrodAlgebraBorrowRust {
        match self {
            AlgebraRust::AdemAlgebraRust(a) => SteenrodAlgebraBorrowRust::BorrowAdem(&a),
            AlgebraRust::MilnorAlgebraRust(a) => SteenrodAlgebraBorrowRust::BorrowMilnor(&a),
            _ => panic!("Invalid algebra type: to_steenrod_algebra requires a AdemAlgebra or a MilnorAlgebra.")
        }
    }
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
        algebra.extract::<&AdemAlgebra>(py).and_then(|a| a.to_arc())
                .or_else(|_err : PyErr| Ok(algebra.extract::<&MilnorAlgebra>(py)?.to_arc()?))
                .or_else(|_err : PyErr| Ok(algebra.extract::<&PythonAlgebra>(py)?.to_arc()?))
                .map( |a| a.clone())
                .map_err(|_err : PyErr| {
                    python_utils::exception!(TypeError,
                        "Invalid algebra!"
                    )
                })
    }
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

impl algebra::Algebra for AlgebraRust {
    algebra::dispatch_algebra!{because_enum_dispatch_doesnt_work_for_me}
}


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
