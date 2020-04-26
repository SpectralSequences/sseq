use std::sync::Arc;

use once::OnceVec;
use fp::prime::ValidPrime;
use fp::vector::{FpVector, FpVectorT};

use crate::algebra::{
    combinatorics::TruncatedPolynomialMonomialBasis,
    Algebra, AdemAlgebraT, AdemAlgebra,
    PolynomialAlgebra, PolynomialAlgebraMonomial, PolynomialAlgebraTableEntry
};
use crate::module::PolynomialAlgebraModule;

pub struct Dickson2<A : AdemAlgebraT> {
    algebra : Arc<A>,
    pub n : i32,
    polynomial_monomials_field : TruncatedPolynomialMonomialBasis,
    exterior_monomials_field : TruncatedPolynomialMonomialBasis,
    basis_table_field : OnceVec<PolynomialAlgebraTableEntry>,
    action_table_field : OnceVec<Vec<Vec<FpVector>>>, // total_degree ==> sq ==> input_idx ==> result
    quadratic_terms_field : Vec<Option<(i32,i32)>> // degree ==> (d1, d2)
}

impl<A : AdemAlgebraT> Dickson2<A> {
    pub fn new(algebra : Arc<A>, n : i32) -> Self {
        let p = algebra.prime();
        let mut quadratic_terms_field = vec![None; (1 << (n + 2)) - 1];
        for k1 in 0 ..= n {
            let l1 = n - k1;
            let d1 = ((1 << l1) - 1) << k1;
            for k2 in 0 ..= n {
                let l2 = n - k2;
                let d2 = ((1 << l2) - 1) << k2;
                let total = d1 + d2;
                let (d1, d2) = if d2 < d1 {
                    (d2, d1)
                } else {
                    (d1, d2)
                };
                quadratic_terms_field[total as usize] = Some((d1, d2));
            }
        }
        Self {
            algebra,
            n,
            polynomial_monomials_field : TruncatedPolynomialMonomialBasis::new(p), 
            exterior_monomials_field : TruncatedPolynomialMonomialBasis::new(ValidPrime::new(2)),
            basis_table_field : OnceVec::new(),
            action_table_field : OnceVec::new(),
            quadratic_terms_field
        }
    }

    fn adem_algebra(&self) -> &AdemAlgebra {
        self.algebra.adem_algebra()
    }

    fn quadratic_terms(&self, degree : i32) -> Option<(i32, i32)> {
        let degree = degree as usize;
        if degree >= self.quadratic_terms_field.len() {
            None
        } else {
            self.quadratic_terms_field[degree]
        }
    }

    fn klfrob(&self, degree : i32) -> Option<(i32, i32, i32)>{
        let frob_plus_k = degree.trailing_zeros()  as i32;
        let two_to_the_frob_plus_n = degree + (1 << frob_plus_k);
        if two_to_the_frob_plus_n.count_ones() != 1 {
            None
        } else {
            let frob_plus_n = two_to_the_frob_plus_n.trailing_zeros()  as i32;
            let frob = frob_plus_n - self.n;
            let k = frob_plus_k - frob;
            let l = self.n - k;
            if frob_plus_n < self.n || k < 0 {
                None
            } else {
                Some((k, l, frob))
            }
        }
    }
}

impl<A : AdemAlgebraT> PolynomialAlgebra for Dickson2<A> {
    fn prime(&self) -> ValidPrime {
        self.adem_algebra().prime()
    }
    
    fn name(&self) -> String {
        format!("Dickson(F{},{})", *self.adem_algebra().prime(), self.n)
    }

    fn polynomial_monomials(&self) -> &TruncatedPolynomialMonomialBasis {
        &self.polynomial_monomials_field
    }

    fn exterior_monomials(&self) -> &TruncatedPolynomialMonomialBasis {
        &self.exterior_monomials_field
    }

    fn basis_table(&self) -> &OnceVec<PolynomialAlgebraTableEntry> {
        &self.basis_table_field
    }

    fn polynomial_generators_in_degree(&self, degree : i32) -> usize {
        if degree == 0 {
            return 0;
        }
        if self.klfrob(degree).is_some() {
            1
        } else {
            0
        }
    }

    fn exterior_generators_in_degree(&self, _degree : i32) -> usize {
        0
    }

    fn repr_poly_generator(&self, degree : i32, index : usize) -> (String, u32) {
        assert!(index == 0);
        let (k, l, frob) = self.klfrob(degree).unwrap();
        let var_str = format!("d_{{{k}, {l}}}", k=k, l=l);
        (var_str, 1<<frob)
    }

    fn repr_ext_generator(&self, _degree : i32, _index : usize) -> String {
        unreachable!()
    }


    fn frobenius_on_generator(&self, _degree : i32, index : usize) -> Option<usize> {
        debug_assert!(index == 0);
        Some(0)
    }

    fn compute_generating_set(&self, _degree : i32) {
        
    }
}

impl<A : AdemAlgebraT> PolynomialAlgebraModule for Dickson2<A> {
    type Algebra = A;

    fn algebra(&self) -> Arc<Self::Algebra> {
        self.algebra.clone()
    }

    fn action_table(&self) -> &OnceVec<Vec<Vec<FpVector>>> {// degree -> square -> monomial idx -> result vector
        &self.action_table_field
    } 

    fn bockstein_table(&self) -> &OnceVec<Vec<FpVector>> {
        unreachable!()
    }

    fn sq_polynomial_generator_to_monomial(&self, result : &mut PolynomialAlgebraMonomial, sq : i32, input_degree : i32, input_index : usize) {
        debug_assert!(input_index == 0);
        let (_k, _l, frob) = self.klfrob(input_degree).unwrap();
        if (sq.trailing_zeros() as i32) < frob {
            result.valid = false;
            return;
        }
        let sq = sq >> frob;
        let degree = input_degree >> frob;
        if let Some((a, b)) = self.quadratic_terms(sq + degree) {
            if b < degree || (a == degree && b > degree) {
                result.valid = false;
                return;
            }
            if a == b && a != 0 {
                let int_idx = self.polynomial_monomials().gen_deg_idx_to_internal_idx((a + b) << frob, 0);
                result.poly.set_entry(int_idx, 1);
                return;
            }
            if a != 0 {
                let int_idx = self.polynomial_monomials().gen_deg_idx_to_internal_idx(a << frob, 0);
                result.poly.set_entry(int_idx, 1);
            }
            if b != 0 {
                let int_idx = self.polynomial_monomials().gen_deg_idx_to_internal_idx(b << frob, 0);
                result.poly.set_entry(int_idx, 1);
            }
            return;
        }
        result.valid = false;
    }

    fn sq_exterior_generator_to_monomial(&self, _result : &mut PolynomialAlgebraMonomial, _sq : i32, _input_degree : i32, _input_idx : usize) {
        unreachable!();
    }
    
    fn bockstein_polynomial_generator_to_monomial(&self, _result : &mut PolynomialAlgebraMonomial, _input_degree : i32, _input_idx : usize) {
        unreachable!();
    }
    
    fn bockstein_exterior_generator_to_monomial(&self, _result : &mut PolynomialAlgebraMonomial, _input_degree : i32, _input_idx : usize) {
        unreachable!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use crate::algebra::AdemAlgebra;
    use crate::module::Module;

    #[rstest(n => [2])]
    fn test_dickson_basis(n : i32){
        let p = 2;
        let p_ = ValidPrime::new(p);
        let max_degree = 7;
        let algebra = Arc::new(AdemAlgebra::new(p_, p != 2, false, true));
        let dickson = Dickson2::new(algebra, n);
        Module::compute_basis(&dickson, max_degree);
        for d in 0 ..= max_degree {
            println!("degree {}:", d);
            for i in 0..Module::dimension(&dickson, d) {
                println!("  {}", Module::basis_element_to_string(&dickson, d, i));
            }
        }
    }

    #[rstest(
        n => [1, 2, 3, 4, 5]
    )]//, case(3), case(5))]
    fn test_dickson_action(n : i32) {
        let p = 2;
        let p_ = ValidPrime::new(p);
        let algebra = Arc::new(AdemAlgebra::new(p_, p != 2, false, true));
        let dickson = Dickson2::new(algebra.clone(), n);
        dickson.test_relations(5, 5);
    }



}