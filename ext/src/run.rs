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
use algebra::module::homomorphism::{
    FiniteModuleHomomorphism, IdentityHomomorphism,
};

use algebra::module::{BoundedModule, FiniteModule, Module, homomorphism::ModuleHomomorphism};
use ext::resolution::Resolution;
use ext::resolution_homomorphism::ResolutionHomomorphism;
use ext::utils::{construct, construct_s_2, Config};
#[cfg(feature = "yoneda")]
use ext::yoneda::yoneda_representative_element;

use query::*;
use saveload::{Load, Save};

#[cfg(feature = "concurrent")]
use thread_token::TokenBucket;

pub fn define_module() -> error::Result<String> {
    ext::cli_module_loaders::interactive_module_define()
}

pub fn resolve(config: &Config) -> error::Result<String> {
    let res = construct(config)?;

    #[cfg(not(feature = "concurrent"))]
    res.resolve_through_degree(config.max_degree);

    #[cfg(feature = "concurrent")]
    {
        let num_threads = query_with_default("Number of threads", 2, Ok);
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
    let resolution = construct(config)?;
    let module = resolution.complex().module(0);
    let min_degree = resolution.min_degree();

    #[cfg(feature = "concurrent")]
    let num_threads = query_with_default("Number of threads", 2, Ok);
    #[cfg(feature = "concurrent")]
    let bucket = Arc::new(TokenBucket::new(num_threads));

    loop {
        let x: i32 = query_with_default("t - s", 200, Ok);
        let s: u32 = query_with_default("s", 200, Ok);
        let i: usize = query_with_default("idx", 200, Ok);

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

        let mut check = bivec::BiVec::from_vec(min_degree, vec![0; t as usize + 1 - min_degree as usize]);
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

pub fn secondary() -> error::Result<String> {
    let mut resolution = construct_s_2("milnor");

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
        resolution = Resolution::load(&mut f, &resolution.complex())?;
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
