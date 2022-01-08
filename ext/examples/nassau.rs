use ext::chain_complex::FreeChainComplex;
use ext::nassau::Resolution;

fn main() {
    let n = query::raw("Max n", str::parse);
    let s = query::raw("Max s", str::parse);

    let res = Resolution::new();
    res.compute_through_stem(s, n);
    println!("{}", res.graded_dimension_string());
}
