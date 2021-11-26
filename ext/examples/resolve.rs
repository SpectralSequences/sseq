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
//!
use ext::chain_complex::{ChainComplex, FreeChainComplex};
use ext::utils::construct;
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let mut res = query::with_default("Module", "S_2", |name| {
        construct(
            name,
            query::optional("Save directory", |filename| {
                core::result::Result::<PathBuf, std::convert::Infallible>::Ok(PathBuf::from(
                    filename,
                ))
            }),
        )
    });
    res.load_quasi_inverse = false;

    let max_s = query::with_default("Max s", "15", str::parse);
    let max_t = query::with_default("Max t", "30", str::parse);

    #[cfg(not(feature = "concurrent"))]
    res.compute_through_bidegree(max_s, max_t);

    #[cfg(feature = "concurrent")]
    {
        let bucket = ext::utils::query_bucket();
        res.compute_through_bidegree_concurrent(max_s, max_t, &bucket);
    }

    println!("{}", res.graded_dimension_string());
    Ok(())
}
