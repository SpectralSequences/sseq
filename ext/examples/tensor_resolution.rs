//! Chart `Ext_A(RP^∞, F_2)` by Nassau's untwisted tensor resolution, for `p = 2`.

use std::sync::Arc;

use algebra::{
    Algebra,
    module::{Module, RealProjectiveSpace},
};
use ext::{
    chain_complex::ChainComplex,
    ext_algebra::{ExtAlgebra, tensor_resolution_ext},
    utils::{construct_standard, unicode_num},
};
use sseq::coordinates::Bidegree;

fn main() -> anyhow::Result<()> {
    ext::utils::init_logging()?;

    eprintln!("Computes Ext_A(RP^∞, F_2) by resolving k and tensoring with RP^∞ (p = 2).");

    let max_n: i32 = query::with_default("Max n", "25", str::parse);
    let max_s: i32 = query::with_default("Max s", "15", str::parse);
    let max = Bidegree::n_s(max_n, max_s);
    let t_max = max.t();

    // The minimal resolution `P_•` of the base field, which is tensored with `M`.
    let resolution = Arc::new(construct_standard::<false, _, _>("S_2", None)?);
    resolution.compute_through_bidegree(Bidegree::s_t(max_s + 1, t_max));
    resolution.algebra().compute_basis(t_max + 1);

    // RP^∞ is infinite, so it cannot be minimally resolved itself; `None` is no top degree.
    let rp_inf = Arc::new(RealProjectiveSpace::new(
        resolution.algebra(),
        1,
        None,
        false,
    ));
    rp_inf.compute_basis(t_max);

    let ext = tensor_resolution_ext(Arc::new(ExtAlgebra::new(resolution)), rp_inf);

    // Rows are s from high to low, columns n. Empty rows at the top are dropped, so the header
    // names the range of s printed.
    let rows: Vec<String> = (0..=max_s)
        .rev()
        .map(|s| {
            (0..=max_n)
                .map(|n| {
                    let dim = ext
                        .cohomology_dimension(Bidegree::n_s(n, s))
                        .expect("computed range");
                    format!("{} ", unicode_num(dim))
                })
                .collect()
        })
        .collect();
    let top_s = max_s - rows.iter().take_while(|row| row.trim().is_empty()).count() as i32;

    if top_s < 0 {
        println!("Ext_A(RP^∞, F_2) vanishes on n = 0..{max_n}, s = 0..{max_s}.");
    } else {
        println!("Ext_A(RP^∞, F_2), rows s = {top_s}..0 (top to bottom), columns n = 0..{max_n}:");
        for row in rows.iter().skip((max_s - top_s) as usize) {
            println!("{}", row.trim_end());
        }
    }

    Ok(())
}
