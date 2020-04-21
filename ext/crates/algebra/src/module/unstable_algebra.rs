
#![allow(unused_variables)]
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;


use once::OnceVec;
use fp::prime::{ValidPrime, multinomial};
use fp::vector::{FpVector, FpVectorT};

use crate::algebra::combinatorics::{PartitionIterator, TruncatedPolynomialPartitions};
use crate::algebra::{Algebra, AdemAlgebraT};
use crate::module::Module;
// use bivec::BiVec;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnstableAlgebraMonomial {
    pub degree : i32,
    pub poly : FpVector,
    pub ext : FpVector,
    pub valid : bool
}

impl fmt::Display for UnstableAlgebraMonomial {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "UAM(degree={}, valid={}, poly={}, ext={})", self.degree, self.valid, self.poly, self.ext)?;
        Ok(())
    }
}

impl UnstableAlgebraMonomial {
    pub fn new(p : ValidPrime) -> Self {
        Self {
            degree : 0xFEDCBA9, // Looks invalid to me!
            poly : FpVector::new(p, 0),
            ext : FpVector::new(ValidPrime::new(2), 0),
            valid : true
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

pub trait UnstableAlgebra : Sized + Send + Sync + 'static {
    type Algebra : AdemAlgebraT;

    fn algebra_inner(&self) -> Arc<Self::Algebra>;
    fn name_inner(&self) -> String;
    fn polynomial_partitions(&self) -> &TruncatedPolynomialPartitions;
    fn exterior_partitions(&self) -> &TruncatedPolynomialPartitions;
    
    fn min_degree(&self) -> i32 { 0 }

    fn polynomial_generators_in_degree(&self, degree : i32) -> usize;
    fn exterior_generators_in_degree(&self, degree : i32) -> usize;

    fn basis_table(&self) -> &OnceVec<UnstableAlgebraTableEntry>;
    fn action_table(&self) -> &OnceVec<Vec<Vec<FpVector>>>; // degree -> square -> monomial idx -> result vector
    fn bockstein_table(&self) -> &OnceVec<Vec<FpVector>>;

    fn frobenius_on_generator(&self, degree : i32, index : usize) -> Option<usize>; 
    fn compute_generating_set(&self, degree : i32);
    
    fn prime(&self) -> ValidPrime {
        self.algebra_inner().prime()
    }
    
    fn monomial_to_index(&self, monomial : &UnstableAlgebraMonomial) -> Option<usize> {
        self.basis_table()[monomial.degree as usize].monomial_to_index.get(monomial).map(|x| *x)
    }
    
    fn index_to_monomial(&self, degree : i32, index : usize) -> &UnstableAlgebraMonomial {
        &self.basis_table()[degree as usize].index_to_monomial[index]
    }

    fn frobenius_monomial(&self, target : &mut FpVector, source : &FpVector) {
        let p = *self.prime() as i32;
        for (i, c) in source.iter_nonzero() {
            let (degree, in_idx) = self.polynomial_partitions().internal_idx_to_gen_deg(i);
            let frob = self.frobenius_on_generator(degree, in_idx);
            if let Some(e) = frob {
                let out_idx = self.polynomial_partitions().gen_deg_idx_to_internal_idx(p*degree, e);
                target.add_basis_element(out_idx, c);
            }
        }
    }

    fn nonzero_squares_on_polynomial_generator(&self, gen_degree : i32, gen_idx : usize) -> Vec<i32>;
    fn nonzero_squares_on_exterior_generator(&self, gen_degree : i32, gen_idx : usize) -> Vec<i32>;

    fn sq_polynomial_generator_to_monomial(&self, result : &mut UnstableAlgebraMonomial, sq : i32, input_degree : i32, input_idx : usize);
    fn sq_exterior_generator_to_monomial(&self, result : &mut UnstableAlgebraMonomial, sq : i32, input_degree : i32, input_idx : usize);
    fn bockstein_polynomial_generator_to_monomial(&self, result : &mut UnstableAlgebraMonomial, input_degree : i32, input_idx : usize);
    fn bockstein_exterior_generator_to_monomial(&self, result : &mut UnstableAlgebraMonomial, input_degree : i32, input_idx : usize);

    fn sq_polynomial_generator_to_polynomial(&self, result : &mut FpVector, coeff : u32, sq : i32, input_degree : i32, input_idx : usize) {
        let mut temp_monomial = UnstableAlgebraMonomial::new(self.prime());
        let q = self.algebra().adem_algebra().q();
        temp_monomial.degree =  q*sq + input_degree;
        println!("degree : {}", temp_monomial.degree);
        temp_monomial.poly.extend_dimension(self.polynomial_partitions().generators_up_to_degree(temp_monomial.degree));
        temp_monomial.ext.extend_dimension(self.exterior_partitions().generators_up_to_degree(temp_monomial.degree));
        self.sq_polynomial_generator_to_monomial(&mut temp_monomial, sq, input_degree, input_idx);
        if temp_monomial.valid {
            result.add_basis_element(self.monomial_to_index(&temp_monomial).unwrap(), coeff);
        }
    }

    fn sq_exterior_generator_to_polynomial(&self, result : &mut FpVector, coeff : u32, sq : i32, input_degree : i32, input_idx : usize) {
        let q = self.algebra().adem_algebra().q();
        let mut temp_monomial = UnstableAlgebraMonomial::new(self.prime());
        temp_monomial.degree = q*sq + input_degree;
        temp_monomial.poly.extend_dimension(self.polynomial_partitions().generators_up_to_degree(temp_monomial.degree));
        temp_monomial.ext.extend_dimension(self.exterior_partitions().generators_up_to_degree(temp_monomial.degree));
        self.sq_exterior_generator_to_monomial(&mut temp_monomial, sq, input_degree, input_idx);
        if temp_monomial.valid {
            result.add_basis_element(self.monomial_to_index(&temp_monomial).unwrap(), coeff);
        }
    }

    fn bockstein_polynomial_generator_to_polynomial(&self, result : &mut FpVector, coeff : u32, input_degree : i32, input_idx : usize) {
        let mut temp_monomial = UnstableAlgebraMonomial::new(self.prime());
        temp_monomial.degree = 1 + input_degree;
        temp_monomial.poly.extend_dimension(self.polynomial_partitions().generators_up_to_degree(temp_monomial.degree));
        temp_monomial.ext.extend_dimension(self.exterior_partitions().generators_up_to_degree(temp_monomial.degree));
        self.bockstein_polynomial_generator_to_monomial(&mut temp_monomial, input_degree, input_idx);
        if temp_monomial.valid {
            result.add_basis_element(self.monomial_to_index(&temp_monomial).unwrap(), coeff);
        }
    }

    fn bockstein_exterior_generator_to_polynomial(&self, result : &mut FpVector, coeff : u32, input_degree : i32, input_idx : usize) {
        let mut temp_monomial = UnstableAlgebraMonomial::new(self.prime());
        self.set_monomial_degree(&mut temp_monomial, input_degree + 1);
        self.bockstein_exterior_generator_to_monomial(&mut temp_monomial, input_degree, input_idx);
        if temp_monomial.valid {
            result.add_basis_element(self.monomial_to_index(&temp_monomial).unwrap(), coeff);
        }
    }


    fn sq_on_basis(&self, result : &mut FpVector, coeff : u32, sq : i32, degree : i32, idx : usize) {
        let q = self.algebra().adem_algebra().q();
        result.add(&self.action_table()[(degree + q*sq) as usize][sq as usize][idx], coeff);
    }

    fn sq_monomial_to_polynomial(&self, result : &mut FpVector, coeff : u32, sq : i32, mono : &UnstableAlgebraMonomial) {
        let idx = self.monomial_to_index(mono).unwrap();
        self.sq_on_basis(result, coeff, sq, mono.degree, idx);
    }

    fn sq_polynomial_to_polynomial(&self, result : &mut FpVector, coeff : u32, sq : i32, input_degree : i32, input : &FpVector) {
        for (idx, c) in input.iter_nonzero() {
            self.sq_on_basis(result, (c*coeff) % *self.prime(), sq, input_degree, idx);
        }
    }

    fn bockstein_on_basis(&self, result : &mut FpVector, coeff : u32, degree : i32, idx : usize) {
        result.add(&self.bockstein_table()[degree  as usize + 1][idx], coeff);
    }

    fn bockstein_monomial_to_polynomial(&self, result : &mut FpVector, coeff : u32, mono : &UnstableAlgebraMonomial) {
        let idx = self.monomial_to_index(mono).unwrap();
        self.bockstein_on_basis(result, coeff, mono.degree, idx);
    }

    fn bockstein_polynomial_to_polynomial(&self, result : &mut FpVector, coeff : u32, input_degree : i32, input : &FpVector) {
        for (idx, c) in input.iter_nonzero() {
            self.bockstein_on_basis(result, (c*coeff) % *self.prime(), input_degree, idx);
        }
    }

    fn multiply_monomials(&self, target : &mut UnstableAlgebraMonomial, source : &UnstableAlgebraMonomial) -> Option<()> {
        self.set_monomial_degree(target, target.degree + source.degree);

        target.ext.set_slice(0, source.ext.dimension());
        target.ext.add_truncate(&source.ext, 1)?;
        target.ext.clear_slice();

        let mut carry_vec = [FpVector::new(self.prime(), target.poly.dimension())];
        let mut source_vec = source.poly.clone();
        source_vec.set_scratch_vector_size(target.poly.dimension());
        let mut carry_q = true;
        while carry_q {
            carry_q = target.poly.add_carry(&source_vec, 1, &mut carry_vec);
            if carry_q {
                source_vec.set_to_zero_pure();
                self.frobenius_monomial(&mut source_vec, &carry_vec[0]);
                carry_vec[0].set_to_zero_pure();
            }
        }
        Some(())
    }

    fn multiply_polynomials(&self, target : &mut FpVector, coeff : u32, left_degree : i32, left : &FpVector, right_degree : i32, right : &FpVector) {
        let p = *self.prime();
        target.extend_dimension(self.dimension(left_degree + right_degree));
        for (left_idx, left_entry) in left.iter_nonzero() {
            for (right_idx, right_entry) in right.iter_nonzero() {
                let mut target_mono = self.index_to_monomial(left_degree, left_idx).clone();
                let source_mono = self.index_to_monomial(right_degree, right_idx);
                self.multiply_monomials(&mut target_mono,  &source_mono);
                let idx = self.monomial_to_index(&target_mono).unwrap();
                target.add_basis_element(idx, (left_entry * right_entry * coeff)%p);
            }
        }
    }

    fn multiply_polynomial_by_monomial(&self, target : &mut FpVector, coeff : u32, left_degree : i32, left : &FpVector, right_mono : &UnstableAlgebraMonomial) {
        let p = *self.prime();
        target.extend_dimension(self.dimension(left_degree + right_mono.degree));
        for (left_idx, left_entry) in left.iter_nonzero() {
            let mut target_mono = self.index_to_monomial(left_degree, left_idx).clone();
            println!("left_mono : {}", target_mono);
            println!("right_mono : {}", right_mono);
            self.multiply_monomials(&mut target_mono,  &right_mono);
            println!("target_mono : {}", target_mono);
            let idx = self.monomial_to_index(&target_mono).unwrap();
            target.add_basis_element(idx, (left_entry * 1 * coeff)%p);
        }
    }

    fn compute_basis_basis_part(&self, degree : i32) {
        debug_assert!(self.basis_table().len() == degree as usize);
        let num_poly_gens = self.polynomial_generators_in_degree(degree);
        let num_ext_gens = self.exterior_generators_in_degree(degree);
        let poly_parts = self.polynomial_partitions();
        let ext_parts = self.exterior_partitions();
        if degree > 0 {
            poly_parts.add_gens_and_calculate_parts(degree, num_poly_gens);
            ext_parts.add_gens_and_calculate_parts(degree, num_ext_gens);
        }
        let mut table = UnstableAlgebraTableEntry::new();
        for poly_deg in 0 ..= degree {
            let ext_deg = degree - poly_deg;
            for p in poly_parts.parts(poly_deg) {
                for e in ext_parts.parts(ext_deg) {
                    let index = table.index_to_monomial.len();
                    let mut m = UnstableAlgebraMonomial {
                        degree,
                        poly : p.clone(),
                        ext : e.clone(),
                        valid : true
                    };
                    self.set_monomial_degree(&mut m, degree);
                    println!("==  idx : {}, m : {}", table.index_to_monomial.len(), m);
                    table.monomial_to_index.insert(m.clone(), index);
                    table.index_to_monomial.push(m);
                }
            }
        }
        self.basis_table().push(table);
    }

    fn compute_action_table(&self, degree : i32){
        let p = self.prime();
        let q = self.algebra().adem_algebra().q();
        // Build action table
        // degree -> first square -> monomial idx -> result vector
        let mut term = FpVector::new(p, 0);
        let mut reducer_a = FpVector::new(p, 0);
        let mut reducer_b = FpVector::new(p, 0);
        let mut table = Vec::with_capacity(degree as usize);
        let mut sq_table = Vec::with_capacity(self.dimension(degree));
        for basis_idx in 0 .. self.dimension(degree) {
            let mut result = FpVector::new(p, self.dimension(degree));
            result.add_basis_element(basis_idx, 1);
            sq_table.push(result);
        }
        table.push(sq_table);
        for sq in 1 .. degree/q {
            let rest = degree - q*sq;
            let dim = self.dimension(rest);
            let mut sq_table = Vec::with_capacity(dim);
            for basis_idx in 0 .. dim {
                let mono = self.index_to_monomial(rest, basis_idx);
                let result;
                if let Some((gen_int_idx, _)) = mono.ext.iter_nonzero().next() {
                    result = self.compute_action_table_ext_case(sq, &mono, gen_int_idx, &mut reducer_a, &mut reducer_b, &mut term);
                } else {
                    result = self.compute_action_table_poly_case(sq,&mono, &mut reducer_a, &mut reducer_b, &mut term);
                }
                sq_table.push(result);
            }
            table.push(sq_table);
        }
        if degree > 0 {
            let mut sq_table = Vec::with_capacity(1);
            let result = FpVector::new(p, self.dimension(degree));
            sq_table.push(result);
            table.push(sq_table);
        }
        self.action_table().push(table);
    }

    fn set_monomial_degree(&self, mono : &mut UnstableAlgebraMonomial, degree : i32) {
        mono.degree = degree;
        mono.ext.set_scratch_vector_size(self.exterior_partitions().generators_up_to_degree(mono.degree));
        mono.poly.set_scratch_vector_size(self.polynomial_partitions().generators_up_to_degree(mono.degree));        
    }

    fn compute_action_table_ext_case<'a>(&self, 
        sq : i32, mono : &UnstableAlgebraMonomial, gen_int_idx : usize, 
        reducer_a : &'a mut FpVector, reducer_b : &'a mut FpVector, term : &mut FpVector
    ) -> FpVector {
        let p = self.prime();
        let mut rest_mono = mono.clone();
        let (gen_deg, gen_idx) = self.exterior_partitions().internal_idx_to_gen_deg(gen_int_idx);
        rest_mono.ext.set_entry(gen_int_idx, 0);
        let rest_mono_new_deg = rest_mono.degree - gen_deg;
        self.set_monomial_degree(&mut rest_mono, rest_mono_new_deg);
        let mut result = FpVector::new(p, self.dimension(mono.degree));
        let nzsqs = self.nonzero_squares_on_exterior_generator(gen_deg, gen_idx);
        for ext_sq in nzsqs {
            let rest_sq = sq - ext_sq;
            let rest_deg = rest_mono.degree + rest_sq;
            reducer_a.set_scratch_vector_size(self.dimension(rest_deg));
            reducer_a.set_to_zero_pure();
            self.sq_monomial_to_polynomial(reducer_a, 1, rest_sq, &rest_mono);
            let term_deg = ext_sq + gen_deg;
            term.set_scratch_vector_size(self.dimension(term_deg));
            term.set_to_zero_pure();
            self.sq_exterior_generator_to_polynomial(term, 1, ext_sq, gen_deg, gen_idx);
            let total_deg = ext_sq + gen_deg + rest_mono.degree + rest_sq;
            reducer_b.set_scratch_vector_size(self.dimension(rest_deg + term_deg));
            reducer_b.set_to_zero_pure();
            self.multiply_polynomials(reducer_b, 1, rest_deg, reducer_a, term_deg, term);
            std::mem::swap(reducer_a, reducer_b);
        }
        reducer_a.set_scratch_vector_size(self.dimension(mono.degree));
        result.add(reducer_a, 1);
        result
    }

    fn compute_action_table_poly_case<'a>(&'a self,
        sq : i32, mono : &UnstableAlgebraMonomial,
        reducer_a : &'a mut FpVector, reducer_b : &'a mut FpVector, term : &'a mut FpVector
    ) -> FpVector {
        let mut rest_mono = mono.clone();
        let p = self.prime();
        let mut result = FpVector::new(p, self.dimension(mono.degree));
        let mut gen_exp = u32::max_value();
        let mut gen_int_idx = usize::max_value();
        for (i, v) in rest_mono.poly.iter_nonzero() {
            if v < gen_exp {
                gen_exp = v;
                gen_int_idx = i;
            }
        }
        let (gen_deg, gen_idx) = self.polynomial_partitions().internal_idx_to_gen_deg(gen_int_idx);
        rest_mono.poly.set_entry(gen_int_idx, 0);
        let rest_mono_new_deg = rest_mono.degree - gen_exp as i32 * gen_deg;
        self.set_monomial_degree(&mut rest_mono, rest_mono_new_deg);
        let nzsqs = self.nonzero_squares_on_polynomial_generator(gen_deg, gen_idx);
        for (rest_sq, part) in PartitionIterator::new(sq, gen_exp, &nzsqs) {
            let coeff = multinomial(p, &mut part.clone());
            let mut cur_deg = rest_mono.degree + rest_sq;
            reducer_a.set_scratch_vector_size(self.dimension(cur_deg));
            reducer_a.set_to_zero_pure();
            self.sq_monomial_to_polynomial(reducer_a, 1, rest_sq, &rest_mono);
            for (idx, mult) in part.iter().enumerate() {
                if *mult == 0 {
                    continue;
                }
                let cur_sq = nzsqs[idx];
                let term_degree = cur_sq + gen_deg;
                term.set_scratch_vector_size(self.dimension(cur_sq + gen_deg));
                term.set_to_zero_pure();
                self.sq_polynomial_generator_to_polynomial(term, 1, cur_sq, gen_deg, gen_idx);
                for _ in 0 .. *mult {
                    reducer_b.set_scratch_vector_size(self.dimension(cur_deg + term_degree));
                    reducer_b.set_to_zero_pure();
                    self.multiply_polynomials(reducer_b, 1, cur_deg, &reducer_a, term_degree, &term);
                    cur_deg += term_degree;
                    std::mem::swap(reducer_a, reducer_b);
                }
            }
            result.add(&reducer_a, coeff);
        }
        result
    }

    fn compute_bockstein_table(&self, degree : i32) {
        if degree == 1 {
            self.bockstein_table().push(vec![FpVector::new(self.prime(), 1)]);
            return;
        }
        let p = self.prime();
        // Build action table
        // degree -> monomial idx -> result vector
        let mut term = FpVector::new(p, 0);
        let rest = degree - 1;
        let dim = self.dimension(rest);
        let mut table = Vec::with_capacity(dim);
        for basis_idx in 0 .. dim {
            let mono = self.index_to_monomial(rest, basis_idx);
            let mut rest_mono = mono.clone();
            let mut result = FpVector::new(p, self.dimension(degree));
            let used_ext_generator;
            let gen_deg;
            let gen_int_idx;
            let gen_idx;
            let coeff;
            if let Some((gen_int_idx_, _)) = mono.ext.iter_nonzero().next() {
                used_ext_generator = true;
                gen_int_idx = gen_int_idx_;
                let (a, b) = self.exterior_partitions().internal_idx_to_gen_deg(gen_int_idx);
                gen_deg = a; gen_idx = b;
                coeff = 1;
                rest_mono.ext.set_entry(gen_int_idx, 0);
            } else if let Some((gen_int_idx_, gen_exp)) = mono.poly.iter_nonzero().next() {
                used_ext_generator = false;
                gen_int_idx = gen_int_idx_;
                let (a, b) = self.polynomial_partitions().internal_idx_to_gen_deg(gen_int_idx);
                gen_deg = a; gen_idx = b;
                coeff = gen_exp;
                rest_mono.poly.set_entry(gen_int_idx, gen_exp - 1);
            } else {
                unreachable!();
            }
            let rest_mono_new_deg = rest_mono.degree - gen_deg;
            self.set_monomial_degree(&mut rest_mono, rest_mono_new_deg);
            // result += (b x) * rest
            term.set_scratch_vector_size(self.dimension(gen_deg + 1));
            term.set_to_zero_pure();
            if used_ext_generator {
                self.bockstein_exterior_generator_to_polynomial(&mut term, 1, gen_deg, gen_idx);
            } else {
                self.bockstein_polynomial_generator_to_polynomial(&mut term, 1, gen_deg, gen_idx);
            }
            self.multiply_polynomial_by_monomial(&mut result, 1, gen_deg + 1, &term, &rest_mono);
            // result += x * (b rest)
            // First get "b rest" into "term"
            let brest_degree = rest_mono.degree + 1;
            term.set_scratch_vector_size(self.dimension(brest_degree));
            term.set_to_zero_pure();
            self.bockstein_monomial_to_polynomial(&mut term, 1, &rest_mono);
            // Now get "x" into "rest_mono"
            self.set_monomial_degree(&mut rest_mono, gen_deg);
            rest_mono.ext.set_to_zero_pure();
            rest_mono.poly.set_to_zero_pure();
            if used_ext_generator {
                rest_mono.ext.set_entry(gen_int_idx, 1);
            } else {
                rest_mono.poly.set_entry(gen_int_idx, 1);
            }
            self.multiply_polynomial_by_monomial(&mut result, coeff, brest_degree, &term, &rest_mono);
            table.push(result);
        }
        self.bockstein_table().push(table);
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
        if degree < 0 { 
            0 
        } else {
            self.basis_table()[degree as usize].index_to_monomial.len()
        }
    }

    fn compute_basis(&self, degree : i32) {
        self.algebra().compute_basis(degree);
        self.compute_generating_set(degree);
        for i in self.max_computed_degree() + 1 ..= degree {
            self.compute_basis_basis_part(i);
            self.compute_action_table(i);
            if self.algebra().adem_algebra().generic {
                self.compute_bockstein_table(i);
            }
        }
    }    

    fn basis_element_to_string(&self, _degree: i32, _index: usize) -> String {
        "".to_string()
    }

    fn act_on_basis(&self,
        result : &mut FpVector, coeff : u32,  op_deg : i32, op_index : usize, input_degree : i32, input_index : usize
    ) {
        let mut input_vec = FpVector::new(self.prime(), self.dimension(input_degree));
        input_vec.set_entry(input_index, 1);
        self.act(result, coeff, op_deg, op_index, input_degree, &input_vec);
    }


    fn act(&self, result : &mut FpVector, coeff : u32, op_degree : i32, op_index : usize, input_degree : i32, input : &FpVector){
        let algebra_outer = self.algebra();
        let algebra = algebra_outer.adem_algebra();
        let q = algebra.q();
        let op = algebra.basis_element_from_index(op_degree, op_index);
        let ps_len = op.ps.len();
        let mut cur_deg = input_degree;
        let mut source_vec = input.clone();
        let mut target_vec = FpVector::new(self.prime(), 0);
        if (op.bocksteins >> ps_len) & 1 == 1 {
            let op_deg = 1;
            target_vec.set_scratch_vector_size(self.dimension(cur_deg + op_deg));
            target_vec.set_to_zero_pure();
            self.bockstein_polynomial_to_polynomial(&mut target_vec, 1, cur_deg, &source_vec);
            cur_deg += op_deg;
            std::mem::swap(&mut source_vec, &mut target_vec);
        }
        for i in (0..ps_len).rev() {
            let op_deg = op.ps[i] as i32 * q;
            target_vec.set_scratch_vector_size(self.dimension(cur_deg + op_deg));
            target_vec.set_to_zero_pure();
            self.sq_polynomial_to_polynomial(&mut target_vec, 1, op.ps[i] as i32, cur_deg, &source_vec);
            cur_deg += op_deg;
            std::mem::swap(&mut source_vec, &mut target_vec);
            if (op.bocksteins >> i) & 1 == 1 {
                let op_deg = 1;
                target_vec.set_scratch_vector_size(self.dimension(cur_deg + op_deg));
                target_vec.set_to_zero_pure();
                self.bockstein_polynomial_to_polynomial(&mut target_vec, 1, cur_deg, &source_vec);
                cur_deg += op_deg;
                std::mem::swap(&mut source_vec, &mut target_vec);
            }
        }
        result.add(&source_vec, coeff);
    }
}