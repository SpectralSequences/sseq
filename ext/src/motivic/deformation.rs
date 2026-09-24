//! The deformation (algebraic Novikov / τ-Bockstein) spectral sequence: assemble
//! the trigraded [`Sseq`] whose $E_1 = \mathrm{Ext}_{A_C/\tau}$ and whose $d_r$ are
//! the τ-Bockstein differentials read off the lifted δ. Inverting τ gives the
//! classical Adams $E_2$ ([`MotivicResolution::classical_ext_rank`]); the
//! finite-page deaths are the motivic τ-torsion.

use std::collections::HashMap;

use fp::{prime::TWO, vector::FpVector};
use sseq::{
    Sseq, SseqProfile,
    coordinates::{degree::MultiDegree, element::MultiDegreeElement},
};

use super::{Gen, MotivicResolution};

/// The direction of the **deformation spectral sequence** (the algebraic Novikov
/// / $\tau$-Bockstein SS), trigraded by (stem $n$, Adams filtration $s$, weight
/// $w$). Each $d_r$ carries δ's fixed $(n, s) \mapsto (n+1, s-1)$ shift — δ lowers
/// Novikov filtration at fixed internal degree, so the stem rises — and jumps the
/// weight by $r$ (the $\tau$-power). $E_1 = \Ext_{A_C/\tau}$; inverting $\tau$
/// ($w \to \infty$) gives $E_\infty = $ classical $\Ext_A$, and finite-page deaths
/// are the motivic $\tau$-torsion.
pub struct Deformation;

impl SseqProfile<3> for Deformation {
    const MIN_R: i32 = 1;

    fn profile(r: i32, b: MultiDegree<3>) -> MultiDegree<3> {
        b + MultiDegree::from([1, -1, r])
    }

    fn profile_inverse(r: i32, b: MultiDegree<3>) -> MultiDegree<3> {
        b + MultiDegree::from([-1, 1, -r])
    }

    fn differential_length(offset: MultiDegree<3>) -> i32 {
        offset.coords()[2] // the weight component
    }
}

impl MotivicResolution {
    /// Group generators by their multidegree `(n, s, w)`. Returns the per-multidegree
    /// generator-index lists (a generator's position in its list is its Sseq
    /// coordinate) and the reverse map `Gen ↦ (multidegree, position)`.
    fn sseq_grouping(
        &self,
    ) -> (
        HashMap<[i32; 3], Vec<usize>>,
        HashMap<Gen, (MultiDegree<3>, usize)>,
    ) {
        let mut groups: HashMap<[i32; 3], Vec<usize>> = HashMap::new();
        let mut pos: HashMap<Gen, (MultiDegree<3>, usize)> = HashMap::new();
        for s in 0..=self.max_s() {
            let t_max = self.compute.n() + s;
            for t in 0..=t_max {
                for idx in 0..self.num_gens(s, t) {
                    let g = Gen { s, t, idx };
                    if let Some(&w) = self.weights.get(&g) {
                        let key = [t - s, s, w];
                        let list = groups.entry(key).or_default();
                        pos.insert(g, (MultiDegree::from(key), list.len()));
                        list.push(idx);
                    }
                }
            }
        }
        (groups, pos)
    }

    /// The deformation spectral sequence as an [`Sseq`], trigraded by $(n, s, w)$
    /// ([`Deformation`]). $E_1 = \Ext_{A_C/\tau}$ — the mod-τ generators, grouped
    /// by weight — with $d_1$ the **weight-1 part of δ** (the induced differential
    /// on the associated graded, read directly off [`delta`](Self::delta)).
    /// [`Sseq::update`] then computes $E_2 = H(d_1)$, whose [`page_data`] are the
    /// `Subquotient`s.
    ///
    /// The higher $d_r$ are then computed by the **τ-Bockstein zig-zag** on the full
    /// $\mathbb{F}_2[\tau]$-linear δ (`slice`/`inject`/`apply_delta` below), pushing
    /// $\delta(\tilde x)$ up in weight one order at a time — each correction solved by
    /// $d_1$'s stored quasi-inverse — and reading the weight-$(w+r)$ part, until a
    /// page adds nothing ($E_\infty$). The products (via [`Sseq::multiply`]) build on
    /// this.
    ///
    /// [`page_data`]: Sseq::page_data
    pub fn deformation_sseq(&self) -> &Sseq<3, Deformation> {
        self.deformation
            .get_or_init(|| self.build_deformation_sseq())
    }

    #[tracing::instrument(
        skip(self),
        fields(max = %self.max, top_page = tracing::field::Empty, num_higher_diffs = tracing::field::Empty)
    )]
    fn build_deformation_sseq(&self) -> Sseq<3, Deformation> {
        let mut sseq = Sseq::<3, Deformation>::new(TWO);

        // Group generators by (n, s, w) — a generator's position within its group
        // is its coordinate in that multidegree.
        let (groups, pos) = self.sseq_grouping();
        for (&key, list) in &groups {
            sseq.set_dimension(MultiDegree::from(key), list.len());
        }

        // d₁ = the weight-1 part of δ; a generator with δ = ∅ is a permanent cycle.
        for (&g, &(deg, p)) in &pos {
            let mut source = FpVector::new(TWO, sseq.dimension(deg));
            source.set_entry(p, 1);
            // s = 0 is the unit: no differential (δ is only defined for s ≥ 1).
            if g.s == 0 {
                sseq.add_permanent_class(&MultiDegreeElement::new(deg, source));
                continue;
            }
            let d = self.delta(g);
            if d.is_empty() {
                sseq.add_permanent_class(&MultiDegreeElement::new(deg, source));
                continue;
            }
            // d₁ = the weight-1 part of δ. A δ with no weight-1 term makes g a
            // d₁-cycle (its leading δ term is higher order); it is left to the
            // τ-Bockstein zig-zag below. Checking this before sizing `target` also
            // avoids `sseq.dimension(profile(1, deg))` on an empty d₁-target degree:
            // that degree need not carry any generators (e.g. at the top-stem edge of
            // the box), and `Sseq::dimension` panics on a degree that was never
            // `set_dimension`'d. A weight-1 term, by contrast, points to a generator
            // *at* that degree, so its presence guarantees the degree is populated.
            if !d.iter().any(|&(_, power)| power == 1) {
                continue;
            }
            let target_deg = Deformation::profile(1, deg);
            let mut target = FpVector::new(TWO, sseq.dimension(target_deg));
            for (gj, power) in d {
                if power == 1 {
                    target.set_entry(pos[&gj].1, 1);
                }
            }
            sseq.add_differential(1, &MultiDegreeElement::new(deg, source), target.as_slice());
        }
        sseq.update();

        // Higher d_r by the τ-Bockstein zig-zag on the full δ. `apply_delta`
        // evaluates δ on a raw cochain over the generators at (s, t); `slice`
        // extracts a fixed-weight part into (n,s,w)-group coordinates; `inject` adds
        // a group-coordinate vector back into a raw cochain.
        let apply_delta = |xtilde: &FpVector, s: i32, t: i32| -> FpVector {
            let mut dvec = FpVector::new(TWO, self.num_gens(s - 1, t));
            for (gi, _) in xtilde.iter_nonzero() {
                for (gj, _power) in self.delta(Gen { s, t, idx: gi }) {
                    dvec.add_basis_element(gj.idx, 1);
                }
            }
            dvec
        };
        let slice = |dvec: &FpVector, key: [i32; 3]| -> FpVector {
            let g = groups.get(&key).map(Vec::as_slice).unwrap_or(&[]);
            let mut y = FpVector::new(TWO, g.len());
            for (p, &gi) in g.iter().enumerate() {
                if dvec.entry(gi) != 0 {
                    y.set_entry(p, 1);
                }
            }
            y
        };
        let inject = |xtilde: &mut FpVector, key: [i32; 3], gv: &FpVector| {
            if let Some(g) = groups.get(&key) {
                for (p, _) in gv.iter_nonzero() {
                    xtilde.add_basis_element(g[p], 1);
                }
            }
        };

        let degrees: Vec<MultiDegree<3>> = sseq.iter_degrees().collect();
        let mut r = 2;
        let mut num_higher_diffs = 0usize;
        loop {
            let mut to_add: Vec<(MultiDegree<3>, FpVector, FpVector)> = Vec::new();
            for &b in &degrees {
                let [n, s, w] = b.coords();
                // Only report-box sources: their δ-targets (stem n+1 ≤ compute.n())
                // are resolved, and every differential touching a report degree has
                // a report-box source, so the report E_∞ is unaffected. Margin
                // sources would reach stem max.n+2 (unresolved).
                if s < 1 || n > self.max.n() {
                    continue;
                }
                let page = sseq.page_data(b);
                if page.len() <= r {
                    continue;
                }
                let t = n + s;
                for rep in page[r].gens() {
                    // x̃ = the E_r representative, lifted to a raw cochain over (s, t).
                    let mut xtilde = FpVector::new(TWO, self.num_gens(s, t));
                    if let Some(g) = groups.get(&[n, s, w]) {
                        for (p, _) in rep.iter_nonzero() {
                            xtilde.add_basis_element(g[p], 1);
                        }
                    }
                    // Push δ(x̃) up in weight, correcting each intermediate order.
                    let mut ok = true;
                    for k in 1..r {
                        let y = slice(&apply_delta(&xtilde, s, t), [n + 1, s - 1, w + k]);
                        if y.is_zero() {
                            continue;
                        }
                        let src = MultiDegree::from([n, s, w + k - 1]);
                        if !sseq.defined(src) || sseq.differentials(src).len() <= 1 {
                            ok = false; // no d₁ to invert (should not happen for a genuine cycle)
                            break;
                        }
                        let mut c = FpVector::new(TWO, sseq.dimension(src));
                        sseq.differentials(src)[1].quasi_inverse(c.as_slice_mut(), y.as_slice());
                        inject(&mut xtilde, [n, s, w + k - 1], &c);
                    }
                    if !ok {
                        continue;
                    }
                    let target = slice(&apply_delta(&xtilde, s, t), [n + 1, s - 1, w + r]);
                    if !target.is_zero() {
                        to_add.push((b, rep.to_owned(), target));
                    }
                }
            }
            let mut added = false;
            for (b, source, target) in to_add {
                if sseq.add_differential(r, &MultiDegreeElement::new(b, source), target.as_slice())
                {
                    added = true;
                    num_higher_diffs += 1;
                }
            }
            sseq.update();
            if !added {
                break;
            }
            r += 1;
        }
        let span = tracing::Span::current();
        span.record("top_page", r);
        span.record("num_higher_diffs", num_higher_diffs);

        sseq
    }

    /// The classical Adams $E_2$ rank at `(s, t)` — invert $\tau$: the free rank of
    /// the motivic $E_2$.
    ///
    /// Read off the deformation SS: the $E_\infty$ survivors at `(n, s)` summed over
    /// weight. Inverting $\tau$ ($w \to \infty$) is exactly the classical Adams
    /// $E_2$; the τ-torsion classes die on finite pages and drop out.
    pub fn classical_ext_rank(&self, s: i32, t: i32) -> usize {
        let n = t - s;
        let sseq = self.deformation_sseq();
        sseq.iter_degrees()
            .filter(|d| {
                let c = d.coords();
                c[0] == n && c[1] == s
            })
            .map(|d| {
                let page = sseq.page_data(d);
                page[page.len() - 1].dimension() // E_∞ at (n, s, w)
            })
            .sum()
    }
}
