use std::sync::Arc;

use fp::matrix::Matrix;
use fp::vector::FpVector;

use ext::chain_complex::ChainComplex;
use ext::resolution_homomorphism::ResolutionHomomorphism;
use ext::secondary::*;
use ext::utils::query_module;

fn main() -> anyhow::Result<()> {
    let data = query_module(
        Some(algebra::AlgebraType::Milnor),
        ext::utils::LoadQuasiInverseOption::IfNoSave,
    )?;
    let resolution = Arc::new(data.resolution);

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
        Arc::clone(&resolution),
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
    let v: Vec<u32> = query::raw("Input ext class", |s| {
        let v = s[1..s.len() - 1]
            .split(',')
            .map(|x| x.parse::<u32>().map_err(|e| e.to_string()))
            .collect::<Result<Vec<_>, String>>()?;
        if v.len() != matrix.rows() {
            return Err(format!(
                "Target has dimension {} but {} coordinates supplied",
                matrix.rows(),
                v.len()
            ));
        }
        Ok(v)
    });
    for (i, &x) in v.iter().enumerate() {
        matrix[i].set_entry(0, x);
    }
    hom.extend_step(shift_s, shift_t, Some(&matrix));

    #[cfg(not(feature = "concurrent"))]
    hom.extend_all();

    #[cfg(feature = "concurrent")]
    hom.extend_all_concurrent(&data.bucket);

    let lift = SecondaryLift::new(Arc::clone(&resolution));
    lift.initialize_homotopies();
    lift.compute_composites();
    lift.compute_intermediates();

    #[cfg(feature = "concurrent")]
    lift.compute_homotopies_concurrent(&data.bucket);

    #[cfg(not(feature = "concurrent"))]
    lift.compute_homotopies();

    // Check that class survives to E3.
    {
        let m = lift.homotopy(shift_s + 2).homotopies.hom_k(shift_t);
        assert_eq!(m.len(), v.len());
        let mut sum = vec![0; m[0].len()];
        for (x, d2) in v.iter().zip(&m) {
            sum.iter_mut().zip(d2).for_each(|(a, b)| *a += x * b);
        }
        assert!(
            sum.iter().all(|x| x & *p == 2),
            "Class supports a non-zero d2"
        );
    }

    let lift = Arc::new(lift);
    let hom = Arc::new(hom);

    let res_lift = SecondaryResolutionHomomorphism::new(
        Arc::clone(&lift),
        Arc::clone(&lift),
        Arc::clone(&hom),
    );

    res_lift.initialize_homotopies();
    res_lift.compute_composites();
    res_lift.compute_intermediates();

    #[cfg(feature = "concurrent")]
    res_lift.compute_homotopies_concurrent(&data.bucket);

    #[cfg(not(feature = "concurrent"))]
    res_lift.compute_homotopies();

    // Compute E3 page
    let sseq = {
        let mut sseq = sseq::Sseq::<sseq::Adams>::new(p, 0, 0);

        let mut source_vec = FpVector::new(p, 0);
        let mut target_vec = FpVector::new(p, 0);

        for (s, n, t) in resolution.iter_stem() {
            let num_gens = resolution.module(s).number_of_gens_in_degree(t);
            sseq.set_dimension(n, s as i32, num_gens);

            if t > 0 && resolution.has_computed_bidegree(s + 2, t + 1) {
                let m = lift.homotopy(s + 2).homotopies.hom_k(t);
                if m.is_empty() || m[0].is_empty() {
                    continue;
                }

                source_vec.set_scratch_vector_size(m.len());
                target_vec.set_scratch_vector_size(m[0].len());

                for (i, row) in m.into_iter().enumerate() {
                    source_vec.set_to_zero();
                    source_vec.set_entry(i, 1);
                    target_vec.copy_from_slice(&row);

                    sseq.add_differential(
                        2,
                        n,
                        s as i32,
                        source_vec.as_slice(),
                        target_vec.as_slice(),
                    );
                }
            }
        }
        for (s, n, _) in resolution.iter_stem() {
            if sseq.invalid(n, s as i32) {
                sseq.update_bidegree(n, s as i32);
            }
        }
        sseq
    };

    let get_page_data = |n, s| {
        let d = sseq.page_data(n, s as i32);
        &d[std::cmp::min(3, d.len() - 1)]
    };

    // Compute products
    let mut scratch0 = FpVector::new(p, 0);
    let mut scratch1 = FpVector::new(p, 0);

    // Iterate through the multiplicand
    for (s, n, t) in resolution.iter_stem() {
        // The potential target has to be hit, and we need to have computed (the data need for) the
        // d2 that hits the potential target.
        if !resolution.has_computed_bidegree(s + shift_s + 1, t + shift_t + 1) {
            continue;
        }
        if !resolution.has_computed_bidegree(s + shift_s - 1, t + shift_t) {
            continue;
        }

        let page_data = get_page_data(n, s);

        if page_data.subspace_dimension() == 0 {
            continue;
        }

        let m0 = Matrix::from_vec(p, &hom.get_map(s + shift_s).hom_k(t));
        let m1 = Matrix::from_vec(p, &res_lift.homotopy(s + shift_s + 1).homotopies.hom_k(t));

        assert_eq!(m0.rows(), m1.rows());
        if m0.columns() == 0 && m1.columns() == 0 {
            continue;
        }

        for gen in page_data.subspace_gens() {
            scratch0.set_scratch_vector_size(m0.columns());
            scratch1.set_scratch_vector_size(m1.columns());

            m0.apply(scratch0.as_slice_mut(), 1, gen.as_slice());
            m1.apply(scratch1.as_slice_mut(), 1, gen.as_slice());
            get_page_data(n + shift_n, s + shift_s + 1).reduce_by_quotient(scratch1.as_slice_mut());

            print!("{name} ");
            ext::utils::print_element(gen.as_slice(), n, s);
            println!(" = {scratch0} + τ {scratch1}");
        }
    }
    Ok(())
}
