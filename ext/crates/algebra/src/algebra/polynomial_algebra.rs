use itertools::Itertools;
use rustc_hash::FxHashMap as HashMap;
use std::fmt;

use fp::prime::ValidPrime;
use fp::vector::{FpVector, SliceMut};
use once::OnceVec;

use crate::algebra::combinatorics::TruncatedPolynomialMonomialBasis;
use crate::algebra::Algebra;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolynomialAlgebraMonomial {
    pub degree: i32,
    pub poly: FpVector,
    pub ext: FpVector,
    pub valid: bool,
}

impl fmt::Display for PolynomialAlgebraMonomial {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "UAM(degree={}, valid={}, poly={}, ext={})",
            self.degree, self.valid, self.poly, self.ext
        )?;
        Ok(())
    }
}

impl PolynomialAlgebraMonomial {
    pub fn new(p: ValidPrime) -> Self {
        Self {
            degree: 0xFEDCBA9, // Looks invalid to me!
            poly: FpVector::new(p, 0),
            ext: FpVector::new(ValidPrime::new(2), 0),
            valid: true,
        }
    }
}

pub struct PolynomialAlgebraTableEntry {
    pub index_to_monomial: Vec<PolynomialAlgebraMonomial>, // degree -> index -> AdemBasisElement
    pub monomial_to_index: HashMap<PolynomialAlgebraMonomial, usize>, // degree -> AdemBasisElement -> index
}

impl Default for PolynomialAlgebraTableEntry {
    fn default() -> Self {
        Self {
            index_to_monomial: Vec::new(),
            monomial_to_index: HashMap::default(),
        }
    }
}

impl PolynomialAlgebraTableEntry {
    pub fn new() -> Self {
        Self::default()
    }
}

#[allow(clippy::derive_hash_xor_eq)]
impl std::hash::Hash for PolynomialAlgebraMonomial {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.poly.hash(state);
        self.ext.hash(state);
    }
}

pub trait PolynomialAlgebra: std::fmt::Display + Sized + Send + Sync {
    fn prime(&self) -> ValidPrime;

    fn polynomial_monomials(&self) -> &TruncatedPolynomialMonomialBasis;
    fn exterior_monomials(&self) -> &TruncatedPolynomialMonomialBasis;

    fn min_degree(&self) -> i32 {
        0
    }

    fn polynomial_generators_in_degree(&self, degree: i32) -> usize;
    fn exterior_generators_in_degree(&self, degree: i32) -> usize;
    fn repr_poly_generator(&self, degree: i32, _index: usize) -> (String, u32);
    fn repr_ext_generator(&self, degree: i32, _index: usize) -> String;

    fn basis_table(&self) -> &OnceVec<PolynomialAlgebraTableEntry>;

    fn frobenius_on_generator(&self, degree: i32, index: usize) -> Option<usize>;
    fn compute_generating_set(&self, degree: i32);

    fn compute_basis_step(&self, degree: i32) {
        assert!(degree as usize == self.basis_table().len());
        let num_poly_gens = self.polynomial_generators_in_degree(degree);
        let num_ext_gens = self.exterior_generators_in_degree(degree);
        let poly_parts = self.polynomial_monomials();
        let ext_parts = self.exterior_monomials();
        if degree > 0 {
            poly_parts.add_gens_and_calculate_parts(degree, num_poly_gens);
            ext_parts.add_gens_and_calculate_parts(degree, num_ext_gens);
        }
        let mut table = PolynomialAlgebraTableEntry::new();
        for poly_deg in 0..=degree {
            let ext_deg = degree - poly_deg;
            for p in poly_parts.parts(poly_deg) {
                for e in ext_parts.parts(ext_deg) {
                    let index = table.index_to_monomial.len();
                    let mut m = PolynomialAlgebraMonomial {
                        degree,
                        poly: p.clone(),
                        ext: e.clone(),
                        valid: true,
                    };
                    self.set_monomial_degree(&mut m, degree);
                    table.monomial_to_index.insert(m.clone(), index);
                    table.index_to_monomial.push(m);
                }
            }
        }
        self.basis_table().push(table);
    }

    fn monomial_to_index(&self, monomial: &PolynomialAlgebraMonomial) -> usize {
        self.basis_table()[monomial.degree as usize]
            .monomial_to_index
            .get(monomial)
            .copied()
            .unwrap_or_else(|| panic!("Didn't find monomial: {}", monomial))
    }

    fn index_to_monomial(&self, degree: i32, index: usize) -> &PolynomialAlgebraMonomial {
        &self.basis_table()[degree as usize].index_to_monomial[index]
    }

    fn frobenius_monomial(&self, target: &mut FpVector, source: &FpVector) {
        let p = *self.prime() as i32;
        for (i, c) in source.iter_nonzero() {
            let (gen_degree, gen_index) = self.polynomial_monomials().internal_idx_to_gen_deg(i);
            let frob = self.frobenius_on_generator(gen_degree, gen_index);
            if let Some(e) = frob {
                let out_idx = self
                    .polynomial_monomials()
                    .gen_deg_idx_to_internal_idx(p * gen_degree, e);
                target.add_basis_element(out_idx, c);
            }
        }
    }

    fn multiply_monomials(
        &self,
        target: &mut PolynomialAlgebraMonomial,
        source: &PolynomialAlgebraMonomial,
    ) -> Option<u32> {
        let minus_one = *self.prime() - 1;
        self.set_monomial_degree(target, target.degree + source.degree);
        let mut temp_source_ext = source.ext.clone();
        temp_source_ext.set_scratch_vector_size(target.ext.len());
        // If we made sign_rule handle vectors of different lengths, we could avoid cloning ext here.
        let coeff = if target.ext.sign_rule(&temp_source_ext) {
            minus_one
        } else {
            1
        };
        target.ext.add_truncate(&temp_source_ext, 1)?;

        let mut carry_vec = [FpVector::new(self.prime(), target.poly.len())];
        let mut source_vec = source.poly.clone();
        source_vec.set_scratch_vector_size(target.poly.len());
        let mut carry_q = true;
        while carry_q {
            carry_q = target.poly.add_carry(&source_vec, 1, &mut carry_vec);
            if carry_q {
                source_vec.set_to_zero();
                self.frobenius_monomial(&mut source_vec, &carry_vec[0]);
                carry_vec[0].set_to_zero();
            }
        }
        Some(coeff)
    }

    fn multiply_polynomials(
        &self,
        target: &mut FpVector,
        coeff: u32,
        left_degree: i32,
        left: &FpVector,
        right_degree: i32,
        right: &FpVector,
    ) {
        let p = *self.prime();
        target
            .set_scratch_vector_size(self.dimension(left_degree + right_degree, i32::max_value()));
        for (left_idx, left_entry) in left.iter_nonzero() {
            for (right_idx, right_entry) in right.iter_nonzero() {
                let mut target_mono = self.index_to_monomial(left_degree, left_idx).clone();
                let source_mono = self.index_to_monomial(right_degree, right_idx);
                let nonzero_result = self.multiply_monomials(&mut target_mono, source_mono);
                if let Some(c) = nonzero_result {
                    let idx = self.monomial_to_index(&target_mono);
                    target.add_basis_element(idx, (left_entry * right_entry * c * coeff) % p);
                }
            }
        }
    }

    fn multiply_polynomial_by_monomial(
        &self,
        target: &mut FpVector,
        coeff: u32,
        left_degree: i32,
        left: &FpVector,
        right_mono: &PolynomialAlgebraMonomial,
    ) {
        let p = *self.prime();
        target.extend_len(self.dimension(left_degree + right_mono.degree, i32::max_value()));
        for (left_idx, left_entry) in left.iter_nonzero() {
            let mut target_mono = self.index_to_monomial(left_degree, left_idx).clone(); // Could reduce cloning a bit but probably best not to worry.
            let nonzero_result = self.multiply_monomials(&mut target_mono, right_mono);
            if let Some(c) = nonzero_result {
                let idx = self.monomial_to_index(&target_mono);
                target.add_basis_element(idx, (left_entry * c * coeff) % p);
            }
        }
    }

    // At p=2 this is redundant but at odd primes one must worry about signs.
    fn multiply_monomial_by_polynomial(
        &self,
        target: &mut FpVector,
        coeff: u32,
        left_mono: &PolynomialAlgebraMonomial,
        right_degree: i32,
        right: &FpVector,
    ) {
        let p = *self.prime();
        target.extend_len(self.dimension(right_degree + left_mono.degree, i32::max_value()));
        for (right_idx, right_entry) in right.iter_nonzero() {
            let mut target_mono = left_mono.clone(); // Could reduce cloning a bit but probably best not to worry.
            let right_mono = self.index_to_monomial(right_degree, right_idx);
            let nonzero_result = self.multiply_monomials(&mut target_mono, right_mono);
            if let Some(c) = nonzero_result {
                let idx = self.monomial_to_index(&target_mono);
                target.add_basis_element(idx, (right_entry * c * coeff) % p);
            }
        }
    }

    fn set_monomial_degree(&self, mono: &mut PolynomialAlgebraMonomial, degree: i32) {
        mono.degree = degree;
        mono.ext.set_scratch_vector_size(
            self.exterior_monomials()
                .generators_up_to_degree(mono.degree),
        );
        mono.poly.set_scratch_vector_size(
            self.polynomial_monomials()
                .generators_up_to_degree(mono.degree),
        );
    }

    fn max_computed_degree(&self) -> i32 {
        self.basis_table().len() as i32 - 1
    }
}

impl<A: PolynomialAlgebra> Algebra for A {
    fn prime(&self) -> ValidPrime {
        self.prime()
    }

    fn compute_basis(&self, degree: i32) {
        self.compute_generating_set(degree);
        for i in self.max_computed_degree() + 1..=degree {
            self.compute_basis_step(i);
        }
    }

    fn dimension(&self, degree: i32, _excess: i32) -> usize {
        if degree < 0 {
            0
        } else {
            self.basis_table()[degree as usize].index_to_monomial.len()
        }
    }

    fn basis_element_to_string(&self, degree: i32, index: usize) -> String {
        let mono = self.index_to_monomial(degree, index);
        let mut exp_map = HashMap::default();
        for (i, e) in mono.poly.iter_nonzero() {
            let (gen_deg, gen_idx) = self.polynomial_monomials().internal_idx_to_gen_deg(i);
            let (var, var_exp) = self.repr_poly_generator(gen_deg, gen_idx);
            let entry = exp_map.entry(var).or_insert((0, gen_deg / var_exp as i32));
            entry.0 += (e * var_exp) as i32;
        }
        let result = exp_map
            .iter()
            .sorted_by_key(|(_, &(_, gen_deg))| gen_deg)
            .map(|(var, &(var_exp, gen_deg))| {
                let pow = if var_exp > 1 {
                    format!("^{{{}}}", var_exp)
                } else {
                    "".to_string()
                };
                let s = format!("{}{}", var, pow);
                (s, gen_deg)
            })
            .merge_by(
                mono.ext.iter_nonzero().map(|(i, _)| {
                    let (gen_deg, gen_idx) = self.exterior_monomials().internal_idx_to_gen_deg(i);
                    let var = self.repr_ext_generator(gen_deg, gen_idx);
                    (var, gen_deg)
                }),
                |x, y| x.1 < y.1,
            )
            .map(|(v, _gen_deg)| v)
            .join(" ");
        if result.is_empty() {
            "1".to_string()
        } else {
            result
        }
    }

    fn multiply_basis_elements(
        &self,
        mut result: SliceMut,
        coeff: u32,
        left_degree: i32,
        left_idx: usize,
        right_degree: i32,
        right_idx: usize,
        _excess: i32,
    ) {
        if coeff == 0 {
            return;
        }
        let mut target = self.index_to_monomial(left_degree, left_idx).clone();
        let source = self.index_to_monomial(right_degree, right_idx);
        if self.multiply_monomials(&mut target, source).is_some() {
            let idx = self.monomial_to_index(&target);
            result.add_basis_element(idx, coeff);
        }
    }
}
