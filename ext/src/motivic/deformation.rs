//! The deformation (algebraic Novikov / τ-Bockstein) spectral sequence of the lift.

use std::collections::HashMap;

use fp::{matrix::Matrix, prime::TWO, vector::FpVector};
use once::MultiIndexed;
use sseq::{
    Product, Sseq, SseqProfile,
    coordinates::{Bidegree, BidegreeGenerator, degree::MultiDegree, element::MultiDegreeElement},
};

use super::MotivicResolution;

/// The direction of the deformation spectral sequence, trigraded by (stem $n$, Adams filtration
/// $s$, weight $w$).
///
/// Each $d_r$ carries δ's $(n, s) \mapsto (n+1, s-1)$ shift and raises the weight by $r$, the
/// $\tau$-power. $E_1 = \mathrm{Ext}_{A_C/\tau}$; inverting $\tau$ leaves $E_\infty$ = classical
/// $\mathrm{Ext}_A$, and the classes that die on finite pages are the motivic $\tau$-torsion.
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
        offset.coords()[2]
    }
}

/// The generators in each multidegree `(n, s, w)`, and where each generator sits among them.
///
/// A generator's position in its multidegree's list is its coordinate there.
struct Grouping {
    /// The generator indices in each `(n, s, w)`.
    groups: HashMap<[i32; 3], Vec<usize>>,
    /// Each generator's multidegree and position.
    position: HashMap<BidegreeGenerator, (MultiDegree<3>, usize)>,
}

impl MotivicResolution {
    /// Group the resolved generators by multidegree `(n, s, w)`.
    fn sseq_grouping(&self) -> Grouping {
        let mut groups: HashMap<[i32; 3], Vec<usize>> = HashMap::new();
        let mut position = HashMap::new();
        for s in 0..=self.max_s() {
            for t in 0..=self.compute.n() + s {
                for idx in 0..self.num_gens(s, t) {
                    let g = BidegreeGenerator::s_t(s, t, idx);
                    let key = [t - s, s, self.weights[&g]];
                    let list = groups.entry(key).or_default();
                    position.insert(g, (MultiDegree::from(key), list.len()));
                    list.push(idx);
                }
            }
        }
        Grouping { groups, position }
    }

    /// Multiplication by each generator in `gens` on [`Self::deformation_sseq`], for
    /// [`Sseq::multiply`].
    ///
    /// The product is that of $\mathrm{Ext}_{A_C/\tau}$, from [`Self::ext`], split by weight.
    /// On $E_1$ it is the ring $\mathrm{Ext}_{A_C/\tau}$, and on $E_\infty$ the associated graded
    /// of the motivic Adams $E_2$.
    ///
    /// # Panics
    ///
    /// If the resolved module is not the unit.
    pub fn deformation_products(&self, gens: &[BidegreeGenerator]) -> Vec<Product<3>> {
        self.assert_unit();
        let ext = self.ext();
        let Grouping { groups, .. } = self.sseq_grouping();
        gens.iter()
            .map(|&a| {
                let a_w = self.weights[&a];
                let a_elem = ext.generator(a);
                let matrices = MultiIndexed::new();
                for (&[n, s, w], sources) in &groups {
                    let Some(full) = ext.multiply_into(&a_elem, Bidegree::n_s(n, s)) else {
                        continue;
                    };
                    let targets = groups
                        .get(&[n + a.n(), s + a.s(), w + a_w])
                        .map(Vec::as_slice)
                        .unwrap_or(&[]);
                    // Row `i` is `a` times source generator `i`, in the target's coordinates.
                    let rows: Vec<Vec<u32>> = sources
                        .iter()
                        .map(|&i| targets.iter().map(|&j| full.row(i).entry(j)).collect())
                        .collect();
                    matrices.insert(MultiDegree::from([n, s, w]), Matrix::from_vec(TWO, &rows));
                }
                Product {
                    b: MultiDegree::from([a.n(), a.s(), a_w]),
                    left: true,
                    matrices,
                }
            })
            .collect()
    }

    /// The deformation spectral sequence, trigraded by $(n, s, w)$ ([`Deformation`]).
    ///
    /// $E_1$ is the mod-$\tau$ generators grouped by weight, and $d_1$ is the weight-1 part of
    /// [`delta`](Self::delta). The higher $d_r$ come from the $\tau$-Bockstein zig-zag on the
    /// whole of δ: lift a class, push $\delta$ of it up one weight at a time, cancelling each
    /// order with $d_1$'s quasi-inverse, and read off the weight-$(w + r)$ part. Pages are added
    /// until one adds no differential.
    pub fn deformation_sseq(&self) -> &Sseq<3, Deformation> {
        self.deformation
            .get_or_init(|| self.build_deformation_sseq())
    }

    /// Build [`Self::deformation_sseq`].
    #[tracing::instrument(
        skip(self),
        fields(
            max = %self.max,
            top_page = tracing::field::Empty,
            num_higher_diffs = tracing::field::Empty,
        )
    )]
    fn build_deformation_sseq(&self) -> Sseq<3, Deformation> {
        let mut sseq = Sseq::<3, Deformation>::new(TWO);
        let Grouping { groups, position } = self.sseq_grouping();
        for (&key, list) in &groups {
            sseq.set_dimension(MultiDegree::from(key), list.len());
        }

        for (&g, &(deg, p)) in &position {
            let mut source = FpVector::new(TWO, sseq.dimension(deg));
            source.set_entry(p, 1);
            let d = if g.s() == 0 {
                Vec::new()
            } else {
                self.delta(g)
            };
            if d.is_empty() {
                sseq.add_permanent_class(&MultiDegreeElement::new(deg, source));
                continue;
            }
            // A δ with no weight-1 term leaves `g` a d₁-cycle for the zig-zag below. Its d₁
            // target degree may hold no generators, and `Sseq::dimension` panics on one.
            if !d.iter().any(|&(_, power)| power == 1) {
                continue;
            }
            let target_deg = Deformation::profile(1, deg);
            let mut target = FpVector::new(TWO, sseq.dimension(target_deg));
            for (gj, power) in d {
                if power == 1 {
                    target.set_entry(position[&gj].1, 1);
                }
            }
            sseq.add_differential(1, &MultiDegreeElement::new(deg, source), target.as_slice());
        }
        sseq.update();

        // δ on a cochain over the generators at `(s, t)`.
        let apply_delta = |xtilde: &FpVector, s: i32, t: i32| -> FpVector {
            let mut dvec = FpVector::new(TWO, self.num_gens(s - 1, t));
            for (gi, _) in xtilde.iter_nonzero() {
                for (gj, _) in self.delta(BidegreeGenerator::s_t(s, t, gi)) {
                    dvec.add_basis_element(gj.idx(), 1);
                }
            }
            dvec
        };
        // The weight part of a cochain lying in `key`, in that multidegree's coordinates.
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
        // Add a vector in `key`'s coordinates back into a cochain.
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
                // Every differential touching the box has its source in the box, whose δ is
                // exact; a margin source's δ is not.
                if s < 1 || n > self.max.n() {
                    continue;
                }
                let page = sseq.page_data(b);
                if page.len() <= r {
                    continue;
                }
                let t = n + s;
                for rep in page[r].gens() {
                    let mut xtilde = FpVector::new(TWO, self.num_gens(s, t));
                    inject(&mut xtilde, [n, s, w], &rep.to_owned());
                    let mut lifted = true;
                    for k in 1..r {
                        let y = slice(&apply_delta(&xtilde, s, t), [n + 1, s - 1, w + k]);
                        if y.is_zero() {
                            continue;
                        }
                        let src = MultiDegree::from([n, s, w + k - 1]);
                        // An E_r class has every lower order cancellable by d₁.
                        if !sseq.defined(src) || sseq.differentials(src).len() <= 1 {
                            lifted = false;
                            break;
                        }
                        let mut c = FpVector::new(TWO, sseq.dimension(src));
                        sseq.differentials(src)[1].quasi_inverse(c.as_slice_mut(), y.as_slice());
                        inject(&mut xtilde, [n, s, w + k - 1], &c);
                    }
                    if !lifted {
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

    /// The classical Adams $E_2$ rank at `(s, t)`: the free rank of the motivic $E_2$, which
    /// inverting $\tau$ leaves.
    ///
    /// It is the $E_\infty$ of [`Self::deformation_sseq`] at `(n, s)`, summed over weight.
    pub fn classical_ext_rank(&self, s: i32, t: i32) -> usize {
        let n = t - s;
        let sseq = self.deformation_sseq();
        sseq.iter_degrees()
            .filter(|d| {
                let [dn, ds, _] = d.coords();
                dn == n && ds == s
            })
            .map(|d| {
                let page = sseq.page_data(d);
                page[page.len() - 1].dimension()
            })
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use sseq::coordinates::Bidegree;

    use super::*;
    use crate::{chain_complex::FreeChainComplex, utils::construct_standard};

    /// Lift the resolution of the module spec `spec` through `max`.
    fn lift(spec: &str, max: Bidegree) -> MotivicResolution {
        let resolution = construct_standard::<false, _, _>(spec, None).unwrap();
        MotivicResolution::new(Arc::new(resolution), max).unwrap()
    }

    #[test]
    fn e1_is_the_mod_tau_ext() {
        let max = Bidegree::n_s(8, 5);
        let res = lift("Ctau", max);
        let sseq = res.deformation_sseq();

        let mut totals: HashMap<(i32, i32), usize> = HashMap::new();
        for deg in sseq.iter_degrees() {
            let [n, s, _] = deg.coords();
            *totals.entry((n, s)).or_default() += sseq.dimension(deg);
        }
        for (&(n, s), &total) in &totals {
            if n <= max.n() && s <= max.s() {
                assert_eq!(
                    total,
                    res.algebraic_novikov_rank(s, n + s),
                    "weight slices at (n={n}, s={s})"
                );
            }
        }

        // δ(h₀) = 0, so h₀ is a permanent cycle.
        let h0 = MultiDegree::from([0, 1, 0]);
        assert_eq!(sseq.get_dimension(h0), Some(1));
        assert_eq!(sseq.permanent_classes(h0).dimension(), 1);

        for deg in sseq.iter_degrees() {
            assert!(
                !sseq.inconsistent(deg),
                "inconsistent differential at {deg}"
            );
        }
    }

    /// Inverting τ, the free rank of the motivic $E_2$ is the classical Adams $E_2$.
    #[test]
    fn inverting_tau_gives_the_classical_ext() {
        let max = Bidegree::n_s(8, 5);
        for (motivic, classical) in [("Ctau", "S_2"), ("C2_sm_Ctau", "C2")] {
            let res = lift(motivic, max);
            let classical = construct_standard::<false, _, _>(classical, None).unwrap();
            classical.compute_through_stem(max);
            // Ext at `s` reads the generators at `s + 1`.
            for s in 0..max.s() {
                for n in 0..=max.n() {
                    assert_eq!(
                        res.classical_ext_rank(s, n + s),
                        classical.number_of_gens_in_bidegree(Bidegree::n_s(n, s)),
                        "{motivic} at (n={n}, s={s})"
                    );
                }
            }
        }
    }
}
