//! Resolves a module up to a fixed $(s, t)$ and prints an ASCII depiction of the Ext groups:
//! ```text
//! ·                                     ·
//! ·                                   · ·
//! ·                                 ·   ·
//! ·                             ·   ·         ·
//! ·                     ·       · · ·           ·
//! ·                   · ·     · · · ·     ·     ·
//! ·                 ·   ·     · :   · ·   · ·   · ·
//! ·             ·   ·         · ·   · :   ·   · ·
//! ·     ·       · · ·         · ·   · · ·   ·
//! ·   · ·     · · ·           · · ·   ·
//! · ·   ·       ·               ·
//! ·
//! ```

use ext::chain_complex::{ChainComplex};
use sseq::coordinates::Bidegree;
use algebra::module::Module;

fn main() -> anyhow::Result<()> {
    ext::utils::init_logging()?;

    let res = ext::utils::query_module_only("Module", None, false)?;

    let t = query::with_default("Max t", "30", str::parse);
    let s = query::with_default("Max s", "15", str::parse);

    let max = Bidegree::s_t(s, t);
    res.compute_through_bidegree(max);

    println!("E2 page:");
    for s in 0..=max.s() {
        for t in s..=max.t() {
            let dim = res.module(s).dimension(t);
            if dim > 0 {
                println!("({},{}): Z/2Z^{}", s, t, dim);
            }
        }
    }

    // println!("{}", res.graded_dimension_string());
    Ok(())
}
