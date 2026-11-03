//! Massey products in the motivic Adams $E_2$, read off a null-homotopy lifted to $A_C$.

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
use sseq::coordinates::{Bidegree, BidegreeGenerator};

use super::{Lifted, MotivicResolution, TauLift, certify::Map, f2tau};
use crate::chain_complex::{ChainComplex, ChainHomotopy};

/// A motivic Massey product `⟨a, b, c⟩` as a coset: a representative and the
/// $\mathbb{F}_2[\tau]$-submodule `a·Ext + Ext·c` it is defined modulo.
#[derive(Debug, Clone)]
pub struct MotivicMassey {
    /// The bracket's bidegree.
    pub degree: Bidegree,
    /// A representative, as generators with the $\tau$-power each carries.
    pub representative: Vec<(BidegreeGenerator, u32)>,
    /// Generators of the indeterminacy, each in the form of `representative`.
    pub indeterminacy: Vec<Vec<(BidegreeGenerator, u32)>>,
    /// Whether the representative lies in the indeterminacy, so the bracket contains zero.
    pub is_zero: bool,
}

impl MotivicResolution {
    /// Lift the null-homotopy of $\varphi_b \varphi_a$, unless it is lifted already.
    fn lift_homotopy(&mut self, a: BidegreeGenerator, b: BidegreeGenerator) {
        self.lift_product(a);
        self.lift_product(b);
        if !self.homotopies.contains_key(&(a, b)) {
            let lifted = self.homotopy_lift(a, b);
            self.homotopies.insert((a, b), lifted);
        }
    }

    /// The null-homotopy $H$ of $\varphi_b \varphi_a$ lifted to $A_C$, through every resolved
    /// generator, so that $dH + Hd = \varphi_b \varphi_a$.
    ///
    /// The mod-$\tau$ seeds are a [`ChainHomotopy`] of the product maps, which needs `ab = 0` mod
    /// $\tau$.
    #[tracing::instrument(skip(self))]
    fn homotopy_lift(&self, a: BidegreeGenerator, b: BidegreeGenerator) -> Lifted {
        let hom_a = self.ext().generator_product_map(a);
        let hom_b = self.ext().generator_product_map(b);
        hom_a.extend_through_stem(self.compute);
        hom_b.extend_through_stem(self.compute);
        let homotopy = ChainHomotopy::new(hom_a, hom_b);
        homotopy.extend(self.compute);

        let (shift_s, shift_t) = (a.s() + b.s(), a.t() + b.t());
        let first_s = (shift_s - 1).max(0);
        let mut seeds: HashMap<BidegreeGenerator, BTreeSet<usize>> = HashMap::new();
        for s in first_s..=self.max.s() {
            let map = homotopy.homotopy(s);
            for t in shift_t..=self.compute.n() + s {
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

        let mut h = Lifted::default();
        for s in first_s..=self.max.s() {
            let gens: Vec<BidegreeGenerator> = (shift_t..=self.compute.n() + s)
                .flat_map(|t| {
                    (0..self.num_gens(s, t)).map(move |idx| BidegreeGenerator::s_t(s, t, idx))
                })
                .collect();
            let cells = HomotopyCells {
                res: self,
                a,
                b,
                seeds: &seeds,
                phi_a: &self.products[&a].cells,
                phi_b: &self.products[&b].cells,
                h: &h.cells,
            };
            let lifted: Vec<_> = gens
                .into_maybe_par_iter()
                .map(|g| {
                    // The image in F_0 has no defect to cancel.
                    let lift = if g.s() + 1 == shift_s {
                        Ok(cells.seed(g))
                    } else {
                        cells.lift_cell(g)
                    };
                    (g, lift)
                })
                .collect();
            for (g, lift) in lifted {
                h.insert(g, lift);
            }
        }
        h
    }

    /// A representative of the motivic Massey product `⟨a, b, c⟩`, as the generators of its
    /// bidegree with the $\tau$-power each carries.
    ///
    /// It is the coefficient of `1 ⊗ a` in $H(g_k)$, for $H$ the null-homotopy of $\varphi_b
    /// \varphi_c$, with $\tau$-power $w(a) + w(b) + w(c) - w(g_k)$.
    ///
    /// # Panics
    ///
    /// If the resolved module is not the unit, if the bracket lies outside `max`, or if `bc` is
    /// not zero mod $\tau$.
    pub fn motivic_massey(
        &mut self,
        a: BidegreeGenerator,
        b: BidegreeGenerator,
        c: BidegreeGenerator,
    ) -> Vec<(BidegreeGenerator, u32)> {
        self.assert_unit();
        let (s, t) = (a.s() + b.s() + c.s() - 1, a.t() + b.t() + c.t());
        assert!(
            s <= self.max.s() && t - s <= self.max.n(),
            "⟨{a:?}, {b:?}, {c:?}⟩ lies outside the box {}",
            self.max
        );
        let w_abc = self.weights[&a] + self.weights[&b] + self.weights[&c];
        let targets: Vec<BidegreeGenerator> = (0..self.num_gens(s, t))
            .map(|k| BidegreeGenerator::s_t(s, t, k))
            .collect();
        loop {
            self.lift_homotopy(c, b);
            let roots = targets
                .iter()
                .map(|&g| (Map::Homotopy(c, b), g, w_abc - self.weights[&g]))
                .collect();
            match self.certify(roots) {
                Ok(()) => break,
                Err(culprit) => self.widen_past(culprit),
            }
        }
        let unit_a = self
            .module(a.s())
            .operation_generator_to_index(0, 0, a.t(), a.idx());
        let h = &self.homotopies[&(c, b)];
        targets
            .into_iter()
            .filter(|g| h.cells.get(g).is_some_and(|cell| cell.contains(&unit_a)))
            .map(|g| (g, (w_abc - self.weights[&g]) as u32))
            .collect()
    }

    /// The motivic Massey product `⟨a, b, c⟩` as a coset (see [`MotivicMassey`]).
    ///
    /// # Panics
    ///
    /// As [`Self::motivic_massey`].
    pub fn motivic_massey_coset(
        &mut self,
        a: BidegreeGenerator,
        b: BidegreeGenerator,
        c: BidegreeGenerator,
    ) -> MotivicMassey {
        let (s, t) = (a.s() + b.s() + c.s() - 1, a.t() + b.t() + c.t());
        let representative = self.motivic_massey(a, b, c);

        let mut indeterminacy: Vec<Vec<(BidegreeGenerator, u32)>> = Vec::new();
        for (left, s_y, t_y) in [(true, s - a.s(), t - a.t()), (false, s - c.s(), t - c.t())] {
            if s_y < 0 {
                continue;
            }
            for idx in 0..self.num_gens(s_y, t_y) {
                let y = BidegreeGenerator::s_t(s_y, t_y, idx);
                let product = if left {
                    self.motivic_product(a, y)
                } else {
                    self.motivic_product(y, c)
                };
                if !product.is_empty() {
                    indeterminacy.push(product);
                }
            }
        }

        let columns = self.num_gens(s, t);
        let to_vec = |terms: &[(BidegreeGenerator, u32)]| -> Vec<u128> {
            let mut v = vec![0u128; columns];
            for &(g, power) in terms {
                v[g.idx()] ^= 1u128 << power;
            }
            v
        };
        let rows: Vec<Vec<u128>> = indeterminacy.iter().map(|terms| to_vec(terms)).collect();
        let is_zero = f2tau::reduce_mod(rows, to_vec(&representative))
            .iter()
            .all(|&x| x == 0);

        MotivicMassey {
            degree: Bidegree::s_t(s, t),
            representative,
            indeterminacy,
            is_zero,
        }
    }
}

/// [`TauLift`] for the null-homotopy $H$ of $\varphi_b \varphi_a$: lift it so that
/// $dH + Hd = \varphi_b \varphi_a$ over $A_C$.
///
/// The image of `g` lies in $F_{s + 1 - a_s - b_s}$ in degree $t - a_t - b_t$, and the defect in
/// the module below.
struct HomotopyCells<'a> {
    res: &'a MotivicResolution,
    a: BidegreeGenerator,
    b: BidegreeGenerator,
    /// The mod-$\tau$ image of each generator.
    seeds: &'a HashMap<BidegreeGenerator, BTreeSet<usize>>,
    /// The lifted $\varphi_a$.
    phi_a: &'a HashMap<BidegreeGenerator, BTreeSet<usize>>,
    /// The lifted $\varphi_b$.
    phi_b: &'a HashMap<BidegreeGenerator, BTreeSet<usize>>,
    /// $H$ lifted in lower homological degrees.
    h: &'a HashMap<BidegreeGenerator, BTreeSet<usize>>,
}

impl HomotopyCells<'_> {
    /// The homological degree $H$ lowers by, less one.
    fn shift_s(&self) -> i32 {
        self.a.s() + self.b.s()
    }

    /// The internal degree $H$ lowers by.
    fn shift_t(&self) -> i32 {
        self.a.t() + self.b.t()
    }
}

impl TauLift for HomotopyCells<'_> {
    fn source_weight(&self, g: BidegreeGenerator) -> i32 {
        self.res.weights[&g] - self.res.weights[&self.a] - self.res.weights[&self.b]
    }

    fn seed(&self, g: BidegreeGenerator) -> BTreeSet<usize> {
        self.seeds.get(&g).cloned().unwrap_or_default()
    }

    fn defect_module(&self, g: BidegreeGenerator) -> (Arc<FreeModule<SteenrodAlgebra>>, i32) {
        (
            self.res.module(g.s() - self.shift_s()),
            g.t() - self.shift_t(),
        )
    }

    fn defect_weight(&self, g: BidegreeGenerator, bidx: usize) -> i32 {
        self.res
            .entry_weight(g.s() - self.shift_s(), g.t() - self.shift_t(), bidx)
    }

    fn seed_constant(&self, g: BidegreeGenerator, parity: &mut FpVector) {
        let def_mod = self.res.module(g.s() - self.shift_s());
        // H(dg).
        let f_sm1 = self.res.module(g.s() - 1);
        for &bidx in &self.res.differential.cells[&g] {
            self.res.compose_into(
                &f_sm1,
                g.t(),
                g.s() - 1,
                self.h,
                self.shift_t(),
                &def_mod,
                bidx,
                parity,
            );
        }
        // φ_b(φ_a g).
        if let Some(phi_a_g) = self.phi_a.get(&g) {
            let a_mod = self.res.module(g.s() - self.a.s());
            for &bidx in phi_a_g {
                self.res.compose_into(
                    &a_mod,
                    g.t() - self.a.t(),
                    g.s() - self.a.s(),
                    self.phi_b,
                    self.b.t(),
                    &def_mod,
                    bidx,
                    parity,
                );
            }
        }
    }

    fn accumulate(&self, g: BidegreeGenerator, bidx: usize, parity: &mut FpVector) {
        // d(Hg).
        let out_mod = self.res.module(g.s() + 1 - self.shift_s());
        let inner_out = self.res.module(g.s() - self.shift_s());
        self.res.compose_into(
            &out_mod,
            g.t() - self.shift_t(),
            g.s() + 1 - self.shift_s(),
            &self.res.differential.cells,
            0,
            &inner_out,
            bidx,
            parity,
        );
    }

    fn solve(&self, g: BidegreeGenerator, e: &FpVector) -> Option<FpVector> {
        let (out_s, out_t) = (g.s() + 1 - self.shift_s(), g.t() - self.shift_t());
        let d = self.res.resolution.differential(out_s);
        let qi = d.quasi_inverse(out_t)?;
        let mut c = FpVector::new(TWO, self.res.module(out_s).dimension(out_t));
        qi.apply(c.as_slice_mut(), 1, e.as_slice());
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::motivic::tests::{h, sphere};

    #[test]
    fn massey_products_with_tau() {
        let mut res = sphere(Bidegree::n_s(10, 7));
        let h0h2 = res.motivic_product(h(0), h(2))[0].0;
        let h1_sq = res.motivic_product(h(1), h(1))[0].0;

        assert_eq!(res.motivic_massey(h(1), h(0), h(1)), vec![(h0h2, 0)]);
        // ⟨h₀, h₁, h₀⟩ = τ h₁², which vanishes mod τ.
        assert_eq!(res.motivic_massey(h(0), h(1), h(0)), vec![(h1_sq, 1)]);

        for (a, b, c) in [(h(1), h(0), h(1)), (h(0), h(1), h(0))] {
            let coset = res.motivic_massey_coset(a, b, c);
            assert!(coset.indeterminacy.is_empty());
            assert!(!coset.is_zero);
        }
    }

    /// Every finished cell of the null-homotopy satisfies $dH + Hd = \varphi_b \varphi_a$ over
    /// $A_C$.
    #[test]
    fn homotopy_lift_is_a_null_homotopy() {
        let mut res = sphere(Bidegree::n_s(12, 8));
        let (a, b) = (h(0), h(1));
        res.lift_homotopy(a, b);
        let lifted = &res.homotopies[&(a, b)];
        let cells = HomotopyCells {
            res: &res,
            a,
            b,
            seeds: &HashMap::new(),
            phi_a: &res.products[&a].cells,
            phi_b: &res.products[&b].cells,
            h: &lifted.cells,
        };
        let mut checked = 0;
        for (&g, support) in &lifted.cells {
            if g.s() + 1 - cells.shift_s() < 1 || lifted.stalled.contains_key(&g) {
                continue;
            }
            let (def_mod, def_t) = cells.defect_module(g);
            let mut parity = FpVector::new(TWO, def_mod.dimension(def_t));
            cells.seed_constant(g, &mut parity);
            for &bidx in support {
                cells.accumulate(g, bidx, &mut parity);
            }
            assert!(parity.is_zero(), "dH + Hd ≠ φφ at {g:?}");
            checked += 1;
        }
        assert!(checked > 0);
    }
}
