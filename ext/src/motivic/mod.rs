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
//!   ([`MotivicResolution::classical_ext_rank`], regressed against `Ext_A`).
//! - **$\tau = 0$** — the generator counts: the algebraic Novikov $E_2$ (Phase 1).
//! - **keep $\tau$** — free plus $\tau$-torsion: the motivic $E_2$ as a full
//!   $\mathbb{F}_2[\tau]$-module ([`MotivicResolution::tau_module`], structure
//!   theorem), including the $h_1$-tower classes ($h_1^n$, which are
//!   $\mathbb{F}_2[\tau]/\tau$-torsion for $n \ge 4$) that the classical page kills.

use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    path::PathBuf,
    sync::{Arc, OnceLock},
};

use algebra::{
    Algebra, CTauAlgebra,
    module::{FDModule, FreeModule, Module, homomorphism::ModuleHomomorphism},
    motivic::MotivicMilnorAlgebra,
};
use bivec::BiVec;
use fp::{prime::TWO, vector::FpVector};
use maybe_rayon::prelude::*;
use sseq::{Sseq, coordinates::Bidegree};

use crate::{
    chain_complex::{ChainComplex, FiniteChainComplex},
    ext_algebra::ExtAlgebra,
    resolution::Resolution,
};

mod cohomology;
mod deformation;
mod f2tau;
mod massey;
mod persist;
mod products;

pub use cohomology::TauModule;
pub use deformation::Deformation;
pub use massey::MotivicMassey;

/// The $A_C/\tau$ resolution type: the trivial module resolved by the ordinary
/// engine over the mod-$\tau$ Steenrod algebra.
pub type CTauResolution = Resolution<FiniteChainComplex<FDModule<CTauAlgebra>>>;

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
    algebra: Arc<CTauAlgebra>,
    resolution: Arc<CTauResolution>,
    /// The Ext DGA: the mod-τ resolution wrapped so it stores the *dualized*
    /// differential δ = Hom(d, k) and takes its cohomology (the motivic Adams
    /// E₂). Built lazily on first cohomology query. This is the "Ext side" of the
    /// resolution/functor split — the raw differential stays in the resolution
    /// (`lifted`); δ lives here.
    ext: OnceLock<ExtAlgebra<CTauResolution>>,
    /// The deformation (algebraic Novikov / τ-Bockstein) spectral sequence, built
    /// lazily — the source of truth for the free rank and τ-torsion. See
    /// [`Self::deformation_sseq`].
    deformation: OnceLock<Sseq<3, Deformation>>,
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
    /// Resolve the trivial module $k$ over $A_C/\tau$ (the sphere) through the box
    /// `max`, in memory. Shorthand for [`Self::with_module`] on `k` with no save
    /// directory.
    pub fn new(max: Bidegree) -> Self {
        Self::with_module(Self::trivial_module(), max, None)
    }

    /// Build a module over $A_C/\tau$ from a `.json` descriptor — the standard module
    /// format (`gens` naming cells by degree, `actions` naming operations as the
    /// motivic Steenrod algebra prints them: `Q_i`, `P(R)`, or products `Q_i … P(R)`)
    /// — ready for [`Self::with_module`]. For example, the Moore space $S/2$ is
    /// ```json
    /// { "gens": { "x0": 0, "x1": 1 }, "actions": ["Q_0 x0 = x1"] }
    /// ```
    ///
    /// This is exactly the classical [`FDModule::from_json`] path, made available by
    /// the [`GeneratedAlgebra`](algebra::GeneratedAlgebra) implementation of
    /// $A_C/\tau$: only the actions of the *generators* ($Q_0$ and the $P(\xi_1^{2^k})
    /// = \mathrm{Sq}^{2^{k+1}}$) need be listed, and the action of every composite
    /// operation is extended from them and cross-checked against the Steenrod
    /// relations. Inconsistent descriptors are rejected.
    pub fn module_from_json(
        json: &serde_json::Value,
    ) -> anyhow::Result<Arc<FDModule<CTauAlgebra>>> {
        let algebra = Arc::new(CTauAlgebra::new());
        // Size the algebra to the top cell so operation names and composite-action
        // extension resolve; `from_json` drives the rest.
        let max_d = json["gens"]
            .as_object()
            .into_iter()
            .flat_map(|g| g.values())
            .filter_map(serde_json::Value::as_i64)
            .max()
            .unwrap_or(0);
        algebra.compute_basis(max_d.max(0) as i32);
        Ok(Arc::new(FDModule::from_json(algebra, json)?))
    }

    /// The trivial module $k = \mathbb{F}_2$ (concentrated in degree 0) over
    /// $A_C/\tau$: the module whose resolution is the sphere.
    pub fn trivial_module() -> Arc<FDModule<CTauAlgebra>> {
        let algebra = Arc::new(CTauAlgebra::new());
        Arc::new(FDModule::new(
            algebra,
            "F2".to_string(),
            BiVec::from_vec(0, vec![1]),
        ))
    }

    /// Resolve `module` over $A_C/\tau$ through the box `max`, lift to $A_C$, and
    /// assign weights, optionally caching to `save_dir` on disk.
    ///
    /// If `save_dir` is set, the mod-τ resolution is saved/loaded there (via
    /// [`Resolution::new_with_save`]) and the weights + lifted differentials are
    /// cached alongside it (`motivic-lift.bin`), so re-running the same box reloads
    /// the whole computation instead of recomputing the resolution and the lift.
    ///
    /// The generators of `module` must be weight-homogeneous, seeded by
    /// `compute_weights` from the s=0 cells; the trivial module needs no
    /// input (its one cell is the weight-0 unit).
    #[tracing::instrument(
        skip(module, save_dir),
        fields(max = %max, compute = tracing::field::Empty, cached = tracing::field::Empty)
    )]
    pub fn with_module(
        module: Arc<FDModule<CTauAlgebra>>,
        max: Bidegree,
        save_dir: Option<PathBuf>,
    ) -> Self {
        let algebra = module.algebra();
        let cc: Arc<FiniteChainComplex<FDModule<CTauAlgebra>>> =
            Arc::new(FiniteChainComplex::ccdz(module));
        let resolution = Arc::new(
            Resolution::new_with_save(cc, save_dir.clone())
                .expect("failed to open the resolution save directory"),
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
            .and_then(|v| v.parse::<i32>().ok())
            .map(|m| m.max(1))
            .unwrap_or(1);
        let compute = Bidegree::n_s(max.n() + margin, max.s());
        tracing::Span::current().record("compute", tracing::field::display(compute));
        let profile = std::env::var("MOT_PROFILE").is_ok();
        let t0 = std::time::Instant::now();
        resolution.compute_through_stem(compute);
        if profile {
            eprintln!("[profile] resolution: {:?}", t0.elapsed());
        }

        let mut this = Self {
            algebra,
            resolution,
            ext: OnceLock::new(),
            deformation: OnceLock::new(),
            weights: Arc::new(HashMap::new()),
            lifted: HashMap::new(),
            max,
            compute,
        };
        // Load the weights + lifted differentials from disk if a matching cache
        // exists; otherwise compute them and save.
        let cached = this.load_lift(&save_dir);
        tracing::Span::current().record("cached", cached);
        if cached {
            if profile {
                eprintln!("[profile] lift:       loaded from disk");
            }
        } else {
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
            this.save_lift(&save_dir);
        }
        this
    }

    /// The mod-$\tau$ resolution (the Phase 1 model).
    pub fn resolution(&self) -> &CTauResolution {
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
    fn module(&self, s: i32) -> Arc<FreeModule<CTauAlgebra>> {
        self.resolution.module(s)
    }

    /// The motivic weight of a generator (panics if out of the computed range or
    /// if the generator's weight could not be determined).
    pub fn generator_weight(&self, g: Gen) -> i32 {
        self.weights[&g]
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
        let op_w = self.algebra.weight(og.operation_degree, og.operation_index);
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
        f_sm1: &FreeModule<CTauAlgebra>,
        f_sm2: &FreeModule<CTauAlgebra>,
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
        outer_mod: &FreeModule<CTauAlgebra>,
        outer_t: i32,
        outer_s: i32,
        inner: &HashMap<Gen, BTreeSet<usize>>,
        inner_shift_t: i32,
        inner_out_mod: &FreeModule<CTauAlgebra>,
        bidx: usize,
        parity: &mut FpVector,
    ) {
        let engine = self.algebra.engine();
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
            engine.product_indexed_with(m_deg, m_idx, mp_deg, mp_idx, |terms| {
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
        let engine = self.algebra.engine();

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
        // s = 0: the single generator is the unit, weight 0. Only the generator at
        // `t = 0` is seeded, so a module needing another one — anything not cyclic on a
        // degree-0 class — has no weight to propagate from. Reject it here rather than
        // let an unseeded generator surface as a `HashMap` index panic much later.
        for t in 0..=self.compute.n() {
            let expected = usize::from(t == 0);
            assert_eq!(
                self.num_gens(0, t),
                expected,
                "the motivic lift supports modules cyclic on a degree-0 class; this one needs {} \
                 generator(s) at (s = 0, t = {t})",
                self.num_gens(0, t),
            );
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
                        let op_w = self.algebra.weight(og.operation_degree, og.operation_index);
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
    fn defect_module(&self, g: Gen) -> (Arc<FreeModule<CTauAlgebra>>, i32);

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

    fn defect_module(&self, g: Gen) -> (Arc<FreeModule<CTauAlgebra>>, i32) {
        (self.0.module(g.s - 2), g.t)
    }

    fn defect_weight(&self, g: Gen, bidx: usize) -> i32 {
        self.0.entry_weight(g.s - 2, g.t, bidx)
    }

    fn accumulate(&self, g: Gen, bidx: usize, parity: &mut FpVector) {
        let f_sm1 = self.0.module(g.s - 1);
        let f_sm2 = self.0.module(g.s - 2);
        self.0.accumulate_term(
            g.s,
            g.t,
            &f_sm1,
            &f_sm2,
            self.0.algebra.engine(),
            bidx,
            parity,
        );
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
    use sseq::coordinates::{BidegreeGenerator, degree::MultiDegree, element::MultiDegreeElement};

    use super::*;

    #[test]
    fn ctau_products_run_via_ext_algebra() {
        // ExtAlgebra's product machinery runs on the motivic (mod-τ) resolution:
        // the cochain ring is Ext_{A_C/τ} = the algebraic Novikov E₂ = the Adams
        // E₂ of Cτ. Exercise the h₀-tower, which is present there:
        //   h₀ ∈ (n=0, s=1); h₀ⁿ is the (single) nonzero class of (n=0, s=n).
        let res = MotivicResolution::new(Bidegree::n_s(8, 5));
        let ext = res.ext();

        assert_eq!(ext.dimension(Bidegree::n_s(0, 1)), 1, "h₀ is 1-dimensional");
        let h0 = ext.generator(BidegreeGenerator::new(Bidegree::n_s(0, 1), 0));

        // h₀ⁿ = h₀·h₀ⁿ⁻¹ stays nonzero up the tower (Cτ has an infinite h₀-tower).
        let mut power = h0.clone();
        for n in 2..=4 {
            power = ext.multiply(&h0, &power);
            let deg = Bidegree::n_s(0, n);
            assert_eq!(power.degree(), deg, "h₀^{n} lands in (0,{n})");
            assert_eq!(ext.dimension(deg), 1, "(0,{n}) is 1-dimensional");
            assert!(!power.vec().is_zero(), "h₀^{n} should be nonzero");
        }
    }

    #[test]
    fn deformation_sseq_foundation() {
        // The deformation SS stored as an Sseq: E₁ = Ext_{A_C/τ} trigraded by
        // (n,s,w), d₁ = the weight-1 part of δ.
        let res = MotivicResolution::new(Bidegree::n_s(8, 5));
        let sseq = res.deformation_sseq();

        // E₁ grouping: summing the weight slices at (n,s) recovers the algebraic
        // Novikov rank (the mod-τ generator count).
        let mut totals: HashMap<(i32, i32), usize> = HashMap::new();
        for deg in sseq.iter_degrees() {
            let [n, s, _] = deg.coords();
            *totals.entry((n, s)).or_default() += sseq.dimension(deg);
        }
        for (&(n, s), &total) in &totals {
            if (0..=8).contains(&n) && (0..=5).contains(&s) {
                assert_eq!(
                    total,
                    res.algebraic_novikov_rank(s, n + s),
                    "E₁ weight slices at (n={n}, s={s}) should sum to the generator count"
                );
            }
        }

        // h₀ ∈ (n=0, s=1, w=0): δ(h₀) = ∅, so it is a permanent cycle.
        let h0 = MultiDegree::from([0, 1, 0]);
        assert_eq!(sseq.get_dimension(h0), Some(1), "h₀ is 1-dimensional");
        assert_eq!(
            sseq.permanent_classes(h0).dimension(),
            1,
            "h₀ (δ = ∅) should be a permanent cycle"
        );

        // Every d₁ we added is consistent.
        for deg in sseq.iter_degrees() {
            assert!(
                !sseq.inconsistent(deg),
                "inconsistent differential at {deg}"
            );
        }
    }

    #[test]
    fn deformation_products_h0_tower() {
        // Products wired into the Sseq: multiplication by h₀ (from ExtAlgebra on the
        // mod-τ resolution) climbs the h₀-tower via Sseq::multiply. h₀ ∈ (0,1,0),
        // and h₀ⁿ ∈ (0,n,0) stays nonzero (Cτ has an infinite h₀-tower), on E₁ and
        // hence on every page.
        let res = MotivicResolution::new(Bidegree::n_s(8, 5));
        let sseq = res.deformation_sseq();
        let prods = res.deformation_products(&[(Bidegree::n_s(0, 1), 0)]);
        let h0 = &prods[0];

        let mut v = FpVector::new(TWO, 1);
        v.set_entry(0, 1);
        let mut class = MultiDegreeElement::new(MultiDegree::from([0, 1, 0]), v);
        for n in 2..=4 {
            class = sseq.multiply(&class, h0).expect("h₀ product in range");
            assert_eq!(
                class.degree(),
                MultiDegree::from([0, n, 0]),
                "h₀^{n} degree"
            );
            assert!(!class.vec().is_zero(), "h₀^{n} should be nonzero");
        }
    }

    #[test]
    fn motivic_ctau_ring_relations() {
        // The Cτ ring (= algebraic Novikov E₂ = E₁ of the deformation SS) acting
        // through Sseq::multiply, checked against the standard Adams-E₂ relations.
        // hᵢ lives at (2ⁱ−1, 1) with weight −(2ⁱ−1) (this presentation's sign).
        let res = MotivicResolution::new(Bidegree::n_s(16, 10));
        let sseq = res.deformation_sseq();

        let h = [(0, "h0"), (1, "h1"), (3, "h2"), (7, "h3")];
        let prods = res.deformation_products(
            &h.iter()
                .map(|&(n, _)| (Bidegree::n_s(n, 1), 0))
                .collect::<Vec<_>>(),
        );
        let wt = |n: i32| {
            res.generator_weight(Gen {
                s: 1,
                t: n + 1,
                idx: 0,
            })
        };
        let class_at = |n: i32, s: i32, w: i32| -> MultiDegreeElement<3> {
            let deg = MultiDegree::from([n, s, w]);
            let mut v = FpVector::new(TWO, sseq.get_dimension(deg).unwrap_or(0).max(1));
            v.set_entry(0, 1);
            MultiDegreeElement::new(deg, v)
        };
        // A product landing in an empty (undefined) bidegree is zero: Sseq::multiply
        // returns None exactly then, so treat None as the zero class.
        let is_zero =
            |c: &Option<MultiDegreeElement<3>>| c.as_ref().is_none_or(|x| x.vec().is_zero());

        // h₁-tower: h₁ⁿ ≠ 0 for all n (into the τ-torsion range n ≥ 4), at (n, n, −n).
        let mut c = Some(class_at(1, 1, wt(1)));
        for n in 2..=6 {
            c = c.and_then(|x| sseq.multiply(&x, &prods[1]));
            let x = c
                .as_ref()
                .unwrap_or_else(|| panic!("h₁^{n} fell out of range"));
            assert_eq!(x.degree(), MultiDegree::from([n, n, -n]), "h₁^{n} degree");
            assert!(!x.vec().is_zero(), "h₁^{n} should be nonzero (Cτ h₁-tower)");
        }

        // Adjacent Hopf elements multiply to zero: h₀h₁ = h₁h₂ = h₂h₃ = 0.
        for i in 0..3 {
            let (n, _) = h[i];
            let prod = sseq.multiply(&class_at(n, 1, wt(n)), &prods[i + 1]);
            assert!(is_zero(&prod), "{}·{} should vanish", h[i].1, h[i + 1].1);
        }

        // The motivic relation h₀²h₂ = τ·h₁³ shows up here as a *hidden τ-extension*:
        // h₁³ ≠ 0 at weight −3, while h₀²h₂ vanishes in the Cτ ring (τ = 0) — it would
        // land at (3, 3, −2), one weight up, empty on E₁. Recovering the τ requires
        // the F₂[τ] product lift; the Cτ ring only sees the τ = 0 shadow.
        let h1_cubed = {
            let mut c = Some(class_at(1, 1, wt(1)));
            for _ in 0..2 {
                c = c.and_then(|x| sseq.multiply(&x, &prods[1]));
            }
            c.expect("h₁³ in range")
        };
        assert_eq!(h1_cubed.degree(), MultiDegree::from([3, 3, -3]));
        assert!(!h1_cubed.vec().is_zero(), "h₁³ ≠ 0");
        let h0sq_h2 = {
            let mut c = Some(class_at(3, 1, wt(3)));
            for _ in 0..2 {
                c = c.and_then(|x| sseq.multiply(&x, &prods[0]));
            }
            c
        };
        assert!(
            is_zero(&h0sq_h2),
            "h₀²h₂ vanishes in the Cτ ring (the τ is hidden)"
        );
    }

    #[test]
    fn product_lift_is_a_chain_map_and_reduces_mod_tau() {
        // The lifted product chain map φₐ must be an honest chain map over A_C —
        // dφₐ = φₐd — and reduce mod τ (its weight-w(g)−w(a) part) to the Cτ product.
        // The product analogue of verify_d_squared_zero. Cover h₀ (weight 0) and h₁
        // (weight −1) so the weight convention w(φₐ g) = w(g) − w(a) is exercised.
        let res = MotivicResolution::new(Bidegree::n_s(12, 8));
        for a in [Gen { s: 1, t: 1, idx: 0 }, Gen { s: 1, t: 2, idx: 0 }] {
            let wa = res.weights[&a];
            let phi = res.lift_product(a, 6);

            for (&g, support) in &phi {
                let out_s = g.s - a.s;
                let stem = g.t - g.s;
                let in_cone = stem <= res.max().n() + res.max().s();
                if out_s < 1 || !in_cone || !res.lifted.contains_key(&g) {
                    continue;
                }

                // The chain-map defect d(φₐ g) + φₐ(dg) over A_C, in F_{s−aₛ−1}.
                let def_mod = res.module(out_s - 1);
                let mut parity = FpVector::new(TWO, def_mod.dimension(g.t - a.t));
                let out_mod = res.module(out_s);
                for &bidx in support {
                    res.compose_into(
                        &out_mod,
                        g.t - a.t,
                        out_s,
                        &res.lifted,
                        0,
                        &def_mod,
                        bidx,
                        &mut parity,
                    );
                }
                let f_sm1 = res.module(g.s - 1);
                for &bidx in &res.lifted[&g] {
                    res.compose_into(&f_sm1, g.t, g.s - 1, &phi, a.t, &def_mod, bidx, &mut parity);
                }
                assert!(
                    parity.is_zero(),
                    "product chain-map defect ≠ 0 at {g:?} (a={a:?})"
                );

                // Mod-τ reduction: the τ⁰ part of φₐ(g) — its entries at weight
                // w(g) − w(a) — is the Cτ product; corrections sit at higher weight.
                let w_src = res.weights[&g] - wa;
                let mod_tau: BTreeSet<usize> = support
                    .iter()
                    .copied()
                    .filter(|&b| res.entry_weight(out_s, g.t - a.t, b) == w_src)
                    .collect();
                let seed: BTreeSet<usize> = res
                    .ext()
                    .generator_product_map(BidegreeGenerator::new(
                        Bidegree::n_s(a.t - a.s, a.s),
                        a.idx,
                    ))
                    .get_map(g.s)
                    .output(g.t, g.idx)
                    .iter_nonzero()
                    .filter(|(_, v)| *v != 0)
                    .map(|(i, _)| i)
                    .collect();
                assert_eq!(mod_tau, seed, "φₐ(g) mod τ ≠ Cτ product at {g:?} (a={a:?})");
            }
        }
    }

    #[test]
    fn nullhomotopy_lift_satisfies_dh_plus_hd_eq_product() {
        // The lifted null-homotopy H of φ_b∘φ_a (a = h₀, b = h₁, so h₀h₁ = 0) must
        // satisfy dH + Hd = φ_bφ_a over A_C — the defining equation of the homotopy,
        // now with τ-powers. This is the third TauLift instance's verify.
        let res = MotivicResolution::new(Bidegree::n_s(12, 8));
        let a = Gen { s: 1, t: 1, idx: 0 }; // h₀
        let b = Gen { s: 1, t: 2, idx: 0 }; // h₁
        let (shift_s, shift_t) = (a.s + b.s, a.t + b.t);
        let max_s = 6;
        let phi_a = res.lift_product(a, max_s);
        let phi_b = res.lift_product(b, max_s);
        let h = res.lift_nullhomotopy(a, b, max_s);

        let mut checked = 0;
        for (&g, hg) in &h {
            let out_s = g.s + 1 - shift_s;
            let stem = g.t - g.s;
            let in_cone = stem <= res.max().n() + res.max().s();
            if out_s < 1 || !in_cone || !res.lifted.contains_key(&g) {
                continue;
            }
            // defect = d(Hg) + H(dg) + φ_b(φ_a g), in F_{s−shiftₛ} at t−shiftₜ.
            let def_mod = res.module(g.s - shift_s);
            let mut parity = FpVector::new(TWO, def_mod.dimension(g.t - shift_t));
            let out_mod = res.module(out_s);
            for &bidx in hg {
                res.compose_into(
                    &out_mod,
                    g.t - shift_t,
                    out_s,
                    &res.lifted,
                    0,
                    &def_mod,
                    bidx,
                    &mut parity,
                );
            }
            let f_sm1 = res.module(g.s - 1);
            for &bidx in &res.lifted[&g] {
                res.compose_into(
                    &f_sm1,
                    g.t,
                    g.s - 1,
                    &h,
                    shift_t,
                    &def_mod,
                    bidx,
                    &mut parity,
                );
            }
            if let Some(pa) = phi_a.get(&g) {
                let a_mod = res.module(g.s - a.s);
                for &bidx in pa {
                    res.compose_into(
                        &a_mod,
                        g.t - a.t,
                        g.s - a.s,
                        &phi_b,
                        b.t,
                        &def_mod,
                        bidx,
                        &mut parity,
                    );
                }
            }
            assert!(parity.is_zero(), "dH + Hd ≠ φ_bφ_a at {g:?}");
            checked += 1;
        }
        assert!(checked > 0, "no null-homotopy cells were checked");
    }

    #[test]
    fn resolves_a_nontrivial_module_moore_space() {
        // Resolve S/2 (cofiber of 2 = h₀): cells x₀, x₁ joined by Q₀ = Sq¹. The whole
        // deformation pipeline runs on a non-trivial module, and because 2 = h₀ is
        // coned off, the h₀-tower on the bottom cell is truncated — the sphere has
        // h₀ⁿ ≠ 0 at (0, n) for all n, but S/2 has only the bottom class.
        // Build S/2 from a .json descriptor (exercising module_from_json /
        // basis_element_from_string over the motivic Steenrod algebra).
        let descriptor = serde_json::json!({
            "gens": { "x0": 0, "x1": 1 },
            "actions": ["Q_0 x0 = x1"],
        });
        let module = MotivicResolution::module_from_json(&descriptor).unwrap();
        let sphere = MotivicResolution::new(Bidegree::n_s(8, 6));
        let moore = MotivicResolution::with_module(module, Bidegree::n_s(8, 6), None);

        // The bottom class survives on both.
        assert_eq!(moore.classical_ext_rank(0, 0), 1, "S/2 bottom cell");
        // The h₀-tower (stem 0, filtration ≥ 1): present on the sphere, gone on S/2.
        for s in 1..=4 {
            assert_eq!(sphere.classical_ext_rank(s, s), 1, "sphere h₀^{s}");
            assert_eq!(moore.classical_ext_rank(s, s), 0, "S/2 kills h₀^{s}");
        }
    }

    #[test]
    fn module_descriptor_auto_extends_composite_operations() {
        // The `.json` path now goes through the standard `FDModule::from_json`, backed
        // by the `GeneratedAlgebra` impl of A_C/τ: a descriptor lists only *generator*
        // actions and the action of every composite operation is extended from them.
        // Here x₀ --P(0,1)--> x₂ --Q₀--> x₃; the composite operation `Q_0 P(0, 1)` (a
        // non-generator basis element in degree 3) is never listed, yet must act
        // x₀ ↦ x₃ — the parity win over an explicit-actions-only loader.
        let descriptor = serde_json::json!({
            "gens": { "x0": 0, "x2": 2, "x3": 3 },
            "actions": ["P(0, 1) x0 = x2", "Q_0 x2 = x3"],
        });
        let module = MotivicResolution::module_from_json(&descriptor).unwrap();
        let algebra = module.algebra();
        let (_, q0) = algebra.basis_element_from_string("Q_0").unwrap();
        let (_, p01) = algebra.basis_element_from_string("P(0, 1)").unwrap();

        // The two-step generator action Q₀(P(0,1)·x₀) = Q₀·x₂ = x₃.
        let mut x2 = FpVector::new(TWO, module.dimension(2));
        module.act_on_basis(x2.as_slice_mut(), 1, 2, p01, 0, 0);
        let mut two_step = FpVector::new(TWO, module.dimension(3));
        module.act(two_step.as_slice_mut(), 1, 1, q0, 2, x2.as_slice());
        let mut x3 = FpVector::new(TWO, module.dimension(3));
        x3.add_basis_element(0, 1);
        assert_eq!(two_step, x3, "Q₀(P(0,1)·x₀) should be x₃");

        // The single-step action by the algebra product Q₀·P(0,1) = Q₁ + Q₀P(0,1)
        // (both degree-3 non-generators, extended not listed) must agree — the module
        // axiom that composite auto-extension is responsible for.
        let mut product = FpVector::new(TWO, algebra.dimension(3));
        algebra.multiply_basis_elements(product.as_slice_mut(), 1, 1, q0, 2, p01);
        assert!(
            product.iter_nonzero().count() >= 2,
            "Q₀·P(0,1) is a genuine composite"
        );
        let mut one_step = FpVector::new(TWO, module.dimension(3));
        for (b, _) in product.iter_nonzero() {
            module.act_on_basis(one_step.as_slice_mut(), 1, 3, b, 0, 0);
        }
        assert_eq!(
            one_step, two_step,
            "acting by the extended composite Q₀·P(0,1) must match the two-step action"
        );
    }

    #[test]
    fn module_descriptor_rejects_inconsistent_actions() {
        // A descriptor whose generator actions violate a Steenrod relation is rejected
        // by `check_validity`, using `GeneratedAlgebra::generating_relations`. Here
        // Q₀ x₀ = x₁ and Q₀ x₁ = x₂ would force Q₀² x₀ = x₂ ≠ 0, but Q₀² = 0 in A_C/τ.
        let descriptor = serde_json::json!({
            "gens": { "x0": 0, "x1": 1, "x2": 2 },
            "actions": ["Q_0 x0 = x1", "Q_0 x1 = x2"],
        });
        let err = match MotivicResolution::module_from_json(&descriptor) {
            Err(e) => e,
            Ok(_) => panic!("Q₀² = 0 must reject this descriptor"),
        };
        let msg = format!("{err}");
        assert!(
            msg.contains("Relation failed") && msg.contains("Q_0"),
            "expected a Q_0² relation failure, got: {msg}"
        );
    }

    #[test]
    fn save_load_round_trips() {
        // Resolving with a save directory, then reloading, reproduces the weights and
        // lifted differentials exactly (and hence every downstream invariant).
        let dir = std::env::temp_dir().join(format!("motivic-save-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let max = Bidegree::n_s(8, 5);

        let fresh = MotivicResolution::with_module(
            MotivicResolution::trivial_module(),
            max,
            Some(dir.clone()),
        );
        // A second construction with the same save dir must load, not recompute.
        let loaded = MotivicResolution::with_module(
            MotivicResolution::trivial_module(),
            max,
            Some(dir.clone()),
        );

        assert_eq!(*fresh.weights, *loaded.weights, "weights survive save/load");
        assert_eq!(
            fresh.lifted, loaded.lifted,
            "lifted differentials survive save/load"
        );
        // And a spot-check that downstream data agrees.
        for s in 0..max.s() {
            for n in 0..=8 {
                assert_eq!(
                    fresh.tau_module(s, n + s).torsion,
                    loaded.tau_module(s, n + s).torsion,
                    "τ-module at (n={n}, s={s})"
                );
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn motivic_massey_products_with_tau() {
        // Triple Massey products over F₂[τ], read off the lifted null-homotopy.
        let res = MotivicResolution::new(Bidegree::n_s(10, 7));
        let h0 = Gen { s: 1, t: 1, idx: 0 };
        let h1 = Gen { s: 1, t: 2, idx: 0 };
        let h2 = Gen { s: 1, t: 4, idx: 0 };

        // ⟨h₁, h₀, h₁⟩ = h₀h₂ (τ⁰): the classic Massey relation. The target is the
        // (3,2) generator, which is exactly h₀·h₂.
        let h0h2 = res.motivic_product(h0, h2)[0].0;
        assert_eq!(
            res.motivic_massey(h1, h0, h1),
            vec![(h0h2, 0)],
            "⟨h₁,h₀,h₁⟩ = h₀h₂"
        );

        // ⟨h₀, h₁, h₀⟩ = τ·h₁²: the C-motivic hidden-τ Massey product. The target is
        // the (2,2) generator h₁², carried by τ¹ (the Cτ bracket vanishes).
        let h1sq = res.motivic_product(h1, h1)[0].0;
        assert_eq!(
            res.motivic_massey(h0, h1, h0),
            vec![(h1sq, 1)],
            "⟨h₀,h₁,h₀⟩ = τ·h₁²"
        );

        // Both brackets have zero indeterminacy in this range (Ext vanishes at the
        // relevant source degrees), so they are genuine non-trivial elements.
        for (a, b, c) in [(h1, h0, h1), (h0, h1, h0)] {
            let coset = res.motivic_massey_coset(a, b, c);
            assert!(
                coset.indeterminacy.is_empty(),
                "expected zero indeterminacy"
            );
            assert!(!coset.is_zero, "bracket should be non-trivial");
        }
    }

    #[test]
    fn f2tau_reduce_mod_detects_membership() {
        use super::f2tau::reduce_mod;
        // Over F₂[τ], reduce vectors modulo the submodule spanned by rows.
        // Rows: (1, τ) and (0, τ²). Column 0 pivot is the unit 1.
        let rows = vec![vec![0b1u128, 0b10], vec![0, 0b100]];
        // (1, τ) is in the span → reduces to 0.
        assert!(
            reduce_mod(rows.clone(), vec![0b1, 0b10])
                .iter()
                .all(|&x| x == 0)
        );
        // (1, 0): col-0 pivot kills entry 0, leaving (0, τ) from the first row; then
        // (0, τ) mod (0, τ²) stays τ (τ has lower degree than τ²) → not in submodule.
        assert!(
            reduce_mod(rows.clone(), vec![0b1, 0])
                .iter()
                .any(|&x| x != 0)
        );
        // (0, τ³) = τ·(0, τ²) is in the span → reduces to 0.
        assert!(reduce_mod(rows, vec![0, 0b1000]).iter().all(|&x| x == 0));
    }

    #[test]
    fn motivic_product_recovers_hidden_tau_extension() {
        // The headline hidden extension h₀²h₂ = τ·h₁³, computed from first principles
        // by the F₂[τ] product lift. In the Cτ ring h₀²h₂ = 0 (motivic_ctau_ring_
        // relations); here the τ appears.
        let res = MotivicResolution::new(Bidegree::n_s(8, 6));
        let h0 = Gen { s: 1, t: 1, idx: 0 };
        let h2 = Gen { s: 1, t: 4, idx: 0 };
        let h1cubed = Gen { s: 3, t: 6, idx: 0 }; // the sole generator at (3,3)

        // h₀·h₂ is the Cτ-visible generator at (3,2) (τ⁰).
        let h0h2 = res.motivic_product(h0, h2);
        assert_eq!(
            h0h2,
            vec![(Gen { s: 2, t: 5, idx: 0 }, 0)],
            "h₀h₂ at (3,2), τ⁰"
        );

        // h₀·(h₀h₂) = τ¹·h₁³ — the hidden extension.
        let h0sq_h2 = res.motivic_product(h0, h0h2[0].0);
        assert_eq!(h0sq_h2, vec![(h1cubed, 1)], "h₀²h₂ = τ·h₁³");

        // Sanity: the h₁-tower products stay τ⁰ (h₁ⁿ is the honest Cτ product, no
        // hidden τ), even though h₁ⁿ is τ-torsion for n ≥ 4.
        let h1 = Gen { s: 1, t: 2, idx: 0 };
        let h1sq = res.motivic_product(h1, h1);
        assert_eq!(h1sq, vec![(Gen { s: 2, t: 4, idx: 0 }, 0)], "h₁² τ⁰");
        assert_eq!(
            res.motivic_product(h1, h1sq[0].0),
            vec![(h1cubed, 0)],
            "h₁³ τ⁰"
        );
    }

    #[test]
    fn deformation_sseq_converges_to_classical() {
        // The strong oracle for the full d_r tower: E_∞ survivors summed over weight
        // = the classical Adams E₂ (invert τ). Every differential is validated —
        // a wrong d_r would leave the free rank off at some bidegree.
        let max = Bidegree::n_s(8, 5);
        let res = MotivicResolution::new(max);
        let sseq = res.deformation_sseq();

        let mut einf: HashMap<(i32, i32), usize> = HashMap::new();
        for deg in sseq.iter_degrees() {
            let [n, s, _] = deg.coords();
            let page = sseq.page_data(deg);
            let last = page.len() - 1; // the final (E_∞) page for this degree
            *einf.entry((n, s)).or_default() += page[last].dimension();
        }

        for s in 0..res.max_s() {
            for n in 0..=8 {
                let got = einf.get(&(n, s)).copied().unwrap_or(0);
                // Cross-check the Sseq E_∞ against the independent ExtAlgebra path.
                let want = res
                    .ext()
                    .cohomology_dimension(Bidegree::n_s(n, s))
                    .unwrap_or(0);
                assert_eq!(got, want, "E_∞ free rank at (n={n}, s={s})");
            }
        }
    }

    #[test]
    fn lift_is_a_complex_and_reduces_correctly() {
        // Build once (expensive: the padded resolution plus the lift), then check
        // everything: every generator in the report box has a weight; reducing
        // each lifted differential mod τ recovers the Phase 1 model; and d² = 0
        // over A_C (the corrections worked).
        let max = Bidegree::n_s(8, 5);
        let res = MotivicResolution::new(max);

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
    fn anchor_invert_tau_is_classical_adams_e2() {
        // Anchor 1: inverting τ (the free rank of H(δ), all generators) reproduces
        // the classical Adams E₂ — Ext over the mod-2 Steenrod algebra. We resolve
        // the classical sphere in-process and compare rank-for-rank. This is where
        // the τ-torsion (e.g. the h₁-tower classes beyond h₁⁴) is *killed*: the
        // motivic algebraic-Novikov extras collapse back onto classical Ext.
        use crate::{chain_complex::FreeChainComplex, utils::construct_standard};

        let max = Bidegree::n_s(8, 5);
        let res = MotivicResolution::new(max);

        let classical = construct_standard::<false, _, _>("S_2", None).unwrap();
        classical.compute_through_stem(max);

        // classical_ext_rank(s, t) needs the lift at s+1, so s ≤ max_s − 1.
        for s in 0..res.max_s() {
            for t in s..=(max.n() + s) {
                let n = t - s;
                if n > max.n() {
                    continue;
                }
                let got = res.classical_ext_rank(s, t);
                let want = classical.number_of_gens_in_bidegree(Bidegree::n_s(n, s));
                assert_eq!(
                    got, want,
                    "invert-τ mismatch at (n={n}, s={s}): H(δ) free rank {got} ≠ classical {want}"
                );
            }
        }
    }

    #[test]
    fn anchor_keep_tau_has_h1_tower_torsion() {
        // Anchor 3: keeping τ, the motivic E₂ carries τ-torsion the classical page
        // does not. The cleanest witness is the h₁-tower: h₁ⁿ = (h₁)ⁿ ∈ Ext^{n,2n}
        // is nonzero for all n motivically, but classically h₁⁴ = 0. So at
        // (s,t)=(4,8) the free (classical) rank is 0, yet h₁⁴ itself is a τ-torsion
        // class there — δ(h₁⁴) = τ·y, so τ·[h₁⁴] = 0: an F₂[τ]/τ summand at (4,4).
        let max = Bidegree::n_s(8, 5);
        let res = MotivicResolution::new(max);

        let m = res.tau_module(4, 8);
        assert_eq!(m.free, 0, "classical h₁⁴ should vanish (free rank 0)");
        assert_eq!(
            m.torsion,
            vec![1],
            "h₁⁴ at (s=4, t=8) should be a single F₂[τ]/τ torsion summand"
        );
        assert!(res.has_tau_torsion(4, 8));

        // Sanity: h₁³ (s=3, t=6) is already nonzero classically, so it is free —
        // not flagged as (extra) torsion beyond its classical class.
        let m3 = res.tau_module(3, 6);
        assert_eq!(m3.free, 1, "classical h₁³ should survive");
        assert!(m3.torsion.is_empty(), "h₁³ is free, not τ-torsion");
    }

    #[test]
    fn tau_module_h1_tower_is_free_then_tau_torsion() {
        // The whole h₁-tower h₁ⁿ ∈ Ext^{n,2n} (stem n, filtration n): free (a genuine
        // classical class) for n ≤ 3, then a single F₂[τ]/τ summand for n ≥ 4 — the
        // motivic τ-torsion the classical page cannot see.
        let res = MotivicResolution::new(Bidegree::n_s(10, 9));
        for n in 1..=3 {
            let m = res.tau_module(n, 2 * n);
            assert_eq!(
                (m.free, m.torsion.as_slice()),
                (1, [].as_slice()),
                "h₁^{n} free"
            );
        }
        for n in 4..=8 {
            let m = res.tau_module(n, 2 * n);
            assert_eq!(
                (m.free, m.torsion.as_slice()),
                (0, [1].as_slice()),
                "h₁^{n} = F₂[τ]/τ"
            );
        }
    }

    #[test]
    fn tau_module_torsion_matches_deformation_sseq_sources() {
        // The F₂[τ]-module torsion (SNF of the outgoing δ) is exactly the data the
        // deformation SS carries: a class at (n, s) is F₂[τ]/τ^r-torsion iff it
        // supports a d_r there. Cross-check the local SNF reading against the SS's
        // differential sources over the interior (away from the report edge, where
        // the finite compute box makes the SS over-count).
        let res = MotivicResolution::new(Bidegree::n_s(16, 11));
        let sseq = res.deformation_sseq();

        let mut sources: HashMap<(i32, i32), Vec<i32>> = HashMap::new();
        for b in sseq.iter_degrees() {
            let [n, s, _] = b.coords();
            let diffs = sseq.differentials(b);
            for r in diffs.min_degree()..diffs.len() {
                for _ in 0..diffs[r].get_source_target_pairs().len() {
                    sources.entry((n, s)).or_default().push(r);
                }
            }
        }

        for n in 0..=12 {
            for s in 1..=10 {
                let mut snf: Vec<i32> = res
                    .tau_module(s, n + s)
                    .torsion
                    .iter()
                    .map(|&k| k as i32)
                    .collect();
                snf.sort_unstable();
                let mut ss = sources.get(&(n, s)).cloned().unwrap_or_default();
                ss.sort_unstable();
                assert_eq!(snf, ss, "torsion vs d_r sources at (n={n}, s={s})");
            }
        }
    }
}
