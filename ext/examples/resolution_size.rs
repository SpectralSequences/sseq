use algebra::module::Module;
use ext::chain_complex::ChainComplex;
use ext::utils::construct;

fn main() -> error::Result<()> {
    let save_file: Option<String> = query::optional("Resolution save file", Ok);

    let max_s = query::with_default("Max s", "7", Ok);
    let max_f = query::with_default("Max f", "30", Ok);

    let res = construct("S_2@milnor", save_file.as_deref()).unwrap();

    #[cfg(not(feature = "concurrent"))]
    res.compute_through_stem(max_s, max_f);

    #[cfg(feature = "concurrent")]
    {
        let num_threads = query::with_default("Number of threads", "2", Ok);
        let bucket = std::sync::Arc::new(thread_token::TokenBucket::new(num_threads));
        res.compute_through_stem_concurrent(max_s, max_f, &bucket);
    }

    for s in (0..=max_s).rev() {
        for f in 0..=max_f {
            print!("{}, ", res.module(s).dimension(f + s as i32));
        }
        println!()
    }
    Ok(())
}
