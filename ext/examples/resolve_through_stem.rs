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

fn main() -> anyhow::Result<()> {
    let res = ext::utils::query_module_only("Module", None, false)?;

    let max_n = query::with_default("Max n", "30", str::parse);
    let max_s = query::with_default("Max s", "15", str::parse);

    res.compute_through_stem(max_s, max_n);

    println!("{}", res.graded_dimension_string());

    Ok(())
}
