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

// ── Multi-CTA (cooperative) panel factorization ──────────────────────────────
//
// The single-CTA `panel_factor` above uses one SM of ~132 and is the dominant
// cost of the forward pass (profiled ~76% of GPU time at n=2^15 half-rank). This
// version does the identical math but spreads each bit-step's find-first and
// masked-XOR across the *whole grid*: the launch is cooperative (all CTAs
// co-resident), so we can barrier the grid between the 64 sequential bit-steps.
//
// Grid barrier is a self-contained sense-counting spin (no cooperative_groups /
// cudadevrt dependency, so it compiles under `nvcc -ptx`): each CTA's leader
// thread __threadfence()s its global writes, atomically arrives at a shared
// counter, and spins until all `total_ctas` CTAs of the current round have
// arrived. `goal` (= round · total_ctas) is tracked in a register that every
// thread advances identically — control flow is grid-uniform (all CTAs branch
// on the same broadcast `g_min`), so the arrival counts always match. Requires
// co-residency, which the cooperative launch guarantees; `barrier` must be 0 at
// launch.
__device__ __forceinline__ void grid_sync(unsigned* barrier, unsigned goal) {
    __syncthreads();
    __threadfence();
    if (threadIdx.x == 0) {
        atomicAdd(barrier, 1u);
        while (atomicAdd(barrier, 0u) < goal) { /* spin until the grid arrives */ }
    }
    __syncthreads();
}

// Same contract as `panel_factor` (factor one 64-bit panel `plimb` in place,
// forward-only from pivot row `r`, capturing multipliers into `l_buf`), but
// grid-parallel. `scratch` is 3 u32: [0]=barrier (must be 0), [1]=g_min (pivot
// position, reinterpreted as int), [2]=g_pr (pivots so far). Launch cooperatively
// with `total_ctas` = gridDim.x.
extern "C" __global__ void panel_factor_coop(
    u64_t* __restrict__ m_buf,
    unsigned* __restrict__ perm,
    u64_t* __restrict__ l_buf,
    unsigned* __restrict__ pivcols,
    unsigned* __restrict__ pr_out,
    unsigned* __restrict__ scratch,   // [barrier, g_min(int), g_pr]
    u64_t* __restrict__ g_pivword,    // broadcast pivot panel word (1 u64)
    unsigned plimb, unsigned r, unsigned n,
    unsigned m, unsigned stride, unsigned l_stride,
    unsigned total_ctas)
{
    extern __shared__ int s_red[]; // blockDim ints for the CTA-local min-reduction
    const int tid = threadIdx.x;
    const int nt = blockDim.x;
    const unsigned gtid = blockIdx.x * blockDim.x + threadIdx.x;
    const unsigned gnt = gridDim.x * blockDim.x;

    unsigned* barrier = &scratch[0];
    int* g_min = (int*)&scratch[1];
    unsigned* g_pr = &scratch[2];

    if (gtid == 0) { *g_min = 0x7fffffff; *g_pr = 0; }
    unsigned goal = 0;
    goal += total_ctas; grid_sync(barrier, goal); // init visible grid-wide

    for (unsigned j = 0; j < 64; ++j) {
        unsigned q = plimb * 64 + j;
        if (q >= n) break;
        unsigned pr = *g_pr;

        // find-first: smallest position p in [r+pr, m) whose row has bit j set.
        int local_min = 0x7fffffff;
        for (unsigned p = r + pr + gtid; p < m; p += gnt) {
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
        if (tid == 0) atomicMin(g_min, s_red[0]);
        goal += total_ctas; grid_sync(barrier, goal); // [A] all atomicMin done

        int pivpos = *g_min;
        if (pivpos != 0x7fffffff) {
            // Only thread 0 touches perm[pivpos]/g_min here: it reads the pivot
            // row's panel word (broadcast via g_pivword), swaps the pivot up to
            // position r+pr (perm swap only), and resets g_min/advances g_pr.
            // The displaced row lands at position pivpos and is handled by the
            // XOR loop below (which, after [B], reads a now-stable perm and never
            // touches perm[pivpos] concurrently with the swap).
            if (gtid == 0) {
                unsigned pivrow = perm[pivpos];
                *g_pivword = m_buf[(u64_t)pivrow * stride + plimb];
                unsigned a = r + pr;
                perm[pivpos] = perm[a]; perm[a] = pivrow;
                pivcols[pr] = q;
                *g_min = 0x7fffffff; // reset for next bit
                *g_pr = pr + 1;
            }
            goal += total_ctas; grid_sync(barrier, goal); // [B] swap + pivword + resets visible

            u64_t pivword = *g_pivword;
            // masked XOR of the rows *below* the pivot (now at position r+pr).
            for (unsigned p = r + pr + 1 + gtid; p < m; p += gnt) {
                unsigned row = perm[p];
                u64_t* cell = &m_buf[(u64_t)row * stride + plimb];
                if ((*cell >> j) & 1ULL) {
                    l_buf[(u64_t)row * l_stride + (pr >> 6)] |= (1ULL << (pr & 63));
                    *cell ^= pivword;
                }
            }
            goal += total_ctas; grid_sync(barrier, goal); // [C] XOR done before next find
        }
        // free column (pivpos == INT_MAX): g_min already INT_MAX, g_pr unchanged;
        // no extra barriers — the branch is grid-uniform so all CTAs agree.
    }
    if (gtid == 0) *pr_out = *g_pr;
}

// ── Forward-pass driver kernels (BLAS3 GPU row-reduction port, design §4.4) ───
//
// After panel_factor establishes `pr` pivots at perm positions [r, r+pr), the
// driver (1) promotes the pivot rows' trailing, (2) drops them from the
// multiplier matrix, (3) gathers them into a contiguous U for the trailing GEMM.

// (1) Promote pivot-row trailings: realize the deferred trailing of each pivot
// by replaying the earlier this-panel pivots recorded in L. Sequential in k
// (pivot k uses the already-promoted pivots i<k), but **embarrassingly parallel
// over trailing limbs** — each column c runs its own full k-loop and never
// touches another column, so no cross-thread ordering is needed. Grid-strided
// over columns: launch as many CTAs as fill the machine (the old version used a
// single CTA and was ~18% of GPU time). Only pivot rows' columns are written,
// each by exactly one thread, so there are no races and no __syncthreads.
extern "C" __global__ void promote_pivots(
    u64_t* __restrict__ m_buf, const unsigned* __restrict__ perm,
    const u64_t* __restrict__ l_buf,
    unsigned r, unsigned pr, unsigned first_limb, unsigned trailing_limbs,
    unsigned stride, unsigned l_stride)
{
    const unsigned gtid = blockIdx.x * blockDim.x + threadIdx.x;
    const unsigned gnt = gridDim.x * blockDim.x;
    for (unsigned c = gtid; c < trailing_limbs; c += gnt) {
        // Ascending k: when pivot k reads pivot i<k at column c, that value was
        // already written by this same thread at its earlier k=i step.
        for (unsigned k = 0; k < pr; ++k) {
            unsigned row_k = perm[r + k];
            u64_t acc = m_buf[(u64_t)row_k * stride + first_limb + c];
            for (unsigned i = 0; i < k; ++i) {
                if ((l_buf[(u64_t)row_k * l_stride + (i >> 6)] >> (i & 63)) & 1ULL)
                    acc ^= m_buf[(u64_t)perm[r + i] * stride + first_limb + c];
            }
            m_buf[(u64_t)row_k * stride + first_limb + c] = acc;
        }
    }
}

// (2) Zero the L rows of the pr pivot rows so the trailing GEMM (which runs over
// all m rows) leaves them untouched — their trailing is already promoted.
extern "C" __global__ void zero_pivot_l(
    const unsigned* __restrict__ perm, u64_t* __restrict__ l_buf,
    unsigned r, unsigned pr, unsigned l_stride)
{
    unsigned idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= pr) return;
    unsigned row = perm[r + idx];
    for (unsigned c = 0; c < l_stride; ++c)
        l_buf[(u64_t)row * l_stride + c] = 0;
}

// (3) Gather the pr pivot rows' trailing limbs [first_limb, first_limb+ncols)
// (through perm) into a contiguous pr × ncols buffer — the GEMM operand U.
extern "C" __global__ void gather_rows(
    u64_t* __restrict__ dst, const u64_t* __restrict__ m_buf,
    const unsigned* __restrict__ perm,
    unsigned r, unsigned first_limb, unsigned pr, unsigned ncols, unsigned stride)
{
    unsigned idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= pr * ncols) return;
    unsigned k = idx / ncols, c = idx % ncols;
    dst[idx] = m_buf[(u64_t)perm[r + k] * stride + first_limb + c];
}

// ── Back-substitution kernels (BLAS3 GPU row-reduction port, design §4.6) ─────
//
// Echelon → RREF, blocked right-to-left over pivot blocks. For a block of pivots
// at perm positions [s, e): (1) reduce the block among itself, then (2) clear
// the block's pivot columns from all rows above [0, s) via one X·U GEMM.

// (1) Reduce the pivot block [s, e) to RREF among itself: process pivots
// high-to-low, clearing pivot column pivcols[k] from the earlier block rows
// [s, k). One CTA; threads parallelize over limbs. Sequential in k (a row used
// as a source must already be fully reduced) — a __syncthreads separates the k
// steps, and one inside the j-loop orders every row's condition-read before any
// write to that row (the pivot bit itself is cleared by the XOR).
extern "C" __global__ void block_reduce_rref(
    u64_t* __restrict__ m_buf, const unsigned* __restrict__ perm,
    const unsigned* __restrict__ pivcols,
    unsigned s, unsigned e, unsigned stride)
{
    // The block has ≤64 pivot rows. For pivot k (processed high-to-low) we clear
    // its pivot column from every earlier block row j∈[s,k) that has the bit set,
    // XORing rowk into rowj across the full row width. The condition for every
    // such j is read at once into shared memory *before* any XOR (rowj[qlimb] is
    // itself cleared by the XOR), then the (j, limb) work is flattened across all
    // threads — two __syncthreads per pivot k instead of the previous ~bp² (one
    // per (k,j) pair). Still one CTA; parallel over (j × limb).
    __shared__ unsigned char cond[64]; // block size ≤ 64
    const int tid = threadIdx.x;
    const int nt = blockDim.x;
    for (unsigned k = e; k-- > s;) {
        unsigned qk = pivcols[k];
        unsigned qlimb = qk >> 6, qbit = qk & 63;
        unsigned rowk = perm[k];
        unsigned nj = k - s; // earlier block rows [s, k)

        // Gather the pivot-k bit of every earlier block row (pre-XOR).
        for (unsigned j = tid; j < nj; j += nt)
            cond[j] = (unsigned char)((m_buf[(u64_t)perm[s + j] * stride + qlimb] >> qbit) & 1ULL);
        __syncthreads();

        // Limb-parallel: each thread owns a set of columns c, loads rowk[c] once
        // and XORs it into every flagged rowj at that column. No 64-bit division;
        // rowk[c] reused across all ≤64 rows. Distinct c per thread ⇒ no races.
        for (unsigned c = tid; c < stride; c += nt) {
            u64_t rowk_c = m_buf[(u64_t)rowk * stride + c];
            for (unsigned j = 0; j < nj; ++j)
                if (cond[j])
                    m_buf[(u64_t)perm[s + j] * stride + c] ^= rowk_c;
        }
        __syncthreads(); // finish this k before the next reads the bits again
    }
}

// (2a) Gather X: for rows at perm positions [0, s), the bits at the `count`
// block pivot columns pivcols[col_start .. col_start+count). One thread per
// (row, dst-limb) builds a full limb, so no atomics. dst is s × dst_stride.
extern "C" __global__ void gather_cols(
    u64_t* __restrict__ dst, const u64_t* __restrict__ m_buf,
    const unsigned* __restrict__ perm, const unsigned* __restrict__ pivcols,
    unsigned col_start, unsigned s, unsigned count,
    unsigned stride, unsigned dst_stride)
{
    unsigned idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= s * dst_stride) return;
    unsigned jpos = idx / dst_stride, dl = idx % dst_stride;
    unsigned row = perm[jpos];
    u64_t val = 0;
    for (unsigned bb = 0; bb < 64; ++bb) {
        unsigned i = dl * 64 + bb;
        if (i >= count) break;
        unsigned q = pivcols[col_start + i];
        if ((m_buf[(u64_t)row * stride + (q >> 6)] >> (q & 63)) & 1ULL)
            val |= (1ULL << bb);
    }
    dst[idx] = val;
}

// (2b) Scatter-XOR the GEMM result C (s × c_stride) into rows at perm positions
// [0, s): M[perm[jpos]][first_limb + col] ^= C[jpos][col].
extern "C" __global__ void xor_into_perm(
    u64_t* __restrict__ m_buf, const u64_t* __restrict__ c,
    const unsigned* __restrict__ perm,
    unsigned s, unsigned width, unsigned stride, unsigned first_limb,
    unsigned c_stride)
{
    unsigned idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= s * width) return;
    unsigned jpos = idx / width, col = idx % width;
    m_buf[(u64_t)perm[jpos] * stride + first_limb + col] ^= c[(u64_t)jpos * c_stride + col];
}
