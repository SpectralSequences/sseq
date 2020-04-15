use std::sync::Arc;

use std::collections::HashMap;


use once::OnceVec;
use fp::prime::ValidPrime;
use fp::vector::{FpVector, FpVectorT};

use crate::algebra::combinatorics::TruncatedPolynomialPartitions;
use crate::algebra::AdemAlgebraT;
use crate::module::Module;
// use bivec::BiVec;

#[derive(Clone, Eq, PartialEq)]
pub struct UnstableAlgebraMonomial {
    degree : i32,
    poly : FpVector,
    ext : FpVector
}

impl UnstableAlgebraMonomial {
    pub fn temp(p : ValidPrime) -> Self {
        Self {
            degree : 0xFEDCBCA, // Looks invalid to me!
            poly : FpVector::new(p, 0),
            ext : FpVector::new(ValidPrime::new(2), 0)
        }
    }
}

pub struct UnstableAlgebraTableEntry {    
    index_to_monomial : Vec<UnstableAlgebraMonomial>, // degree -> index -> AdemBasisElement
    monomial_to_index : HashMap<UnstableAlgebraMonomial, usize>, // degree -> AdemBasisElement -> index
}

impl UnstableAlgebraTableEntry {
    pub fn new() -> Self {
        Self {
            index_to_monomial : Vec::new(),
            monomial_to_index : HashMap::new()
        }
    }
}

impl std::hash::Hash for UnstableAlgebraMonomial {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.poly.hash(state);
        self.ext.hash(state);
    }
}

pub trait UnstableAlgebra {
    type Algebra : AdemAlgebraT;

    fn algebra_inner(&self) -> Arc<Self::Algebra>;
    fn name_inner(&self) -> String;
    fn polynomial_partitions(&self) -> &TruncatedPolynomialPartitions;
    fn exterior_partitions(&self) -> &TruncatedPolynomialPartitions;
    
    fn min_degree(&self) -> i32 { 0 }

    fn polynomial_generators_in_degree(&self, degree : i32) -> usize;
    fn exterior_generators_in_degree(&self, degree : i32) -> usize;

    fn basis_table(&self) -> &OnceVec<UnstableAlgebraTableEntry>;
    // Maybe later could allow frobenius to not give another generator?
    fn frobenius_on_generator(&self, degree : i32, index : usize) -> Option<usize>; 
    fn compute_generating_set(&self, degree : i32);
    
    
    
    fn monomial_to_index(&self, monomial : &UnstableAlgebraMonomial) -> Option<usize> {
        self.basis_table()[monomial.degree as usize].monomial_to_index.get(monomial).map(|x| *x)
    }
    
    fn index_to_monomial(&self, degree : i32, index : usize) -> &UnstableAlgebraMonomial {
        &self.basis_table()[degree as usize].index_to_monomial[index]
    }

    
    fn generator_to_monomial_action(&self, result : &mut UnstableAlgebraMonomial, op_deg : i32, op_index : usize, input_degree : i32, input_idx : usize) {

    }

    fn monomial_to_monomial_action(&self, 
        result : &mut UnstableAlgebraMonomial, 
        op_deg : i32, 
        op_index : usize, 
        input: &UnstableAlgebraMonomial
    ) {
        
    }
    
    fn monomial_to_polynomial_action(&self,
        result : &mut FpVector, temp_monomial : &mut UnstableAlgebraMonomial, c : u32,  op_deg : i32, op_index : usize, input: &UnstableAlgebraMonomial
    ) {
        temp_monomial.poly.extend_dimension(self.polynomial_partitions().generators_up_to_degree(input.degree + op_deg));
        temp_monomial.ext.extend_dimension(self.exterior_partitions().generators_up_to_degree(input.degree + op_deg));
        temp_monomial.poly.set_to_zero_pure();
        temp_monomial.ext.set_to_zero_pure();
        self.monomial_to_monomial_action(temp_monomial, op_deg, op_index, input);
        let index = self.monomial_to_index(temp_monomial).unwrap();
        result.add_basis_element(index, c);
    }

    fn multiply_monomials(&self, target : &mut UnstableAlgebraMonomial, source : &UnstableAlgebraMonomial){
        
    }

    fn frobenius_multiply_monomials(&self, target : &mut UnstableAlgebraMonomial, source : &UnstableAlgebraMonomial) {

    }
}


impl<Adem : AdemAlgebraT, A: UnstableAlgebra<Algebra=Adem> + Send + Sync + 'static> Module for A {
    type Algebra = Adem;
    fn algebra(&self) -> Arc<Self::Algebra> {
        self.algebra_inner()
    }

    fn name(&self) -> String {
        self.name_inner()
    }

    fn min_degree(&self) -> i32 { 0 }

    fn max_computed_degree(&self) -> i32 {
        self.basis_table().len() as i32 - 1
    }


    fn dimension(&self, degree: i32) -> usize {
        self.basis_table()[degree as usize].index_to_monomial.len()
    }


    fn compute_basis(&self, degree : i32) {
        self.compute_generating_set(degree);
        for i in self.max_computed_degree() + 1 ..= degree {
            let num_poly_gens = self.polynomial_generators_in_degree(i);
            let num_ext_gens = self.exterior_generators_in_degree(i);
            let poly_parts = self.polynomial_partitions();
            let ext_parts = self.exterior_partitions();
            poly_parts.add_gens_and_calculate_parts(degree, num_poly_gens);
            ext_parts.add_gens_and_calculate_parts(degree, num_ext_gens);
            let mut table = UnstableAlgebraTableEntry::new();
            for poly_deg in 0 ..= degree {
                let ext_deg = degree - poly_deg;
                for p in poly_parts.parts(poly_deg) {
                    for e in ext_parts.parts(ext_deg) {
                        let index = table.index_to_monomial.len();
                        let m = UnstableAlgebraMonomial {
                            degree,
                            poly : p.clone(),
                            ext : e.clone()
                        };
                        table.monomial_to_index.insert(m.clone(), index);
                        table.index_to_monomial.push(m);
                    }
                }
            }
            self.basis_table().push(table);
        }
    }

    fn basis_element_to_string(&self, _degree: i32, _index: usize) -> String {
        "".to_string()
    }


    fn act_on_basis(&self,
        result : &mut FpVector, c : u32,  op_deg : i32, op_index : usize, input_degree : i32, input_index : usize
    ) {
        let mut temp_monomial = UnstableAlgebraMonomial::temp(self.prime());
        let mono = self.index_to_monomial(input_degree, input_index);
        self.monomial_to_polynomial_action(result, &mut temp_monomial, c, op_deg, op_index, mono);
    }

    fn act(&self, result : &mut FpVector, coeff : u32, op_degree : i32, op_index : usize, input_degree : i32, input : &FpVector){
        let _scratch_summand = UnstableAlgebraMonomial::temp(self.prime());
        let _scratch_carry = UnstableAlgebraMonomial::temp(self.prime());
        let p = self.prime();
        for (i, v) in input.iter_nonzero() {
            self.act_on_basis(
                result,
                (coeff * v) % *p,
                op_degree,
                op_index,
                input_degree,
                i,
            );
        }
    }

    fn act_by_element(
        &self,
        result: &mut FpVector,
        coeff: u32,
        op_degree: i32,
        op: &FpVector,
        input_degree: i32,
        input: &FpVector,
    ) {
        assert_eq!(input.dimension(), self.dimension(input_degree));
        let p = self.prime();
        for (i, v) in op.iter_nonzero() {
            self.act(result, (coeff * v) % *p, op_degree, i, input_degree, input);
        }
    }


}