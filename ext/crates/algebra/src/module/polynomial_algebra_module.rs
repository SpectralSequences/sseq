use std::sync::Arc;

use fp::vector::{FpVector, Slice, SliceMut};
use once::OnceVec;

use crate::algebra::{AdemAlgebraT, Algebra};
use crate::algebra::{PolynomialAlgebra, PolynomialAlgebraMonomial};
use crate::module::Module;

pub trait PolynomialAlgebraModule: PolynomialAlgebra {
    type Algebra: AdemAlgebraT;
    fn algebra(&self) -> Arc<Self::Algebra>;

    fn sq_polynomial_generator_to_monomial(
        &self,
        result: &mut PolynomialAlgebraMonomial,
        sq: i32,
        input_degree: i32,
        input_idx: usize,
    );
    fn sq_exterior_generator_to_monomial(
        &self,
        result: &mut PolynomialAlgebraMonomial,
        sq: i32,
        input_degree: i32,
        input_idx: usize,
    );
    fn bockstein_polynomial_generator_to_monomial(
        &self,
        result: &mut PolynomialAlgebraMonomial,
        input_degree: i32,
        input_idx: usize,
    );
    fn bockstein_exterior_generator_to_monomial(
        &self,
        result: &mut PolynomialAlgebraMonomial,
        input_degree: i32,
        input_idx: usize,
    );

    fn action_table(&self) -> &OnceVec<Vec<Vec<FpVector>>>; // degree -> square -> monomial idx -> result vector
    fn bockstein_table(&self) -> &OnceVec<Vec<FpVector>>;

    fn bottom_truncation(&self) -> i32 {
        1
    }

    fn top_truncation(&self) -> i32 {
        i32::max_value()
    }

    fn sq_polynomial_generator_to_polynomial(
        &self,
        mut result: SliceMut,
        coeff: u32,
        sq: i32,
        input_degree: i32,
        input_index: usize,
    ) {
        let mut temp_monomial = PolynomialAlgebraMonomial::new(self.prime());
        let q = self.algebra().adem_algebra().q();
        temp_monomial.degree = q * sq + input_degree;
        temp_monomial.poly.extend_len(
            self.polynomial_monomials()
                .generators_up_to_degree(temp_monomial.degree),
        );
        temp_monomial.ext.extend_len(
            self.exterior_monomials()
                .generators_up_to_degree(temp_monomial.degree),
        );
        self.sq_polynomial_generator_to_monomial(&mut temp_monomial, sq, input_degree, input_index);
        if temp_monomial.valid {
            result.add_basis_element(self.monomial_to_index(&temp_monomial), coeff);
        }
    }

    fn sq_exterior_generator_to_polynomial(
        &self,
        mut result: SliceMut,
        coeff: u32,
        sq: i32,
        input_degree: i32,
        input_index: usize,
    ) {
        let q = self.algebra().adem_algebra().q();
        let mut temp_monomial = PolynomialAlgebraMonomial::new(self.prime());
        temp_monomial.degree = q * sq + input_degree;
        temp_monomial.poly.extend_len(
            self.polynomial_monomials()
                .generators_up_to_degree(temp_monomial.degree),
        );
        temp_monomial.ext.extend_len(
            self.exterior_monomials()
                .generators_up_to_degree(temp_monomial.degree),
        );
        self.sq_exterior_generator_to_monomial(&mut temp_monomial, sq, input_degree, input_index);
        if temp_monomial.valid {
            result.add_basis_element(self.monomial_to_index(&temp_monomial), coeff);
        }
    }

    fn bockstein_polynomial_generator_to_polynomial(
        &self,
        mut result: SliceMut,
        coeff: u32,
        input_degree: i32,
        input_index: usize,
    ) {
        let mut temp_monomial = PolynomialAlgebraMonomial::new(self.prime());
        temp_monomial.degree = 1 + input_degree;
        temp_monomial.poly.extend_len(
            self.polynomial_monomials()
                .generators_up_to_degree(temp_monomial.degree),
        );
        temp_monomial.ext.extend_len(
            self.exterior_monomials()
                .generators_up_to_degree(temp_monomial.degree),
        );
        self.bockstein_polynomial_generator_to_monomial(
            &mut temp_monomial,
            input_degree,
            input_index,
        );
        if temp_monomial.valid {
            result.add_basis_element(self.monomial_to_index(&temp_monomial), coeff);
        }
    }

    fn bockstein_exterior_generator_to_polynomial(
        &self,
        mut result: SliceMut,
        coeff: u32,
        input_degree: i32,
        input_index: usize,
    ) {
        let mut temp_monomial = PolynomialAlgebraMonomial::new(self.prime());
        self.set_monomial_degree(&mut temp_monomial, input_degree + 1);
        self.bockstein_exterior_generator_to_monomial(
            &mut temp_monomial,
            input_degree,
            input_index,
        );
        if temp_monomial.valid {
            result.add_basis_element(self.monomial_to_index(&temp_monomial), coeff);
        }
    }

    fn sq_on_basis(&self, mut result: SliceMut, coeff: u32, sq: i32, degree: i32, index: usize) {
        let q = self.algebra().adem_algebra().q();
        result.add(
            self.action_table()[(degree + q * sq) as usize][sq as usize][index].as_slice(),
            coeff,
        );
    }

    fn sq_monomial_to_polynomial(
        &self,
        result: SliceMut,
        coeff: u32,
        sq: i32,
        mono: &PolynomialAlgebraMonomial,
    ) {
        let index = self.monomial_to_index(mono);
        self.sq_on_basis(result, coeff, sq, mono.degree, index);
    }

    fn sq_polynomial_to_polynomial(
        &self,
        mut result: SliceMut,
        coeff: u32,
        sq: i32,
        input_degree: i32,
        input: Slice,
    ) {
        for (index, c) in input.iter_nonzero() {
            self.sq_on_basis(
                result.copy(),
                (c * coeff) % *self.prime(),
                sq,
                input_degree,
                index,
            );
        }
    }

    fn bockstein_on_basis(&self, mut result: SliceMut, coeff: u32, degree: i32, index: usize) {
        result.add(
            self.bockstein_table()[degree as usize + 1][index].as_slice(),
            coeff,
        );
    }

    fn bockstein_monomial_to_polynomial(
        &self,
        result: SliceMut,
        coeff: u32,
        mono: &PolynomialAlgebraMonomial,
    ) {
        let index = self.monomial_to_index(mono);
        self.bockstein_on_basis(result, coeff, mono.degree, index);
    }

    fn bockstein_polynomial_to_polynomial(
        &self,
        mut result: SliceMut,
        coeff: u32,
        input_degree: i32,
        input: Slice,
    ) {
        for (idx, c) in input.iter_nonzero() {
            self.bockstein_on_basis(
                result.copy(),
                (c * coeff) % *self.prime(),
                input_degree,
                idx,
            );
        }
    }

    fn compute_action_table(&self, degree: i32) {
        let p = self.prime();
        let q = self.algebra().adem_algebra().q();
        // Build action table
        // degree -> first square -> monomial idx -> result vector
        let mut term = FpVector::new(p, 0);
        let mut reducer_a = FpVector::new(p, 0);
        let mut reducer_b = FpVector::new(p, 0);
        let mut table = Vec::with_capacity(degree as usize);
        let dimension = Algebra::dimension(self, degree, i32::max_value());
        let mut sq_table = Vec::with_capacity(dimension);
        for basis_idx in 0..dimension {
            let mut result = FpVector::new(p, dimension);
            result.add_basis_element(basis_idx, 1);
            sq_table.push(result);
        }
        table.push(sq_table);
        for sq in 1..=degree / q {
            // println!(" sq : {}, degree : {}", sq, degree);
            let rest_degree = degree - q * sq;
            let dim = Algebra::dimension(self, rest_degree, i32::max_value());
            let mut sq_table = Vec::with_capacity(dim);
            for basis_idx in 0..dim {
                let mono = self.index_to_monomial(rest_degree, basis_idx);
                let result;
                if let Some((gen_int_idx, _)) = mono.ext.iter_nonzero().next() {
                    result = self.compute_action_table_ext_case(
                        sq,
                        mono,
                        gen_int_idx,
                        &mut reducer_a,
                        &mut reducer_b,
                        &mut term,
                    );
                } else if let Some((gen_int_idx, _)) = mono.poly.iter_nonzero().next() {
                    result = self.compute_action_table_poly_case(
                        sq,
                        mono,
                        gen_int_idx,
                        &mut reducer_a,
                        &mut reducer_b,
                        &mut term,
                    );
                } else {
                    // Mono is the identity element
                    let mut result_vec =
                        FpVector::new(p, Algebra::dimension(self, degree, i32::max_value()));
                    if sq == 0 {
                        result_vec.set_entry(0, 1);
                    }
                    result = result_vec;
                }
                //         let (op_deg, op_idx) = self.algebra().adem_algebra().beps_pn(0, sq as u32);
                // println!("   {}({}) == {}\n ",
                //     self.algebra().basis_element_to_string(op_deg, op_idx),
                //     Module::basis_element_to_string(self, rest_degree, basis_idx),
                //     Module::element_to_string(self, degree, &result)
                // );
                sq_table.push(result);
            }
            table.push(sq_table);
        }
        if degree > 0 {
            let mut sq_table = Vec::with_capacity(1);
            let result = FpVector::new(p, Algebra::dimension(self, degree, i32::max_value()));
            sq_table.push(result);
            table.push(sq_table);
        }
        self.action_table().push(table);
    }

    fn compute_action_table_ext_case<'a>(
        &self,
        sq: i32,
        mono: &PolynomialAlgebraMonomial,
        gen_int_idx: usize,
        reducer_a: &'a mut FpVector,
        reducer_b: &'a mut FpVector,
        term: &mut FpVector,
    ) -> FpVector {
        let p = self.prime();
        let q = self.algebra().adem_algebra().q();
        let mut rest_mono = mono.clone();
        let (gen_deg, gen_idx) = self
            .exterior_monomials()
            .internal_idx_to_gen_deg(gen_int_idx);
        rest_mono.ext.set_entry(gen_int_idx, 0);
        let rest_mono_new_deg = rest_mono.degree - gen_deg;
        self.set_monomial_degree(&mut rest_mono, rest_mono_new_deg);
        let result_degree = mono.degree + q * sq;
        let mut result =
            FpVector::new(p, Algebra::dimension(self, result_degree, i32::max_value()));
        for ext_sq in 0..=sq {
            let rest_sq = sq - ext_sq;
            let rest_deg = rest_mono.degree + q * rest_sq;
            reducer_a.set_scratch_vector_size(Algebra::dimension(self, rest_deg, i32::max_value()));
            self.sq_monomial_to_polynomial(reducer_a.as_slice_mut(), 1, rest_sq, &rest_mono);
            let term_deg = q * ext_sq + gen_deg;
            term.set_scratch_vector_size(Algebra::dimension(self, term_deg, i32::max_value()));
            self.sq_exterior_generator_to_polynomial(
                term.as_slice_mut(),
                1,
                ext_sq,
                gen_deg,
                gen_idx,
            );
            reducer_b.set_scratch_vector_size(Algebra::dimension(
                self,
                rest_deg + term_deg,
                i32::max_value(),
            ));
            self.multiply_polynomials(reducer_b, 1, term_deg, term, rest_deg, reducer_a);
            std::mem::swap(reducer_a, reducer_b);
            reducer_a.set_scratch_vector_size(Algebra::dimension(
                self,
                result_degree,
                i32::max_value(),
            ));
            result.add(reducer_a, 1);
        }
        result
    }

    fn compute_action_table_poly_case<'a>(
        &'a self,
        sq: i32,
        mono: &PolynomialAlgebraMonomial,
        gen_int_idx: usize,
        reducer_a: &'a mut FpVector,
        reducer_b: &'a mut FpVector,
        term: &'a mut FpVector,
    ) -> FpVector {
        let mut rest_mono = mono.clone();
        let p = self.prime();
        let q = self.algebra().adem_algebra().q();
        let gen_exp = rest_mono.poly.entry(gen_int_idx);
        let (gen_deg, gen_idx) = self
            .polynomial_monomials()
            .internal_idx_to_gen_deg(gen_int_idx);
        rest_mono.poly.set_entry(gen_int_idx, gen_exp - 1);
        let rest_mono_new_deg = rest_mono.degree - gen_deg;
        self.set_monomial_degree(&mut rest_mono, rest_mono_new_deg);
        let result_degree = mono.degree + q * sq;
        let mut result =
            FpVector::new(p, Algebra::dimension(self, result_degree, i32::max_value()));
        for poly_sq in 0..=sq {
            let rest_sq = sq - poly_sq;
            let rest_deg = rest_mono.degree + q * rest_sq;
            reducer_a.set_scratch_vector_size(Algebra::dimension(self, rest_deg, i32::max_value()));
            self.sq_monomial_to_polynomial(reducer_a.as_slice_mut(), 1, rest_sq, &rest_mono);
            let term_deg = q * poly_sq + gen_deg;
            term.set_scratch_vector_size(Algebra::dimension(self, term_deg, i32::max_value()));
            self.sq_polynomial_generator_to_polynomial(
                term.as_slice_mut(),
                1,
                poly_sq,
                gen_deg,
                gen_idx,
            );
            reducer_b.set_scratch_vector_size(Algebra::dimension(
                self,
                rest_deg + term_deg,
                i32::max_value(),
            ));
            self.multiply_polynomials(reducer_b, 1, rest_deg, reducer_a, term_deg, term);
            std::mem::swap(reducer_a, reducer_b);
            reducer_a.set_scratch_vector_size(Algebra::dimension(
                self,
                result_degree,
                i32::max_value(),
            ));
            result.add(reducer_a, 1);
        }
        result
    }

    fn compute_bockstein_table(&self, degree: i32) {
        if degree == 1 {
            self.bockstein_table()
                .push(vec![FpVector::new(self.prime(), 1)]);
            return;
        }
        let p = self.prime();
        // Build action table
        // degree -> monomial idx -> result vector
        let mut term = FpVector::new(p, 0);
        let rest_degree = degree - 1;
        let dim = Algebra::dimension(self, rest_degree, i32::max_value());
        let mut table = Vec::with_capacity(dim);
        // println!("degree : {}", degree);
        for basis_idx in 0..dim {
            // println!("   b({}) == ?", Module::basis_element_to_string(self, rest_degree, basis_idx));
            let mono = self.index_to_monomial(rest_degree, basis_idx);
            // println!("   mono : {}", mono);
            let mut rest_mono = mono.clone();
            let mut result = FpVector::new(p, Algebra::dimension(self, degree, i32::max_value()));
            let used_ext_generator;
            let gen_deg;
            let gen_int_idx;
            let gen_idx;
            let coeff;
            if let Some((gen_int_idx_, _)) = mono.ext.iter_nonzero().next() {
                used_ext_generator = true;
                gen_int_idx = gen_int_idx_;
                let (a, b) = self
                    .exterior_monomials()
                    .internal_idx_to_gen_deg(gen_int_idx);
                gen_deg = a;
                gen_idx = b;
                coeff = *p - 1;
                rest_mono.ext.set_entry(gen_int_idx, 0);
            } else if let Some((gen_int_idx_, gen_exp)) = mono.poly.iter_nonzero().next() {
                used_ext_generator = false;
                gen_int_idx = gen_int_idx_;
                let (a, b) = self
                    .polynomial_monomials()
                    .internal_idx_to_gen_deg(gen_int_idx);
                gen_deg = a;
                gen_idx = b;
                coeff = 1;
                rest_mono.poly.set_entry(gen_int_idx, gen_exp - 1);
            } else {
                unreachable!();
            }
            let rest_mono_new_deg = rest_mono.degree - gen_deg;
            self.set_monomial_degree(&mut rest_mono, rest_mono_new_deg);
            // result += (b x) * rest
            term.set_scratch_vector_size(Algebra::dimension(self, gen_deg + 1, i32::max_value()));
            if used_ext_generator {
                self.bockstein_exterior_generator_to_polynomial(
                    term.as_slice_mut(),
                    1,
                    gen_deg,
                    gen_idx,
                );
                // println!("   ext gen : b({}) = {}", self.repr_ext_generator(gen_deg, gen_idx), Module::element_to_string(self, gen_deg + 1, &term));
            } else {
                self.bockstein_polynomial_generator_to_polynomial(
                    term.as_slice_mut(),
                    1,
                    gen_deg,
                    gen_idx,
                );
                // println!("   poly gen : b({:?}) = {}", self.repr_poly_generator(gen_deg, gen_idx), Module::element_to_string(self, gen_deg + 1, &term));
                // println!("   rest_mono : {}", rest_mono);
            }
            self.multiply_polynomial_by_monomial(&mut result, 1, gen_deg + 1, &term, &rest_mono);
            // println!("   term : {}, rest_mono : {}",
            //     Module::element_to_string(self, gen_deg + 1, &term),
            //     rest_mono
            // );
            // println!("   b({}) == {} + ??",
            //     Module::basis_element_to_string(self, rest_degree, basis_idx), Module::element_to_string(self, degree, &result)
            // );
            // result += x * (b rest)
            // First get "b rest" into "term"
            let brest_degree = rest_mono.degree + 1;
            term.set_scratch_vector_size(Algebra::dimension(self, brest_degree, i32::max_value()));
            self.bockstein_monomial_to_polynomial(term.as_slice_mut(), 1, &rest_mono);
            // Now get "x" into "rest_mono"
            let mut x_mono = rest_mono;
            self.set_monomial_degree(&mut x_mono, gen_deg);
            x_mono.ext.set_to_zero();
            x_mono.poly.set_to_zero();
            if used_ext_generator {
                x_mono.ext.set_entry(gen_int_idx, 1);
            } else {
                x_mono.poly.set_entry(gen_int_idx, 1);
            }

            self.multiply_monomial_by_polynomial(&mut result, coeff, &x_mono, brest_degree, &term);
            // println!("   b({}) == {}\n",
            //     Module::basis_element_to_string(self, rest_degree, basis_idx), Module::element_to_string(self, degree, &result),
            // );
            table.push(result);
        }
        self.bockstein_table().push(table);
    }
}

impl<Adem: AdemAlgebraT, A: PolynomialAlgebraModule<Algebra = Adem> + Send + Sync> Module for A {
    type Algebra = Adem;
    fn algebra(&self) -> Arc<Self::Algebra> {
        PolynomialAlgebraModule::algebra(self)
    }

    fn min_degree(&self) -> i32 {
        0
    }

    fn max_computed_degree(&self) -> i32 {
        self.basis_table().len() as i32 - 1
    }

    fn dimension(&self, degree: i32) -> usize {
        if degree < 0 || degree < self.bottom_truncation() || degree > self.top_truncation() {
            0
        } else {
            self.basis_table()[degree as usize].index_to_monomial.len()
        }
    }

    fn compute_basis(&self, degree: i32) {
        let prev_max = Module::max_computed_degree(self);
        self.algebra().compute_basis(degree);
        Algebra::compute_basis(self, degree);
        for i in prev_max + 1..=degree {
            self.compute_action_table(i);
            if self.algebra().adem_algebra().generic {
                self.compute_bockstein_table(i);
            }
        }
    }

    fn basis_element_to_string(&self, degree: i32, index: usize) -> String {
        Algebra::basis_element_to_string(self, degree, index)
    }

    fn act_on_basis(
        &self,
        result: SliceMut,
        coeff: u32,
        op_deg: i32,
        op_index: usize,
        input_degree: i32,
        input_index: usize,
    ) {
        let mut input_vec = FpVector::new(
            self.prime(),
            Algebra::dimension(self, input_degree, i32::max_value()),
        );
        input_vec.set_entry(input_index, 1);
        self.act(
            result,
            coeff,
            op_deg,
            op_index,
            input_degree,
            input_vec.as_slice(),
        );
    }

    fn act(
        &self,
        mut result: SliceMut,
        coeff: u32,
        op_degree: i32,
        op_index: usize,
        input_degree: i32,
        input: Slice,
    ) {
        debug_assert!(result.as_slice().len() == Module::dimension(self, op_degree + input_degree));
        let algebra_outer = self.algebra();
        let algebra = algebra_outer.adem_algebra();
        let q = algebra.q();
        let op = algebra.basis_element_from_index(op_degree, op_index);
        let ps_len = op.ps.len();
        let mut cur_deg = input_degree;
        let mut source_vec = input.to_owned();
        let mut target_vec = FpVector::new(self.prime(), 0);
        if (op.bocksteins >> ps_len) & 1 == 1 {
            let op_deg = 1;
            target_vec.set_scratch_vector_size(Algebra::dimension(
                self,
                cur_deg + op_deg,
                i32::max_value(),
            ));
            self.bockstein_polynomial_to_polynomial(
                target_vec.as_slice_mut(),
                1,
                cur_deg,
                source_vec.as_slice(),
            );
            cur_deg += op_deg;
            std::mem::swap(&mut source_vec, &mut target_vec);
        }
        for i in (0..ps_len).rev() {
            let op_deg = op.ps[i] as i32 * q;
            target_vec.set_scratch_vector_size(Algebra::dimension(
                self,
                cur_deg + op_deg,
                i32::max_value(),
            ));
            self.sq_polynomial_to_polynomial(
                target_vec.as_slice_mut(),
                1,
                op.ps[i] as i32,
                cur_deg,
                source_vec.as_slice(),
            );
            cur_deg += op_deg;
            std::mem::swap(&mut source_vec, &mut target_vec);
            if (op.bocksteins >> i) & 1 == 1 {
                let op_deg = 1;
                target_vec.set_scratch_vector_size(Algebra::dimension(
                    self,
                    cur_deg + op_deg,
                    i32::max_value(),
                ));
                self.bockstein_polynomial_to_polynomial(
                    target_vec.as_slice_mut(),
                    1,
                    cur_deg,
                    source_vec.as_slice(),
                );
                cur_deg += op_deg;
                std::mem::swap(&mut source_vec, &mut target_vec);
            }
        }
        result.add(source_vec.as_slice(), coeff);
    }
}
