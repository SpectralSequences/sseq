//! This script converts between our basis and Bruner's basis. At the moment, most inputs are
//! hardcoded, and this only works for the sphere.
//!
//! The script performs the following procedure:
//!
//! 1. Compute our own resolution with the Milnor basis
//! 2. Create Bruner's resolution as a
//!    [`FiniteChainComplex`](ext::chain_complex::FiniteChainComplex) object
//! 3. Use a [`ResolutionHomomorphism`](ext::resolution_homomorphism::ResolutionHomomorphism) to
//!    lift the identity to a chain map from Bruner's resolution
//!    to our resolution. We should do it in this direction because we have stored the
//!    quasi-inverses for our resolution, but not Bruner's.
//! 4. Read off the transformation matrix we need
//!
//! The main extra work to put in is step (2), where we have to parse Bruner's differentials and
//! interpret it as a chain complex. Bruner's resolution can be found at
//! <https://archive.sigma2.no/pages/public/datasetDetail.jsf?id=10.11582/2022.00015>
//! while the descirption of his save file is at <https://arxiv.org/abs/2109.13117>.

use std::{
    fs::File,
    io,
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
};

use algebra::{
    milnor_algebra::MilnorBasisElement,
    module::{homomorphism::FreeModuleHomomorphism as FMH, FreeModule as FM, Module},
    Algebra, MilnorAlgebra,
};
use anyhow::{Context, Error, Result};
use ext::{
    chain_complex::{ChainComplex, FiniteChainComplex as FCC},
    resolution_homomorphism::ResolutionHomomorphism,
};
use fp::{matrix::Matrix, prime::TWO, vector::FpVector};
use sseq::coordinates::{Bidegree, BidegreeGenerator};

#[cfg(feature = "nassau")]
type FreeModule = FM<MilnorAlgebra>;
#[cfg(not(feature = "nassau"))]
type FreeModule = FM<algebra::SteenrodAlgebra>;

type FreeModuleHomomorphism = FMH<FreeModule>;
type FiniteChainComplex = FCC<FreeModule, FreeModuleHomomorphism>;

/// Read the first non-empty line of `data` into `buf`. Returns whether a line is read
fn read_line(data: &mut impl io::BufRead, buf: &mut String) -> Result<bool> {
    buf.clear();
    while buf.is_empty() {
        let num_bytes = data.read_line(buf)?;
        if num_bytes == 0 {
            return Ok(false);
        }
        // Remove newline character
        buf.pop();
    }
    Ok(true)
}

/// Viewing `s` as a whitespace-delimited array, take the first item and parse it into T.
fn entry<T>(x: &str) -> Result<(&str, T)>
where
    T: FromStr,
    Error: From<<T as FromStr>::Err>,
{
    let x = x.trim();
    match x.find(' ') {
        Some(k) => Ok((&x[k..], x[..k].parse()?)),
        None => Ok(("", x.parse()?)),
    }
}

/// Read an algebra element, where input contains
/// ```text
/// $op_deg _ $op
/// ```
/// This returns an iterator of indices of the operators whose sum is the element sought.
fn get_algebra_element<'a>(
    a: &'a MilnorAlgebra,
    input: &'a str,
) -> Result<impl Iterator<Item = usize> + 'a> {
    let (input, t) = entry(input)?;
    let (input, _) = entry::<u32>(input)?;

    let input = input.trim();
    assert_eq!(&input[0..1], "i");

    // Remove the i
    let input = &input[1..];
    // Remove the trailing ).
    let input = &input[..input.len() - 2];

    Ok(input.split(')').map(move |entry| {
        let entry = &entry[1..];
        let elt = MilnorBasisElement {
            q_part: 0,
            p_part: entry.split(',').map(|x| x.parse().unwrap()).collect(),
            degree: t,
        };
        a.basis_element_to_index(&elt)
    }))
}

/// Get a block describing a generator. Returns the degree and the value of the differential.
fn get_element(
    a: &MilnorAlgebra,
    m: &FreeModule,
    input: &mut impl io::BufRead,
) -> Result<Option<(i32, FpVector)>> {
    let mut buf = String::new();
    if !read_line(input, &mut buf)? {
        return Ok(None);
    }
    let degree: i32 = buf.trim().parse()?;
    a.compute_basis(degree);
    m.compute_basis(degree);

    read_line(input, &mut buf)?;
    let num_lines: usize = buf.trim().parse()?;

    let mut result = FpVector::new(TWO, m.dimension(degree));

    for _ in 0..num_lines {
        read_line(input, &mut buf)?;
        let (rem, gen_idx) = entry::<usize>(&buf)?;
        let offset = m.internal_generator_offset(degree, gen_idx);
        for op in get_algebra_element(a, &rem[1..])? {
            result.add_basis_element(offset + op, 1);
        }
    }
    Ok(Some((degree, result)))
}

/// Create a new `FiniteChainComplex` with `num_s` many non-zero modules.
fn create_chain_complex(num_s: usize) -> FiniteChainComplex {
    #[cfg(feature = "nassau")]
    let algebra: Arc<MilnorAlgebra> = Arc::new(MilnorAlgebra::new(TWO, false));

    #[cfg(not(feature = "nassau"))]
    let algebra: Arc<algebra::SteenrodAlgebra> = Arc::new(algebra::SteenrodAlgebra::MilnorAlgebra(
        MilnorAlgebra::new(TWO, false),
    ));

    let mut modules: Vec<Arc<FreeModule>> = Vec::with_capacity(num_s);
    let mut differentials: Vec<Arc<FreeModuleHomomorphism>> = Vec::with_capacity(num_s - 1);
    for _ in 0..num_s {
        modules.push(Arc::new(FreeModule::new(
            Arc::clone(&algebra),
            String::new(),
            0,
        )));
    }
    for s in 1..num_s {
        differentials.push(Arc::new(FreeModuleHomomorphism::new(
            Arc::clone(&modules[s]),
            Arc::clone(&modules[s - 1]),
            0,
        )));
    }
    FiniteChainComplex::new(modules, differentials)
}

/// Read the Diff.$N files in `data_dir` and produce the corresponding chain complex object.
fn read_bruner_resolution(data_dir: &Path, max_n: i32) -> Result<(u32, FiniteChainComplex)> {
    let num_s: usize = data_dir.read_dir()?.count();

    let cc = create_chain_complex(num_s);
    let algebra = cc.algebra();

    let algebra: &MilnorAlgebra = algebra.as_ref().try_into()?;

    let mut buf = String::new();
    let s = num_s as u32 - 1;

    algebra.compute_basis(max_n + s as i32 + 1);
    // Handle s = 0
    {
        // TODO: actually parse file
        let m = cc.module(0);
        m.add_generators(0, 1, None);
        m.extend_by_zero(max_n + 1);
    }

    for s in 1..num_s as u32 {
        let m = cc.module(s);
        let d = cc.differential(s);

        let mut f = io::BufReader::new(
            File::open(data_dir.join(format!("hDiff.{s}")))
                .with_context(|| format!("Failed to read hDiff.{s}"))?,
        );

        read_line(&mut f, &mut buf)?;

        let mut entries: Vec<FpVector> = Vec::new();
        let mut cur_degree: i32 = 0;

        while let Some((t, gen)) = get_element(algebra, cc.module(s - 1).as_ref(), &mut f)? {
            if t == cur_degree {
                entries.push(gen);
            } else {
                m.add_generators(cur_degree, entries.len(), None);
                d.add_generators_from_rows(cur_degree, entries);

                m.extend_by_zero(t - 1);
                d.extend_by_zero(t - 1);

                entries = vec![gen];
                cur_degree = t;
            }
        }
        m.add_generators(cur_degree, entries.len(), None);
        d.add_generators_from_rows(cur_degree, entries);

        m.extend_by_zero(max_n + s as i32 + 1);
        d.extend_by_zero(max_n + s as i32);
    }

    Ok((s, cc))
}

fn main() {
    ext::utils::init_logging();

    let data_dir = Path::new(file!()).parent().unwrap().join("bruner_data");
    let max_n: i32 = query::with_default("Max n", "20", str::parse);

    // Read in Bruner's resolution
    let (max_s, cc) = read_bruner_resolution(&data_dir, max_n).unwrap();
    let max = Bidegree::n_s(max_n, max_s);
    let cc = Arc::new(cc);

    let save_dir = query::optional("Save directory", |x| {
        core::result::Result::<PathBuf, std::convert::Infallible>::Ok(PathBuf::from(x))
    });

    #[cfg(feature = "nassau")]
    assert!(
        save_dir.is_some(),
        "A save directory is required for comparison between Bruner and Nassau resolutions."
    );

    let resolution = ext::utils::construct("S_2@milnor", save_dir).unwrap();

    resolution.compute_through_stem(max);

    let resolution = Arc::new(resolution);

    // Create a ResolutionHomomorphism object
    let hom = ResolutionHomomorphism::new(String::new(), cc, resolution, Bidegree::zero());

    // We have to explicitly tell it what to do at (0, 0)
    hom.extend_step(Bidegree::zero(), Some(&Matrix::from_vec(TWO, &[vec![1]])));
    hom.extend_all();

    // Now print the results
    println!("sseq_basis | bruner_basis");
    for b in hom.target.iter_stem() {
        let matrix = hom.get_map(b.s()).hom_k(b.t());

        for (i, row) in matrix.into_iter().enumerate() {
            let gen = BidegreeGenerator::new(b, i);
            println!("x_{gen:#} = {row:?}");
        }
    }
}
