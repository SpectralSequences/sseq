#![cfg_attr(rustfmt, rustfmt_skip)]
//! This file contains code used by main.rs

use serde_json::value::Value;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use ext::chain_complex::ChainComplex;
#[cfg(feature = "yoneda")]
use ext::chain_complex::TensorChainComplex;
use ext::module::homomorphism::{
    FiniteModuleHomomorphism, FreeModuleHomomorphism, IdentityHomomorphism, ModuleHomomorphism,
};
use ext::module::{BoundedModule, FiniteModule, Module};
use ext::resolution::Resolution;
use ext::resolution_homomorphism::ResolutionHomomorphism;
use ext::utils::{construct, construct_s_2, Config};
#[cfg(feature = "yoneda")]
use ext::yoneda::yoneda_representative_element;
use fp::matrix::Matrix;
use fp::prime::ValidPrime;
use fp::vector::FpVector;

use bivec::BiVec;
use query::*;
use saveload::{Load, Save};

#[cfg(feature = "concurrent")]
use std::{thread, thread::JoinHandle};

#[cfg(feature = "concurrent")]
use crossbeam_channel::{unbounded, Receiver};

#[cfg(feature = "concurrent")]
use thread_token::TokenBucket;

pub fn define_module() -> error::Result<String> {
    ext::cli_module_loaders::interactive_module_define()
}

pub fn resolve(config: &Config) -> error::Result<String> {
    let bundle = construct(config)?;
    let res = bundle.resolution.read();

    #[cfg(not(feature = "concurrent"))]
    res.resolve_through_degree(config.max_degree);

    #[cfg(feature = "concurrent")]
    {
        let num_threads = query_with_default_no_default_indicated("Number of threads", 2, Ok);
        let bucket = Arc::new(TokenBucket::new(num_threads));
        res.resolve_through_degree_concurrent(config.max_degree, &bucket);
    }

    // let hom = HomComplex::new(Arc::clone(&res), Arc::clone(&bundle.module));
    // hom.compute_cohomology_through_bidegree(res.max_computed_homological_degree(), res.max_computed_degree());
    Ok(res.graded_dimension_string())
}

#[cfg(not(feature = "yoneda"))]
pub fn yoneda(_: &Config) -> error::Result<String> {
    error::from_string("Compile with yoneda feature to enable yoneda command")
}

#[cfg(feature = "yoneda")]
pub fn yoneda(config: &Config) -> error::Result<String> {
    let bundle = construct(config)?;
    let module = bundle.chain_complex.module(0);
    let resolution = bundle.resolution.read();
    let min_degree = resolution.min_degree();

    #[cfg(feature = "concurrent")]
    let num_threads = query_with_default_no_default_indicated("Number of threads", 2, Ok);
    #[cfg(feature = "concurrent")]
    let bucket = Arc::new(TokenBucket::new(num_threads));

    loop {
        let x: i32 = query_with_default_no_default_indicated("t - s", 200, Ok);
        let s: u32 = query_with_default_no_default_indicated("s", 200, Ok);
        let i: usize = query_with_default_no_default_indicated("idx", 200, Ok);

        let start = Instant::now();
        let t = x + s as i32;

        #[cfg(not(feature = "concurrent"))]
        resolution.resolve_through_bidegree(s + 1, t + 1);

        #[cfg(feature = "concurrent")]
        resolution.resolve_through_bidegree_concurrent(s + 1, t + 1, &bucket);

        println!("Resolving time: {:?}", start.elapsed());

        let start = Instant::now();
        let yoneda = Arc::new(yoneda_representative_element(
            Arc::clone(&resolution.inner),
            s,
            t,
            i,
        ));

        println!("Finding representative time: {:?}", start.elapsed());

        let f = ResolutionHomomorphism::from_module_homomorphism(
            "".to_string(),
            Arc::clone(&resolution.inner),
            Arc::clone(&yoneda),
            &FiniteModuleHomomorphism::identity_homomorphism(Arc::clone(&module)),
        );

        f.extend(s, t);
        let final_map = f.get_map(s);
        let num_gens = resolution.inner.number_of_gens_in_bidegree(s, t);
        for i_ in 0..num_gens {
            assert_eq!(final_map.output(t, i_).dimension(), 1);
            if i_ == i {
                assert_eq!(final_map.output(t, i_).entry(0), 1);
            } else {
                assert_eq!(final_map.output(t, i_).entry(0), 0);
            }
        }

        let mut check = BiVec::from_vec(min_degree, vec![0; t as usize + 1 - min_degree as usize]);
        for s in 0..=s {
            let module = yoneda.module(s);

            println!(
                "Dimension of {}th module is {}",
                s,
                module.total_dimension()
            );

            for t in min_degree..=t {
                check[t] += (if s % 2 == 0 { 1 } else { -1 }) * module.dimension(t) as i32;
            }
        }
        for t in min_degree..=t {
            assert_eq!(
                check[t],
                module.dimension(t) as i32,
                "Incorrect Euler characteristic at t = {}",
                t
            );
        }

        let filename: String = query("Output file name (empty to skip)", Ok);

        if filename.is_empty() {
            continue;
        }

        let mut module_strings = Vec::with_capacity(s as usize + 2);
        match &*module {
            FiniteModule::FDModule(m) => {
                module_strings.push(m.to_minimal_json());
            }
            _ => {
                // This should never happen
                panic!();
            }
        };

        for s in 0..=s {
            match &*yoneda.module(s) {
                FiniteModule::FDModule(m) => module_strings.push(m.to_minimal_json()),
                _ => panic!(),
            }
        }

        let mut output_path_buf = PathBuf::from(filename.to_string());
        output_path_buf.set_extension("json");
        std::fs::write(&output_path_buf, Value::from(module_strings).to_string())?;
    }
}

#[cfg(not(feature = "yoneda"))]
pub fn steenrod() -> error::Result<String> {
    error::from_string("Compile with yoneda feature to enable steenrod command")
}

#[cfg(feature = "yoneda")]
pub fn steenrod() -> error::Result<String> {
    let bundle = construct_s_2("adem");
    let mut resolution = &*bundle.resolution.read();
    let module = bundle.chain_complex.module(0);

    let saved_resolution;

    if Path::new("resolution.save").exists() {
        print!("Loading saved resolution: ");
        let start = Instant::now();
        let f = File::open("resolution.save")?;
        let mut f = BufReader::new(f);
        saved_resolution = Resolution::load(&mut f, &bundle.chain_complex)?;
        resolution = &saved_resolution;
        println!("{:?}", start.elapsed());
    }

    let p = ValidPrime::new(2);
    #[cfg(feature = "concurrent")]
    let num_threads = query_with_default_no_default_indicated("Number of threads", 2, Ok);

    #[cfg(feature = "concurrent")]
    let bucket = Arc::new(TokenBucket::new(num_threads));

    loop {
        let x: i32 = query_with_default_no_default_indicated("t - s", 8, Ok);
        let s: u32 = query_with_default_no_default_indicated("s", 3, Ok);
        let idx: usize = query_with_default_no_default_indicated("idx", 0, Ok);

        let t = s as i32 + x;
        print!("Resolving ext: ");
        let start = Instant::now();

        #[cfg(feature = "concurrent")]
        resolution.resolve_through_bidegree_concurrent(2 * s, 2 * t, &bucket);

        #[cfg(not(feature = "concurrent"))]
        resolution.resolve_through_bidegree(2 * s, 2 * t);

        println!("{:?}", start.elapsed());

        print!("Saving resolution: ");
        let start = Instant::now();
        let file = File::create("resolution.save")?;
        let mut file = BufWriter::new(file);
        resolution.save(&mut file)?;
        drop(file);
        println!("{:?}", start.elapsed());

        print!("Computing Yoneda representative: ");
        let start = Instant::now();
        let yoneda = Arc::new(yoneda_representative_element(
            Arc::clone(&resolution.inner),
            s,
            t,
            idx,
        ));
        println!("{:?}", start.elapsed());

        print!("Dimensions of Yoneda representative: 1");
        let mut check = vec![0; t as usize + 1];
        for s in 0..=s {
            let module = yoneda.module(s);
            print!(" {}", module.total_dimension());

            for t in 0..=t {
                check[t as usize] += (if s % 2 == 0 { 1 } else { -1 }) * module.dimension(t) as i32;
            }
        }
        println!();

        // We check that lifting the identity returns the original class. Even if the
        // algorithm in yoneda.rs is incorrect, this ensures that a posteriori we happened
        // to have a valid Yoneda representative. (Not really --- we don't check it is exact, just
        // that its Euler characteristic is 0 in each degree)
        print!("Checking Yoneda representative: ");
        let start = Instant::now();
        {
            assert_eq!(check[0], 1, "Incorrect Euler characteristic at t = 0");
            for entry in check.into_iter().skip(1) {
                assert_eq!(entry, 0, "Incorrect Euler characteristic at t = {}", t);
            }
            let f = ResolutionHomomorphism::from_module_homomorphism(
                "".to_string(),
                Arc::clone(&resolution.inner),
                Arc::clone(&yoneda),
                &FiniteModuleHomomorphism::identity_homomorphism(Arc::clone(&module)),
            );

            f.extend(s, t);
            let final_map = f.get_map(s);
            let num_gens = resolution.inner.number_of_gens_in_bidegree(s, t);
            for i_ in 0..num_gens {
                assert_eq!(final_map.output(t, i_).dimension(), 1);
                if i_ == idx {
                    assert_eq!(final_map.output(t, i_).entry(0), 1);
                } else {
                    assert_eq!(final_map.output(t, i_).entry(0), 0);
                }
            }
        }
        println!("{:?}", start.elapsed());

        let square = Arc::new(TensorChainComplex::new(
            Arc::clone(&yoneda),
            Arc::clone(&yoneda),
        ));

        print!("Computing quasi_inverses: ");
        let start = Instant::now();
        square.compute_through_bidegree(2 * s, 2 * t);
        for s in 0..=2 * s {
            square
                .differential(s as u32)
                .compute_kernels_and_quasi_inverses_through_degree(2 * t);
        }
        println!("{:?}", start.elapsed());

        println!("Computing Steenrod operations: ");

        let mut delta = Vec::with_capacity(s as usize);

        for i in 0..=s {
            let mut maps: Vec<Arc<FreeModuleHomomorphism<_>>> =
                Vec::with_capacity(2 * s as usize - 1);

            for s in 0..=2 * s - i {
                let source = resolution.inner.module(s);
                let target = square.module(s + i);

                let map = FreeModuleHomomorphism::new(Arc::clone(&source), Arc::clone(&target), 0);
                maps.push(Arc::new(map));
            }
            delta.push(maps);
        }

        #[cfg(feature = "concurrent")]
        let mut prev_i_receivers: Vec<Option<Receiver<()>>> = Vec::new();
        #[cfg(feature = "concurrent")]
        for _ in 0..=2 * s {
            prev_i_receivers.push(None);
        }

        #[cfg(feature = "concurrent")]
        let mut handles: Vec<Vec<JoinHandle<()>>> = Vec::with_capacity(s as usize + 1);

        let start = Instant::now();

        // We use the formula d Δ_i + Δ_i d = Δ_{i-1} + τΔ_{i-1}
        for i in 0..=s {
            // Δ_i is a map C_s -> C_{s + i}. So to hit C_{2s}, we only need to compute up to 2
            // * s - i
            #[cfg(not(feature = "concurrent"))]
            let start = Instant::now();

            #[cfg(feature = "concurrent")]
            let mut handles_inner: Vec<JoinHandle<()>> =
                Vec::with_capacity((2 * s - i + 1) as usize);

            #[cfg(feature = "concurrent")]
            let mut last_receiver: Option<Receiver<()>> = None;

            #[cfg(feature = "concurrent")]
            let top_s = 2 * s - i;

            for s in 0..=2 * s - i {
                if i == 0 && s == 0 {
                    let map = &delta[0][0];
                    let lock = map.lock();
                    map.add_generators_from_matrix_rows(&lock, 0, Matrix::from_vec(p, &[vec![1]]).as_slice_mut());
                    map.extend_by_zero(&lock, 2 * t);
                    continue;
                }

                let square = Arc::clone(&square);

                let source = resolution.inner.module(s);
                let target = square.module(s + i);

                let dtarget_module = square.module(s + i - 1);

                let d_res = resolution.inner.differential(s);
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

                #[cfg(feature = "concurrent")]
                let (sender, new_receiver) = unbounded();
                #[cfg(feature = "concurrent")]
                let (prev_i_sender, new_prev_i_receiver) = unbounded();

                #[cfg(feature = "concurrent")]
                let bucket = Arc::clone(&bucket);

                #[cfg(feature = "concurrent")]
                let prev_i_receiver =
                    std::mem::replace(&mut prev_i_receivers[s as usize], Some(new_prev_i_receiver));

                // Define this as a closure so that we can easily switch between threaded and
                // un-threaded
                let fun = move || {
                    #[cfg(feature = "concurrent")]
                    let mut token = bucket.take_token();
                    let lock = map.lock();

                    for t in 0..=2 * t {
                        #[cfg(feature = "concurrent")]
                        {
                            token =
                                bucket.recv2_or_release(token, &last_receiver, &prev_i_receiver);
                        }

                        let num_gens = source.number_of_gens_in_degree(t);

                        let fx_dim = target.dimension(t);
                        let fdx_dim = dtarget_module.dimension(t);

                        if fx_dim == 0 || fdx_dim == 0 || num_gens == 0 {
                            map.extend_by_zero(&lock, t);

                            #[cfg(feature = "concurrent")]
                            {
                                if s < top_s {
                                    sender.send(()).unwrap();
                                    prev_i_sender.send(()).unwrap();
                                }
                            }

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
                            d_target.apply_quasi_inverse(output_matrix[j].as_slice_mut(), t, result.as_slice());

                            result.set_to_zero();
                        }
                        map.add_generators_from_matrix_rows(&lock, t, output_matrix.as_slice_mut());

                        #[cfg(feature = "concurrent")]
                        {
                            if s < top_s {
                                sender.send(()).unwrap();
                                prev_i_sender.send(()).unwrap();
                            }
                        }
                    }
                };

                #[cfg(feature = "concurrent")]
                {
                    let handle = thread::Builder::new()
                        .name(format!("D_{}, s = {}", i, s))
                        .spawn(fun);
                    last_receiver = Some(new_receiver);
                    handles_inner.push(handle.unwrap());
                }
                #[cfg(not(feature = "concurrent"))]
                fun();
            }
            #[cfg(feature = "concurrent")]
            handles.push(handles_inner);

            #[cfg(not(feature = "concurrent"))]
            {
                let final_map = &delta[i as usize][(2 * s - i) as usize];
                let num_gens = resolution
                    .inner
                    .number_of_gens_in_bidegree(2 * s - i, 2 * t);
                println!(
                    "Sq^{} x_{{{}, {}}}^({}) = [{}] ({:?})",
                    s - i,
                    t - s as i32,
                    s,
                    idx,
                    (0..num_gens)
                        .map(|k| format!("{}", final_map.output(2 * t, k).entry(0)))
                        .collect::<Vec<_>>()
                        .join(", "),
                    start.elapsed()
                );
            }
        }

        #[cfg(feature = "concurrent")]
        for (i, handle_inner) in handles.into_iter().enumerate() {
            let i = i as u32;

            for handle in handle_inner {
                handle.join().unwrap();
            }
            let final_map = &delta[i as usize][(2 * s - i) as usize];
            let num_gens = resolution
                .inner
                .number_of_gens_in_bidegree(2 * s - i, 2 * t);
            println!(
                "Sq^{} x_{{{}, {}}}^({}) = [{}] ({:?} total)",
                s - i,
                t - s as i32,
                s,
                idx,
                (0..num_gens)
                    .map(|k| format!("{}", final_map.output(2 * t, k).entry(0)))
                    .collect::<Vec<_>>()
                    .join(", "),
                start.elapsed()
            );
        }

        println!("Computing Steenrod operations: {:?}", start.elapsed());
    }
}

pub fn secondary() -> error::Result<String> {
    let bundle = construct_s_2("milnor");
    let mut resolution = &*bundle.resolution.read();

    let saved_resolution;

    let max_s = query_with_default("Max s", 7, Ok);
    let max_t = query_with_default("Max t", 30, Ok);

    let res_save_file: String = query_with_default("Resolution save file", String::from("resolution.save"), Ok);
    #[cfg(feature = "concurrent")]
    let del_save_file: String = query_with_default("Delta save file", String::from("ddelta.save"), Ok);

    #[cfg(feature = "concurrent")]
    let num_threads = query_with_default("Number of threads", 2, Ok);

    if res_save_file != "-" && Path::new(&*res_save_file).exists() {
        print!("Loading saved resolution: ");
        let start = Instant::now();
        let f = File::open(&*res_save_file)?;
        let mut f = BufReader::new(f);
        saved_resolution = Resolution::load(&mut f, &bundle.chain_complex)?;
        resolution = &saved_resolution;
        println!("{:.2?}", start.elapsed());
    }

    let should_resolve = max_s >= *resolution.next_s.lock() || max_t >= *resolution.next_t.lock();

    let save = || {
        if res_save_file != "-" {
            print!("Saving resolution: ");
            let start = Instant::now();
            let file = File::create(&*res_save_file).unwrap();
            let mut file = BufWriter::new(file);
            resolution.save(&mut file).unwrap();
            drop(file);
            println!("{:.2?}", start.elapsed());
        }
    };

    #[cfg(not(feature = "concurrent"))]
    let deltas = {
        if should_resolve {
            print!("Resolving module: ");
            let start = Instant::now();
            resolution.resolve_through_bidegree(max_s, max_t);
            println!("{:.2?}", start.elapsed());

            save();
        }

        ext::secondary::compute_delta(&resolution.inner, max_s, max_t)
    };

    #[cfg(feature = "concurrent")]
    let deltas = {
        let bucket = Arc::new(TokenBucket::new(num_threads));

        if should_resolve {
            print!("Resolving module: ");
            let start = Instant::now();
            resolution.resolve_through_bidegree_concurrent(max_s, max_t, &bucket);
            println!("{:.2?}", start.elapsed());

            save();
        }

        ext::secondary::compute_delta_concurrent(&resolution.inner, max_s, max_t, &bucket, &*del_save_file)
    };

    let mut filename = String::from("d2");
    while Path::new(&filename).exists() {
        filename.push('_');
    }
    let mut output = File::create(&filename).unwrap();

    for f in 1 .. max_t {
        for s in 1.. (max_s - 1) {
            let t = s as i32 + f;
            if t >= max_t {
                break;
            }
            let delta = &deltas[s as usize - 1];

            if delta.source().number_of_gens_in_degree(t + 1) == 0 {
                continue;
            }
            let d = delta.hom_k(t);

            for (i, entry) in d.into_iter().enumerate() {
                writeln!(output,
                    "d_2 x_({}, {}, {}) = {:?}", f, s, i, entry
                ).unwrap();
            }
        }
    }
    Ok(String::new())
}
