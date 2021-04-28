//! Resolves a module and prints an ASCII depiction of the Ext groups.

use algebra::module::homomorphism::BoundedModuleHomomorphism;
use algebra::JsonAlgebra;
use ext::chain_complex::ChainComplex;
use ext::resolution_homomorphism::ResolutionHomomorphism;
use ext::utils::construct;
use std::sync::Arc;

fn main() -> error::Result {
    let target = query::with_default("Target module", "S_2", |name| construct(name, None));
    let source = query::with_default("Source module", "Cnu", |name| {
        let source = construct((name, target.algebra().prefix()), None)?;
        if source.prime() != target.prime() {
            return Err(String::from("Source and target must have the same prime"));
        }
        if !source.complex().module(0).is_fd_module() {
            return Err(String::from("Source must be finite dimensional"));
        }
        Ok(source)
    });

    let s = query::with_default("Max s", "2", str::parse);
    let n: i32 = query::with_default("Max n", "7", str::parse);

    #[cfg(feature = "concurrent")]
    let bucket = ext::utils::query_bucket();

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
                if v.len() != row.len() {
                    return Err(format!(
                        "Target has dimension {} but {} coordinates supplied",
                        row.len(),
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
        source.compute_through_stem_concurrent(s, n, &bucket);
        target.compute_through_stem_concurrent(s, n, &bucket);
    }

    #[cfg(not(feature = "concurrent"))]
    {
        source.compute_through_stem(s, n);
        target.compute_through_stem(s, n);
    }

    let hom = ResolutionHomomorphism::from_module_homomorphism(
        String::new(),
        Arc::new(source),
        Arc::new(target),
        &module_hom.into(),
    );

    #[cfg(not(feature = "concurrent"))]
    hom.extend_through_stem(s, n);

    #[cfg(feature = "concurrent")]
    hom.extend_through_stem_concurrent(s, n, &bucket);

    for (s, n, t) in hom.target.iter_stem() {
        let matrix = hom.get_map(s).hom_k(t);
        for (i, r) in matrix.iter().enumerate() {
            println!("F(x_({}, {}, {})) = {:?}", n, s, i, r);
        }
    }
    Ok(())
}
