//! Experimental: test which term orders Bruner's saved differentials are in normal form for,
//! modulo boundaries. Parsing code is copied from `bruner.rs`.
//!
//! The rest of this comment is from `bruner.rs`.
//!
//! This script converts between our basis and Bruner's basis. At the moment, most inputs are
//! hardcoded, and this only works for the sphere.
//!
//! The script performs the following procedure:
//!
//! 1. Compute our own resolution with the Milnor basis
//! 2. Create Bruner's resolution as a [`FiniteChainComplex`](FCC) object
//! 3. Use a [`ResolutionHomomorphism`] to lift the identity to a chain map from Bruner's resolution
//!    to our resolution. We should do it in this direction because we have stored the
//!    quasi-inverses for our resolution, but not Bruner's.
//! 4. Read off the transformation matrix we need
//!
//! The main extra work to put in is step (2), where we have to parse Bruner's differentials and
//! interpret it as a chain complex. Bruner's resolution can be found at
//! <https://archive.sigma2.no/pages/public/datasetDetail.jsf?id=10.11582/2022.00015> while the
//! descirption of his save file is at <https://arxiv.org/abs/2109.13117>.

use std::{fs::File, io, path::Path, str::FromStr, sync::Arc};

use algebra::{
    Algebra, MilnorAlgebra,
    milnor_algebra::{MilnorBasisElement, PPartEntry},
    module::{FreeModule as FM, Module, homomorphism::FreeModuleHomomorphism as FMH},
};
use anyhow::{Context, Error, Result};
use ext::chain_complex::{ChainComplex, FiniteChainComplex as FCC};
use fp::{matrix::Matrix, prime::TWO, vector::FpVector};

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
            p_part: entry
                .split(',')
                .map(|x| x.parse::<PPartEntry>().unwrap())
                .collect(),
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
fn read_bruner_resolution(data_dir: &Path, max_n: i32) -> Result<(i32, FiniteChainComplex)> {
    let num_s = data_dir.read_dir()?.count() as i32;

    let cc = create_chain_complex(num_s as usize);
    let algebra = cc.algebra();

    #[cfg_attr(feature = "nassau", allow(clippy::useless_conversion))]
    let algebra: &MilnorAlgebra = algebra.as_ref().try_into()?;

    let mut buf = String::new();
    let s = num_s - 1;

    algebra.compute_basis(max_n + s + 1);
    // Handle s = 0
    {
        // TODO: actually parse file
        let m = cc.module(0);
        m.add_generators(0, 1, None);
        m.extend_by_zero(max_n + 1);
    }

    for s in 1..num_s {
        let m = cc.module(s);
        let d = cc.differential(s);

        let mut f = io::BufReader::new(
            File::open(data_dir.join(format!("hDiff.{s}")))
                .with_context(|| format!("Failed to read hDiff.{s}"))?,
        );

        read_line(&mut f, &mut buf)?;

        let mut entries: Vec<FpVector> = Vec::new();
        let mut cur_degree: i32 = 0;

        while let Some((t, g)) = get_element(algebra, cc.module(s - 1).as_ref(), &mut f)? {
            if t == cur_degree {
                entries.push(g);
            } else {
                m.add_generators(cur_degree, entries.len(), None);
                d.add_generators_from_rows(cur_degree, entries);

                m.extend_by_zero(t - 1);
                d.extend_by_zero(t - 1);

                entries = vec![g];
                cur_degree = t;
            }
        }
        m.add_generators(cur_degree, entries.len(), None);
        d.add_generators_from_rows(cur_degree, entries);

        m.extend_by_zero(max_n + s + 1);
        d.extend_by_zero(max_n + s);
    }

    Ok((s, cc))
}

use algebra::module::homomorphism::ModuleHomomorphism;
use fp::matrix::Subspace;

fn key(order: &str, op: &MilnorBasisElement, op_idx: usize, gdeg: i32, gidx: usize) -> Vec<i64> {
    let (gpart, opart) = order.split_once('/').unwrap();
    let r: Vec<i64> = op.p_part.iter().map(|x| x as i64).collect();
    let may: i64 = r
        .iter()
        .enumerate()
        .map(|(j, x)| (j as i64 + 1) * (x.count_ones() as i64))
        .sum();
    let len = r.iter().rposition(|&x| x != 0).map_or(0, |x| x + 1) as i64;
    let g = vec![gdeg as i64, gidx as i64];
    let gk: Vec<i64> = match gpart {
        "gasc" => g.clone(),
        "gdesc" => g.iter().map(|x| -x).collect(),
        "none" => vec![],
        _ => panic!(),
    };
    let mut ok: Vec<i64> = match opart {
        "idx" => vec![op_idx as i64],
        "idxrev" => vec![-(op_idx as i64)],
        "may" => vec![may, op_idx as i64],
        "mayrev" => vec![-may, op_idx as i64],
        "lex" => {
            let mut v = r.clone();
            v.resize(8, 0);
            v
        }
        "lexrev" => {
            let mut v: Vec<i64> = r.iter().map(|x| -x).collect();
            v.resize(8, 0);
            v
        }
        "out" => {
            let mut v = vec![len];
            v.extend(r.iter().map(|x| -x));
            v.resize(9, 0);
            v
        }
        "outrev" => {
            let mut v = vec![-len];
            v.extend(r.iter().cloned());
            v.resize(9, 0);
            v
        }
        "rlex" => {
            let mut v = r.clone();
            v.resize(8, 0);
            v.reverse();
            v
        }
        "rlexrev" => {
            let mut v: Vec<i64> = r.iter().map(|x| -x).collect();
            v.resize(8, 0);
            v.reverse();
            v
        }
        _ => panic!(),
    };
    let mut k = gk;
    k.append(&mut ok);
    if gpart == "none" {
        k.extend(g.iter().map(|x| -x));
    } // op first, then generator (desc)
    k
}

fn main() -> anyhow::Result<()> {
    let data_dir = Path::new(file!()).parent().unwrap().join("bruner_data");
    let (max_s, cc) = read_bruner_resolution(&data_dir, 30).unwrap();
    let algebra = cc.algebra();
    let algebra: &MilnorAlgebra = algebra.as_ref().try_into()?;
    let mut orders = vec![];
    for g in ["gasc", "gdesc", "none"] {
        for o in [
            "idx", "idxrev", "may", "mayrev", "lex", "lexrev", "out", "outrev", "rlex", "rlexrev",
        ] {
            orders.push(format!("{g}/{o}"));
        }
    }
    let mut results = vec![];
    for order in &orders {
        let (mut viol_gens, mut viol_terms, mut total_gens, mut total_terms) = (0, 0, 0, 0);
        for s in 1..=max_s {
            let src = cc.module(s);
            let tgt = cc.module(s - 1);
            let d = cc.differential(s);
            for t in 0..=30 {
                let ng = src.number_of_gens_in_degree(t);
                if ng == 0 {
                    continue;
                }
                let dim = tgt.dimension(t);
                let old: Vec<usize> = (0..src.dimension(t) - ng).collect();
                let bmat = d.get_partial_matrix(t, &old);
                let keys: Vec<Vec<i64>> = (0..dim)
                    .map(|i| {
                        let og = tgt.index_to_op_gen(t, i);
                        let op = algebra
                            .basis_element_from_index(og.operation_degree, og.operation_index);
                        key(
                            order,
                            &op,
                            og.operation_index,
                            og.generator_degree,
                            og.generator_index,
                        )
                    })
                    .collect();
                let mut perm: Vec<usize> = (0..dim).collect();
                perm.sort_by(|&a, &b| keys[a].cmp(&keys[b]).then(a.cmp(&b)));
                let mut pm = Matrix::new(TWO, bmat.rows(), dim);
                for r in 0..bmat.rows() {
                    for (j, &c) in perm.iter().enumerate() {
                        pm.row_mut(r).set_entry(j, bmat.row(r).entry(c));
                    }
                }
                let sub = Subspace::from_matrix(pm);
                let piv = sub.pivots();
                for i in 0..ng {
                    let v = d.output(t, i);
                    let mut bad = 0;
                    for (j, &c) in perm.iter().enumerate() {
                        if piv[j] >= 0 && v.entry(c) != 0 {
                            bad += 1;
                        }
                    }
                    total_gens += 1;
                    total_terms += v.iter_nonzero().count();
                    viol_terms += bad;
                    if bad > 0 {
                        viol_gens += 1;
                    }
                }
            }
        }
        results.push((
            viol_terms,
            viol_gens,
            order.clone(),
            total_gens,
            total_terms,
        ));
    }
    results.sort();
    for r in results {
        println!(
            "{:>14}  violating terms {:4}  gens not in normal form {:3}/{}  (total terms {})",
            r.2, r.0, r.1, r.3, r.4
        );
    }
    Ok(())
}
