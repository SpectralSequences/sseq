//! The C-motivic resolution: resolve over $A_C/\tau$, then lift the differentials to $A_C$.

use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    sync::Arc,
};

use algebra::{
    SteenrodAlgebra, SteenrodKind,
    module::{FreeModule, Module, homomorphism::ModuleHomomorphism},
    motivic::MotivicMilnorAlgebra,
};
use fp::{prime::TWO, vector::FpVector};
use maybe_rayon::prelude::*;
use sseq::coordinates::Bidegree;

use crate::{CCC, chain_complex::ChainComplex, resolution::Resolution};

/// A generator of the resolution, identified by homological degree `s`, internal
/// degree `t`, and index within that `(s, t)` bidegree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Gen {
    pub s: i32,
    pub t: i32,
    pub idx: usize,
}

/// The C-motivic resolution: a minimal resolution over $A_C/\tau$, lifted to $A_C$.
///
/// $A_C/\tau$ is bigraded by stem and motivic weight but resolved by the ordinary engine, which
/// sees the stem alone. The minimal resolution still comes out weight-homogeneous, with each
/// generator's weight found by descending the differential
/// ([`MotivicResolution::generator_weight`]). Homogeneity forces the $\tau$-power of every entry:
/// a term $m \otimes g_j$ of $d_s(g_k)$ carries $\tau^{\,w(g_k) - w(m) - w(g_j)}$. So a lifted
/// differential is stored as an $\mathbb{F}_2$ support plus the weights, and its $\tau$-powers are
/// reconstructed on demand.
pub struct MotivicResolution {
    /// The $A_C$ product engine, which recovers the $\tau$-powers the resolution discards. It
    /// indexes each degree as $A_C/\tau$ does, so the resolution's operation indices are its own.
    engine: MotivicMilnorAlgebra,
    resolution: Arc<Resolution<CCC>>,
    /// Motivic weight of each generator.
    weights: Arc<HashMap<Gen, i32>>,
    /// The lifted $A_C$ differential of each generator, as its support in $F_{s-1}$. Each
    /// coefficient is 1 and its $\tau$-power is forced by the weights.
    lifted: HashMap<Gen, BTreeSet<usize>>,
    /// The box results are reported in.
    max: Bidegree,
    /// The box actually resolved: `max` with a stem margin for the reach of δ (see
    /// [`Self::new`]).
    compute: Bidegree,
}

impl MotivicResolution {
    /// Resolve through the box `max` and lift the resolution to $A_C$.
    ///
    /// `resolution` is an ordinary resolution over all of $A_C/\tau$, such as
    /// [`construct_standard`](crate::utils::construct_standard) builds from the `Ctau` module.
    ///
    /// # Errors
    ///
    /// If `resolution` is not over all of $A_C/\tau$, or if its module is not cyclic on a single
    /// class in degree 0, the only cell the lift seeds a weight on.
    #[tracing::instrument(skip(resolution), fields(max = %max, compute = tracing::field::Empty))]
    pub fn new(resolution: Arc<Resolution<CCC>>, max: Bidegree) -> anyhow::Result<Self> {
        let algebra = resolution.algebra();
        let full_ctau = match &*algebra {
            SteenrodAlgebra::MilnorAlgebra(a) => {
                a.kind() == SteenrodKind::MotivicModTau && a.profile().is_trivial()
            }
            SteenrodAlgebra::AdemAlgebra(_) => false,
        };
        anyhow::ensure!(
            full_ctau,
            "the motivic lift needs a resolution over all of A_C/τ, not {algebra}"
        );
        // δ maps `(s, t)` to `(s - 1, t)`, one stem up, so Ext at stem `n` reads the generators at
        // stem `n + 1` and nothing beyond. A margin of one stem is therefore exact; `MOT_MARGIN`
        // only widens it.
        let margin: i32 = std::env::var("MOT_MARGIN")
            .ok()
            .and_then(|v| v.parse::<i32>().ok())
            .map(|m| m.max(1))
            .unwrap_or(1);
        let compute = Bidegree::n_s(max.n() + margin, max.s());
        tracing::Span::current().record("compute", tracing::field::display(compute));
        let profile = std::env::var("MOT_PROFILE").is_ok();
        let t0 = std::time::Instant::now();
        resolution.compute_through_stem(compute);
        let engine = MotivicMilnorAlgebra::new();
        engine.compute_basis(compute.n() + compute.s());
        if profile {
            eprintln!("[profile] resolution: {:?}", t0.elapsed());
        }

        let mut this = Self {
            engine,
            resolution,
            weights: Arc::new(HashMap::new()),
            lifted: HashMap::new(),
            max,
            compute,
        };
        let t1 = std::time::Instant::now();
        this.compute_weights()?;
        if profile {
            eprintln!("[profile] weights:    {:?}", t1.elapsed());
        }
        let t2 = std::time::Instant::now();
        this.lift();
        if profile {
            eprintln!("[profile] lift:       {:?}", t2.elapsed());
        }
        Ok(this)
    }

    /// The resolution over $A_C/\tau$.
    pub fn resolution(&self) -> &Resolution<CCC> {
        &self.resolution
    }

    /// The box results are reported in.
    pub fn max(&self) -> Bidegree {
        self.max
    }

    /// The algebraic Novikov $E_2$ rank at `(s, t)`: the number of generators, since δ vanishes
    /// mod $\tau$.
    pub fn algebraic_novikov_rank(&self, s: i32, t: i32) -> usize {
        self.num_gens(s, t)
    }

    /// The maximum homological degree computed.
    fn max_s(&self) -> i32 {
        self.max.s()
    }

    /// The free module $F_s$.
    fn module(&self, s: i32) -> Arc<FreeModule<SteenrodAlgebra>> {
        self.resolution.module(s)
    }

    /// The motivic weight of a generator (panics if out of the computed range or
    /// if the generator's weight could not be determined).
    pub fn generator_weight(&self, g: Gen) -> i32 {
        self.weights[&g]
    }

    /// Call `f` on the terms of the $A_C$ product of the operations `(t1, idx1)` and `(t2, idx2)`,
    /// in that order as $A_C/\tau$ multiplies them.
    ///
    /// The engine orders its factors oppositely to [`MilnorAlgebra`](algebra::MilnorAlgebra), so this asks it for the
    /// transposed product, which is $A_C$'s in [`MilnorAlgebra`](algebra::MilnorAlgebra)'s convention.
    fn product_with<R>(
        &self,
        t1: i32,
        idx1: usize,
        t2: i32,
        idx2: usize,
        f: impl FnOnce(&[usize]) -> R,
    ) -> R {
        self.engine.product_indexed_with(t2, idx2, t1, idx1, f)
    }

    /// The motivic weight of the `idx`-th operation in degree `t`.
    fn op_weight(&self, t: i32, idx: usize) -> i32 {
        self.engine.bidegree(t, idx).1
    }

    /// The number of generators in bidegree `(s, t)`.
    fn num_gens(&self, s: i32, t: i32) -> usize {
        self.module(s).number_of_gens_in_degree(t)
    }

    /// The weight of an $F_s$ basis element `bidx` in degree `t`: decode it into
    /// `operation ⊗ generator` and add the operation's weight to the generator's.
    fn entry_weight(&self, s: i32, t: i32, bidx: usize) -> i32 {
        let module = self.module(s);
        let og = module.index_to_op_gen(t, bidx);
        let op_w = self.op_weight(og.operation_degree, og.operation_index);
        let gen_w = self.weights[&Gen {
            s,
            t: og.generator_degree,
            idx: og.generator_index,
        }];
        op_w + gen_w
    }

    /// Lift every differential to $A_C$, with `s` ascending so that `d_{s-1}` is final before
    /// `d_s` is corrected against it.
    ///
    /// Each $d_s$ starts as its mod-$\tau$ reduction. Over $A_C$ the composite $d_{s-1} d_s$ is
    /// zero mod $\tau$ but not exactly zero, and the remainder is cancelled one $\tau$-order at a
    /// time by [`TauLift::lift_cell`]. `d_1` needs no correction: `d_0` is the augmentation and
    /// `d_1` lands in the augmentation ideal.
    #[tracing::instrument(skip(self), fields(max = %self.max, num_lifted = tracing::field::Empty))]
    fn lift(&mut self) {
        // Correcting a generator reads only `s - 1` data, so the generators at one `s` are
        // independent.
        for s in 1..=self.max_s() {
            let t_max = self.compute.n() + s;
            let gens: Vec<Gen> = (0..=t_max)
                .flat_map(|t| (0..self.num_gens(s, t)).map(move |idx| Gen { s, t, idx }))
                .collect();
            let _span = tracing::info_span!("lift_s", s, num_gens = gens.len()).entered();
            let lifted: Vec<(Gen, BTreeSet<usize>)> = gens
                .into_maybe_par_iter()
                .map(|g| (g, self.lift_generator(g)))
                .collect();
            self.lifted.extend(lifted);
        }
        tracing::Span::current().record("num_lifted", self.lifted.len());
    }

    /// Lift one generator's differential to $A_C$.
    ///
    /// The corrections of a generator at stem `n` reach up to stem `n + s`, so one past the
    /// report box's `n + s` cannot converge. Nothing in the report box reads it, so it keeps its
    /// mod-$\tau$ support.
    fn lift_generator(&self, g: Gen) -> BTreeSet<usize> {
        let cells = DifferentialCells(self);
        let stem = g.t - g.s;
        let in_cone = stem <= self.max.n() + self.max.s();
        if g.s >= 2 && in_cone && self.weights.contains_key(&g) {
            cells.lift_cell(g)
        } else {
            cells.seed(g)
        }
    }

    /// XOR the contribution of the $F_{s-1}$ basis element `bidx` of `d_s(g_k)` into a running
    /// `d_{s-1} d_s` parity vector over $F_{s-2}$ in degree `t`.
    ///
    /// The composite is linear in the support and each output element's $\tau$-power depends on
    /// that element alone, so toggling a support term in or out is one call here.
    #[allow(clippy::too_many_arguments)]
    fn accumulate_term(
        &self,
        s: i32,
        t: i32,
        f_sm1: &FreeModule<SteenrodAlgebra>,
        f_sm2: &FreeModule<SteenrodAlgebra>,
        _engine: &MotivicMilnorAlgebra,
        bidx: usize,
        parity: &mut FpVector,
    ) {
        // The differential preserves degree, hence an inner shift of 0.
        self.compose_into(f_sm1, t, s - 1, &self.lifted, 0, f_sm2, bidx, parity);
    }

    /// Compose the basis element `bidx` of `outer_mod` with the lifted map `inner`, XORing the
    /// resulting $A_C$ products into `parity`.
    ///
    /// `bidx` is `m ⊗ gⱼ` in internal degree `outer_t`, with `gⱼ` in homological degree
    /// `outer_s`. Each `m′ ⊗ gₗ` of `inner[gⱼ]`, which lives in `inner_out_mod`, contributes
    /// every term of `(m·m′) ⊗ gₗ`; their $\tau$-powers are recovered later from the weights. A
    /// generator absent from `inner` contributes nothing.
    #[allow(clippy::too_many_arguments)]
    fn compose_into(
        &self,
        outer_mod: &FreeModule<SteenrodAlgebra>,
        outer_t: i32,
        outer_s: i32,
        inner: &HashMap<Gen, BTreeSet<usize>>,
        inner_shift_t: i32,
        inner_out_mod: &FreeModule<SteenrodAlgebra>,
        bidx: usize,
        parity: &mut FpVector,
    ) {
        let og = outer_mod.index_to_op_gen(outer_t, bidx);
        let (m_deg, m_idx) = (og.operation_degree, og.operation_index);
        let gj = Gen {
            s: outer_s,
            t: og.generator_degree,
            idx: og.generator_index,
        };
        let Some(gj_inner) = inner.get(&gj) else {
            return;
        };
        let inner_t = gj.t - inner_shift_t;
        for &bidx2 in gj_inner {
            let og2 = inner_out_mod.index_to_op_gen(inner_t, bidx2);
            let (mp_deg, mp_idx) = (og2.operation_degree, og2.operation_index);
            let (gl_deg, gl_idx) = (og2.generator_degree, og2.generator_index);
            let z_deg = m_deg + mp_deg;
            self.product_with(m_deg, m_idx, mp_deg, mp_idx, |terms| {
                for &z_idx in terms {
                    let fidx =
                        inner_out_mod.operation_generator_to_index(z_deg, z_idx, gl_deg, gl_idx);
                    parity.add_basis_element(fidx, 1); // XOR at p = 2
                }
            });
        }
    }

    /// The composite `d_{s-1} d_s(g_k)` over $A_C$, as its surviving $F_{s-2}$ basis elements in
    /// degree `g_k.t`, each with its forced $\tau$-power.
    fn composite(&self, g_k: Gen, support: &BTreeSet<usize>) -> BTreeMap<usize, i32> {
        let s = g_k.s;
        let t = g_k.t;
        let w_k = self.weights[&g_k];
        let f_sm1 = self.module(s - 1);
        let f_sm2 = self.module(s - 2);
        let engine = &self.engine;

        let mut parity = FpVector::new(TWO, f_sm2.dimension(t));
        for &bidx in support {
            self.accumulate_term(s, t, &f_sm1, &f_sm2, engine, bidx, &mut parity);
        }

        parity
            .iter_nonzero()
            .map(|(fidx, _)| {
                // Weight-homogeneity gives every path to `fidx` this same τ-power, so the
                // parity is well defined.
                let power = self.entry_weight(s - 2, t, fidx) - w_k;
                (fidx, power)
            })
            .collect()
    }

    /// Assign a motivic weight to every generator by descending its differential.
    ///
    /// Weight-homogeneity forces `w(g) = w(m) + w(g')` for every term `m ⊗ g'` of `d(g)`,
    /// starting from weight 0 on the degree-0 generator.
    ///
    /// # Errors
    ///
    /// If the module is not cyclic on a single degree-0 class.
    ///
    /// # Panics
    ///
    /// If a generator is not weight-homogeneous.
    #[tracing::instrument(skip(self), fields(num_weights = tracing::field::Empty))]
    fn compute_weights(&mut self) -> anyhow::Result<()> {
        let mut weights: HashMap<Gen, i32> = HashMap::new();
        // Only the degree-0 generator is seeded, so any other `s = 0` generator would have no
        // weight to propagate from.
        for t in 0..=self.compute.n() {
            let expected = usize::from(t == 0);
            let actual = self.num_gens(0, t);
            if actual != expected {
                anyhow::bail!(
                    "the motivic lift supports modules cyclic on a degree-0 class; this one needs \
                     {actual} generator(s) at (s = 0, t = {t})"
                );
            }
        }
        weights.insert(Gen { s: 0, t: 0, idx: 0 }, 0);

        for s in 1..=self.max_s() {
            let d = self.resolution.differential(s);
            let target = self.module(s - 1);
            let t_max = self.compute.n() + s;
            for t in 0..=t_max {
                for idx in 0..self.num_gens(s, t) {
                    let out = d.output(t, idx);
                    let mut weight: Option<i32> = None;
                    for (bidx, v) in out.iter_nonzero() {
                        if v == 0 {
                            continue;
                        }
                        let og = target.index_to_op_gen(t, bidx);
                        let op_w = self.op_weight(og.operation_degree, og.operation_index);
                        let tgt = Gen {
                            s: s - 1,
                            t: og.generator_degree,
                            idx: og.generator_index,
                        };
                        let Some(&tgt_w) = weights.get(&tgt) else {
                            continue;
                        };
                        let w = op_w + tgt_w;
                        match weight {
                            None => weight = Some(w),
                            Some(w0) => assert_eq!(
                                w0, w,
                                "weight-inhomogeneous generator at (s={s}, t={t}, idx={idx})"
                            ),
                        }
                    }
                    if let Some(w) = weight {
                        weights.insert(Gen { s, t, idx }, w);
                    }
                }
            }
        }
        tracing::Span::current().record("num_weights", weights.len());
        self.weights = Arc::new(weights);
        Ok(())
    }

    /// The δ-entries out of generator `g`: the unit-operation part of the lifted `d_s(g)`.
    ///
    /// Each entry is a generator `g'` in `(s - 1, t)` with the $\tau$-power on the unit. δ is a
    /// differential on the free $\mathbb{F}_2[\tau]$-modules of generators, and
    /// $\mathrm{Ext}_{A_C} = H(\delta)$ is the motivic Adams $E_2$. A minimal resolution has no
    /// unit-operation part mod $\tau$; the lift's $\tau$-power corrections create it.
    pub fn delta(&self, g: Gen) -> Vec<(Gen, u32)> {
        let module = self.module(g.s - 1);
        let w_k = self.weights[&g];
        let mut out = Vec::new();
        for &bidx in &self.lifted[&g] {
            let og = module.index_to_op_gen(g.t, bidx);
            // The unit is the only operation in degree 0.
            if og.operation_degree == 0 {
                let gj = Gen {
                    s: g.s - 1,
                    t: og.generator_degree,
                    idx: og.generator_index,
                };
                let diff = self.weights[&gj] - w_k;
                let power = u32::try_from(diff).unwrap_or_else(|_| {
                    panic!(
                        "δ must raise the weight, got τ-power {diff} at (s={}, t={})",
                        g.s, g.t
                    )
                });
                out.push((gj, power));
            }
        }
        out
    }

    /// The mod-$\tau$ support of the lifted `d_s(g)`: its terms with $\tau$-power 0.
    fn mod_tau_support(&self, g: Gen) -> BTreeSet<usize> {
        let w = self.weights[&g];
        self.lifted[&g]
            .iter()
            .copied()
            .filter(|&bidx| w - self.entry_weight(g.s - 1, g.t, bidx) == 0)
            .collect()
    }

    /// Assert `d_{s-1} d_s = 0` over $A_C$ for every generator in the report box.
    pub fn verify_d_squared_zero(&self) {
        for s in 2..=self.max_s() {
            for t in 0..=(self.max.n() + s) {
                for idx in 0..self.num_gens(s, t) {
                    let g = Gen { s, t, idx };
                    let err = self.composite(g, &self.lifted[&g]);
                    assert!(
                        err.is_empty(),
                        "d² ≠ 0 at (s={s}, t={t}, idx={idx}): {} surviving terms",
                        err.len()
                    );
                }
            }
        }
    }

    /// Assert that every lifted differential reduces mod $\tau$ to the resolution's.
    pub fn verify_mod_tau_reduction(&self) {
        for s in 1..=self.max_s() {
            for t in 0..=(self.max.n() + s) {
                for idx in 0..self.num_gens(s, t) {
                    let g = Gen { s, t, idx };
                    let engine_support: BTreeSet<usize> = self
                        .resolution
                        .differential(s)
                        .output(t, idx)
                        .iter_nonzero()
                        .filter(|(_, v)| *v != 0)
                        .map(|(i, _)| i)
                        .collect();
                    assert_eq!(
                        self.mod_tau_support(g),
                        engine_support,
                        "mod-τ reduction differs from the model at (s={s}, t={t}, idx={idx})"
                    );
                }
            }
        }
    }
}

/// Lifting a map over $A_C/\tau$ to one over $A_C$, in the style of
/// [`crate::secondary::SecondaryLift`].
///
/// The lift starts from the $\tau^0$ part and cancels the *defect*, the amount by which the
/// defining equation fails over $A_C$, one weight-order at a time, solving each order with the
/// quasi-inverse of the target's mod-$\tau$ differential. Weight-homogeneity forces each
/// correction to a single $\tau$-power. An implementor supplies the defect and inherits
/// [`Self::lift_cell`].
trait TauLift {
    /// The weight the defect is graded against: the source generator's.
    fn source_weight(&self, g: Gen) -> i32;

    /// The mod-$\tau$ support to start from, in the output module.
    fn seed(&self, g: Gen) -> BTreeSet<usize>;

    /// The module and degree the defect lives in.
    fn defect_module(&self, g: Gen) -> (Arc<FreeModule<SteenrodAlgebra>>, i32);

    /// The weight of defect-module basis element `bidx`.
    fn defect_weight(&self, g: Gen, bidx: usize) -> i32;

    /// Seed the running defect with the part that does not depend on the output support.
    fn seed_constant(&self, _g: Gen, _parity: &mut FpVector) {}

    /// XOR into `parity` the defect contribution of output-support element `bidx`.
    fn accumulate(&self, g: Gen, bidx: usize, parity: &mut FpVector);

    /// Solve `d̄(c) = e` for a correction `c` in the output module, or `None` past the resolved
    /// box, where there is no quasi-inverse.
    fn solve(&self, g: Gen, e: &FpVector) -> Option<FpVector>;

    /// Cancel the defect one weight-order at a time and return the lifted support.
    ///
    /// A cell whose corrections leave the resolved box keeps a partial lift; nothing in the
    /// report box reads it.
    fn lift_cell(&self, g: Gen) -> BTreeSet<usize> {
        let (def_mod, def_t) = self.defect_module(g);
        let def_dim = def_mod.dimension(def_t);
        let w_k = self.source_weight(g);

        // The defect is linear in the support, so it is kept as a running parity vector.
        let mut support = self.seed(g);
        let mut parity = FpVector::new(TWO, def_dim);
        self.seed_constant(g, &mut parity);
        for &bidx in support.iter() {
            self.accumulate(g, bidx, &mut parity);
        }

        for _ in 0..256 {
            // The lowest τ-order among the surviving error terms.
            let Some(min_power) = parity
                .iter_nonzero()
                .map(|(fidx, _)| self.defect_weight(g, fidx) - w_k)
                .min()
            else {
                return support;
            };
            assert!(
                min_power >= 1,
                "mod-τ defect ≠ 0 at (s={}, t={}, idx={}) — the model is not a complex",
                g.s,
                g.t,
                g.idx
            );

            // The error at that lowest τ-order, as a defect-module vector.
            let mut e = FpVector::new(TWO, def_dim);
            for (fidx, _) in parity.iter_nonzero() {
                if self.defect_weight(g, fidx) - w_k == min_power {
                    e.set_entry(fidx, 1);
                }
            }

            // Each term of `c` is forced by weight to τ-power `min_power`, cancelling this order.
            let Some(c) = self.solve(g, &e) else {
                return support;
            };
            for (idx, v) in c.iter_nonzero() {
                if v == 0 {
                    continue;
                }
                if !support.insert(idx) {
                    support.remove(&idx);
                }
                self.accumulate(g, idx, &mut parity);
            }
        }
        tracing::debug!(
            "motivic lift did not converge at (s={}, t={}, idx={}); leaving partial (outside \
             report cone)",
            g.s,
            g.t,
            g.idx
        );
        support
    }
}

/// [`TauLift`] for the differential: lift `d_s(g)` so that `d_{s-1} d_s(g) = 0` over $A_C$.
///
/// The output module is $F_{s-1}$ and the defect is the composite in $F_{s-2}$.
struct DifferentialCells<'a>(&'a MotivicResolution);

impl TauLift for DifferentialCells<'_> {
    fn source_weight(&self, g: Gen) -> i32 {
        self.0.weights[&g]
    }

    fn seed(&self, g: Gen) -> BTreeSet<usize> {
        self.0
            .resolution
            .differential(g.s)
            .output(g.t, g.idx)
            .iter_nonzero()
            .filter(|(_, v)| *v != 0)
            .map(|(i, _)| i)
            .collect()
    }

    fn defect_module(&self, g: Gen) -> (Arc<FreeModule<SteenrodAlgebra>>, i32) {
        (self.0.module(g.s - 2), g.t)
    }

    fn defect_weight(&self, g: Gen, bidx: usize) -> i32 {
        self.0.entry_weight(g.s - 2, g.t, bidx)
    }

    fn accumulate(&self, g: Gen, bidx: usize, parity: &mut FpVector) {
        let f_sm1 = self.0.module(g.s - 1);
        let f_sm2 = self.0.module(g.s - 2);
        self.0
            .accumulate_term(g.s, g.t, &f_sm1, &f_sm2, &self.0.engine, bidx, parity);
    }

    fn solve(&self, g: Gen, e: &FpVector) -> Option<FpVector> {
        let d_prev = self.0.resolution.differential(g.s - 1);
        let qi = d_prev.quasi_inverse(g.t)?;
        let mut c = FpVector::new(TWO, self.0.module(g.s - 1).dimension(g.t));
        qi.apply(c.as_slice_mut(), 1, e.as_slice());
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use algebra::{Algebra, MilnorAlgebra};

    use super::*;

    #[test]
    fn ctau_algebra_indexes_like_the_engine() {
        // The lift reads the engine's products and weights by the resolution's operation indices.
        let algebra = MilnorAlgebra::motivic_mod_tau(Default::default(), false);
        let engine = MotivicMilnorAlgebra::new();
        algebra.compute_basis(40);
        engine.compute_basis(40);
        for t in 0..=40 {
            assert_eq!(
                algebra.dimension(t),
                engine.dimension(t),
                "dimension in degree {t}"
            );
            for idx in 0..algebra.dimension(t) {
                let elt = algebra.basis_element_from_index(t, idx);
                let algebra::motivic::milnor::Dual(m) = *engine.basis_element(t, idx);
                assert!(
                    elt.q_part == m.q_part && elt.p_part == m.p_part,
                    "basis order differs at ({t}, {idx})"
                );
            }
        }
    }

    /// The ordinary resolution of a module spec, unresolved.
    fn resolution(spec: &str) -> Arc<Resolution<CCC>> {
        Arc::new(crate::utils::construct_standard::<false, _, _>(spec, None).unwrap())
    }

    #[test]
    fn only_all_of_ctau_is_accepted() {
        let max = Bidegree::n_s(2, 2);
        assert!(MotivicResolution::new(resolution("S_2"), max).is_err());
        // A(1)/τ: the engine indexes all of A_C, so a profile would be misread.
        assert!(MotivicResolution::new(resolution("kq_sm_Ctau"), max).is_err());
    }

    /// Check that `spec` lifts to a complex over $A_C$ that reduces to its resolution mod $\tau$,
    /// and that the lift creates δ.
    fn check_lift(spec: &str, max: Bidegree) {
        let res = MotivicResolution::new(resolution(spec), max).unwrap();

        // The unit has weight 0, and every generator in range has a weight.
        assert_eq!(res.generator_weight(Gen { s: 0, t: 0, idx: 0 }), 0);
        let mut count = 0;
        for s in 0..=res.max_s() {
            for t in 0..=(max.n() + s) {
                for idx in 0..res.num_gens(s, t) {
                    let _ = res.generator_weight(Gen { s, t, idx });
                    count += 1;
                }
            }
        }
        assert!(count > 15, "expected many generators, got {count}");

        res.verify_mod_tau_reduction();
        res.verify_d_squared_zero();

        // The lift must create some δ, and every δ-entry carries a positive τ-power.
        let mut delta_entries = 0;
        for s in 1..=res.max_s() {
            for t in 0..=(max.n() + s) {
                for idx in 0..res.num_gens(s, t) {
                    for (_target, power) in res.delta(Gen { s, t, idx }) {
                        assert!(power >= 1, "δ entry with non-positive τ-power");
                        delta_entries += 1;
                    }
                }
            }
        }
        assert!(
            delta_entries > 0,
            "the lift produced no δ (augmentation) terms"
        );
    }

    #[test]
    fn the_sphere_lifts() {
        check_lift("Ctau", Bidegree::n_s(8, 5));
    }

    #[test]
    fn the_moore_space_lifts() {
        check_lift("C2_sm_Ctau", Bidegree::n_s(8, 5));
    }
}
