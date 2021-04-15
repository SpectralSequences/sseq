//! Resolves a module up to a fixed stem and prints an ASCII depiction of the Ext groups.

use ext::chain_complex::FreeChainComplex;
use ext::utils::{construct, get_config};
use saveload::Save;

fn main() -> error::Result<()> {
    // Read command line arguments
    let config = get_config();
    let res = construct(&config)?;

    let max_s = query::query_with_default("Max s", "15", Ok);
    let max_f = query::query_with_default("Max f", "30", Ok);

    #[cfg(not(feature = "concurrent"))]
    res.resolve_through_stem(max_s, max_f);

    #[cfg(feature = "concurrent")]
    {
        let num_threads = query::query_with_default("Number of threads", "2", Ok);
        let bucket = std::sync::Arc::new(thread_token::TokenBucket::new(num_threads));
        res.resolve_through_stem_concurrent(max_s, max_f, &bucket);
    }

    println!("\x1b[1m{}", res.graded_dimension_string());

    if let Some(file_name) = query::query_optional::<String, _, _>("Save file", Ok) {
        let file = std::fs::File::create(file_name)?;
        let mut file = std::io::BufWriter::new(file);
        res.save(&mut file)?;
    }

    Ok(())
}
