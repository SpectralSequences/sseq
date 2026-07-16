// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Hopper wgmma.b1 F_2 GEMM kernel — 128B-swizzle operands, pipelined wgmmas,
// the widest binary MMA shape (m64n256k256), a persistent grid with grouped
// tile rasterization (Phase 8), and thread-block clusters + TMA B-multicast
// (Phase 9) — both target L2 residency of B at large N.
//
// Both operands are K-major. They are pre-arranged on the host as plain
// row-major tiles and loaded via TMA cp.async.bulk.tensor.2d with
// CU_TENSOR_MAP_SWIZZLE_128B: the TMA hardware applies the 128B swizzle on the
// way into SMEM, landing the data exactly where the swizzled wgmma matrix
// descriptor expects it — so the host emits the natural layout and there is no
// hand-rolled interleave.
//
// Each loaded tile spans a full 128B K-major swizzle atom (8 rows × 1024 bits),
// i.e. KSUB = 4 consecutive k256 sub-chunks. A CTA computes a register-blocked
// TM×NB output block (MSTRIPS m64 row-strips × 128 columns). Each k256 step
// loads B once and issues MSTRIPS m64n128k256 wgmmas that all reuse it — one
// L2→SMEM read of B feeds every strip, which cuts the operand-refill bytes per
// MAC (the measured bottleneck on Hopper: the tensor core out-runs L2→SMEM
// bandwidth). Each strip accumulates into its own resident 64-reg accumulator
// (scale-D = 1, popcounts summed in-hardware), all live across the whole K loop.
//
// The grid is persistent: ~SM-count CTAs (in clusters of CLUSTER along M) sweep
// the output tile grid in a grouped-along-M rasterized order so each B-panel's
// reuse distance stays short (L2-resident). Within a cluster the CLUSTER CTAs
// share one HBM read of each B-panel via TMA multicast — each computes a
// different M-tile but receives the same B into its own SMEM. The pipeline
// barriers are initialized once and flow continuously across tiles; the empty
// barrier is cluster-wide. This mirrors the proven pattern in
// pranjalssh/fast.cu matmul_9.cuh.

#include <cstdint>
#include <cuda_runtime.h>
#include <cuda.h>

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

__device__ __forceinline__ void mbar_init(uint64_t* b, uint32_t cnt) {
    asm volatile("mbarrier.init.shared::cta.b64 [%0], %1;\n"
        :: "r"((uint32_t)__cvta_generic_to_shared(b)), "r"(cnt));
}
__device__ __forceinline__ void mbar_tx(uint64_t* b, uint32_t bytes) {
    asm volatile("mbarrier.arrive.expect_tx.shared::cta.b64 _, [%0], %1;\n"
        :: "r"((uint32_t)__cvta_generic_to_shared(b)), "r"(bytes) : "memory");
}
__device__ __forceinline__ void mbar_wait(uint64_t* b, uint32_t phase) {
    uint32_t a = (uint32_t)__cvta_generic_to_shared(b);
    asm volatile(
        "{ .reg .pred p;\n"
        "  L: mbarrier.try_wait.parity.shared::cta.b64 p, [%0], %1;\n"
        "  @!p bra L;\n"
        "}\n" :: "r"(a), "r"(phase) : "memory");
}
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

// ── Cluster helpers (Phase 9: clusters + TMA multicast) ───────────────────────
// All mirror the proven pattern in pranjalssh/fast.cu matmul_9.cuh.

// This CTA's rank within its cluster (0..CLUSTER-1).
__device__ __forceinline__ uint32_t cluster_ctarank() {
    uint32_t r;
    asm volatile("mov.u32 %0, %cluster_ctarank;\n" : "=r"(r) :);
    return r;
}

// Cluster-wide barrier: every thread of every CTA in the cluster must arrive.
__device__ __forceinline__ void cluster_sync() {
    asm volatile("barrier.cluster.arrive;\n" ::: "memory");
    asm volatile("barrier.cluster.wait;\n" ::: "memory");
}

// Arrive (count 1) on the mbarrier `b` located in cluster-mate CTA `cta_id`,
// using mapa to translate the local SMEM address into that CTA's window.
__device__ __forceinline__ void arrive_cluster(uint64_t* b, uint32_t cta_id) {
    uint32_t local = (uint32_t)__cvta_generic_to_shared(b);
    asm volatile(
        "{ .reg .b32 rem;\n"
        "  mapa.shared::cluster.u32 rem, %0, %1;\n"
        "  mbarrier.arrive.shared::cluster.b64 _, [rem], 1;\n"
        "}\n" :: "r"(local), "r"(cta_id) : "memory");
}

// TMA load with cluster multicast: one HBM read of the source tile is fanned
// out into the SMEM of every CTA whose bit is set in `mask` (same `dst` SMEM
// offset and `b` mbarrier offset in each), and counts complete_tx bytes against
// each of their barriers. Issued by a single thread of one CTA.
__device__ __forceinline__ void tma_2d_multicast(
    void* dst, const CUtensorMap* tm, int x, int y, uint64_t* b, uint16_t mask) {
    asm volatile(
        "cp.async.bulk.tensor.2d.shared::cluster.global"
        ".mbarrier::complete_tx::bytes.multicast::cluster"
        " [%0], [%1, {%2,%3}], [%4], %5;\n"
        :: "r"((uint32_t)__cvta_generic_to_shared(dst)),
           "l"((uint64_t)tm), "r"(x), "r"(y),
           "r"((uint32_t)__cvta_generic_to_shared(b)), "h"(mask)
        : "memory");
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
__device__ __forceinline__ void wgmma_fence()  { asm volatile("wgmma.fence.sync.aligned;\n" ::: "memory"); }
__device__ __forceinline__ void wgmma_commit() { asm volatile("wgmma.commit_group.sync.aligned;\n" ::: "memory"); }
__device__ __forceinline__ void wgmma_wait()   { asm volatile("wgmma.wait_group.sync.aligned 0;\n" ::: "memory"); }

// TMA bulk tensor store (SMEM → global) plus its completion group helpers and
// the async-proxy fence that makes generic-proxy SMEM writes visible to it.
__device__ __forceinline__ void tma_store_2d(
    const CUtensorMap* tm, int x, int y, const void* src) {
    asm volatile(
        "cp.async.bulk.tensor.2d.global.shared::cta.bulk_group [%0, {%1, %2}], [%3];\n"
        :: "l"((uint64_t)tm), "r"(x), "r"(y),
           "r"((uint32_t)__cvta_generic_to_shared(src)) : "memory");
}
__device__ __forceinline__ void tma_store_commit() { asm volatile("cp.async.bulk.commit_group;\n" ::: "memory"); }
__device__ __forceinline__ void tma_store_wait()   { asm volatile("cp.async.bulk.wait_group 0;\n" ::: "memory"); }
// Wait until all but the most-recent store group have *read* their SMEM source
// (`.read` = don't wait for global visibility). Lets a double-buffered sC free
// the buffer from two tiles ago while the newest store drains in the background.
__device__ __forceinline__ void tma_store_wait_read1() { asm volatile("cp.async.bulk.wait_group.read 1;\n" ::: "memory"); }
__device__ __forceinline__ void fence_async_shared(){ asm volatile("fence.proxy.async.shared::cta;\n" ::: "memory"); }

// Per-warpgroup register reallocation (warpgroup-aligned). The producer needs
// few registers, so it releases its surplus; the consumer (MSTRIPS*ACC_N-reg
// accumulator) claims them. Counts must be multiples of 8 in [24,256]; at
// 1 CTA/SM the SM's 64K-register budget is ample (128*(40+232) = 34816 at
// MSTRIPS=3), so the binding limit is the 255-reg-per-thread hardware cap.
#define SET_MAXNREG_DEC(N) asm volatile("setmaxnreg.dec.sync.aligned.u32 %0;\n" :: "n"(N))
#define SET_MAXNREG_INC(N) asm volatile("setmaxnreg.inc.sync.aligned.u32 %0;\n" :: "n"(N))

// Output block = MSTRIPS m64 row-strips × NB columns per CTA. Each k256 step
// issues MSTRIPS m64n128 wgmmas that SHARE one B sub-tile, so a single L2→SMEM
// load of B feeds MSTRIPS strips — cutting refill bytes/MAC (the bottleneck) by
// ~1/(1+NB/BM). MSTRIPS is the block knob: 2 → 128×128 block (−20% bytes/MAC,
// 128 acc regs), 3 → 192×128 (−33%, 192 acc regs). NB is fixed at 128 (the
// wgmma_n128 shape); acc regs per thread = MSTRIPS*ACC_N ≤ 240.
constexpr int MSTRIPS = 3;         // m64 row-strips per CTA (block knob)
constexpr int MW = 64;             // wgmma M extent (fixed for binary wgmma)
constexpr int TK = 1024, KL = TK/64;
constexpr int TM = MW*MSTRIPS;     // output rows per CTA (192 at MSTRIPS=3)
constexpr int NB = 128;            // n128 output width (columns) per CTA
constexpr int NG = NB/64;          // 2 output column-limbs per CTA
constexpr int ACC_N = NB/2;        // 64 s32 accumulator regs per m64n128 strip
constexpr int TILE_A = TM*KL;      // A tile: TM rows × 16 u64 (192 rows → 3072 u64 = 24 KB)
constexpr int TILE_B = NB*KL;      // B tile: 128 cols × 16 u64 = 2048 u64 = 16 KB
constexpr int STROW = MW*KL;       // u64 per m64 strip in sA (64*16 = 1024 = 8 KB)
constexpr int SC_STRIDE = NG*TM;   // u64 per sC output buffer (double-buffered)
constexpr int KSUB = TK/256;       // 4 k256 wgmma sub-chunks per loaded tile
constexpr int KSUB_U64 = 256/64;   // 4 u64 = 32 bytes per k256 sub-chunk
// K-loop pipeline depth. Cap is 4 under CLUSTER>1: each stage is 40 KB, so
// STAGES=5 needs ~206 KB, which is under the 227 KB opt-in cap BUT a cluster
// launch reserves extra shared memory (distributed-SMEM / cluster-barrier
// bookkeeping), so 206 KB intermittently over-commits and the kernel faults
// with a flaky "unspecified launch failure" (verified 2026-07-07 H200; the
// sanitizers miss it because it is an async/resource fault). STAGES=5 is stable
// only with CLUSTER=1, and gives no large-N speedup there, so 4 it is.
constexpr int STAGES = 4;          // K-loop pipeline depth (full/empty buffers)
constexpr int THREADS_PER_WG = 128;
constexpr int GROUP_M = 16;        // M-tiles per rasterization group (L2 reuse knob)
constexpr int CLUSTER = 2;         // CTAs per cluster along M (multicast B; reuse knob)
constexpr int PRODUCER_REGS = 40;
// Consumer holds MSTRIPS*ACC_N accumulator regs live across K, plus addressing;
// round up to a multiple of 8, ≤ 240. 1 CTA/SM so the SM reg budget is ample.
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

// Producer-consumer kernel: 2 warpgroups (256 threads/CTA).
//   Warpgroup 0 (t in [0, 128))  = PRODUCER: issues TMA loads in a tight
//                                  K-loop into a STAGES-deep circular SMEM
//                                  buffer.
//   Warpgroup 1 (t in [128, 256)) = CONSUMER: waits for each stage to be full,
//                                   runs KSUB*MSTRIPS pipelined m64n128 wgmmas
//                                   against it, signals the stage empty so the
//                                   producer can refill.
//
// Dynamic SMEM per CTA (carved from `smem`, 128B-aligned for TMA):
//   sA[STAGES][TILE_A]  = STAGES * 24576 B (TM=192 rows)
//   sB[STAGES][TILE_B]  = STAGES * 16384 B (NB=128 cols)
//   sC[2][TM][NG]       = 2 * TM * NG * 8 B (double-buffered so the output store
//                         of tile T overlaps tile T+1's compute)
//   mbar_full[STAGES] + mbar_empty[STAGES]
//
// Per stage = sA (24 KB) + sB (16 KB) = 40 KB; STAGES=4 ≈ 163 KB total (requires
// the opt-in CU_FUNC_ATTRIBUTE_MAX_DYNAMIC_SHARED_SIZE_BYTES set host-side).
// At 40 KB/stage the block runs 1 CTA/SM; STAGES is the K-pipeline-depth knob.
//
// The output block (TM rows × NG limbs) is packed row-major into sC and written
// back with a single TMA bulk store (S2G). C is padded to whole NG-limb column
// groups on the host so every stored tile is complete.
extern "C" __global__ void __cluster_dims__(CLUSTER, 1, 1) matmul_b1_kernel(
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
    // host to a multiple of NB columns, so it is always a complete tile). A is
    // loaded per-CTA; B arrives via multicast — both target this CTA's full
    // barrier, so the expected bytes are the same as the single-CTA case.
    const uint32_t expected_tx = (uint32_t)((TILE_A + TILE_B) * sizeof(uint64_t));

    // Cluster geometry: CLUSTER CTAs along M share one B-panel via multicast,
    // so the schedule walks "M-super-rows" of CLUSTER M-tiles. The host pads
    // m_tiles to a multiple of CLUSTER, so m_super divides exactly.
    const uint32_t rank         = cluster_ctarank();      // 0..CLUSTER-1 (= M offset)
    const uint32_t cluster_id   = blockIdx.x / CLUSTER;
    const uint32_t num_clusters = gridDim.x / CLUSTER;
    const uint32_t m_super      = m_tiles / CLUSTER;
    const uint32_t total_cl     = m_super * n_groups;
    const uint16_t bmask        = (uint16_t)((1u << CLUSTER) - 1u); // all ranks

    // Register reallocation is a one-time per-warpgroup action.
    if (wg == 0) SET_MAXNREG_DEC(PRODUCER_REGS);
    else         SET_MAXNREG_INC(CONSUMER_REGS);

    // Initialize the pipeline barriers ONCE; they flow continuously across the
    // persistent tile loop (no per-tile re-init, which would race with the
    // cross-CTA arrivals/multicast of a cluster). The empty barrier is
    // cluster-wide: it needs one arrival from every CTA's consumer.
    if (t == 0) {
        #pragma unroll
        for (int s = 0; s < STAGES; ++s) {
            mbar_init(&mbar_full[s], 1);
            mbar_init(&mbar_empty[s], CLUSTER);
        }
    }
    __syncthreads();
    cluster_sync();   // all CTAs' barriers initialized before any cross-CTA arrive

    // Pre-arrive every empty barrier cluster-wide so the producer's first
    // STAGES `mbar_wait(empty, 0)` succeed immediately (stages logically free).
    if (wg == 1 && t_wg < CLUSTER) {
        #pragma unroll
        for (int s = 0; s < STAGES; ++s) arrive_cluster(&mbar_empty[s], t_wg);
    }

    // ===================== PERSISTENT CLUSTER LOOP =====================
    // A 1-D grid of clusters sweeps the M-super × N tile grid. The grouped
    // rasterizer (super-row varies fastest within a GROUP_M band) keeps each
    // B-panel's reuse distance short for L2 residency; the cluster additionally
    // shares each B-panel HBM read across its CLUSTER CTAs via multicast.
    // qidx/p are the running pipeline slot/phase, carried across tiles.
    // sC is double-buffered so tile T's output store overlaps tile T+1's
    // compute: cbuf ping-pongs per tile, and the store's wait is deferred one
    // tile (see epilogue). titer counts this CTA's tiles for the ping-pong.
    uint32_t qidx = 0, p = 0, titer = 0;
    for (uint32_t ct = cluster_id; ct < total_cl; ct += num_clusters, ++titer) {
        const uint32_t gid    = ct / (GROUP_M * n_groups);
        const uint32_t firstm = gid * GROUP_M;
        const uint32_t curm   = min((uint32_t)GROUP_M, m_super - firstm);
        const uint32_t local  = ct - gid * GROUP_M * n_groups;
        const uint32_t sbi    = firstm + local % curm;
        const int bj = (int)(local / curm);
        const int bi = (int)(sbi * CLUSTER + rank);  // this CTA's M-tile
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
                    // Wait for all CTAs' consumers to release this stage, then
                    // set expected bytes (A + multicast B) and issue the loads.
                    mbar_wait(&mbar_empty[s], p);
                    mbar_tx(&mbar_full[s], expected_tx);
                    // A: this CTA's own TM-row block (MSTRIPS m64 strips).
                    tma_2d(&sA[s * TILE_A], &tma_a, 0,
                           (kk * m_tiles + bi) * TM, &mbar_full[s]);
                    // B: one HBM read, multicast into every cluster member's sB
                    // and counted against every member's full barrier. Issued by
                    // rank 0 only (its mask bit is set, so it fills itself too).
                    if (rank == 0) {
                        tma_2d_multicast(&sB[s * TILE_B], &tma_b, 0,
                                         (kk * n_groups + bj) * NB, &mbar_full[s],
                                         bmask);
                    }
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

                // Release this stage cluster-wide: arrive on every CTA's empty
                // barrier (so rank 0 may overwrite their multicast sB).
                if (t_wg < CLUSTER) arrive_cluster(&mbar_empty[s], t_wg);
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
        __syncthreads();        // sC[cbuf] fully packed by the consumer
        fence_async_shared();   // make the atomicXor writes visible to the async proxy
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

// ── Device-resident packing kernels (BLAS3 GPU row-reduction port) ───────────
//
// These reproduce, on device, the host operand pre-arrangement in src/lib.rs
// (pad_2d + interleave_a for A; pad_2d + transpose_b for B) so a GEMM can run
// over persistent device buffers with no host round-trip. One thread per output
// u64. Packing is lower-order work, so these favor clarity over peak bandwidth.
// They reference the tiling constants (TM, KL, NG, TK) above — the single source
// of truth shared with the compute kernel.

typedef unsigned long long u64_t;

// A: gather the natural row-major (m_orig × sa_orig) limb array into row-major
// K-major tiles (TM rows × KL u64), ordered K-chunk-major then M-tile-major.
// Rows/limbs past the real extent (padding M→m_padded, K→k_padded) read as zero.
extern "C" __global__ void pack_a(
    u64_t* __restrict__ out, const u64_t* __restrict__ a,
    unsigned m_orig, unsigned sa_orig, unsigned m_tiles, unsigned total)
{
    unsigned idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= total) return;
    const unsigned tile_u64s = TM * KL;
    unsigned tile_idx = idx / tile_u64s;
    unsigned within   = idx % tile_u64s;
    unsigned row = within / KL;
    unsigned kl  = within % KL;
    unsigned bi = tile_idx % m_tiles;
    unsigned kk = tile_idx / m_tiles;
    unsigned global_row = bi * TM + row;
    unsigned global_kl  = kk * KL + kl;
    u64_t val = 0;
    if (global_row < m_orig && global_kl < sa_orig)
        val = a[(u64_t)global_row * sa_orig + global_kl];
    out[idx] = val;
}

// B: pad_2d(K→k_padded) + transpose_b. Natural (k_orig × n_lim) limb array →
// K-major bit-transposed tiles (NG*64 operand rows × KL u64), ordered K-chunk-
// major then column-group-major. Column groups whose limb ≥ n_lim stay zero.
extern "C" __global__ void pack_b(
    u64_t* __restrict__ out, const u64_t* __restrict__ b,
    unsigned k_orig, unsigned n_lim, unsigned n_groups, unsigned total)
{
    unsigned idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= total) return;
    const unsigned tile = NG * 64 * KL;
    unsigned tile_idx = idx / tile;
    unsigned within   = idx % tile;
    unsigned j  = within / KL;   // operand row within the NG*64-row tile
    unsigned kl = within % KL;
    unsigned cg = tile_idx % n_groups;
    unsigned kk = tile_idx / n_groups;
    unsigned lg = j / 64;
    unsigned jj = j % 64;
    unsigned limb = cg * NG + lg;
    if (limb >= n_lim) { out[idx] = 0; return; }
    u64_t val = 0;
    #pragma unroll
    for (unsigned bit = 0; bit < 64; ++bit) {
        unsigned br = kk * TK + kl * 64 + bit;
        u64_t word = (br < k_orig) ? b[(u64_t)br * n_lim + limb] : 0;
        val |= ((word >> jj) & 1ULL) << bit;
    }
    out[idx] = val;
}

// Fused XOR-accumulate the padded GEMM output C (m × c_stride limbs/row) into a
// destination region of a persistent matrix (dst_stride limbs/row) starting at
// limb dst_limb: dst[j][dst_limb + col] ^= C[j][col], for j<m, col<width.
extern "C" __global__ void xor_into(
    u64_t* __restrict__ dst, const u64_t* __restrict__ c,
    unsigned m, unsigned width, unsigned dst_stride, unsigned dst_limb,
    unsigned c_stride)
{
    unsigned idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= m * width) return;
    unsigned j   = idx / width;
    unsigned col = idx % width;
    dst[(u64_t)j * dst_stride + dst_limb + col] ^= c[(u64_t)j * c_stride + col];
}

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
