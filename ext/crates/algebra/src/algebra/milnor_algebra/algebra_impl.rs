use fp::{
    prime::{Prime, ValidPrime},
    vector::{FpSlice, FpSliceMut},
};
use rustc_hash::FxHashMap as HashMap;

use super::{MilnorAlgebra, MilnorBasisElement, PPart, PPartAllocation};
use crate::algebra::{Algebra, UnstableAlgebra, combinatorics};

impl Algebra for MilnorAlgebra {
    fn prefix(&self) -> &str {
        "milnor"
    }

    fn magic(&self) -> u32 {
        (self.p << 16)
            + if self.profile.is_trivial() {
                0x8000
            } else {
                0x8001
            }
    }

    fn prime(&self) -> ValidPrime {
        self.p
    }

    fn default_filtration_one_products(&self) -> Vec<(String, i32, usize)> {
        let mut products = Vec::with_capacity(4);
        let max_degree = if self.generic() {
            if self.profile.q_part & 1 != 0 {
                products.push((
                    "a_0".to_string(),
                    MilnorBasisElement {
                        degree: 1,
                        q_part: 1,
                        p_part: PPart::zero(),
                    },
                ));
            }
            if (self.profile.p_part.is_empty() && !self.profile.truncated)
                || (!self.profile.p_part.is_empty() && self.profile.p_part[0] > 0)
            {
                products.push((
                    "h_0".to_string(),
                    MilnorBasisElement {
                        degree: (2 * self.prime() - 2) as i32,
                        q_part: 0,
                        p_part: PPart::from_iter([1]),
                    },
                ));
            }
            (2 * self.prime() - 2) as i32
        } else {
            let mut max = 4;
            if !self.profile.p_part.is_empty() {
                max = std::cmp::min(4, self.profile.p_part[0]);
            } else if self.profile.truncated {
                max = 0;
            }
            for i in 0..max {
                let degree = 1 << i; // degree is 2^hi
                products.push((
                    format!("h_{i}"),
                    MilnorBasisElement {
                        degree,
                        q_part: 0,
                        p_part: PPart::from_iter([1 << i]),
                    },
                ));
            }
            1 << 3
        };
        self.compute_basis(max_degree + 1);

        products
            .into_iter()
            .map(|(name, b)| (name, b.degree, self.basis_element_to_index(&b)))
            .collect()
    }

    fn compute_basis(&self, max_degree: i32) {
        // This is the single gate that makes [`PPart`]'s packing safe: past this degree an
        // exponent could outgrow its field. Everything downstream may then assume entries fit.
        assert!(
            max_degree <= PPart::MAX_DEGREE,
            "Milnor basis elements are only supported up to degree {}, got {max_degree}",
            PPart::MAX_DEGREE,
        );
        self.compute_ppart(max_degree);

        if self.generic() {
            self.generate_basis_generic(max_degree);
        } else {
            self.generate_basis_2(max_degree);
        }

        // Populate hash map
        self.basis_element_to_index_map
            .extend(max_degree as usize, |d| {
                let mut map = HashMap::default();
                let dim = self.dimension(d as i32);
                map.reserve(dim);
                for i in 0..dim {
                    let b = self.basis_element_from_index(d as i32, i);
                    assert!(map.insert(b, i).is_none(), "Duplicate entry for {b}");
                }
                map
            });

        #[cfg(feature = "cache-multiplication")]
        {
            use fp::vector::FpVector;
            use once::OnceVec;

            self.multiplication_table
                .extend(max_degree as usize, |_| OnceVec::new());

            for d in 0..=max_degree as usize {
                self.multiplication_table[d].extend(max_degree as usize - d, |e| {
                    (0..self.dimension(d as i32))
                        .map(|i| {
                            (0..self.dimension(e as i32))
                                .map(|j| {
                                    let mut res =
                                        FpVector::new(self.prime(), self.dimension((d + e) as i32));
                                    self.multiply(
                                        res.as_slice_mut(),
                                        1,
                                        self.basis_element_from_index(d as i32, i),
                                        self.basis_element_from_index(e as i32, j),
                                    );
                                    res
                                })
                                .collect::<Vec<_>>()
                        })
                        .collect::<Vec<_>>()
                });
            }
        }

        if self.unstable_enabled {
            self.generate_excess_table(max_degree);
        }
    }

    fn dimension(&self, degree: i32) -> usize {
        if degree < 0 {
            return 0;
        }
        if self.stores_basis_table() {
            self.basis_table[degree as usize].len()
        } else {
            self.ppart_table[degree as usize].len()
        }
    }

    #[cfg(not(feature = "cache-multiplication"))]
    fn multiply_basis_elements(
        &self,
        result: FpSliceMut,
        coef: u32,
        r_degree: i32,
        r_idx: usize,
        s_degree: i32,
        s_idx: usize,
    ) {
        self.multiply(
            result,
            coef,
            self.basis_element_from_index(r_degree, r_idx),
            self.basis_element_from_index(s_degree, s_idx),
        );
    }

    #[cfg(feature = "cache-multiplication")]
    fn multiply_basis_elements(
        &self,
        mut result: FpSliceMut,
        coef: u32,
        r_degree: i32,
        r_idx: usize,
        s_degree: i32,
        s_idx: usize,
    ) {
        result.add(
            self.multiplication_table[r_degree as usize][s_degree as usize][r_idx][s_idx]
                .as_slice(),
            coef,
        );
    }

    fn multiply_basis_element_by_element(
        &self,
        mut result: FpSliceMut,
        coeff: u32,
        r_degree: i32,
        r_idx: usize,
        s_degree: i32,
        s: FpSlice,
    ) {
        let p = self.prime();
        let r = self.basis_element_from_index(r_degree, r_idx);
        PPartAllocation::with_local(|mut allocation| {
            for (i, v) in s.iter_nonzero() {
                allocation = self.multiply_with_allocation(
                    result.copy(),
                    (coeff * v) % p,
                    r,
                    self.basis_element_from_index(s_degree, i),
                    i32::MAX,
                    allocation,
                );
            }
            allocation
        });
    }

    fn multiply_element_by_element(
        &self,
        mut res: FpSliceMut,
        coef: u32,
        r_deg: i32,
        r: FpSlice,
        s_deg: i32,
        s: FpSlice,
    ) {
        PPartAllocation::with_local(|mut allocation| {
            for (i, c) in r.iter_nonzero() {
                allocation = self.multiply_basis_by_element_with_allocation(
                    res.copy(),
                    coef * c,
                    self.basis_element_from_index(r_deg, i),
                    s_deg,
                    s,
                    allocation,
                );
            }
            allocation
        })
    }

    fn basis_element_to_string(&self, degree: i32, idx: usize) -> String {
        format!("{}", self.basis_element_from_index(degree, idx))
    }

    fn basis_element_from_string(&self, elt: &str) -> Option<(i32, usize)> {
        use nom::{
            Parser,
            branch::alt,
            bytes::complete::tag,
            character::complete::char,
            combinator::{map, opt},
            multi::{many0, separated_list1},
            sequence::preceded,
        };

        use crate::steenrod_parser::{brackets, digits, p_or_sq};

        let p = self.prime();

        let mut parser = alt((
            map(char('1'), |_| Some((0, 0))),
            map(char('b'), |_| Some((1, 0))),
            map(preceded(p_or_sq, digits), |i| self.try_beps_pn(0, i)),
            map(
                (tag("P^"), digits, char('_'), digits::<usize>),
                |(_, s, _, t)| {
                    let entry = p.pow(s);
                    let degree = entry as i32 * self.q() * combinatorics::xi_degrees(p)[t];
                    // Packing the entry and computing the basis both assert their range, where an
                    // unpacked p-part simply stored the value.
                    if degree > PPart::MAX_DEGREE || entry > PPart::max_entry(t - 1) {
                        return None;
                    }
                    let mut p_part = PPart::zero();
                    p_part.set(t - 1, entry);
                    let elt = MilnorBasisElement {
                        degree,
                        q_part: 0,
                        p_part,
                    };
                    self.compute_basis(degree);
                    self.try_basis_element_to_index(&elt)
                        .map(|idx| (degree, idx))
                },
            ),
            map(
                (
                    many0(preceded(tag("Q_"), digits::<u32>)),
                    opt(preceded(
                        char('P'),
                        brackets(separated_list1(char(','), digits)),
                    )),
                ),
                |(q_list, p_list)| {
                    let q_part = q_list.into_iter().fold(0, |acc, q| acc + (1 << q));
                    let p_part = PPart::try_from_slice(&p_list.unwrap_or_default())?;
                    let mut elt = MilnorBasisElement {
                        degree: 0,
                        q_part,
                        p_part,
                    };
                    elt.compute_degree(p);
                    if elt.degree > PPart::MAX_DEGREE {
                        return None;
                    }
                    self.compute_basis(elt.degree);

                    self.try_basis_element_to_index(&elt)
                        .map(|idx| (elt.degree, idx))
                },
            ),
        ));

        if let Ok(("", res)) = parser.parse(elt) {
            res
        } else {
            None
        }
    }
}

impl UnstableAlgebra for MilnorAlgebra {
    fn dimension_unstable(&self, degree: i32, excess: i32) -> usize {
        if degree < 0 || excess < 0 {
            0
        } else if excess < degree {
            self.excess_table[degree as usize][excess as usize]
        } else {
            self.dimension(degree)
        }
    }

    fn multiply_basis_elements_unstable(
        &self,
        result: FpSliceMut,
        coeff: u32,
        r_degree: i32,
        r_index: usize,
        s_degree: i32,
        s_index: usize,
        excess: i32,
    ) {
        let m1 = self.basis_element_from_index(r_degree, r_index);
        let m2 = self.basis_element_from_index(s_degree, s_index);
        PPartAllocation::with_local(|allocation| {
            self.multiply_with_allocation(result, coeff, m1, m2, excess, allocation)
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algebra::milnor_algebra::MilnorProfile;

    #[test]
    fn basis_element_from_string_total_milnor() {
        let p = ValidPrime::new(2);
        let algebra = MilnorAlgebra::new(p, false);
        algebra.compute_basis(8);

        // Sanity: valid names round-trip through the canonical string form.
        for name in ["P(1)", "P(2)", "P(0, 1)"] {
            let (d, i) = algebra
                .basis_element_from_string(name)
                .unwrap_or_else(|| panic!("expected Some for {name}"));
            assert_eq!(algebra.basis_element_to_string(d, i), name);
        }

        // "P0"/"Sq0" name the identity. A packed p-part does not represent trailing zeros, so
        // `P(0)` and `P()` are the same value, and `try_beps_pn(0, 0)` finds the degree-0 basis
        // element. This matches `AdemAlgebra::try_beps_pn`, which special-cases `x == 0` to
        // `Some((0, 0))`; the previous `None` here came from `vec![0]` and `vec![]` hashing
        // differently, which was an artifact of the unpacked representation.
        assert_eq!(algebra.basis_element_from_string("P0"), Some((0, 0)));
        assert_eq!(algebra.basis_element_from_string("Sq0"), Some((0, 0)));

        // Syntactically-valid names that name no basis element must still return `None`
        // (they previously panicked in `basis_element_to_index`).
        //
        // "Q_5" parses via the Q/P branch into a candidate element (degree 63)
        // whose basis lookup finds nothing at p = 2.
        assert_eq!(algebra.basis_element_from_string("Q_5"), None);

        // On A(2) (profile [3, 2, 1], truncated) the first xi exponent is bounded
        // by 2^3 - 1 = 7. "P7" exists; the out-of-profile "P8" parses to a valid
        // candidate that is excluded by the profile, so it must return `None`.
        let a2 = MilnorAlgebra::new_with_profile(
            p,
            MilnorProfile {
                q_part: !0,
                p_part: vec![3, 2, 1],
                truncated: true,
            },
            false,
        );
        a2.compute_basis(16);
        assert!(a2.basis_element_from_string("P7").is_some());
        assert_eq!(a2.basis_element_from_string("P8"), None);
    }
}
