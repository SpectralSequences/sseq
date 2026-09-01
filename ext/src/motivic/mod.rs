//! The C-motivic Adams $E_2$ by deformation: lift the $A_C/\tau$ resolution to
//! $A_C$ over $\mathbb{F}_2[\tau]$.
//!
//! Phase 1 ([`crate`]'s `resolve_motivic_ctau` example) resolves the trivial
//! module over $A_C/\tau$ with the ordinary engine. That resolution is *minimal
//! mod $\tau$*: its differentials $\bar d_s$ have entries in the augmentation
//! ideal (positive-degree operations). This module performs Phase 2 of
//! `MOTIVIC_PLAN.md`: it lifts $\bar d_s$ to honest differentials $d_s$ over
//! $A_C$ (coefficients in $\mathbb{F}_2[\tau]$) with $d_{s-1} d_s = 0$, reducing
//! to $\bar d_s$ mod $\tau$.
//!
//! # Weights and the valuation representation
//!
//! $A_C/\tau$ is bigraded by (stem $t$, motivic weight $w$), but we present it to
//! the engine graded by $t$ alone (it is connected and finite-type there). The
//! minimal resolution nonetheless comes out **weight-homogeneous**: every
//! generator has a well-defined weight, computed here by descending the
//! differential ([`MotivicResolution::generator_weight`]). Homogeneity forces the
//! $\tau$-power of every differential entry — a term $m \otimes g_j$ (operation
//! $m$, target generator $g_j$) in $d_s(g_k)$ carries exactly
//! $\tau^{\,w(g_k) - w(m) - w(g_j)}$ — so a lifted differential is an
//! $\mathbb{F}_2$ support (which basis elements occur) plus the weights, with the
//! $\tau$-powers reconstructed on demand. This is the one-integer-per-entry
//! valuation representation of the plan.
//!
//! # The lift
//!
//! Initialize $d_s = \bar d_s$ (each mod-$\tau$ operation at $\tau^0$). Over
//! $A_C$ the composite $d_{s-1} d_s$ is $\equiv 0 \bmod \tau$ (its $\tau^0$ part
//! is $\bar d_{s-1}\bar d_s = 0$) but not exactly $0$: the $A_C$ products
//! $m \cdot m'$ generate $\tau$-divisible terms. That remainder is a
//! $\bar d_{s-2}$-cycle, so the quasi-inverse of $\bar d_{s-1}$ (already computed
//! by the engine) yields a $\tau$-power correction to $d_s$ that cancels the
//! lowest-order remainder; iterating to bounded $\tau$-order gives $d_{s-1}d_s=0$.
//! (Guozhen Wang's Adams–Novikov lift, module side; see `MOTIVIC_PLAN.md` §5.)
//!
//! # The cohomology $H(\delta)$ — the motivic Adams $E_2$
//!
//! The lift creates $\delta$, the identity-operation (augmentation) part of the
//! differential ([`MotivicResolution::delta`]): a differential on the free
//! $\mathbb{F}_2[\tau]$-modules of generators at fixed internal degree $t$. The
//! motivic Adams $E_2$ is `Ext_{A_C} = H(δ)`, a graded $\mathbb{F}_2[\tau]$-module
//! — free $\oplus\ \mathbb{F}_2[\tau]/\tau^k$, since $\tau$ is the only homogeneous
//! prime. Because $\delta$ raises the weight, `{weight ≤ cap}` is a subcomplex and
//! the whole computation is pure $\mathbb{F}_2$ linear algebra. The three anchors
//! fall out:
//!
//! - **invert $\tau$** — the free rank (all generators): the classical Adams $E_2$
//!   (`classical_ext_rank`, regressed against `Ext_A`; lands with the deformation
//!   spectral sequence).
//! - **$\tau = 0$** — the generator counts: the algebraic Novikov $E_2$ (Phase 1).
//! - **keep $\tau$** — free plus $\tau$-torsion: the motivic $E_2$ as a full
//!   $\mathbb{F}_2[\tau]$-module (`tau_module`, structure
//!   theorem), including the $h_1$-tower classes ($h_1^n$, which are
//!   $\mathbb{F}_2[\tau]/\tau$-torsion for $n \ge 4$) that the classical page kills.

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

/// The C-motivic resolution: the mod-$\tau$ model plus the weight assignment and
/// (Phase 2) the lifted $A_C$ differentials.
pub struct MotivicResolution {
    /// The $A_C$ product engine, which recovers the $\tau$-powers the resolution discards. It
    /// indexes each degree as $A_C/\tau$ does, so the resolution's operation indices are its own.
    engine: MotivicMilnorAlgebra,
    resolution: Arc<Resolution<CCC>>,
    /// Motivic weight of each generator. `Arc`-shared with the Ext DGA's
    /// the Ext DGA's coboundary (which slices by it) rather than copied.
    weights: Arc<HashMap<Gen, i32>>,
    /// The lifted $A_C$ differential of each generator: the set of $F_{s-1}$ basis
    /// elements in its image. The coefficient of each is $1 \in \mathbb{F}_2$ and
    /// its $\tau$-power is forced by the weights, so the support is the whole
    /// datum (see the module docs).
    lifted: HashMap<Gen, BTreeSet<usize>>,
    /// The box the results are trusted/reported in.
    max: Bidegree,
    /// The (padded) square actually resolved: `{stem ≤ compute.n(), filt ≤
    /// compute.s()}`. It is the report box `max` with a small stem margin for the
    /// lift's δ-reach (see [`Self::new`]).
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
        // Resolve the report box **plus exactly one stem** with
        // `compute_through_stem`. Ext at `(s, t)` is `H(δ)`, and δ maps `(s, t) →
        // (s-1, t)` — same internal degree, one lower Novikov filtration, hence
        // stem `n → n+1`. So computing Ext at stem `n` needs the δ *out of* stem
        // `n`, whose targets are the generators at stem `n+1`; those must exist
        // (`delta_star_rank` reads `num_gens` there). That is the whole margin: a
        // hard, structural `+1`, not a fudge.
        //
        // Nothing at stem `n+2` is needed, and `compute_through_stem` gives the
        // `n+1` strip cheaply: at its edge it records only *kernels* one stem out
        // (`resolution.rs`), never resolving generators there. The lift of the
        // stem-`(n+1)` boundary generators therefore can't emit δ-terms into
        // stem `n+2` (those generators don't exist), and such a term would land in
        // internal degree `> n+s` anyway — invisible to every stem-`n` composite.
        // So `+1` is provably sufficient; `MOT_MARGIN` is only an escape hatch.
        let margin: i32 = std::env::var("MOT_MARGIN")
            .ok()
            .and_then(|v| v.parse().ok())
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
        this.compute_weights();
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

    /// The mod-$\tau$ resolution (the Phase 1 model).
    pub fn resolution(&self) -> &Resolution<CCC> {
        &self.resolution
    }

    /// The box results are reported/trusted in.
    pub fn max(&self) -> Bidegree {
        self.max
    }

    /// The algebraic Novikov $E_2$ rank at `(s, t)` — set $\tau = 0$: the number
    /// of generators (with $\delta \equiv 0 \bmod \tau$, `Ext = generators`).
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

    /// Lift every differential to $A_C$. Processed with `s` ascending so that
    /// `d_{s-1}` is finalized before `d_s` is corrected against it.
    ///
    /// `d_1` needs no correction: `d_0 d_1 = 0` already, since `d_0` is the
    /// augmentation into the trivial module and `d_1`'s entries lie in the
    /// augmentation ideal. For `s ≥ 2` we start from the mod-$\tau$ support and
    /// cancel the $\tau$-divisible remainder of `d_{s-1} d_s`.
    #[tracing::instrument(skip(self), fields(max = %self.max, num_lifted = tracing::field::Empty))]
    fn lift(&mut self) {
        // Wavefront over `s`: correcting a generator reads only its `s-1`
        // neighbors (the mod-τ targets at stem ≤ n and the δ-targets at stem
        // n+1), and runs its own weight-loop locally. So `s` is a sequential
        // barrier, but at each `s` every generator is independent — fan them out.
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

    /// Lift a single generator's differential to `A_C`. Parallel-safe: it reads
    /// only already-finalized `s-1` data (and the shared, thread-safe product
    /// cache), writing nothing.
    ///
    /// The correction is the [`TauLift`] driver applied to the differential cell
    /// (via [`DifferentialCells`]); it runs only for generators whose δ-correction
    /// cone stays inside the padded box. The cone of a generator at stem `n` reaches
    /// up to stem `n + s` (each augmentation correction pushes one stem out), so a
    /// generator with stem `> report.n + report.s` cannot converge — and it is never
    /// referenced by the report cohomology (differentials go to stem ≤ n, δ-terms to
    /// n+1, and the report cone is bounded by report.n + report.s). Leaving those as
    /// their mod-τ support is correct.
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

    /// XOR the contribution of a single `d_s(g_k)` support element `bidx` (a basis
    /// element of $F_{s-1}$ in degree `t`) into a running `d_{s-1} d_s` parity
    /// vector over the $F_{s-2}$ basis in degree `t`.
    ///
    /// `composite` is mod-2 linear in the support and each output basis element's
    /// $\tau$-power is a function of that element alone (weight-homogeneity), so
    /// this per-term atom is all both [`Self::composite`] and the incremental
    /// the correction step needs: toggling a term in or out of the support is exactly
    /// one call here (XOR is its own inverse).
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
        // The differential's own composite: compose with `d = self.lifted`
        // (degree-preserving, so inner_shift_t = 0).
        self.compose_into(f_sm1, t, s - 1, &self.lifted, 0, f_sm2, bidx, parity);
    }

    /// The shared atom of every τ-adic lift: compose the outer basis element `bidx`
    /// of `outer_mod` (internal degree `outer_t`, generators at homological degree
    /// `outer_s`) with the lifted map `inner` (whose value on a generator lives in
    /// `inner_out_mod`), XORing the resulting $A_C$ products into `parity`.
    ///
    /// `bidx` decodes to `m ⊗ gⱼ`; `inner[gⱼ]` to `m′ ⊗ gₗ`; the term contributed is
    /// `(m·m′) ⊗ gₗ` (all $\tau$-powers of the $A_C$ product `m·m′`, since the
    /// forced power of each output basis element is fixed by weight-homogeneity and
    /// recovered later). For the differential `inner = d` (`self.lifted`); for a
    /// product chain map `inner = φₐ`. A generator absent from `inner` contributes 0.
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
            return; // inner map is zero on gⱼ (e.g. outside φₐ's range) ⇒ no term
        };
        // `inner[gⱼ]` lives at degree `gⱼ.t − inner_shift_t` (0 for the
        // degree-preserving differential, aₜ for the chain map φₐ).
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

    /// The composite `d_{s-1} d_s(g_k)` over `A_C`, as a map from $F_{s-2}$ basis
    /// element (in degree `g_k.t`) to its forced $\tau$-power. Only odd-parity
    /// (surviving) terms are returned. The `A_C` products `m · m'` are where the
    /// $\tau$-divisible terms are generated.
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
                // Every path to `fidx` has the same τ-power = W(fidx) − W(g_k)
                // (weight-homogeneity), so the parity above is well-defined.
                let power = self.entry_weight(s - 2, t, fidx) - w_k;
                (fidx, power)
            })
            .collect()
    }

    /// Assign a motivic weight to every generator by descending its differential:
    /// weight-homogeneity forces `w(g) = w(m) + w(g')` for every term `m ⊗ g'` of
    /// `d(g)`. The unit generator has weight 0; higher weights propagate. Panics
    /// if any generator turns out weight-inhomogeneous (which would violate the
    /// bigraded structure and break the valuation representation).
    #[tracing::instrument(skip(self), fields(num_weights = tracing::field::Empty))]
    fn compute_weights(&mut self) {
        // Built locally, then `Arc`-shared into `self.weights` (and the Ext DGA) — no copy.
        let mut weights: HashMap<Gen, i32> = HashMap::new();
        // s = 0: the single generator is the unit, weight 0.
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
    }

    /// The $\delta$-entries out of generator `g`: the identity-operation
    /// (augmentation) part of the lifted differential `d_s(g)`. Each entry is a
    /// target generator `g'` (at the same internal degree `t`, homological degree
    /// `s-1`) together with the $\tau$-power on the unit operation.
    ///
    /// This is the datum Phase 3 takes cohomology of: `δ` is a differential on the
    /// free $\mathbb{F}_2[\tau]$-modules of generators, and `Ext_{A_C} = H(δ)`
    /// (the motivic Adams $E_2$). Over a field the augmentation part of a minimal
    /// differential vanishes; here the $\tau$-power corrections create it.
    pub fn delta(&self, g: Gen) -> Vec<(Gen, u32)> {
        let module = self.module(g.s - 1);
        let w_k = self.weights[&g];
        let mut out = Vec::new();
        for &bidx in &self.lifted[&g] {
            let og = module.index_to_op_gen(g.t, bidx);
            // The identity operation is the unit: degree 0, index 0.
            if og.operation_degree == 0 {
                let gj = Gen {
                    s: g.s - 1,
                    t: og.generator_degree,
                    idx: og.generator_index,
                };
                let power = (self.weights[&gj] - w_k) as u32;
                out.push((gj, power));
            }
        }
        out
    }

    // ---- Phase 3: the cohomology H(δ) = the motivic Adams E₂ ----
    //
    // δ is a differential on the free F₂[τ]-modules of generators (fixed internal
    // degree `t`), with δ↓: gens(s,t) → gens(s−1,t) given by [`delta`]. Because
    // everything is weight-graded and τ is the only homogeneous prime, `Ext = H(δ)`
    // is a graded F₂[τ]-module: free ⊕ F₂[τ]/τᵏ. δ↓ raises the weight, so
    // `{weight ≤ cap}` is a subcomplex — and taking that cohomology (free rank at
    // cap = ∞, torsion exposed at lower caps) is now the job of the [`ExtAlgebra`]
    // built by [`Self::build_ext`], via [`MotivicCoboundary`] as its differential.

    /// The mod-$\tau$ support of `d_s(g)`: the lifted terms whose forced
    /// $\tau$-power is $0$. These should reproduce the engine's $\bar d_s$ exactly.
    fn mod_tau_support(&self, g: Gen) -> BTreeSet<usize> {
        let w = self.weights[&g];
        self.lifted[&g]
            .iter()
            .copied()
            .filter(|&bidx| w - self.entry_weight(g.s - 1, g.t, bidx) == 0)
            .collect()
    }

    /// Verify `d_{s-1} d_s = 0` over `A_C` for every generator in range: the
    /// defining property of the lifted resolution.
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

    /// Verify that reducing every lifted differential mod $\tau$ recovers the
    /// original mod-$\tau$ resolution the engine computed.
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

/// The shared τ-adic lifting problem, in the style of [`crate::secondary::SecondaryLift`].
///
/// Every "make it motivic" step in this module has the same shape: a map given over
/// the mod-τ algebra $A_C/\tau$ must be lifted to an honest map over $A_C$
/// (coefficients in $\mathbb{F}_2[\tau]$). The lift starts from the mod-τ datum (the
/// $\tau^0$ part) and cancels the τ-divisible *defect* — the amount by which the
/// defining equation fails over $A_C$ — one weight-order at a time, solving each
/// order with the quasi-inverse of the target complex's mod-τ differential.
/// Weight-homogeneity forces every correction to a single τ-power, so the
/// order-by-order cancellation converges.
///
/// The differential lift ($d^2 = 0$, [`DifferentialCells`]) is the first instance;
/// the product lift ($d\varphi = \varphi d$) and — eventually — the chain-homotopy
/// lift (Massey products) are the same driver with a different *defect*. An
/// implementor supplies the object-specific hooks and inherits [`Self::lift_cell`].
trait TauLift {
    /// The weight the defect is graded against — the source generator's weight.
    fn source_weight(&self, g: Gen) -> i32;

    /// The mod-τ ($\tau^0$) support to start from, as basis-element indices of the
    /// output module (where the lifted support and the corrections live).
    fn seed(&self, g: Gen) -> BTreeSet<usize>;

    /// The defect module `(module, t)` — where the error `e` lives — used to size
    /// the running parity vector.
    fn defect_module(&self, g: Gen) -> (Arc<FreeModule<SteenrodAlgebra>>, i32);

    /// The weight of defect-module basis element `bidx`.
    fn defect_weight(&self, g: Gen, bidx: usize) -> i32;

    /// Seed the running defect with the part that does not depend on the output
    /// support (e.g. the `φ(dg)` term of a chain map). Default: none — the
    /// differential's `d²` is entirely a function of its support.
    fn seed_constant(&self, _g: Gen, _parity: &mut FpVector) {}

    /// XOR into `parity` the defect contribution of output-support element `bidx`.
    fn accumulate(&self, g: Gen, bidx: usize, parity: &mut FpVector);

    /// Solve `d̄(c) = e` for a correction `c` in the output module, via the target's
    /// mod-τ quasi-inverse. `None` when the quasi-inverse is unavailable (a cell just
    /// past the padded box) — the driver then leaves the mod-τ seed uncorrected.
    fn solve(&self, g: Gen, e: &FpVector) -> Option<FpVector>;

    /// The shared driver: cancel the τ-divisible defect one weight-order at a time.
    /// Returns the lifted support — the seed if there is nothing to correct, or a
    /// partial lift if a cell outside the report cone does not converge (those are
    /// never read by the report cohomology).
    fn lift_cell(&self, g: Gen) -> BTreeSet<usize> {
        let (def_mod, def_t) = self.defect_module(g);
        let def_dim = def_mod.dimension(def_t);
        let w_k = self.source_weight(g);

        // Maintain the defect incrementally: it is mod-2 linear in the support, so we
        // keep a running parity vector and XOR in only each term we toggle.
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
                return support; // defect fully cancelled
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

            // Solve d̄(c) = e and toggle c into the support, updating the running
            // parity in lockstep. Each correction term is forced (by weight) to
            // τ-power min_power, cancelling this order.
            let Some(c) = self.solve(g, &e) else {
                return support; // out of range: leave the partial lift
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
        // Non-convergence within the cap: only outside the report cone (report-cone
        // cells converge). Never read by the report cohomology, so leave the partial.
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

/// The differential-lift instance of [`TauLift`]: lift `d_s(g)` so that
/// `d_{s-1} d_s(g) = 0` over `A_C`. Output module `F_{s-1}`, defect module
/// `F_{s-2}`, defect the composite `d_{s-1} d_s(g)` (accumulated by
/// [`MotivicResolution::accumulate_term`]); `d̄_{s-1}`'s quasi-inverse solves the
/// corrections.
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

        // The lift must create a nontrivial augmentation part δ (the τ-power
        // corrections on the unit operation) — this is what Phase 3 takes
        // cohomology of. Every δ-entry carries a positive τ-power.
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
