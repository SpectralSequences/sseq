use std::collections::HashMap;
use std::sync::Arc;

use once::OnceVec;
use fp::prime::multinomial;
use fp::vector::{FpVector, FpVectorT};

use crate::algebra::combinatorics::PartitionIterator;
use crate::algebra::{Algebra, AdemAlgebraT};
use crate::algebra::{PolynomialAlgebra, PolynomialAlgebraMonomial};
use crate::module::Module;

pub trait PolynomialAlgebraModule : PolynomialAlgebra {
    type Algebra : AdemAlgebraT;
    fn algebra(&self) -> Arc<Self::Algebra>;

    fn nonzero_squares_on_polynomial_generator(&self, gen_degree : i32, gen_idx : usize) -> Vec<i32>;
    fn nonzero_squares_on_exterior_generator(&self, gen_degree : i32, gen_idx : usize) -> Vec<i32>;

    fn sq_polynomial_generator_to_monomial(&self, result : &mut PolynomialAlgebraMonomial, sq : i32, input_degree : i32, input_idx : usize);
    fn sq_exterior_generator_to_monomial(&self, result : &mut PolynomialAlgebraMonomial, sq : i32, input_degree : i32, input_idx : usize);
    fn bockstein_polynomial_generator_to_monomial(&self, result : &mut PolynomialAlgebraMonomial, input_degree : i32, input_idx : usize);
    fn bockstein_exterior_generator_to_monomial(&self, result : &mut PolynomialAlgebraMonomial, input_degree : i32, input_idx : usize);

    fn action_table(&self) -> &OnceVec<Vec<Vec<FpVector>>>; // degree -> square -> monomial idx -> result vector
    fn bockstein_table(&self) -> &OnceVec<Vec<FpVector>>;


    fn sq_polynomial_generator_to_polynomial(&self, result : &mut FpVector, coeff : u32, sq : i32, input_degree : i32, input_idx : usize) {
        let mut temp_monomial = PolynomialAlgebraMonomial::new(self.prime());
        let q = self.algebra().adem_algebra().q();
        temp_monomial.degree =  q*sq + input_degree;
        println!("degree : {}", temp_monomial.degree);
        temp_monomial.poly.extend_dimension(self.polynomial_partitions().generators_up_to_degree(temp_monomial.degree));
        temp_monomial.ext.extend_dimension(self.exterior_partitions().generators_up_to_degree(temp_monomial.degree));
        self.sq_polynomial_generator_to_monomial(&mut temp_monomial, sq, input_degree, input_idx);
        println!("temp_monomial: {}", temp_monomial);
        if temp_monomial.valid {
            result.add_basis_element(self.monomial_to_index(&temp_monomial).unwrap(), coeff);
        }
    }

    fn sq_exterior_generator_to_polynomial(&self, result : &mut FpVector, coeff : u32, sq : i32, input_degree : i32, input_idx : usize) {
        let q = self.algebra().adem_algebra().q();
        let mut temp_monomial = PolynomialAlgebraMonomial::new(self.prime());
        temp_monomial.degree = q*sq + input_degree;
        temp_monomial.poly.extend_dimension(self.polynomial_partitions().generators_up_to_degree(temp_monomial.degree));
        temp_monomial.ext.extend_dimension(self.exterior_partitions().generators_up_to_degree(temp_monomial.degree));
        self.sq_exterior_generator_to_monomial(&mut temp_monomial, sq, input_degree, input_idx);
        if temp_monomial.valid {
            result.add_basis_element(self.monomial_to_index(&temp_monomial).unwrap(), coeff);
        }
    }

    fn bockstein_polynomial_generator_to_polynomial(&self, result : &mut FpVector, coeff : u32, input_degree : i32, input_idx : usize) {
        let mut temp_monomial = PolynomialAlgebraMonomial::new(self.prime());
        temp_monomial.degree = 1 + input_degree;
        temp_monomial.poly.extend_dimension(self.polynomial_partitions().generators_up_to_degree(temp_monomial.degree));
        temp_monomial.ext.extend_dimension(self.exterior_partitions().generators_up_to_degree(temp_monomial.degree));
        self.bockstein_polynomial_generator_to_monomial(&mut temp_monomial, input_degree, input_idx);
        if temp_monomial.valid {
            result.add_basis_element(self.monomial_to_index(&temp_monomial).unwrap(), coeff);
        }
    }

    fn bockstein_exterior_generator_to_polynomial(&self, result : &mut FpVector, coeff : u32, input_degree : i32, input_idx : usize) {
        let mut temp_monomial = PolynomialAlgebraMonomial::new(self.prime());
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

    fn sq_monomial_to_polynomial(&self, result : &mut FpVector, coeff : u32, sq : i32, mono : &PolynomialAlgebraMonomial) {
        let idx = self.monomial_to_index(mono).unwrap();
        self.sq_on_basis(result, coeff, sq, mono.degree, idx);
    }

    fn sq_polynomial_to_polynomial(&self, result : &mut FpVector, coeff : u32, sq : i32, input_degree : i32, input : &FpVector) {
        for (idx, c) in input.iter_nonzero() {
            self.sq_on_basis(result, (c*coeff) % *self.prime(), sq, input_degree, idx);
        }
    }

    fn bockstein_on_basis(&self, result : &mut FpVector, coeff : u32, degree : i32, idx : usize) {
        println!("bock_on_basis {}, {}", degree, idx);
        result.add(&self.bockstein_table()[degree  as usize + 1][idx], coeff);
    }

    fn bockstein_monomial_to_polynomial(&self, result : &mut FpVector, coeff : u32, mono : &PolynomialAlgebraMonomial) {
        let idx = self.monomial_to_index(mono).unwrap();
        self.bockstein_on_basis(result, coeff, mono.degree, idx);
    }

    fn bockstein_polynomial_to_polynomial(&self, result : &mut FpVector, coeff : u32, input_degree : i32, input : &FpVector) {
        for (idx, c) in input.iter_nonzero() {
            self.bockstein_on_basis(result, (c*coeff) % *self.prime(), input_degree, idx);
        }
    }    

    fn compute_action_table(&self, degree : i32) {
        println!("compute_action_table degree : {}", degree);
        let p = self.prime();
        let q = self.algebra().adem_algebra().q();
        // Build action table
        // degree -> first square -> monomial idx -> result vector
        let mut term = FpVector::new(p, 0);
        let mut reducer_a = FpVector::new(p, 0);
        let mut reducer_b = FpVector::new(p, 0);
        let mut table = Vec::with_capacity(degree as usize);
        let mut sq_table = Vec::with_capacity(Module::dimension(self, degree));
        for basis_idx in 0 .. Module::dimension(self, degree) {
            let mut result = FpVector::new(p, Module::dimension(self, degree));
            result.add_basis_element(basis_idx, 1);
            sq_table.push(result);
        }
        table.push(sq_table);
        for sq in 1 .. degree/q {
            let rest = degree - q*sq;
            let dim = Module::dimension(self, rest);
            let mut sq_table = Vec::with_capacity(dim);
            for basis_idx in 0 .. dim {
                let mono = self.index_to_monomial(rest, basis_idx);
                let result;
                if let Some((gen_int_idx, _)) = mono.ext.iter_nonzero().next() {
                    result = self.compute_action_table_ext_case(sq, &mono, gen_int_idx, &mut reducer_a, &mut reducer_b, &mut term);
                } else {
                    result = self.compute_action_table_poly_case(sq, &mono, &mut reducer_a, &mut reducer_b, &mut term);
                }
                sq_table.push(result);
            }
            table.push(sq_table);
        }
        if degree > 0 {
            let mut sq_table = Vec::with_capacity(1);
            let result = FpVector::new(p, Module::dimension(self, degree));
            sq_table.push(result);
            table.push(sq_table);
        }
        self.action_table().push(table);
    }

    fn compute_action_table_ext_case<'a>(&self, 
        sq : i32, mono : &PolynomialAlgebraMonomial, gen_int_idx : usize, 
        reducer_a : &'a mut FpVector, reducer_b : &'a mut FpVector, term : &mut FpVector
    ) -> FpVector {
        let p = self.prime();
        let mut rest_mono = mono.clone();
        let (gen_deg, gen_idx) = self.exterior_partitions().internal_idx_to_gen_deg(gen_int_idx);
        rest_mono.ext.set_entry(gen_int_idx, 0);
        let rest_mono_new_deg = rest_mono.degree - gen_deg;
        self.set_monomial_degree(&mut rest_mono, rest_mono_new_deg);
        let mut result = FpVector::new(p, Module::dimension(self, mono.degree));
        let nzsqs = self.nonzero_squares_on_exterior_generator(gen_deg, gen_idx);
        for ext_sq in nzsqs {
            let rest_sq = sq - ext_sq;
            let rest_deg = rest_mono.degree + rest_sq;
            reducer_a.set_scratch_vector_size(Module::dimension(self, rest_deg));
            reducer_a.set_to_zero_pure();
            self.sq_monomial_to_polynomial(reducer_a, 1, rest_sq, &rest_mono);
            let term_deg = ext_sq + gen_deg;
            term.set_scratch_vector_size(Module::dimension(self, term_deg));
            term.set_to_zero_pure();
            self.sq_exterior_generator_to_polynomial(term, 1, ext_sq, gen_deg, gen_idx);
            reducer_b.set_scratch_vector_size(Module::dimension(self, rest_deg + term_deg));
            reducer_b.set_to_zero_pure();
            self.multiply_polynomials(reducer_b, 1, rest_deg, reducer_a, term_deg, term);
            std::mem::swap(reducer_a, reducer_b);
        }
        reducer_a.set_scratch_vector_size(Module::dimension(self, mono.degree));
        result.add(reducer_a, 1);
        result
    }

    fn compute_action_table_poly_case<'a>(&'a self,
        sq : i32, mono : &PolynomialAlgebraMonomial,
        reducer_a : &'a mut FpVector, reducer_b : &'a mut FpVector, term : &'a mut FpVector
    ) -> FpVector {
        let mut rest_mono = mono.clone();
        let p = self.prime();
        let mut result = FpVector::new(p, Module::dimension(self, mono.degree));
        let (gen_int_idx, gen_exp) = rest_mono.poly.iter_nonzero().min_by_key(|(_i, v)| *v).unwrap();
        let (gen_deg, gen_idx) = self.polynomial_partitions().internal_idx_to_gen_deg(gen_int_idx);
        rest_mono.poly.set_entry(gen_int_idx, 0);
        let rest_mono_new_deg = rest_mono.degree - gen_exp as i32 * gen_deg;
        self.set_monomial_degree(&mut rest_mono, rest_mono_new_deg);
        let nzsqs = self.nonzero_squares_on_polynomial_generator(gen_deg, gen_idx);
        for (rest_sq, part) in PartitionIterator::new(sq, gen_exp, &nzsqs) {
            let coeff = multinomial(p, &mut part.clone());
            let mut cur_deg = rest_mono.degree + rest_sq;
            reducer_a.set_scratch_vector_size(Module::dimension(self, cur_deg));
            reducer_a.set_to_zero_pure();
            self.sq_monomial_to_polynomial(reducer_a, 1, rest_sq, &rest_mono);
            for (idx, mult) in part.iter().enumerate() {
                if *mult == 0 {
                    continue;
                }
                let cur_sq = nzsqs[idx];
                let term_degree = cur_sq + gen_deg;
                term.set_scratch_vector_size(Module::dimension(self, cur_sq + gen_deg));
                term.set_to_zero_pure();
                self.sq_polynomial_generator_to_polynomial(term, 1, cur_sq, gen_deg, gen_idx);
                for _ in 0 .. *mult {
                    reducer_b.set_scratch_vector_size(Module::dimension(self, cur_deg + term_degree));
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
        let dim = Module::dimension(self, rest);
        let mut table = Vec::with_capacity(dim);
        for basis_idx in 0 .. dim {
            let mono = self.index_to_monomial(rest, basis_idx);
            let mut rest_mono = mono.clone();
            let mut result = FpVector::new(p, Module::dimension(self, degree));
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
            term.set_scratch_vector_size(Module::dimension(self, gen_deg + 1));
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
            term.set_scratch_vector_size(Module::dimension(self, brest_degree));
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


impl<Adem : AdemAlgebraT, A : PolynomialAlgebraModule<Algebra=Adem> + Send + Sync + 'static> Module for A {
    type Algebra = Adem;
    fn algebra(&self) -> Arc<Self::Algebra> {
        PolynomialAlgebraModule::algebra(self)
    }

    fn name(&self) -> String {
        Algebra::name(self).to_string()
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
        let prev_max = Module::max_computed_degree(self);
        println!("prev_max : {}", prev_max);
        self.algebra().compute_basis(degree);
        Algebra::compute_basis(self, degree);
        for i in prev_max + 1 ..= degree {
            self.compute_action_table(i);
            if self.algebra().adem_algebra().generic {
                println!("compute bockstein table i : {}", i);
                self.compute_bockstein_table(i);
            }
        }
    }    

    fn basis_element_to_string(&self, degree: i32, index: usize) -> String {
        let mono = self.index_to_monomial(degree, index);
        let mut exp_map = HashMap::new();
        for (i, e) in mono.poly.iter_nonzero() {
            let (gen_deg, gen_idx) = self.polynomial_partitions().internal_idx_to_gen_deg(i);
            let (var, var_exp) = self.repr_poly_generator(gen_deg, gen_idx);
            exp_map.entry(var).or_insert((0, gen_deg/var_exp as i32)).0 += e as i32 * gen_deg;
        }
        // exp_map.iter().sor

        unimplemented!()
    }

    fn act_on_basis(&self,
        result : &mut FpVector, coeff : u32,  op_deg : i32, op_index : usize, input_degree : i32, input_index : usize
    ) {
        let mut input_vec = FpVector::new(self.prime(), Module::dimension(self, input_degree));
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
            target_vec.set_scratch_vector_size(Module::dimension(self, cur_deg + op_deg));
            target_vec.set_to_zero_pure();
            self.bockstein_polynomial_to_polynomial(&mut target_vec, 1, cur_deg, &source_vec);
            cur_deg += op_deg;
            std::mem::swap(&mut source_vec, &mut target_vec);
        }
        for i in (0..ps_len).rev() {
            let op_deg = op.ps[i] as i32 * q;
            target_vec.set_scratch_vector_size(Module::dimension(self, cur_deg + op_deg));
            target_vec.set_to_zero_pure();
            self.sq_polynomial_to_polynomial(&mut target_vec, 1, op.ps[i] as i32, cur_deg, &source_vec);
            cur_deg += op_deg;
            std::mem::swap(&mut source_vec, &mut target_vec);
            if (op.bocksteins >> i) & 1 == 1 {
                let op_deg = 1;
                target_vec.set_scratch_vector_size(Module::dimension(self, cur_deg + op_deg));
                target_vec.set_to_zero_pure();
                self.bockstein_polynomial_to_polynomial(&mut target_vec, 1, cur_deg, &source_vec);
                cur_deg += op_deg;
                std::mem::swap(&mut source_vec, &mut target_vec);
            }
        }
        result.add(&source_vec, coeff);
    }
}