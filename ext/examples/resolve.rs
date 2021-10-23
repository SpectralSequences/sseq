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
use saveload::Save;
use std::fs::File;

fn main() -> error::Result {
    let res = query::with_default("Module", "S_2", |name| {
        construct(
            name,
            query::optional("Load from save?", |filename| std::fs::File::open(filename)),
        )
    });

    let max_s = query::with_default("Max s", "15", str::parse);
    let max_t = query::with_default("Max t", "30", str::parse);
    // Clippy false positive
    #[allow(clippy::redundant_closure)]
    let save_file: Option<File> = query::optional("Save file", |s| File::create(s));

    #[cfg(not(feature = "concurrent"))]
    res.compute_through_bidegree(max_s, max_t);

    #[cfg(feature = "concurrent")]
    {
        let bucket = ext::utils::query_bucket();
        res.compute_through_bidegree_concurrent(max_s, max_t, &bucket);
    }

    println!("{}", res.graded_dimension_string());

    if let Some(file) = save_file {
        let mut file = std::io::BufWriter::new(file);
        res.save(&mut file)?;
    }
    Ok(())
}
