//! Resolves a module up to an $(n, s)$ and prints an ASCII depiction of the Ext groups:
//! ```text
//! ·
//! ·                                                     ·
//! ·                                                   · ·     ·
//! ·                                                 ·   ·     ·
//! ·                                             ·   ·         ·
//! ·                                     ·       · · ·         ·
//! ·                                   · ·     · · · ·     ·   ·
//! ·                                 ·   ·     · :   · ·   · · ·
//! ·                             ·   ·         · ·   · ·   · · ·
//! ·                     ·       · · ·           ·     ·     · ·
//! ·                   · ·     · · · ·     ·     ·     ·       ·
//! ·                 ·   ·     · :   · ·   · ·   · ·           ·
//! ·             ·   ·         · ·   · :   ·   · ·             ·
//! ·     ·       · · ·         · ·   · · ·   ·                 ·
//! ·   · ·     · · ·           · · ·   ·                       ·
//! · ·   ·       ·               ·
//! ·
//! ```

use ext::chain_complex::FreeChainComplex;
use sseq::coordinates::Bidegree;

fn main() -> anyhow::Result<()> {
    let res = ext::utils::query_module_only("Module", None, false)?;

    let max = Bidegree::n_s(
        query::with_default("Max n", "30", str::parse),
        query::with_default("Max s", "15", str::parse),
    );

    res.compute_through_stem(max);

    println!("{}", res.graded_dimension_string());

    Ok(())
}
