//! The F₂ row reduction to reduced row echelon form, dispatched to the device.

use super::{GpuContext, context, driver};
use crate::{matrix::Matrix, prime::TWO};

/// Least elimination work, in `rank² · cols`, for which we attempt the GPU row reduction.
///
/// Elimination is `O(r²c)`, and that is what the crossover follows — not the short side, and not
/// the size in bits. Neither of those separates the measured wins from the losses: an 8192 square
/// wins at 8 MB while a 64 × 1,600,000 matrix loses at 12 MB.
///
/// Set where the device wins outright rather than at the break-even point, because the two shape
/// families do not cross together: at equal work a near-square reduction runs 1.5–2x worse than a
/// wide one, so a threshold placed on the wide curve would admit squares the device loses. See
/// `EXPERIMENTS.md` for the sweep. Override with `FP_CUDA_RR_MIN_WORK`.
///
/// This is a high bar on purpose, and on near-square inputs it will rarely be met.
const DEFAULT_RR_MIN_WORK: u64 = 100_000_000_000;

/// Legacy minimum on the short side, `FP_CUDA_RR_THRESHOLD`.
///
/// Inert at its default of 0: [`DEFAULT_RR_MIN_WORK`] decides. Kept so that scripts setting it keep
/// working, and so `FP_CUDA_RR_THRESHOLD=8192` restores the original behaviour exactly.
const DEFAULT_RR_THRESHOLD: usize = 0;

/// The legacy short-side floor in use, overridable via `FP_CUDA_RR_THRESHOLD`.
fn rr_threshold() -> usize {
    std::env::var("FP_CUDA_RR_THRESHOLD")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_RR_THRESHOLD)
}

/// The elimination-work floor in use, overridable via `FP_CUDA_RR_MIN_WORK`.
fn rr_min_work() -> u64 {
    std::env::var("FP_CUDA_RR_MIN_WORK")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_RR_MIN_WORK)
}

/// Is this reduction worth the device?
///
/// Work decides; the legacy short-side floor applies only when explicitly set.
///
/// The rank is not known until the reduction runs, so this estimates it as `min(rows, cols) / 2`,
/// which is exact for the half-rank inputs the crossover was measured on and an over-estimate for
/// rank-deficient ones. Over-estimating admits a little work the device would lose on, which is the
/// cheaper error: the loss is bounded by the ratio at the threshold, while rejecting a large
/// reduction forfeits the whole win.
pub(crate) fn rr_worth_gpu(rows: usize, cols: usize) -> bool {
    let t = rr_threshold();
    if rows < t || cols < t {
        return false;
    }
    let rank = (rows.min(cols) / 2) as u64;
    rank.saturating_mul(rank).saturating_mul(cols as u64) >= rr_min_work()
}

/// Bytes one [`download_rows`] chunk may take, on the device and, for a padded destination, on
/// the host.
const DOWNLOAD_CHUNK_BYTES: usize = 256 << 20;

/// Try to row-reduce `m` to RREF on the GPU, in place.
///
/// Returns `Some(rank)`, leaving `m` in the same canonical reduced form `Matrix::row_reduce`
/// produces — pivot rows at the top in column order, zeros below, `pivots` set — and bit-identical
/// to it. Returns `None` if the GPU is unavailable, the matrix is below threshold, or the upload or
/// the reduction fails; `m` is then untouched, and the caller takes the CPU M4RI path.
///
/// The host holds nothing beside `m`: the device reads it in place, and writes the pivot rows back
/// into it a bounded chunk at a time. That also means a download failing after its first rows have
/// landed cannot fall back, since `m` is then neither its input nor its reduction, and panics.
pub(crate) fn try_row_reduce(m: &mut Matrix) -> Option<usize> {
    debug_assert_eq!(m.prime(), TWO);
    let (rows, cols) = (m.rows(), m.columns());
    if !rr_worth_gpu(rows, cols) {
        return None;
    }
    let gpu = context()?;
    match driver::run(|| device_job(gpu, m)) {
        Outcome::Declined => None,
        Outcome::Reduced { pivot_cols } => {
            finish_rref(m, &pivot_cols);
            Some(pivot_cols.len())
        }
        Outcome::Lost(e) => panic!(
            "[fp-cuda] row reduction {rows}x{cols}: the download failed after the matrix was \
             partly overwritten, so it cannot fall back to the CPU: {e}"
        ),
    }
}

/// What [`device_job`] did.
enum Outcome {
    /// Nothing written: the upload or the reduction failed.
    Declined,
    /// Rows `0..pivot_cols.len()` of the matrix hold its pivot rows, in order; the rest of it is
    /// still to be cleared ([`finish_rref`]).
    Reduced { pivot_cols: Vec<usize> },
    /// The download failed part way.
    Lost(String),
}

/// The reduction's device job, on the driver thread: upload `m` (a packed copy only when its row
/// stride is padded for column capacity), reduce, and download the pivot rows, in pivot order, into
/// `m`'s first rows.
///
/// The device copy's rows are padded to an even number of limbs, as every `DeviceMatrix`'s are,
/// which is what lets the forward pass XOR its trailing updates straight into it.
fn device_job(gpu: &GpuContext, m: &mut Matrix) -> Outcome {
    let (rows, cols) = (m.rows(), m.columns());
    let (stride, ms) = (cols.div_ceil(64), m.stride());
    let uploaded = if ms == stride {
        gpu.upload(&m.data()[..rows * stride], rows, cols)
    } else {
        let packed: Vec<u64> = (0..rows)
            .flat_map(|i| m.data()[i * ms..][..stride].iter().copied())
            .collect();
        gpu.upload(&packed, rows, cols)
    };
    let Ok(mut dm) = uploaded else {
        return Outcome::Declined;
    };
    let Ok((perm, r, pivot_cols)) = gpu.row_reduce_dev(&mut dm) else {
        return Outcome::Declined;
    };
    match download_rows(gpu, &dm, &perm, r, stride, m.data_mut(), ms) {
        Ok(()) => Outcome::Reduced { pivot_cols },
        Err(e) => Outcome::Lost(e.to_string()),
    }
}

/// Download the first `ncols` limbs of the rows at perm positions `[0, r)` of a reduced device
/// matrix — pivot order — into rows `0..r` of `dst`, whose rows are `dst_stride >= ncols` limbs
/// apart.
///
/// A chunk at a time, so neither side holds more than [`DOWNLOAD_CHUNK_BYTES`] beyond the matrices
/// themselves: a tight `dst` takes each chunk directly, a padded one through one chunk of staging.
fn download_rows(
    gpu: &GpuContext,
    dm: &super::DeviceMatrix,
    perm: &cudarc::driver::CudaSlice<u32>,
    r: usize,
    ncols: usize,
    dst: &mut [u64],
    dst_stride: usize,
) -> anyhow::Result<()> {
    debug_assert!(dst_stride >= ncols);
    if r == 0 || ncols == 0 {
        return Ok(());
    }
    // Few enough rows that the gather's 32-bit row count cannot overflow either.
    let chunk = (DOWNLOAD_CHUNK_BYTES / (8 * ncols)).clamp(1, (u32::MAX as usize / ncols).max(1));
    let mut staging = Vec::new();
    let mut a = 0;
    while a < r {
        let b = (a + chunk).min(r);
        let n = b - a;
        if dst_stride == ncols {
            gpu.download_rows_into(dm, perm, a, b, 0, ncols, &mut dst[a * ncols..b * ncols])?;
        } else {
            staging.resize(n * ncols, 0u64);
            gpu.download_rows_into(dm, perm, a, b, 0, ncols, &mut staging)?;
            for (j, row) in staging.chunks_exact(ncols).enumerate() {
                dst[(a + j) * dst_stride..][..ncols].copy_from_slice(row);
            }
        }
        a = b;
    }
    Ok(())
}

/// Finish the canonical RREF once its pivot rows are in place: every limb of `m` outside rows
/// `0..r`'s `cols.div_ceil(64)` logical limbs zeroed (a padded stride's tail, the zero rows, spare
/// row capacity), and the pivots set.
fn finish_rref(m: &mut Matrix, pivot_cols: &[usize]) {
    let stride = m.columns().div_ceil(64);
    let ms = m.stride();
    let r = pivot_cols.len();
    {
        let data = m.data_mut();
        if ms != stride {
            for i in 0..r {
                data[i * ms + stride..(i + 1) * ms].fill(0);
            }
        }
        data[r * ms..].fill(0);
    }
    m.initialize_pivots();
    let piv = m.pivots_mut();
    for (k, &q) in pivot_cols.iter().enumerate() {
        piv[q] = k as isize;
    }
}
