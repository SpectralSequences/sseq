//! Chart the deformation spectral sequence of a module over $A_C/\tau$, in SVG on stdout.

use std::{collections::BTreeMap, sync::Arc};

use ext::motivic::{Deformation, MotivicResolution};
use sseq::{
    SseqProfile,
    charting::{Backend, SvgBackend},
    coordinates::{Bidegree, BidegreeGenerator, degree::MultiDegree},
};

/// Draw one page of the deformation spectral sequence as an $(n, s)$ Adams chart.
///
/// The spectral sequence is trigraded by stem, Adams filtration and weight, so it cannot go
/// through the bigraded `Sseq::write_to_graph`. A node at $(n, s)$ stacks the classes of every
/// weight there, and $d_r\colon (n, s, w) \to (n+1, s-1, w+r)$ is drawn between the projected
/// nodes. Page 1 is $\mathrm{Ext}_{A_C/\tau}$ with $d_1$; later pages converge to the classical
/// Adams $E_2$, and the classes that die on the way are the motivic $\tau$-torsion.
fn main() -> anyhow::Result<()> {
    ext::utils::init_logging()?;

    let module: String = query::with_default("Module", "Ctau", |s| {
        Ok::<_, std::convert::Infallible>(s.to_owned())
    });
    let max = Bidegree::n_s(
        query::with_default("Max n", "30", str::parse),
        query::with_default("Max s", "17", str::parse),
    );
    let page: i32 = query::with_default("Page", "1", str::parse);

    let resolution = ext::utils::construct_standard::<false, _, _>(module.as_str(), None)?;
    let res = MotivicResolution::new(Arc::new(resolution), max)?;
    let sseq = res.deformation_sseq();

    // Ext at the top filtration would read generators one filtration higher.
    let max_n = res.max().n();
    let max_s = res.max().s() - 1;
    let in_box = |n: i32, s: i32| (0..=max_n).contains(&n) && (0..=max_s).contains(&s);

    // The weights present at each (n, s), sorted to fix the stacking order within a node.
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

    // An (n, s, w) with no classes is never registered, and `page_data` panics on one.
    let dim_at = |n: i32, s: i32, w: i32| {
        let deg = MultiDegree::from([n, s, w]);
        if sseq.defined(deg) {
            sseq.page_data(deg).get_max(page).dimension()
        } else {
            0
        }
    };

    // Where the weight-`w` block starts within the (n, s) node.
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

    // Structlines panic on a node that was not drawn first.
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
        // A target with no classes is unregistered and receives no differential.
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
