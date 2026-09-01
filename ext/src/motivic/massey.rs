//! Motivic Massey products `⟨a, b, c⟩` over $\mathbb{F}_2[\tau]$: lift the
//! null-homotopy `H` of `φ_b ∘ φ_c` to $A_C$ (`dH + Hd = φ_bφ_c`) via the third
//! [`TauLift`] instance ([`NullHomotopyCells`]) and read the bracket off the `1 ⊗ a`
//! augmentation. The full coset ([`MotivicMassey`]) carries the indeterminacy
//! `a·Ext + Ext·c`, reduced over $\mathbb{F}_2[\tau]$ to decide triviality.

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

use super::{Gen, MotivicResolution, TauLift, f2tau};
use crate::chain_complex::{ChainComplex, ChainHomotopy};

/// A motivic Massey product `⟨a, b, c⟩` as a coset in the motivic Ext: a
/// representative together with the $\mathbb{F}_2[\tau]$-submodule of indeterminacy
/// `a·Ext + Ext·c` it is defined modulo. Produced by
/// [`MotivicResolution::motivic_massey_coset`].
#[derive(Debug, Clone)]
pub struct MotivicMassey {
    /// The bracket bidegree `(aₛ+bₛ+cₛ−1, aₜ+bₜ+cₜ)`.
    pub degree: Bidegree,
    /// A representative, as `(target generator, τ-power)` terms.
    pub representative: Vec<(Gen, u32)>,
    /// $\mathbb{F}_2[\tau]$-module generators of the indeterminacy.
    pub indeterminacy: Vec<Vec<(Gen, u32)>>,
    /// Whether the bracket contains zero — the representative lies in the
    /// indeterminacy submodule (so the bracket is the trivial coset).
    pub is_zero: bool,
}

impl MotivicResolution {
    /// Lift the null-homotopy `H` of `φ_b ∘ φ_a` (the product `ab`, which must vanish
    /// mod τ) to $A_C$ over $\mathbb{F}_2[\tau]$: `g ↦ H(g)` supports keyed by source
    /// generator, up to source degree `max_s`. The mod-τ seed is the ExtAlgebra
    /// [`ChainHomotopy`]; the τ-corrections make `dH + Hd = φ_bφ_a` hold over $A_C$,
    /// via the third [`TauLift`] instance ([`NullHomotopyCells`]). This is the datum
    /// a Massey product `⟨a, b, c⟩` is built from.
    #[tracing::instrument(skip(self, a, b), fields(a = ?a, b = ?b))]
    pub(super) fn lift_nullhomotopy(
        &self,
        a: Gen,
        b: Gen,
        max_s: i32,
    ) -> HashMap<Gen, BTreeSet<usize>> {
        let (wa, wb) = (self.weights[&a], self.weights[&b]);
        let phi_a = self.lift_product(a, max_s);
        let phi_b = self.lift_product(b, max_s);

        let hom_a = self
            .ext()
            .generator_product_map(BidegreeGenerator::new(Bidegree::n_s(a.t - a.s, a.s), a.idx));
        let hom_b = self
            .ext()
            .generator_product_map(BidegreeGenerator::new(Bidegree::n_s(b.t - b.s, b.s), b.idx));
        // Extend only to the box we lift over (stem ≤ compute.n(), s ≤ max_s); the
        // resolution is stem-computed, so extend_all would over-reach into unresolved
        // bidegrees.
        let box_max = Bidegree::n_s(self.compute.n(), max_s);
        hom_a.extend_through_stem(box_max);
        hom_b.extend_through_stem(box_max);
        let ch = ChainHomotopy::new(hom_a, hom_b); // null-homotopy of φ_b ∘ φ_a
        ch.extend(box_max);

        let shift_s = a.s + b.s;
        let shift_t = a.t + b.t;

        // Mod-τ seeds: H(g) mod τ ∈ F_{s+1−shiftₛ} at degree t−shiftₜ.
        let mut seeds: HashMap<Gen, BTreeSet<usize>> = HashMap::new();
        for s in (shift_s - 1).max(0)..=max_s {
            let map = ch.homotopy(s);
            for t in shift_t..=(self.compute.n() + s) {
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

        // Lift, s ascending (the constant defect Hd reads H at s−1).
        let mut h_phi: HashMap<Gen, BTreeSet<usize>> = HashMap::new();
        for s in (shift_s - 1).max(0)..=max_s {
            let gens: Vec<Gen> = (shift_t..=(self.compute.n() + s))
                .flat_map(|t| (0..self.num_gens(s, t)).map(move |idx| Gen { s, t, idx }))
                .collect();
            let lifted: Vec<(Gen, BTreeSet<usize>)> = gens
                .into_maybe_par_iter()
                .map(|g| {
                    let cells = NullHomotopyCells {
                        res: self,
                        a,
                        b,
                        wa,
                        wb,
                        seeds: &seeds,
                        phi_a: &phi_a,
                        phi_b: &phi_b,
                        h_phi: &h_phi,
                    };
                    (g, cells.lift_or_seed(g))
                })
                .collect();
            h_phi.extend(lifted);
        }
        h_phi
    }

    /// The motivic Massey product `⟨a, b, c⟩` of three generators (requires
    /// `ab = 0` and `bc = 0` mod τ), over $\mathbb{F}_2[\tau]$: a list of
    /// `(target generator, τ-power)` at bidegree `(aₛ+bₛ+cₛ−1, aₜ+bₜ+cₜ)`. A
    /// representative modulo the indeterminacy `a·⟨·⟩ + ⟨·⟩·c`.
    ///
    /// Read off the lifted null-homotopy `H` of `φ_b ∘ φ_c` (`lift_nullhomotopy`):
    /// evaluated at the top degree `H(gₖ)` lands in `F_{aₛ}` at degree `aₜ`, and the
    /// bracket is the coefficient of the augmentation `1 ⊗ a`, whose forced τ-power is
    /// `w(a) + w(b) + w(c) − w(gₖ)`.
    pub fn motivic_massey(&self, a: Gen, b: Gen, c: Gen) -> Vec<(Gen, u32)> {
        let tot_s = a.s + b.s + c.s - 1;
        let tot_t = a.t + b.t + c.t;
        if tot_s > self.max.s() || tot_t - tot_s > self.compute.n() {
            return Vec::new();
        }
        let h = self.lift_nullhomotopy(c, b, tot_s);
        let a_mod = self.module(a.s);
        let idx_1a = a_mod.operation_generator_to_index(0, 0, a.t, a.idx);
        let wsum = self.weights[&a] + self.weights[&b] + self.weights[&c];
        (0..self.num_gens(tot_s, tot_t))
            .filter_map(|k| {
                let gk = Gen {
                    s: tot_s,
                    t: tot_t,
                    idx: k,
                };
                h.get(&gk)
                    .filter(|support| support.contains(&idx_1a))
                    .map(|_| (gk, (wsum - self.weights[&gk]) as u32))
            })
            .collect()
    }

    /// The motivic Massey product `⟨a, b, c⟩` as a full coset (see [`MotivicMassey`]):
    /// the [representative](Self::motivic_massey) together with its indeterminacy
    /// `a·Ext^{tot−|a|} + Ext^{tot−|c|}·c` (an $\mathbb{F}_2[\tau]$-submodule), and
    /// whether the bracket is the trivial coset. The representative is reduced
    /// against the indeterminacy over $\mathbb{F}_2[\tau]$ to decide `is_zero`.
    pub fn motivic_massey_coset(&self, a: Gen, b: Gen, c: Gen) -> MotivicMassey {
        let tot_s = a.s + b.s + c.s - 1;
        let tot_t = a.t + b.t + c.t;
        let representative = self.motivic_massey(a, b, c);
        let ncols = self.num_gens(tot_s, tot_t);

        // Indeterminacy generators: a·y for y ∈ Ext^{tot−|a|}, and x·c for
        // x ∈ Ext^{tot−|c|}.
        let mut indeterminacy: Vec<Vec<(Gen, u32)>> = Vec::new();
        let (ya_s, ya_t) = (tot_s - a.s, tot_t - a.t);
        if ya_s >= 0 {
            for idx in 0..self.num_gens(ya_s, ya_t) {
                let prod = self.motivic_product(
                    a,
                    Gen {
                        s: ya_s,
                        t: ya_t,
                        idx,
                    },
                );
                if !prod.is_empty() {
                    indeterminacy.push(prod);
                }
            }
        }
        let (xc_s, xc_t) = (tot_s - c.s, tot_t - c.t);
        if xc_s >= 0 {
            for idx in 0..self.num_gens(xc_s, xc_t) {
                let prod = self.motivic_product(
                    Gen {
                        s: xc_s,
                        t: xc_t,
                        idx,
                    },
                    c,
                );
                if !prod.is_empty() {
                    indeterminacy.push(prod);
                }
            }
        }

        // Reduce the representative modulo the indeterminacy over F₂[τ]: pack each
        // term list into a coefficient vector (τ-powers as F₂[τ] monomials).
        let to_vec = |terms: &[(Gen, u32)]| -> Vec<u128> {
            let mut v = vec![0u128; ncols];
            for &(g, p) in terms {
                v[g.idx] ^= 1u128 << p;
            }
            v
        };
        let rows: Vec<Vec<u128>> = indeterminacy.iter().map(|t| to_vec(t)).collect();
        let remainder = f2tau::reduce_mod(rows, to_vec(&representative));
        let is_zero = remainder.iter().all(|&x| x == 0);

        MotivicMassey {
            degree: Bidegree::n_s(tot_t - tot_s, tot_s),
            representative,
            indeterminacy,
            is_zero,
        }
    }
}

/// The null-homotopy-lift instance of [`TauLift`]: lift `H` (a null-homotopy of
/// `φ_b ∘ φ_a`) so that `dH + Hd = φ_bφ_a` over `A_C`. For a source generator `g`
/// at `(s, t)`, `H(g)` lives in `F_{s+1−aₛ−bₛ}` at degree `t−aₜ−bₜ`; the defect
/// module is `F_{s−aₛ−bₛ}`, the variable part of the defect is `d(Hg)`, and the
/// constant part is `H(dg) + φ_b(φ_a g)`.
struct NullHomotopyCells<'a> {
    res: &'a MotivicResolution,
    a: Gen,
    b: Gen,
    wa: i32,
    wb: i32,
    /// Mod-τ seeds `H(g)` from the ExtAlgebra chain homotopy.
    seeds: &'a HashMap<Gen, BTreeSet<usize>>,
    phi_a: &'a HashMap<Gen, BTreeSet<usize>>,
    phi_b: &'a HashMap<Gen, BTreeSet<usize>>,
    /// `H` lifted at strictly lower homological degree (read by `H(dg)`).
    h_phi: &'a HashMap<Gen, BTreeSet<usize>>,
}

impl NullHomotopyCells<'_> {
    fn shift_s(&self) -> i32 {
        self.a.s + self.b.s
    }

    fn shift_t(&self) -> i32 {
        self.a.t + self.b.t
    }

    fn lift_or_seed(&self, g: Gen) -> BTreeSet<usize> {
        let out_s = g.s + 1 - self.shift_s();
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

impl TauLift for NullHomotopyCells<'_> {
    fn source_weight(&self, g: Gen) -> i32 {
        // H(g) has weight w(g) − w(a) − w(b) (it witnesses the weight-(wₐ+w_b) product).
        self.res.weights[&g] - self.wa - self.wb
    }

    fn seed(&self, g: Gen) -> BTreeSet<usize> {
        self.seeds.get(&g).cloned().unwrap_or_default()
    }

    fn defect_module(&self, g: Gen) -> (Arc<FreeModule<CTauAlgebra>>, i32) {
        (self.res.module(g.s - self.shift_s()), g.t - self.shift_t())
    }

    fn defect_weight(&self, g: Gen, bidx: usize) -> i32 {
        self.res
            .entry_weight(g.s - self.shift_s(), g.t - self.shift_t(), bidx)
    }

    fn seed_constant(&self, g: Gen, parity: &mut FpVector) {
        let def_mod = self.res.module(g.s - self.shift_s());
        // H(dg): apply H to the lifted differential of g (inner = H shifts by shiftₜ).
        let f_sm1 = self.res.module(g.s - 1);
        for &bidx in &self.res.lifted[&g] {
            self.res.compose_into(
                &f_sm1,
                g.t,
                g.s - 1,
                self.h_phi,
                self.shift_t(),
                &def_mod,
                bidx,
                parity,
            );
        }
        // φ_b(φ_a g): apply φ_b to the lifted φ_a(g) (inner = φ_b shifts by bₜ).
        if let Some(pa) = self.phi_a.get(&g) {
            let a_mod = self.res.module(g.s - self.a.s);
            for &bidx in pa {
                self.res.compose_into(
                    &a_mod,
                    g.t - self.a.t,
                    g.s - self.a.s,
                    self.phi_b,
                    self.b.t,
                    &def_mod,
                    bidx,
                    parity,
                );
            }
        }
    }

    fn accumulate(&self, g: Gen, bidx: usize, parity: &mut FpVector) {
        // d(Hg): apply the lifted differential to the current H(g) support, which
        // lives in F_{s+1−shiftₛ}. inner = d is degree-preserving.
        let out_mod = self.res.module(g.s + 1 - self.shift_s());
        let inner_out = self.res.module(g.s - self.shift_s());
        self.res.compose_into(
            &out_mod,
            g.t - self.shift_t(),
            g.s + 1 - self.shift_s(),
            &self.res.lifted,
            0,
            &inner_out,
            bidx,
            parity,
        );
    }

    fn solve(&self, g: Gen, e: &FpVector) -> Option<FpVector> {
        let out_s = g.s + 1 - self.shift_s();
        let out_t = g.t - self.shift_t();
        let d = self.res.resolution.differential(out_s);
        let qi = d.quasi_inverse(out_t)?;
        let mut c = FpVector::new(TWO, self.res.module(out_s).dimension(out_t));
        qi.apply(c.as_slice_mut(), 1, e.as_slice());
        Some(c)
    }
}
