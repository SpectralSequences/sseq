//! The differential matrices of Nassau's algorithm, built as batches of Milnor products.
//!
//! A row of `d` applied to a basis element `Sq(R) g` is `Sq(R) · d(g)`, which [`FreeModule::act`]
//! computes one target generator block at a time. Here those multiplications are collected as
//! [`BatchProduct`]s instead, and evaluated together by [`multiply_batch`], which runs large
//! batches on the GPU. Rows with `R = 0` are copies of `d(g)` and are filled on the CPU.
//!
//! The rows are processed in chunks, so that the products, their term lists and the output of one
//! chunk stay within [`CHUNK_BYTES`].

use std::{collections::HashMap, sync::Arc};

use algebra::{
    MilnorAlgebra,
    milnor_algebra::batch::{
        BatchProduct, COL_MAP_DROP, batch_applicable, gpu_available, multiply_batch,
    },
    module::{
        FreeModule, Module,
        homomorphism::{FreeModuleHomomorphism, ModuleHomomorphism},
    },
};
use fp::{matrix::Matrix, vector::FpVector};

type Differential = FreeModuleHomomorphism<FreeModule<MilnorAlgebra>>;

/// Host bytes one chunk may hold in products and term lists, and again in output.
const CHUNK_BYTES: usize = 1 << 30;

/// Whether [`partial_matrix`] should build `hom`'s matrices: there is a GPU to run the batches
/// on, and the algebra is one the batches support.
pub(super) fn applicable(hom: &Differential) -> bool {
    gpu_available() && batch_applicable(&hom.target().algebra())
}

/// [`FreeModuleHomomorphism::get_partial_matrix_restricted`] of `hom`, with only the columns in
/// `col_mask` if one is given, in that order.
pub(super) fn partial_matrix(
    hom: &Differential,
    degree: i32,
    inputs: &[usize],
    target_dim: usize,
    col_mask: Option<&[usize]>,
) -> Matrix {
    let p = hom.prime();
    let out_cols = col_mask.map_or(target_dim, <[usize]>::len);
    let mut matrix = Matrix::new(p, inputs.len(), out_cols);
    if target_dim == 0 || out_cols == 0 {
        return matrix;
    }
    let target = hom.target();
    let algebra = target.algebra();
    algebra.compute_seqno_tables(degree - hom.degree_shift());

    // Under a mask the products write masked columns directly, through a map from every column
    // of the target to its position in the mask.
    let col_map: Option<Vec<u32>> = col_mask.map(|mask| {
        let mut map = vec![COL_MAP_DROP; target_dim];
        for (j, &c) in mask.iter().enumerate() {
            map[c] = j as u32;
        }
        map
    });

    let row_bytes = out_cols.div_ceil(32) * size_of::<u32>();
    let mut scratch = FpVector::new(p, out_cols);
    let mut buf = vec![0u8; out_cols.div_ceil(64) * size_of::<u64>()];
    let mut start = 0;
    while start < inputs.len() {
        let (products, end) = extract(
            hom,
            degree,
            inputs,
            start,
            target_dim,
            col_mask,
            &mut matrix,
            row_bytes,
        );
        if !products.is_empty() {
            let out = multiply_batch(
                &algebra,
                out_cols,
                col_map.as_deref(),
                end - start,
                &products,
            );
            for (row, limbs) in out.iter_rows().enumerate() {
                read_row(&mut buf, limbs, out_cols);
                scratch
                    .update_from_bytes(&mut &buf[..])
                    .expect("the buffer holds exactly the vector's limbs");
                matrix.row_mut(start + row).add(scratch.as_slice(), 1);
            }
        }
        start = end;
    }
    matrix
}

/// Copy a row of `u32` limbs into the little-endian bytes of an [`FpVector`] of length `len`,
/// clearing the bits past `len`, which the vector requires to be zero.
fn read_row(buf: &mut [u8], limbs: &[u32], len: usize) {
    buf.fill(0);
    for (bytes, limb) in buf.chunks_exact_mut(4).zip(limbs) {
        bytes.copy_from_slice(&limb.to_le_bytes());
    }
    let tail = len % 64;
    if tail != 0 {
        let last = buf.len() - 8;
        let v = u64::from_le_bytes(buf[last..].try_into().unwrap()) & ((1 << tail) - 1);
        buf[last..].copy_from_slice(&v.to_le_bytes());
    }
}

/// The products of the rows of `inputs` from `start` on, until a chunk is full, with row indices
/// relative to `start`; the rows with `R = 0` are filled into `matrix` directly. Returns the
/// products and the end of the chunk.
#[allow(clippy::too_many_arguments)]
fn extract(
    hom: &Differential,
    degree: i32,
    inputs: &[usize],
    start: usize,
    target_dim: usize,
    col_mask: Option<&[usize]>,
    matrix: &mut Matrix,
    row_bytes: usize,
) -> (Vec<BatchProduct>, usize) {
    let source = hom.source();
    let target = hom.target();
    let shift = hom.degree_shift();
    let mut products = Vec::new();
    let mut scratch = col_mask.map(|_| FpVector::new(hom.prime(), target_dim));
    // `d(g)` restricted to one target generator block, shared by every product that multiplies
    // it: keyed by the generator and the start of the block.
    let mut lists: HashMap<(i32, usize, usize), Option<Arc<[usize]>>> = HashMap::new();
    let mut list_bytes = 0;

    let mut row = start;
    while row < inputs.len() {
        let r = row;
        row += 1;
        let op_gen = source.index_to_op_gen(degree, inputs[r]);
        if op_gen.generator_degree < hom.min_degree() {
            continue;
        }
        if op_gen.operation_degree == 0 {
            match (&mut scratch, col_mask) {
                (Some(scratch), Some(mask)) => {
                    scratch.set_to_zero();
                    hom.apply_to_basis_element_restricted(
                        scratch.as_slice_mut(),
                        1,
                        degree,
                        inputs[r],
                    );
                    matrix.row_mut(r).add_masked(scratch.as_slice(), 1, mask);
                }
                _ => hom.apply_to_basis_element_restricted(matrix.row_mut(r), 1, degree, inputs[r]),
            }
            continue;
        }
        let output = hom.output(op_gen.generator_degree, op_gen.generator_index);
        let input_degree = op_gen.generator_degree - shift;
        for block in target.iter_gen_offsets([input_degree, input_degree + op_gen.operation_degree])
        {
            let [input_start, output_start] = block.start;
            if input_start >= output.len() {
                break;
            }
            // The blocks are contiguous and in order, so one starting past the prefix lies
            // entirely outside it.
            if output_start >= target_dim {
                break;
            }
            let key = (op_gen.generator_degree, op_gen.generator_index, input_start);
            let terms = lists
                .entry(key)
                .or_insert_with(|| {
                    let s = output.as_slice().restrict(input_start, block.end[0]);
                    let terms: Option<Arc<[usize]>> =
                        (!s.is_zero()).then(|| s.iter_nonzero().map(|(i, _)| i).collect());
                    list_bytes += 64 + terms.as_ref().map_or(0, |t| size_of_val(&**t));
                    terms
                })
                .clone();
            let Some(term_indices) = terms else {
                continue;
            };
            products.push(BatchProduct {
                r_degree: op_gen.operation_degree,
                r_idx: op_gen.operation_index,
                s_degree: input_degree - block.gen_deg,
                term_indices,
                row: r - start,
                out_offset: output_start,
            });
        }
        // Whole rows per chunk: a row's products go in one batch.
        if products.len() * size_of::<BatchProduct>() + list_bytes >= CHUNK_BYTES
            || (row - start) * row_bytes >= CHUNK_BYTES
        {
            break;
        }
    }
    (products, row)
}

#[cfg(test)]
mod tests {
    use algebra::milnor_algebra::batch::{gpu_failures, gpu_launches};
    use sseq::coordinates::Bidegree;

    use super::*;
    use crate::chain_complex::ChainComplex;

    /// The batched builds of real differentials against the CPU, plain and masked. Fails without a
    /// GPU, which it requires to have run some of the batches.
    #[test]
    fn batched_matrices_match_the_cpu() {
        assert!(gpu_available(), "this test needs a CUDA device");
        let res = crate::utils::construct_nassau("S_2", None).unwrap();
        let (max_n, max_s) = (60, 8);
        res.compute_through_stem(Bidegree::n_s(max_n, max_s));
        let before = gpu_launches();
        for s in 1..=max_s {
            let d = res.differential(s);
            for t in s..=max_n + s {
                let source_dim = d.source().dimension(t);
                let target_dim = d.target().dimension(t);
                if source_dim == 0 || target_dim == 0 {
                    continue;
                }
                let inputs: Vec<usize> = (0..source_dim).collect();
                let cpu = d.get_partial_matrix_restricted(t, &inputs, target_dim);
                let gpu = partial_matrix(&d, t, &inputs, target_dim, None);
                assert_eq!(gpu, cpu, "d_{s} in degree {t}");

                // Every third column of the first two thirds.
                let mask: Vec<usize> = (0..target_dim - target_dim / 3).step_by(3).collect();
                let mut want = Matrix::new(d.prime(), source_dim, mask.len());
                for (mut row, full) in want.iter_mut().zip(cpu.iter()) {
                    row.add_masked(full, 1, &mask);
                }
                let gpu = partial_matrix(&d, t, &inputs, target_dim, Some(&mask));
                assert_eq!(gpu, want, "d_{s} in degree {t}, masked");
            }
        }
        assert!(gpu_launches() > before, "no batch ran on the device");
        assert_eq!(gpu_failures(), 0, "batches failed on the device");
    }
}
