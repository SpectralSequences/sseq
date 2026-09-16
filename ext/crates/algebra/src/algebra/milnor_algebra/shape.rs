use fp::prime::{Prime, ValidPrime, factor_pk, iter::BitflagIterator};

use super::{MilnorAlgebraInner, MilnorBasisElement, PPart};
use crate::algebra::{Algebra, combinatorics};

mod private {
    /// Seals [`MilnorShape`](super::MilnorShape) so that `q` and the presence of an exterior
    /// part cannot be chosen independently.
    pub trait Sealed {}
}

/// The shape of the Milnor basis of a dual Steenrod algebra.
///
/// The dual Steenrod algebra is a polynomial algebra on the $\xi_i$ tensored with an exterior
/// algebra on the $\tau_k$. The exterior part is absent in exactly one case, the classical algebra
/// at $p = 2$, which is why the two shapes are distinguished here rather than by the prime: the
/// mod-$\tau$ C-motivic algebra $A^{\mathbb{C}}/\tau$ has the exterior shape *at* $p = 2$.
pub trait MilnorShape: private::Sealed + Sized + Send + Sync + 'static {
    /// Whether basis elements carry an exterior part.
    const HAS_EXTERIOR: bool;

    /// The scale of the polynomial grading: $\xi_i$ has degree `q * XI_DEGREES[i]`.
    fn q(p: ValidPrime) -> i32;

    /// Fill in the algebra's basis table up to `max_degree`.
    fn generate_basis(algebra: &MilnorAlgebraInner<Self>, max_degree: i32);

    /// The indices in `degree` of the algebra generators, as required by
    /// [`GeneratedAlgebra`](crate::algebra::GeneratedAlgebra).
    fn generators(algebra: &MilnorAlgebraInner<Self>, degree: i32) -> Vec<usize>;

    /// The name of the generator at `(degree, idx)`.
    fn generator_to_string(algebra: &MilnorAlgebraInner<Self>, degree: i32, idx: usize) -> String;

    /// The elements that induce the filtration one products, with the degree to compute up to.
    fn filtration_one_products(
        algebra: &MilnorAlgebraInner<Self>,
    ) -> (Vec<(String, MilnorBasisElement)>, i32);
}

/// The polynomial-only Milnor basis: the classical dual Steenrod algebra at $p = 2$.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoExterior;

/// The Milnor basis with an exterior part: the classical dual Steenrod algebra at odd primes, and
/// $A^{\mathbb{C}}/\tau$ at $p = 2$.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Exterior;

impl private::Sealed for NoExterior {}
impl private::Sealed for Exterior {}

/// Operations shared by both shapes, parameterised only by [`MilnorShape::q`].
impl<F: MilnorShape> MilnorAlgebraInner<F> {
    /// Set `elt`'s degree component to the degree it has in this algebra.
    pub fn compute_degree(&self, elt: &mut MilnorBasisElement) {
        let p = self.prime();
        let xi_degrees = combinatorics::xi_degrees(p);
        let tau_degrees = combinatorics::tau_degrees(p);

        elt.degree = self.q()
            * std::iter::zip(xi_degrees, elt.p_part.iter())
                .map(|(&a, b)| a * b as i32)
                .sum::<i32>()
            + BitflagIterator::set_bit_iterator(elt.q_part as u64)
                .map(|k| tau_degrees[k])
                .sum::<i32>();
    }

    /// The index of the polynomial generator in `degree`, if there is one.
    ///
    /// The generators are $P(0, \ldots, 0, p^k)$ with the entry in slot `j - 1`, of degree
    /// `q * XI_DEGREES[j - 1] * p^k`. Dividing by `q` first makes the decoding uniform in the
    /// prime: the cofactor left by [`factor_pk`] is then exactly `XI_DEGREES[j - 1]`, which is
    /// coprime to `p` because it is $1 + p + \cdots + p^{j-1}$. Factoring the undivided degree
    /// instead would over-count the powers of `p` whenever `q` is itself divisible by `p`, which
    /// is what happens for [`Exterior`] at `p = 2`.
    fn polynomial_generator(&self, degree: i32) -> Vec<usize> {
        let p = self.prime();
        let q = self.q() as u32;
        let degree = degree as u32;

        if !degree.is_multiple_of(q) {
            return vec![];
        }
        let (k, cofactor) = factor_pk(p, degree / q);

        // `is_an` profiles only keep the generators with `j == 1`, where `XI_DEGREES[0] == 1`.
        if self.profile.is_an(F::HAS_EXTERIOR) {
            if cofactor != 1 || k >= self.profile.get_p_part(0) {
                return vec![];
            }
            return vec![self.basis_element_to_index(&MilnorBasisElement {
                degree: degree as i32,
                q_part: 0,
                p_part: PPart::from_iter([p.pow(k)]),
            })];
        }

        let Some(j) = combinatorics::xi_degrees(p)
            .iter()
            .position(|&d| d as u32 == cofactor)
            .map(|i| i + 1)
        else {
            return vec![];
        };
        if self.profile.get_p_part(j - 1) <= k {
            return vec![];
        }
        let mut p_part = PPart::zero();
        p_part.set(j - 1, p.pow(k));
        vec![self.basis_element_to_index(&MilnorBasisElement {
            degree: degree as i32,
            q_part: 0,
            p_part,
        })]
    }

    /// The name of the polynomial generator at `(degree, idx)`, given the name of $P(n)$.
    fn polynomial_generator_to_string(&self, degree: i32, idx: usize, single: &str) -> String {
        let elt = self.basis_element_from_index(degree, idx);
        let len = elt.p_part.len();
        if len == 1 {
            format!("{single}{}", degree / self.q())
        } else {
            format!(
                "P^{}_{len}",
                degree / (self.q() * combinatorics::xi_degrees(self.prime())[len - 1]),
            )
        }
    }
}

impl MilnorShape for NoExterior {
    const HAS_EXTERIOR: bool = false;

    /// The polynomial grading is unscaled: $\xi_i$ has degree `XI_DEGREES[i]` exactly.
    fn q(_p: ValidPrime) -> i32 {
        1
    }

    /// Without an exterior part the p-part table is already the basis.
    fn generate_basis(algebra: &MilnorAlgebraInner<Self>, max_degree: i32) {
        algebra.generate_basis_polynomial(max_degree);
    }

    /// Every generator is polynomial; there is no exterior part to contribute any.
    fn generators(algebra: &MilnorAlgebraInner<Self>, degree: i32) -> Vec<usize> {
        algebra.polynomial_generator(degree)
    }

    /// $P(n)$ is written $Sq^n$ at the prime 2.
    fn generator_to_string(algebra: &MilnorAlgebraInner<Self>, degree: i32, idx: usize) -> String {
        algebra.polynomial_generator_to_string(degree, idx, "Sq")
    }

    /// The $h_i$, dual to $Sq^{2^i}$, as far as the profile allows.
    fn filtration_one_products(
        algebra: &MilnorAlgebraInner<Self>,
    ) -> (Vec<(String, MilnorBasisElement)>, i32) {
        let profile = &algebra.profile;
        let max = if !profile.p_part.is_empty() {
            std::cmp::min(4, profile.p_part[0])
        } else if profile.truncated {
            0
        } else {
            4
        };
        let products = (0..max)
            .map(|i| {
                let degree = 1 << i; // degree is 2^hi
                (
                    format!("h_{i}"),
                    MilnorBasisElement {
                        degree,
                        q_part: 0,
                        p_part: PPart::from_iter([1 << i]),
                    },
                )
            })
            .collect();
        (products, 1 << 3)
    }
}

impl MilnorShape for Exterior {
    const HAS_EXTERIOR: bool = true;

    /// At `p = 2` this is 2, the scale $A^{\mathbb{C}}/\tau$ needs — the classical algebra takes
    /// `q = 1` there because it is the *other* shape, not because the formula fails.
    fn q(p: ValidPrime) -> i32 {
        2 * (p.as_i32() - 1)
    }

    /// A basis element is an exterior part together with a p-part.
    fn generate_basis(algebra: &MilnorAlgebraInner<Self>, max_degree: i32) {
        algebra.generate_basis_exterior(max_degree);
    }

    /// The $Q_k$ in odd degrees, the polynomial generators in even ones.
    fn generators(algebra: &MilnorAlgebraInner<Self>, degree: i32) -> Vec<usize> {
        // The polynomial part sits in degrees divisible by `q`, which is even, so an odd degree
        // can only hold a $Q_k$.
        if degree % 2 == 1 {
            if algebra.profile.is_an(true) {
                return vec![];
            }
            // $|Q_k| = 2p^k - 1$ is exactly `TAU_DEGREES[k]`. Looking it up keeps this correct at
            // `p = 2`, where testing `factor_pk(p, degree + 1) == (k, 2)` fails: `2p^k` is then a
            // pure power of the prime and the cofactor is 1, not 2.
            let Some(k) = combinatorics::tau_degrees(algebra.prime())
                .iter()
                .position(|&d| d == degree)
            else {
                return vec![];
            };
            let q_part = 1 << k;
            if algebra.profile.q_part & q_part == 0 {
                return vec![];
            }
            return vec![algebra.basis_element_to_index(&MilnorBasisElement {
                degree,
                q_part,
                p_part: PPart::zero(),
            })];
        }
        algebra.polynomial_generator(degree)
    }

    /// $Q_0$ is written `b`, the other exterior generators by their own `Display`, and the
    /// polynomial ones $P^n$.
    fn generator_to_string(algebra: &MilnorAlgebraInner<Self>, degree: i32, idx: usize) -> String {
        if degree == 1 {
            return "b".to_string();
        }
        let elt = algebra.basis_element_from_index(degree, idx);
        if elt.q_part != 0 {
            elt.to_string()
        } else {
            algebra.polynomial_generator_to_string(degree, idx, "P")
        }
    }

    /// $a_0$, dual to the Bockstein, and $h_0$, dual to $P(1)$.
    fn filtration_one_products(
        algebra: &MilnorAlgebraInner<Self>,
    ) -> (Vec<(String, MilnorBasisElement)>, i32) {
        let profile = &algebra.profile;
        let mut products = Vec::with_capacity(2);
        if profile.q_part & 1 != 0 {
            products.push((
                "a_0".to_string(),
                MilnorBasisElement {
                    degree: 1,
                    q_part: 1,
                    p_part: PPart::zero(),
                },
            ));
        }
        if (profile.p_part.is_empty() && !profile.truncated)
            || (!profile.p_part.is_empty() && profile.p_part[0] > 0)
        {
            products.push((
                "h_0".to_string(),
                MilnorBasisElement {
                    degree: Self::q(algebra.prime()),
                    q_part: 0,
                    p_part: PPart::from_iter([1]),
                },
            ));
        }
        (products, Self::q(algebra.prime()))
    }
}

#[cfg(test)]
mod tests {
    use fp::vector::FpVector;

    use super::*;
    use crate::algebra::{
        Bialgebra, GeneratedAlgebra,
        milnor_algebra::{MilnorAlgebra, MilnorProfile},
    };

    /// The exterior shape at `p = 2`, which is $A^{\mathbb{C}}/\tau$.
    ///
    /// That configuration is unreachable through [`MilnorAlgebra`], so these check it against the
    /// independent Kong–Lin closed form in [`crate::algebra::motivic::milnor`], which shares no
    /// code with this file.
    mod exterior_at_two {
        use fp::prime::TWO;

        use super::*;
        use crate::algebra::motivic::milnor::{
            Bigraded, Dual, Monomial, enum_basis, multiply_closed_mod_tau,
        };

        /// The mod-$\tau$ C-motivic Steenrod algebra: the exterior shape at the prime 2.
        fn ctau() -> MilnorAlgebraInner<Exterior> {
            MilnorAlgebraInner::<Exterior>::new(TWO, false)
        }

        /// The two presentations must at least agree on which elements exist in each degree.
        #[test]
        fn basis_matches_the_engine() {
            let algebra = ctau();
            const MAX: i32 = 12;
            algebra.compute_basis(MAX);

            for t in 0..=MAX {
                let engine: Vec<Dual<Monomial>> = enum_basis(t);
                assert_eq!(
                    algebra.dimension(t),
                    engine.len(),
                    "dimension disagrees in degree {t}"
                );
                for Dual(m) in engine {
                    let elt = MilnorBasisElement {
                        q_part: m.q_part,
                        p_part: m.p_part,
                        degree: t,
                    };
                    assert_eq!(
                        m.bidegree().0,
                        t,
                        "the engine's own grading disagrees with its enumeration"
                    );
                    // Degree computed from the entries, not taken on trust from the engine.
                    let mut recomputed = elt;
                    algebra.compute_degree(&mut recomputed);
                    assert_eq!(recomputed.degree, t, "degree of {elt} in degree {t}");
                    assert!(
                        algebra.try_basis_element_to_index(&elt).is_some(),
                        "{elt} is in the engine's degree {t} but not the algebra's"
                    );
                }
            }
        }

        /// Every structure constant, against the closed form.
        ///
        /// The two order their factors oppositely: this algebra commutes the *right* factor's
        /// exterior part leftwards, and the engine the left factor's. The orientation is asserted
        /// rather than assumed — [`product_orientation_is_not_symmetric`] shows the transposed
        /// reading is a genuinely different answer, so a wrong choice here would fail loudly
        /// rather than quietly agree.
        #[test]
        fn products_match_the_engine() {
            let algebra = ctau();
            const MAX: i32 = 18;
            algebra.compute_basis(MAX);

            let mut checked = 0;
            for t1 in 0..=MAX {
                for t2 in 0..=(MAX - t1) {
                    let t = t1 + t2;
                    for i1 in 0..algebra.dimension(t1) {
                        for i2 in 0..algebra.dimension(t2) {
                            let m1 = algebra.basis_element_from_index(t1, i1);
                            let m2 = algebra.basis_element_from_index(t2, i2);

                            let mut ours = FpVector::new(TWO, algebra.dimension(t));
                            algebra.multiply(ours.as_slice_mut(), 1, m1, m2);

                            let theirs = multiply_closed_mod_tau(
                                Dual(Monomial::new(m2.q_part, m2.p_part)),
                                Dual(Monomial::new(m1.q_part, m1.p_part)),
                            );
                            let mut expected = FpVector::new(TWO, algebra.dimension(t));
                            for &Dual(m) in &theirs {
                                let elt = MilnorBasisElement {
                                    q_part: m.q_part,
                                    p_part: m.p_part,
                                    degree: t,
                                };
                                expected.add_basis_element(algebra.basis_element_to_index(&elt), 1);
                            }

                            assert_eq!(ours, expected, "({m1}) * ({m2}) in degree {t}");
                            checked += 1;
                        }
                    }
                }
            }
            assert!(checked > 1000, "only {checked} products checked");
        }

        /// The orientation in [`products_match_the_engine`] is load-bearing.
        ///
        /// Transposing the factors is not a no-op that happens to agree: it disagrees with the
        /// engine on some product. Without this, passing the check above would be equally
        /// consistent with the two conventions coinciding, and a transposed reading of a
        /// non-commutative product is a well-formed wrong answer rather than an error.
        #[test]
        fn the_transposed_orientation_disagrees() {
            let algebra = ctau();
            const MAX: i32 = 10;
            algebra.compute_basis(MAX);

            let mut disagreements = 0;
            for t1 in 0..=MAX {
                for t2 in 0..=(MAX - t1) {
                    let t = t1 + t2;
                    for i1 in 0..algebra.dimension(t1) {
                        for i2 in 0..algebra.dimension(t2) {
                            let m1 = algebra.basis_element_from_index(t1, i1);
                            let m2 = algebra.basis_element_from_index(t2, i2);

                            let mut ours = FpVector::new(TWO, algebra.dimension(t));
                            algebra.multiply(ours.as_slice_mut(), 1, m1, m2);

                            // The factors the wrong way round.
                            let transposed = multiply_closed_mod_tau(
                                Dual(Monomial::new(m1.q_part, m1.p_part)),
                                Dual(Monomial::new(m2.q_part, m2.p_part)),
                            );
                            let mut expected = FpVector::new(TWO, algebra.dimension(t));
                            for &Dual(m) in &transposed {
                                let elt = MilnorBasisElement {
                                    q_part: m.q_part,
                                    p_part: m.p_part,
                                    degree: t,
                                };
                                expected.add_basis_element(algebra.basis_element_to_index(&elt), 1);
                            }
                            if ours != expected {
                                disagreements += 1;
                            }
                        }
                    }
                }
            }
            assert!(
                disagreements > 0,
                "the two orientations agree everywhere, so the orientation is untested"
            );
        }

        /// The full algebra reports no $Q_k$ with `k >= 1`, because they are decomposable.
        ///
        /// As at odd primes: $Q_{k+1} = P(2^k) Q_k - Q_k P(2^k)$.
        #[test]
        fn the_full_algebra_has_no_odd_degree_generators_above_one() {
            let algebra = ctau();
            const MAX: i32 = 33;
            algebra.compute_basis(MAX);

            for degree in (3..=MAX).step_by(2) {
                assert!(
                    algebra.generators(degree).is_empty(),
                    "degree {degree} should have no generators"
                );
            }
        }

        /// Under a profile that is not an A(n), $Q_k$ *is* a generator.
        ///
        /// It sits in degree `|Q_k| = 2^{k+1} - 1`.
        ///
        /// This is the branch where the prime alone no longer identifies the degrees: the
        /// classical test for it, `factor_pk(p, degree + 1) == (k, 2)`, matches nothing at
        /// `p = 2`, because `2 p^k` is then a pure power of the prime and leaves cofactor 1.
        #[test]
        fn q_k_is_a_generator_under_a_profile() {
            let profile = MilnorProfile {
                truncated: false,
                q_part: !0,
                p_part: vec![2],
            };
            assert!(!profile.is_an(true), "the test needs a non-A(n) profile");
            let algebra = MilnorAlgebraInner::<Exterior>::new_with_profile(TWO, profile, false);
            const MAX: i32 = 33;
            algebra.compute_basis(MAX);

            for k in 1..5 {
                let degree = (1 << (k + 1)) - 1;
                if degree > MAX {
                    break;
                }
                let q_k = MilnorBasisElement {
                    q_part: 1 << k,
                    p_part: PPart::zero(),
                    degree,
                };
                let idx = algebra.basis_element_to_index(&q_k);
                assert_eq!(
                    algebra.generators(degree),
                    vec![idx],
                    "Q_{k} (degree {degree}) should be the generator in its degree"
                );
            }
        }

        /// The polynomial generators are the $P(2^k)$, in degree `q * 2^k = 2^{k+1}`.
        ///
        /// The classical decoding factors the undivided degree, which at `q = 2` counts one power
        /// of the prime too many.
        #[test]
        fn polynomial_generators_are_found() {
            let algebra = ctau();
            const MAX: i32 = 60;
            algebra.compute_basis(MAX);

            for k in 0..4 {
                let degree = 1 << (k + 1);
                let gens = algebra.generators(degree);
                let generator = MilnorBasisElement {
                    q_part: 0,
                    p_part: PPart::from_iter([1 << k]),
                    degree,
                };
                let idx = algebra.basis_element_to_index(&generator);
                assert!(
                    gens.contains(&idx),
                    "P({}) (degree {degree}) is missing from the generators {gens:?}",
                    1 << k
                );
            }
        }

        /// The polynomial generators under a profile that is not an A(n).
        ///
        /// They are the $P(0, \ldots, 0, 2^k)$ there, rather than only the $P(2^k)$.
        ///
        /// This is the branch that has to divide by `q` before factoring out the prime. Factoring
        /// the undivided degree — as the classical code does — absorbs `q`'s own factor of 2 at
        /// `p = 2` and decodes degree 6 as $P(2)$, which lives in degree 4.
        #[test]
        fn polynomial_generators_under_a_profile() {
            let profile = MilnorProfile {
                truncated: false,
                q_part: !0,
                p_part: vec![2],
            };
            assert!(!profile.is_an(true), "the test needs a non-A(n) profile");
            let algebra = MilnorAlgebraInner::<Exterior>::new_with_profile(TWO, profile, false);
            const MAX: i32 = 30;
            algebra.compute_basis(MAX);

            // `P(0, ..., 0, 2^k)` with the entry in slot `j - 1` has degree
            // `q * XI_DEGREES[j - 1] * 2^k = 2 (2^j - 1) 2^k`. The profile caps slot 0 at `k < 2`.
            let expected: &[(i32, &[u32])] = &[
                (2, &[1]),
                (4, &[2]),
                (6, &[0, 1]),
                (12, &[0, 2]),
                (14, &[0, 0, 1]),
                (24, &[0, 4]),
            ];
            for &(degree, entries) in expected {
                let generator = MilnorBasisElement {
                    q_part: 0,
                    p_part: PPart::try_from_slice(entries).unwrap(),
                    degree,
                };
                let idx = algebra.basis_element_to_index(&generator);
                assert_eq!(
                    algebra.generators(degree),
                    vec![idx],
                    "degree {degree} should be generated by {generator}"
                );
            }

            // `8 = 2 * 1 * 2^2` only as `j = 1, k = 2`, which the profile excludes.
            assert!(
                algebra.generators(8).is_empty(),
                "the profile should exclude P(4) in degree 8"
            );
        }

        /// The coproduct is not available on this shape at all.
        ///
        /// It is a compile-time restriction rather than an assertion, so this only records that
        /// the classical one still works and that `MilnorAlgebraInner<Exterior>` does not offer
        /// the method. The formula ignores the exterior part and grades $\xi_i$ with `q = 1`, so
        /// reaching it here would give a wrong answer, not an error.
        #[test]
        fn the_classical_coproduct_is_unaffected() {
            let classical = MilnorAlgebraInner::<NoExterior>::new(TWO, false);
            classical.compute_basis(8);
            let idx = classical.basis_element_to_index(&MilnorBasisElement {
                q_part: 0,
                p_part: PPart::from_iter([2]),
                degree: 2,
            });
            // Sq^2 |-> Sq^2 (x) 1 + Sq^1 (x) Sq^1 + 1 (x) Sq^2.
            assert_eq!(classical.coproduct(2, idx).len(), 3);
        }

        /// The two shapes at `p = 2` must not share a [`Algebra::magic`].
        ///
        /// `SaveFile` validates the header against it, so a collision would let a file written
        /// over one basis load as the other with every coefficient reindexed. The literals pin
        /// the classical values, which are a wire format: changing one invalidates saved
        /// resolutions without any error at load time.
        #[test]
        fn magic_distinguishes_the_shapes() {
            let exterior = MilnorAlgebraInner::<Exterior>::new(TWO, false);
            let classical = MilnorAlgebraInner::<NoExterior>::new(TWO, false);
            assert_ne!(exterior.magic(), classical.magic());

            assert_eq!(classical.magic(), 0x0002_8000);
            assert_eq!(MilnorAlgebra::new(TWO, false).magic(), 0x0002_8000);
            assert_eq!(
                MilnorAlgebra::new(ValidPrime::new(3), false).magic(),
                0x0003_8000
            );
            assert_eq!(
                MilnorAlgebra::new(ValidPrime::new(5), false).magic(),
                0x0005_8000
            );
        }

        /// Generators generate: every basis element in low degrees is a product of them.
        #[test]
        fn generators_span_the_algebra() {
            let algebra = ctau();
            const MAX: i32 = 16;
            algebra.compute_basis(MAX);

            for t in 1..=MAX {
                let generators = algebra.generators(t);
                for i in 0..algebra.dimension(t) {
                    if generators.contains(&i) {
                        continue;
                    }
                    let decomposition = algebra.decompose_basis_element(t, i);
                    assert!(
                        !decomposition.is_empty(),
                        "{} (degree {t}) does not decompose",
                        algebra.basis_element_from_index(t, i)
                    );
                }
            }
        }
    }
}
