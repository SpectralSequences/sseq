use ext::chain_complex::ChainComplex;
use ext::utils::query_module;

fn main() -> error::Result<()> {
    let resolution = query_module(None)?.resolution;

    for (s, f, t) in resolution.iter_stem() {
        let mut i = 0;
        while let Some(products) = resolution.filtration_one_product(1 << i, 0, s + 1, t + (1 << i))
        {
            for (idx, row) in products.into_iter().enumerate() {
                if !row.is_empty() {
                    println!("h_{} x_({}, {}, {}) = {:?}", i, f, s, idx, row);
                }
            }
            i += 1;
        }
    }
    Ok(())
}
