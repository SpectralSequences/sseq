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
use saveload::Save;
use std::fs::File;

fn main() -> error::Result {
    let res = query::with_default("Module", "S_2", |name| {
        construct(
            name,
            query::optional("Load from save?", |filename| File::open(filename)),
        )
    });

    let max_s = query::with_default("Max s", "15", str::parse);
    let max_n = query::with_default("Max n", "30", str::parse);
    // Clippy false positive
    #[allow(clippy::redundant_closure)]
    let save_file: Option<File> = query::optional("Save file", |s| File::create(s));

    #[cfg(not(feature = "concurrent"))]
    res.compute_through_stem(max_s, max_n);

    #[cfg(feature = "concurrent")]
    {
        let bucket = ext::utils::query_bucket();
        res.compute_through_stem_concurrent(max_s, max_n, &bucket);
    }

    println!("{}", res.graded_dimension_string());

    if let Some(file) = save_file {
        let mut file = std::io::BufWriter::new(file);
        res.save(&mut file)?;
    }

    Ok(())
}
