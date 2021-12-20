//! Computes massey products in $\Mod_{C\tau^2}$. This is only correct up to a sign!!!

use std::path::PathBuf;
use std::sync::Arc;

use algebra::module::Module;
use algebra::pair_algebra::PairAlgebra;
use algebra::SteenrodAlgebra;
use ext::resolution::Resolution;
use fp::matrix::{Matrix, Subspace};
use fp::vector::FpVector;

use ext::chain_complex::{AugmentedChainComplex, ChainComplex, ChainHomotopy, FreeChainComplex};
use ext::resolution_homomorphism::ResolutionHomomorphism;
use ext::utils::query_module;
use ext::{secondary::*, CCC};

use itertools::Itertools;

fn get_hom(
    name: &str,
    source: Arc<SecondaryResolution<SteenrodAlgebra, Resolution<CCC>>>,
    target: Arc<SecondaryResolution<SteenrodAlgebra, Resolution<CCC>>>,
) -> (
    FpVector,
    Arc<SecondaryResolutionHomomorphism<SteenrodAlgebra, Resolution<CCC>, Resolution<CCC>>>,
) {
    let p = source.prime();

    let name: String = query::raw(&format!("Name of Ext class {name}"), str::parse);

    let shift_s: u32 = query::with_default(&format!("s of Ext class {name}"), "0", str::parse);
    let shift_n: i32 = query::with_default(&format!("n of Ext class {name}"), "0", str::parse);
    let shift_t = shift_n + shift_s as i32;

    source
        .underlying()
        .compute_through_stem(shift_s + 1, shift_n);

    let hom = Arc::new(ResolutionHomomorphism::new(
        name.clone(),
        source.underlying(),
        target.underlying(),
        shift_s,
        shift_t,
    ));

    let num_gens = source
        .underlying()
        .number_of_gens_in_bidegree(shift_s, shift_t);
    let num_tau_gens = hom
        .source
        .number_of_gens_in_bidegree(shift_s + 1, shift_t + 1);

    let mut class = FpVector::new(p, num_gens + num_tau_gens);

    let mut matrix = Matrix::new(p, num_gens, 1);

    if matrix.rows() == 0 {
        eprintln!("No classes in this bidegree");
    } else {
        let v: Vec<u32> = query::vector(&format!("Input Ext class {name}"), num_gens);
        for (i, &x) in v.iter().enumerate() {
            matrix[i].set_entry(0, x);
            class.set_entry(i, x);
        }
    }

    hom.extend_step(shift_s, shift_t, Some(&matrix));
    // Make room for the tau part
    hom.extend_through_stem(shift_s + 1, shift_t - shift_s as i32);

    let hom_lift = SecondaryResolutionHomomorphism::new(
        Arc::clone(&source),
        Arc::clone(&target),
        Arc::clone(&hom),
    );

    let num_tau_gens = hom
        .source
        .number_of_gens_in_bidegree(shift_s + 1, shift_t + 1);
    if num_tau_gens > 0 {
        let v = query::vector(&format!("Input τ part of {name}"), num_tau_gens);
        for (i, &x) in v.iter().enumerate() {
            class.set_entry(num_gens + i, x);
        }
        let rows = v
            .into_iter()
            .map(|x| FpVector::from_slice(p, &[x]))
            .collect();

        hom_lift.initialize_homotopies();
        hom_lift.homotopies()[shift_s as i32 + 1]
            .homotopies
            .add_generators_from_rows(shift_t + 1, rows);
    }
    (class, Arc::new(hom_lift))
}

fn main() -> anyhow::Result<()> {
    eprintln!("We are going to compute <a, b, -> for all (-), where a is an element in Ext(M, k) and b and (-) are elements in Ext(k, k).");

    let resolution = Arc::new(query_module(
        Some(algebra::AlgebraType::Milnor),
        ext::utils::LoadQuasiInverseOption::IfNoSave,
    )?);

    let is_unit = resolution.target().modules.len() == 1 && resolution.target().module(0).is_unit();

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
            resolution.target().module(0)
        );
        return Ok(());
    }

    let p = resolution.prime();

    let res_lift = Arc::new(SecondaryResolution::new(Arc::clone(&resolution)));
    let unit_lift = if is_unit {
        Arc::clone(&res_lift)
    } else {
        let lift = SecondaryResolution::new(Arc::clone(&unit));
        Arc::new(lift)
    };

    let (_, a) = get_hom("a", Arc::clone(&res_lift), Arc::clone(&unit_lift));
    let (b_class, b) = get_hom("b", Arc::clone(&unit_lift), Arc::clone(&unit_lift));

    let shift_s = a.underlying().shift_s + b.underlying().shift_s;
    let shift_t = a.shift_t() + b.shift_t();

    // Extend resolutions
    if !is_unit {
        unit.compute_through_stem(
            resolution.next_homological_degree() - 1 - a.underlying().shift_s,
            resolution.module(0).max_computed_degree()
                - (a.shift_t() - a.underlying().shift_s as i32),
        );
    }

    if is_unit {
        res_lift.extend_all();
    } else {
        #[cfg(feature = "concurrent")]
        rayon::join(|| res_lift.extend_all(), || unit_lift.extend_all());

        #[cfg(not(feature = "concurrent"))]
        {
            res_lift.extend_all();
            unit_lift.extend_all();
        }
    }

    // Now extend homomorphisms
    #[cfg(feature = "concurrent")]
    rayon::join(
        || {
            a.underlying().extend_all();
            a.extend_all();
        },
        || {
            b.underlying().extend_all();
            b.extend_all();
        },
    );

    #[cfg(not(feature = "concurrent"))]
    {
        a.underlying().extend_all();
        a.extend_all();
        b.underlying().extend_all();
        b.extend_all();
    }

    let res_sseq = Arc::new(res_lift.e3_page());
    let unit_sseq = if is_unit {
        Arc::clone(&res_sseq)
    } else {
        Arc::new(res_lift.e3_page())
    };

    let b_shift_s = b.underlying().shift_s;
    let b_shift_t = b.underlying().shift_t;
    let b_shift_n = b_shift_t - b_shift_s as i32;

    let chain_homotopy = Arc::new(ChainHomotopy::new(a.underlying(), b.underlying()));
    chain_homotopy.initialize_homotopies(b_shift_s + a.underlying().shift_s);

    // Compute first homotopy
    {
        let v = a.product_nullhomotopy(&res_sseq, b_shift_s, b_shift_t, b_class.as_slice());
        let homotopy = chain_homotopy.homotopy(b_shift_s + a.underlying().shift_s - 1);
        homotopy.extend_by_zero(a.shift_t() + b_shift_t - 1);
        homotopy.add_generators_from_rows(
            a.shift_t() + b_shift_t,
            v.into_iter()
                .map(|x| FpVector::from_slice(p, &[x]))
                .collect(),
        );
    }

    chain_homotopy.extend_all();

    let ch_lift =
        SecondaryChainHomotopy::new(Arc::clone(&a), Arc::clone(&b), Arc::clone(&chain_homotopy));

    ch_lift.extend_all();

    fn get_page_data(sseq: &sseq::Sseq, n: i32, s: u32) -> &fp::matrix::Subquotient {
        let d = sseq.page_data(n, s as i32);
        &d[std::cmp::min(3, d.len() - 1)]
    }

    let mut scratch0: Vec<u32> = Vec::new();
    let mut scratch1 = FpVector::new(p, 0);

    let h_0 = ch_lift.algebra().p_tilde();

    // Iterate through the multiplicand
    for (s, n, t) in unit.iter_stem() {
        if !resolution.has_computed_bidegree(s + shift_s - 2, t + shift_t)
            || !resolution.has_computed_bidegree(s + shift_s, t + shift_t + 1)
        {
            continue;
        }

        // Now read off the products
        let source_s = s + shift_s - 1;
        let source_t = t + shift_t;

        let source_num_gens = resolution.number_of_gens_in_bidegree(source_s, source_t);
        let source_tau_num_gens = resolution.number_of_gens_in_bidegree(source_s + 1, source_t + 1);

        if source_num_gens + source_tau_num_gens == 0 {
            continue;
        }

        // We find the kernel of multiplication by b.
        let target_num_gens = unit.number_of_gens_in_bidegree(s, t);
        let target_tau_num_gens = unit.number_of_gens_in_bidegree(s + 1, t + 1);
        let target_all_gens = target_num_gens + target_tau_num_gens;

        let prod_num_gens = unit.number_of_gens_in_bidegree(s + b_shift_s, t + b_shift_t);
        let prod_tau_num_gens =
            unit.number_of_gens_in_bidegree(s + b_shift_s + 1, t + b_shift_t + 1);
        let prod_all_gens = prod_num_gens + prod_tau_num_gens;

        let e3_kernel = {
            let target_page_data = get_page_data(&unit_sseq, n, s);
            let target_tau_page_data = get_page_data(&unit_sseq, n, s + 1);
            let product_tau_page_data = get_page_data(&unit_sseq, n + b_shift_n, s + b_shift_s + 1);

            // We first compute elements whose product vanish mod tau, and later see what the possible
            // lifts are. We do it this way to avoid Z/p^2 problems

            let e2_kernel: Subspace = {
                let mut product_matrix = Matrix::new(
                    p,
                    target_page_data.subspace_dimension(),
                    target_num_gens + prod_num_gens,
                );

                let m0 = Matrix::from_vec(
                    p,
                    &b.underlying().get_map(s + b.underlying().shift_s).hom_k(t),
                );
                for (gen, out) in target_page_data
                    .subspace_gens()
                    .zip_eq(product_matrix.iter_mut())
                {
                    out.slice_mut(prod_num_gens, prod_num_gens + target_num_gens)
                        .add(gen.as_slice(), 1);
                    for (i, v) in gen.iter_nonzero() {
                        out.slice_mut(0, prod_num_gens).add(m0[i].as_slice(), v);
                    }
                }
                product_matrix.row_reduce();
                product_matrix.compute_kernel(prod_num_gens)
            };

            // Now compute the e3 kernel
            {
                // First add the lifts from Ext
                let e2_ker_dim = e2_kernel.dimension();
                let mut product_matrix = Matrix::new(
                    p,
                    e2_ker_dim + target_tau_page_data.quotient_dimension(),
                    target_all_gens + prod_all_gens,
                );

                b.hom_k(
                    Some(&unit_sseq),
                    s,
                    t,
                    e2_kernel.basis().iter().map(|x| x.as_slice()),
                    product_matrix[0..e2_ker_dim]
                        .iter_mut()
                        .map(|x| x.slice_mut(0, prod_all_gens)),
                );
                for (v, t) in e2_kernel.basis().iter().zip(product_matrix.iter_mut()) {
                    t.slice_mut(prod_all_gens, prod_all_gens + target_num_gens)
                        .assign(v.as_slice());
                }

                // Now add the tau multiples
                let m =
                    Matrix::from_vec(p, &b.underlying().get_map(b_shift_s + s + 1).hom_k(t + 1));

                let mut count = 0;
                for (i, &v) in target_tau_page_data.quotient_pivots().iter().enumerate() {
                    if v >= 0 {
                        continue;
                    }
                    let row = &mut product_matrix[e2_ker_dim + count as usize];
                    row.add_basis_element(prod_all_gens + target_num_gens + i, 1);
                    row.slice_mut(prod_num_gens, prod_all_gens)
                        .add(m[i].as_slice(), 1);
                    product_tau_page_data
                        .reduce_by_quotient(row.slice_mut(prod_num_gens, prod_all_gens));
                    count += 1;
                }

                product_matrix.row_reduce();
                product_matrix.compute_kernel(prod_all_gens)
            }
        };

        if e3_kernel.dimension() == 0 {
            continue;
        }

        let m0 = chain_homotopy.homotopy(source_s).hom_k(t);
        let mt = Matrix::from_vec(p, &chain_homotopy.homotopy(source_s + 1).hom_k(t + 1));
        let m1 = Matrix::from_vec(
            p,
            &ch_lift.homotopies()[source_s as i32 + 1]
                .homotopies
                .hom_k(t),
        );
        let mp = Matrix::from_vec(
            p,
            &resolution
                .filtration_one_product(1, h_0, source_s + 1, t + shift_t + 1)
                .unwrap(),
        );
        let ma = a.underlying().get_map(source_s).hom_k(t + b_shift_t);
        let mb = b.underlying().get_map(s + b_shift_s).hom_k(t);

        for gen in e3_kernel.iter() {
            // Print name
            {
                print!("<{}, {}, ", a.underlying().name(), b.underlying().name());
                let has_ext = {
                    let ext_part = gen.slice(0, target_num_gens);
                    if ext_part.iter_nonzero().count() > 0 {
                        ext::utils::print_element(ext_part, n, s);
                        true
                    } else {
                        false
                    }
                };

                let tau_part = gen.slice(target_num_gens, target_all_gens);
                let num_entries = tau_part.iter_nonzero().count();
                if num_entries > 0 {
                    if has_ext {
                        print!(" + ");
                    }
                    print!("τ");
                    if num_entries == 1 {
                        ext::utils::print_element(
                            gen.slice(target_num_gens, target_all_gens),
                            n,
                            s + 1,
                        );
                    } else {
                        print!("(");
                        ext::utils::print_element(
                            gen.slice(target_num_gens, target_all_gens),
                            n,
                            s + 1,
                        );
                        print!(")");
                    }
                }
                print!("> = ±");
            }

            scratch0.clear();
            scratch0.resize(source_num_gens, 0);
            scratch1.set_scratch_vector_size(source_tau_num_gens);

            // First deal with the null-homotopy of ab
            for (i, v) in gen.slice(0, target_num_gens).iter_nonzero() {
                scratch0
                    .iter_mut()
                    .zip_eq(&m0[i])
                    .for_each(|(a, b)| *a += v * b);
                scratch1.add(&m1[i], v);
            }
            for (i, v) in gen.slice(target_num_gens, target_all_gens).iter_nonzero() {
                scratch1.add(&mt[i], v);
            }
            // Now do the -1 part of the null-homotopy of bc.
            {
                let sign = *p * *p - 1;
                let out = b.product_nullhomotopy(&unit_sseq, s, t, gen.as_slice());
                for (i, v) in out.iter_nonzero() {
                    scratch0
                        .iter_mut()
                        .zip_eq(&ma[i])
                        .for_each(|(a, b)| *a += v * b * sign);
                }
            }

            for (i, v) in scratch0.iter().enumerate() {
                let extra = *v / *p;
                scratch1.add(&mp[i], extra % *p);
            }

            print!("[{}]", scratch0.iter().map(|x| x % *p).format(", "));

            // Then deal with the rest of the null-homotopy of bc. This is just the null-homotopy
            // of 2.
            scratch0.clear();
            scratch0.resize(prod_num_gens, 0);

            for (i, v) in gen.slice(0, target_num_gens).iter_nonzero() {
                scratch0
                    .iter_mut()
                    .zip_eq(&mb[i])
                    .for_each(|(a, b)| *a += v * b);
            }
            for (i, v) in scratch0.iter().enumerate() {
                let extra = (v / *p) % *p;
                if extra == 0 {
                    continue;
                }
                for gen_idx in 0..source_tau_num_gens {
                    let m = a.underlying().get_map(source_s + 1);
                    let dx = m.output(source_t + 1, gen_idx);
                    let idx = unit.module(s + b_shift_s).operation_generator_to_index(
                        1,
                        h_0,
                        t + b_shift_t,
                        i,
                    );
                    scratch1.add_basis_element(gen_idx, dx.entry(idx))
                }
            }
            println!(" + τ{}", scratch1);
        }
    }
    Ok(())
}
