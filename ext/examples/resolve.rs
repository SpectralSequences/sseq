//! Resolves a module and prints an ASCII depiction of the Ext groups.

use ext::chain_complex::{ChainComplex, FreeChainComplex};
use ext::utils::construct;
use saveload::Save;

fn main() -> error::Result {
    let res = query::with_default("Module", "S_2", |name: String| {
        construct(&*name, None).map_err(|e| e.to_string())
    });

    let max_s = query::with_default("Max s", "15", Ok);
    let max_t = query::with_default("Max t", "30", Ok);
    let save_file: Option<String> = query::optional("Save file", Ok);

    #[cfg(not(feature = "concurrent"))]
    res.compute_through_bidegree(max_s, max_t);

    #[cfg(feature = "concurrent")]
    {
        let bucket = ext::utils::query_bucket();
        res.compute_through_bidegree_concurrent(max_s, max_t, &bucket);
    }

    println!("{}", res.graded_dimension_string());

    if let Some(file_name) = save_file {
        let file = std::fs::File::create(file_name)?;
        let mut file = std::io::BufWriter::new(file);
        res.save(&mut file)?;
    }
    Ok(())
}
