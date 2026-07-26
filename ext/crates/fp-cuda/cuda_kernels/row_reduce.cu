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
        atomicAdd(barrier, 1u); // arrive (one RMW per CTA)
        // Spin on a *plain* volatile load, not atomicAdd(...,0): a read-modify-
        // write acquires the cache line exclusively every iteration, serializing
        // ~1000 spinning CTAs on one line and making each barrier cost ~µs. A
        // volatile load leaves the line shared, so the spin is nearly free.
        while (*((volatile unsigned*)barrier) < goal) { /* wait for the grid */ }
    }
    __syncthreads();
}

// Wide-panel factorization (design §5(2) / CPU blas3.rs Step A): factor the b=bl·64
// column panel at limbs [ppanel, ppanel+bl) in place, forward-only from pivot row
// r, grid-parallel across all bl·64 columns. Unlike a single-limb panel, the
// masked XOR clears each pivot from the below rows across **all bl panel limbs**
// (the intra-panel Schur update, done inline), so a later sub-column's find-first
// sees fully reduced bits and one wide trailing GEMM (K = pr ≤ b) fixes up the
// columns beyond the panel. Multipliers go into a bl-limb-wide L (indexed by pr).
//
// `scratch` is 3 u32: [0]=barrier (must be 0), [1]=g_min (int), [2]=g_pr.
// `g_pivword` is bl u64 (the pivot row's panel limbs, broadcast). Launch
// cooperatively with `total_ctas` = gridDim.x. bl==1 reproduces the single-limb
// kernel exactly.
extern "C" __global__ void panel_factor_coop(
    u64_t* __restrict__ m_buf,
    unsigned* __restrict__ perm,
    u64_t* __restrict__ l_buf,
    unsigned* __restrict__ pivcols,
    unsigned* __restrict__ pr_out,
    unsigned* __restrict__ scratch,   // [barrier, g_min(int), g_pr]
    u64_t* __restrict__ g_pivword,    // broadcast pivot panel limbs (bl u64)
    unsigned ppanel, unsigned bl, unsigned r, unsigned n,
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

    for (unsigned cc = 0; cc < bl * 64; ++cc) {
        unsigned q = ppanel * 64 + cc;
        if (q >= n) break;
        unsigned plimb = ppanel + cc / 64; // absolute limb of column q
        unsigned j = cc & 63;              // bit within that limb
        unsigned pr = *g_pr;

        // find-first: smallest position p in [r+pr, m) whose row has bit q set.
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
            // Thread 0 reads the pivot row's bl panel limbs (broadcast via
            // g_pivword), swaps the pivot up to position r+pr, resets g_min and
            // advances g_pr. The displaced row lands at pivpos and is handled by
            // the XOR loop (which reads a now-stable perm after [B]).
            if (gtid == 0) {
                unsigned pivrow = perm[pivpos];
                for (unsigned t = 0; t < bl; ++t)
                    g_pivword[t] = m_buf[(u64_t)pivrow * stride + ppanel + t];
                unsigned a = r + pr;
                perm[pivpos] = perm[a]; perm[a] = pivrow;
                pivcols[pr] = q;
                *g_min = 0x7fffffff; // reset for next column
                *g_pr = pr + 1;
            }
            goal += total_ctas; grid_sync(barrier, goal); // [B] swap + pivword + resets visible

            // masked XOR of the rows *below* the pivot, across ALL bl panel limbs.
            for (unsigned p = r + pr + 1 + gtid; p < m; p += gnt) {
                unsigned row = perm[p];
                u64_t* base = &m_buf[(u64_t)row * stride + ppanel];
                if ((base[cc / 64] >> j) & 1ULL) {
                    l_buf[(u64_t)row * l_stride + (pr >> 6)] |= (1ULL << (pr & 63));
                    for (unsigned t = 0; t < bl; ++t)
                        base[t] ^= g_pivword[t];
                }
            }
            goal += total_ctas; grid_sync(barrier, goal); // [C] XOR done before next find
        }
        // free column (pivpos == INT_MAX): grid-uniform, no extra barriers.
    }
    if (gtid == 0) *pr_out = *g_pr;
}

// ── Streamed (kernel-boundary) panel factorization ───────────────────────────
//
// Same all-SM parallelism as panel_factor_coop, but WITHOUT a cooperative launch:
// each of the ≤ bl·64 sequential bit-steps is three ordinary grid-wide kernels
// (pf_find → pf_swap → pf_xor), and the *kernel boundary* — stream ordering —
// replaces the in-grid `grid_sync`. This is how cuSOLVER/cuBLAS build grid-wide
// multi-step algorithms: no all-CTAs-co-resident requirement, so it composes with
// a concurrent kernel from another CUDA runtime (cubecl's Milnor multiply) instead
// of deadlocking its grid barrier (the intermittent stem-150 wedge).
//
// All state stays on the device — g_pr (pivots so far), g_min (find-first result),
// g_pivpos (this step's pivot position, for pf_xor's guard), g_pivword (the pivot
// row's bl panel limbs). The host never reads back inside the loop, so it races
// ahead queuing launches and their latency hides behind the GPU work. Bit-for-bit
// identical to panel_factor_coop; g_min must be INF and g_pr 0 at entry.

// find-first + swap, fused into one launch. Every CTA reduces its row slice and
// atomicMin's into g_min; then a threadfence "last-CTA finalize" (the CTA whose
// leader increments the arrival counter last) reads the grid-wide minimum and does
// the swap. This is a grid-wide *reduction*, not a barrier — the last CTA to run
// finalizes, so it needs NO co-residency (unlike a spin barrier) and cannot
// deadlock against concurrent GPU work; `arrival` self-resets to 0 via atomicInc's
// wrap at gridDim-1. On a pivot: read its bl panel limbs into g_pivword, swap it up
// to r+g_pr (perm swap), record the column, bump g_pr, reset g_min for the next
// step. Publishes the pivot position (or INF) to g_pivpos so pf_xor knows whether
// to run. `arrival` and g_min must be 0 / INF at the first step. q ≥ n ⇒ no-op
// (lets a fixed-length step sequence cover a short final panel).
extern "C" __global__ void pf_find_swap(
    u64_t* __restrict__ m_buf,
    unsigned* __restrict__ perm,
    unsigned* __restrict__ pivcols,
    u64_t* __restrict__ g_pivword,
    int* __restrict__ g_min,
    unsigned* __restrict__ g_pr,
    int* __restrict__ g_pivpos,
    unsigned* __restrict__ arrival,
    unsigned ppanel, unsigned bl, unsigned j, unsigned plimb, unsigned cc,
    unsigned r, unsigned m, unsigned stride, unsigned n)
{
    extern __shared__ int s_red[];
    const int tid = threadIdx.x;
    const int nt = blockDim.x;
    const unsigned gtid = blockIdx.x * blockDim.x + threadIdx.x;
    const unsigned gnt = gridDim.x * blockDim.x;
    const unsigned q = ppanel * 64 + cc;
    const unsigned pr = *g_pr;

    int local_min = 0x7fffffff;
    if (q < n) {
        for (unsigned p = r + pr + gtid; p < m; p += gnt) {
            unsigned row = perm[p];
            if ((m_buf[(u64_t)row * stride + plimb] >> j) & 1ULL)
                local_min = min(local_min, (int)p);
        }
    }
    s_red[tid] = local_min;
    __syncthreads();
    for (int off = nt / 2; off > 0; off >>= 1) {
        if (tid < off) s_red[tid] = min(s_red[tid], s_red[tid + off]);
        __syncthreads();
    }
    if (tid == 0) atomicMin(g_min, s_red[0]);
    __threadfence();

    __shared__ bool am_last;
    if (tid == 0) am_last = (atomicInc(arrival, gridDim.x - 1) == gridDim.x - 1);
    __syncthreads();
    if (!am_last || tid != 0) return;

    int pivpos = *g_min;
    *g_pivpos = pivpos;
    if (pivpos == 0x7fffffff) return; // free column: g_min stays INF for next step
    unsigned pivrow = perm[pivpos];
    for (unsigned t = 0; t < bl; ++t)
        g_pivword[t] = m_buf[(u64_t)pivrow * stride + ppanel + t];
    unsigned a = r + pr;
    perm[pivpos] = perm[a];
    perm[a] = pivrow;
    pivcols[pr] = q;
    *g_min = 0x7fffffff; // reset for the next column
    *g_pr = pr + 1;
}

// masked XOR of the pivot row into the rows *below* it, across all bl panel limbs,
// recording the multiplier bit into L. No-op on a free column (g_pivpos == INF).
// g_pr has already been bumped by pf_swap, so this pivot's index is *g_pr - 1.
extern "C" __global__ void pf_xor(
    u64_t* __restrict__ m_buf,
    const unsigned* __restrict__ perm,
    u64_t* __restrict__ l_buf,
    const u64_t* __restrict__ g_pivword,
    const int* __restrict__ g_pivpos,
    const unsigned* __restrict__ g_pr,
    unsigned ppanel, unsigned bl, unsigned cc, unsigned j,
    unsigned r, unsigned m, unsigned stride, unsigned l_stride)
{
    if (*g_pivpos == 0x7fffffff) return; // free column: nothing to clear
    const unsigned pr = *g_pr - 1;       // index of the pivot just placed
    const unsigned gtid = blockIdx.x * blockDim.x + threadIdx.x;
    const unsigned gnt = gridDim.x * blockDim.x;
    for (unsigned p = r + pr + 1 + gtid; p < m; p += gnt) {
        unsigned row = perm[p];
        u64_t* base = &m_buf[(u64_t)row * stride + ppanel];
        if ((base[cc / 64] >> j) & 1ULL) {
            l_buf[(u64_t)row * l_stride + (pr >> 6)] |= (1ULL << (pr & 63));
            for (unsigned t = 0; t < bl; ++t)
                base[t] ^= g_pivword[t];
        }
    }
}

// ── Active-row compaction (design §8.2) ──────────────────────────────────────
//
// A below row that is entirely zero across the remaining columns [start_limb,
// stride) can never become a pivot and never contributes to a trailing update
// (its multiplier bits are always zero), so it is permanently dead. At half rank
// ~half the rows die during elimination. Marking them lets the driver shrink the
// row range panel_factor scans. `live[idx]` (idx over rows [r, m)) = 1 iff row
// perm[r+idx] has any set bit in limbs [start_limb, stride).
extern "C" __global__ void mark_live(
    unsigned* __restrict__ live, const u64_t* __restrict__ m_buf,
    const unsigned* __restrict__ perm,
    unsigned r, unsigned m, unsigned start_limb, unsigned stride)
{
    unsigned idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= m - r) return;
    unsigned row = perm[r + idx];
    u64_t acc = 0;
    for (unsigned c = start_limb; c < stride; ++c)
        acc |= m_buf[(u64_t)row * stride + c];
    live[idx] = (acc != 0) ? 1u : 0u;
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

// Cooperative multi-CTA promotion (right-looking): identical result to
// promote_pivots but grid-parallel. Processing pivots i = 0..pr in order, once
// pivot i's trailing is final we add it to every later pivot k>i that carries its
// multiplier (L[perm[r+k]][i]) — M[k] ^= M[i] across all trailing limbs — so by
// the time we reach pivot i it is already fully promoted. Sequential in i (grid
// barrier between steps), but each step's XOR is flattened over (later-pivot ×
// limb) across the whole grid, replacing the single-CTA kernel's ~pr² per-column
// serial replay. `barrier` must be 0 at launch; `cond` holds ≥ pr unsigned.
// `l_limb_off` offsets the L read so this can promote a sub-block [lo, lo+pr):
// the caller passes r = r0+lo and l_limb_off = lo/64 (lo is 64-aligned), so bit i
// here maps to global multiplier bit lo+i. Zero for a full-panel promote.
extern "C" __global__ void promote_coop(
    u64_t* __restrict__ m_buf, const unsigned* __restrict__ perm,
    const u64_t* __restrict__ l_buf,
    unsigned r, unsigned pr, unsigned first_limb, unsigned trailing_limbs,
    unsigned stride, unsigned l_stride, unsigned l_limb_off,
    unsigned* __restrict__ barrier, unsigned* __restrict__ cond,
    unsigned total_ctas)
{
    const unsigned gtid = blockIdx.x * blockDim.x + threadIdx.x;
    const unsigned gnt = gridDim.x * blockDim.x;
    (void)cond; // multiplier bits are read straight from L (never mutated here)
    unsigned goal = 0;
    for (unsigned i = 0; i < pr; ++i) {
        unsigned rowi = perm[r + i];
        unsigned nk = pr - i - 1; // later pivots k in (i, pr)
        // The clear-condition reads L, which the XOR never writes, so we test it
        // inline instead of gathering it under a separate barrier — one grid
        // barrier per pivot instead of two (these kernels are barrier-bound).
        unsigned total = nk * trailing_limbs; // fits u32
        for (unsigned idx = gtid; idx < total; idx += gnt) {
            unsigned krel = idx / trailing_limbs;
            unsigned rowk = perm[r + i + 1 + krel];
            if (!((l_buf[(u64_t)rowk * l_stride + l_limb_off + (i >> 6)] >> (i & 63)) & 1ULL))
                continue;
            unsigned c = idx - krel * trailing_limbs;
            m_buf[(u64_t)rowk * stride + first_limb + c] ^=
                m_buf[(u64_t)rowi * stride + first_limb + c];
        }
        goal += total_ctas; grid_sync(barrier, goal); // finish i before next reads M[i+1]
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

// Cooperative multi-CTA version of block_reduce_rref: identical math, but the
// per-pivot clear is spread across the whole grid instead of one SM (the
// single-CTA kernel was ~22% of GPU time at n=2^16). Same structure — gather the
// clear-conditions for pivot k, barrier, XOR rowk into the flagged block rows
// across all limbs, barrier — but with a global `cond` buffer and the atomic grid
// barrier (grid-uniform k-loop, so arrival counts always match). The (j, limb)
// work is flattened over the grid for full-machine parallelism. `barrier` must be
// 0 at launch; `cond` holds ≥ (e-s) unsigned. Launch cooperatively.
extern "C" __global__ void block_reduce_coop(
    u64_t* __restrict__ m_buf, const unsigned* __restrict__ perm,
    const unsigned* __restrict__ pivcols,
    unsigned s, unsigned e, unsigned stride,
    unsigned* __restrict__ barrier, unsigned* __restrict__ cond,
    unsigned total_ctas)
{
    const unsigned gtid = blockIdx.x * blockDim.x + threadIdx.x;
    const unsigned gnt = gridDim.x * blockDim.x;
    unsigned goal = 0;
    for (unsigned k = e; k-- > s;) {
        unsigned qk = pivcols[k];
        unsigned qlimb = qk >> 6, qbit = qk & 63;
        unsigned rowk = perm[k];
        unsigned nj = k - s; // earlier block rows [s, k)

        for (unsigned j = gtid; j < nj; j += gnt)
            cond[j] = (unsigned)((m_buf[(u64_t)perm[s + j] * stride + qlimb] >> qbit) & 1ULL);
        goal += total_ctas; grid_sync(barrier, goal); // [A] conds visible pre-XOR

        // XOR rowk into each flagged rowj, flattened over (j, limb) across the grid.
        unsigned total = nj * stride; // ≤ 64 · stride, fits u32
        for (unsigned idx = gtid; idx < total; idx += gnt) {
            unsigned j = idx / stride;
            if (!cond[j]) continue;
            unsigned c = idx - j * stride;
            m_buf[(u64_t)perm[s + j] * stride + c] ^= m_buf[(u64_t)rowk * stride + c];
        }
        goal += total_ctas; grid_sync(barrier, goal); // [B] finish k before next reads
    }
}

// ── Streamed (kernel-boundary) block reduction ───────────────────────────────
//
// Non-cooperative equivalent of block_reduce_coop: the same grid-wide per-pivot
// clear, but each of block_reduce_coop's two grid_syncs becomes a kernel boundary
// (br_cond → br_xor per pivot k, high-to-low). No cooperative launch, so it
// composes with concurrent GPU work. `cond` holds ≥ (e-s) unsigned; both are
// launched per pivot with the same block-relative index k. Bit-identical to
// block_reduce_coop / block_reduce_rref.

// Gather the pivot-k bit of every earlier block row j ∈ [s, k) into cond[j-s],
// *before* any XOR clears it. Grid-strided over the ≤64 earlier rows.
extern "C" __global__ void br_cond(
    const u64_t* __restrict__ m_buf, const unsigned* __restrict__ perm,
    const unsigned* __restrict__ pivcols,
    unsigned s, unsigned k, unsigned stride, unsigned* __restrict__ cond)
{
    unsigned qk = pivcols[k];
    unsigned qlimb = qk >> 6, qbit = qk & 63;
    unsigned nj = k - s;
    const unsigned gtid = blockIdx.x * blockDim.x + threadIdx.x;
    const unsigned gnt = gridDim.x * blockDim.x;
    for (unsigned j = gtid; j < nj; j += gnt)
        cond[j] = (unsigned)((m_buf[(u64_t)perm[s + j] * stride + qlimb] >> qbit) & 1ULL);
}

// XOR row k into every flagged earlier block row across all limbs, flattened over
// (j, limb) across the grid.
extern "C" __global__ void br_xor(
    u64_t* __restrict__ m_buf, const unsigned* __restrict__ perm,
    unsigned s, unsigned k, unsigned stride, const unsigned* __restrict__ cond)
{
    unsigned rowk = perm[k];
    unsigned nj = k - s;
    unsigned total = nj * stride;
    const unsigned gtid = blockIdx.x * blockDim.x + threadIdx.x;
    const unsigned gnt = gridDim.x * blockDim.x;
    for (unsigned idx = gtid; idx < total; idx += gnt) {
        unsigned j = idx / stride;
        if (!cond[j]) continue;
        unsigned c = idx - j * stride;
        m_buf[(u64_t)perm[s + j] * stride + c] ^= m_buf[(u64_t)rowk * stride + c];
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
