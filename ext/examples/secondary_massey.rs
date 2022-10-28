//! Computes massey products in $\Mod_{C\tau^2}$.
//!
//! # Usage
//! This computes all Massey products of the form $\langle -, b, a\rangle$, where
//! $a \in \Ext^{\*, \*}(M, k)$ and $b, (-) \in \Ext^{\*, \*}(k, k)$. It does not verify that the
//! Massey product is valid, i.e. $a$ and $b$ both lift to $\Mod_{C\tau^2}$ and have trivial
//! product.
//!
//! Since we must choose $a$ and $b$ to have trivial product, it is necessary to be able to specify
//! the $\tau$ part of them, and not insist that they are standard lifts of the $\Ext$ classes.
//! Thus, the user is first prompted for the $\Ext$ part, then the $\tau$ part of each class. To
//! set a part to zero, supply an empty name. Note that if the bidegree right above the class is
//! empty, the user is not prompted for the $\tau$ part.
//!
//! # Output
//! This computes the Massey products up to a sign. We write our output in the category
//! $\Mod_{C\tau^2}$, so the format is $\langle a, b, -\rangle$ instead of $\langle -, b,
//! a\rangle$. Brave souls are encouraged to figure out the correct sign for the products.

use std::sync::Arc;

use algebra::module::Module;
use algebra::pair_algebra::PairAlgebra;
use fp::matrix::{Matrix, Subspace};
use fp::vector::FpVector;

use ext::chain_complex::{ChainComplex, ChainHomotopy, FreeChainComplex};
use ext::resolution_homomorphism::ResolutionHomomorphism;
use ext::secondary::*;
use ext::utils::{query_module, QueryModuleResolution};

use itertools::Itertools;

struct HomData {
    name: String,
    class: FpVector,
    hom_lift: Arc<SecondaryResolutionHomomorphism<QueryModuleResolution, QueryModuleResolution>>,
    tau_part: Option<Arc<ResolutionHomomorphism<QueryModuleResolution, QueryModuleResolution>>>,
}

fn get_hom(
    name: &str,
    source: Arc<SecondaryResolution<QueryModuleResolution>>,
    target: Arc<SecondaryResolution<QueryModuleResolution>>,
) -> HomData {
    let p = source.prime();

    let shift_n: i32 = query::raw(&format!("n of {name}"), str::parse);
    let shift_s: u32 = query::raw(&format!("s of {name}"), str::parse);

    let ext_name: String = query::raw(&format!("Name of Ext part of {name}"), str::parse);

    let shift_t = shift_n + shift_s as i32;

    source
        .underlying()
        .compute_through_stem(shift_s + 1, shift_n);

    let hom = Arc::new(ResolutionHomomorphism::new(
        ext_name.clone(),
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

    if !hom.name().is_empty() {
        if matrix.rows() == 0 {
            eprintln!("No classes in this bidegree");
        } else {
            let v: Vec<u32> = query::vector(&format!("Input Ext class {ext_name}"), num_gens);
            for (i, &x) in v.iter().enumerate() {
                matrix[i].set_entry(0, x);
                class.set_entry(i, x);
            }
        }
    }

    hom.extend_step(shift_s, shift_t, Some(&matrix));

    let hom_lift = Arc::new(SecondaryResolutionHomomorphism::new(source, target, hom));

    let tau_part = if num_tau_gens > 0 {
        let tau_name: String = query::raw(&format!("Name of τ part of {name}"), str::parse);
        if tau_name.is_empty() {
            None
        } else {
            let v = query::vector(&format!("Input Ext class {tau_name}"), num_tau_gens);
            for (i, &x) in v.iter().enumerate() {
                class.set_entry(num_gens + i, x);
            }
            Some(Arc::new(ResolutionHomomorphism::from_class(
                tau_name,
                hom_lift.source(),
                hom_lift.target(),
                shift_s + 1,
                shift_t + 1,
                &v,
            )))
        }
    } else {
        None
    };

    let name = match (&*ext_name, tau_part.as_ref().map_or("", |x| x.name())) {
        ("", "") => panic!("Do not compute zero Massey product"),
        ("", x) => format!("τ{x}"),
        (x, "") => format!("[{x}]"),
        (x, y) => format!("[{x}] + τ{y}"),
    };
    HomData {
        name,
        class,
        hom_lift,
        tau_part,
    }
}

fn main() -> anyhow::Result<()> {
    eprintln!("We are going to compute <-, b, a> for all (-), where a is an element in Ext(M, k) and b and (-) are elements in Ext(k, k).");

    let resolution = Arc::new(query_module(Some(algebra::AlgebraType::Milnor), true)?);

    let (is_unit, unit) = ext::utils::get_unit(Arc::clone(&resolution))?;

    let p = resolution.prime();

    let res_lift = Arc::new(SecondaryResolution::new(Arc::clone(&resolution)));
    let unit_lift = if is_unit {
        Arc::clone(&res_lift)
    } else {
        let lift = SecondaryResolution::new(Arc::clone(&unit));
        Arc::new(lift)
    };

    let HomData {
        name: a_name,
        class: _,
        hom_lift: a,
        tau_part: a_tau,
    } = get_hom("a", Arc::clone(&res_lift), Arc::clone(&unit_lift));
    let HomData {
        name: b_name,
        class: b_class,
        hom_lift: b,
        tau_part: b_tau,
    } = get_hom("b", Arc::clone(&unit_lift), Arc::clone(&unit_lift));

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
    rayon::scope(|s| {
        s.spawn(|_| {
            a.underlying().extend_all();
            a.extend_all();
        });
        s.spawn(|_| {
            b.underlying().extend_all();
            b.extend_all();
        });
        if let Some(a_tau) = &a_tau {
            s.spawn(|_| a_tau.extend_all());
        }
        if let Some(b_tau) = &b_tau {
            s.spawn(|_| b_tau.extend_all());
        }
    });

    #[cfg(not(feature = "concurrent"))]
    {
        a.underlying().extend_all();
        a.extend_all();
        b.underlying().extend_all();
        b.extend_all();
        if let Some(a_tau) = &a_tau {
            a_tau.extend_all();
        }
        if let Some(b_tau) = &b_tau {
            b_tau.extend_all();
        }
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
        let v = a.product_nullhomotopy(
            a_tau.as_deref(),
            &res_sseq,
            b_shift_s,
            b_shift_t,
            b_class.as_slice(),
        );
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

    let ch_lift = SecondaryChainHomotopy::new(
        Arc::clone(&a),
        Arc::clone(&b),
        a_tau.as_ref().map(Arc::clone),
        b_tau.as_ref().map(Arc::clone),
        Arc::clone(&chain_homotopy),
    );

    if let Some(s) = ext::utils::secondary_job() {
        ch_lift.compute_partial(s);
        return Ok(());
    }

    let timer = ext::utils::Timer::start();
    ch_lift.extend_all();
    timer.end(format_args!("Total computation time"));

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
                        .add(gen, 1);
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

                b.hom_k_with(
                    b_tau.as_deref(),
                    Some(&unit_sseq),
                    s,
                    t,
                    e2_kernel.basis().iter().map(FpVector::as_slice),
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
                .filtration_one_product(1, h_0, source_s, t + shift_t)
                .unwrap(),
        );
        let ma = a.underlying().get_map(source_s).hom_k(t + b_shift_t);
        let mb = b.underlying().get_map(s + b_shift_s).hom_k(t);

        for gen in e3_kernel.iter() {
            // Print name
            {
                print!("<{a_name}, {b_name}, ");
                let has_ext = {
                    let ext_part = gen.slice(0, target_num_gens);
                    if ext_part.iter_nonzero().count() > 0 {
                        print!("[");
                        ext::utils::print_element(ext_part, n, s);
                        print!("]");
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
                let out = b.product_nullhomotopy(b_tau.as_deref(), &unit_sseq, s, t, gen);
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
                    scratch1.add_basis_element(gen_idx, dx.entry(idx));
                }
            }
            println!(" + τ{scratch1}");
        }
    }
    Ok(())
}
