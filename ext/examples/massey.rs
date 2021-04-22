//! Computes the triple Massey product up to a sign

use algebra::module::{
    homomorphism::{BoundedModuleHomomorphism, ModuleHomomorphism},
    FDModule,
};
use algebra::{AlgebraType, SteenrodAlgebra};
use ext::chain_complex::{ChainComplex, ChainHomotopy, FiniteChainComplex};
use ext::resolution::Resolution;
use ext::resolution_homomorphism::ResolutionHomomorphism;
use fp::matrix::Matrix;
use fp::prime::ValidPrime;
use saveload::Load;
use std::{fs::File, io::BufReader, path::Path, sync::Arc};

type CC<M> = FiniteChainComplex<M, BoundedModuleHomomorphism<M, M>>;

fn parse_vec(s: String) -> Result<Vec<u32>, String> {
    s[1..s.len() - 1]
        .split(',')
        .map(|x| x.trim().parse())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|x: core::num::ParseIntError| x.to_string())
}

fn main() -> error::Result<()> {
    let p: ValidPrime = query::with_default("p", "2", Ok);
    let algebra: AlgebraType = query::with_default("Basis", "adem", Ok);

    let algebra = Arc::new(SteenrodAlgebra::new(p, algebra));
    let module = Arc::new(FDModule::new(
        algebra,
        format!("S_{}", p),
        bivec::BiVec::from_vec(0, vec![1]),
    ));

    let ccdz: Arc<CC<_>> = Arc::new(FiniteChainComplex::ccdz(module));

    let save_file = query::optional("Resolution save file", |s: String| {
        if Path::new(&s).exists() {
            Ok(s)
        } else {
            Err("File not found".into())
        }
    });

    let resolution = match save_file {
        Some(f) => Resolution::load(&mut BufReader::new(File::open(f)?), &ccdz)?,
        None => Resolution::new(ccdz),
    };

    let resolution = Arc::new(resolution);

    const ORDINAL: [&str; 3] = ["first", "second", "third"];
    let mut s: [u32; 3] = [0; 3];
    let mut t: [i32; 3] = [0; 3];
    let mut class: [Vec<u32>; 3] = [vec![], vec![], vec![]];

    for i in 0..3 {
        eprintln!("\nEnter {} element:", ORDINAL[i]);
        let f: i32 = query::with_default("f", if i == 1 { "1" } else { "0" }, Ok);
        s[i] = query::with_default("s", "1", |v| {
            if v == 0 {
                Err("Must be positive filtration class".into())
            } else {
                Ok(v)
            }
        });
        t[i] = f + s[i] as i32;
        class[i] = query::with_default("class", "[1]", parse_vec);
    }

    let tot_s = s[0] + s[1] + s[2] - 1;
    let tot_t = t[0] + t[1] + t[2];

    if !resolution.has_computed_bidegree(tot_s, tot_t) {
        resolution.compute_through_stem(tot_s, tot_t - tot_s as i32);
    }

    let hom = [
        ResolutionHomomorphism::new(
            String::new(),
            Arc::clone(&resolution),
            Arc::clone(&resolution),
            s[0],
            t[0],
        ),
        ResolutionHomomorphism::new(
            String::new(),
            Arc::clone(&resolution),
            Arc::clone(&resolution),
            s[1],
            t[1],
        ),
    ];

    for i in 0..2 {
        let num_gens = resolution.module(s[i]).number_of_gens_in_degree(t[i]);
        assert_eq!(
            num_gens,
            class[i].len(),
            "Invalid class in bidegree ({}, {})",
            s[i],
            t[i] - s[i] as i32
        );

        let mut matrix = Matrix::new(p, num_gens, 1);

        for (k, &v) in class[i].iter().enumerate() {
            matrix[k].set_entry(0, v);
        }

        hom[i].extend_step(s[i], t[i], Some(&matrix));
    }

    hom[0].extend_through_stem(tot_s, tot_t - tot_s as i32);
    hom[1].extend_through_stem(s[1] + s[2] - 1, t[1] + t[2] - (s[1] + s[2] - 1) as i32);

    let homotopy = ChainHomotopy::new(
        Arc::clone(&resolution),
        Arc::clone(&resolution),
        s[0] + s[1],
        t[0] + t[1],
        |last_s, last_t, idx, row| {
            let mid_s = last_s + s[1];
            let mid_t = last_t + t[1];
            let source_t = last_t + t[0] + t[1];

            hom[1].get_map(last_s).apply(
                row,
                1,
                mid_t,
                hom[0].get_map(mid_s).output(source_t, idx).as_slice(),
            );
        },
    );

    homotopy.extend(tot_s, tot_t);

    let last = homotopy.homotopy(tot_s);
    let offset = resolution.module(s[2]).generator_offset(t[2], t[2], 0);
    print!("[");

    for i in 0..resolution.module(tot_s).number_of_gens_in_degree(tot_t) {
        let mut entry = 0;
        let output = last.output(tot_t, i);
        for (k, &v) in class[2].iter().enumerate() {
            if v != 0 {
                entry += v * output.entry(offset + k);
            }
        }
        print!("{}", entry % *p);
        if i != 0 {
            print!(", ");
        }
    }
    println!("]");
    Ok(())
}
