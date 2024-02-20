use std::collections::BTreeMap;

use anyhow::anyhow;
use fp::{
    prime::{Prime, ValidPrime},
    vector::FpVector,
};

use crate::{
    algebra::{adem_algebra::AdemBasisElement, AdemAlgebra, Algebra, MilnorAlgebra},
    milnor_algebra::{MilnorBasisElement, PPartEntry},
    steenrod_parser::*,
};

pub struct SteenrodEvaluator {
    pub adem: AdemAlgebra,
    pub milnor: MilnorAlgebra,
}

impl SteenrodEvaluator {
    pub fn new(p: ValidPrime) -> Self {
        Self {
            adem: AdemAlgebra::new(p, false),
            milnor: MilnorAlgebra::new(p, false),
        }
    }

    pub fn milnor_to_adem(&self, result: &mut FpVector, coeff: u32, degree: i32, input: &FpVector) {
        let p = self.prime();
        for (i, v) in input.iter_nonzero() {
            self.milnor_to_adem_on_basis(result, (coeff * v) % p, degree, i);
        }
    }

    pub fn adem_to_milnor(&self, result: &mut FpVector, coeff: u32, degree: i32, input: &FpVector) {
        let p = self.prime();
        for (i, v) in input.iter_nonzero() {
            self.adem_to_milnor_on_basis(result, (coeff * v) % p, degree, i);
        }
    }

    pub fn evaluate_algebra_adem(&self, input: &str) -> anyhow::Result<(i32, FpVector)> {
        self.evaluate_algebra_node(None, parse_algebra(input)?)
    }

    pub fn evaluate_algebra_milnor(&self, input: &str) -> anyhow::Result<(i32, FpVector)> {
        let adem_result = self.evaluate_algebra_adem(input);
        if let Ok((degree, adem_vector)) = adem_result {
            let mut milnor_vector = FpVector::new(adem_vector.prime(), adem_vector.len());
            self.adem_to_milnor(&mut milnor_vector, 1, degree, &adem_vector);
            Ok((degree, milnor_vector))
        } else {
            adem_result
        }
    }

    /// # Returns
    /// This returns a [`BTreeMap`] so that we get deterministic outputs for testing purposes
    pub fn evaluate_module_adem(
        &self,
        items: &str,
    ) -> anyhow::Result<BTreeMap<String, (i32, FpVector)>> {
        let mut result: BTreeMap<String, (i32, FpVector)> = BTreeMap::new();
        if items.is_empty() {
            return Ok(result);
        }
        for (op, gen) in parse_module(items)? {
            if let Some((deg, vec)) = result.get_mut(&gen) {
                let (_, adem_v) = self.evaluate_algebra_node(Some(*deg), op)?;
                vec.add(&adem_v, 1);
            } else {
                let (deg, adem_v) = self.evaluate_algebra_node(None, op)?;
                result.insert(gen, (deg, adem_v));
            }
        }
        Ok(result)
    }

    fn prime(&self) -> ValidPrime {
        self.adem.prime()
    }

    fn compute_basis(&self, degree: i32) {
        self.adem.compute_basis(degree);
        self.milnor.compute_basis(degree);
    }

    fn dimension(&self, degree: i32) -> usize {
        self.adem.dimension(degree)
    }

    fn evaluate_algebra_node(
        &self,
        mut output_degree: Option<i32>,
        tree: AlgebraNode,
    ) -> anyhow::Result<(i32, FpVector)> {
        let p = self.prime();
        match tree {
            AlgebraNode::Sum(left, right) => {
                let (degree, mut output_left) = self.evaluate_algebra_node(output_degree, *left)?;
                let (_, output_right) = self.evaluate_algebra_node(Some(degree), *right)?;
                output_left += &output_right;
                Ok((degree, output_left))
            }
            AlgebraNode::Product(left, right) => {
                let (degree_left, output_left) = self.evaluate_algebra_node(None, *left)?;
                if let Some(degree) = output_degree {
                    if degree < degree_left {
                        return Err(anyhow!("Mismatched degree"));
                    }
                    output_degree = Some(degree - degree_left);
                }
                let (degree_right, output_right) =
                    self.evaluate_algebra_node(output_degree, *right)?;
                let degree = degree_left + degree_right;
                self.compute_basis(degree);
                let mut result = FpVector::new(p, self.adem.dimension(degree));
                self.adem.multiply_element_by_element(
                    result.as_slice_mut(),
                    1,
                    degree_left,
                    output_left.as_slice(),
                    degree_right,
                    output_right.as_slice(),
                );
                Ok((degree, result))
            }
            AlgebraNode::BasisElt(basis_elt) => {
                self.evaluate_basis_element(output_degree, basis_elt)
            }
            AlgebraNode::Scalar(x) => {
                if let Some(degree) = output_degree {
                    if degree != 0 {
                        return Err(anyhow!("Mismatched Degree"));
                    }
                }
                let mut result = FpVector::new(p, 1);
                result.set_entry(0, x.rem_euclid(p.as_i32()) as u32);
                Ok((0, result))
            }
        }
    }

    fn evaluate_basis_element(
        &self,
        output_degree: Option<i32>,
        basis_elt: AlgebraBasisElt,
    ) -> anyhow::Result<(i32, FpVector)> {
        let p = self.prime();
        let q = self.adem.q();
        let (degree, result) = match basis_elt {
            AlgebraBasisElt::AList(p_or_b_list) => self.evaluate_p_or_b_list(&p_or_b_list),
            AlgebraBasisElt::PList(p_list) => {
                let degree = std::iter::zip(crate::algebra::combinatorics::xi_degrees(p), &p_list)
                    .map(|(&a, &b)| a * b as i32)
                    .sum::<i32>()
                    * q;
                let elt = MilnorBasisElement {
                    degree,
                    p_part: p_list,
                    q_part: 0,
                };

                self.compute_basis(degree);
                let mut result = FpVector::new(p, self.dimension(degree));
                self.milnor_to_adem_on_basis(
                    &mut result,
                    1,
                    degree,
                    self.milnor.basis_element_to_index(&elt),
                );
                (degree, result)
            }
            AlgebraBasisElt::P(x) => {
                self.compute_basis(x as i32 * q);
                let (degree, idx) = self.adem.beps_pn(0, x);
                let mut result = FpVector::new(p, self.dimension(degree));
                result.set_entry(idx, 1);
                (degree, result)
            }
            AlgebraBasisElt::Q(x) => {
                let tau_degrees = crate::algebra::combinatorics::tau_degrees(p);
                let degree = tau_degrees[x as usize];
                self.compute_basis(degree);
                let mut result = FpVector::new(p, self.dimension(degree));
                self.adem_q(&mut result, 1, x);
                (degree, result)
            }
        };
        if let Some(requested_degree) = output_degree {
            if degree != requested_degree {
                return Err(anyhow!("Mismatched degree"));
            }
        }
        Ok((degree, result))
    }

    /// Translate from the adem basis to the milnor basis, adding `coeff` times the result to `result`.
    /// This uses the fact that that $P^n = P(n)$ and $Q_1 = \beta$ and multiplies out the admissible
    /// monomial.
    fn adem_to_milnor_on_basis(&self, result: &mut FpVector, coeff: u32, degree: i32, idx: usize) {
        let elt = self.adem.basis_element_from_index(degree, idx);
        let p = self.prime();
        let dim = self.dimension(elt.degree);
        if dim == 1 {
            result.set_entry(0, coeff);
            return;
        }
        let mut tmp_vector_a = FpVector::new(p, 1);
        let mut tmp_vector_b = FpVector::new(p, 0);

        tmp_vector_a.set_entry(0, 1);

        let mut bocksteins = elt.bocksteins;
        let mut total_degree = 0;

        for &sqn in &elt.ps {
            let (deg, idx) = self.milnor.beps_pn(bocksteins & 1, sqn as PPartEntry);
            bocksteins >>= 1;
            self.compute_basis(total_degree + deg);

            tmp_vector_b.set_scratch_vector_size(self.dimension(total_degree + deg));
            self.milnor.multiply_element_by_basis_element(
                tmp_vector_b.as_slice_mut(),
                1,
                total_degree,
                tmp_vector_a.as_slice(),
                deg,
                idx,
            );
            total_degree += deg;
            std::mem::swap(&mut tmp_vector_a, &mut tmp_vector_b);
        }
        if bocksteins & 1 == 0 {
            result.add(&tmp_vector_a, coeff);
        } else {
            self.milnor.multiply_element_by_basis_element(
                result.as_slice_mut(),
                coeff,
                total_degree,
                tmp_vector_a.as_slice(),
                1,
                0,
            );
        }
    }

    // This is currently pretty inefficient... We should memoize results so that we don't repeatedly
    // recompute the same inverse.
    fn milnor_to_adem_on_basis(&self, result: &mut FpVector, coeff: u32, degree: i32, idx: usize) {
        if self.milnor.generic() {
            self.milnor_to_adem_on_basis_generic(result, coeff, degree, idx);
        } else {
            self.milnor_to_adem_on_basis_2(result, coeff, degree, idx);
        }
    }

    fn milnor_to_adem_on_basis_2(
        &self,
        result: &mut FpVector,
        coeff: u32,
        degree: i32,
        idx: usize,
    ) {
        let elt = self.milnor.basis_element_from_index(degree, idx);
        let p = self.prime();
        let dim = self.dimension(elt.degree);
        if dim == 1 {
            result.set_entry(0, coeff);
            return;
        }
        let mut t: Vec<u32> = vec![0; elt.p_part.len()];
        t[elt.p_part.len() - 1] = elt.p_part[elt.p_part.len() - 1];
        for i in (0..elt.p_part.len() - 1).rev() {
            t[i] = elt.p_part[i] + 2 * t[i + 1];
        }
        let t_idx = self.adem.basis_element_to_index(&AdemBasisElement {
            degree,
            bocksteins: 0,
            ps: t,
            p_or_sq: p != 2,
        });
        let mut tmp_vector_a = FpVector::new(p, dim);
        self.adem_to_milnor_on_basis(&mut tmp_vector_a, 1, degree, t_idx);
        assert!(tmp_vector_a.entry(idx) == 1);
        tmp_vector_a.set_entry(idx, 0);
        self.milnor_to_adem(result, coeff, degree, &tmp_vector_a);
        result.add_basis_element(t_idx, coeff);
    }

    fn milnor_to_adem_on_basis_generic(
        &self,
        result: &mut FpVector,
        coeff: u32,
        degree: i32,
        idx: usize,
    ) {
        let elt = self.milnor.basis_element_from_index(degree, idx);
        let p = self.prime();
        let dim = self.dimension(elt.degree);
        if dim == 1 {
            result.set_entry(0, coeff);
            return;
        }
        let t_len = std::cmp::max(
            elt.p_part.len(),
            (31u32.saturating_sub(elt.q_part.leading_zeros())) as usize,
        );
        let mut t = vec![0; t_len];
        let last_p_part = if t_len <= elt.p_part.len() {
            elt.p_part[t_len - 1]
        } else {
            0
        };
        t[t_len - 1] = last_p_part + ((elt.q_part >> (t_len)) & 1);
        for i in (0..t_len - 1).rev() {
            let p_part = if i < elt.p_part.len() {
                elt.p_part[i]
            } else {
                0
            };
            t[i] = p_part + ((elt.q_part >> (i + 1)) & 1) + p * t[i + 1];
        }
        let t_idx = self.adem.basis_element_to_index(&AdemBasisElement {
            degree,
            bocksteins: elt.q_part,
            ps: t,
            p_or_sq: p != 2,
        });
        let mut tmp_vector_a = FpVector::new(p, dim);
        self.adem_to_milnor_on_basis(&mut tmp_vector_a, 1, degree, t_idx);
        assert!(tmp_vector_a.entry(idx) == 1);
        tmp_vector_a.set_entry(idx, 0);
        tmp_vector_a.scale(p - 1);
        self.milnor_to_adem(result, coeff, degree, &tmp_vector_a);
        result.add_basis_element(t_idx, coeff);
    }

    /// Express $Q_{qi}$ in the adem basis.
    fn adem_q(&self, result: &mut FpVector, coeff: u32, qi: u32) {
        let p = self.prime();
        let degree = crate::algebra::combinatorics::tau_degrees(p)[qi as usize];
        let mbe = if self.adem.generic() {
            MilnorBasisElement {
                degree,
                q_part: 1 << qi,
                p_part: vec![],
            }
        } else {
            let mut p_part = vec![0; qi as usize + 1];
            p_part[qi as usize] = 1;
            MilnorBasisElement {
                degree,
                q_part: 0,
                p_part,
            }
        };
        let idx = self.milnor.basis_element_to_index(&mbe);
        self.milnor_to_adem_on_basis(result, coeff, degree, idx);
    }

    fn evaluate_p_or_b_list(&self, list: &[BocksteinOrSq]) -> (i32, FpVector) {
        let p = self.prime();
        let q = self.adem.q();

        let mut total_degree = 0;

        let mut tmp_vector_a = FpVector::new(p, 1);
        let mut tmp_vector_b = FpVector::new(p, 0);

        tmp_vector_a.set_entry(0, 1);

        for item in list {
            let cur_elt = item.to_adem_basis_elt(q);

            self.compute_basis(total_degree + cur_elt.degree);
            tmp_vector_b.set_scratch_vector_size(self.dimension(total_degree + cur_elt.degree));
            self.adem.multiply_element_by_basis_element(
                tmp_vector_b.as_slice_mut(),
                1,
                total_degree,
                tmp_vector_a.as_slice(),
                cur_elt.degree,
                self.adem.basis_element_to_index(&cur_elt),
            );
            total_degree += cur_elt.degree;
            std::mem::swap(&mut tmp_vector_a, &mut tmp_vector_b);
        }
        (total_degree, tmp_vector_a)
    }
}

#[cfg(test)]
mod tests {
    use expect_test::{expect, Expect};
    use rstest::rstest;

    use super::*;

    #[test]
    fn test_evaluate_2() {
        let ev = SteenrodEvaluator::new(ValidPrime::new(2));

        let check = |input, adem_output: Expect, milnor_output: Expect| {
            let (degree, result) = ev.evaluate_algebra_adem(input).unwrap();
            adem_output.assert_eq(&ev.adem.element_to_string(degree, result.as_slice()));

            let (degree, result) = ev.evaluate_algebra_milnor(input).unwrap();
            milnor_output.assert_eq(&ev.milnor.element_to_string(degree, result.as_slice()));
        };

        check(
            "Sq2 * Sq2",
            expect![[r#"Sq3 Sq1"#]],
            expect![[r#"P(1, 1)"#]],
        );
        check("A(2 2)", expect![[r#"Sq3 Sq1"#]], expect![[r#"P(1, 1)"#]]);
        check(
            "Sq2 * Sq2 * Sq2 + Sq3 * Sq3",
            expect![[r#"0"#]],
            expect![[r#"0"#]],
        );
        check(
            "Sq2 * (Sq2 * Sq2 + Sq4)",
            expect![[r#"Sq6"#]],
            expect![[r#"P(6)"#]],
        );
        check(
            "Sq7 + Q2",
            expect![[r#"Sq5 Sq2 + Sq6 Sq1 + Sq4 Sq2 Sq1"#]],
            expect![[r#"P(7) + P(0, 0, 1)"#]],
        );
        check(
            "(Q2 + Sq7) * Q1",
            expect![[r#"Sq6 Sq3 Sq1"#]],
            expect![[r#"P(7, 1) + P(3, 0, 1) + P(0, 1, 1)"#]],
        );
    }

    #[test]
    fn test_evaluate_3() {
        let ev = SteenrodEvaluator::new(ValidPrime::new(3));

        let check = |input, adem_output: Expect, milnor_output: Expect| {
            let (degree, result) = ev.evaluate_algebra_adem(input).unwrap();
            adem_output.assert_eq(&ev.adem.element_to_string(degree, result.as_slice()));

            let (degree, result) = ev.evaluate_algebra_milnor(input).unwrap();
            milnor_output.assert_eq(&ev.milnor.element_to_string(degree, result.as_slice()));
        };

        check("P1 * P1", expect![[r#"2 * P2"#]], expect![[r#"2 * P(2)"#]]);
        check("A(1 1)", expect![[r#"2 * P2"#]], expect![[r#"2 * P(2)"#]]);
        check(
            "A(1 b 1)",
            expect![[r#"b P2 + P2 b"#]],
            expect![[r#"2 * Q_0 P(2) + Q_1 P(1)"#]],
        );
        check(
            "A(4 2)",
            expect![[r#"2 * P5 P1"#]],
            expect![[r#"2 * P(2, 1)"#]],
        );
        check("A(5 2)", expect![[r#"0"#]], expect![[r#"0"#]]);
        check(
            "A(6 2)",
            expect![[r#"P6 P2"#]],
            expect![[r#"P(8) + P(4, 1) + P(0, 2)"#]],
        );
    }

    #[rstest(p, max_degree, case(2, 32), case(3, 60))]
    #[trace]
    fn test_cob_adem_to_milnor(p: u32, max_degree: i32) {
        let p = ValidPrime::new(p);
        let ev = SteenrodEvaluator::new(p);
        ev.compute_basis(max_degree);

        for degree in 0..max_degree {
            println!("degree : {degree}");
            let dim = ev.dimension(degree);
            let mut milnor_result = FpVector::new(p, dim);
            let mut adem_result = FpVector::new(p, dim);
            for i in 0..dim {
                // println!("i : {}", i);
                ev.milnor_to_adem_on_basis(&mut adem_result, 1, degree, i);
                ev.adem_to_milnor(&mut milnor_result, 1, degree, &adem_result);
                assert!(
                    milnor_result.entry(i) == 1,
                    "{} ==> {} ==> {}",
                    ev.milnor.basis_element_to_string(degree, i),
                    ev.adem.element_to_string(degree, adem_result.as_slice()),
                    ev.milnor
                        .element_to_string(degree, milnor_result.as_slice())
                );
                milnor_result.set_entry(i, 0);
                assert!(
                    milnor_result.is_zero(),
                    "{} ==> {} ==> {}",
                    ev.milnor.basis_element_to_string(degree, i),
                    ev.adem.element_to_string(degree, adem_result.as_slice()),
                    ev.milnor
                        .element_to_string(degree, milnor_result.as_slice())
                );
                adem_result.set_to_zero();
                milnor_result.set_to_zero();
            }
        }
    }
}
