//! Finite sub-Hopf-algebras of a Milnor algebra.

use fp::prime::{Prime, iter::BitflagIterator};
use itertools::Itertools;

use super::{
    Exterior, MilnorAlgebra, MilnorAlgebraInner, MilnorBasisElement, MilnorProfile, NoExterior,
    PPart, PPartEntry,
};
use crate::algebra::{Algebra, combinatorics};

/// A finite sub-Hopf-algebra of a Milnor algebra, given by a profile.
///
/// It is a [`MilnorAlgebra`] of the same shape and prime as the algebras it sits in, and is
/// guaranteed to be finite, so its degrees and dimension are bounded.
pub struct MilnorSubalgebra(MilnorAlgebra);

impl MilnorSubalgebra {
    /// The algebra of `ambient`'s shape and prime with the given profile, or `None` if that is not
    /// a finite sub-Hopf-algebra whose top degree fits in an `i32`.
    ///
    /// At the exterior shape, a q-part with every bit set, the default, stands for all the $Q_k$.
    /// This does not check that the result lies in `ambient`; see [`Self::is_subalgebra_of`].
    pub fn new(ambient: &MilnorAlgebra, profile: MilnorProfile) -> Option<Self> {
        let p = ambient.prime();
        let finite = profile.truncated
            && profile.is_valid()
            && profile.p_part.len() <= combinatorics::MAX_XI
            && !(ambient.has_exterior() && profile.q_part == !0)
            // A saturated entry stands for a degree past `i32::MAX`.
            && !(ambient.has_exterior()
                && BitflagIterator::set_bit_iterator(profile.q_part as u64)
                    .any(|k| combinatorics::tau_degrees(p)[k] == i32::MAX))
            && profile
                .p_part
                .iter()
                .all(|&e| p.as_u32().checked_pow(e).is_some());
        if !finite {
            return None;
        }
        let algebra: MilnorAlgebra = match ambient {
            MilnorAlgebra::Polynomial(_) => {
                MilnorAlgebraInner::<NoExterior>::new_with_profile(p, profile, false).into()
            }
            MilnorAlgebra::Exterior(_) => {
                MilnorAlgebraInner::<Exterior>::new_with_profile(p, profile, false).into()
            }
        };
        let subalgebra = Self(algebra);
        i32::try_from(subalgebra.top_degree_i64()).ok()?;
        Some(subalgebra)
    }

    /// The trivial subalgebra of `ambient`, the ground field.
    pub fn trivial(ambient: &MilnorAlgebra) -> Self {
        let profile = MilnorProfile {
            truncated: true,
            q_part: 0,
            p_part: vec![],
        };
        Self::new(ambient, profile).unwrap()
    }

    /// This subalgebra as an algebra in its own right.
    pub fn algebra(&self) -> &MilnorAlgebra {
        &self.0
    }

    /// The profile defining this subalgebra.
    pub fn profile(&self) -> &MilnorProfile {
        self.0.profile()
    }

    /// The $Q_k$ in this algebra, as a mask of the q-part. Only the exterior shape has any.
    fn exterior_part(&self) -> u32 {
        if self.0.has_exterior() {
            self.profile().q_part
        } else {
            0
        }
    }

    /// `p^{e_i}` for the entries `e_i` of the profile, which bound the entries of the p-part.
    fn moduli(&self) -> impl Iterator<Item = PPartEntry> + '_ {
        let p = self.0.prime().as_u32();
        self.profile().p_part.iter().map(move |&e| p.pow(e))
    }

    /// See [`Self::top_degree`].
    fn top_degree_i64(&self) -> i64 {
        let p = self.0.prime();
        let tau_degrees = combinatorics::tau_degrees(p);
        let xi_degrees = combinatorics::xi_degrees(p);
        let exterior: i64 = BitflagIterator::set_bit_iterator(self.exterior_part() as u64)
            .map(|k| tau_degrees[k] as i64)
            .sum();
        let polynomial: i64 = std::iter::zip(self.moduli(), xi_degrees)
            .map(|(m, &d)| (m as i64 - 1) * d as i64)
            .sum();
        exterior + self.0.q() as i64 * polynomial
    }

    /// The degree of the top basis element. This is a Poincaré duality algebra, so it is the only
    /// basis element in its degree.
    pub fn top_degree(&self) -> i32 {
        self.top_degree_i64() as i32
    }

    /// The dimension of the algebra, saturating at `u64::MAX`.
    pub fn dimension(&self) -> u64 {
        self.moduli()
            .fold(1u64 << self.exterior_part().count_ones(), |acc, m| {
                acc.saturating_mul(m as u64)
            })
    }

    /// Whether this is a sub-Hopf-algebra of `ambient`, which requires the same prime and shape.
    pub fn is_subalgebra_of(&self, ambient: &MilnorAlgebra) -> bool {
        let ours = self.profile();
        let theirs = ambient.profile();
        let their_exterior = if ambient.has_exterior() {
            theirs.q_part
        } else {
            0
        };
        // Comparing one entry past the longer profile compares the two tails.
        let len = std::cmp::max(ours.p_part.len(), theirs.p_part.len());
        self.0.prime() == ambient.prime()
            && self.0.has_exterior() == ambient.has_exterior()
            && self.exterior_part() & !their_exterior == 0
            && (0..=len).all(|i| ours.get_p_part(i) <= theirs.get_p_part(i))
    }

    /// The component in this algebra of `elt`, a basis element of an algebra containing it.
    ///
    /// The component of $Q(E)P(R)$ is $Q(E \cap E')P(R \bmod p^e)$, where $E'$ and $e$ are this
    /// algebra's q-part and profile. Grouping the basis of the larger algebra by component splits
    /// it into translates of the elements whose component is $1$, one per basis element of this
    /// algebra, which is the freeness of the larger algebra over this one.
    pub fn component(&self, elt: &MilnorBasisElement) -> MilnorBasisElement {
        let mut component = MilnorBasisElement {
            q_part: elt.q_part & self.exterior_part(),
            p_part: PPart::zero(),
            degree: 0,
        };
        for (i, m) in self.moduli().enumerate() {
            component.p_part.set(i, elt.p_part.get(i) % m);
        }
        self.0.compute_degree(&mut component);
        component
    }

    /// Whether the component of `elt` is `component`, without building it.
    pub fn has_component(&self, elt: &MilnorBasisElement, component: &MilnorBasisElement) -> bool {
        elt.q_part & self.exterior_part() == component.q_part
            && self
                .moduli()
                .enumerate()
                .all(|(i, m)| elt.p_part.get(i) % m == component.p_part.get(i))
    }

    /// A mask that compiles [`Self::has_component`] to `elt.p_part.bits() & mask ==
    /// component.p_part.bits()`. It exists when the basis has no exterior part and every modulus
    /// is a power of two, i.e. at the polynomial shape at `p = 2`.
    ///
    /// Each entry occupies a fixed field of the packed word, so its residue mod $2^e$ is the low
    /// $e$ bits of that field.
    pub fn packed_component_mask(&self) -> Option<u64> {
        if self.0.has_exterior() || self.0.prime() != 2 {
            return None;
        }
        Some(
            self.profile()
                .p_part
                .iter()
                .enumerate()
                .fold(0, |mask, (i, &e)| {
                    // A profile wider than the field constrains the whole field.
                    let width = std::cmp::min(e, PPart::width(i));
                    mask | ((1u64 << width) - 1) << PPart::shift(i)
                }),
        )
    }
}

/// `A(n)`, `E(Q_0..Q_n)` or `F_p` where it is one of those, and the profile otherwise.
impl std::fmt::Display for MilnorSubalgebra {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let MilnorProfile { q_part, p_part, .. } = self.profile();
        let len = p_part.len();
        let is_an = p_part.iter().rev().copied().eq(1..=len as PPartEntry);
        if self.0.has_exterior() {
            let top_q = (u32::BITS - q_part.leading_zeros()) as usize;
            let q_is_initial = q_part.count_ones() as usize == top_q;
            if *q_part == 0 && len == 0 {
                write!(out, "F_{}", self.0.prime())
            } else if is_an && q_is_initial && top_q == len + 1 {
                write!(out, "A({len})")
            } else if len == 0 && q_is_initial {
                write!(out, "E(Q_0..Q_{})", top_q - 1)
            } else {
                write!(out, "B({q_part:#b}; {})", p_part.iter().join(","))
            }
        } else if len == 0 {
            write!(out, "F_{}", self.0.prime())
        } else if is_an {
            write!(out, "A({})", len - 1)
        } else {
            write!(out, "B({})", p_part.iter().join(","))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use fp::prime::{TWO, ValidPrime};

    use super::*;

    /// The subalgebra of `ambient` with the given finite profile.
    fn sub(ambient: &MilnorAlgebra, q_part: u32, p_part: &[u32]) -> MilnorSubalgebra {
        let profile = MilnorProfile {
            truncated: true,
            q_part,
            p_part: p_part.to_vec(),
        };
        MilnorSubalgebra::new(ambient, profile).unwrap()
    }

    /// Check `sub`'s top degree and dimension against its basis, and its components against the
    /// basis of `ambient` through `max_degree`.
    fn check(ambient: &MilnorAlgebra, sub: &MilnorSubalgebra, max_degree: i32) {
        assert!(sub.is_subalgebra_of(ambient));

        let algebra = sub.algebra();
        let top = sub.top_degree();
        algebra.compute_basis(top + 4);
        assert_eq!(algebra.dimension(top), 1, "top degree of {sub}");
        assert!((top + 1..=top + 4).all(|t| algebra.dimension(t) == 0));
        let total: usize = (0..=top).map(|t| algebra.dimension(t)).sum();
        assert_eq!(sub.dimension(), total as u64);

        // `counts[t][c]` is the number of basis elements of `ambient` in degree `t` whose
        // component is `c`, given by its `(degree, index)` in `sub`.
        ambient.compute_basis(max_degree);
        let mut counts = vec![HashMap::new(); max_degree as usize + 1];
        for t in 0..=max_degree {
            for idx in 0..ambient.dimension(t) {
                let elt = ambient.basis_element_from_index(t, idx);
                let component = sub.component(&elt);
                let index = algebra
                    .try_basis_element_to_index(&component)
                    .unwrap_or_else(|| panic!("component {component} of {elt} is not in {sub}"));
                assert!(sub.has_component(&elt, &component));
                if let Some(mask) = sub.packed_component_mask() {
                    assert_eq!(elt.p_part.bits() & mask, component.p_part.bits());
                }
                *counts[t as usize]
                    .entry((component.degree, index))
                    .or_insert(0usize) += 1;
            }
        }

        // Freeness: the elements with component `c` are the translates of those with component 1.
        for t in 0..=max_degree {
            for k in 0..=std::cmp::min(t, top) {
                for index in 0..algebra.dimension(k) {
                    let translates = counts[(t - k) as usize].get(&(0, 0)).copied();
                    let actual = counts[t as usize].get(&(k, index)).copied();
                    assert_eq!(
                        actual.unwrap_or(0),
                        translates.unwrap_or(0),
                        "{sub}: degree {t}, component ({k}, {index})"
                    );
                }
            }
        }
    }

    /// Profiles of the classical algebra at `p = 2`, in it and in each other.
    #[test]
    fn test_polynomial_at_two() {
        let ambient = MilnorAlgebra::new(TWO, false);
        for p_part in [&[1][..], &[2, 1], &[2, 2, 1], &[3, 2, 1]] {
            check(&ambient, &sub(&ambient, 0, p_part), 40);
        }
        // Nested finite algebras, A(1) in A(2).
        let a2 = sub(&ambient, 0, &[3, 2, 1]);
        check(a2.algebra(), &sub(&ambient, 0, &[2, 1]), 23);

        assert_eq!(sub(&ambient, 0, &[2, 1]).top_degree(), 6);
        assert_eq!(a2.top_degree(), 23);
        assert_eq!(a2.dimension(), 64);
    }

    /// Infinite profiles are not subalgebras.
    #[test]
    fn test_infinite() {
        let ambient = MilnorAlgebra::new(TWO, false);
        assert!(MilnorSubalgebra::new(&ambient, MilnorProfile::default()).is_none());
        let unbounded = MilnorProfile {
            truncated: false,
            q_part: !0,
            p_part: vec![2, 1],
        };
        assert!(MilnorSubalgebra::new(&ambient, unbounded).is_none());
    }

    /// Containment compares profiles, and needs the same prime.
    #[test]
    fn test_is_subalgebra_of() {
        let ambient = MilnorAlgebra::new(TWO, false);
        let a1 = sub(&ambient, 0, &[2, 1]);
        let a2 = sub(&ambient, 0, &[3, 2, 1]);
        assert!(a1.is_subalgebra_of(a2.algebra()));
        assert!(!a2.is_subalgebra_of(a1.algebra()));
        assert!(a2.is_subalgebra_of(&ambient));
        assert!(!a1.is_subalgebra_of(&MilnorAlgebra::new(ValidPrime::new(3), false)));
    }

    /// The names of the trivial algebra, an `A(n)` and another profile.
    #[test]
    fn test_fmt() {
        let ambient = MilnorAlgebra::new(TWO, false);
        assert_eq!(MilnorSubalgebra::trivial(&ambient).to_string(), "F_2");
        assert_eq!(sub(&ambient, 0, &[3, 2, 1]).to_string(), "A(2)");
        assert_eq!(sub(&ambient, 0, &[3, 3, 2, 1]).to_string(), "B(3,3,2,1)");
    }

    /// $A^{\mathbb{C}}/\tau$ and the odd primary algebras, where the q-part counts.
    #[cfg(feature = "odd-primes")]
    #[test]
    fn test_exterior() {
        let c_tau: MilnorAlgebra = MilnorAlgebraInner::<Exterior>::new(TWO, false).into();
        for (q_part, p_part) in [(0b1, &[][..]), (0b11, &[]), (0b11, &[1]), (0b111, &[2, 1])] {
            check(&c_tau, &sub(&c_tau, q_part, p_part), 30);
        }
        assert_eq!(sub(&c_tau, 0b11, &[1]).top_degree(), 6);
        assert_eq!(MilnorSubalgebra::trivial(&c_tau).to_string(), "F_2");
        assert_eq!(sub(&c_tau, 0b1, &[]).to_string(), "A(0)");
        assert_eq!(sub(&c_tau, 0b11, &[1]).to_string(), "A(1)");
        assert_eq!(sub(&c_tau, 0b11, &[]).to_string(), "E(Q_0..Q_1)");
        assert!(MilnorSubalgebra::new(&c_tau, MilnorProfile::default()).is_none());

        let p3 = MilnorAlgebra::new(ValidPrime::new(3), false);
        for (q_part, p_part) in [(0b1, &[][..]), (0b11, &[1])] {
            check(&p3, &sub(&p3, q_part, p_part), 60);
        }
        assert_eq!(sub(&p3, 0b11, &[1]).top_degree(), 14);

        // |Q_19| = 2 * 3^19 - 1 does not fit in an `i32`, even on its own.
        let q19 = MilnorProfile {
            truncated: true,
            q_part: 1 << 19,
            p_part: vec![],
        };
        assert!(MilnorSubalgebra::new(&p3, q19).is_none());

        // Containment compares both parts.
        let a1 = sub(&p3, 0b11, &[1]);
        assert!(sub(&p3, 0b11, &[]).is_subalgebra_of(a1.algebra()));
        assert!(!a1.is_subalgebra_of(sub(&p3, 0b11, &[]).algebra()));
        assert!(!sub(&p3, 0b111, &[]).is_subalgebra_of(a1.algebra()));
    }
}
