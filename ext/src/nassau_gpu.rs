//! GPU-accelerated `get_partial_matrix` for Nassau's Milnor differentials.
//!
//! A Nassau differential is a `FreeModuleHomomorphism<FreeModule<MilnorAlgebra>>`;
//! building its (partial) matrix applies the differential to each input basis element,
//! whose dominant cost is the Milnor multiply `Sq(R) · s` in [`FreeModule::act`]. This
//! module batches every such multiply of one matrix build into a single GPU launch via
//! [`algebra::milnor_gpu::multiply_batch_on_gpu`]. See `benches/milnor_gpu_ab.rs` for a
//! CPU/GPU A/B of that launch.
//!
//! Only the *multiply* work is offloaded. Identity operations (`Sq(∅) = 1`, i.e.
//! `operation_degree == 0`) are plain copies with no admissible-matrix work, so they
//! are left to the CPU `apply_to_basis_element` per row. The output F₂ bits the kernel
//! returns are XORed into the matrix rows (bit `i` → `add_basis_element(i, 1)`), the
//! same layout the CPU path produces.
//!
//! Gated behind the `gpu` feature. Callers must ensure
//! [`MilnorAlgebra::gpu_multiply_applicable`] (`p = 2`, trivial profile, stable) — the
//! Nassau `S_2` regime — and that the algebra's basis and seqno tables reach `degree`.

use algebra::{
    MilnorAlgebra,
    milnor_gpu::{GpuProduct, multiply_batch_on_gpu},
    module::{
        FreeModule, Module,
        homomorphism::{FreeModuleHomomorphism, ModuleHomomorphism},
    },
};
use fp::matrix::Matrix;

type NassauDifferential = FreeModuleHomomorphism<FreeModule<MilnorAlgebra>>;

/// Whether the GPU `get_partial_matrix` path applies to this differential — the
/// seqno-table regime (`p = 2`, trivial profile, stable), i.e. Nassau `S_2`. The
/// (cheap, idempotent) seqno tables are built on demand in [`get_partial_matrix`].
pub fn applicable(hom: &NassauDifferential) -> bool {
    hom.target().algebra().gpu_multiply_applicable()
}

/// GPU analogue of [`ModuleHomomorphism::get_partial_matrix`] for a Nassau differential.
///
/// Produces the same matrix as the CPU path: rows are the differential applied to
/// `inputs[i]`, columns the target basis at `degree`. The multiply work of every input
/// is fused into one GPU launch; identity operations are filled in on the CPU.
///
/// Callers must ensure [`applicable`]; this mirrors the extraction the CPU
/// [`FreeModuleHomomorphism::apply_to_basis_element`] performs.
pub fn get_partial_matrix(hom: &NassauDifferential, degree: i32, inputs: &[usize]) -> Matrix {
    let (mut matrix, products) = extract(hom, degree, inputs);
    if !products.is_empty() {
        let target = hom.target();
        let algebra = target.algebra();
        // Idempotent + cheap (O(degree · width)); returns immediately once built.
        algebra.compute_seqno_tables(degree);
        let num_cols = target.dimension(degree);
        let rows = multiply_batch_on_gpu(&algebra, num_cols, inputs.len(), &products);
        for (row, limbs) in rows.iter().enumerate() {
            let mut target_row = matrix.row_mut(row);
            for (limb_idx, &limb) in limbs.iter().enumerate() {
                let mut bits = limb;
                while bits != 0 {
                    let b = bits.trailing_zeros() as usize;
                    target_row.add_basis_element(limb_idx * 32 + b, 1);
                    bits &= bits - 1;
                }
            }
        }
    }
    matrix
}

/// Extract the non-identity Milnor multiplies of one matrix build as [`GpuProduct`]s,
/// mirroring `apply_to_basis_element` → [`FreeModule::act`]. Returns the matrix with its
/// identity (`Sq(∅)`) and out-of-range rows already filled, plus the products whose GPU
/// (or CPU-reference) output completes it.
fn extract(hom: &NassauDifferential, degree: i32, inputs: &[usize]) -> (Matrix, Vec<GpuProduct>) {
    let p = hom.prime();
    let source = hom.source();
    let target = hom.target();
    let shift = hom.degree_shift();
    let out_dim = target.dimension(degree);
    let mut matrix = Matrix::new(p, inputs.len(), out_dim);
    let mut products: Vec<GpuProduct> = Vec::new();

    if out_dim == 0 {
        return (matrix, products);
    }

    for (row, &input_index) in inputs.iter().enumerate() {
        let ogp = source.index_to_op_gen(degree, input_index);
        if ogp.generator_degree < hom.min_degree() {
            continue;
        }
        if ogp.operation_degree == 0 {
            // Sq(∅) · s = s: let the CPU copy it directly into this row.
            hom.apply_to_basis_element(matrix.row_mut(row), 1, degree, input_index);
            continue;
        }
        let out_on_gen = hom.output(ogp.generator_degree, ogp.generator_index);
        let act_input_degree = ogp.generator_degree - shift;
        for gd in target
            .iter_gen_offsets::<2>([act_input_degree, act_input_degree + ogp.operation_degree])
        {
            let (input_start, input_end) = (gd.start[0], gd.end[0]);
            if input_start >= out_on_gen.len() {
                break;
            }
            let s_slice = out_on_gen.as_slice().restrict(input_start, input_end);
            if s_slice.is_zero() {
                continue;
            }
            products.push(GpuProduct {
                r_degree: ogp.operation_degree,
                r_idx: ogp.operation_index,
                s_degree: act_input_degree - gd.gen_deg,
                term_indices: s_slice.iter_nonzero().map(|(i, _)| i).collect(),
                row,
                // `Sq(R) · s` lands in this target generator's block of the output row,
                // which starts at gd.start[1] (the offsets at the *output* degree).
                out_offset: gd.start[1],
            });
        }
    }
    (matrix, products)
}

/// Build the matrix both ways and assert they agree; returns the CPU matrix. For the
/// `NASSAU_GPU_VERIFY` env gate — validates the GPU path over a real resolution before
/// it is trusted unconditionally.
pub fn get_partial_matrix_verified(
    hom: &NassauDifferential,
    degree: i32,
    inputs: &[usize],
) -> Matrix {
    let gpu = get_partial_matrix(hom, degree, inputs);
    let cpu = hom.get_partial_matrix(degree, inputs);
    for row in 0..inputs.len() {
        let g: Vec<usize> = gpu.row(row).iter_nonzero().map(|(i, _)| i).collect();
        let c: Vec<usize> = cpu.row(row).iter_nonzero().map(|(i, _)| i).collect();
        assert_eq!(
            g, c,
            "GPU/CPU get_partial_matrix mismatch at degree {degree}, row {row} (input {})",
            inputs[row],
        );
    }
    cpu
}
