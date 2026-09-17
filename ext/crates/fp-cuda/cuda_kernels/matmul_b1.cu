// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Hopper wgmma.b1 F_2 GEMM kernel — 128B-swizzle operands, pipelined wgmmas, a register-blocked
// output tile, and a persistent grid with grouped tile rasterization, which is what keeps B
// L2-resident at large N.
//
// Both operands are K-major. They are pre-arranged on the host as plain row-major tiles and loaded
// via TMA cp.async.bulk.tensor.2d with CU_TENSOR_MAP_SWIZZLE_128B: the TMA hardware applies the
// 128B swizzle on the way into SMEM, landing the data exactly where the swizzled wgmma matrix
// descriptor expects it — so the host emits the natural layout and there is no hand-rolled
// interleave.
//
// Each loaded tile spans a full 128B K-major swizzle atom (8 rows × TK bits), i.e. KSUB consecutive
// k256 sub-chunks. A CTA computes a register-blocked TM×NB output block (MSTRIPS m64 row-strips ×
// NB columns). Each k256 step loads B once and issues MSTRIPS m64n128k256 wgmmas that all reuse it
// — one L2→SMEM read of B feeds every strip, which cuts the operand-refill bytes per MAC (the
// measured bottleneck on Hopper: the tensor core out-runs L2→SMEM bandwidth). Each strip
// accumulates into its own resident ACC_N-register accumulator (scale-D = 1, popcounts summed
// in-hardware), all live across the whole K loop.
//
// The grid is persistent: ~SM-count CTAs sweep the output tile grid in a grouped-along-M rasterized
// order so each B-panel's reuse distance stays short (L2-resident). The pipeline barriers are
// initialized once and flow continuously across tiles.
//
// The CTAs are independent: no thread-block cluster, no TMA multicast, so the grid carries no
// placement constraint. Clusters were tried and removed; see EXPERIMENTS.md.

#include <cstdint>
#include <cuda_runtime.h>
#include <cuda.h>

#include "params.h"

// ── Helpers ─────────────────────────────────────────────────────────────────

// Build a wgmma SMEM matrix descriptor.
//   p     : SMEM address of the operand sub-tile (already swizzled by TMA).
//   lead  : leading-dimension byte offset (LBO), per CUTLASS make_gmma_desc.
//   stride: stride-dimension byte offset (SBO).
//   swiz  : layout_type — 0 = none, 1 = 128B, 2 = 64B, 3 = 32B.
// Byte offsets are stored with their low 4 bits dropped (uint128 units).
__device__ __forceinline__ uint64_t make_desc(
    const void* p, uint32_t lead, uint32_t stride, uint32_t swiz) {
    uint32_t a = (uint32_t)__cvta_generic_to_shared(p);
    uint64_t d = 0;
    d |= ((uint64_t)a >> 4) & 0x3FFFULL;
    d |= ((uint64_t)(lead   >> 4) & 0x3FFFULL) << 16;
    d |= ((uint64_t)(stride >> 4) & 0x3FFFULL) << 32;
    d |= ((uint64_t)(swiz & 0x3)) << 62;
    return d;
}

// Initialize the mbarrier `b` to expect `cnt` arrivals per phase.
__device__ __forceinline__ void mbar_init(uint64_t* b, uint32_t cnt) {
    asm volatile("mbarrier.init.shared::cta.b64 [%0], %1;\n"
        :: "r"((uint32_t)__cvta_generic_to_shared(b)), "r"(cnt));
}
// Arrive on `b` and declare that `bytes` bytes of async traffic will complete against it.
__device__ __forceinline__ void mbar_tx(uint64_t* b, uint32_t bytes) {
    asm volatile("mbarrier.arrive.expect_tx.shared::cta.b64 _, [%0], %1;\n"
        :: "r"((uint32_t)__cvta_generic_to_shared(b)), "r"(bytes) : "memory");
}
// Spin until `b` flips to `phase`. try_wait can wake spuriously, hence the retry loop.
__device__ __forceinline__ void mbar_wait(uint64_t* b, uint32_t phase) {
    uint32_t a = (uint32_t)__cvta_generic_to_shared(b);
    asm volatile(
        "{ .reg .pred p;\n"
        "  L: mbarrier.try_wait.parity.shared::cta.b64 p, [%0], %1;\n"
        "  @!p bra L;\n"
        "}\n" :: "r"(a), "r"(phase) : "memory");
}
// TMA load (global → SMEM) of the tile at tensor coords (x, y) into `dst`, completing against the
// mbarrier `b`. Issued by a single thread. `.shared::cluster` is the PTX name of the state space
// the bulk-copy family addresses; it does not require a cluster launch.
__device__ __forceinline__ void tma_2d(
    void* dst, const CUtensorMap* tm, int x, int y, uint64_t* b) {
    asm volatile(
        "cp.async.bulk.tensor.2d.shared::cluster.global.mbarrier::complete_tx::bytes"
        " [%0], [%1, {%2,%3}], [%4];\n"
        :: "r"((uint32_t)__cvta_generic_to_shared(dst)),
           "l"((uint64_t)tm), "r"(x), "r"(y),
           "r"((uint32_t)__cvta_generic_to_shared(b))
        : "memory");
}

// Arrive (count 1) on a local mbarrier. The only CTA that ever releases a stage is the one that
// consumed it, so the barrier is never reached from another CTA.
__device__ __forceinline__ void arrive_local(uint64_t* b) {
    asm volatile("mbarrier.arrive.shared::cta.b64 _, [%0], 1;\n"
        :: "r"((uint32_t)__cvta_generic_to_shared(b)) : "memory");
}

// m64n128k256 binary MMA, scale-D = 1 (accumulate into the 64 s32 regs of
// `acc`). da/db are the swizzled operand descriptors. Half the N of the
// widest binary shape, so MSTRIPS of these share one B tile to cut refill BW.
__device__ __forceinline__ void wgmma_n128(int32_t acc[64], uint64_t da, uint64_t db) {
    asm volatile(
        "wgmma.mma_async.sync.aligned.m64n128k256.row.col.s32.b1.b1.and.popc "
        "{" \
        "%0,%1,%2,%3,%4,%5,%6,%7,%8,%9,%10,%11,%12,%13,%14,%15," \
        "%16,%17,%18,%19,%20,%21,%22,%23,%24,%25,%26,%27,%28,%29,%30,%31," \
        "%32,%33,%34,%35,%36,%37,%38,%39,%40,%41,%42,%43,%44,%45,%46,%47," \
        "%48,%49,%50,%51,%52,%53,%54,%55,%56,%57,%58,%59,%60,%61,%62,%63}," \
        "%64,%65, 1;\n"
        : "+r"(acc[0]),"+r"(acc[1]),"+r"(acc[2]),"+r"(acc[3]),"+r"(acc[4]),"+r"(acc[5]),"+r"(acc[6]),"+r"(acc[7]),
          "+r"(acc[8]),"+r"(acc[9]),"+r"(acc[10]),"+r"(acc[11]),"+r"(acc[12]),"+r"(acc[13]),"+r"(acc[14]),"+r"(acc[15]),
          "+r"(acc[16]),"+r"(acc[17]),"+r"(acc[18]),"+r"(acc[19]),"+r"(acc[20]),"+r"(acc[21]),"+r"(acc[22]),"+r"(acc[23]),
          "+r"(acc[24]),"+r"(acc[25]),"+r"(acc[26]),"+r"(acc[27]),"+r"(acc[28]),"+r"(acc[29]),"+r"(acc[30]),"+r"(acc[31]),
          "+r"(acc[32]),"+r"(acc[33]),"+r"(acc[34]),"+r"(acc[35]),"+r"(acc[36]),"+r"(acc[37]),"+r"(acc[38]),"+r"(acc[39]),
          "+r"(acc[40]),"+r"(acc[41]),"+r"(acc[42]),"+r"(acc[43]),"+r"(acc[44]),"+r"(acc[45]),"+r"(acc[46]),"+r"(acc[47]),
          "+r"(acc[48]),"+r"(acc[49]),"+r"(acc[50]),"+r"(acc[51]),"+r"(acc[52]),"+r"(acc[53]),"+r"(acc[54]),"+r"(acc[55]),
          "+r"(acc[56]),"+r"(acc[57]),"+r"(acc[58]),"+r"(acc[59]),"+r"(acc[60]),"+r"(acc[61]),"+r"(acc[62]),"+r"(acc[63])
        : "l"(da), "l"(db));
}
// Make prior register writes to the accumulators visible to the async wgmma proxy.
__device__ __forceinline__ void wgmma_fence()  { asm volatile("wgmma.fence.sync.aligned;\n" ::: "memory"); }
// Close the current group of issued wgmmas so it can be waited on.
__device__ __forceinline__ void wgmma_commit() { asm volatile("wgmma.commit_group.sync.aligned;\n" ::: "memory"); }
// Wait until every committed wgmma group has retired and the accumulators are readable.
__device__ __forceinline__ void wgmma_wait()   { asm volatile("wgmma.wait_group.sync.aligned 0;\n" ::: "memory"); }

// TMA bulk tensor store (SMEM → global) of the tile at `src` to tensor coords (x, y).
__device__ __forceinline__ void tma_store_2d(
    const CUtensorMap* tm, int x, int y, const void* src) {
    asm volatile(
        "cp.async.bulk.tensor.2d.global.shared::cta.bulk_group [%0, {%1, %2}], [%3];\n"
        :: "l"((uint64_t)tm), "r"(x), "r"(y),
           "r"((uint32_t)__cvta_generic_to_shared(src)) : "memory");
}
// Close the current group of issued bulk stores so it can be waited on.
__device__ __forceinline__ void tma_store_commit() { asm volatile("cp.async.bulk.commit_group;\n" ::: "memory"); }
// Wait until every committed store group is globally visible.
__device__ __forceinline__ void tma_store_wait()   { asm volatile("cp.async.bulk.wait_group 0;\n" ::: "memory"); }
// Wait until all but the most-recent store group have *read* their SMEM source
// (`.read` = don't wait for global visibility). Lets a double-buffered sC free
// the buffer from two tiles ago while the newest store drains in the background.
__device__ __forceinline__ void tma_store_wait_read1() { asm volatile("cp.async.bulk.wait_group.read 1;\n" ::: "memory"); }
// Make generic-proxy SMEM writes visible to the async proxy, so a bulk store reads the new data.
__device__ __forceinline__ void fence_async_shared(){ asm volatile("fence.proxy.async.shared::cta;\n" ::: "memory"); }

// Per-warpgroup register reallocation (warpgroup-aligned). The producer needs few registers, so it
// releases its surplus; the consumer (MSTRIPS*ACC_N-reg accumulator) claims them. Counts must be
// multiples of 8 in [24,256] and must fit the register file:
// THREADS_PER_WG*(PRODUCER_REGS + CONSUMER_REGS) <= 65536. At 1 CTA/SM that budget is ample, so
// the binding limit is the 255-reg-per-thread hardware cap.
// Release registers down to N per thread (the producer warpgroup).
#define SET_MAXNREG_DEC(N) asm volatile("setmaxnreg.dec.sync.aligned.u32 %0;\n" :: "n"(N))
// Claim registers up to N per thread (the consumer warpgroup).
#define SET_MAXNREG_INC(N) asm volatile("setmaxnreg.inc.sync.aligned.u32 %0;\n" :: "n"(N))

// Output block = MSTRIPS m64 row-strips × NB columns per CTA. Each k256 step
// issues MSTRIPS m64n128 wgmmas that SHARE one B sub-tile, so a single L2→SMEM
// load of B feeds MSTRIPS strips — cutting refill bytes/MAC (the bottleneck) by
// ~1/(1+NB/BM). The knobs themselves live in params.h (shared with the host);
// everything below is derived.
constexpr int KL = TK/64;          // u64 per tile row (= 128 B, the swizzle width)
constexpr int TM = MW*MSTRIPS;     // output rows per CTA
constexpr int NG = NB/64;          // output column-limbs per CTA
constexpr int ACC_N = NB/2;        // s32 accumulator regs per m64n128 strip
constexpr int TILE_A = TM*KL;      // A tile: TM rows × KL u64
constexpr int TILE_B = NB*KL;      // B tile: NB cols × KL u64
constexpr int STROW = MW*KL;       // u64 per m64 strip in sA
constexpr int SC_STRIDE = NG*TM;   // u64 per sC output buffer (double-buffered)
constexpr int KSUB = TK/256;       // k256 wgmma sub-chunks per loaded tile
constexpr int KSUB_U64 = 256/64;   // u64 per k256 sub-chunk (32 bytes)
constexpr int PRODUCER_REGS = 40;
// Consumer holds MSTRIPS*ACC_N accumulator regs live across K, plus addressing;
// round up to a multiple of 8, <= 240. 1 CTA/SM so the SM reg budget is ample.
constexpr int CONSUMER_REGS = ((MSTRIPS*ACC_N + 40 + 7)/8)*8;

// wgmma 128B K-major descriptor constants (CUTLASS make_gmma_desc<Major::K>,
// LayoutType::B128): LBO = 1 uint128 = 16 bytes, SBO = 8-row-brick stride =
// 1024 bytes (independent of the MN extent), swizzle = 1. A k256 sub-chunk c
// sits at byte offset c*32 within the tile (advance start_address; the
// hardware re-applies the swizzle).
constexpr uint32_t DESC_LBO = 16;
constexpr uint32_t DESC_SBO = 1024;
constexpr uint32_t DESC_SWIZ = 1;

// ── Kernel ──────────────────────────────────────────────────────────────────

// Producer-consumer kernel: 2 warpgroups of THREADS_PER_WG threads each.
//   Warpgroup 0 = PRODUCER: issues TMA loads in a tight K-loop into a STAGES-deep
//                 circular SMEM buffer.
//   Warpgroup 1 = CONSUMER: waits for each stage to be full, runs KSUB*MSTRIPS
//                 pipelined m64n128 wgmmas against it, signals the stage empty so
//                 the producer can refill.
//
// Dynamic SMEM per CTA (carved from `smem`, 128B-aligned for TMA):
//   sA[STAGES][TILE_A]  = STAGES * TILE_A u64
//   sB[STAGES][TILE_B]  = STAGES * TILE_B u64
//   sC[2][TM][NG]       = 2 * TM * NG u64 (double-buffered so the output store of
//                         tile T overlaps tile T+1's compute)
//   mbar_full[STAGES] + mbar_empty[STAGES]
//
// A stage is sA + sB, and the total exceeds the 48 KB static cap, so the host must opt in via
// CU_FUNC_ATTRIBUTE_MAX_DYNAMIC_SHARED_SIZE_BYTES. The resulting footprint is what holds the
// kernel to 1 CTA/SM; STAGES is the K-pipeline-depth knob.
//
// The output block (TM rows × NG limbs) is packed row-major into sC and written
// back with a single TMA bulk store (S2G). C is padded to whole NG-limb column
// groups on the host so every stored tile is complete.
extern "C" __global__ void matmul_b1_kernel(
    const __grid_constant__ CUtensorMap tma_a,
    const __grid_constant__ CUtensorMap tma_b,
    const __grid_constant__ CUtensorMap tma_c,
    uint32_t m_tiles,
    uint32_t n_groups,
    uint32_t M, uint32_t K)
{
    extern __shared__ __align__(128) uint64_t smem[];
    uint64_t* sA = smem;                          // [STAGES][TILE_A]
    uint64_t* sB = sA + STAGES * TILE_A;          // [STAGES][TILE_B]
    uint64_t* sC = sB + STAGES * TILE_B;          // [2][TM][NG] row-major (double-buffered)
    uint64_t* mbar_full  = sC + 2 * SC_STRIDE;    // [STAGES]
    uint64_t* mbar_empty = mbar_full + STAGES;    // [STAGES]

    const int t = threadIdx.x;
    const int wg = t / THREADS_PER_WG;        // 0 = producer, 1 = consumer
    const int t_wg = t - wg * THREADS_PER_WG; // 0..127 within warpgroup

    const int nchunks = (K + TK - 1) / TK;
    // One full A tile + one full B tile per stage (B is zero-padded on the
    // host to a multiple of NB columns, so it is always a complete tile). Both
    // target this CTA's full barrier.
    const uint32_t expected_tx = (uint32_t)((TILE_A + TILE_B) * sizeof(uint64_t));

    // Tile-grid geometry: one CTA per output tile-iteration, striding the whole
    // m_tiles × n_groups grid.
    const uint32_t total_tiles = m_tiles * n_groups;

    // Register reallocation is a one-time per-warpgroup action.
    if (wg == 0) SET_MAXNREG_DEC(PRODUCER_REGS);
    else         SET_MAXNREG_INC(CONSUMER_REGS);

    // Initialize the pipeline barriers ONCE; they flow continuously across the
    // persistent tile loop (no per-tile re-init). Each stage is this CTA's
    // alone, so the empty barrier takes a single arrival, from its consumer.
    if (t == 0) {
        #pragma unroll
        for (int s = 0; s < STAGES; ++s) {
            mbar_init(&mbar_full[s], 1);
            mbar_init(&mbar_empty[s], 1);
        }
    }
    __syncthreads();

    // Pre-arrive every empty barrier so the producer's first STAGES
    // `mbar_wait(empty, 0)` succeed immediately (stages logically free).
    if (wg == 1 && t_wg == 0) {
        #pragma unroll
        for (int s = 0; s < STAGES; ++s) arrive_local(&mbar_empty[s]);
    }

    // ===================== PERSISTENT TILE LOOP =====================
    // A 1-D grid of independent CTAs sweeps the M × N tile grid. The grouped
    // rasterizer (M-tile varies fastest within a GROUP_M band) keeps each
    // B-panel's reuse distance short for L2 residency.
    // qidx/p are the running pipeline slot/phase, carried across tiles.
    // sC is double-buffered so tile T's output store overlaps tile T+1's
    // compute: cbuf ping-pongs per tile, and the store's wait is deferred one
    // tile (see epilogue). titer counts this CTA's tiles for the ping-pong.
    uint32_t qidx = 0, p = 0, titer = 0;
    for (uint32_t ct = blockIdx.x; ct < total_tiles; ct += gridDim.x, ++titer) {
        const uint32_t gid    = ct / (GROUP_M * n_groups);
        const uint32_t firstm = gid * GROUP_M;
        const uint32_t curm   = min((uint32_t)GROUP_M, m_tiles - firstm);
        const uint32_t local  = ct - gid * GROUP_M * n_groups;
        const int bi = (int)(firstm + local % curm);  // this CTA's M-tile
        const int bj = (int)(local / curm);
        const int row0 = bi * TM, col0 = bj * NG;
        uint64_t* sCb = sC + (titer & 1) * SC_STRIDE;   // this tile's sC buffer

        // Before reusing this buffer, make sure the store that last used it (two
        // tiles ago) has finished reading it. The deferred .read-1 wait below
        // already drained it during the previous tile; this syncs all consumer
        // threads to that wait before they overwrite the buffer.
        if (wg == 1) {
            for (int r = t_wg; r < TM; r += THREADS_PER_WG) {
                #pragma unroll
                for (int g = 0; g < NG; ++g) sCb[r * NG + g] = 0;
            }
        }
        __syncthreads();

        if (wg == 0) {
            // ===================== PRODUCER =====================
            for (int kk = 0; kk < nchunks; ++kk) {
                const uint32_t s = qidx;

                if (t_wg == 0) {
                    // Wait for the consumer to release this stage, then set the
                    // expected bytes (A + B) and issue both loads.
                    mbar_wait(&mbar_empty[s], p);
                    mbar_tx(&mbar_full[s], expected_tx);
                    // A: this CTA's own TM-row block (MSTRIPS m64 strips).
                    tma_2d(&sA[s * TILE_A], &tma_a, 0,
                           (kk * m_tiles + bi) * TM, &mbar_full[s]);
                    // B: this CTA's own copy of the panel. Rasterization keeps
                    // the panel L2-resident, so this is an L2 hit, not HBM.
                    tma_2d(&sB[s * TILE_B], &tma_b, 0,
                           (kk * n_groups + bj) * NB, &mbar_full[s]);
                }
                if (++qidx == STAGES) { qidx = 0; p ^= 1; }
            }
        } else {
            // ===================== CONSUMER =====================
            // MSTRIPS m64n128 accumulators (MSTRIPS*ACC_N s32 regs/thread), all
            // resident across the whole K loop, re-zeroed per output tile.
            int32_t acc[MSTRIPS][ACC_N];
            #pragma unroll
            for (int si = 0; si < MSTRIPS; ++si)
                #pragma unroll
                for (int r = 0; r < ACC_N; ++r) acc[si][r] = 0;

            // One wgmma.fence for the whole K-loop: it orders the non-wgmma
            // accumulator zeroing above before the first wgmma. Inside the loop
            // the accumulators are written only by wgmma, so no per-chunk fence
            // is needed (a warpgroup-wide sync we don't want 32× per tile). The
            // per-chunk wgmma.wait_group 0 makes the results readable at the end.
            wgmma_fence();
            for (int kk = 0; kk < nchunks; ++kk) {
                const uint32_t s = qidx;

                // Wait for the producer's TMAs to finish populating this stage.
                mbar_wait(&mbar_full[s], p);

                // Issue every k256 wgmma for this stage behind one commit/wait so
                // they pipeline. Each k256 loads B once and reuses it across all
                // MSTRIPS strips (independent accumulators → they can overlap).
                // scale-D = 1 accumulates each sub-chunk in-hardware.
                #pragma unroll
                for (int c = 0; c < KSUB; ++c) {
                    uint64_t db = make_desc(&sB[s * TILE_B + c * KSUB_U64],
                                            DESC_LBO, DESC_SBO, DESC_SWIZ);
                    #pragma unroll
                    for (int si = 0; si < MSTRIPS; ++si) {
                        uint64_t da = make_desc(&sA[s * TILE_A + si * STROW + c * KSUB_U64],
                                                DESC_LBO, DESC_SBO, DESC_SWIZ);
                        wgmma_n128(acc[si], da, db);
                    }
                }
                wgmma_commit();
                wgmma_wait();

                // Release this stage: the stage is this CTA's alone, so a local
                // arrive is the whole of the release.
                if (t_wg == 0) arrive_local(&mbar_empty[s]);
                if (++qidx == STAGES) { qidx = 0; p ^= 1; }
            }

            // Pack each strip's NB-wide accumulator into sC's NG output limbs.
            // The m64n128 fragment is the m64n64 layout tiled along N: register
            // group gi (0..NB/8-1) covers output columns [gi*8, gi*8+8); within it
            // this thread owns columns cb, cb+1 for rows rb and rb+8. Strip si adds
            // si*MW to the row. Column c maps to limb c/64, bit c%64.
            const int wid = t_wg >> 5, lane = t_wg & 31;
            const int rb = wid*16 + (lane>>2), cb = (lane&3)*2;
            #pragma unroll
            for (int si = 0; si < MSTRIPS; ++si) {
                uint64_t lo[NG] = {0}, hi[NG] = {0};
                #pragma unroll
                for (int gi = 0; gi < NB/8; ++gi) {
                    int c0 = cb + gi*8, c1 = c0 + 1;
                    int l0 = c0 >> 6, b0p = c0 & 63;
                    int l1 = c1 >> 6, b1p = c1 & 63;
                    lo[l0] |= (uint64_t)(acc[si][gi*4+0]&1) << b0p;
                    lo[l1] |= (uint64_t)(acc[si][gi*4+1]&1) << b1p;
                    hi[l0] |= (uint64_t)(acc[si][gi*4+2]&1) << b0p;
                    hi[l1] |= (uint64_t)(acc[si][gi*4+3]&1) << b1p;
                }
                // Row-major sC[row*NG + limb]; padded limbs (out-of-range columns)
                // get zero popcounts from the zero-padded B, so they store harmless
                // zeros into C's padded region (trimmed on the host).
                const int rlo = si*MW + rb, rhi = si*MW + rb + 8;
                #pragma unroll
                for (int g = 0; g < NG; ++g) {
                    uint32_t* clo = reinterpret_cast<uint32_t*>(&sCb[rlo * NG + g]);
                    uint32_t* chi = reinterpret_cast<uint32_t*>(&sCb[rhi * NG + g]);
                    atomicXor(&clo[0], (uint32_t)lo[g]);
                    atomicXor(&clo[1], (uint32_t)(lo[g]>>32));
                    atomicXor(&chi[0], (uint32_t)hi[g]);
                    atomicXor(&chi[1], (uint32_t)(hi[g]>>32));
                }
            }
        }

        // Write the TM×NG output block back with a single TMA bulk store, then
        // DEFER its wait: `.read 1` blocks only until every store but the newest
        // (this one) has finished reading its sC buffer, so tile T's store drains
        // during tile T+1's compute instead of stalling here. The buffer freed is
        // the one from two tiles ago — safe to reuse next time cbuf lands on it.
        // The fence precedes the barrier: fence.proxy.async orders only the *executing* thread's
        // prior generic-proxy writes, so every thread has to fence its own atomicXor output before
        // the barrier lets t == 0 hand sC to the async proxy.
        fence_async_shared();
        __syncthreads();        // sC[cbuf] fully packed, and every consumer's fence retired
        if (t == 0) {
            tma_store_2d(&tma_c, col0 * 2, row0, sCb); // x in UINT32 units (2 per limb)
            tma_store_commit();
            tma_store_wait_read1();
        }
        __syncthreads();        // order the deferred wait before the next tile
                                // reuses (zeroes) an sC buffer
    }
    // Drain the last outstanding output store before the CTA exits.
    if (t == 0) tma_store_wait();
}

// Build the K-major tiles the matmul consumes, reading B in its natural row-major layout.
//
// Tile (k_chunk, column group) row `lg*64 + jj`, limb `kl`, bit `bit` is bit `jj` of
// B[k_chunk*TK + kl*64 + bit][cg*NG + lg]. One block owns one (k_chunk, cg, lg, kl) quadruple and
// so one 64-square bit block: thread `bit` loads that block's row into shared memory, then thread
// `jj` gathers bit `jj` out of all 64 rows. The gather reads one shared slot at a time across the
// whole block, so every read is a broadcast rather than a bank conflict.
//
// Both the loads (a column of B, stride n_lim) and the stores (stride KL) are strided.
extern "C" __global__ void transpose_tile_b1_kernel(
    const unsigned long long* __restrict__ b, // k_padded x n_lim, row-major
    unsigned long long* __restrict__ out,     // k_chunks x n_groups x (NB*KL)
    int n_lim,                                // limbs per row of B
    int k_rows,                               // rows of B actually uploaded (the unpadded k)
    int n_groups)                             // column groups of NG limbs
{
    __shared__ unsigned long long sB[64];

    const int kl = blockIdx.x % KL;
    const int lg = (blockIdx.x / KL) % (NB / 64);
    const int cg = blockIdx.y;
    const int kk = blockIdx.z;
    const int t = threadIdx.x; // 0..63

    const int limb = cg * (NB / 64) + lg;
    const int row = kk * TK + kl * 64 + t;
    // Column groups past the operand, and K rows past the end of B, contribute zeros — so B is
    // uploaded unpadded and the K padding costs no host copy.
    sB[t] = (limb < n_lim && row < k_rows) ? b[(long long)row * n_lim + limb] : 0ULL;
    __syncthreads();

    unsigned long long val = 0;
    for (int bit = 0; bit < 64; ++bit) {
        val |= ((sB[bit] >> t) & 1ULL) << bit;
    }

    const long long tile = (long long)NB * KL;
    const long long base = ((long long)kk * n_groups + cg) * tile;
    out[base + (long long)(lg * 64 + t) * KL + kl] = val;
}
