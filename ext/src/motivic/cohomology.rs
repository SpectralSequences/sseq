//! The motivic Adams $E_2 = H(\delta)$, as cohomology and as an $\mathbb{F}_2[\tau]$-module.

use std::{collections::HashMap, sync::Arc};

use fp::{matrix::Matrix, prime::TWO};
use sseq::coordinates::{Bidegree, BidegreeGenerator};

use super::MotivicResolution;
use crate::{
    CCC,
    ext_algebra::{ExtDifferential, ExtModule},
    resolution::Resolution,
};

/// The motivic Adams $E_2$ in one bidegree as an $\mathbb{F}_2[\tau]$-module,
/// $\mathbb{F}_2[\tau]^{\mathrm{free}} \oplus \bigoplus_i \mathbb{F}_2[\tau]/\tau^{k_i}$.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TauModule {
    /// The free rank: the classical Adams $E_2$.
    pub free: usize,
    /// The orders $k_i$ of the $\tau$-torsion summands, ascending.
    pub torsion: Vec<u32>,
}

impl std::fmt::Display for TauModule {
    /// A compact label: `2` for a free rank of 2, `τ` and `τ^k` for torsion summands, joined by
    /// `+`, and `·` for zero.
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut parts: Vec<String> = Vec::new();
        if self.free > 0 {
            parts.push(self.free.to_string());
        }
        for &k in &self.torsion {
            parts.push(if k == 1 {
                "τ".to_string()
            } else {
                format!("τ^{k}")
            });
        }
        if parts.is_empty() {
            write!(f, "·")
        } else {
            write!(f, "{}", parts.join("+"))
        }
    }
}

/// δ as the coboundary of the Ext cochain complex, with every $\tau$ set to 1.
///
/// Its cohomology is then $H(\delta) \otimes_{\mathbb{F}_2[\tau]} \mathbb{F}_2[\tau^{\pm 1}]$
/// at $\tau = 1$, the classical Adams $E_2$.
struct MotivicCoboundary {
    /// The generators each generator's δ hits.
    deltas: HashMap<BidegreeGenerator, Vec<BidegreeGenerator>>,
    /// The number of generators in each bidegree.
    num_gens: HashMap<Bidegree, usize>,
    /// The box δ is certified in.
    max: Bidegree,
}

impl ExtDifferential for MotivicCoboundary {
    fn shift(&self) -> Bidegree {
        Bidegree::n_s(1, -1)
    }

    fn matrix(&self, b: Bidegree) -> Option<Matrix> {
        if b.s() < 0 || b.s() > self.max.s() || b.n() > self.max.n() {
            return None;
        }
        let rows = self.num_gens[&b];
        let columns = if b.s() == 0 {
            0
        } else {
            self.num_gens[&(b + self.shift())]
        };
        let mut m = Matrix::new(TWO, rows, columns);
        for idx in 0..rows {
            let g = BidegreeGenerator::new(b, idx);
            for target in self.deltas.get(&g).into_iter().flatten() {
                m.row_mut(idx).add_basis_element(target.idx(), 1);
            }
        }
        Some(m)
    }
}

impl MotivicResolution {
    /// $\mathrm{Ext}_{A_C/\tau}$ with δ as its differential.
    ///
    /// Its products are those of $\mathrm{Ext}_{A_C/\tau}$, the $E_1$ page of
    /// [`Self::deformation_sseq`], and its cohomology is the classical Adams $E_2$.
    pub fn ext(&self) -> &ExtModule<Resolution<CCC>> {
        self.ext.get_or_init(|| {
            let mut deltas = HashMap::new();
            let mut num_gens = HashMap::new();
            for s in 0..=self.max.s() {
                for t in 0..=self.compute.n() + s {
                    num_gens.insert(Bidegree::s_t(s, t), self.num_gens(s, t));
                    if s == 0 {
                        continue;
                    }
                    for idx in 0..self.num_gens(s, t) {
                        let g = BidegreeGenerator::s_t(s, t, idx);
                        let targets: Vec<_> = self.delta(g).into_iter().map(|(gj, _)| gj).collect();
                        deltas.insert(g, targets);
                    }
                }
            }
            let coboundary = MotivicCoboundary {
                deltas,
                num_gens,
                max: self.max,
            };
            ExtModule::intrinsic(Arc::clone(&self.resolution))
                .with_differential(Arc::new(coboundary))
        })
    }

    /// The motivic Adams $E_2$ at `(s, t)` as an $\mathbb{F}_2[\tau]$-module.
    ///
    /// The free rank is [`Self::classical_ext_rank`]. A class is $\tau^r$-torsion exactly when
    /// it supports a $d_r$ of [`Self::deformation_sseq`], whose δ is then $\tau^r$ times a cycle.
    pub fn tau_module(&self, s: i32, t: i32) -> TauModule {
        let n = t - s;
        let sseq = self.deformation_sseq();
        let mut torsion: Vec<u32> = Vec::new();
        for b in sseq.iter_degrees() {
            let [bn, bs, _] = b.coords();
            if bn != n || bs != s {
                continue;
            }
            let diffs = sseq.differentials(b);
            for r in diffs.min_degree()..diffs.len() {
                let sources = diffs[r].get_source_target_pairs().len();
                torsion.extend(std::iter::repeat_n(r as u32, sources));
            }
        }
        torsion.sort_unstable();
        TauModule {
            free: self.classical_ext_rank(s, t),
            torsion,
        }
    }
}

#[cfg(test)]
mod tests {
    use fp::vector::FpVector;
    use sseq::coordinates::{degree::MultiDegree, element::MultiDegreeElement};

    use super::*;
    use crate::motivic::{
        f2tau,
        tests::{h, sphere},
    };

    #[test]
    fn ext_products_climb_the_h0_tower() {
        let res = sphere(Bidegree::n_s(8, 5));
        let ext = res.ext();
        let h0 = ext.generator(h(0));
        let mut power = h0.clone();
        for n in 2..=4 {
            power = ext.multiply(&h0, &power);
            assert_eq!(power.degree(), Bidegree::n_s(0, n));
            assert!(!power.vec().is_zero(), "h₀^{n}");
        }
    }

    /// The $E_\infty$ of the deformation spectral sequence is the cohomology of δ at $\tau = 1$.
    #[test]
    fn deformation_sseq_converges_to_cohomology() {
        let max = Bidegree::n_s(8, 5);
        let res = sphere(max);
        for s in 0..max.s() {
            for n in 0..=max.n() {
                assert_eq!(
                    res.classical_ext_rank(s, n + s),
                    res.ext().cohomology_dimension(Bidegree::n_s(n, s)).unwrap(),
                    "(n={n}, s={s})"
                );
            }
        }
    }

    #[test]
    fn deformation_products_climb_the_h0_tower() {
        let res = sphere(Bidegree::n_s(8, 5));
        let sseq = res.deformation_sseq();
        let h0 = &res.deformation_products(&[h(0)])[0];
        let mut v = FpVector::new(TWO, 1);
        v.set_entry(0, 1);
        let mut class = MultiDegreeElement::new(MultiDegree::from([0, 1, 0]), v);
        for n in 2..=4 {
            class = sseq.multiply(&class, h0).unwrap();
            assert_eq!(class.degree(), MultiDegree::from([0, n, 0]));
            assert!(!class.vec().is_zero(), "h₀^{n}");
        }
    }

    /// $h_1^n$ is free for $n \le 3$ and $\mathbb{F}_2[\tau]/\tau$ from $h_1^4$ on, where the
    /// classical $h_1^4$ vanishes.
    #[test]
    fn the_h1_tower_turns_to_tau_torsion() {
        let res = sphere(Bidegree::n_s(10, 9));
        for n in 1..=8 {
            let expected = if n <= 3 { (1, vec![]) } else { (0, vec![1]) };
            let m = res.tau_module(n, 2 * n);
            assert_eq!((m.free, m.torsion), expected, "h₁^{n}");
        }
    }

    /// The torsion read off the deformation spectral sequence is that of the Smith normal form of
    /// δ over $\mathbb{F}_2[\tau]$.
    #[test]
    fn tau_torsion_matches_the_smith_normal_form() {
        let max = Bidegree::n_s(16, 11);
        let res = sphere(max);
        for s in 1..=max.s() {
            for n in 0..=max.n() {
                let t = n + s;
                let (rows, columns) = (res.num_gens(s, t), res.num_gens(s - 1, t));
                let mut m = vec![vec![0u128; columns]; rows];
                for (i, row) in m.iter_mut().enumerate() {
                    for (gj, power) in res.delta(BidegreeGenerator::s_t(s, t, i)) {
                        row[gj.idx()] ^= 1 << power;
                    }
                }
                let mut snf: Vec<u32> = f2tau::invariant_factors(m)
                    .into_iter()
                    .map(|f| f2tau::deg(f) as u32)
                    .collect();
                snf.sort_unstable();
                assert_eq!(res.tau_module(s, t).torsion, snf, "(n={n}, s={s})");
            }
        }
    }
}
