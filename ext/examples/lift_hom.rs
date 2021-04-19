//! Resolves a module and prints an ASCII depiction of the Ext groups.

use algebra::module::homomorphism::BoundedModuleHomomorphism;
use algebra::JsonAlgebra;
use ext::chain_complex::ChainComplex;
use ext::resolution_homomorphism::ResolutionHomomorphism;
use ext::utils::{construct, iter_stems_f};
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
        if !source.complex().module(0).is_fd_module() {
            return Err("Source must be finite dimensional".into());
        }
        Ok(source)
    });

    let s = query::with_default("s", "2", Ok);
    let f: i32 = query::with_default("f", "7", Ok);

    #[cfg(feature = "concurrent")]
    let bucket = {
        let num_threads = query::with_default("Number of threads", "2", Ok);
        std::sync::Arc::new(thread_token::TokenBucket::new(num_threads))
    };

    let source_module = source.complex().module(0);
    let target_module = target.complex().module(0);

    eprintln!("\nInput module homomorphism to lift:");
    let mut module_hom = BoundedModuleHomomorphism::new(source_module, target_module, 0);
    for (t, matrix) in module_hom.matrices.iter_mut_enum() {
        if matrix.columns() == 0 {
            continue;
        }
        for (idx, row) in matrix.iter_mut().enumerate() {
            let v: Vec<u32> = query::raw(&format!("f(x_({}, {}))", t, idx), |s| {
                let v = s
                    .split(',')
                    .map(|x| x.parse::<u32>().map_err(|e| e.to_string()))
                    .collect::<Result<Vec<_>, String>>()?;
                if v.len() != row.dimension() {
                    return Err(format!(
                        "Target has dimension {} but {} coordinates supplied",
                        row.dimension(),
                        v.len()
                    ));
                }
                Ok(v)
            });
            for (i, &x) in v.iter().enumerate() {
                row.set_entry(i, x);
            }
        }
    }

    #[cfg(feature = "concurrent")]
    {
        source.compute_through_stem_concurrent(s, f, &bucket);
        target.compute_through_stem_concurrent(s, f, &bucket);
    }

    #[cfg(not(feature = "concurrent"))]
    {
        source.compute_through_stem(s, f);
        target.compute_through_stem(s, f);
    }

    let hom = ResolutionHomomorphism::from_module_homomorphism(
        String::new(),
        Arc::new(source),
        Arc::new(target),
        &module_hom.into(),
    );

    hom.extend_through_stem(s, f);

    for (s, f, t) in iter_stems_f(s, f) {
        let matrix = hom.get_map(s).hom_k(t);
        for (i, r) in matrix.iter().enumerate() {
            println!("F(x_{{{}, {}, {}}}) = {:?}", f, s, i, r);
        }
    }
    Ok(())
}
