//! Products in the motivic Adams $E_2$, read off the product chain map lifted to $A_C$.

use std::{
    collections::{BTreeSet, HashMap},
    sync::Arc,
};

use algebra::{
    SteenrodAlgebra,
    module::{FreeModule, Module, homomorphism::ModuleHomomorphism},
};
use fp::{prime::TWO, vector::FpVector};
use maybe_rayon::prelude::*;
use sseq::coordinates::BidegreeGenerator;

use super::{Lifted, MotivicResolution, TauLift, certify::Map};
use crate::chain_complex::{AugmentedChainComplex, ChainComplex};

impl MotivicResolution {
    /// Check that products are defined: the resolution is of the unit, so it is its own `k`.
    ///
    /// # Panics
    ///
    /// If the resolved module is not the unit.
    pub(super) fn assert_unit(&self) {
        assert!(
            self.resolution.target().module(0).is_unit(),
            "products need a resolution of the unit, such as `Ctau`"
        );
    }

    /// Lift the product chain map $\varphi_a$, left multiplication by the generator `a`, unless
    /// it is lifted already.
    pub(super) fn lift_product(&mut self, a: BidegreeGenerator) {
        if !self.products.contains_key(&a) {
            let lifted = self.product_lift(a);
            self.products.insert(a, lifted);
        }
    }

    /// The product chain map $\varphi_a$ lifted to $A_C$, through every resolved generator.
    ///
    /// The mod-$\tau$ seeds are [`ExtModule`](crate::ext_algebra::ExtModule)'s product map, and
    /// the corrections make it a chain map over $A_C$, with `s` ascending since the cell at `g`
    /// reads $\varphi_a$ on $d(g)$.
    #[tracing::instrument(skip(self))]
    fn product_lift(&self, a: BidegreeGenerator) -> Lifted {
        let hom = self.ext().generator_product_map(a);
        hom.extend_through_stem(self.compute);

        let mut seeds: HashMap<BidegreeGenerator, BTreeSet<usize>> = HashMap::new();
        for s in a.s()..=self.max.s() {
            let map = hom.get_map(s);
            for t in a.t()..=self.compute.n() + s {
                for idx in 0..self.num_gens(s, t) {
                    let support: BTreeSet<usize> = map
                        .output(t, idx)
                        .iter_nonzero()
                        .filter(|(_, v)| *v != 0)
                        .map(|(i, _)| i)
                        .collect();
                    seeds.insert(BidegreeGenerator::s_t(s, t, idx), support);
                }
            }
        }

        let mut phi = Lifted::default();
        for s in a.s()..=self.max.s() {
            let gens: Vec<BidegreeGenerator> = (a.t()..=self.compute.n() + s)
                .flat_map(|t| {
                    (0..self.num_gens(s, t)).map(move |idx| BidegreeGenerator::s_t(s, t, idx))
                })
                .collect();
            let cells = ProductCells {
                res: self,
                a,
                seeds: &seeds,
                phi: &phi.cells,
            };
            let lifted: Vec<_> = gens
                .into_maybe_par_iter()
                .map(|g| {
                    // The image in F_0 has no defect to cancel.
                    let lift = if g.s() == a.s() {
                        Ok(cells.seed(g))
                    } else {
                        cells.lift_cell(g)
                    };
                    (g, lift)
                })
                .collect();
            for (g, lift) in lifted {
                phi.insert(g, lift);
            }
        }
        phi
    }

    /// The motivic product `a · b` of two generators, as the generators of its bidegree with
    /// the $\tau$-power each carries.
    ///
    /// The product is the coefficient of `1 ⊗ b` in $\varphi_a(g_k)$ for each generator $g_k$
    /// of the product's bidegree, with $\tau$-power $w(a) + w(b) - w(g_k)$. A positive power
    /// where the product vanishes mod $\tau$ is a hidden extension, such as $h_0 \cdot h_0 h_2 =
    /// \tau h_1^3$.
    ///
    /// # Panics
    ///
    /// If the resolved module is not the unit, or if `a · b` lies outside `max`.
    pub fn motivic_product(
        &mut self,
        a: BidegreeGenerator,
        b: BidegreeGenerator,
    ) -> Vec<(BidegreeGenerator, u32)> {
        self.assert_unit();
        let (s, t) = (a.s() + b.s(), a.t() + b.t());
        assert!(
            s <= self.max.s() && t - s <= self.max.n(),
            "{a:?} · {b:?} lies outside the box {}",
            self.max
        );
        let w_ab = self.weights[&a] + self.weights[&b];
        let targets: Vec<BidegreeGenerator> = (0..self.num_gens(s, t))
            .map(|k| BidegreeGenerator::s_t(s, t, k))
            .collect();
        loop {
            self.lift_product(a);
            let roots = targets
                .iter()
                .map(|&g| (Map::Product(a), g, w_ab - self.weights[&g]))
                .collect();
            match self.certify(roots) {
                Ok(()) => break,
                Err(culprit) => self.widen_past(culprit),
            }
        }
        let unit_b = self
            .module(b.s())
            .operation_generator_to_index(0, 0, b.t(), b.idx());
        let phi = &self.products[&a];
        targets
            .into_iter()
            .filter(|g| phi.cells.get(g).is_some_and(|c| c.contains(&unit_b)))
            .map(|g| (g, (w_ab - self.weights[&g]) as u32))
            .collect()
    }
}

/// [`TauLift`] for the product chain map: lift $\varphi_a$ so that $d \varphi_a = \varphi_a d$
/// over $A_C$.
///
/// The image of `g` lies in $F_{s - a_s}$ in degree $t - a_t$, and the defect in the module
/// below.
struct ProductCells<'a> {
    res: &'a MotivicResolution,
    a: BidegreeGenerator,
    /// The mod-$\tau$ image of each generator.
    seeds: &'a HashMap<BidegreeGenerator, BTreeSet<usize>>,
    /// $\varphi_a$ lifted in lower homological degrees.
    phi: &'a HashMap<BidegreeGenerator, BTreeSet<usize>>,
}

impl TauLift for ProductCells<'_> {
    fn source_weight(&self, g: BidegreeGenerator) -> i32 {
        self.res.weights[&g] - self.res.weights[&self.a]
    }

    fn seed(&self, g: BidegreeGenerator) -> BTreeSet<usize> {
        self.seeds.get(&g).cloned().unwrap_or_default()
    }

    fn defect_module(&self, g: BidegreeGenerator) -> (Arc<FreeModule<SteenrodAlgebra>>, i32) {
        (self.res.module(g.s() - self.a.s() - 1), g.t() - self.a.t())
    }

    fn defect_weight(&self, g: BidegreeGenerator, bidx: usize) -> i32 {
        self.res
            .entry_weight(g.s() - self.a.s() - 1, g.t() - self.a.t(), bidx)
    }

    fn seed_constant(&self, g: BidegreeGenerator, parity: &mut FpVector) {
        // φₐ(dg).
        let f_sm1 = self.res.module(g.s() - 1);
        let inner_out = self.res.module(g.s() - self.a.s() - 1);
        for &bidx in &self.res.differential.cells[&g] {
            self.res.compose_into(
                &f_sm1,
                g.t(),
                g.s() - 1,
                self.phi,
                self.a.t(),
                &inner_out,
                bidx,
                parity,
            );
        }
    }

    fn accumulate(&self, g: BidegreeGenerator, bidx: usize, parity: &mut FpVector) {
        // d(φₐ g).
        let out_mod = self.res.module(g.s() - self.a.s());
        let inner_out = self.res.module(g.s() - self.a.s() - 1);
        self.res.compose_into(
            &out_mod,
            g.t() - self.a.t(),
            g.s() - self.a.s(),
            &self.res.differential.cells,
            0,
            &inner_out,
            bidx,
            parity,
        );
    }

    fn solve(&self, g: BidegreeGenerator, e: &FpVector) -> Option<FpVector> {
        let (out_s, out_t) = (g.s() - self.a.s(), g.t() - self.a.t());
        let d = self.res.resolution.differential(out_s);
        let qi = d.quasi_inverse(out_t)?;
        let mut c = FpVector::new(TWO, self.res.module(out_s).dimension(out_t));
        qi.apply(c.as_slice_mut(), 1, e.as_slice());
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use sseq::coordinates::Bidegree;

    use super::*;
    use crate::motivic::tests::{h, sphere};

    #[test]
    fn hidden_tau_extension() {
        let mut res = sphere(Bidegree::n_s(8, 6));
        let h0h2 = BidegreeGenerator::s_t(2, 5, 0);
        let h1_cubed = BidegreeGenerator::s_t(3, 6, 0);

        assert_eq!(res.motivic_product(h(0), h(2)), vec![(h0h2, 0)]);
        // h₀ · h₀h₂ = τ h₁³, invisible mod τ.
        assert_eq!(res.motivic_product(h(0), h0h2), vec![(h1_cubed, 1)]);
        // The h₁-tower has no hidden τ.
        let h1_sq = BidegreeGenerator::s_t(2, 4, 0);
        assert_eq!(res.motivic_product(h(1), h(1)), vec![(h1_sq, 0)]);
        assert_eq!(res.motivic_product(h(1), h1_sq), vec![(h1_cubed, 0)]);
    }

    /// A product read past a stalled cell widens the margin and lifts again.
    #[test]
    fn a_stalled_product_widens_the_margin() {
        let mut res = sphere(Bidegree::n_s(8, 6));
        let compute = res.compute;
        let (h0h2, h1_cubed) = (
            BidegreeGenerator::s_t(2, 5, 0),
            BidegreeGenerator::s_t(3, 6, 0),
        );
        res.lift_product(h(0));
        res.products
            .get_mut(&h(0))
            .unwrap()
            .stalled
            .insert(h1_cubed, 1);
        assert_eq!(res.motivic_product(h(0), h0h2), vec![(h1_cubed, 1)]);
        assert_eq!(res.compute, compute + Bidegree::n_s(1, 0));
    }

    /// Every finished cell of $\varphi_a$ is a chain map over $A_C$ and reduces mod $\tau$ to
    /// the product map it was seeded from.
    #[test]
    fn product_lift_is_a_chain_map() {
        let mut res = sphere(Bidegree::n_s(12, 8));
        for a in [h(0), h(1)] {
            res.lift_product(a);
            let phi = &res.products[&a];
            let hom = res.ext().generator_product_map(a);
            let w_a = res.weights[&a];
            let cells = ProductCells {
                res: &res,
                a,
                seeds: &HashMap::new(),
                phi: &phi.cells,
            };
            let mut checked = 0;
            for (&g, support) in &phi.cells {
                let (out_s, out_t) = (g.s() - a.s(), g.t() - a.t());
                if out_s < 1 || phi.stalled.contains_key(&g) {
                    continue;
                }
                let (def_mod, def_t) = cells.defect_module(g);
                let mut parity = FpVector::new(TWO, def_mod.dimension(def_t));
                cells.seed_constant(g, &mut parity);
                for &bidx in support {
                    cells.accumulate(g, bidx, &mut parity);
                }
                assert!(parity.is_zero(), "dφ ≠ φd at {g:?} for {a:?}");

                let w = res.weights[&g] - w_a;
                let mod_tau: BTreeSet<usize> = support
                    .iter()
                    .copied()
                    .filter(|&bidx| res.entry_weight(out_s, out_t, bidx) == w)
                    .collect();
                let seed: BTreeSet<usize> = hom
                    .get_map(g.s())
                    .output(g.t(), g.idx())
                    .iter_nonzero()
                    .map(|(i, _)| i)
                    .collect();
                assert_eq!(mod_tau, seed, "φ mod τ at {g:?} for {a:?}");
                checked += 1;
            }
            assert!(checked > 0);
        }
    }
}
