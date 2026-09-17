use fp::prime::{Prime, ValidPrime};
#[cfg(feature = "cache-multiplication")]
use fp::vector::FpVector;
use once::OnceVec;
use rustc_hash::FxHashMap as HashMap;

use crate::algebra::{Algebra, combinatorics};

mod algebra_impl;
mod basis_element;
mod bialgebra_impl;
mod generated_impl;
mod multiplication;
mod ppart;
mod profile;

pub use basis_element::MilnorBasisElement;
pub use multiplication::{PPartAllocation, PPartMultiplier, next_disjoint};
pub use ppart::{PPart, PPartEntry};
pub use profile::MilnorProfile;

pub struct MilnorAlgebra {
    profile: MilnorProfile,
    p: ValidPrime,
    #[cfg(feature = "odd-primes")]
    generic: bool,

    unstable_enabled: bool,

    /// This is a list of possible P(R) of each degree, where `ppart_table[i]` contains elements of
    /// degree `q * i`.
    ppart_table: OnceVec<Vec<PPart>>,

    /// A list of all basis elements of each degree, constructed from [`Self::ppart_table`].
    ///
    /// Only populated when [`Self::stores_basis_table`] holds. At `p = 2` with unstable support
    /// off, the basis element at index `i` of degree `t` is exactly
    /// `MilnorBasisElement::from_p(ppart_table[t][i], t)`, so storing it is redundant.
    basis_table: OnceVec<Vec<MilnorBasisElement>>,

    excess_table: OnceVec<Vec<usize>>,

    /// degree -> MilnorBasisElement -> index
    basis_element_to_index_map: OnceVec<HashMap<MilnorBasisElement, usize>>,

    #[cfg(feature = "cache-multiplication")]
    /// source_deg -> target_deg -> source_op -> target_op
    multiplication_table: OnceVec<OnceVec<Vec<Vec<FpVector>>>>,
}

impl std::fmt::Display for MilnorAlgebra {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "MilnorAlgebra(p={})", self.prime())
    }
}

impl MilnorAlgebra {
    pub fn new(p: ValidPrime, unstable_enabled: bool) -> Self {
        Self::new_with_profile(p, MilnorProfile::default(), unstable_enabled)
    }

    pub fn new_with_profile(p: ValidPrime, profile: MilnorProfile, unstable_enabled: bool) -> Self {
        assert!(profile.is_valid());
        Self {
            p,
            #[cfg(feature = "odd-primes")]
            generic: p != 2,
            unstable_enabled,
            profile,
            ppart_table: OnceVec::new(),
            basis_table: OnceVec::new(),
            excess_table: OnceVec::new(),
            basis_element_to_index_map: OnceVec::new(),
            #[cfg(feature = "cache-multiplication")]
            multiplication_table: OnceVec::new(),
        }
    }

    #[inline]
    pub fn generic(&self) -> bool {
        #[cfg(feature = "odd-primes")]
        {
            self.generic
        }

        #[cfg(not(feature = "odd-primes"))]
        {
            false
        }
    }

    pub fn q(&self) -> i32 {
        if self.generic() {
            2 * (self.prime().as_i32() - 1)
        } else {
            1
        }
    }

    pub fn profile(&self) -> &MilnorProfile {
        &self.profile
    }

    /// Whether the basis of each degree has to be stored rather than derived.
    ///
    /// At odd primes the q-part varies within a degree, and with unstable support enabled the
    /// basis is re-sorted by excess; in both cases the basis is not a re-wrapping of
    /// [`Self::ppart_table`] and must be kept.
    fn stores_basis_table(&self) -> bool {
        self.generic() || self.unstable_enabled
    }

    pub fn basis_element_from_index(&self, degree: i32, idx: usize) -> MilnorBasisElement {
        if self.stores_basis_table() {
            self.basis_table[degree as usize][idx]
        } else {
            MilnorBasisElement::from_p(self.ppart_table[degree as usize][idx], degree)
        }
    }

    pub fn try_basis_element_to_index(&self, elt: &MilnorBasisElement) -> Option<usize> {
        self.basis_element_to_index_map[elt.degree as usize]
            .get(elt)
            .copied()
    }

    pub fn basis_element_to_index(&self, elt: &MilnorBasisElement) -> usize {
        self.try_basis_element_to_index(elt)
            .unwrap_or_else(|| panic!("Didn't find element: {elt:?}"))
    }

    /// Gives a list of PPart's in degree `t`.
    pub fn ppart_table(&self, t: i32) -> &[PPart] {
        &self.ppart_table[t as usize]
    }
}

// Compute basis functions
impl MilnorAlgebra {
    fn compute_ppart(&self, max_degree: i32) {
        self.ppart_table.extend(0, |_| vec![PPart::zero()]);

        let p = self.prime().as_i32();
        let q = if p == 2 { 1 } else { 2 * p - 2 };
        let new_deg = max_degree / q;

        let xi_degrees = combinatorics::xi_degrees(self.prime());
        let mut profile_list = Vec::with_capacity(xi_degrees.len());
        for i in 0..xi_degrees.len() {
            if i < self.profile.p_part.len() {
                profile_list.push(self.prime().pow(self.profile.p_part[i]) - 1);
            } else if self.profile.truncated {
                profile_list.push(0);
            } else {
                profile_list.push(PPartEntry::MAX);
            }
        }

        self.ppart_table.extend(new_deg as usize, |d| {
            let d = d as i32;
            let mut new_row = Vec::new(); // Improve this
            for i in 0..xi_degrees.len() {
                if xi_degrees[i] > d {
                    break;
                }
                if profile_list[i] == 0 {
                    continue;
                }

                let rem = (d - xi_degrees[i]) as usize;
                for &old in &self.ppart_table[rem] {
                    // ppart_table[rem] is arranged in increasing order of highest
                    // xi_i. If we get something too large, we may abort;
                    if old.len() > i + 1 {
                        break;
                    }
                    if old.get(i) == profile_list[i] {
                        continue;
                    }
                    let mut new = old;
                    new.set(i, old.get(i) + 1);
                    new_row.push(new);
                }
            }
            new_row
        });
    }

    fn generate_basis_generic(&self, max_degree: i32) {
        let q = 2 * self.prime() - 2;
        let tau_degrees = combinatorics::tau_degrees(self.prime());

        self.basis_table.extend(max_degree as usize, |d| {
            let mut table = Vec::new();
            let residue = d as u32 % q;

            for q_part in 0u32.. {
                if q_part.count_ones() % q != residue {
                    continue;
                }

                let mut q_degree = 0;
                let mut bs = q_part;
                for &entry in tau_degrees {
                    q_degree += entry * (bs & 1) as i32;
                    bs >>= 1;
                    if bs == 0 {
                        break;
                    }
                }

                if q_degree > d as i32 {
                    break;
                }

                if q_part & !self.profile.q_part != 0 {
                    continue;
                }

                table.extend(
                    self.ppart_table[(d - q_degree as usize) / q as usize]
                        .iter()
                        .map(|&p_part| MilnorBasisElement {
                            p_part,
                            q_part,
                            degree: d as i32,
                        }),
                );
            }
            if self.unstable_enabled {
                table.sort_by_cached_key(|e| e.excess(self.p));
            }
            table
        });
    }

    fn generate_basis_2(&self, max_degree: i32) {
        if !self.stores_basis_table() {
            // Derived on demand from `ppart_table`; see the field docs.
            return;
        }
        self.basis_table.extend(max_degree as usize, |d| {
            let mut table: Vec<_> = self.ppart_table[d]
                .iter()
                .map(|&p| MilnorBasisElement::from_p(p, d as i32))
                .collect();
            table.sort_by_cached_key(|e| e.excess(fp::prime::TWO));
            table
        });
    }

    fn generate_excess_table(&self, max_degree: i32) {
        let p = self.prime();
        self.excess_table.extend(max_degree as usize, |n| {
            let mut new_entry = Vec::with_capacity(n);
            let mut cur_excess = 0;
            for (i, elt) in self.basis_table[n].iter().enumerate() {
                let excess = elt.excess(p);
                for _ in cur_excess..excess {
                    new_entry.push(i);
                }
                cur_excess = excess;
            }
            let dim = self.dimension(n as i32);
            for _ in cur_excess..n as u32 {
                new_entry.push(dim);
            }
            new_entry
        });
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::algebra::GeneratedAlgebra;

    #[rstest]
    #[trace]
    #[case(2, 32)]
    #[case(3, 106)]
    fn test_milnor_string(#[case] p: u32, #[case] max_degree: i32) {
        let p = ValidPrime::new(p);
        let algebra = MilnorAlgebra::new(p, false);
        algebra.compute_basis(max_degree);
        for t in 0..max_degree {
            for i in 0..algebra.dimension(t) {
                let elt = algebra.basis_element_to_string(t, i);
                assert_eq!(
                    Some((t, i)),
                    algebra.basis_element_from_string(&elt),
                    "Error parsing {elt}"
                );
            }
            for i in algebra.generators(t) {
                let elt = algebra.generator_to_string(t, i);
                assert_eq!(
                    Some((t, i)),
                    algebra.basis_element_from_string(&elt),
                    "Error parsing {elt}"
                );
            }
        }
    }

    /// Pack every basis element the algebra actually produces and check nothing collides or is
    /// lost. This is the property the whole representation rests on.
    #[rstest]
    #[case(2, 120)]
    #[case(3, 200)]
    fn ppart_packing_is_faithful(#[case] p: u32, #[case] max_degree: i32) {
        let algebra = MilnorAlgebra::new(ValidPrime::new(p), false);
        algebra.compute_basis(max_degree);

        for t in 0..=max_degree {
            let mut seen = HashMap::default();
            for i in 0..algebra.dimension(t) {
                let elt = algebra.basis_element_from_index(t, i);
                // The packed value plus the q-part identifies the element within its degree.
                assert!(
                    seen.insert((elt.p_part.bits(), elt.q_part), i).is_none(),
                    "collision at degree {t} for {elt}"
                );
                // Round-trip through a slice, and back through the index map.
                assert_eq!(
                    PPart::from_slice(&elt.p_part.iter().collect::<Vec<_>>()),
                    elt.p_part
                );
                assert_eq!(algebra.basis_element_to_index(&elt), i);
                // The degree really is recoverable from the entries.
                let mut recomputed = elt;
                recomputed.compute_degree(ValidPrime::new(p));
                assert_eq!(recomputed.degree, t);
            }
        }
    }

    /// The basis *order* at `p = 2` is a wire format: saved resolutions store coefficients by
    /// index, so reordering silently invalidates them without `magic()` changing. Deriving the
    /// basis from `ppart_table` preserves the order `generate_basis_2` produced, since the stable
    /// path never sorted. Pin that down against fixed expected names, so the check does not depend
    /// on `ppart_table` -- the very thing it is guarding.
    #[test]
    fn basis_order_at_p2_is_stable() {
        let algebra = MilnorAlgebra::new(fp::prime::TWO, false);
        algebra.compute_basis(8);

        let expected: [&[&str]; 9] = [
            &["1"],
            &["P(1)"],
            &["P(2)"],
            &["P(3)", "P(0, 1)"],
            &["P(4)", "P(1, 1)"],
            &["P(5)", "P(2, 1)"],
            &["P(6)", "P(3, 1)", "P(0, 2)"],
            &["P(7)", "P(4, 1)", "P(1, 2)", "P(0, 0, 1)"],
            &["P(8)", "P(5, 1)", "P(2, 2)", "P(1, 0, 1)"],
        ];
        for (t, names) in expected.iter().enumerate() {
            let t = t as i32;
            assert_eq!(algebra.dimension(t), names.len(), "dimension in degree {t}");
            for (i, name) in names.iter().enumerate() {
                assert_eq!(
                    &algebra.basis_element_to_string(t, i),
                    name,
                    "degree {t}, index {i}"
                );
            }
        }
    }

    /// At `p = 2` with unstable support off, the basis is not stored: it is derived from
    /// `ppart_table`. Check the derivation reproduces exactly what the table used to hold, so the
    /// redundancy this relies on is asserted rather than assumed.
    #[test]
    fn basis_is_derived_at_p2() {
        let p = fp::prime::TWO;
        let algebra = MilnorAlgebra::new(p, false);
        algebra.compute_basis(120);
        assert!(
            !algebra.stores_basis_table(),
            "p = 2 stable should not be storing the basis"
        );

        for t in 0..=120 {
            let pparts = algebra.ppart_table(t);
            assert_eq!(algebra.dimension(t), pparts.len());
            for (i, &p_part) in pparts.iter().enumerate() {
                // This is precisely what `generate_basis_2` used to store.
                let expected = MilnorBasisElement {
                    q_part: 0,
                    p_part,
                    degree: t,
                };
                let actual = algebra.basis_element_from_index(t, i);
                assert_eq!(actual.p_part, expected.p_part, "degree {t}, index {i}");
                assert_eq!(actual.q_part, expected.q_part, "degree {t}, index {i}");
                assert_eq!(actual.degree, expected.degree, "degree {t}, index {i}");
            }
        }
    }

    /// The two configurations that still need the table really do differ from `ppart_table`, so
    /// the exemption in `stores_basis_table` is not over-broad.
    #[rstest]
    #[case(3, false)]
    #[case(2, true)]
    fn basis_is_stored_when_it_must_be(#[case] p: u32, #[case] unstable: bool) {
        let algebra = MilnorAlgebra::new(ValidPrime::new(p), unstable);
        algebra.compute_basis(60);
        assert!(algebra.stores_basis_table());
        // Every stored element still round-trips through the index map.
        for t in 0..=60 {
            for i in 0..algebra.dimension(t) {
                let elt = algebra.basis_element_from_index(t, i);
                assert_eq!(algebra.basis_element_to_index(&elt), i);
            }
        }
    }
}
