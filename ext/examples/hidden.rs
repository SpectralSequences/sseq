//! Compute hidden α-extensions using precomputed tables. This assumes prime 2.
//!
//! # Usage guide
//! We use X to denote the spectrum whose hidden extensions we want to compute, and α the class we
//! multiply with.
//!
//! # Arguments
//! We clarify some less obvious arguments:
//!
//!  - **Name of α:** See "α products"
//!
//!  - **Dimension of Ext of X:** The output of the `num_gens` example on X
//!
//!  - **α products:** A file containing lines of the form `α x_(n, s, i) = [...]`, where the first
//!    word of each line is the name of the class we multiply with. The file can contain products
//!    with different elements, and the "Name of α" argument is used to filter out the relevant
//!    lines.
//!
//!    In existing applications, we use the output of `filtration_one` on X.
//!
//!  - **Inclusion map:** The output of `lift_hom` on the inclusion map of the bottom cell X -> X/α
//!
//!  - **Projection map:** The output of `lift_hom` on the projection map of the top cell X/α -> X
//!
//!  - **d2 of X:** The output of `secondary` on X.

#![allow(clippy::redundant_closure)]

use fp::matrix::AugmentedMatrix;
use fp::prime::ValidPrime;
use fp::vector::FpVector;
use std::fs::File;
use std::io::{BufRead, BufReader};

const TWO: ValidPrime = ValidPrime::new(2);

fn parse_vec<T: std::str::FromStr>(s: &str) -> Result<Vec<T>, T::Err> {
    if s.is_empty() {
        Ok(vec![])
    } else {
        s.split(',')
            .map(|x| x.trim().parse())
            .collect::<Result<Vec<_>, _>>()
    }
}

/// Monkey patch a function onto bigraded vectors
trait _Get {
    fn get_or(self, x: isize, y: isize, default: usize) -> usize;
}

impl _Get for &[Vec<usize>] {
    fn get_or(self, x: isize, y: isize, default: usize) -> usize {
        if x < 0 || y < 0 {
            return 0;
        }
        let x = x as usize;
        let y = y as usize;
        self.get(x as usize)
            .map(|v| v.get(y as usize).copied())
            .flatten()
            .unwrap_or(default)
    }
}

fn map_two<T, S>(input: &[Vec<T>], mut f: impl FnMut(usize, usize, &T) -> S) -> Vec<Vec<S>> {
    input
        .iter()
        .enumerate()
        .map(|(x, v)| v.iter().enumerate().map(|(y, v)| f(x, y, v)).collect())
        .collect()
}

pub fn gen_matrix_aug(
    source: &[Vec<usize>],
    target: &[Vec<usize>],
    offset: (isize, isize),
) -> Vec<Vec<AugmentedMatrix<2>>> {
    map_two(source, |f, s, &dim| {
        let mut matrix = AugmentedMatrix::<2>::new(
            TWO,
            dim,
            [
                target.get_or(f as isize + offset.0, s as isize + offset.1, 0),
                dim,
            ],
        );
        matrix.segment(1, 1).add_identity();
        matrix
    })
}

pub fn gen_matrix(
    source: &[Vec<usize>],
    target: &[Vec<usize>],
    offset: (isize, isize),
) -> Vec<Vec<AugmentedMatrix<1>>> {
    map_two(source, |f, s, &dim| {
        AugmentedMatrix::<1>::new(
            TWO,
            dim,
            [target.get_or(f as isize + offset.0, s as isize + offset.1, 0)],
        )
    })
}

pub fn parse_matrix<const N: usize>(
    input: impl std::io::Read,
    matrices: &mut [Vec<AugmentedMatrix<N>>],
    segment: usize,
    prefix: &str,
) -> error::Result {
    for line in BufReader::new(input).lines() {
        let line = line?;
        if !line.starts_with(prefix) {
            continue;
        }
        let (left, right) = {
            let split = line.split_once('=').unwrap();
            (split.0.trim(), split.1.trim())
        };
        let (f, s, i) = {
            let start = left.rfind('(').unwrap();
            let end = left.find(')').unwrap();
            let d: Vec<usize> = parse_vec(&left[start + 1..end])?;
            (d[0], d[1], d[2])
        };
        if f >= matrices.len() || s >= matrices[0].len() {
            continue;
        }

        let mut m = matrices[f][s].segment(segment, segment);
        let mut row = m.row_mut(i);

        // The target of the matrix might be greater than max_n/max_s but was still computed. In
        // this case, we set the dimension to 0. Attempting to write values will cause errors.
        if row.as_slice().is_empty() {
            continue;
        }
        let value: Vec<u32> = parse_vec(&right[1..right.len() - 1])?;
        for (k, v) in value.into_iter().enumerate() {
            if k < row.as_slice().len() {
                row.set_entry(k, v);
            }
        }
    }
    Ok(())
}

fn main() -> error::Result {
    let max_s: usize = query::raw("Max s", str::parse);
    let max_n: usize = query::raw("Max n", str::parse);
    let alpha_n: usize = query::raw("Stem of α", str::parse);
    let alpha_name: String = query::raw("Name of α", str::parse);

    let x_dim_file = query::raw("Dimension of Ext of X", |s| File::open(s));
    let xa_dim_file = query::raw("Dimension of Ext of X/α", |s| File::open(s));
    let product_file = query::raw("α products", |s| File::open(s));
    let inclusion_file = query::raw("Inclusion map", |s| File::open(s));
    let projection_file = query::raw("Projection map", |s| File::open(s));
    let x_d2_file = query::raw("d2 of X", |s| File::open(s));
    let xa_d2_file = query::raw("d2 of X/α", |s| File::open(s));

    let mut x_dim = vec![vec![0; 1 + max_s]; 1 + max_n];
    for line in BufReader::new(x_dim_file).lines() {
        let data: Vec<usize> = parse_vec(&*line?)?;
        if data[0] <= max_n && data[1] <= max_s {
            x_dim[data[0]][data[1]] = data[2];
        }
    }

    let mut xa_dim = vec![vec![0; 1 + max_s]; 1 + max_n];
    for line in BufReader::new(xa_dim_file).lines() {
        let data: Vec<usize> = parse_vec(&*line?)?;
        if data[0] <= max_n && data[1] <= max_s {
            xa_dim[data[0]][data[1]] = data[2];
        }
    }

    let mut alpha_d2 = map_two(&x_dim, |f, s, &dim| {
        let mut m = AugmentedMatrix::<3>::new(
            TWO,
            dim,
            [
                x_dim.get_or(f as isize - 1, s as isize + 2, 0),
                x_dim.get_or((f + alpha_n) as isize, s as isize + 1, 0),
                dim,
            ],
        );
        m.segment(2, 2).add_identity();
        m
    });

    let mut inclusion = gen_matrix_aug(&x_dim, &xa_dim, (0, 0));
    let mut projection = gen_matrix_aug(&xa_dim, &x_dim, (-1 - alpha_n as isize, 0));
    let mut xa_d2 = gen_matrix(&xa_dim, &xa_dim, (-1, 2));

    parse_matrix(x_d2_file, &mut alpha_d2, 0, "")?;
    parse_matrix(product_file, &mut alpha_d2, 1, &alpha_name)?;
    parse_matrix(inclusion_file, &mut inclusion, 0, "")?;
    parse_matrix(projection_file, &mut projection, 0, "")?;
    parse_matrix(xa_d2_file, &mut xa_d2, 0, "")?;

    alpha_d2.iter_mut().flatten().for_each(|m| m.row_reduce());
    inclusion.iter_mut().flatten().for_each(|m| m.row_reduce());
    projection.iter_mut().flatten().for_each(|m| m.row_reduce());

    for (f, m) in alpha_d2.iter().enumerate() {
        if f + alpha_n + 1 > max_n {
            continue;
        }
        for (s, m) in m.iter().enumerate() {
            if s + 2 > max_s || x_dim[f + alpha_n][s + 2] == 0 {
                continue;
            }
            let ker = m.compute_kernel();

            if ker.is_empty() {
                continue;
            }

            let proj_qi = projection[f + alpha_n + 1][s].compute_quasi_inverse();
            let inc_qi = inclusion[f + alpha_n][s + 2].compute_quasi_inverse();
            let d2 = &xa_d2[f + alpha_n + 1][s];

            let mut lift = FpVector::new(TWO, proj_qi.preimage().columns());
            let mut d2_val = FpVector::new(TWO, d2.columns());
            let mut ext_val = FpVector::new(TWO, inc_qi.preimage().columns());

            let d2_image = {
                let m = &alpha_d2[f + alpha_n + 1][s];
                // first_source_col is only useful for knowing where the first block ends
                (&**m).compute_image(m.end[0], m.start[1])
            };

            for row in ker.iter() {
                ext::utils::print_element(row.as_slice(), f as i32, s as u32);

                proj_qi.apply(lift.as_slice_mut(), 1, row.as_slice());
                d2.apply(d2_val.as_slice_mut(), 1, lift.as_slice());
                inc_qi.apply(ext_val.as_slice_mut(), 1, d2_val.as_slice());

                // Reduce by the image of the differential. The answer is correct either way but
                // this way the result will be zero if it vanishes on the E_3 page, thereby
                // eliminating "spurious" hidden extensions.
                d2_image.reduce(ext_val.as_slice_mut());
                println!(" -> {}", ext_val);

                lift.set_to_zero();
                d2_val.set_to_zero();
                ext_val.set_to_zero();
            }
        }
    }
    Ok(())
}
