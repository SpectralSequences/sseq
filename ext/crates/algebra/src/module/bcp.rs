#![cfg_attr(rustfmt, rustfmt_skip)]
use std::sync::Arc;

use once::OnceVec;
use fp::prime::ValidPrime;
use fp::vector::{FpVector, FpVectorT};

use crate::algebra::combinatorics::TruncatedPolynomialMonomialBasis;
use crate::algebra::AdemAlgebraT;
use crate::algebra::{PolynomialAlgebra, PolynomialAlgebraMonomial, PolynomialAlgebraTableEntry};
use crate::module::PolynomialAlgebraModule;

pub struct BCp<A : AdemAlgebraT> {
    algebra : Arc<A>,
    polynomial_monomials_field : TruncatedPolynomialMonomialBasis,
    exterior_monomials_field : TruncatedPolynomialMonomialBasis,
    basis_table_field : OnceVec<PolynomialAlgebraTableEntry>,
    action_table_field : OnceVec<Vec<Vec<FpVector>>>,
    bockstein_table_field : OnceVec<Vec<FpVector>>,
}

impl<A : AdemAlgebraT> BCp<A> {
    pub fn new(algebra : Arc<A>) -> Self {
        let p = algebra.prime();
        Self {
            algebra,
            polynomial_monomials_field : TruncatedPolynomialMonomialBasis::new(p), 
            exterior_monomials_field : TruncatedPolynomialMonomialBasis::new(ValidPrime::new(2)),
            basis_table_field : OnceVec::new(),
            action_table_field : OnceVec::new(),
            bockstein_table_field : OnceVec::new()
        }
    }
}

fn is_two_times_power_of_p(p : i32, generic : bool, mut degree : i32) -> bool {
    let q = if generic { 2 } else { 1 };
    if degree % q != 0 {
        return false;
    }
    degree /= q;
    // let mut pow = 0;
    while degree > 1 && degree % p == 0 {
        degree /= p;
        // pow += 1;
    }
    degree == 1
}

impl<A : AdemAlgebraT> PolynomialAlgebra for BCp<A> {
    fn prime(&self) -> ValidPrime {
        self.algebra().prime()
    }
    
    fn name(&self) -> String {
        format!("BC{}", *self.prime())
    }

    fn polynomial_monomials(&self) -> &TruncatedPolynomialMonomialBasis {
        &self.polynomial_monomials_field
    }

    fn exterior_monomials(&self) -> &TruncatedPolynomialMonomialBasis {
        &self.exterior_monomials_field
    }

    fn polynomial_generators_in_degree(&self, degree : i32) -> usize {
        if is_two_times_power_of_p(*self.prime() as i32, self.algebra().adem_algebra().generic, degree) { 1 } else { 0 }
    }

    fn exterior_generators_in_degree(&self, degree : i32) -> usize {
        if degree == 1 && self.algebra().adem_algebra().generic { 1 } else { 0 }
    }

    fn repr_poly_generator(&self, degree : i32, _index : usize) -> (String, u32) {
        let q = if self.algebra().adem_algebra().generic { 2 } else { 1 };
        ("x".to_string(), (degree / q) as u32)
    }

    fn repr_ext_generator(&self, _degree : i32, _index : usize) -> String {
        "a".to_string()
    }


    fn frobenius_on_generator(&self, _degree : i32, _index : usize) -> Option<usize> {
        Some(0)
    }

    fn compute_generating_set(&self, _degree : i32) {}

    fn basis_table(&self) -> &OnceVec<PolynomialAlgebraTableEntry> {
        &self.basis_table_field
    }
}

impl<A : AdemAlgebraT> PolynomialAlgebraModule for BCp<A> {
    type Algebra = A;

    fn algebra(&self) -> Arc<Self::Algebra> {
        self.algebra.clone()
    }

    fn action_table(&self) -> &OnceVec<Vec<Vec<FpVector>>> {// degree -> square -> monomial idx -> result vector
        &self.action_table_field
    } 

    fn bockstein_table(&self) -> &OnceVec<Vec<FpVector>> {
        &self.bockstein_table_field
    }

    fn sq_polynomial_generator_to_monomial(&self, result : &mut PolynomialAlgebraMonomial, sq : i32, input_degree : i32, input_idx : usize) {
        let p = *self.prime() as i32;
        // let q = self.algebra.adem_algebra().q();
        if is_two_times_power_of_p(p, self.algebra().adem_algebra().generic, input_degree) && sq == input_degree {
            result.degree = p*input_degree;
            let int_idx = self.polynomial_monomials().gen_deg_idx_to_internal_idx(
                result.degree, 
                self.frobenius_on_generator(input_degree, input_idx).unwrap()
            );
            result.poly.set_entry(int_idx, 1);
        } else {
            result.valid = false;
        }
    }

    fn sq_exterior_generator_to_monomial(&self, result : &mut PolynomialAlgebraMonomial, _sq : i32, _input_degree : i32, _input_idx : usize) {
        // assert!(false);
        result.valid = false;
    }
    
    fn bockstein_polynomial_generator_to_monomial(&self, result : &mut PolynomialAlgebraMonomial, _input_degree : i32, _input_idx : usize) {
        result.valid = false;
    }
    
    fn bockstein_exterior_generator_to_monomial(&self, result : &mut PolynomialAlgebraMonomial, _input_degree : i32, _input_idx : usize) {
        result.poly.set_entry(0, 1);
        // result.ext.set_entry(0, 0);
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use crate::algebra::AdemAlgebra;
    use crate::module::Module;

    #[rstest(p, case(2), case(3), case(5))]
    #[ignore]
    fn test_bcp(p : u32){
        let p_ = ValidPrime::new(p);
        let algebra = Arc::new(AdemAlgebra::new(p_, p != 2, false, false));
        let bcp = BCp::new(algebra);
        bcp.compute_basis(20);
    }
}
