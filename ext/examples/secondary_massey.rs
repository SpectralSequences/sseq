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
use sseq::coordinates::{Bidegree, BidegreeElement};

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

    let shift = Bidegree::n_s(
        query::raw(&format!("n of {name}"), str::parse),
        query::raw(&format!("s of {name}"), str::parse),
    );

    let ext_name: String = query::raw(&format!("Name of Ext part of {name}"), str::parse);

    source
        .underlying()
        .compute_through_stem(shift + TAU_BIDEGREE);

    let hom = Arc::new(ResolutionHomomorphism::new(
        ext_name.clone(),
        source.underlying(),
        target.underlying(),
        shift,
    ));

    let num_gens = source.underlying().number_of_gens_in_bidegree(shift);
    let num_tau_gens = hom.source.number_of_gens_in_bidegree(shift + TAU_BIDEGREE);

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

    hom.extend_step(shift, Some(&matrix));

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
                shift + TAU_BIDEGREE,
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

    let shift = Bidegree::s_t(
        (a.underlying().shift + b.underlying().shift).s(),
        (a.shift() + b.shift()).t(),
    );

    // Extend resolutions
    if !is_unit {
        let res_max = Bidegree::n_s(
            resolution.module(0).max_computed_degree(),
            resolution.next_homological_degree() - 1,
        );
        unit.compute_through_stem(res_max - a.underlying().shift);
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

    let b_shift = b.underlying().shift;

    let chain_homotopy = Arc::new(ChainHomotopy::new(a.underlying(), b.underlying()));
    chain_homotopy.initialize_homotopies((b_shift + a.underlying().shift).s());

    // Compute first homotopy
    {
        let v = a.product_nullhomotopy(a_tau.as_deref(), &res_sseq, b_shift, b_class.as_slice());
        let homotopy = chain_homotopy.homotopy(b_shift.s() + a.underlying().shift.s() - 1);
        let htpy_source = a.shift() + b_shift;
        homotopy.extend_by_zero(htpy_source.t() - 1);
        homotopy.add_generators_from_rows(
            htpy_source.t(),
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

    fn get_page_data(sseq: &sseq::Sseq, b: Bidegree) -> &fp::matrix::Subquotient {
        let d = sseq.page_data(b.n(), b.s() as i32);
        &d[std::cmp::min(3, d.len() - 1)]
    }

    let mut scratch0: Vec<u32> = Vec::new();
    let mut scratch1 = FpVector::new(p, 0);

    let h_0 = ch_lift.algebra().p_tilde();

    // Iterate through the multiplicand
    for c in unit.iter_stem() {
        if !resolution.has_computed_bidegree(c + shift - Bidegree::s_t(2, 0))
            || !resolution.has_computed_bidegree(c + shift + Bidegree::s_t(0, 1))
        {
            continue;
        }

        // Now read off the products
        let source = c + shift - Bidegree::s_t(1, 0);

        let source_num_gens = resolution.number_of_gens_in_bidegree(source);
        let source_tau_num_gens = resolution.number_of_gens_in_bidegree(source + TAU_BIDEGREE);

        if source_num_gens + source_tau_num_gens == 0 {
            continue;
        }

        // We find the kernel of multiplication by b.
        let target_num_gens = unit.number_of_gens_in_bidegree(c);
        let target_tau_num_gens = unit.number_of_gens_in_bidegree(c + TAU_BIDEGREE);
        let target_all_gens = target_num_gens + target_tau_num_gens;

        let prod_num_gens = unit.number_of_gens_in_bidegree(c + b_shift);
        let prod_tau_num_gens = unit.number_of_gens_in_bidegree(c + b_shift + TAU_BIDEGREE);
        let prod_all_gens = prod_num_gens + prod_tau_num_gens;

        let e3_kernel = {
            let target_page_data = get_page_data(&unit_sseq, c);
            let target_tau_page_data = get_page_data(&unit_sseq, c + TAU_BIDEGREE);
            let product_tau_page_data = get_page_data(&unit_sseq, c + b_shift + TAU_BIDEGREE);

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
                    &b.underlying()
                        .get_map(c.s() + b.underlying().shift.s())
                        .hom_k(c.t()),
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
                    c,
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
                let m = Matrix::from_vec(
                    p,
                    &b.underlying()
                        .get_map(b_shift.s() + c.s() + 1)
                        .hom_k(c.t() + 1),
                );

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

        let m0 = chain_homotopy.homotopy(source.s()).hom_k(c.t());
        let mt = Matrix::from_vec(p, &chain_homotopy.homotopy(source.s() + 1).hom_k(c.t() + 1));
        let m1 = Matrix::from_vec(
            p,
            &ch_lift.homotopies()[source.s() as i32 + 1]
                .homotopies
                .hom_k(c.t()),
        );
        let mp = Matrix::from_vec(
            p,
            &resolution
                .filtration_one_product(1, h_0, Bidegree::s_t(source.s(), c.t() + shift.t()))
                .unwrap(),
        );
        let ma = a
            .underlying()
            .get_map(source.s())
            .hom_k(c.t() + b_shift.t());
        let mb = b.underlying().get_map(c.s() + b_shift.s()).hom_k(c.t());

        for gen in e3_kernel.iter() {
            // Print name
            {
                print!("<{a_name}, {b_name}, ");
                let has_ext = {
                    let ext_part = gen.slice(0, target_num_gens);
                    if ext_part.iter_nonzero().count() > 0 {
                        print!("[");
                        BidegreeElement::new(c, ext_part).print();
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
                        BidegreeElement::new(
                            c + TAU_BIDEGREE,
                            gen.slice(target_num_gens, target_all_gens),
                        )
                        .print();
                    } else {
                        print!("(");
                        BidegreeElement::new(
                            c + TAU_BIDEGREE,
                            gen.slice(target_num_gens, target_all_gens),
                        )
                        .print();
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
                let out = b.product_nullhomotopy(b_tau.as_deref(), &unit_sseq, c, gen);
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
                    let m = a.underlying().get_map((source + TAU_BIDEGREE).s());
                    let dx = m.output((source + TAU_BIDEGREE).t(), gen_idx);
                    let idx = unit.module((c + shift).s()).operation_generator_to_index(
                        1,
                        h_0,
                        (c + shift).t(),
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
