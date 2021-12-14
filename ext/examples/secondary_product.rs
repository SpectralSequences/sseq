//! Computes products in $\Mod_{C\tau^2}$.

use std::path::PathBuf;
use std::sync::Arc;

use algebra::module::Module;
use fp::matrix::Matrix;
use fp::vector::FpVector;

use algebra::pair_algebra::PairAlgebra;

use ext::chain_complex::ChainComplex;
use ext::resolution_homomorphism::ResolutionHomomorphism;
use ext::secondary::*;
use ext::utils::query_module;

fn main() -> anyhow::Result<()> {
    let data = query_module(
        Some(algebra::AlgebraType::Milnor),
        ext::utils::LoadQuasiInverseOption::IfNoSave,
    )?;

    #[cfg(feature = "concurrent")]
    rayon::ThreadPoolBuilder::new()
        .num_threads(data.bucket.max_threads.into())
        .build_global()?;

    let resolution = Arc::new(data.resolution);

    let is_unit =
        resolution.complex().modules.len() == 1 && resolution.complex().module(0).is_unit();

    let unit = if is_unit {
        Arc::clone(&resolution)
    } else {
        let save_dir = query::optional("Unit save directory", |x| {
            core::result::Result::<PathBuf, std::convert::Infallible>::Ok(PathBuf::from(x))
        });
        Arc::new(ext::utils::construct("S_2@milnor", save_dir)?)
    };

    if !can_compute(&resolution) {
        eprintln!(
            "Cannot compute d2 for the module {}",
            resolution.complex().module(0)
        );
        return Ok(());
    }

    let p = resolution.prime();

    let name: String = query::raw("Name of product", str::parse);

    let shift_s: u32 = query::with_default("s of Ext class", "0", str::parse);
    let shift_n: i32 = query::with_default("n of Ext class", "0", str::parse);
    let shift_t = shift_n + shift_s as i32;

    let hom = ResolutionHomomorphism::new(
        name.clone(),
        Arc::clone(&resolution),
        Arc::clone(&unit),
        shift_s,
        shift_t,
    );

    let mut matrix = Matrix::new(
        p,
        hom.source.number_of_gens_in_bidegree(shift_s, shift_t),
        1,
    );

    if matrix.rows() == 0 || matrix.columns() == 0 {
        panic!("No classes in this bidegree");
    }
    let v: Vec<u32> = query::vector("Input ext class", matrix.rows());
    for (i, &x) in v.iter().enumerate() {
        matrix[i].set_entry(0, x);
    }

    if !is_unit {
        #[cfg(feature = "concurrent")]
        unit.compute_through_stem_concurrent(
            resolution.next_homological_degree() - 1 - shift_s,
            resolution.module(0).max_computed_degree() - shift_n,
            &data.bucket,
        );

        #[cfg(not(feature = "concurrent"))]
        unit.compute_through_stem(
            resolution.next_homological_degree() - 1 - shift_s,
            resolution.module(0).max_computed_degree() - shift_n,
        );
    }

    hom.extend_step(shift_s, shift_t, Some(&matrix));
    hom.extend_all();

    let res_lift = SecondaryResolution::new(Arc::clone(&resolution));
    res_lift.extend_all();

    // Check that class survives to E3.
    {
        let m = res_lift.homotopy(shift_s + 2).homotopies.hom_k(shift_t);
        assert_eq!(m.len(), v.len());
        let mut sum = vec![0; m[0].len()];
        for (x, d2) in v.iter().zip(&m) {
            sum.iter_mut().zip(d2).for_each(|(a, b)| *a += x * b);
        }
        assert!(
            sum.iter().all(|x| x % *p == 0),
            "Class supports a non-zero d2"
        );
    }
    let res_lift = Arc::new(res_lift);

    let unit_lift = if is_unit {
        Arc::clone(&res_lift)
    } else {
        let lift = SecondaryResolution::new(Arc::clone(&unit));
        lift.extend_all();
        Arc::new(lift)
    };

    let hom = Arc::new(hom);
    let hom_lift = SecondaryResolutionHomomorphism::new(
        Arc::clone(&res_lift),
        Arc::clone(&unit_lift),
        Arc::clone(&hom),
    );

    let start = std::time::Instant::now();

    hom_lift.extend_all();

    eprintln!("Time spent: {:?}", start.elapsed());

    // Compute E3 page
    let res_sseq = Arc::new(res_lift.e3_page());
    let unit_sseq = if is_unit {
        Arc::clone(&res_sseq)
    } else {
        Arc::new(unit_lift.e3_page())
    };

    fn get_page_data(sseq: &sseq::Sseq<sseq::Adams>, n: i32, s: u32) -> &fp::matrix::Subquotient {
        let d = sseq.page_data(n, s as i32);
        &d[std::cmp::min(3, d.len() - 1)]
    }

    // Compute products
    // scratch0 is an element over Z/p^2, so not an FpVector
    let mut scratch0 = Vec::new();
    let mut scratch1 = FpVector::new(p, 0);

    let h_0 = resolution.algebra().p_tilde();

    // Iterate through the multiplicand
    for (s, n, t) in unit.iter_stem() {
        // The potential target has to be hit, and we need to have computed (the data need for) the
        // d2 that hits the potential target.
        if !resolution.has_computed_bidegree(s + shift_s + 1, t + shift_t + 1) {
            continue;
        }
        if !resolution.has_computed_bidegree(s + shift_s - 1, t + shift_t) {
            continue;
        }

        let page_data = get_page_data(&*unit_sseq, n, s);

        if page_data.subspace_dimension() == 0 {
            continue;
        }

        // m0 is a Vec<Vec<u32>> because it is actually over Z/p^2.
        let m0 = hom.get_map(s + shift_s).hom_k(t);
        let m1 = Matrix::from_vec(p, &hom_lift.homotopy(s + shift_s + 1).homotopies.hom_k(t));
        // The multiplication by p map
        let mp = Matrix::from_vec(
            p,
            &resolution
                .filtration_one_product(1, h_0, s + shift_s + 1, t + shift_t + 1)
                .unwrap(),
        );

        assert_eq!(m0.len(), m1.len());
        if m0.is_empty() {
            continue;
        }
        if m0[0].is_empty() && m1[0].is_empty() {
            continue;
        }

        // The product in Ext differs from the product in the Adams E_2 page by (-1)^{t' s}. At the
        // prime 2, we use the fact that -1 = 1 + 2 mod 4, so we add \tilde{2} times the E_2
        // product to the homotopy part.
        let sign = if (shift_s as i32 * t) % 2 == 1 {
            *p * *p - 1
        } else {
            1
        };

        let filtration_one_sign = if (t as i32 % 2) == 1 { *p - 1 } else { 1 };

        for gen in page_data.subspace_gens() {
            scratch0.clear();
            scratch0.resize(m0[0].len(), 0);

            scratch1.set_scratch_vector_size(m1.columns());

            for (i, v) in gen.iter_nonzero() {
                scratch0
                    .iter_mut()
                    .zip(&m0[i])
                    .for_each(|(a, b)| *a += v * b * sign);
                scratch1.add(&m1[i], (v * sign) % *p);
            }
            for (i, v) in scratch0.iter_mut().enumerate() {
                let extra = *v / *p;
                *v %= *p;

                if extra == 0 {
                    continue;
                }
                scratch1.add(&mp[i], (extra * filtration_one_sign) % *p);
            }

            scratch0.iter_mut().for_each(|a| *a %= *p);

            get_page_data(&*res_sseq, n + shift_n, s + shift_s + 1)
                .reduce_by_quotient(scratch1.as_slice_mut());

            print!("{name} ");
            ext::utils::print_element(gen.as_slice(), n, s);
            println!(" = {scratch0:?} + τ {scratch1}");
        }
    }
    Ok(())
}
