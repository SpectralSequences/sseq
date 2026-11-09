//! Every [`MilnorAlgebra`] as a sub-Hopf-algebra of its [kind](SteenrodKind), cut out by its profile.

use fp::prime::{Prime, iter::BitflagIterator};
use itertools::Itertools;

use super::{
    Exterior, MilnorAlgebra, MilnorAlgebraInner, MilnorBasisElement, MilnorProfile, MilnorShape,
    NoExterior, PPart, PPartEntry, SteenrodKind,
};
use crate::algebra::{Algebra, combinatorics};

impl MilnorAlgebra {
    /// The sub-Hopf-algebra of this algebra's kind with the given profile, at the same prime, or
    /// `None` if the profile is not valid. Unstable support is off.
    ///
    /// This need not lie in `self`; see [`Self::is_subalgebra_of`].
    pub fn with_profile(&self, profile: MilnorProfile) -> Option<Self> {
        if !profile.is_valid() {
            return None;
        }
        let p = self.prime();
        Some(match self {
            Self::Polynomial(_) => {
                MilnorAlgebraInner::<NoExterior>::new_with_profile(p, profile, false).into()
            }
            Self::Exterior(_) => {
                MilnorAlgebraInner::<Exterior>::new_with_profile(p, profile, false).into()
            }
        })
    }

    /// The trivial sub-Hopf-algebra of this algebra's kind, the ground field.
    pub fn ground_field(&self) -> Self {
        let profile = MilnorProfile {
            truncated: true,
            q_part: 0,
            p_part: vec![],
        };
        self.with_profile(profile).unwrap()
    }

    /// Whether `self` is a sub-Hopf-algebra of `other`: the same kind and prime, and a smaller
    /// profile.
    pub fn is_subalgebra_of(&self, other: &Self) -> bool {
        let len = std::cmp::max(self.profile().p_part.len(), other.profile().p_part.len());
        self.kind() == other.kind()
            && self.prime() == other.prime()
            && self.exterior_part() & !other.exterior_part() == 0
            // One entry past the longer profile compares the two tails.
            && (0..=len).all(|i| self.profile().get_p_part(i) <= other.profile().get_p_part(i))
    }

    /// The $Q_k$ in this algebra, as a mask of the q-part. Only the exterior shape has any.
    fn exterior_part(&self) -> u32 {
        if self.has_exterior() {
            self.profile().q_part
        } else {
            0
        }
    }

    /// `p^{e_i}`, which bounds entry `i` of the p-part of every basis element, or `None` if that
    /// entry is unbounded. A bound past [`PPartEntry::MAX`] bounds nothing.
    fn modulus(&self, i: usize) -> Option<PPartEntry> {
        self.prime()
            .as_u32()
            .checked_pow(self.profile().get_p_part(i))
    }

    /// The degree of the top basis element, or `None` if the algebra is infinite or that degree
    /// does not fit in an `i32`.
    ///
    /// A finite sub-Hopf-algebra is a Poincaré duality algebra, so the top basis element is the
    /// only one in its degree.
    pub fn top_degree(&self) -> Option<i32> {
        let p = self.prime();
        let profile = self.profile();
        if !profile.truncated || profile.p_part.len() > combinatorics::MAX_XI {
            return None;
        }
        let tau_degrees = combinatorics::tau_degrees(p);
        let xi_degrees = combinatorics::xi_degrees(p);

        let mut top = 0i64;
        for k in BitflagIterator::set_bit_iterator(self.exterior_part() as u64) {
            // A saturated entry stands for a degree past `i32::MAX`.
            if tau_degrees[k] == i32::MAX {
                return None;
            }
            top += tau_degrees[k] as i64;
        }
        for (i, &d) in xi_degrees.iter().enumerate().take(profile.p_part.len()) {
            top += self.q() as i64 * (self.modulus(i)? as i64 - 1) * d as i64;
        }
        i32::try_from(top).ok()
    }

    /// The dimension of the algebra, saturating at `u64::MAX`, or `None` where
    /// [`Self::top_degree`] is.
    pub fn total_dimension(&self) -> Option<u64> {
        self.top_degree()?;
        let exterior = 1u64 << self.exterior_part().count_ones();
        Some(
            (0..self.profile().p_part.len())
                .map(|i| self.modulus(i).unwrap() as u64)
                .fold(exterior, u64::saturating_mul),
        )
    }

    /// The slope of the vanishing line of Ext over this algebra, or `None` where
    /// [`Self::top_degree`] is: $\mathrm{Ext}^{s, t}$ vanishes for
    /// `t >= slope * (s + 1) + top_degree` ([Nassau](https://arxiv.org/abs/1910.04063), Theorem
    /// 3.1).
    ///
    /// This is the slope of the steepest polynomial generator of the May spectral sequence: `v_k`
    /// for each `Q_k`, and for each `ξ_i^{p^j}` either `h_{i, j}` at `p = 2` or `b_{i, j}` at odd
    /// primes. The exterior `h_{i, j}` at odd primes are bounded by the top degree.
    pub fn vanishing_slope(&self) -> Option<i64> {
        self.top_degree()?;
        let p = self.prime().as_i32() as i64;
        let q = self.q() as i64;
        let tau_degrees = combinatorics::tau_degrees(self.prime());
        let xi_degrees = combinatorics::xi_degrees(self.prime());

        let polynomial = self
            .profile()
            .p_part
            .iter()
            .enumerate()
            .flat_map(|(i, &e)| {
                (0..e).map(move |j| {
                    let h = q * xi_degrees[i] as i64 * p.pow(j);
                    if p == 2 { h } else { h * p / 2 }
                })
            });
        BitflagIterator::set_bit_iterator(self.exterior_part() as u64)
            .map(|k| tau_degrees[k] as i64)
            .chain(polynomial)
            .max()
            .or(Some(0))
    }

    /// The component in this algebra of `elt`, a basis element of an algebra containing it.
    ///
    /// The component of $Q(E)P(R)$ is $Q(E \cap E')P(R \bmod p^e)$, where $E'$ and $e$ are this
    /// algebra's q-part and profile, and an unbounded entry of $e$ leaves that entry of $R$ alone.
    /// Grouping the basis of the larger algebra by component splits it into translates of the
    /// elements whose component is $1$, one per basis element of this algebra, which is the
    /// freeness of the larger algebra over this one.
    pub fn component(&self, elt: &MilnorBasisElement) -> MilnorBasisElement {
        let mut component = MilnorBasisElement {
            q_part: elt.q_part & self.exterior_part(),
            p_part: PPart::zero(),
            degree: 0,
        };
        for i in 0..PPart::MAX_LEN {
            let entry = elt.p_part.get(i);
            component
                .p_part
                .set(i, self.modulus(i).map_or(entry, |m| entry % m));
        }
        self.compute_degree(&mut component);
        component
    }

    /// Whether the component of `elt` is `component`, without building it.
    pub fn has_component(&self, elt: &MilnorBasisElement, component: &MilnorBasisElement) -> bool {
        elt.q_part & self.exterior_part() == component.q_part
            && (0..PPart::MAX_LEN).all(|i| {
                let entry = elt.p_part.get(i);
                self.modulus(i).map_or(entry, |m| entry % m) == component.p_part.get(i)
            })
    }

    /// A mask that compiles [`Self::has_component`] to `elt.p_part.bits() & mask ==
    /// component.p_part.bits()`. It exists when the basis has no exterior part and every modulus
    /// is a power of two, i.e. at the polynomial shape at `p = 2`.
    ///
    /// Each entry occupies a fixed field of the packed word, so its residue mod $2^e$ is the low
    /// $e$ bits of that field.
    pub fn packed_component_mask(&self) -> Option<u64> {
        if self.kind() != SteenrodKind::Classical || self.prime() != 2 {
            return None;
        }
        Some((0..PPart::MAX_LEN).fold(0, |mask, i| {
            // A profile entry wider than the field, unbounded ones included, constrains all of it.
            let width = std::cmp::min(self.profile().get_p_part(i), PPart::width(i));
            mask | ((1u64 << width) - 1) << PPart::shift(i)
        }))
    }
}

/// The whole algebra as its kind and prime, e.g. `A(p=2)`, and otherwise `F_p`, `A(n)` or
/// `E(Q_0..Q_n)` where the profile is one of those, and the profile itself where not. An unbounded
/// tail is written `..`.
impl<F: MilnorShape> std::fmt::Display for MilnorAlgebraInner<F> {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let p = self.prime();
        let profile = self.profile();
        if *profile == MilnorProfile::default() {
            return write!(
                out,
                "{}(p={p})",
                SteenrodKind::from_shape(p, F::HAS_EXTERIOR)
            );
        }
        let MilnorProfile {
            truncated,
            q_part,
            p_part,
        } = profile;
        let len = p_part.len();
        let is_an = *truncated && p_part.iter().rev().copied().eq(1..=len as PPartEntry);
        let entries = p_part
            .iter()
            .map(|&e| {
                if e == PPartEntry::MAX {
                    "inf".to_string()
                } else {
                    e.to_string()
                }
            })
            .chain((!truncated).then(|| "..".to_string()))
            .join(",");
        if F::HAS_EXTERIOR {
            let top_q = (u32::BITS - q_part.leading_zeros()) as usize;
            let q_is_initial = q_part.count_ones() as usize == top_q;
            if *truncated && *q_part == 0 && len == 0 {
                write!(out, "F_{p}")
            } else if is_an && q_is_initial && top_q == len + 1 {
                write!(out, "A({len})")
            } else if *truncated && len == 0 && q_is_initial {
                write!(out, "E(Q_0..Q_{})", top_q - 1)
            } else {
                write!(out, "B({q_part:#b}; {entries})")
            }
        } else if *truncated && len == 0 {
            write!(out, "F_{p}")
        } else if is_an {
            write!(out, "A({})", len - 1)
        } else {
            write!(out, "B({entries})")
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use fp::prime::TWO;

    use super::*;

    /// The sub-Hopf-algebra of `ambient`'s kind with the given finite profile.
    fn sub(ambient: &MilnorAlgebra, q_part: u32, p_part: &[u32]) -> MilnorAlgebra {
        let profile = MilnorProfile {
            truncated: true,
            q_part,
            p_part: p_part.to_vec(),
        };
        ambient.with_profile(profile).unwrap()
    }

    /// Check `ambient`'s components in `sub` through `max_degree`, and if `sub` is finite its top
    /// degree and dimension against its basis.
    fn check(ambient: &MilnorAlgebra, sub: &MilnorAlgebra, max_degree: i32) {
        assert!(sub.is_subalgebra_of(ambient));

        let top = sub.top_degree();
        if let Some(top) = top {
            sub.compute_basis(top + 4);
            assert_eq!(sub.dimension(top), 1, "top degree of {:?}", sub.profile());
            assert!((top + 1..=top + 4).all(|t| sub.dimension(t) == 0));
            let total: usize = (0..=top).map(|t| sub.dimension(t)).sum();
            assert_eq!(sub.total_dimension(), Some(total as u64));
        }

        // `counts[t][c]` is the number of basis elements of `ambient` in degree `t` whose
        // component is `c`, given by its `(degree, index)` in `sub`.
        ambient.compute_basis(max_degree);
        sub.compute_basis(max_degree);
        let mut counts = vec![HashMap::new(); max_degree as usize + 1];
        for t in 0..=max_degree {
            for idx in 0..ambient.dimension(t) {
                let elt = ambient.basis_element_from_index(t, idx);
                let component = sub.component(&elt);
                let index = sub
                    .try_basis_element_to_index(&component)
                    .unwrap_or_else(|| panic!("component {component} of {elt} is not in sub"));
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
        let top = top.unwrap_or(max_degree);
        for t in 0..=max_degree {
            for k in 0..=std::cmp::min(t, top) {
                for index in 0..sub.dimension(k) {
                    let translates = counts[(t - k) as usize].get(&(0, 0)).copied();
                    let actual = counts[t as usize].get(&(k, index)).copied();
                    assert_eq!(
                        actual.unwrap_or(0),
                        translates.unwrap_or(0),
                        "degree {t}, component ({k}, {index})"
                    );
                }
            }
        }
    }

    /// The classical algebra at `p = 2`.
    fn ambient_at_two() -> MilnorAlgebra {
        MilnorAlgebra::new(TWO, false)
    }

    /// Profiles of the classical algebra at `p = 2`, in it and in each other.
    #[test]
    fn test_polynomial_at_two() {
        let ambient = ambient_at_two();
        for p_part in [&[1][..], &[2, 1], &[2, 2, 1], &[3, 2, 1]] {
            check(&ambient, &sub(&ambient, 0, p_part), 40);
        }
        // Nested finite algebras, A(1) in A(2).
        let a2 = sub(&ambient, 0, &[3, 2, 1]);
        check(&a2, &sub(&ambient, 0, &[2, 1]), 23);

        assert_eq!(sub(&ambient, 0, &[2, 1]).top_degree(), Some(6));
        assert_eq!(a2.top_degree(), Some(23));
        assert_eq!(a2.total_dimension(), Some(64));
    }

    /// Infinite sub-Hopf-algebras have no top degree, but components still split the ambient.
    #[test]
    fn test_infinite() {
        let ambient = ambient_at_two();
        assert_eq!(ambient.top_degree(), None);
        assert_eq!(ambient.total_dimension(), None);

        let unbounded = ambient
            .with_profile(MilnorProfile {
                truncated: false,
                q_part: !0,
                p_part: vec![2, 1],
            })
            .unwrap();
        assert_eq!(unbounded.top_degree(), None);
        check(&ambient, &unbounded, 40);
        check(&ambient, &ambient, 40);
    }

    /// Containment compares profiles, and needs the same kind and prime.
    #[test]
    fn test_is_subalgebra_of() {
        let ambient = ambient_at_two();
        let a1 = sub(&ambient, 0, &[2, 1]);
        let a2 = sub(&ambient, 0, &[3, 2, 1]);
        assert!(a1.is_subalgebra_of(&a2));
        assert!(!a2.is_subalgebra_of(&a1));
        assert!(a2.is_subalgebra_of(&ambient));
        assert!(ambient.ground_field().is_subalgebra_of(&a1));
    }

    /// $A^{\mathbb{C}}/\tau$ and the odd primary algebras, where the q-part counts.
    #[cfg(feature = "odd-primes")]
    #[test]
    fn test_exterior() {
        let c_tau = MilnorAlgebra::motivic_mod_tau(MilnorProfile::default(), false);
        for (q_part, p_part) in [(0b1, &[][..]), (0b11, &[]), (0b11, &[1]), (0b111, &[2, 1])] {
            check(&c_tau, &sub(&c_tau, q_part, p_part), 30);
        }
        assert_eq!(sub(&c_tau, 0b11, &[1]).top_degree(), Some(6));
        assert_eq!(c_tau.top_degree(), None);
        // A(1) of A_C/tau is not a subalgebra of the classical A(2).
        assert!(!sub(&c_tau, 0b11, &[1]).is_subalgebra_of(&ambient_at_two()));

        let p3 = MilnorAlgebra::new(fp::prime::ValidPrime::new(3), false);
        assert!(!sub(&ambient_at_two(), 0, &[2, 1]).is_subalgebra_of(&p3));
        for (q_part, p_part) in [(0b1, &[][..]), (0b11, &[1])] {
            check(&p3, &sub(&p3, q_part, p_part), 60);
        }
        assert_eq!(sub(&p3, 0b11, &[1]).top_degree(), Some(14));

        // |Q_19| = 2 * 3^19 - 1 does not fit in an `i32`, even on its own.
        assert_eq!(sub(&p3, 1 << 19, &[]).top_degree(), None);

        // Containment compares both parts.
        let a1 = sub(&p3, 0b11, &[1]);
        assert!(sub(&p3, 0b11, &[]).is_subalgebra_of(&a1));
        assert!(!a1.is_subalgebra_of(&sub(&p3, 0b11, &[])));
        assert!(!sub(&p3, 0b111, &[]).is_subalgebra_of(&a1));

        // The reduced powers are their own kind, not a subalgebra of A.
        let reduced = MilnorAlgebra::reduced_powers(p3.prime(), MilnorProfile::default(), false);
        assert_eq!(reduced.kind(), SteenrodKind::ReducedPowers);
        assert!(!reduced.is_subalgebra_of(&p3));
        check(&reduced, &sub(&reduced, 0, &[2, 1]), 60);
    }

    /// Each constructor builds the kind it names, and the names of whole algebras say so.
    #[cfg(feature = "odd-primes")]
    #[test]
    fn test_kinds() {
        let p3 = fp::prime::ValidPrime::new(3);
        let full = MilnorProfile::default;
        for (algebra, kind, name) in [
            (
                MilnorAlgebra::new(TWO, false),
                SteenrodKind::Classical,
                "A(p=2)",
            ),
            (
                MilnorAlgebra::new(p3, false),
                SteenrodKind::Classical,
                "A(p=3)",
            ),
            (
                MilnorAlgebra::motivic_mod_tau(full(), false),
                SteenrodKind::MotivicModTau,
                "A_C/tau(p=2)",
            ),
            (
                MilnorAlgebra::reduced_powers(p3, full(), false),
                SteenrodKind::ReducedPowers,
                "P(p=3)",
            ),
        ] {
            assert_eq!(algebra.kind(), kind);
            assert_eq!(algebra.to_string(), name);
        }
    }

    /// The reduced powers do not exist at `p = 2`.
    #[test]
    #[should_panic(expected = "only defined at odd primes")]
    fn test_reduced_powers_at_two() {
        MilnorAlgebra::reduced_powers(TWO, MilnorProfile::default(), false);
    }

    /// The names of the trivial algebra, an `A(n)`, another finite profile and an infinite one.
    #[test]
    fn test_fmt() {
        let ambient = ambient_at_two();
        assert_eq!(ambient.ground_field().to_string(), "F_2");
        assert_eq!(sub(&ambient, 0, &[3, 2, 1]).to_string(), "A(2)");
        assert_eq!(sub(&ambient, 0, &[3, 3, 2, 1]).to_string(), "B(3,3,2,1)");
        let infinite = MilnorProfile {
            truncated: false,
            q_part: !0,
            p_part: vec![2, 1],
        };
        assert_eq!(
            ambient.with_profile(infinite).unwrap().to_string(),
            "B(2,1,..)"
        );
    }

    /// The vanishing slope is that of the steepest May generator, and only finite algebras have
    /// one.
    #[test]
    fn test_vanishing_slope() {
        let ambient = ambient_at_two();
        assert_eq!(ambient.vanishing_slope(), None);
        assert_eq!(ambient.ground_field().vanishing_slope(), Some(0));
        // h_{1,1} in A(1) has degree 2, h_{2,0} has degree 3.
        assert_eq!(sub(&ambient, 0, &[2, 1]).vanishing_slope(), Some(3));
    }

    /// The trivial algebra and `E(Q_0)` have no polynomial part, and their generators are what
    /// they should be.
    #[test]
    fn test_empty_p_part_generators() {
        use crate::algebra::GeneratedAlgebra;

        let ambient = ambient_at_two();
        let trivial = ambient.ground_field();
        trivial.compute_basis(8);
        assert!((0..=8).all(|t| trivial.generators(t).is_empty()));

        #[cfg(feature = "odd-primes")]
        {
            let p3 = MilnorAlgebra::new(fp::prime::ValidPrime::new(3), false);
            let trivial = p3.ground_field();
            trivial.compute_basis(8);
            assert!((0..=8).all(|t| trivial.generators(t).is_empty()));

            let e0 = sub(&p3, 0b1, &[]);
            e0.compute_basis(8);
            assert_eq!(e0.generators(1).len(), 1, "Q_0 generates E(Q_0)");
            assert!((2..=8).all(|t| e0.generators(t).is_empty()));
        }
    }
}
