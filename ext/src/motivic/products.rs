//! Motivic Ext products over $\mathbb{F}_2[\tau]$: lift the left-multiplication
//! chain map $\varphi_a$ to $A_C$ and read the product $a \cdot b$ off it. The
//! $\tau$-powers of the result are the hidden extensions (e.g. $h_0^2 h_2 = \tau
//! h_1^3$). The lift is the [`TauLift`] driver applied to the product chain map
//! ([`ProductCells`]).

use std::{
    collections::{BTreeSet, HashMap},
    sync::Arc,
};

use algebra::{
    CTauAlgebra,
    module::{FreeModule, Module, homomorphism::ModuleHomomorphism},
};
use fp::{prime::TWO, vector::FpVector};
use maybe_rayon::prelude::*;
use sseq::coordinates::{Bidegree, BidegreeGenerator};

use super::{Gen, MotivicResolution, TauLift};
use crate::chain_complex::ChainComplex;

impl MotivicResolution {
    /// Lift the product chain map $\varphi_a$ (left-multiplication by the generator
    /// `a`) to $A_C$ over $\mathbb{F}_2[\tau]$: `g ↦ φₐ(g)` supports keyed by source
    /// generator, computed up to source homological degree `max_s`.
    ///
    /// The mod-τ seeds are the ExtAlgebra's `ResolutionHomomorphism` (the Cτ
    /// product); the τ-corrections make it an honest chain map over $A_C$
    /// (`dφₐ = φₐd`), via the [`TauLift`] driver ([`ProductCells`]). Processed with
    /// `s` ascending, since a cell's constant defect `φₐ(dg)` reads `φₐ` at `s-1`.
    /// This is the motivic product: reducing mod τ recovers the Cτ product, and the
    /// τ-powers are the hidden extensions (e.g. `h₀²h₂ = τ·h₁³`).
    #[tracing::instrument(skip(self, a), fields(a = ?a))]
    pub(super) fn lift_product(&self, a: Gen, max_s: i32) -> HashMap<Gen, BTreeSet<usize>> {
        let wa = self.weights[&a];
        let a_deg = Bidegree::n_s(a.t - a.s, a.s);
        let hom = self
            .ext()
            .generator_product_map(BidegreeGenerator::new(a_deg, a.idx));
        hom.extend_all();

        // Mod-τ seeds: φₐ(g) mod τ = the ExtAlgebra chain map on each generator.
        // φₐ shifts degree by aₜ, so it is zero on source generators below degree aₜ.
        let mut seeds: HashMap<Gen, BTreeSet<usize>> = HashMap::new();
        for s in a.s..=max_s {
            let map = hom.get_map(s);
            for t in a.t..=(self.compute.n() + s) {
                for idx in 0..self.num_gens(s, t) {
                    let support: BTreeSet<usize> = map
                        .output(t, idx)
                        .iter_nonzero()
                        .filter(|(_, v)| *v != 0)
                        .map(|(i, _)| i)
                        .collect();
                    if !support.is_empty() {
                        seeds.insert(Gen { s, t, idx }, support);
                    }
                }
            }
        }

        // Lift, s ascending.
        let mut phi: HashMap<Gen, BTreeSet<usize>> = HashMap::new();
        for s in a.s..=max_s {
            let gens: Vec<Gen> = (a.t..=(self.compute.n() + s))
                .flat_map(|t| (0..self.num_gens(s, t)).map(move |idx| Gen { s, t, idx }))
                .collect();
            let lifted: Vec<(Gen, BTreeSet<usize>)> = gens
                .into_maybe_par_iter()
                .map(|g| {
                    let cells = ProductCells {
                        res: self,
                        a,
                        wa,
                        seeds: &seeds,
                        phi: &phi,
                    };
                    (g, cells.lift_or_seed(g))
                })
                .collect();
            phi.extend(lifted);
        }
        phi
    }

    /// The motivic product `a · b` of two resolution generators, over
    /// $\mathbb{F}_2[\tau]$: a list of `(target generator, τ-power)` at bidegree
    /// `(aₛ+bₛ, aₜ+bₜ)`. The τ-powers are the hidden extensions — 0 for a product
    /// visible mod τ (the Cτ product), positive where the Cτ product vanishes but
    /// the motivic product does not (e.g. `h₀·(h₀h₂) = τ·h₁³`).
    ///
    /// Read off the lifted chain map `lift_product`: `a·b` is the cocycle
    /// `gₖ ↦ ε_b(φₐ(gₖ))` — the coefficient of the augmentation term `1 ⊗ b` in
    /// `φₐ(gₖ)`. Since `φₐ(gₖ)` has weight `w(gₖ) − w(a)` and `1 ⊗ b` has weight
    /// `w(b)`, that coefficient's forced τ-power is `w(a) + w(b) − w(gₖ)`.
    pub fn motivic_product(&self, a: Gen, b: Gen) -> Vec<(Gen, u32)> {
        let target_s = a.s + b.s;
        let target_t = a.t + b.t;
        // Out of the stem-computed box ⇒ uncomputed, reported as the zero product.
        if target_s > self.max.s() || target_t - target_s > self.compute.n() {
            return Vec::new();
        }
        let phi = self.lift_product(a, target_s);
        let out_mod = self.module(b.s); // φₐ(gₖ) ∈ F_{bₛ} at degree bₜ
        let idx_1b = out_mod.operation_generator_to_index(0, 0, b.t, b.idx);
        let wa = self.weights[&a];
        let wb = self.weights[&b];
        (0..self.num_gens(target_s, target_t))
            .filter_map(|k| {
                let gk = Gen {
                    s: target_s,
                    t: target_t,
                    idx: k,
                };
                phi.get(&gk)
                    .filter(|support| support.contains(&idx_1b))
                    .map(|_| (gk, (wa + wb - self.weights[&gk]) as u32))
            })
            .collect()
    }
}

/// The product-lift instance of [`TauLift`]: lift the left-multiplication chain
/// map $\varphi_a$ so that `dφₐ = φₐd` over `A_C`. For a source generator `g` at
/// `(s, t)`, `φₐ(g)` lives in `F_{s−aₛ}` at degree `t−aₜ`; the defect module is
/// `F_{s−aₛ−1}`, the variable part of the defect is `d(φₐ g)`, and the constant
/// part is `φₐ(dg)`.
struct ProductCells<'a> {
    res: &'a MotivicResolution,
    a: Gen,
    wa: i32,
    /// Mod-τ seeds `φₐ(g)` from the ExtAlgebra chain map.
    seeds: &'a HashMap<Gen, BTreeSet<usize>>,
    /// `φₐ` lifted at strictly lower homological degree (read by the constant defect).
    phi: &'a HashMap<Gen, BTreeSet<usize>>,
}

impl ProductCells<'_> {
    /// Correct `φₐ(g)` if the cell admits a defect and stays in the box; else return
    /// the mod-τ seed unchanged.
    fn lift_or_seed(&self, g: Gen) -> BTreeSet<usize> {
        let out_s = g.s - self.a.s;
        let stem = g.t - g.s;
        let in_cone = stem <= self.res.max.n() + self.res.max.s();
        if out_s >= 1
            && in_cone
            && self.res.weights.contains_key(&g)
            && self.res.lifted.contains_key(&g)
        {
            self.lift_cell(g)
        } else {
            self.seed(g)
        }
    }
}

impl TauLift for ProductCells<'_> {
    fn source_weight(&self, g: Gen) -> i32 {
        // φₐ(g) has weight w(g) − w(a): the chain map for left-multiplication by `a`
        // lowers weight by w(a) (its τ⁰ entries recover the Cτ product).
        self.res.weights[&g] - self.wa
    }

    fn seed(&self, g: Gen) -> BTreeSet<usize> {
        self.seeds.get(&g).cloned().unwrap_or_default()
    }

    fn defect_module(&self, g: Gen) -> (Arc<FreeModule<CTauAlgebra>>, i32) {
        (self.res.module(g.s - self.a.s - 1), g.t - self.a.t)
    }

    fn defect_weight(&self, g: Gen, bidx: usize) -> i32 {
        self.res
            .entry_weight(g.s - self.a.s - 1, g.t - self.a.t, bidx)
    }

    fn seed_constant(&self, g: Gen, parity: &mut FpVector) {
        // The support-independent part of the defect: φₐ(dg) = Σ φₐ over the lifted
        // differential of g, landing in F_{s−aₛ−1}.
        let f_sm1 = self.res.module(g.s - 1);
        let inner_out = self.res.module(g.s - self.a.s - 1);
        for &bidx in &self.res.lifted[&g] {
            // inner = φₐ shifts degree by aₜ.
            self.res.compose_into(
                &f_sm1,
                g.t,
                g.s - 1,
                self.phi,
                self.a.t,
                &inner_out,
                bidx,
                parity,
            );
        }
    }

    fn accumulate(&self, g: Gen, bidx: usize, parity: &mut FpVector) {
        // The variable part: d(φₐ g), applying the lifted differential to the current
        // φₐ(g) support (which lives in F_{s−aₛ}). inner = d is degree-preserving.
        let out_mod = self.res.module(g.s - self.a.s);
        let inner_out = self.res.module(g.s - self.a.s - 1);
        self.res.compose_into(
            &out_mod,
            g.t - self.a.t,
            g.s - self.a.s,
            &self.res.lifted,
            0,
            &inner_out,
            bidx,
            parity,
        );
    }

    fn solve(&self, g: Gen, e: &FpVector) -> Option<FpVector> {
        let out_s = g.s - self.a.s;
        let out_t = g.t - self.a.t;
        let d = self.res.resolution.differential(out_s);
        let qi = d.quasi_inverse(out_t)?;
        let mut c = FpVector::new(TWO, self.res.module(out_s).dimension(out_t));
        qi.apply(c.as_slice_mut(), 1, e.as_slice());
        Some(c)
    }
}
