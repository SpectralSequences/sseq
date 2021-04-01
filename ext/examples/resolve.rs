//! Resolves a module and prints an ASCII depiction of the Ext groups.

use ext::chain_complex::FreeChainComplex;
use ext::utils::{construct, get_config};

fn main() -> error::Result<()> {
    // Read command line arguments
    let config = get_config();
    let max_degree = config.max_degree;
    let res = construct(&config)?;

    #[cfg(not(feature = "concurrent"))]
    res.resolve_through_bidegree(max_degree as u32, max_degree);

    #[cfg(feature = "concurrent")]
    {
        let num_threads = query::query_with_default("Number of threads", "2", Ok);
        let bucket = std::sync::Arc::new(thread_token::TokenBucket::new(num_threads));
        res.resolve_through_bidegree_concurrent(max_degree as u32, max_degree, &bucket);
    }

    println!("\x1b[1m{}", res.graded_dimension_string());
    Ok(())
}
