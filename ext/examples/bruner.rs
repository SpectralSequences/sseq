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
//! interpret it as a chain complex.
//!
//! By default, Bruner's differentials are stored in files `Diff.N`, where `Diff.N` contains
//! differentials starting at filtration 0. This employs various encodings of Milnor basis
//! elements. We use the `i` encoding.
//!
//! If it comes in a different encoding, run ./seeres in the directory (after compiling the
//! program).  This outputs the data in `hDiff.N` files, which ought to be moved back to
//! `Diff.N`. We will read these as input, and they are stored in `bruner_data/`. The location is
//! hardcoded, but the range of Bruner's resolution is not. The bundled version includes the
//! resolution up to $n = 20$, but it can be replaced with farther resolutions.
//!
//! We should interpret Diff.N as a space-separated "CSV", where blank lines are insignificant.
//! The first line is a header file, which includes two numbers
//! ```text
//! s=$s n=$num_gens
//! ```
//! These are the filtration and the number of generators in the filtration.
//!
//! After the header, we have blocks corresponding to the generators.
//!
//! A block looks like
//! ```text
//! $num_gen: $gen_t
//!
//! $num_lines
//! $line1
//! $line2
//! $line3
//! ...
//! ```
//! In the first line `$num_gen` is the number of the generator, starting at 0, and `$gen_t` is the
//! degree of the generator added. The generators are listed in increasing `$gen_t`.
//!
//! The second line `$num_lines` is the number of lines in the value of the differential on this
//! generator, and the value of the differential is the sum of the following lines.
//!
//! Each line encodes the product of a generator with a basis element. The format of the line is as
//! follows:
//!
//! ```text
//! $gen_idx $op_deg $alg_dim $op
//! ```
//! Here `$gen_idx` is the index of the generator. This is the index within the free module one
//! filtration lower (i.e. the index in the file Diff.$(N-1)), and not the index within the whole
//! resolution.
//!
//! The next entry `$op_deg` is the degree of the operation. This information is redundant, as it
//! can be computed from either the generator index or the upcoming representation of the operation
//! itself. Nevertheless, it is convenient to have it available upfront.
//!
//! The third entry is the dimension of the algebra in degree $op_deg. Again this is not needed for
//! us.
//!
//! The final entry is the operation itself. This best explained by example:
//! ```text
//! i(7)(4,1)(0,0,1).
//! ```
//! denotes the operation $\Sq(7) + \Sq(4, 1) + \Sq(0, 0, 1)$.
//!
//! As an example, the block
//! ```text
//! 5 : 10
//!
//! 3
//! 0 8 4 i(8)(2,2).
//! 1 6 3 i(6)(0,2).
//! 4 1 1 i(1).
//! ```
//! means the fifth generator is a generator in degree 10, whose differential is
//!
//! $$(\Sq(8) + \Sq(2, 2)) g_0 + (\Sq(6) + \Sq(0, 2)) g_1 + \Sq(1) g_4.$$
//!

use algebra::{
    milnor_algebra::MilnorBasisElement, module::homomorphism::FreeModuleHomomorphism as FMH,
    module::FreeModule as FM, module::Module, Algebra, MilnorAlgebra, MilnorAlgebraT,
    SteenrodAlgebra,
};
use error::{Error, Result};
use ext::{
    chain_complex::{ChainComplex, FiniteChainComplex as FCC},
    resolution_homomorphism::ResolutionHomomorphism,
    utils::construct,
};
use fp::{matrix::Matrix, prime::ValidPrime, vector::FpVector};
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
};

type FreeModule = FM<SteenrodAlgebra>;
type FreeModuleHomomorphism = FMH<FreeModule>;
type FiniteChainComplex = FCC<FreeModule, FreeModuleHomomorphism>;

const TWO: ValidPrime = ValidPrime::new(2);

/// Read the first non-empty line of `data` into `buf`. Returns whether a line is read
fn read_line(data: &mut impl BufRead, buf: &mut String) -> Result<bool> {
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
    input: &mut impl BufRead,
) -> Result<Option<(i32, FpVector)>> {
    let mut buf = String::new();
    if !read_line(input, &mut buf)? {
        return Ok(None);
    }
    let degree: i32 = buf.rsplit(':').next().unwrap().trim().parse()?;

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

/// Create a new FiniteChainComplex with `num_s` many non-zero modules.
fn create_chain_complex(num_s: usize) -> FiniteChainComplex {
    let algebra: Arc<SteenrodAlgebra> = Arc::new(MilnorAlgebra::new(TWO).into());

    let zero_module = Arc::new(FreeModule::new(Arc::clone(&algebra), String::from("0"), 0));

    let mut modules: Vec<Arc<FreeModule>> = Vec::with_capacity(num_s);
    let mut differentials: Vec<Arc<FreeModuleHomomorphism>> = Vec::with_capacity(num_s);
    for _ in 0..num_s {
        modules.push(Arc::new(FreeModule::new(
            Arc::clone(&algebra),
            String::new(),
            0,
        )));
    }
    differentials.push(Arc::new(FreeModuleHomomorphism::new(
        Arc::clone(&modules[0]),
        Arc::clone(&zero_module),
        0,
    )));
    for s in 1..num_s {
        differentials.push(Arc::new(FreeModuleHomomorphism::new(
            Arc::clone(&modules[s]),
            Arc::clone(&modules[s - 1]),
            0,
        )));
    }
    FiniteChainComplex {
        modules,
        zero_module,
        differentials,
    }
}

/// Read the Diff.$N files in `data_dir` and produce the corresponding chain complex object.
fn read_bruner_resolution(data_dir: PathBuf, max_n: i32) -> Result<(u32, FiniteChainComplex)> {
    let num_s: usize = data_dir.read_dir()?.count();

    let cc = create_chain_complex(num_s);
    let algebra = cc.algebra();
    let algebra = algebra.milnor_algebra();

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

        let mut f = BufReader::new(File::open(data_dir.join(format!("Diff.{}", s)))?);

        read_line(&mut f, &mut buf)?;

        let mut entries: Vec<FpVector> = Vec::new();
        let mut cur_degree: i32 = 0;

        while let Some((t, gen)) = get_element(algebra, &*cc.module(s - 1), &mut f)? {
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
    let data_dir = Path::new(file!()).parent().unwrap().join("bruner_data");
    let max_n: i32 = query::with_default("Max n", "20", str::parse);

    #[cfg(feature = "concurrent")]
    let bucket = ext::utils::query_bucket();

    // Read in Bruner's resolution
    let (max_s, cc) = read_bruner_resolution(data_dir, max_n).unwrap();
    let cc = Arc::new(cc);

    let resolution = construct("S_2@milnor", None).unwrap();

    #[cfg(feature = "concurrent")]
    resolution.compute_through_stem_concurrent(max_s, max_n, &bucket);

    #[cfg(not(feature = "concurrent"))]
    resolution.compute_through_stem(max_s, max_n);

    let resolution = Arc::new(resolution);

    // Create a ResolutionHomomorphism object
    let hom = ResolutionHomomorphism::new(String::new(), cc, resolution, 0, 0);

    // We have to explicitly tell it what to do at (0, 0)
    hom.extend_step(0, 0, Some(&Matrix::from_vec(TWO, &[vec![1]])));

    // We can then lift it by requiring it to be a chain map.
    #[cfg(feature = "concurrent")]
    hom.extend_through_stem_concurrent(max_s, max_n, &bucket);

    #[cfg(not(feature = "concurrent"))]
    hom.extend_through_stem(max_s, max_n);

    // Now print the results
    println!("sseq_basis | bruner_basis");
    for (s, n, t) in hom.target.iter_stem() {
        let matrix = hom.get_map(s).hom_k(t);

        for (i, row) in matrix.into_iter().enumerate() {
            println!("x_({},{},{}) = {:?}", n, s, i, row);
        }
    }
}
