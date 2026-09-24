//! Reading the motivic Adams $E_2 = H(\delta)$ as an $\mathbb{F}_2[\tau]$-module.
//!
//! The lifted differential's augmentation part δ ([`MotivicCoboundary`]) is a
//! complex of free $\mathbb{F}_2[\tau]$-modules; its cohomology is the motivic
//! Adams $E_2$. Since δ raises the weight, `{weight ≤ cap}` is a subcomplex and
//! the whole computation is $\mathbb{F}_2$ linear algebra. The free rank is the
//! classical Adams $E_2$; the non-unit invariant factors of the *outgoing* δ are
//! the τ-torsion ([`MotivicResolution::tau_module`], via [`super::f2tau`]).

use std::{collections::HashMap, sync::Arc};

use fp::{matrix::Matrix, prime::TWO};
use sseq::coordinates::Bidegree;

use super::{CTauResolution, Gen, MotivicResolution, f2tau};
use crate::{
    chain_complex::ChainComplex,
    ext_algebra::{ExtAlgebra, ExtDifferential},
};

/// The motivic Adams $E_2$ at one bidegree as an $\mathbb{F}_2[\tau]$-module
/// (structure theorem over the PID $\mathbb{F}_2[\tau]$):
/// $\mathbb{F}_2[\tau]^{\,\mathrm{free}} \oplus \bigoplus_i
/// \mathbb{F}_2[\tau]/\tau^{k_i}$. Produced by [`MotivicResolution::tau_module`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TauModule {
    /// The free rank — the classical Adams $E_2$ (invert $\tau$).
    pub free: usize,
    /// The orders $k_i$ of the $\tau$-torsion summands $\mathbb{F}_2[\tau]/\tau^{k_i}$,
    /// ascending. Empty iff the module is $\tau$-torsion-free.
    pub torsion: Vec<u32>,
}

impl std::fmt::Display for TauModule {
    /// A compact chart label, e.g. `2` (free rank 2, no torsion), `τ` (one
    /// $\mathbb{F}_2[\tau]/\tau$), `1+τ²`, or `·` for the zero module.
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

/// The dualized differential $\delta = \Hom(d, k)$ of the motivic resolution,
/// stored on the [`ExtAlgebra`] side (see the resolution/functor split in
/// [`MotivicResolution`]). It maps each generator to the target generators of the
/// augmentation (unit-operation) part of its lifted $A_C$ differential, together
/// with the forced $\tau$-power. As an [`ExtDifferential`] it shifts by
/// $(n, s) \mapsto (n+1, s-1)$ — δ lowers Novikov filtration at fixed internal
/// degree, which raises the stem — and its cohomology is the motivic Adams $E_2$.
struct MotivicCoboundary {
    resolution: Arc<CTauResolution>,
    /// `g ↦ [(target generator, τ-power)]` for the augmentation part of `d(g)`.
    /// Generators with no δ-terms are absent.
    deltas: HashMap<Gen, Vec<(Gen, u32)>>,
    /// The motivic weight of every generator — the $\mathbb{F}_2[\tau]$ grading
    /// the capped cohomology slices by. A generator absent here is weightless and
    /// excluded from every slice (matching the old `gen_list`). `Arc`-shared with
    /// [`MotivicResolution`], not copied.
    weights: Arc<HashMap<Gen, i32>>,
}

impl MotivicCoboundary {
    /// The generators at filtration `s`, internal degree `t` with weight `≤ cap`
    /// (weightless generators excluded), as their indices.
    fn kept(&self, s: i32, t: i32, cap: i32) -> Vec<usize> {
        (0..self.resolution.module(s).number_of_gens_in_degree(t))
            .filter(|&idx| {
                self.weights
                    .get(&Gen { s, t, idx })
                    .is_some_and(|&w| w <= cap)
            })
            .collect()
    }
}

impl ExtDifferential for MotivicCoboundary {
    fn shift(&self) -> Bidegree {
        Bidegree::n_s(1, -1)
    }

    fn matrix(&self, b: Bidegree) -> Option<Matrix> {
        self.matrix_capped(b, i32::MAX)
    }

    fn graded_dimension(&self, b: Bidegree, cap: i32) -> Option<usize> {
        if b.s() < 0 {
            return Some(0);
        }
        Some(self.kept(b.s(), b.t(), cap).len())
    }

    fn matrix_capped(&self, b: Bidegree, cap: i32) -> Option<Matrix> {
        let (t, s) = (b.t(), b.s());
        if s < 0 {
            return None;
        }
        let rows = self.kept(s, t, cap);
        if s == 0 {
            // δ out of filtration 0 lands in filtration −1: no targets.
            return Some(Matrix::new(TWO, rows.len(), 0));
        }
        let cols = self.kept(s - 1, t, cap);
        let col_pos: HashMap<usize, usize> =
            cols.iter().enumerate().map(|(p, &i)| (i, p)).collect();
        let mut m = Matrix::new(TWO, rows.len(), cols.len());
        for (rp, &idx) in rows.iter().enumerate() {
            if let Some(targets) = self.deltas.get(&Gen { s, t, idx }) {
                for &(tgt, _power) in targets {
                    if let Some(&cp) = col_pos.get(&tgt.idx) {
                        m.row_mut(rp).set_entry(cp, 1);
                    }
                }
            }
        }
        Some(m)
    }
}

impl MotivicResolution {
    /// Build the Ext DGA: apply $\Hom(-, k)$ to the raw lifted differential
    /// (`lifted`), producing the dualized coboundary δ, and hand it to an
    /// [`ExtAlgebra`] over the mod-τ resolution. This is where the $\Hom$ functor
    /// is applied and its result stored on the Ext side; the resolution keeps only
    /// the raw differential.
    fn build_ext(&self) -> ExtAlgebra<CTauResolution> {
        let mut deltas: HashMap<Gen, Vec<(Gen, u32)>> = HashMap::new();
        for s in 1..=self.max_s() {
            let t_max = self.compute.n() + s;
            for t in 0..=t_max {
                for idx in 0..self.num_gens(s, t) {
                    let g = Gen { s, t, idx };
                    if self.lifted.contains_key(&g) && self.weights.contains_key(&g) {
                        let d = self.delta(g);
                        if !d.is_empty() {
                            deltas.insert(g, d);
                        }
                    }
                }
            }
        }
        let coboundary = Arc::new(MotivicCoboundary {
            resolution: Arc::clone(&self.resolution),
            deltas,
            weights: Arc::clone(&self.weights),
        });
        ExtAlgebra::without_unit(Arc::clone(&self.resolution)).with_differential(coboundary)
    }

    /// The Ext DGA (built lazily): the mod-τ resolution wrapped so it stores the
    /// dualized differential δ.
    ///
    /// Its cochain level is `Ext_{A_C/τ}` — the algebraic Novikov $E_2$, equal to
    /// the Adams $E_2$ of $C\tau$ — so its [`multiply`](ExtAlgebra::multiply) and
    /// Massey products are the **$C\tau$ (E₁-page) products**. Its
    /// [`cohomology_dimension`](ExtAlgebra::cohomology_dimension) is the motivic
    /// Adams $E_2$ = `H(δ)`. (Products *on* `H(δ)` — the motivic ring — are the
    /// cochain products descended through the cohomology, which is separate.)
    pub fn ext(&self) -> &ExtAlgebra<CTauResolution> {
        self.ext.get_or_init(|| self.build_ext())
    }

    /// The motivic Adams $E_2$ at `(s, t)` as an $\mathbb{F}_2[\tau]$-module, via the
    /// structure theorem for modules over the PID $\mathbb{F}_2[\tau]$:
    /// $$H(\delta)^{n,s} \;\cong\; \mathbb{F}_2[\tau]^{\,\mathrm{free}} \;\oplus\;
    /// \bigoplus_i \mathbb{F}_2[\tau]/\tau^{k_i}.$$
    /// The free rank is the classical Adams $E_2$ (invert $\tau$); the torsion orders
    /// $k_i$ are the sizes of the $\tau$-torsion summands the classical page cannot
    /// see (e.g. the $h_1$-tower).
    ///
    /// $(C^\bullet, \delta)$ is a complex of *free* $\mathbb{F}_2[\tau]$-modules of
    /// generators. `Ext = H(δ)` is its *cohomology*, so by universal coefficients its
    /// torsion at $s$ is the torsion of the homology one degree down, i.e. the
    /// non-unit invariant factors of the *outgoing* boundary
    /// $\delta\colon \mathrm{gens}(s, t) \to \mathrm{gens}(s{-}1, t)$: a class is
    /// $\tau^{k}$-torsion exactly when its own δ is $\tau^{k}$ times a cycle
    /// (e.g. $\delta(h_1^4) = \tau\, y$ puts $\mathbb{F}_2[\tau]/\tau$ at $h_1^4$).
    /// Those factors are read off the Smith normal form of that δ matrix over
    /// $\mathbb{F}_2[\tau]$ (`tau_torsion_orders`); the only homogeneous prime
    /// is $\tau$, so every invariant factor is a power $\tau^{k_i}$. This is the same
    /// data the deformation SS carries as its $d_r$ differentials *supported* at
    /// `(n, s)` (order $= r$), computed locally here so it is exact up to the resolved
    /// range rather than the SS's report box.
    pub fn tau_module(&self, s: i32, t: i32) -> TauModule {
        TauModule {
            free: self.classical_ext_rank(s, t),
            torsion: self.tau_torsion_orders(s, t),
        }
    }

    /// Whether `(s, t)` carries a $\tau$-torsion class in the motivic $E_2$ — a class
    /// that dies when $\tau$ is inverted, invisible to the classical Adams $E_2$.
    /// The boolean shadow of [`Self::tau_module`].
    pub fn has_tau_torsion(&self, s: i32, t: i32) -> bool {
        !self.tau_torsion_orders(s, t).is_empty()
    }

    /// The orders $k_i$ of the $\tau$-torsion summands $\mathbb{F}_2[\tau]/\tau^{k_i}$
    /// of the motivic $E_2$ at `(s, t)`, ascending: the non-unit invariant factors
    /// (as powers of $\tau$) of the outgoing coboundary
    /// $\delta\colon \mathrm{gens}(s, t) \to \mathrm{gens}(s{-}1, t)$, over
    /// $\mathbb{F}_2[\tau]$. (Outgoing, not incoming: `Ext = H(δ)` is cohomology, so
    /// the torsion sits on the class whose own δ is $\tau$-divisible.)
    fn tau_torsion_orders(&self, s: i32, t: i32) -> Vec<u32> {
        if s < 1 {
            return Vec::new(); // the s = 0 unit is free; no outgoing δ.
        }
        let rows = self.num_gens(s, t);
        let cols = self.num_gens(s - 1, t);
        if rows == 0 || cols == 0 {
            return Vec::new();
        }
        // The δ matrix over F₂[τ]: entry (i, j) is the sum of τ^power over the terms
        // of δ(gᵢ) hitting the j-th generator at (s-1, t). Packed as a bitmask of
        // τ-exponents (F₂ coefficients).
        let mut m = vec![vec![0u128; cols]; rows];
        for (ri, row) in m.iter_mut().enumerate() {
            for (gj, power) in self.delta(Gen { s, t, idx: ri }) {
                if gj.s == s - 1 && gj.idx < cols {
                    row[gj.idx] ^= 1u128 << power;
                }
            }
        }
        let mut orders: Vec<u32> = f2tau::invariant_factors(m)
            .into_iter()
            .map(|f| f2tau::deg(f) as u32)
            .collect();
        orders.sort_unstable();
        orders
    }
}
