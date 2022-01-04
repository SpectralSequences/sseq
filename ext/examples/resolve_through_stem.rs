//! Resolves a module up to a $(s, n)$ and prints an ASCII depiction of the Ext groups:
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

    let max_n = query::with_default("Max n", "30", str::parse);
    let max_s = query::with_default("Max s", "15", str::parse);

    res.compute_through_stem(max_s, max_n);

    println!("{}", res.graded_dimension_string());

    Ok(())
}
