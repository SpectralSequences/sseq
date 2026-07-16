// SPDX-License-Identifier: MIT OR Apache-2.0
//
// F_2 row reduction to reduced row echelon form, on device.
//
// A separate translation unit from the GEMM: the two share no device code, no constants and no
// types, and neither uses the other's tuning knobs. What they do share is the host, which drives
// the reduction's trailing update through the GEMM kernels in `matmul_b1.cu`.
//
// The reduction is a forward pass (panel factorization, then a right-looking promotion of the
// trailing region) followed by a back-substitution pass, matching the CPU blas3 structure in
// `fp/src/matrix/blas3.rs`. Several kernels come in two flavours: a plain multi-CTA form
// synchronized at kernel boundaries, which composes with other GPU work, and a `_coop` form using
// a grid barrier, which is faster but needs a co-resident grid. The composable form is the
// default; see EXPERIMENTS.md.

#include <cstdint>
#include <cuda_runtime.h>

// The GEMM half declares this too. Both are self-contained translation units, so each needs its
// own alias for the limb type; there is no shared header worth adding for one line.
typedef unsigned long long u64_t;

// ── Panel factorization kernel (BLAS3 GPU row-reduction port, design §5) ──────
//
// Forward panel factorization of ONE 64-bit column panel (limb `plimb`), the
// only column-indexed region of the reduction. A single CTA sweeps the 64 bit
// positions in order, with a __syncthreads between bits; the panel limb (m u64,
// a few MB) streams through L2. This is the b=64 base kernel of design §5(1):
// per bit, a find-first reduction picks the pivot (the lone column op), then a
// row-parallel masked XOR clears it from the rows *below* and records the
// multiplier bit into L. Forward-only (rows above pivots are left for the
// back-substitution pass), matching the CPU Step A in src/matrix/blas3.rs.
//
// Rows are addressed through the virtual permutation `perm` (design §4.3): a
// "row swap" swaps two perm entries; the matrix bytes never move. L is indexed
// by ORIGINAL row id (perm[p]), so it needs no swapping. Emitted to host: `pr`
// (pivots found) and `pivcols` (their absolute columns). L, the reduced panel,
// and perm stay on device.
//
// Launch with ONE block. THREADS threads (a power of two ≤ 1024).
extern "C" __global__ void panel_factor(
    u64_t* __restrict__ m_buf,     // m × stride limbs, in place
    unsigned* __restrict__ perm,   // length m, virtual row order
    u64_t* __restrict__ l_buf,     // m × l_stride limbs (multipliers), in place
    unsigned* __restrict__ pivcols,// out: absolute pivot columns, length ≤ 64
    unsigned* __restrict__ pr_out, // out: pivots found in this panel (1 int)
    unsigned plimb, unsigned r, unsigned n,
    unsigned m, unsigned stride, unsigned l_stride)
{
    extern __shared__ int s_red[]; // blockDim ints for the min-reduction
    __shared__ int s_pivpos;
    __shared__ unsigned s_pr;
    const int tid = threadIdx.x;
    const int nt = blockDim.x;
    if (tid == 0) s_pr = 0;
    __syncthreads();

    for (unsigned j = 0; j < 64; ++j) {
        unsigned q = plimb * 64 + j;
        if (q >= n) break;
        unsigned pr = s_pr;

        // find-first: smallest position p in [r+pr, m) whose row has bit j set.
        int local_min = 0x7fffffff;
        for (unsigned p = r + pr + tid; p < m; p += nt) {
            unsigned row = perm[p];
            if ((m_buf[(u64_t)row * stride + plimb] >> j) & 1ULL)
                local_min = min(local_min, (int)p);
        }
        s_red[tid] = local_min;
        __syncthreads();
        for (int off = nt / 2; off > 0; off >>= 1) {
            if (tid < off) s_red[tid] = min(s_red[tid], s_red[tid + off]);
            __syncthreads();
        }
        if (tid == 0) s_pivpos = s_red[0];
        __syncthreads();
        if (s_pivpos == 0x7fffffff) continue; // free column: no pivot

        // Promote: swap the pivot row up to position r+pr (perm swap only).
        if (tid == 0) {
            unsigned a = r + pr, b = (unsigned)s_pivpos;
            unsigned t = perm[a]; perm[a] = perm[b]; perm[b] = t;
            pivcols[pr] = q;
            s_pr = pr + 1;
        }
        __syncthreads();

        unsigned pivrow = perm[r + pr];
        u64_t pivword = m_buf[(u64_t)pivrow * stride + plimb];

        // Row-parallel masked XOR: clear bit j from the rows *below* the pivot,
        // recording the multiplier bit pr into L[row].
        for (unsigned p = r + pr + 1 + tid; p < m; p += nt) {
            unsigned row = perm[p];
            u64_t* cell = &m_buf[(u64_t)row * stride + plimb];
            if ((*cell >> j) & 1ULL) {
                l_buf[(u64_t)row * l_stride + (pr >> 6)] |= (1ULL << (pr & 63));
                *cell ^= pivword;
            }
        }
        __syncthreads();
    }
    if (tid == 0) *pr_out = s_pr;
}
