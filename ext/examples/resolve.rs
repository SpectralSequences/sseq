//! Resolves a module and prints an ASCII depiction of the Ext groups.

use ext::utils::{construct, get_config};

fn main() -> error::Result<()> {
    // Read command line arguments
    let config = get_config();
    let res = construct(&config)?;

    #[cfg(not(feature = "concurrent"))]
    res.resolve_through_degree(config.max_degree);

    #[cfg(feature = "concurrent")]
    {
        let num_threads = query::query_with_default("Number of threads", "2", Ok);
        let bucket = std::sync::Arc::new(thread_token::TokenBucket::new(num_threads));
        res.resolve_through_degree_concurrent(config.max_degree, &bucket);
    }

    println!("\x1b[1m{}", res.graded_dimension_string());
    Ok(())
}
