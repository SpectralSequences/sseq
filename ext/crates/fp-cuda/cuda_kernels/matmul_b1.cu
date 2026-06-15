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
// i.e. KSUB = 4 consecutive k256 sub-chunks. A is one 64-row tile; B is one
// 256-column tile, so each k256 step is a single m64n256k256 wgmma covering all
// NG = 4 output column-limbs of the CTA at once (instead of four m64n64 wgmmas).
// The consumer issues all KSUB wgmmas behind a single commit/wait and
// accumulates the popcounts in-hardware (scale-D = 1) into one resident
// accumulator that stays live across the whole K loop.
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

// m64n256k256 binary MMA, scale-D = 1 (accumulate into the 128 s32 regs of
// `acc`, which the consumer pre-zeroes). da/db are the swizzled operand
// descriptors.
__device__ __forceinline__ void wgmma_n256(int32_t acc[128], uint64_t da, uint64_t db) {
    asm volatile(
        "wgmma.mma_async.sync.aligned.m64n256k256.row.col.s32.b1.b1.and.popc "
        "{%0,%1,%2,%3,%4,%5,%6,%7,%8,%9,%10,%11,%12,%13,%14,%15," \
        "%16,%17,%18,%19,%20,%21,%22,%23,%24,%25,%26,%27,%28,%29,%30,%31," \
        "%32,%33,%34,%35,%36,%37,%38,%39,%40,%41,%42,%43,%44,%45,%46,%47," \
        "%48,%49,%50,%51,%52,%53,%54,%55,%56,%57,%58,%59,%60,%61,%62,%63," \
        "%64,%65,%66,%67,%68,%69,%70,%71,%72,%73,%74,%75,%76,%77,%78,%79," \
        "%80,%81,%82,%83,%84,%85,%86,%87,%88,%89,%90,%91,%92,%93,%94,%95," \
        "%96,%97,%98,%99,%100,%101,%102,%103,%104,%105,%106,%107,%108,%109,%110,%111," \
        "%112,%113,%114,%115,%116,%117,%118,%119,%120,%121,%122,%123,%124,%125,%126,%127}," \
        "%128,%129, 1;\n"
        : "+r"(acc[0]),"+r"(acc[1]),"+r"(acc[2]),"+r"(acc[3]),
          "+r"(acc[4]),"+r"(acc[5]),"+r"(acc[6]),"+r"(acc[7]),
          "+r"(acc[8]),"+r"(acc[9]),"+r"(acc[10]),"+r"(acc[11]),
          "+r"(acc[12]),"+r"(acc[13]),"+r"(acc[14]),"+r"(acc[15]),
          "+r"(acc[16]),"+r"(acc[17]),"+r"(acc[18]),"+r"(acc[19]),
          "+r"(acc[20]),"+r"(acc[21]),"+r"(acc[22]),"+r"(acc[23]),
          "+r"(acc[24]),"+r"(acc[25]),"+r"(acc[26]),"+r"(acc[27]),
          "+r"(acc[28]),"+r"(acc[29]),"+r"(acc[30]),"+r"(acc[31]),
          "+r"(acc[32]),"+r"(acc[33]),"+r"(acc[34]),"+r"(acc[35]),
          "+r"(acc[36]),"+r"(acc[37]),"+r"(acc[38]),"+r"(acc[39]),
          "+r"(acc[40]),"+r"(acc[41]),"+r"(acc[42]),"+r"(acc[43]),
          "+r"(acc[44]),"+r"(acc[45]),"+r"(acc[46]),"+r"(acc[47]),
          "+r"(acc[48]),"+r"(acc[49]),"+r"(acc[50]),"+r"(acc[51]),
          "+r"(acc[52]),"+r"(acc[53]),"+r"(acc[54]),"+r"(acc[55]),
          "+r"(acc[56]),"+r"(acc[57]),"+r"(acc[58]),"+r"(acc[59]),
          "+r"(acc[60]),"+r"(acc[61]),"+r"(acc[62]),"+r"(acc[63]),
          "+r"(acc[64]),"+r"(acc[65]),"+r"(acc[66]),"+r"(acc[67]),
          "+r"(acc[68]),"+r"(acc[69]),"+r"(acc[70]),"+r"(acc[71]),
          "+r"(acc[72]),"+r"(acc[73]),"+r"(acc[74]),"+r"(acc[75]),
          "+r"(acc[76]),"+r"(acc[77]),"+r"(acc[78]),"+r"(acc[79]),
          "+r"(acc[80]),"+r"(acc[81]),"+r"(acc[82]),"+r"(acc[83]),
          "+r"(acc[84]),"+r"(acc[85]),"+r"(acc[86]),"+r"(acc[87]),
          "+r"(acc[88]),"+r"(acc[89]),"+r"(acc[90]),"+r"(acc[91]),
          "+r"(acc[92]),"+r"(acc[93]),"+r"(acc[94]),"+r"(acc[95]),
          "+r"(acc[96]),"+r"(acc[97]),"+r"(acc[98]),"+r"(acc[99]),
          "+r"(acc[100]),"+r"(acc[101]),"+r"(acc[102]),"+r"(acc[103]),
          "+r"(acc[104]),"+r"(acc[105]),"+r"(acc[106]),"+r"(acc[107]),
          "+r"(acc[108]),"+r"(acc[109]),"+r"(acc[110]),"+r"(acc[111]),
          "+r"(acc[112]),"+r"(acc[113]),"+r"(acc[114]),"+r"(acc[115]),
          "+r"(acc[116]),"+r"(acc[117]),"+r"(acc[118]),"+r"(acc[119]),
          "+r"(acc[120]),"+r"(acc[121]),"+r"(acc[122]),"+r"(acc[123]),
          "+r"(acc[124]),"+r"(acc[125]),"+r"(acc[126]),"+r"(acc[127])
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
__device__ __forceinline__ void fence_async_shared(){ asm volatile("fence.proxy.async.shared::cta;\n" ::: "memory"); }

// Per-warpgroup register reallocation (warpgroup-aligned). The producer needs
// few registers, so it releases its surplus; the consumer (128-reg accumulator)
// claims them. Counts must be multiples of 8 in [24,256] and sum, weighted by
// 128 threads/warpgroup, to ≤ the 64K-register SM budget:
// 128*(40 + 216) = 32768, leaving room for 2 CTAs/SM.
#define SET_MAXNREG_DEC(N) asm volatile("setmaxnreg.dec.sync.aligned.u32 %0;\n" :: "n"(N))
#define SET_MAXNREG_INC(N) asm volatile("setmaxnreg.inc.sync.aligned.u32 %0;\n" :: "n"(N))
constexpr int PRODUCER_REGS = 40;
constexpr int CONSUMER_REGS = 216;

constexpr int TM = 64, TK = 1024, KL = TK/64;
constexpr int TILE = TM*KL;        // A tile: 64 rows × 16 u64 = 1024 u64
constexpr int NB = 256;            // n256 output width (columns) per CTA
constexpr int TILE_B = NB*KL;      // B tile: 256 cols × 16 u64 = 4096 u64
constexpr int NG = NB/64;          // 4 output column-limbs per CTA
constexpr int KSUB = TK/256;       // 4 k256 wgmma sub-chunks per loaded tile
constexpr int KSUB_U64 = 256/64;   // 4 u64 = 32 bytes per k256 sub-chunk
constexpr int STAGES = 3;          // K-loop pipeline depth (full/empty buffers)
constexpr int THREADS_PER_WG = 128;
constexpr int GROUP_M = 8;         // M-tiles per rasterization group (L2 reuse knob)
constexpr int CLUSTER = 2;         // CTAs per cluster along M (multicast B; reuse knob)

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
//                                   runs KSUB pipelined m64n256 wgmmas against
//                                   it, signals the stage empty so producer can
//                                   refill.
//
// Dynamic SMEM per CTA (carved from `smem`, 128B-aligned for TMA):
//   sA[STAGES][TILE]    = STAGES * 8192 B
//   sB[STAGES][TILE_B]  = STAGES * 32768 B
//   sC[TM][NG]          = 64 * NG * 8 B (consumer-only, row-major for TMA store)
//   mbar_full[STAGES] + mbar_empty[STAGES]
//
// Per stage = sA (8 KB) + sB (32 KB) = 40 KB; STAGES=3 ≈ 122 KB total (requires
// the opt-in CU_FUNC_ATTRIBUTE_MAX_DYNAMIC_SHARED_SIZE_BYTES set host-side).
// STAGES is the latency-vs-occupancy knob: 2 → 2 CTAs/SM (82 KB), 3 → 1 CTA/SM.
//
// The output tile (64 rows × NG limbs) is packed row-major into sC and written
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
    uint64_t* sA = smem;                          // [STAGES][TILE]
    uint64_t* sB = sA + STAGES * TILE;            // [STAGES][TILE_B]
    uint64_t* sC = sB + STAGES * TILE_B;          // [TM][NG] row-major
    uint64_t* mbar_full  = sC + NG * TM;          // [STAGES]
    uint64_t* mbar_empty = mbar_full + STAGES;    // [STAGES]

    const int t = threadIdx.x;
    const int wg = t / THREADS_PER_WG;        // 0 = producer, 1 = consumer
    const int t_wg = t - wg * THREADS_PER_WG; // 0..127 within warpgroup

    const int nchunks = (K + TK - 1) / TK;
    // One full A tile + one full B tile per stage (B is zero-padded on the
    // host to a multiple of NB columns, so it is always a complete tile). A is
    // loaded per-CTA; B arrives via multicast — both target this CTA's full
    // barrier, so the expected bytes are the same as the single-CTA case.
    const uint32_t expected_tx = (uint32_t)((TILE + TILE_B) * sizeof(uint64_t));

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
    uint32_t qidx = 0, p = 0;
    for (uint32_t ct = cluster_id; ct < total_cl; ct += num_clusters) {
        const uint32_t gid    = ct / (GROUP_M * n_groups);
        const uint32_t firstm = gid * GROUP_M;
        const uint32_t curm   = min((uint32_t)GROUP_M, m_super - firstm);
        const uint32_t local  = ct - gid * GROUP_M * n_groups;
        const uint32_t sbi    = firstm + local % curm;
        const int bj = (int)(local / curm);
        const int bi = (int)(sbi * CLUSTER + rank);  // this CTA's M-tile
        const int row0 = bi * TM, col0 = bj * NG;

        if (t_wg < TM && wg == 1) {
            #pragma unroll
            for (int g = 0; g < NG; ++g) sC[t_wg * NG + g] = 0;
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
                    // A: this CTA's own 64-row tile.
                    tma_2d(&sA[s * TILE], &tma_a, 0,
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
            // One m64n256 accumulator (128 s32 regs/thread), re-zeroed per tile.
            int32_t acc[128];
            #pragma unroll
            for (int r = 0; r < 128; ++r) acc[r] = 0;

            for (int kk = 0; kk < nchunks; ++kk) {
                const uint32_t s = qidx;

                // Wait for the producer's TMAs to finish populating this stage.
                mbar_wait(&mbar_full[s], p);

                // Issue every k256 wgmma for this stage behind one commit/wait so
                // they pipeline. scale-D = 1 accumulates each sub-chunk in-hardware.
                wgmma_fence();
                #pragma unroll
                for (int c = 0; c < KSUB; ++c) {
                    uint64_t da = make_desc(&sA[s * TILE + c * KSUB_U64],
                                            DESC_LBO, DESC_SBO, DESC_SWIZ);
                    uint64_t db = make_desc(&sB[s * TILE_B + c * KSUB_U64],
                                            DESC_LBO, DESC_SBO, DESC_SWIZ);
                    wgmma_n256(acc, da, db);
                }
                wgmma_commit();
                wgmma_wait();
                wgmma_fence();

                // Release this stage cluster-wide: arrive on every CTA's empty
                // barrier (so rank 0 may overwrite their multicast sB).
                if (t_wg < CLUSTER) arrive_cluster(&mbar_empty[s], t_wg);
                if (++qidx == STAGES) { qidx = 0; p ^= 1; }
            }

            // Pack the 256-wide accumulator into sC's NG=4 output limbs. The
            // m64n256 fragment is the m64n64 layout tiled along N: register group
            // gi (0..31) covers output columns [gi*8, gi*8+8); within it this
            // thread owns columns cb, cb+1 for rows rb and rb+8. Column c maps to
            // limb c/64, bit c%64.
            const int wid = t_wg >> 5, lane = t_wg & 31;
            const int rb = wid*16 + (lane>>2), cb = (lane&3)*2;
            uint64_t lo[NG] = {0}, hi[NG] = {0};
            #pragma unroll
            for (int gi = 0; gi < 32; ++gi) {
                int c0 = cb + gi*8, c1 = c0 + 1;
                int l0 = c0 >> 6, b0p = c0 & 63;
                int l1 = c1 >> 6, b1p = c1 & 63;
                lo[l0] |= (uint64_t)(acc[gi*4+0]&1) << b0p;
                lo[l1] |= (uint64_t)(acc[gi*4+1]&1) << b1p;
                hi[l0] |= (uint64_t)(acc[gi*4+2]&1) << b0p;
                hi[l1] |= (uint64_t)(acc[gi*4+3]&1) << b1p;
            }
            // Row-major sC[row*NG + limb]; padded limbs (out-of-range columns) get
            // zero popcounts from the zero-padded B, so they store harmless zeros
            // into C's padded region (trimmed on the host).
            #pragma unroll
            for (int g = 0; g < NG; ++g) {
                uint32_t* clo = reinterpret_cast<uint32_t*>(&sC[rb * NG + g]);
                uint32_t* chi = reinterpret_cast<uint32_t*>(&sC[(rb + 8) * NG + g]);
                atomicXor(&clo[0], (uint32_t)lo[g]);
                atomicXor(&clo[1], (uint32_t)(lo[g]>>32));
                atomicXor(&chi[0], (uint32_t)hi[g]);
                atomicXor(&chi[1], (uint32_t)(hi[g]>>32));
            }
        }

        // Write the 64×NG output tile back with a single TMA bulk store.
        __syncthreads();        // sC fully packed by the consumer
        fence_async_shared();   // make the atomicXor writes visible to the async proxy
        if (t == 0) {
            tma_store_2d(&tma_c, col0 * 2, row0, sC); // x in UINT32 units (2 per limb)
            tma_store_commit();
            tma_store_wait();
        }
        __syncthreads();        // keep sC alive until the store completes, and
                                // fence this tile before the next reuses SMEM
    }
}
