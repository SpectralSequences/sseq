use enum_dispatch::enum_dispatch;

use fp::vector::FpVector;
use algebra::{
    Algebra,
    AdemAlgebra as AdemAlgebraRust, 
    MilnorAlgebra as MilnorAlgebraRust
};


use crate::python_algebra::PythonAlgebraRust;

// For some reason the enum_dispatch doesn't work right?
#[enum_dispatch(Algebra)]
pub enum AlgebraRust {
    AdemAlgebraRust,
    MilnorAlgebraRust,
    PythonAlgebraRust
}

macro_rules! because_enum_dispatch_doesnt_work_for_me {
    ($self_ : expr, $method : ident, $( $args : ident ),*) => {
        match $self_ {
            AlgebraRust::AdemAlgebraRust(alg) => alg.$method($($args),*),
            AlgebraRust::MilnorAlgebraRust(alg) => alg.$method($($args),*),
            AlgebraRust::PythonAlgebraRust(alg) => alg.$method($($args),*)
        }
    };
}

impl Algebra for AlgebraRust {
    fn algebra_type(&self) -> &str {
        because_enum_dispatch_doesnt_work_for_me!(self, algebra_type,)
    }

    fn prime(&self) -> fp::prime::ValidPrime {
        because_enum_dispatch_doesnt_work_for_me!(self, prime,)
    }

    fn compute_basis(&self, degree : i32) {
        because_enum_dispatch_doesnt_work_for_me!(self, compute_basis, degree)
    }

    fn max_degree(&self) -> i32 {
        because_enum_dispatch_doesnt_work_for_me!(self, max_degree, )
    }

    fn dimension(&self, degree : i32, excess : i32) -> usize {
        because_enum_dispatch_doesnt_work_for_me!(self, dimension, degree, excess)
    }

    fn multiply_basis_elements(&self, result : &mut FpVector, coeff : u32, 
        r_deg : i32, r_idx : usize,
        s_deg : i32, s_idx : usize,
        excess : i32
    ){
        because_enum_dispatch_doesnt_work_for_me!(self, multiply_basis_elements, result, coeff, r_deg, r_idx, s_deg, s_idx, excess)
    }
}
