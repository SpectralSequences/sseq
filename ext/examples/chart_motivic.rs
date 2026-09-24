//! Render the deformation (algebraic Novikov / τ-Bockstein) spectral sequence of
//! $A_C/\tau$ as an $(n, s)$ Adams chart, in SVG on stdout.
//!
//! The deformation SS is trigraded — stem $n$, Adams filtration $s$, motivic
//! weight $w$ — so it cannot go through the bigraded `Sseq::write_to_graph`.
//! Instead we project onto the $(n, s)$ plane: a node at $(n, s)$ stacks the
//! classes of every weight there, and the τ-Bockstein differentials
//! $d_r\colon (n, s, w) \to (n{+}1, s{-}1, w{+}r)$ are drawn between the projected
//! nodes.
//!
//! At page `1` the nodes are $E_1 = \Ext_{A_C/\tau}$ (the algebraic Novikov $E_2$,
//! equivalently the Adams $E_2$ of $C\tau$) and the structlines are $d_1 = \delta$.
//! Inverting $\tau$ ($w \to \infty$) leaves $E_\infty = $ the classical Adams
//! $E_2$; the finite-page deaths are the motivic τ-torsion. Ask for a larger page
//! to watch the SS converge.
//!
//! Prompts for `Max n`, `Max s`, and the `Page` to draw (defaulting to 30 / 17 / 1).

use std::collections::BTreeMap;

use ext::motivic::{Deformation, MotivicResolution};
use sseq::{
    SseqProfile,
    charting::{Backend, SvgBackend},
    coordinates::{Bidegree, BidegreeGenerator, degree::MultiDegree},
};

fn main() -> anyhow::Result<()> {
    ext::utils::init_logging()?;

    let max = Bidegree::n_s(
        query::with_default("Max n", "30", str::parse),
        query::with_default("Max s", "17", str::parse),
    );
    let page: i32 = query::with_default("Page", "1", str::parse);

    let res = MotivicResolution::new(max);
    let sseq = res.deformation_sseq();

    // The reported box: stems `0..=max.n()`, filtrations `0..=max.s()-1` (the top
    // filtration needs the lift one degree higher, so it is not reported).
    let max_n = res.max().n();
    let max_s = res.max().s() - 1;
    let in_box = |n: i32, s: i32| (0..=max_n).contains(&n) && (0..=max_s).contains(&s);

    // Group the degrees of the reported box by (n, s), collecting the weights
    // present at each — sorted, so the stacking order (and hence the per-weight
    // offset within a projected node) is deterministic.
    let mut weights_at: BTreeMap<(i32, i32), Vec<i32>> = BTreeMap::new();
    for b in sseq.iter_degrees() {
        let [n, s, w] = b.coords();
        if in_box(n, s) && sseq.page_data(b).get_max(page).dimension() > 0 {
            weights_at.entry((n, s)).or_default().push(w);
        }
    }
    for ws in weights_at.values_mut() {
        ws.sort_unstable();
    }

    // The Page-`page` dimension of the (n, s, w) slice. The deformation SS is
    // sparse — an (n, s, w) with no classes was never registered, and `page_data`
    // panics on an unregistered degree — so an undefined slice is dimension 0.
    let dim_at = |n: i32, s: i32, w: i32| {
        let deg = MultiDegree::from([n, s, w]);
        if sseq.defined(deg) {
            sseq.page_data(deg).get_max(page).dimension()
        } else {
            0
        }
    };

    // Flat index of the weight-`w` block within the projected (n, s) node: the
    // total dimension of all lighter weights present there.
    let offset_at = |n: i32, s: i32, w: i32| -> usize {
        weights_at
            .get(&(n, s))
            .into_iter()
            .flatten()
            .take_while(|&&ww| ww < w)
            .map(|&ww| dim_at(n, s, ww))
            .sum()
    };

    let mut backend = SvgBackend::new(std::io::stdout());
    backend.init(Bidegree::n_s(max_n, max_s))?;

    // Draw every node first: get_coords (used by structlines) panics unless the
    // node was registered.
    for (&(n, s), ws) in &weights_at {
        let total: usize = ws.iter().map(|&w| dim_at(n, s, w)).sum();
        backend.node(Bidegree::n_s(n, s), total)?;
    }

    // Then the page-`page` τ-Bockstein differentials, projected onto (n, s).
    for b in sseq.iter_degrees() {
        let [n, s, w] = b.coords();
        if !in_box(n, s) {
            continue;
        }
        let target = Deformation::profile(page, b);
        let [tn, ts, tw] = target.coords();
        if !in_box(tn, ts) {
            continue;
        }
        // `target` cleared the (n, s) *range* check, but the SS is sparse: an
        // in-range slice with no classes was never registered, and
        // `page_data`/`differentials` panic on an unregistered degree. A d_page out
        // of `b` can only land where classes exist, so an unregistered target
        // carries no structlines — skip it.
        if !sseq.defined(target) {
            continue;
        }

        let diffs = sseq.differentials(b);
        if page < diffs.min_degree() || page >= diffs.len() {
            continue;
        }
        let source_data = sseq.page_data(b).get_max(page);
        let target_data = sseq.page_data(target).get_max(page);
        let off_src = offset_at(n, s, w);
        let off_tgt = offset_at(tn, ts, tw);

        for (mut src, mut tgt) in diffs[page].get_source_target_pairs() {
            let src = source_data.reduce(src.as_slice_mut());
            let tgt = target_data.reduce(tgt.as_slice_mut());
            for (i, &sv) in src.iter().enumerate() {
                if sv == 0 {
                    continue;
                }
                for (j, &tv) in tgt.iter().enumerate() {
                    if tv == 0 {
                        continue;
                    }
                    backend.structline(
                        BidegreeGenerator::new(Bidegree::n_s(n, s), off_src + i),
                        BidegreeGenerator::new(Bidegree::n_s(tn, ts), off_tgt + j),
                        Some(&format!("d{page}")),
                    )?;
                }
            }
        }
    }

    Ok(())
}
