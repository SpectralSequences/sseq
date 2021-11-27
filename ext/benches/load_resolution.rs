use algebra::Algebra;
use ext::chain_complex::ChainComplex;
use ext::utils::construct;

fn main() {
    let resolution = construct("S_2@milnor", Some("/tmp/.ext_bench/S_2_milnor".into())).unwrap();
    resolution.algebra().compute_basis(100);
    let start = std::time::Instant::now();
    resolution.compute_through_bidegree(50, 100);
    println!("Time: {:?}", start.elapsed());
}
