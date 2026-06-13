// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Hopper wgmma.b1 F_2 GEMM kernel — Phase 3: 128B-swizzle operands + pipelined
// wgmmas.
//
// Both operands are K-major. They are pre-arranged on the host as plain
// row-major tiles (64 rows × 1024 bits = 64 × 128 bytes) and loaded via TMA
// cp.async.bulk.tensor.2d with CU_TENSOR_MAP_SWIZZLE_128B: the TMA hardware
// applies the 128B swizzle on the way into SMEM, landing the data exactly where
// the swizzled wgmma matrix descriptor expects it — so the host emits the
// natural layout and there is no hand-rolled interleave.
//
// Each loaded tile spans a full 128B K-major swizzle atom (8 rows × 1024 bits),
// i.e. KSUB = 4 consecutive k256 sub-chunks. The consumer issues all
// NG × KSUB wgmmas for a stage behind a single commit/wait and accumulates the
// popcounts in-hardware (scale-D = 1) into one accumulator per column group that
// stays resident across the whole K loop.

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
__device__ __forceinline__ void mbar_arrive(uint64_t* b) {
    asm volatile("mbarrier.arrive.shared::cta.b64 _, [%0];\n"
        :: "r"((uint32_t)__cvta_generic_to_shared(b)) : "memory");
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

// scale-D = SD: 0 overwrites the accumulator, 1 accumulates into it.
#define WGMMA_B1(SD) \
    asm volatile( \
        "wgmma.mma_async.sync.aligned.m64n64k256.row.col.s32.b1.b1.and.popc " \
        "{%0,%1,%2,%3,%4,%5,%6,%7,%8,%9,%10,%11,%12,%13,%14,%15," \
        "%16,%17,%18,%19,%20,%21,%22,%23,%24,%25,%26,%27,%28,%29,%30,%31}," \
        "%32,%33," #SD ";\n" \
        : "+r"(acc[0]),"+r"(acc[1]),"+r"(acc[2]),"+r"(acc[3]), \
          "+r"(acc[4]),"+r"(acc[5]),"+r"(acc[6]),"+r"(acc[7]), \
          "+r"(acc[8]),"+r"(acc[9]),"+r"(acc[10]),"+r"(acc[11]), \
          "+r"(acc[12]),"+r"(acc[13]),"+r"(acc[14]),"+r"(acc[15]), \
          "+r"(acc[16]),"+r"(acc[17]),"+r"(acc[18]),"+r"(acc[19]), \
          "+r"(acc[20]),"+r"(acc[21]),"+r"(acc[22]),"+r"(acc[23]), \
          "+r"(acc[24]),"+r"(acc[25]),"+r"(acc[26]),"+r"(acc[27]), \
          "+r"(acc[28]),"+r"(acc[29]),"+r"(acc[30]),"+r"(acc[31]) \
        : "l"(da), "l"(db))

// Accumulating wgmma (scale-D = 1); accumulators are pre-zeroed by the consumer.
__device__ __forceinline__ void wgmma_go(int32_t acc[32], uint64_t da, uint64_t db) { WGMMA_B1(1); }
__device__ __forceinline__ void wgmma_fence()  { asm volatile("wgmma.fence.sync.aligned;\n" ::: "memory"); }
__device__ __forceinline__ void wgmma_commit() { asm volatile("wgmma.commit_group.sync.aligned;\n" ::: "memory"); }
__device__ __forceinline__ void wgmma_wait()   { asm volatile("wgmma.wait_group.sync.aligned 0;\n" ::: "memory"); }

constexpr int TM = 64, TK = 1024, KL = TK/64, TILE = TM*KL; // 1024 u64s per tile
constexpr int KSUB = TK/256;       // 4 k256 wgmma sub-chunks per loaded tile
constexpr int NG = 4;
constexpr int STAGES = 2;          // K-loop pipeline depth (full/empty buffers)
constexpr int THREADS_PER_WG = 128;

// wgmma 128B K-major descriptor constants (CUTLASS make_gmma_desc<Major::K>,
// LayoutType::B128): LBO = 1 uint128 = 16 bytes, SBO = 8-row-brick stride =
// 1024 bytes, swizzle = 1. A k256 sub-chunk c sits at byte offset c*32 within
// the tile (advance start_address; the hardware re-applies the swizzle).
constexpr uint32_t DESC_LBO = 16;
constexpr uint32_t DESC_SBO = 1024;
constexpr uint32_t DESC_SWIZ = 1;
constexpr int KSUB_U64 = 256/64;   // 4 u64 = 32 bytes per k256 sub-chunk

// ── Kernel ──────────────────────────────────────────────────────────────────

// Producer-consumer kernel: 2 warpgroups (256 threads/CTA).
//   Warpgroup 0 (t in [0, 128))  = PRODUCER: issues TMA loads in a tight
//                                  K-loop into a STAGES-deep circular SMEM
//                                  buffer.
//   Warpgroup 1 (t in [128, 256)) = CONSUMER: waits for each stage to be full,
//                                   runs NG×KSUB pipelined wgmmas against it,
//                                   signals the stage empty so producer can
//                                   refill.
//
// Dynamic SMEM per CTA (carved from `smem`, 128B-aligned for TMA):
//   sA[STAGES][TILE]       = STAGES * 8192 B
//   sB[STAGES][NG][TILE]   = STAGES * NG * 8192 B
//   sC[NG][TM]             = NG * 64 * 8 B (consumer-only)
//   mbar_full[STAGES] + mbar_empty[STAGES]
//
// With STAGES=2 and NG=4:  16 KB + 64 KB + 2 KB + 32 B ≈ 82 KB (requires the
// opt-in CU_FUNC_ATTRIBUTE_MAX_DYNAMIC_SHARED_SIZE_BYTES set host-side).
extern "C" __global__ void matmul_b1_kernel(
    const __grid_constant__ CUtensorMap tma_a,
    const __grid_constant__ CUtensorMap tma_b,
    uint32_t m_tiles,
    uint32_t M, uint32_t K, uint32_t nlim,
    uint64_t* __restrict__ C)
{
    extern __shared__ __align__(128) uint64_t smem[];
    uint64_t* sA = smem;                          // [STAGES][TILE]
    uint64_t* sB = sA + STAGES * TILE;            // [STAGES][NG][TILE]
    uint64_t* sC = sB + STAGES * NG * TILE;       // [NG][TM]
    uint64_t* mbar_full  = sC + NG * TM;          // [STAGES]
    uint64_t* mbar_empty = mbar_full + STAGES;    // [STAGES]

    const int bi = blockIdx.y, bj = blockIdx.x, t = threadIdx.x;
    const int row0 = bi * TM, col0 = bj * NG;
    if (row0 >= (int)M) return;

    const int wg = t / THREADS_PER_WG;      // 0 = producer, 1 = consumer
    const int t_wg = t - wg * THREADS_PER_WG; // 0..127 within warpgroup

    // How many of the NG column groups are in-bounds for this CTA?
    int active_ng = 0;
    #pragma unroll
    for (int g = 0; g < NG; ++g) {
        if (col0 + g < (int)nlim) ++active_ng;
    }

    if (t == 0) {
        #pragma unroll
        for (int s = 0; s < STAGES; ++s) {
            mbar_init(&mbar_full[s], 1);
            mbar_init(&mbar_empty[s], 1);
            // Pre-arrive each empty barrier so the producer's first
            // `mbar_wait(empty, 0)` succeeds immediately — the stage is
            // logically "free" before iteration 0.
            mbar_arrive(&mbar_empty[s]);
        }
    }
    if (t_wg < TM && wg == 1) {
        #pragma unroll
        for (int g = 0; g < NG; ++g) sC[g * TM + t_wg] = 0;
    }
    __syncthreads();

    const int nchunks = (K + TK - 1) / TK;
    const uint32_t tile_bytes = (uint32_t)(TILE * sizeof(uint64_t)); // 8192
    const uint32_t expected_tx = (1 + active_ng) * tile_bytes; // A + active Bs

    if (wg == 0) {
        // ===================== PRODUCER =====================
        uint32_t phase_empty[STAGES] = {0, 0};

        for (int kk = 0; kk < nchunks; ++kk) {
            const int s = kk % STAGES;

            // Wait for the consumer to release this stage. Pre-arrival in
            // the init block makes the first STAGES iterations no-wait.
            if (t_wg == 0) {
                mbar_wait(&mbar_empty[s], phase_empty[s]);
            }
            phase_empty[s] ^= 1;

            // Set expected transaction bytes for this stage's full barrier
            // and issue all the TMAs (A + the active B's). Each tile is a
            // 64-row × 128-byte box loaded with 128B swizzle.
            if (t_wg == 0) {
                mbar_tx(&mbar_full[s], expected_tx);
                tma_2d(&sA[s * TILE], &tma_a, 0,
                       (kk * m_tiles + bi) * TM, &mbar_full[s]);
                #pragma unroll
                for (int g = 0; g < NG; ++g) {
                    int col = col0 + g;
                    if (col < (int)nlim) {
                        tma_2d(&sB[(s * NG + g) * TILE], &tma_b, 0,
                               (kk * nlim + col) * TM, &mbar_full[s]);
                    }
                }
            }
        }
    } else {
        // ===================== CONSUMER =====================
        uint32_t phase_full[STAGES] = {0, 0};

        // One accumulator per column group, resident across the whole K loop.
        // Pre-zeroed so every wgmma can use scale-D = 1 (accumulate). ~128 s32
        // regs/thread.
        int32_t acc[NG][32];
        #pragma unroll
        for (int g = 0; g < NG; ++g) {
            #pragma unroll
            for (int r = 0; r < 32; ++r) acc[g][r] = 0;
        }

        for (int kk = 0; kk < nchunks; ++kk) {
            const int s = kk % STAGES;

            // Wait for the producer's TMAs to finish populating this stage.
            mbar_wait(&mbar_full[s], phase_full[s]);
            phase_full[s] ^= 1;

            // Issue every wgmma for this stage behind one commit/wait so they
            // pipeline. scale-D = 1 accumulates each k256 sub-chunk in-hardware.
            wgmma_fence();
            #pragma unroll
            for (int g = 0; g < NG; ++g) {
                int col = col0 + g;
                if (col >= (int)nlim) continue;
                #pragma unroll
                for (int c = 0; c < KSUB; ++c) {
                    uint64_t da = make_desc(&sA[s * TILE + c * KSUB_U64],
                                            DESC_LBO, DESC_SBO, DESC_SWIZ);
                    uint64_t db = make_desc(&sB[(s * NG + g) * TILE + c * KSUB_U64],
                                            DESC_LBO, DESC_SBO, DESC_SWIZ);
                    wgmma_go(acc[g], da, db);
                }
            }
            wgmma_commit();
            wgmma_wait();
            wgmma_fence();

            // Signal that this stage's SMEM can be reused.
            if (t_wg == 0) mbar_arrive(&mbar_empty[s]);
        }

        // Pack each column group's accumulator into sC. Layout uses the
        // warpgroup-local thread id since this is consumer-only.
        const int wid = t_wg >> 5, lane = t_wg & 31;
        const int rb = wid*16 + (lane>>2), cb = (lane&3)*2;
        #pragma unroll
        for (int g = 0; g < NG; ++g) {
            int col = col0 + g;
            if (col >= (int)nlim) continue;

            uint64_t b0 = 0, b8 = 0;
            #pragma unroll
            for (int gi = 0; gi < 8; ++gi) {
                int c0 = cb + gi*8, c1 = c0+1;
                b0 |= (uint64_t)(acc[g][gi*4+0]&1) << c0;
                b0 |= (uint64_t)(acc[g][gi*4+1]&1) << c1;
                b8 |= (uint64_t)(acc[g][gi*4+2]&1) << c0;
                b8 |= (uint64_t)(acc[g][gi*4+3]&1) << c1;
            }
            uint32_t* c32 = reinterpret_cast<uint32_t*>(&sC[g * TM]);
            atomicXor(&c32[rb*2],       (uint32_t)b0);
            atomicXor(&c32[rb*2+1],     (uint32_t)(b0>>32));
            atomicXor(&c32[(rb+8)*2],   (uint32_t)b8);
            atomicXor(&c32[(rb+8)*2+1], (uint32_t)(b8>>32));
        }
    }

    // Both warpgroups meet here before the global write.
    __syncthreads();

    // Consumer's first TM threads write the output rows back to global.
    if (wg == 1 && t_wg < TM) {
        #pragma unroll
        for (int g = 0; g < NG; ++g) {
            int col = col0 + g;
            if (row0 + t_wg < (int)M && col < (int)nlim)
                C[(row0 + t_wg) * nlim + col] = sC[g * TM + t_wg];
        }
    }
}
