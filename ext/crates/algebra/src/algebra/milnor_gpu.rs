//! GPU offload for the Milnor multiply at `p = 2`, built on [CubeCL].
//!
//! Runs the admissible-matrix multiply
//! ([`super::milnor_algebra::MilnorAlgebra::multiply_basis_element_by_element_2`]) and the
//! hash-free `seqno` index as CubeCL kernels, batched per `get_partial_matrix` launch.
//!
//! The batched kernel `multiply_batch_kernel` fuses all `(R, s)` products of one
//! `get_partial_matrix` into a single launch — one thread per `(product, matrix, term)` pair,
//! decoded on-device from a prefix-sum over per-product pair counts. Admissible-matrix data is
//! deduplicated by distinct `R`, so a launch uploads compact per-`R`/per-product tables rather
//! than a per-pair table (which at scale would be gigabytes of almost-entirely-redundant data).
//! Its building blocks are the F₂ XOR accumulation (`xor_f2`), the on-device `seqno` index
//! (`seqno_core`/`seqno_kernel`, porting [`MilnorAlgebra::seqno`] as integer arithmetic over the
//! flat `g` table), and the single-`R` product (`multiply_pair`).
//!
//! Gated behind the `gpu` feature. Running needs the CUDA toolkit on `CUDA_PATH` /
//! `LD_LIBRARY_PATH` (the `gpu` dev shell in `ext/flake.nix` sets both) and a live
//! device; `cargo check`/`build` need neither (cudarc dlopens at runtime).
//!
//! [CubeCL]: https://github.com/tracel-ai/cubecl

use cubecl::{
    cuda::{CudaDevice, CudaRuntime},
    prelude::*,
};
use cubecl_common::stream_id::StreamId;

/// The single CUDA stream all GPU work is pinned to (via [`StreamId::executes`]).
///
/// CubeCL's memory pools are per-stream, and the resolution issues launches from many
/// rayon worker threads (each its own stream). Left alone, each stream's pool retains its
/// freed per-launch buffers (chiefly the hundreds-of-MB `out_h`), and across ~16 streams
/// they accumulate until the 4 GB card OOMs — `memory_cleanup` only trims the *calling*
/// stream's pool. Pinning every launch to one stream gives one pool that each launch's
/// `memory_cleanup` fully reclaims. Value 0 is a valid stream id (the first thread's).
const GPU_STREAM: StreamId = StreamId { value: 0 };

// Only the `#[cfg(test)]` standalone `seqno_kernel` sizes its working array by this bound; the
// production kernels use `WORKING_CAP`.
#[cfg(test)]
use crate::algebra::combinatorics::MAX_XI_TAU;
use crate::algebra::{Algebra, MilnorAlgebra, combinatorics::xi_degrees};

/// Comptime capacity for the per-thread `working` p_part in the multiply kernel.
/// The assembled p_part has length `max(term_len, mk_len)` before trimming, where
/// `mk_len = rows + cols − 1 ≤ MAX_XI_TAU + ⌈log2⌉`; 32 covers every in-range case.
const WORKING_CAP: usize = 32;

/// Narrow an admissible-matrix / p-part entry to the `u16` the GPU buffers use, failing loudly
/// instead of silently wrapping. Every entry is well within `u16` for the stem ranges this path
/// targets; a panic here means that assumption was pushed past its limit, which must not ship
/// truncated data to the device.
fn narrow_u16(v: u32) -> u16 {
    u16::try_from(v).expect("admissible/term entry exceeds u16")
}

use std::sync::atomic::{AtomicU64, Ordering};

/// Aggregate [`multiply_batch_on_gpu`] counters across all launches (call count, host
/// marshal µs, device µs, total pairs), for splitting a whole resolution's GPU overhead.
static BATCH_CALLS: AtomicU64 = AtomicU64::new(0);
static BATCH_MARSHAL_US: AtomicU64 = AtomicU64::new(0);
static BATCH_DEVICE_US: AtomicU64 = AtomicU64::new(0);
static BATCH_PAIRS: AtomicU64 = AtomicU64::new(0);

/// Read and reset the aggregate batch counters: `(calls, marshal_us, device_us, pairs)`.
pub fn take_batch_stats() -> (u64, u64, u64, u64) {
    (
        BATCH_CALLS.swap(0, Ordering::Relaxed),
        BATCH_MARSHAL_US.swap(0, Ordering::Relaxed),
        BATCH_DEVICE_US.swap(0, Ordering::Relaxed),
        BATCH_PAIRS.swap(0, Ordering::Relaxed),
    )
}

use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
};

use cubecl::server::Handle;

use crate::algebra::milnor_algebra::PPartEntry;

/// Where one `R`'s admissible-matrix data lives inside the resident master buffers.
#[derive(Clone, Copy)]
struct RInfo {
    cs_off: u32,
    mk_off: u32,
    cs_len: u32,
    mk_len: u32,
    num_mats: u32,
}

/// Process-global resident store of admissible-matrix data, both host- and device-side.
///
/// Admissible-matrix enumeration is a pure function of `R`'s p-part and the same
/// low-degree `R`s recur in essentially every bidegree, so the host master (`col_sums` /
/// `masks`, append-only, keyed by p-part in `index`) is enumerated once per distinct `R`
/// and never recomputed. The device copies (`cs_handle` / `mk_handle`) mirror the master
/// and are re-uploaded *only when it grows* — after the `R`s saturate (early in a
/// resolution) launches upload no admissible data at all, cutting the dominant transfer.
///
/// Guarded by a `Mutex` so the device section serializes across rayon worker threads: each
/// launch runs to its blocking readback before releasing, giving a happens-before edge and
/// no concurrent access — which is what CubeCL's single-device-thread managed-memory model
/// (its `unsafe impl Sync`) requires for a handle created on one thread to be reused on
/// another. Safe as a global because in the GPU path's regime (`p = 2`, trivial profile)
/// `admissible_matrices` depends only on the p-part, not on the algebra instance.
#[derive(Default)]
struct Resident {
    col_sums: Vec<u16>,
    masks: Vec<u16>,
    index: HashMap<Vec<PPartEntry>, RInfo>,
    cs_handle: Option<Handle>,
    mk_handle: Option<Handle>,
    cs_uploaded: usize,
    mk_uploaded: usize,
}

impl Resident {
    /// Global offsets/lengths of `R`'s admissible matrices in the master, enumerating and
    /// appending them on first sight (the append order fixes the offsets forever).
    fn ensure(&mut self, algebra: &MilnorAlgebra, p_part: &[PPartEntry]) -> RInfo {
        if let Some(info) = self.index.get(p_part) {
            return *info;
        }
        let (cs_len, mk_len, cs, mk) = algebra.admissible_matrices(p_part);
        let info = RInfo {
            cs_off: self.col_sums.len() as u32,
            mk_off: self.masks.len() as u32,
            cs_len: cs_len as u32,
            mk_len: mk_len as u32,
            num_mats: (mk.len() / mk_len) as u32,
        };
        self.col_sums.extend(cs.iter().map(|&v| narrow_u16(v)));
        self.masks.extend(mk.iter().map(|&v| narrow_u16(v)));
        self.index.insert(p_part.to_vec(), info);
        info
    }
}

static RESIDENT: LazyLock<Mutex<Resident>> = LazyLock::new(|| Mutex::new(Resident::default()));

/// Elementwise F₂ addition of two bit-packed vectors: `out[i] = a[i] ^ b[i]`.
///
/// One thread per `u32` limb. F₂ addition is XOR of the packed limbs, so this is
/// the output primitive the multiply kernels accumulate with.
#[cfg(test)]
#[cube(launch)]
fn xor_f2(a: &Array<u32>, b: &Array<u32>, out: &mut Array<u32>) {
    if ABSOLUTE_POS < out.len() {
        out[ABSOLUTE_POS] = a[ABSOLUTE_POS] ^ b[ABSOLUTE_POS];
    }
}

/// Compute `a ^ b` limb-wise on the default CUDA device.
///
/// Host-side driver for `xor_f2`: uploads both operands, launches one thread per
/// limb, and reads the result back. Panics if the operands differ in length.
#[cfg(test)]
pub fn xor_f2_on_gpu(a: &[u32], b: &[u32]) -> Vec<u32> {
    assert_eq!(a.len(), b.len(), "operands must have equal limb counts");
    let n = a.len();
    let client = CudaRuntime::client(&CudaDevice::default());

    let a_handle = client.create_from_slice(u32::as_bytes(a));
    let b_handle = client.create_from_slice(u32::as_bytes(b));
    let out_handle = client.empty(std::mem::size_of_val(a));

    // One 1-D block of `THREADS` units, enough blocks to cover every limb.
    const THREADS: u32 = 256;
    let cubes = (n as u32).div_ceil(THREADS);
    unsafe {
        xor_f2::launch::<CudaRuntime>(
            &client,
            CubeCount::Static(cubes, 1, 1),
            CubeDim::new_1d(THREADS),
            ArrayArg::from_raw_parts(a_handle, n),
            ArrayArg::from_raw_parts(b_handle, n),
            ArrayArg::from_raw_parts(out_handle.clone(), n),
        );
    }

    let bytes = client.read_one(out_handle).unwrap();
    u32::from_bytes(&bytes).to_vec()
}

/// Device port of [`MilnorAlgebra::seqno`]: the index of `P(working)` in the Milnor
/// basis of its degree, from the flat `g` table with no hashing. `working` holds the
/// (trimmed) p_part in its first `wlen` entries; `g` has row width `width`, entry
/// `(e, h)` at `g[e*width + h]`; `xi` are the ξ-degrees.
///
/// Thread/array indices are `usize`; p_part and table *values* are `u32`. A degree
/// (`cur_d`) is a value computed from `u32`s but also indexes `g`, so it is cast to
/// `usize` at the index sites. Shared by `seqno_kernel` and
/// `multiply_single_r_kernel` so both index outputs identically.
#[cube]
fn seqno_core(
    g: &Array<u32>,
    xi: &Array<u32>,
    working: &Array<u32>,
    wlen: usize,
    width: usize,
) -> u32 {
    // cur_d = Σ working[h] · xi[h].
    let mut cur_d = 0u32;
    for h in 0..wlen {
        cur_d += working[h] * xi[h];
    }

    // Rank by consuming positions from high to low; position 0 contributes nothing.
    let mut rank = 0u32;
    for hh in 1..wlen {
        let h = wlen - hh; // wlen-1 down to 1
        let r = working[h];
        if r != 0 {
            let below = cur_d - r * xi[h];
            let cur_row = usize::cast_from(cur_d) * width + h;
            let below_row = usize::cast_from(below) * width + h;
            rank += g[cur_row] - g[below_row];
            cur_d = below;
        }
    }
    rank
}

/// One thread per padded p_part: `out[i] = seqno(p_parts[i])`. `p_parts` is
/// `n × width` row-major, each row a p_part zero-padded to `width` (padding entries
/// are zero and skipped, so `wlen == width` matches the CPU's trimmed loop).
#[cfg(test)]
#[cube(launch)]
fn seqno_kernel(
    g: &Array<u32>,
    xi: &Array<u32>,
    p_parts: &Array<u32>,
    out: &mut Array<u32>,
    width: usize,
) {
    let idx = ABSOLUTE_POS;
    if idx >= out.len() {
        terminate!();
    }
    let base = idx * width;

    let mut working = Array::<u32>::new(MAX_XI_TAU);
    for h in 0..width {
        working[h] = p_parts[base + h];
    }
    out[idx] = seqno_core(g, xi, &working, width, width);
}

/// Run `seqno_kernel` over `n` padded p_parts and return their seqno indices.
///
/// `g`/`xi` come from `MilnorAlgebra::seqno_table_u32` and
/// [`crate::algebra::combinatorics::xi_degrees`]; `p_parts` is `n × width` row-major,
/// each row a p_part zero-padded to `width`.
#[cfg(test)]
pub fn seqno_batch_on_gpu(
    width: usize,
    xi: &[u32],
    g: &[u32],
    p_parts: &[u32],
    n: usize,
) -> Vec<u32> {
    assert_eq!(xi.len(), width, "xi must have `width` entries");
    assert_eq!(p_parts.len(), n * width, "p_parts must be n × width");
    let client = CudaRuntime::client(&CudaDevice::default());

    let g_h = client.create_from_slice(u32::as_bytes(g));
    let xi_h = client.create_from_slice(u32::as_bytes(xi));
    let pp_h = client.create_from_slice(u32::as_bytes(p_parts));
    let out_h = client.empty(n * size_of::<u32>());

    const THREADS: u32 = 256;
    let cubes = (n as u32).div_ceil(THREADS);
    unsafe {
        seqno_kernel::launch::<CudaRuntime>(
            &client,
            CubeCount::Static(cubes, 1, 1),
            CubeDim::new_1d(THREADS),
            ArrayArg::from_raw_parts(g_h, g.len()),
            ArrayArg::from_raw_parts(xi_h, xi.len()),
            ArrayArg::from_raw_parts(pp_h, p_parts.len()),
            ArrayArg::from_raw_parts(out_h.clone(), n),
            width,
        );
    }

    let bytes = client.read_one(out_h).unwrap();
    u32::from_bytes(&bytes).to_vec()
}

/// Assemble one `(admissible matrix, term)` product and XOR its F₂ output bit into
/// `out` at `row_base + idx`. The whole per-term test + output assembly of
/// [`MilnorAlgebra::multiply_basis_element_by_element_2`] lives here; both the
/// single-`R` and batch kernels call it with per-pair offsets.
///
/// The reference's three tail branches collapse into one uniform per-position rule:
/// for column `j`, with `b`, `cs`, `mk` the term / `col_sums` / `masks` entries (zero
/// outside their lengths) and `low = min(term_len, cs_len)` —
/// - `j < low`: reject if `cs > b` or `(b−cs) & mk`; else `working[j] = (b−cs) | mk`.
/// - `j ≥ low`: reject if `cs > 0` or `b & mk`; else `working[j] = b | mk`.
///
/// (For `j ≥ low` at most one of `b`, `cs` is in range, so this reproduces every
/// branch.) `seqno_core` gives the output index; the F₂ bit is XORed atomically
/// (collisions cancel mod 2). No explicit trailing-zero trim is needed — `seqno_core`
/// skips zero entries and `working` beyond the assembled length is zero, so the full
/// `WORKING_CAP` length is equivalent to the CPU's trimmed p_part (`xi` is host-padded
/// to `WORKING_CAP` so the `cur_d` sum stays in bounds; the extra terms are `0 · xi`).
#[cube]
#[allow(clippy::too_many_arguments)]
fn multiply_pair(
    col_sums: &Array<u16>,
    masks: &Array<u16>,
    term_pparts: &Array<u16>,
    g: &Array<u32>,
    xi: &Array<u32>,
    out: &mut Array<Atomic<u32>>,
    cs_base: usize,
    mk_base: usize,
    b_base: usize,
    term_len: usize,
    cs_len: usize,
    mk_len: usize,
    row_base: usize,
    out_offset: usize,
    width: usize,
) {
    let mut low = cs_len;
    if term_len < cs_len {
        low = term_len;
    }

    let mut working = Array::<u32>::new(WORKING_CAP);
    let mut rejected = false;

    for j in 0..WORKING_CAP {
        let mut b = 0u32;
        if j < term_len {
            b = u32::cast_from(term_pparts[b_base + j]);
        }
        let mut cs = 0u32;
        if j < cs_len {
            cs = u32::cast_from(col_sums[cs_base + j]);
        }
        let mut mk = 0u32;
        if j < mk_len {
            mk = u32::cast_from(masks[mk_base + j]);
        }

        let mut val = 0u32;
        if j < low {
            if cs > b {
                rejected = true;
            } else {
                let diff = b - cs;
                if (diff & mk) != 0u32 {
                    rejected = true;
                } else {
                    val = diff | mk;
                }
            }
        } else {
            if cs > 0u32 {
                rejected = true;
            }
            if (b & mk) != 0u32 {
                rejected = true;
            }
            val = b | mk;
        }
        working[j] = val;
    }

    if !rejected {
        // `seqno` indexes the algebra basis of the output degree; `out_offset` shifts it
        // to this product's target-generator block within the row (0 for a single-block
        // output). Both are bit offsets, added before splitting into (limb, bit).
        let idx = seqno_core(g, xi, &working, WORKING_CAP, width);
        let global_bit = out_offset + usize::cast_from(idx);
        let word = row_base + global_bit / 32;
        let bit = u32::cast_from(global_bit % 32);
        out[word].fetch_xor(1u32 << bit);
    }
}

/// Multiply `Sq(R) · s` for a single fixed operation `R` into one F₂ output vector.
/// One thread per `(matrix, term)` pair; delegates the assembly to `multiply_pair`.
#[cfg(test)]
#[cube(launch)]
#[allow(clippy::too_many_arguments)]
fn multiply_single_r_kernel(
    col_sums: &Array<u16>,
    masks: &Array<u16>,
    term_pparts: &Array<u16>,
    term_lens: &Array<u32>,
    g: &Array<u32>,
    xi: &Array<u32>,
    out: &mut Array<Atomic<u32>>,
    num_terms: usize,
    num_matrices: usize,
    cs_len: usize,
    mk_len: usize,
    width: usize,
) {
    let pair = ABSOLUTE_POS;
    if pair >= num_matrices * num_terms {
        terminate!();
    }
    let m = pair / num_terms;
    let t = pair % num_terms;
    let term_len = usize::cast_from(term_lens[t]);
    multiply_pair(
        col_sums,
        masks,
        term_pparts,
        g,
        xi,
        out,
        m * cs_len,
        m * mk_len,
        t * width,
        term_len,
        cs_len,
        mk_len,
        0,
        0,
        width,
    );
}

/// Batched multiply: one launch covering all `(R, s)` products of (e.g.) a
/// `get_partial_matrix` call. One thread per `(product, matrix, term)` pair.
///
/// Rather than a per-pair table (7 arrays × total-pairs — up to gigabytes at scale,
/// almost all redundant), the pair a thread handles is *decoded* from compact data:
/// - `prod_pair_start` is a prefix-sum of each product's pair count (`num_matrices ×
///   num_terms`), length `num_products + 1`. A binary search finds the product `p`
///   owning thread `k`, then `local = k − prod_pair_start[p]` splits into matrix
///   `m = local / num_terms` and term `t = local % num_terms`.
/// - Admissible-matrix data (`col_sums`/`masks`) is deduplicated by distinct `R`:
///   `prod_r_index[p]` indexes the per-`R` `r_*` tables, so an `R` shared across many
///   rows is stored (and uploaded) once.
///
/// Output is `num_rows` F₂ vectors of `num_limbs` `u32` limbs, row `r` at
/// `out[r*num_limbs ..]`.
#[cube(launch)]
#[allow(clippy::too_many_arguments)]
fn multiply_batch_kernel(
    col_sums: &Array<u16>,
    masks: &Array<u16>,
    term_pparts: &Array<u16>,
    term_lens: &Array<u32>,
    g: &Array<u32>,
    xi: &Array<u32>,
    out: &mut Array<Atomic<u32>>,
    r_cs_offset: &Array<u32>,
    r_mk_offset: &Array<u32>,
    r_cs_len: &Array<u32>,
    r_mk_len: &Array<u32>,
    prod_r_index: &Array<u32>,
    prod_term_start: &Array<u32>,
    prod_num_terms: &Array<u32>,
    prod_row_base: &Array<u32>,
    prod_out_offset: &Array<u32>,
    prod_pair_start: &Array<u32>,
    width: usize,
) {
    let k = ABSOLUTE_POS;
    let num_products = prod_pair_start.len() - 1;
    if k >= usize::cast_from(prod_pair_start[num_products]) {
        terminate!();
    }

    // Largest product `p` with `prod_pair_start[p] <= k` (every product owns ≥ 1 pair,
    // so `prod_pair_start` is strictly increasing and `p` is unique). 32 iterations
    // cover any realistic product count; once `hi = lo + 1` the update is idempotent.
    let mut lo = 0usize;
    let mut hi = num_products;
    for _ in 0..32 {
        if hi - lo > 1 {
            let mid = (lo + hi) / 2;
            if usize::cast_from(prod_pair_start[mid]) <= k {
                lo = mid;
            } else {
                hi = mid;
            }
        }
    }
    let p = lo;

    let ri = usize::cast_from(prod_r_index[p]);
    let nt = usize::cast_from(prod_num_terms[p]);
    let local = k - usize::cast_from(prod_pair_start[p]);
    let m = local / nt;
    let t = local % nt;

    let cs_len = usize::cast_from(r_cs_len[ri]);
    let mk_len = usize::cast_from(r_mk_len[ri]);
    let term_slot = usize::cast_from(prod_term_start[p]) + t;
    multiply_pair(
        col_sums,
        masks,
        term_pparts,
        g,
        xi,
        out,
        usize::cast_from(r_cs_offset[ri]) + m * cs_len,
        usize::cast_from(r_mk_offset[ri]) + m * mk_len,
        term_slot * width,
        usize::cast_from(term_lens[term_slot]),
        cs_len,
        mk_len,
        usize::cast_from(prod_row_base[p]),
        usize::cast_from(prod_out_offset[p]),
        width,
    );
}

/// Compute `Sq(R) · s` on the GPU for a single operation `R = (r_degree, r_idx)`,
/// returning the F₂ result as bit-packed `u32` limbs (bit `i` = basis index `i`).
///
/// `term_indices` are the nonzero indices of `s` in the degree-`s_degree` basis.
/// `R` must be non-empty (`Sq(∅) = 1` is the trivial identity the caller handles).
/// Requires the algebra's basis and seqno tables built through `r_degree + s_degree`.
#[cfg(test)]
pub fn multiply_single_r_on_gpu(
    algebra: &MilnorAlgebra,
    r_degree: i32,
    r_idx: usize,
    s_degree: i32,
    term_indices: &[usize],
) -> Vec<u32> {
    let (width, g) = algebra.seqno_table_u32();
    // Pad `xi` to `WORKING_CAP` so the kernel's `cur_d` sum (which runs to the full
    // working capacity) never reads out of bounds; padding entries multiply zero.
    let mut xi: Vec<u32> = xi_degrees(algebra.prime())
        .iter()
        .map(|&x| x as u32)
        .collect();
    xi.resize(WORKING_CAP, 0);

    let r = algebra.basis_element_from_index(r_degree, r_idx);
    assert!(
        !r.p_part.is_empty(),
        "R must be non-empty (Sq(∅) = 1 is the identity)"
    );
    let (cs_len, mk_len, cs32, mk32) = algebra.admissible_matrices(&r.p_part);
    // Ship admissible-matrix / term data as u16 (see `multiply_batch_on_gpu`).
    let mut col_sums: Vec<u16> = cs32.iter().map(|&v| narrow_u16(v)).collect();
    let masks: Vec<u16> = mk32.iter().map(|&v| narrow_u16(v)).collect();
    let num_matrices = masks.len() / mk_len;

    // Terms of s, each p_part padded to `width`, with their true (trimmed) lengths.
    let num_terms = term_indices.len();
    let mut term_pparts = vec![0u16; num_terms * width];
    let mut term_lens = vec![0u32; num_terms];
    for (t, &ti) in term_indices.iter().enumerate() {
        let elt = algebra.basis_element_from_index(s_degree, ti);
        term_lens[t] = elt.p_part.len() as u32;
        for (slot, &v) in term_pparts[t * width..(t + 1) * width]
            .iter_mut()
            .zip(&elt.p_part)
        {
            *slot = narrow_u16(v);
        }
    }

    let out_degree = r_degree + s_degree;
    let dim = algebra.dimension(out_degree);
    let num_limbs = dim.div_ceil(32).max(1);

    // Device buffers must be non-empty; `cs_len == 0` (R's max entry is 1) leaves
    // `col_sums` empty. The kernel never reads past the real lengths.
    if col_sums.is_empty() {
        col_sums.push(0);
    }

    let client = CudaRuntime::client(&CudaDevice::default());
    let cs_h = client.create_from_slice(u16::as_bytes(&col_sums));
    let mk_h = client.create_from_slice(u16::as_bytes(&masks));
    let tp_h = client.create_from_slice(u16::as_bytes(&term_pparts));
    let tl_h = client.create_from_slice(u32::as_bytes(&term_lens));
    let g_h = client.create_from_slice(u32::as_bytes(&g));
    let xi_h = client.create_from_slice(u32::as_bytes(&xi));
    let zeros = vec![0u32; num_limbs];
    let out_h = client.create_from_slice(u32::as_bytes(&zeros));

    let total_pairs = num_matrices * num_terms;
    const THREADS: u32 = 256;
    let cubes = (total_pairs as u32).div_ceil(THREADS).max(1);
    unsafe {
        multiply_single_r_kernel::launch::<CudaRuntime>(
            &client,
            CubeCount::Static(cubes, 1, 1),
            CubeDim::new_1d(THREADS),
            ArrayArg::from_raw_parts(cs_h, col_sums.len()),
            ArrayArg::from_raw_parts(mk_h, masks.len()),
            ArrayArg::from_raw_parts(tp_h, term_pparts.len()),
            ArrayArg::from_raw_parts(tl_h, term_lens.len()),
            ArrayArg::from_raw_parts(g_h, g.len()),
            ArrayArg::from_raw_parts(xi_h, xi.len()),
            ArrayArg::from_raw_parts(out_h.clone(), num_limbs),
            num_terms,
            num_matrices,
            cs_len,
            mk_len,
            width,
        );
    }

    let bytes = client.read_one(out_h).unwrap();
    u32::from_bytes(&bytes).to_vec()
}

/// One `Sq(R) · s` product of a batched launch, written into output row `row` at bit
/// offset `out_offset`.
///
/// `term_indices` are the nonzero indices of `s` in the degree-`s_degree` basis.
/// Multiple products may target the same `row` (their F₂ contributions XOR together),
/// mirroring how `get_partial_matrix` accumulates a row over generator blocks. The
/// product's `seqno` output indexes the algebra basis of the output degree; `out_offset`
/// is the start of the target-generator block that basis maps into within the row (0 when
/// the whole row is a single algebra element, as in the single-generator tests).
pub struct GpuProduct {
    pub r_degree: i32,
    pub r_idx: usize,
    pub s_degree: i32,
    pub term_indices: Vec<usize>,
    pub row: usize,
    pub out_offset: usize,
}

/// Compute a whole batch of `Sq(R) · s` products in a single GPU launch — the
/// The batched unit of one `get_partial_matrix` call. `R`s may differ (each contributes its
/// own admissible matrices). Returns `num_rows` F₂ vectors, each `⌈num_cols/32⌉`
/// bit-packed `u32` limbs.
///
/// `num_cols` is the *row* width — for a module row that is the module dimension (a sum
/// over generator blocks, generally larger than any single algebra degree's dimension),
/// with each product's `out_offset` selecting its block. Every product's
/// `out_offset + index` must be `< num_cols`. Every `R` must be non-empty; the algebra's
/// basis and seqno tables must reach each product's output degree (`r_degree + s_degree`).
pub fn multiply_batch_on_gpu(
    algebra: &MilnorAlgebra,
    num_cols: usize,
    num_rows: usize,
    products: &[GpuProduct],
) -> Vec<Vec<u32>> {
    let (width, g) = algebra.seqno_table_u32();
    let mut xi: Vec<u32> = xi_degrees(algebra.prime())
        .iter()
        .map(|&x| x as u32)
        .collect();
    xi.resize(WORKING_CAP, 0);

    let num_limbs = num_cols.div_ceil(32).max(1);

    let t_marshal = std::time::Instant::now();

    // The two heavy parts of marshalling — enumerating each distinct `R`'s admissible
    // matrices, and looking up + padding every term's p-part — are independent per item,
    // so they run in parallel (rayon via `concurrent`; serial otherwise). The cheap
    // sequential glue (interning `R`s, concatenation, prefix sums) stays on one thread.
    use maybe_rayon::prelude::*;

    // Intern distinct `R`s in first-seen order (cheap, sequential); record each product's
    // `R` index. Admissible-matrix data is thus deduplicated: an `R` shared across many
    // rows is enumerated and uploaded once.
    let mut r_index: std::collections::HashMap<(i32, usize), u32> =
        std::collections::HashMap::new();
    let mut distinct_r: Vec<(i32, usize)> = Vec::new();
    let mut prod_r_index: Vec<u32> = Vec::with_capacity(products.len());
    for prod in products {
        let ri = *r_index
            .entry((prod.r_degree, prod.r_idx))
            .or_insert_with(|| {
                let i = distinct_r.len() as u32;
                distinct_r.push((prod.r_degree, prod.r_idx));
                i
            });
        prod_r_index.push(ri);
    }

    // Admissible-matrix data (`col_sums`/`masks` + per-`R` offsets) is resident (built
    // under the `RESIDENT` lock below), so nothing to enumerate or lay out here.

    // Parallel: each product's term p-parts (padded to `width`) and lengths.
    let per_prod: Vec<(Vec<u16>, Vec<u32>)> = (0..products.len())
        .into_maybe_par_iter()
        .map(|pi| {
            let prod = &products[pi];
            let nt = prod.term_indices.len();
            let mut tp = vec![0u16; nt * width];
            let mut tl = Vec::with_capacity(nt);
            for (k, &ti) in prod.term_indices.iter().enumerate() {
                let elt = algebra.basis_element_from_index(prod.s_degree, ti);
                tl.push(elt.p_part.len() as u32);
                for (slot, &v) in tp[k * width..(k + 1) * width].iter_mut().zip(&elt.p_part) {
                    *slot = narrow_u16(v);
                }
            }
            (tp, tl)
        })
        .collect();

    // Resident admissible-matrix store: enumerate each new `R` once and reuse forever;
    // the per-`R` offsets are global (into the master `col_sums`/`masks`). Taking the lock
    // here also serializes the device section across rayon workers (see [`Resident`]).
    let mut resident = RESIDENT.lock().unwrap();
    let mut r_cs_offset: Vec<u32> = Vec::with_capacity(distinct_r.len());
    let mut r_mk_offset: Vec<u32> = Vec::with_capacity(distinct_r.len());
    let mut r_cs_len: Vec<u32> = Vec::with_capacity(distinct_r.len());
    let mut r_mk_len: Vec<u32> = Vec::with_capacity(distinct_r.len());
    let mut r_num_matrices: Vec<usize> = Vec::with_capacity(distinct_r.len());
    for &(rd, ridx) in &distinct_r {
        let r = algebra.basis_element_from_index(rd, ridx);
        assert!(!r.p_part.is_empty(), "each R must be non-empty");
        let info = resident.ensure(algebra, &r.p_part);
        r_cs_offset.push(info.cs_off);
        r_mk_offset.push(info.mk_off);
        r_cs_len.push(info.cs_len);
        r_mk_len.push(info.mk_len);
        r_num_matrices.push(info.num_mats as usize);
    }

    // Lay out per-product term data + records + the pair-count prefix sum (sequential).
    let mut term_pparts: Vec<u16> = Vec::new();
    let mut term_lens: Vec<u32> = Vec::new();
    let mut prod_term_start: Vec<u32> = Vec::with_capacity(products.len());
    let mut prod_num_terms: Vec<u32> = Vec::with_capacity(products.len());
    let mut prod_row_base: Vec<u32> = Vec::with_capacity(products.len());
    let mut prod_out_offset: Vec<u32> = Vec::with_capacity(products.len());
    let mut prod_pair_start: Vec<u32> = Vec::with_capacity(products.len() + 1);
    let mut pair_acc: usize = 0;
    for (pi, (tp, tl)) in per_prod.iter().enumerate() {
        let prod = &products[pi];
        let ri = prod_r_index[pi];
        prod_term_start.push(term_lens.len() as u32);
        term_lens.extend_from_slice(tl);
        term_pparts.extend_from_slice(tp);
        prod_pair_start.push(pair_acc as u32);
        pair_acc += r_num_matrices[ri as usize] * prod.term_indices.len();
        prod_num_terms.push(prod.term_indices.len() as u32);
        prod_row_base.push((prod.row * num_limbs) as u32);
        prod_out_offset.push(prod.out_offset as u32);
    }
    prod_pair_start.push(pair_acc as u32); // sentinel: total pair count at index num_products

    let total_pairs = pair_acc;
    let out_len = num_rows * num_limbs;
    if total_pairs == 0 {
        return vec![vec![0u32; num_limbs]; num_rows];
    }

    // The resident `col_sums`/`masks` are non-empty once any `R` is present (guaranteed
    // here, since `total_pairs > 0`); only `term_pparts` needs the non-empty guard.
    if term_pparts.is_empty() {
        term_pparts.push(0);
    }

    let marshal_ms = t_marshal.elapsed().as_secs_f64() * 1e3;

    let t_device = std::time::Instant::now();

    // Pin the whole device section to one CUDA stream (see [`GPU_STREAM`]) so a single
    // memory pool is reclaimed by `memory_cleanup`. Held under the `resident` lock, so this
    // stream is used by at most one thread at a time.
    let result = GPU_STREAM.executes(|| {
        let client = CudaRuntime::client(&CudaDevice::default());
        // Resident admissible buffers: (re-)upload the master only when it grew this
        // launch; otherwise reuse the handle from a previous launch and upload nothing.
        if resident.cs_handle.is_none() || resident.cs_uploaded != resident.col_sums.len() {
            resident.cs_handle = Some(client.create_from_slice(u16::as_bytes(&resident.col_sums)));
            resident.cs_uploaded = resident.col_sums.len();
        }
        if resident.mk_handle.is_none() || resident.mk_uploaded != resident.masks.len() {
            resident.mk_handle = Some(client.create_from_slice(u16::as_bytes(&resident.masks)));
            resident.mk_uploaded = resident.masks.len();
        }
        let cs_len_master = resident.col_sums.len();
        let mk_len_master = resident.masks.len();
        let cs_h = resident.cs_handle.clone().unwrap();
        let mk_h = resident.mk_handle.clone().unwrap();
        let tp_h = client.create_from_slice(u16::as_bytes(&term_pparts));
        let tl_h = client.create_from_slice(u32::as_bytes(&term_lens));
        let g_h = client.create_from_slice(u32::as_bytes(&g));
        let xi_h = client.create_from_slice(u32::as_bytes(&xi));
        let rco_h = client.create_from_slice(u32::as_bytes(&r_cs_offset));
        let rmo_h = client.create_from_slice(u32::as_bytes(&r_mk_offset));
        let rcl_h = client.create_from_slice(u32::as_bytes(&r_cs_len));
        let rml_h = client.create_from_slice(u32::as_bytes(&r_mk_len));
        let pri_h = client.create_from_slice(u32::as_bytes(&prod_r_index));
        let pts_h = client.create_from_slice(u32::as_bytes(&prod_term_start));
        let pnt_h = client.create_from_slice(u32::as_bytes(&prod_num_terms));
        let prb_h = client.create_from_slice(u32::as_bytes(&prod_row_base));
        let poo_h = client.create_from_slice(u32::as_bytes(&prod_out_offset));
        let pps_h = client.create_from_slice(u32::as_bytes(&prod_pair_start));
        let zeros = vec![0u32; out_len];
        let out_h = client.create_from_slice(u32::as_bytes(&zeros));
        const THREADS: u32 = 256;
        let cubes = (total_pairs as u32).div_ceil(THREADS).max(1);
        unsafe {
            multiply_batch_kernel::launch::<CudaRuntime>(
                &client,
                CubeCount::Static(cubes, 1, 1),
                CubeDim::new_1d(THREADS),
                ArrayArg::from_raw_parts(cs_h, cs_len_master),
                ArrayArg::from_raw_parts(mk_h, mk_len_master),
                ArrayArg::from_raw_parts(tp_h, term_pparts.len()),
                ArrayArg::from_raw_parts(tl_h, term_lens.len()),
                ArrayArg::from_raw_parts(g_h, g.len()),
                ArrayArg::from_raw_parts(xi_h, xi.len()),
                ArrayArg::from_raw_parts(out_h.clone(), out_len),
                ArrayArg::from_raw_parts(rco_h, r_cs_offset.len()),
                ArrayArg::from_raw_parts(rmo_h, r_mk_offset.len()),
                ArrayArg::from_raw_parts(rcl_h, r_cs_len.len()),
                ArrayArg::from_raw_parts(rml_h, r_mk_len.len()),
                ArrayArg::from_raw_parts(pri_h, prod_r_index.len()),
                ArrayArg::from_raw_parts(pts_h, prod_term_start.len()),
                ArrayArg::from_raw_parts(pnt_h, prod_num_terms.len()),
                ArrayArg::from_raw_parts(prb_h, prod_row_base.len()),
                ArrayArg::from_raw_parts(poo_h, prod_out_offset.len()),
                ArrayArg::from_raw_parts(pps_h, prod_pair_start.len()),
                width,
            );
        }

        let bytes = client.read_one(out_h).unwrap();
        let flat = u32::from_bytes(&bytes);
        let result: Vec<Vec<u32>> = (0..num_rows)
            .map(|r| flat[r * num_limbs..(r + 1) * num_limbs].to_vec())
            .collect();

        // `out_h` alone is `num_rows × num_limbs` u32 — hundreds of MB at record degrees.
        // It (and the small per-launch buffers, now dropped) varies in size launch to
        // launch, so CubeCL's pool cannot reuse the slab and would accumulate them until
        // the 4 GB card OOMs. Return the freed memory to the driver each launch; the
        // resident admissible handles stay alive (refcount > 0) so cleanup skips them.
        client.memory_cleanup();

        result
    });

    // Aggregate marshal/device totals across every launch (cheap, always on) so a whole
    // resolution's GPU overhead can be split host-vs-device via [`take_batch_stats`].
    let device_ms = t_device.elapsed().as_secs_f64() * 1e3;
    BATCH_CALLS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    BATCH_MARSHAL_US.fetch_add(
        (marshal_ms * 1e3) as u64,
        std::sync::atomic::Ordering::Relaxed,
    );
    BATCH_DEVICE_US.fetch_add(
        (device_ms * 1e3) as u64,
        std::sync::atomic::Ordering::Relaxed,
    );
    BATCH_PAIRS.fetch_add(total_pairs as u64, std::sync::atomic::Ordering::Relaxed);

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Smoke test proving the CubeCL `cuda` runtime launches and returns correct
    /// results. Requires a live GPU + the CUDA toolkit env (run under the `gpu`
    /// dev shell, unsandboxed).
    #[test]
    fn xor_f2_matches_host() {
        let a: Vec<u32> = (0..1000u32).map(|i| i.wrapping_mul(2654435761)).collect();
        let b: Vec<u32> = (0..1000u32).map(|i| i.wrapping_mul(40503)).collect();
        let expected: Vec<u32> = a.iter().zip(&b).map(|(x, y)| x ^ y).collect();
        assert_eq!(xor_f2_on_gpu(&a, &b), expected);
    }

    /// The device `seqno` must reproduce the CPU basis order exactly: for every
    /// basis element of every degree, `seqno(elt.p_part) == index`. Mirrors the CPU
    /// `seqno_matches_enumeration_order` test on-device. Requires a live GPU + the
    /// CUDA toolkit env (run under the `gpu` dev shell, unsandboxed).
    #[test]
    fn seqno_matches_index_on_gpu() {
        use fp::prime::ValidPrime;

        let p = ValidPrime::new(2);
        let algebra = MilnorAlgebra::new(p, false);
        let max_degree = 60;
        algebra.compute_basis(max_degree);
        algebra.compute_seqno_tables(max_degree);

        let (width, g) = algebra.seqno_table_u32();
        assert_eq!(width, MAX_XI_TAU);
        let xi: Vec<u32> = xi_degrees(p).iter().map(|&x| x as u32).collect();

        // Marshal every basis element, padded to `width`; the expected seqno is the
        // element's own index (the identity permutation the CPU proves).
        let mut p_parts = Vec::new();
        let mut expected = Vec::new();
        for d in 0..=max_degree {
            let dim = algebra.dimension(d);
            for i in 0..dim {
                let elt = algebra.basis_element_from_index(d, i);
                let mut row = vec![0u32; width];
                for (slot, &v) in row.iter_mut().zip(&elt.p_part) {
                    *slot = v;
                }
                p_parts.extend_from_slice(&row);
                expected.push(i as u32);
            }
        }

        let n = expected.len();
        let got = seqno_batch_on_gpu(width, &xi, &g, &p_parts, n);
        assert_eq!(got, expected, "device seqno diverged from CPU basis order");
    }

    /// The single-`R` multiply kernel must match the CPU reference
    /// `multiply_basis_element_by_element_2` bit-for-bit. For many `(R, s)` with `R`
    /// non-empty and `s` the dense (all-ones) element — exercising the admissible
    /// path and mod-2 cancellation — compare the GPU's packed F₂ output to the CPU's.
    /// Requires a live GPU + the CUDA toolkit env (run under the `gpu` dev shell).
    #[test]
    fn multiply_single_r_matches_reference() {
        use fp::{prime::ValidPrime, vector::FpVector};

        let p = ValidPrime::new(2);
        let algebra = MilnorAlgebra::new(p, false);
        let max_degree = 40;
        algebra.compute_basis(max_degree);
        algebra.compute_seqno_tables(max_degree);

        let mut checked = 0usize;
        for r_degree in 1..=12 {
            let r_dim = algebra.dimension(r_degree);
            for r_idx in 0..r_dim {
                if algebra
                    .basis_element_from_index(r_degree, r_idx)
                    .p_part
                    .is_empty()
                {
                    continue; // Sq(∅) = 1 is handled separately
                }
                for s_degree in 1..=(max_degree - r_degree) {
                    let s_dim = algebra.dimension(s_degree);
                    if s_dim == 0 {
                        continue;
                    }
                    let out_dim = algebra.dimension(r_degree + s_degree);

                    // s = dense (all basis elements): multi-term, mod-2 cancellation.
                    let mut s = FpVector::new(p, s_dim);
                    for j in 0..s_dim {
                        s.set_entry(j, 1);
                    }
                    let mut cpu = FpVector::new(p, out_dim);
                    algebra.multiply_basis_element_by_element_2(
                        cpu.as_slice_mut(),
                        1,
                        r_degree,
                        r_idx,
                        s_degree,
                        s.as_slice(),
                    );
                    let num_limbs = out_dim.div_ceil(32).max(1);
                    let mut golden = vec![0u32; num_limbs];
                    for (i, _) in cpu.iter_nonzero() {
                        golden[i / 32] ^= 1u32 << (i % 32);
                    }

                    let term_indices: Vec<usize> = (0..s_dim).collect();
                    let got = multiply_single_r_on_gpu(
                        &algebra,
                        r_degree,
                        r_idx,
                        s_degree,
                        &term_indices,
                    );
                    assert_eq!(
                        got, golden,
                        "GPU multiply diverged from reference: R(deg {r_degree}, idx {r_idx}) * \
                         dense s(deg {s_degree})",
                    );
                    checked += 1;
                }
            }
        }
        assert!(checked > 0, "no (R, s) cases exercised");
        eprintln!("multiply_single_r: {checked} (R, s) cases matched reference");
    }

    /// The batched kernel must reproduce a whole output matrix: many heterogeneous
    /// `(R, s)` products, several accumulating into the same row (XOR), computed in a
    /// single launch, must equal the CPU reference matrix built product-by-product.
    /// Requires a live GPU + the CUDA toolkit env (run under the `gpu` dev shell).
    #[test]
    fn multiply_batch_matches_reference() {
        use fp::{prime::ValidPrime, vector::FpVector};

        let p = ValidPrime::new(2);
        let algebra = MilnorAlgebra::new(p, false);
        let max_degree = 40;
        algebra.compute_basis(max_degree);
        algebra.compute_seqno_tables(max_degree);

        let out_degree = 24;
        let out_dim = algebra.dimension(out_degree);
        let num_rows = 8;

        // Products: every non-empty R of degree 1..out_degree, s dense at the
        // complementary degree, assigned round-robin to rows so rows accumulate.
        let mut products = Vec::new();
        for r_degree in 1..out_degree {
            let s_degree = out_degree - r_degree;
            let s_dim = algebra.dimension(s_degree);
            if s_dim == 0 {
                continue;
            }
            let r_dim = algebra.dimension(r_degree);
            for r_idx in 0..r_dim {
                if algebra
                    .basis_element_from_index(r_degree, r_idx)
                    .p_part
                    .is_empty()
                {
                    continue;
                }
                let row = products.len() % num_rows;
                products.push(GpuProduct {
                    r_degree,
                    r_idx,
                    s_degree,
                    term_indices: (0..s_dim).collect(),
                    row,
                    out_offset: 0,
                });
            }
        }

        // CPU golden matrix: accumulate each product into its row.
        let mut cpu_rows: Vec<FpVector> =
            (0..num_rows).map(|_| FpVector::new(p, out_dim)).collect();
        for prod in &products {
            let s_dim = algebra.dimension(prod.s_degree);
            let mut s = FpVector::new(p, s_dim);
            for &ti in &prod.term_indices {
                s.set_entry(ti, 1);
            }
            let mut tmp = FpVector::new(p, out_dim);
            algebra.multiply_basis_element_by_element_2(
                tmp.as_slice_mut(),
                1,
                prod.r_degree,
                prod.r_idx,
                prod.s_degree,
                s.as_slice(),
            );
            cpu_rows[prod.row].add(&tmp, 1);
        }
        let num_limbs = out_dim.div_ceil(32).max(1);
        let golden: Vec<Vec<u32>> = cpu_rows
            .iter()
            .map(|row| {
                let mut packed = vec![0u32; num_limbs];
                for (i, _) in row.iter_nonzero() {
                    packed[i / 32] ^= 1u32 << (i % 32);
                }
                packed
            })
            .collect();

        let got = multiply_batch_on_gpu(&algebra, out_dim, num_rows, &products);
        assert_eq!(
            got, golden,
            "batched GPU multiply diverged from reference matrix"
        );
        eprintln!(
            "multiply_batch: {} products across {num_rows} rows matched reference",
            products.len()
        );
    }
}
