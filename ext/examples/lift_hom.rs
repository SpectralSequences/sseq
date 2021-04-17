//! Resolves a module and prints an ASCII depiction of the Ext groups.

use algebra::JsonAlgebra;
use ext::chain_complex::ChainComplex;
use ext::resolution_homomorphism::ResolutionHomomorphism;
use ext::utils::construct;
use fp::matrix::Matrix;
use std::sync::Arc;

fn main() -> error::Result<()> {
    let target = query::with_default("Target module", "S_2", |name: String| {
        construct(&*name, None).map_err(|e| e.to_string())
    });
    let source = query::with_default("Source module", "Cnu", |name: String| {
        let source = construct((&*name, target.algebra().prefix()), None)?;
        if source.prime() != target.prime() {
            return Err("Source and target must have the same prime".into());
        }
        Ok(source)
    });
    let p = source.prime();

    let s = query::with_default("s", "2", Ok);
    let f: i32 = query::with_default("f", "7", Ok);
    let t = f + s as i32;

    #[cfg(feature = "concurrent")]
    {
        let num_threads = query::with_default("Number of threads", "2", Ok);
        let bucket = std::sync::Arc::new(thread_token::TokenBucket::new(num_threads));

        source.compute_through_stem_concurrent(s, f, &bucket);
        target.compute_through_stem_concurrent(s, f, &bucket);
    }

    #[cfg(not(feature = "concurrent"))]
    {
        source.compute_through_stem(s, f);
        target.compute_through_stem(s, f);
    }

    let hom = ResolutionHomomorphism::new(String::new(), Arc::new(source), Arc::new(target), 0, 0);

    hom.extend_step(0, 0, Some(&Matrix::from_vec(p, &[vec![1]])));
    hom.extend_through_stem(s, f);

    let matrix = hom.get_map(s).hom_k(t);
    for (i, r) in matrix.iter().enumerate() {
        println!("f(x_{{{}, {}, {}}}) = {:?}", f, s, i, r);
    }
    Ok(())
}
