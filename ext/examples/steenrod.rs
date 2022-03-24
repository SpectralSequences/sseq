use algebra::module::homomorphism::{FreeModuleHomomorphism, ModuleHomomorphism};
use algebra::module::Module;
use ext::chain_complex::{
    AugmentedChainComplex, BoundedChainComplex, ChainComplex, FreeChainComplex, TensorChainComplex,
};
use ext::utils;
use ext::yoneda::yoneda_representative_element;
use fp::matrix::Matrix;
use fp::vector::FpVector;
use itertools::Itertools;

use std::io::{stderr, stdout, Write};
use std::sync::Arc;

fn main() -> anyhow::Result<()> {
    let resolution = Arc::new(utils::query_module_only("Module", None, false)?);
    let module = resolution.target().module(0);
    let p = resolution.prime();

    if resolution.target().max_s() != 1 || !module.is_unit() || *p != 2 {
        panic!("Can only run Steenrod on the sphere");
    }

    let n: i32 = query::raw("n of Ext class", str::parse);
    let s: u32 = query::raw("s of Ext class", str::parse);
    let t = n + s as i32;

    resolution.compute_through_bidegree(2 * s, 2 * t);

    let class: Vec<u32> = query::vector(
        "Input Ext class",
        resolution.number_of_gens_in_bidegree(s, t),
    );

    let yoneda = Arc::new(yoneda_representative_element(
        Arc::clone(&resolution),
        s,
        t,
        &class,
    ));

    print!("Dimensions of Yoneda representative: 1");
    for s in 0..=s {
        print!(" {}", yoneda.module(s).total_dimension());
    }
    println!();

    let square = Arc::new(TensorChainComplex::new(
        Arc::clone(&yoneda),
        Arc::clone(&yoneda),
    ));

    let timer = utils::Timer::start();
    square.compute_through_bidegree(2 * s, 2 * t);
    for s in 0..=2 * s {
        square
            .differential(s as u32)
            .compute_auxiliary_data_through_degree(2 * t);
    }
    timer.end(format_args!("Computed quasi-inverses"));

    eprintln!("Computing Steenrod operations: ");

    let mut delta = Vec::with_capacity(s as usize);

    for i in 0..=s {
        let mut maps: Vec<Arc<FreeModuleHomomorphism<_>>> = Vec::with_capacity(2 * s as usize - 1);

        for s in 0..=2 * s - i {
            let source = resolution.module(s);
            let target = square.module(s + i);

            let map = FreeModuleHomomorphism::new(Arc::clone(&source), Arc::clone(&target), 0);
            maps.push(Arc::new(map));
        }
        delta.push(maps);
    }

    /* #[cfg(feature = "concurrent")]
    let mut prev_i_receivers: Vec<Option<Receiver<()>>> = Vec::new();
    #[cfg(feature = "concurrent")]
    for _ in 0..=2 * s {
        prev_i_receivers.push(None);
    }

    #[cfg(feature = "concurrent")]
    let mut handles: Vec<Vec<JoinHandle<()>>> = Vec::with_capacity(s as usize + 1);*/

    let timer = utils::Timer::start();

    // We use the formula d Δ_i + Δ_i d = Δ_{i-1} + τΔ_{i-1}
    for i in 0..=s {
        // Δ_i is a map C_s -> C_{s + i}. So to hit C_{2s}, we only need to compute up to 2
        // * s - i
        //        #[cfg(not(feature = "concurrent"))]
        let start = std::time::Instant::now();

        /* #[cfg(feature = "concurrent")]
        let mut handles_inner: Vec<JoinHandle<()>> = Vec::with_capacity((2 * s - i + 1) as usize);

        #[cfg(feature = "concurrent")]
        let mut last_receiver: Option<Receiver<()>> = None;

        #[cfg(feature = "concurrent")]
        let top_s = 2 * s - i;*/

        for s in 0..=2 * s - i {
            if i == 0 && s == 0 {
                let map = &delta[0][0];
                map.add_generators_from_matrix_rows(
                    0,
                    Matrix::from_vec(p, &[vec![1]]).as_slice_mut(),
                );
                map.extend_by_zero(2 * t);
                continue;
            }

            let square = Arc::clone(&square);

            let source = resolution.module(s);
            let target = square.module(s + i);

            let dtarget_module = square.module(s + i - 1);

            let d_res = resolution.differential(s);
            let d_target = square.differential(s + i as u32);

            let map = Arc::clone(&delta[i as usize][s as usize]);
            let prev_map = match s {
                0 => None,
                _ => Some(Arc::clone(&delta[i as usize][s as usize - 1])),
            };

            let prev_delta = match i {
                0 => None,
                _ => Some(Arc::clone(&delta[i as usize - 1][s as usize])),
            };

            /* #[cfg(feature = "concurrent")]
            let (sender, new_receiver) = unbounded();
            #[cfg(feature = "concurrent")]
            let (prev_i_sender, new_prev_i_receiver) = unbounded();


            #[cfg(feature = "concurrent")]
            let prev_i_receiver =
                std::mem::replace(&mut prev_i_receivers[s as usize], Some(new_prev_i_receiver));*/

            // Define this as a closure so that we can easily switch between threaded and
            // un-threaded
            let fun = move || {
                /* #[cfg(feature = "concurrent")]
                let mut token = bucket.take_token();*/

                for t in 0..=2 * t {
                    /* #[cfg(feature = "concurrent")]
                    {
                        token = bucket.recv2_or_release(token, &last_receiver, &prev_i_receiver);
                    }*/

                    let num_gens = source.number_of_gens_in_degree(t);

                    let fx_dim = target.dimension(t);
                    let fdx_dim = dtarget_module.dimension(t);

                    if fx_dim == 0 || fdx_dim == 0 || num_gens == 0 {
                        map.extend_by_zero(t);

                        /* #[cfg(feature = "concurrent")]
                        {
                            if s < top_s {
                                sender.send(()).unwrap();
                                prev_i_sender.send(()).unwrap();
                            }
                        }*/

                        continue;
                    }

                    let mut output_matrix = Matrix::new(p, num_gens, fx_dim);
                    let mut result = FpVector::new(p, fdx_dim);
                    for j in 0..num_gens {
                        if let Some(m) = &prev_delta {
                            // Δ_{i-1} x
                            let prevd = m.output(t, j);

                            // τ Δ_{i-1}x
                            square.swap(&mut result, prevd, s + i as u32 - 1, t);
                            result += prevd;
                        }

                        if let Some(m) = &prev_map {
                            let dx = d_res.output(t, j);
                            m.apply(result.as_slice_mut(), 1, t, dx.as_slice());
                        }
                        assert!(d_target.apply_quasi_inverse(
                            output_matrix[j].as_slice_mut(),
                            t,
                            result.as_slice(),
                        ));

                        result.set_to_zero();
                    }
                    map.add_generators_from_matrix_rows(t, output_matrix.as_slice_mut());

                    /* #[cfg(feature = "concurrent")]
                    {
                        if s < top_s {
                            sender.send(()).unwrap();
                            prev_i_sender.send(()).unwrap();
                        }
                    }*/
                }
            };

            /* #[cfg(feature = "concurrent")]
            {
                let handle = thread::Builder::new()
                    .name(format!("D_{}, s = {}", i, s))
                    .spawn(fun);
                last_receiver = Some(new_receiver);
                handles_inner.push(handle.unwrap());
            }*/
            // #[cfg(not(feature = "concurrent"))]
            fun();
        }
        /* #[cfg(feature = "concurrent")]
        handles.push(handles_inner); */

        // #[cfg(not(feature = "concurrent"))]
        {
            let final_map = &delta[i as usize][(2 * s - i) as usize];
            let num_gens = resolution.number_of_gens_in_bidegree(2 * s - i, 2 * t);
            print!("Sq^{} ", s - i);
            utils::print_element(FpVector::from_slice(p, &class).as_slice(), n, s);

            print!(
                " = [{}]",
                (0..num_gens)
                    .map(|k| format!("{}", final_map.output(2 * t, k).entry(0)))
                    .format(", "),
            );
            stdout().flush().unwrap();
            eprint!(" ({:?})", start.elapsed());
            stderr().flush().unwrap();
            println!();
        }
    }

    /* #[cfg(feature = "concurrent")]
    for (i, handle_inner) in handles.into_iter().enumerate() {
        let i = i as u32;

        for handle in handle_inner {
            handle.join().unwrap();
        }
        let final_map = &delta[i as usize][(2 * s - i) as usize];
        let num_gens = resolution.number_of_gens_in_bidegree(2 * s - i, 2 * t);
        print!(
            "Sq^{} x_({}, {}, {}) = [{}]",
            s - i,
            t - s as i32,
            s,
            idx,
            (0..num_gens)
                .map(|k| format!("{}", final_map.output(2 * t, k).entry(0)))
                .collect::<Vec<_>>()
                .join(", "),
        );
        stdout().flush().unwrap();
        eprint!(" ({:?} total)", start.elapsed());
        stderr().flush().unwrap();
        println!();
    }*/

    timer.end(format_args!("Computed Steenrod operations"));
    Ok(())
}
