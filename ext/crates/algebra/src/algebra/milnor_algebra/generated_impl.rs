use fp::{
    prime::{Binomial, Prime, factor_pk},
    vector::FpVector,
};

use super::{MilnorAlgebra, MilnorBasisElement, PPart, PPartEntry};
use crate::algebra::{Algebra, GeneratedAlgebra, combinatorics};

impl GeneratedAlgebra for MilnorAlgebra {
    fn generator_to_string(&self, degree: i32, idx: usize) -> String {
        if self.generic() {
            if degree == 1 {
                return "b".to_string();
            }
            let elt = self.basis_element_from_index(degree, idx);
            let len = elt.p_part.len();
            if elt.q_part != 0 {
                elt.to_string()
            } else if len == 1 {
                format!("P{}", degree / self.q())
            } else {
                format!(
                    "P^{}_{}",
                    degree / (self.q() * combinatorics::xi_degrees(self.prime())[len - 1]),
                    len
                )
            }
        } else {
            let elt = self.basis_element_from_index(degree, idx);
            let len = elt.p_part.len();
            if len == 1 {
                format!("Sq{degree}")
            } else {
                format!(
                    "P^{}_{}",
                    degree / (combinatorics::xi_degrees(self.prime())[len - 1]),
                    len
                )
            }
        }
    }

    fn generators(&self, degree: i32) -> Vec<usize> {
        if degree <= 0 {
            return vec![];
        } else if degree == 1 {
            return vec![0]; // Q_0
        }

        let p = self.prime();

        // Check for the Q_k
        if self.generic() && degree % 2 == 1 {
            if self.profile.is_an(true) {
                return vec![];
            }

            // If this is 2p^k - 1, then return Q_k
            if let (k, 2) = factor_pk(p, degree as u32 + 1) {
                let q_part = 1 << k;
                if self.profile.q_part & q_part != 0 {
                    return vec![self.basis_element_to_index(&MilnorBasisElement {
                        degree,
                        q_part,
                        p_part: PPart::zero(),
                    })];
                }
            }
            return vec![];
        }

        if self.profile.is_an(self.generic()) {
            // Look for P(p^k), which has degree p^k q.
            let q = self.q() as u32;
            if !(degree as u32).is_multiple_of(q) {
                return vec![];
            }
            if let (k, 1) = factor_pk(p, degree as u32 / q)
                && (k) < self.profile.get_p_part(0)
            {
                return vec![self.basis_element_to_index(&MilnorBasisElement {
                    degree,
                    q_part: 0,
                    p_part: PPart::from_iter([degree as u32 / q]),
                })];
            }
            vec![]
        } else {
            // Look for P(0, ..., 0, p^k), which has degree (2p^j - 2) p^k.
            let (k, rem) = factor_pk(p, degree as u32);

            let reduced = if self.generic() {
                // rem must be even because degree is even
                (rem + 2) / 2
            } else {
                rem + 1
            };

            if let (j, 1) = factor_pk(p, reduced) {
                if self.profile.get_p_part(j as usize - 1) <= k {
                    return vec![];
                }
                let mut p_part = PPart::zero();
                p_part.set(j as usize - 1, p.pow(k));
                return vec![self.basis_element_to_index(&MilnorBasisElement {
                    degree,
                    q_part: 0,
                    p_part,
                })];
            }
            vec![]
        }
    }

    fn decompose_basis_element(
        &self,
        degree: i32,
        idx: usize,
    ) -> Vec<(u32, (i32, usize), (i32, usize))> {
        let basis = self.basis_element_from_index(degree, idx);
        // If qpart = 0, return self
        if basis.q_part == 0 {
            self.decompose_basis_element_ppart(degree, idx)
        } else {
            self.decompose_basis_element_qpart(degree, idx)
        }
    }

    fn generating_relations(&self, degree: i32) -> Vec<Vec<(u32, (i32, usize), (i32, usize))>> {
        if self.generic() && degree == 2 {
            // beta^2 = 0 is an edge case
            return vec![vec![(1, (1, 0), (1, 0))]];
        }
        let p = self.prime();
        let inadmissible_pairs = combinatorics::inadmissible_pairs(p, self.generic(), degree);
        let mut result = Vec::new();
        for (x, b, y) in inadmissible_pairs {
            let mut relation = Vec::new();
            // Adem relation. Sometimes these don't exist because of profiles. Then just ignore it.
            (|| {
                let (first_degree, first_index) = self.try_beps_pn(0, x)?;
                let (second_degree, second_index) = self.try_beps_pn(b, y)?;
                relation.push((
                    p - 1,
                    (first_degree, first_index),
                    (second_degree, second_index),
                ));
                for e1 in 0..=b {
                    let e2 = b - e1;
                    // e1 and e2 determine where a bockstein shows up.
                    // e1 determines whether a bockstein shows up in front
                    // e2 determines whether a bockstein shows up in middle
                    // So our output term looks like b^{e1} P^{x+y-j} b^{e2} P^{j}
                    for j in 0..=x / p {
                        let c = combinatorics::adem_relation_coefficient(p, x, y, j, e1, e2);
                        if c == 0 {
                            continue;
                        }
                        if j == 0 {
                            relation.push((c, self.try_beps_pn(e1, x + y)?, (e2 as i32, 0)));
                            continue;
                        }
                        let first_sq = self.try_beps_pn(e1, x + y - j)?;
                        let second_sq = self.try_beps_pn(e2, j)?;
                        relation.push((c, first_sq, second_sq));
                    }
                }
                result.push(relation);
                Some(())
            })();
        }
        result
    }
}

impl MilnorAlgebra {
    fn decompose_basis_element_qpart(
        &self,
        degree: i32,
        idx: usize,
    ) -> Vec<(u32, (i32, usize), (i32, usize))> {
        let basis = self.basis_element_from_index(degree, idx);
        // Look for left-most non-zero qpart
        let i = basis.q_part.trailing_zeros();
        // If the basis element is just Q_{k+1}, we decompose Q_{k+1} = P(p^k) Q_k - Q_k P(p^k).
        if basis.q_part == 1 << i && basis.p_part.is_empty() {
            let ppow = self.prime().pow(i - 1);

            let q_degree = (2 * ppow - 1) as i32;
            let p_degree = (ppow * (2 * self.prime() - 2)) as i32;

            let p_idx = self
                .basis_element_to_index(&MilnorBasisElement::from_p(
                    PPart::from_iter([ppow]),
                    p_degree,
                ))
                .to_owned();

            let q_idx = self
                .basis_element_to_index(&MilnorBasisElement {
                    q_part: 1 << (i - 1),
                    p_part: PPart::zero(),
                    degree: q_degree,
                })
                .to_owned();

            vec![
                (1, (p_degree, p_idx), (q_degree, q_idx)),
                (self.prime() - 1, (q_degree, q_idx), (p_degree, p_idx)),
            ]
        } else {
            // Otherwise, separate out the first Q_k.
            let first_degree = combinatorics::tau_degrees(self.prime())[i as usize];
            let second_degree = degree - first_degree;

            let first_idx = self.basis_element_to_index(&MilnorBasisElement {
                q_part: 1 << i,
                p_part: PPart::zero(),
                degree: first_degree,
            });

            let second_idx = self.basis_element_to_index(&MilnorBasisElement {
                q_part: basis.q_part ^ (1 << i),
                p_part: basis.p_part,
                degree: second_degree,
            });

            vec![(1, (first_degree, first_idx), (second_degree, second_idx))]
        }
    }

    fn decompose_basis_element_ppart(
        &self,
        degree: i32,
        idx: usize,
    ) -> Vec<(u32, (i32, usize), (i32, usize))> {
        let p = self.prime();

        // We define an ordering on the p parts as follows: we order each entry in reverse, and
        // then impose the reverse lexicographic ordering. Then for each non-generator P(R), `init`
        // is a partial decomposition such that the non-zero terms in the `init` product are all
        // greater than or equal to P(R) (and P(R) has non-zero coefficient in `init`). We can then
        // apply this algorithm recursively to decompose an element.

        // result is the products we have added so far
        let mut result = Vec::new();
        // buffer is the products we are adding in the current iteration
        let mut buffer = Vec::new();

        // out_vec is the remaining items we have to kill. We are done when this hits zero.
        let mut out_vec = FpVector::new(p, self.dimension(degree));
        out_vec.set_entry(idx, p - 1);

        while let Some((idx, c)) = out_vec.iter_nonzero().next() {
            let b = self.basis_element_from_index(degree, idx);
            let len = b.p_part.len();

            if b.p_part.truncate(len - 1).is_empty() {
                // There is only one entry
                let entry = b.p_part.get(len - 1);
                let (k, m) = factor_pk(p, entry);

                // This is a power of p
                if m == 1 {
                    if len == 1 || !self.profile.is_an(self.generic()) {
                        buffer.extend([(p - c, (degree, idx), (0, 0))]);
                    } else {
                        // Write this as [P(p^(len + k - 1)), P(0, .., 0, P^k)] plus higher order
                        // terms.
                        let l_entry = p.pow(len as u32 + k - 1);
                        let r_entry = p.pow(k);

                        let l_degree = l_entry as i32 * self.q();
                        let l_index = self.basis_element_to_index(&MilnorBasisElement {
                            q_part: 0,
                            p_part: PPart::from_iter([l_entry]),
                            degree: l_degree,
                        });

                        let mut r_p_part = PPart::zero();
                        r_p_part.set(len - 2, r_entry);
                        let r_degree =
                            r_entry as i32 * combinatorics::xi_degrees(p)[len - 2] * self.q();

                        let r_index = self.basis_element_to_index(&MilnorBasisElement {
                            q_part: 0,
                            p_part: r_p_part,
                            degree: r_degree,
                        });
                        buffer.extend(vec![
                            (p - c, (l_degree, l_index), (r_degree, r_index)),
                            (c, (r_degree, r_index), (l_degree, l_index)),
                        ])
                    }
                } else {
                    // This is not a power of p. Just subtract the lowest power of p.
                    let pk = p.pow(k);
                    let rem_entry = entry - pk;

                    let entry_deg = combinatorics::xi_degrees(p)[len - 1] * self.q();

                    let mut elt = MilnorBasisElement {
                        q_part: 0,
                        degree: 0,
                        p_part: PPart::zero(),
                    };

                    elt.p_part.set(len - 1, pk);
                    elt.degree = entry_deg * pk as i32;
                    let first = (elt.degree, self.basis_element_to_index(&elt));

                    elt.p_part.set(len - 1, rem_entry);
                    elt.degree = entry_deg * rem_entry as i32;
                    let second = (elt.degree, self.basis_element_to_index(&elt));

                    let coef =
                        p - fp::prime::inverse(p, PPartEntry::binomial(p, pk + rem_entry, pk));
                    buffer.extend([(coef, first, second)])
                }
            } else {
                // There is more than one entry. Just separate out the last entry.
                let last_entry = b.p_part.get(len - 1);
                let last_deg = combinatorics::xi_degrees(p)[len - 1] * self.q() * last_entry as i32;
                let mut elt = MilnorBasisElement {
                    q_part: 0,
                    p_part: PPart::zero(),
                    degree: last_deg,
                };
                elt.p_part.set(len - 1, last_entry);
                let first = (elt.degree, self.basis_element_to_index(&elt));

                elt.degree = degree - last_deg;
                elt.p_part = b.p_part.truncate(len - 1);
                let second = (elt.degree, self.basis_element_to_index(&elt));
                buffer.extend([(p - c, first, second)]);
            };
            for (c, first, second) in &buffer {
                self.multiply_basis_elements(
                    out_vec.as_slice_mut(),
                    *c,
                    first.0,
                    first.1,
                    second.0,
                    second.1,
                );
            }
            result.extend(&buffer);
            buffer.clear();
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _; // Needed for write! macro for String

    use fp::{prime::ValidPrime, vector::FpVector};
    use rstest::rstest;

    use super::*;
    use crate::algebra::milnor_algebra::MilnorProfile;

    #[rstest]
    #[trace]
    #[case(2, 32, None)]
    #[case(2, 32, Some(MilnorProfile { q_part: !0, p_part: vec!(3, 2, 1), truncated: true }))]
    #[case(2, 32, Some(MilnorProfile { q_part: !0, p_part: vec!(2, 2, 1), truncated: true }))]
    #[case(2, 32, Some(MilnorProfile { q_part: !0, p_part: vec!(0), truncated: false }))]
    #[case(3, 106, None)]
    #[case(3, 106, Some(MilnorProfile { q_part: 0b1111, p_part: vec!(3, 2, 1), truncated: true }))]
    #[case(3, 106, Some(MilnorProfile { q_part: 0b1111, p_part: vec!(2, 2, 1), truncated: true }))]
    fn test_milnor_decompose(
        #[case] p: u32,
        #[case] max_degree: i32,
        #[case] profile: Option<MilnorProfile>,
    ) {
        let p = ValidPrime::new(p);
        let algebra = MilnorAlgebra::new_with_profile(p, profile.unwrap_or_default(), false);
        algebra.compute_basis(max_degree);
        for i in 1..max_degree {
            let dim = algebra.dimension(i);
            let gens = algebra.generators(i);
            // println!("i : {}, gens : {:?}", i, gens);
            let mut out_vec = FpVector::new(p, dim);
            for j in 0..dim {
                if gens.contains(&j) {
                    continue;
                }
                for (coeff, (first_degree, first_idx), (second_degree, second_idx)) in
                    algebra.decompose_basis_element(i, j)
                {
                    // print!("{} * {} * {}  +  ", coeff, algebra.basis_element_to_string(first_degree,first_idx), algebra.basis_element_to_string(second_degree, second_idx));
                    algebra.multiply_basis_elements(
                        out_vec.as_slice_mut(),
                        coeff,
                        first_degree,
                        first_idx,
                        second_degree,
                        second_idx,
                    );
                }
                assert!(
                    out_vec.entry(j) == 1,
                    "{} != {}",
                    algebra.basis_element_to_string(i, j),
                    algebra.element_to_string(i, out_vec.as_slice())
                );
                out_vec.set_entry(j, 0);
                assert!(
                    out_vec.is_zero(),
                    "\n{} != {}",
                    algebra.basis_element_to_string(i, j),
                    algebra.element_to_string(i, out_vec.as_slice())
                );
            }
        }
    }

    use crate::module::ModuleFailedRelationError;
    #[rstest(p, max_degree, case(2, 32), case(3, 106))]
    #[trace]
    fn test_adem_relations(p: u32, max_degree: i32) {
        let p = ValidPrime::new(p);
        let algebra = MilnorAlgebra::new(p, false);
        algebra.compute_basis(max_degree + 2);
        let mut output_vec = FpVector::new(p, 0);
        for i in 1..max_degree {
            let output_dim = algebra.dimension(i);
            output_vec.set_scratch_vector_size(output_dim);
            let relations = algebra.generating_relations(i);
            println!("{relations:?}");
            for relation in relations {
                for (coeff, (deg_1, idx_1), (deg_2, idx_2)) in &relation {
                    algebra.multiply_basis_elements(
                        output_vec.as_slice_mut(),
                        *coeff,
                        *deg_1,
                        *idx_1,
                        *deg_2,
                        *idx_2,
                    );
                }
                if !output_vec.is_zero() {
                    let mut relation_string = String::new();
                    for (coeff, (deg_1, idx_1), (deg_2, idx_2)) in &relation {
                        let _ = write!(
                            relation_string,
                            "{} * {} * {}  +  ",
                            coeff,
                            algebra.basis_element_to_string(*deg_1, *idx_1),
                            algebra.basis_element_to_string(*deg_2, *idx_2)
                        );
                    }
                    relation_string.pop();
                    relation_string.pop();
                    relation_string.pop();
                    relation_string.pop();
                    relation_string.pop();
                    let value_string = algebra.element_to_string(i, output_vec.as_slice());
                    panic!(
                        "{}",
                        ModuleFailedRelationError {
                            relation: relation_string,
                            value: value_string
                        }
                    );
                }
            }
        }
    }
}
