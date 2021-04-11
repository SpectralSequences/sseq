use algebra::module::Module;
use ext::chain_complex::ChainComplex;
use ext::utils::construct_s_2;
use query::*;

fn main() -> error::Result<()> {
    let save_file: Option<String> = query_optional("Resolution save file", Ok);

    let max_s = query_with_default("Max s", "7", Ok);
    let max_f = query_with_default("Max f", "30", Ok);

    let res = construct_s_2("milnor", save_file);

    #[cfg(not(feature = "concurrent"))]
    res.resolve_through_stem(max_s, max_f);

    #[cfg(feature = "concurrent")]
    {
        let num_threads = query::query_with_default("Number of threads", "2", Ok);
        let bucket = std::sync::Arc::new(thread_token::TokenBucket::new(num_threads));
        res.resolve_through_stem_concurrent(max_s, max_f, &bucket);
    }

    for s in (0..=max_s).rev() {
        for f in 0..=max_f {
            print!("{}, ", res.module(s).dimension(f + s as i32));
        }
        println!()
    }
    Ok(())
}
