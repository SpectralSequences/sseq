use std::sync::Arc;

use once::OnceVec;
use fp::prime::ValidPrime;
use fp::vector::{FpVector, FpVectorT};

use crate::algebra::combinatorics::TruncatedPolynomialPartitions;
use crate::algebra::AdemAlgebraT;
use crate::module::{UnstableAlgebra, UnstableAlgebraMonomial, UnstableAlgebraTableEntry};

struct UnstableAlgebraBCp<A : AdemAlgebraT> {
    algebra : Arc<A>,
    polynomial_monomials : TruncatedPolynomialPartitions,
    exterior_monomials : TruncatedPolynomialPartitions,
    basis_table_field : OnceVec<UnstableAlgebraTableEntry>,
    action_table_field : OnceVec<Vec<Vec<FpVector>>>,
    bockstein_table_field : OnceVec<Vec<FpVector>>,
}

impl<A : AdemAlgebraT> UnstableAlgebraBCp<A> {
    pub fn new(algebra : Arc<A>) -> Self {
        let p = algebra.prime();
        Self {
            algebra,
            polynomial_monomials : TruncatedPolynomialPartitions::new(p), 
            exterior_monomials : TruncatedPolynomialPartitions::new(ValidPrime::new(2)),
            basis_table_field : OnceVec::new(),
            action_table_field : OnceVec::new(),
            bockstein_table_field : OnceVec::new()
        }
    }
}

fn is_two_times_power_of_p(p : i32, mut degree : i32) -> bool {
    let q = 2;
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

impl<A : AdemAlgebraT> UnstableAlgebra for UnstableAlgebraBCp<A> {
    type Algebra = A;
    fn algebra_inner(&self) -> Arc<Self::Algebra> {
        self.algebra.clone()
    }

    fn name_inner(&self) -> String {
        format!("BC{}", *self.prime())
    }

    fn polynomial_partitions(&self) -> &TruncatedPolynomialPartitions {
        &self.polynomial_monomials
    }

    fn exterior_partitions(&self) -> &TruncatedPolynomialPartitions {
        &self.exterior_monomials
    }

    fn polynomial_generators_in_degree(&self, degree : i32) -> usize {
        if is_two_times_power_of_p(*self.prime() as i32, degree) { 1 } else { 0 }
    }

    fn exterior_generators_in_degree(&self, degree : i32) -> usize {
        if degree == 1 { 1 } else { 0 }
    }

    fn frobenius_on_generator(&self, _degree : i32, _index : usize) -> Option<usize> {
        Some(0)
    }

    fn compute_generating_set(&self, _degree : i32) {}

    fn nonzero_squares_on_exterior_generator(&self, _gen_degree : i32, _gen_idx : usize) -> Vec<i32> {
        vec![0]
    }

    fn nonzero_squares_on_polynomial_generator(&self, gen_degree : i32, _gen_idx : usize) -> Vec<i32> {
        let q = self.algebra.adem_algebra().q();
        vec![0, gen_degree/q]
    }    

    fn basis_table(&self) -> &OnceVec<UnstableAlgebraTableEntry> {
        &self.basis_table_field
    }

    fn action_table(&self) -> &OnceVec<Vec<Vec<FpVector>>> {// degree -> square -> monomial idx -> result vector
        &self.action_table_field
    } 

    fn bockstein_table(&self) -> &OnceVec<Vec<FpVector>> {
        &self.bockstein_table_field
    }

    fn sq_polynomial_generator_to_monomial(&self, result : &mut UnstableAlgebraMonomial, sq : i32, input_degree : i32, input_idx : usize) {
        let p = *self.prime() as i32;
        // let q = self.algebra.adem_algebra().q();
        if is_two_times_power_of_p(p, input_degree) && sq == input_degree {
            result.degree = p*input_degree;
            let int_idx = self.polynomial_monomials.gen_deg_idx_to_internal_idx(input_degree, input_idx);
            result.poly.set_entry(int_idx, 1);
        } else {
            result.valid = false;
        }
    }

    fn sq_exterior_generator_to_monomial(&self, result : &mut UnstableAlgebraMonomial, _sq : i32, _input_degree : i32, _input_idx : usize) {
        // assert!(false);
        result.valid = false;
    }
    
    fn bockstein_polynomial_generator_to_monomial(&self, result : &mut UnstableAlgebraMonomial, _input_degree : i32, _input_idx : usize) {
        result.valid = false;
    }
    
    fn bockstein_exterior_generator_to_monomial(&self, result : &mut UnstableAlgebraMonomial, _input_degree : i32, _input_idx : usize) {
        result.poly.set_entry(0, 1);
        // result.ext.set_entry(0, 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algebra::AdemAlgebra;
    use crate::module::Module;

    #[test]
    fn test_uabcp(){
        let p = 3;
        let p_ = ValidPrime::new(p);
        let algebra = Arc::new(AdemAlgebra::new(p_, p != 2, false, false));
        let bcp = UnstableAlgebraBCp::new(algebra);
        bcp.compute_basis(20);
    }
}