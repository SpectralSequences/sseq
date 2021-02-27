use std::sync::Arc;

use once::OnceVec;
use fp::prime::ValidPrime;
use fp::vector::{FpVector, FpVectorT};

use crate::algebra::{
    combinatorics::TruncatedPolynomialMonomialBasis,
    Algebra, AdemAlgebraT, AdemAlgebra, adem_algebra::AdemBasisElement,
    PolynomialAlgebra, PolynomialAlgebraMonomial, PolynomialAlgebraTableEntry
};
use crate::module::{Module, PolynomialAlgebraModule};

pub struct KFpn<A : AdemAlgebraT> {
    algebra : Arc<A>,
    pub n : i32,
    polynomial_monomials_field : TruncatedPolynomialMonomialBasis,
    exterior_monomials_field : TruncatedPolynomialMonomialBasis,
    basis_table_field : OnceVec<PolynomialAlgebraTableEntry>,
    action_table_field : OnceVec<Vec<Vec<FpVector>>>, // total_degree ==> sq ==> input_idx ==> result
    bockstein_table_field : OnceVec<Vec<FpVector>>,
    frobenius_table : OnceVec<Vec<usize>>, // degree => idx => idx of x^p.
    inverse_frobenius_table : OnceVec<Vec<(i32, usize, u32)>>, // degree => idx => (root degree, idx, frob_p_power)
}

impl<A : AdemAlgebraT> KFpn<A> {
    pub fn new(algebra : Arc<A>, n : i32) -> Self {
        let p = algebra.prime();
        assert!(algebra.adem_algebra().unstable_enabled);
        Self {
            algebra,
            n,
            polynomial_monomials_field : TruncatedPolynomialMonomialBasis::new(p), 
            exterior_monomials_field : TruncatedPolynomialMonomialBasis::new(ValidPrime::new(2)),
            basis_table_field : OnceVec::new(),
            action_table_field : OnceVec::new(),
            bockstein_table_field : OnceVec::new(),
            frobenius_table : OnceVec::new(),
            inverse_frobenius_table : OnceVec::new()
        }
    }

    fn adem_algebra(&self) -> &AdemAlgebra {
        self.algebra.adem_algebra()
    }

    fn action_on_generator_helper(&self, result : &mut FpVector, coeff : u32, 
        bockstein : u32, sq : u32, input_degree : i32, input_idx : usize
    ) {
        let p = self.adem_algebra().prime();
        let q = self.adem_algebra().q();
        let output_degree = bockstein as i32 + q*sq as i32 + input_degree;
        let output_dim = self.adem_algebra().dimension(output_degree - self.n, self.n);
        let mut temp_vec = FpVector::new(p, output_dim);
        let (left_deg, left_idx) = self.adem_algebra().beps_pn(bockstein, sq);
        let right_deg = input_degree - self.n;
        let right_idx = input_idx;
        let basis_filter = |_, _| true;
        self.adem_algebra().multiply_basis_elements_unstable(
            &mut temp_vec, coeff,
            left_deg, left_idx, right_deg, right_idx,
            self.n, &basis_filter
        );
        let mut mono = PolynomialAlgebraMonomial::new(p);
        self.set_monomial_degree(&mut mono, output_degree);
        if self.adem_algebra().generic && output_degree % 2 == 1 {
            for (idx, c) in temp_vec.iter_nonzero() {
                let gen_index = self.exterior_monomials().gen_deg_idx_to_internal_idx(output_degree, idx);
                mono.ext.set_to_zero_pure();
                mono.ext.set_entry(gen_index, 1);
                let poly_idx = self.monomial_to_index(&mono);
                result.add_basis_element(poly_idx, c);
            }
        } else {            
            for (idx, c) in temp_vec.iter_nonzero() {
                let gen_index = self.polynomial_monomials().gen_deg_idx_to_internal_idx(output_degree, idx);
                mono.poly.set_to_zero_pure();
                mono.poly.set_entry(gen_index, 1);
                let poly_idx = self.monomial_to_index(&mono);
                result.add_basis_element(poly_idx, c);
            }
        }
    }
}

impl<A : AdemAlgebraT> PolynomialAlgebra for KFpn<A> {
    fn prime(&self) -> ValidPrime {
        self.adem_algebra().prime()
    }
    
    fn name(&self) -> String {
        format!("K(F{},{})", *self.adem_algebra().prime(), self.n)
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
        if self.adem_algebra().generic && degree % 2 == 1 { 
            return 0;
        }
        self.adem_algebra().dimension_unstable(degree - self.n, self.n)
    }

    fn exterior_generators_in_degree(&self, degree : i32) -> usize {
        if !self.adem_algebra().generic || degree % 2 == 0 { 
            return 0;
        }
        self.adem_algebra().dimension_unstable(degree - self.n, self.n)
    }

    fn repr_poly_generator(&self, degree : i32, index : usize) -> (String, u32) {
        let two_or_one = if self.adem_algebra().generic { 2 } else { 1 };
        let (root_degree, root_index, pow) = self.inverse_frobenius_table[degree as usize / two_or_one][index];
        let root_degree = root_degree - self.n;
        let var_str = if root_degree == 0 {
            format!("iota{}", self.n)
        } else {
            format!("{}(iota{})", self.adem_algebra().basis_element_to_string(root_degree, root_index), self.n)
        };
        (var_str, pow)
    }

    fn repr_ext_generator(&self, degree : i32, index : usize) -> String {
        let degree = degree - self.n;
        if degree == 0 {
            format!("iota{}", self.n)
        } else {
            format!("{}(iota{})", self.adem_algebra().basis_element_to_string(degree, index), self.n)
        }
    }


    fn frobenius_on_generator(&self, degree : i32, index : usize) -> Option<usize> {
        let p = *Module::prime(self) as i32;
        debug_assert!(p*degree <= Module::max_computed_degree(self), 
            "degree : {}, max_computed_degree : {}", degree, Module::max_computed_degree(self)
        );
        let two_or_one = if self.adem_algebra().generic { 2 } else { 1 };
        Some(self.frobenius_table[degree as usize / two_or_one][index])
    }

    fn compute_generating_set(&self, degree : i32) {
        let p = *self.adem_algebra().prime() as i32;
        let two_or_one = if self.adem_algebra().generic { 2 } else { 1 };
        self.adem_algebra().compute_basis(degree - self.n);
        // OnceVec<Vec<(i32, usize, u32)>> // degree => idx => (root degree, idx, frob_p_power)
        for d in self.inverse_frobenius_table.len() as i32 ..= degree/two_or_one {
            let dim = self.adem_algebra().dimension_unstable(d * two_or_one - self.n, self.n);
            let mut table = Vec::with_capacity(dim);
            for idx in 0 .. dim {
                let b = self.adem_algebra().basis_element_from_index(d * two_or_one - self.n, idx);
                let mut frob_num = 0;
                let mut frob_p_power = 1;
                let mut frob_deg = d;
                for &e in &b.ps {
                    if frob_deg % p != 0 || e as i32 != frob_deg / p {
                        break;
                    }
                    frob_num += 1;
                    frob_deg /= p;
                    frob_p_power *= p as u32;
                }
                let new_idx;
                if frob_num == 0 {
                    new_idx = idx;
                } else {
                    let new_basis_elt = AdemBasisElement {
                        degree : frob_deg * two_or_one - self.n,
                        excess : -1, // not sure what this is, doesn't matter for hash
                        bocksteins : b.bocksteins >> frob_num,
                        ps : b.ps[frob_num..].to_vec(),
                        p_or_sq : true // doesn't matter for hash
                    };
                    new_idx = self.adem_algebra().basis_element_to_index(&new_basis_elt);
                }
                table.push((frob_deg * two_or_one, new_idx, frob_p_power));
            }
            self.inverse_frobenius_table.push(table);
        }
        // degree => idx => idx of x^p.
        let two_p_or_p = p * two_or_one;
        for d in self.frobenius_table.len() as i32 ..= (degree)/two_p_or_p {
            let dim = self.adem_algebra().dimension_unstable(two_or_one * d - self.n, self.n);
            let mut table = Vec::with_capacity(dim);
            for idx in 0 .. dim {
                let b = self.adem_algebra().basis_element_from_index(two_or_one * d - self.n, idx);
                let mut ps = vec![d as u32];
                ps.extend(&b.ps);
                let new_basis_elt = AdemBasisElement {
                    degree : two_or_one * d * p - self.n,
                    excess : self.n, // doesn't matter for hash
                    bocksteins : b.bocksteins << 1,
                    ps,
                    p_or_sq : true // doesn't matter for hash
                };
                let new_idx = self.adem_algebra().basis_element_to_index(&new_basis_elt);
                table.push(new_idx);
            }
            self.frobenius_table.push(table);
        }
    }
}

impl<A : AdemAlgebraT> PolynomialAlgebraModule for KFpn<A> {
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

    fn sq_polynomial_generator_to_monomial(&self, _result : &mut PolynomialAlgebraMonomial, _sq : i32, _input_degree : i32, _input_idx : usize) {
        unreachable!();
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

    fn sq_polynomial_generator_to_polynomial(&self, result : &mut FpVector, coeff : u32, sq : i32, input_degree : i32, input_index : usize) {
        self.action_on_generator_helper(result, coeff, 0, sq as u32, input_degree, input_index);
    }

    fn sq_exterior_generator_to_polynomial(&self, result : &mut FpVector, coeff : u32, sq : i32, input_degree : i32, input_index : usize) {
        self.action_on_generator_helper(result, coeff, 0, sq as u32, input_degree, input_index);
    }

    fn bockstein_polynomial_generator_to_polynomial(&self, result : &mut FpVector, coeff : u32, input_degree : i32, input_index : usize) {
        self.action_on_generator_helper(result, coeff, 1, 0, input_degree, input_index);
    }

    fn bockstein_exterior_generator_to_polynomial(&self, result : &mut FpVector, coeff : u32, input_degree : i32, input_index : usize) {
        self.action_on_generator_helper(result, coeff, 1, 0, input_degree, input_index);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use crate::algebra::AdemAlgebra;
    use crate::module::Module;

    #[rstest(p, case(2), case(3), case(5))]//, case(3), case(5))]
    fn test_kfp1_basis(p : u32){
        let p_ = ValidPrime::new(p);
        let n = 1;
        let max_degree = 20;
        let algebra = Arc::new(AdemAlgebra::new(p_, p != 2, false, true));
        let kfp = KFpn::new(algebra, n);
        Module::compute_basis(&kfp, max_degree);
        for d in 0..max_degree {
            println!("degree {}:", d);
            for i in 0..Module::dimension(&kfp, d) {
                println!("  {}", Module::basis_element_to_string(&kfp, d, i));
            }
        }
        for d in 1..max_degree {
            assert!(Module::dimension(&kfp, d) == 1);
        }
    }


    #[rstest(
        p => [2, 3, 5],
        n => [2, 3, 4]
    )]
    fn test_kfpn_action(p : u32, n : i32){
        let p_ = ValidPrime::new(p);
        let algebra = Arc::new(AdemAlgebra::new(p_, p != 2, false, true));
        let kfpn = KFpn::new(algebra.clone(), n);
        kfpn.test_relations(30, 5);
        // Module::compute_basis(&kfpn, 17);
        // let mut scratch_vec = FpVector::new(p_, 0);
        // let mut discrepancy_vec = FpVector::new(p_, 0);
        // kfpn.check_relation(&mut discrepancy_vec, &mut scratch_vec,
        //     4, 0, 5, 1, 7, 1
        // );
    }


    // #[rstest(p, case(2), case(3), case(5))]//, case(3))]//, case(5)
    #[test]
    fn test_kfpn_b(){
        let p = 3;
        let n = 3;

        let p_ = ValidPrime::new(p);
        let max_degree = 40;
        let algebra = Arc::new(AdemAlgebra::new(p_, p != 2, false, true));
        let kfpn = KFpn::new(algebra, n);
        Module::compute_basis(&kfpn, max_degree);
    }

    #[rstest(p, case(3))]//, case(3), case(5))]//, case(3))]//, case(5)
    fn test_kfp3(p : u32){
        let p_ = ValidPrime::new(p);
        let max_degree = 12;
        let n = 3;
        let algebra = Arc::new(AdemAlgebra::new(p_, p != 2, false, true));
        let kfp = KFpn::new(algebra, n);
        Module::compute_basis(&kfp, max_degree);
        for d in 0..max_degree {
            println!("degree {}:", d);
            for i in 0..Module::dimension(&kfp, d) {
                println!("  {}", Module::basis_element_to_string(&kfp, d, i));
            }
        }        
    }
}
