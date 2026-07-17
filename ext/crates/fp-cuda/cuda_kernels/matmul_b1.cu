// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Hopper wgmma.b1 F_2 GEMM kernel — Phase 2 with TMA swizzle for A.
//
// A is pre-interleaved on the host (4 rows per 128-byte block) and loaded
// via TMA cp.async.bulk.tensor.2d with CU_TENSOR_MAP_SWIZZLE_128B directly
// into the CM-blocked SMEM layout wgmma expects — zero thread stores.
//
// B is pre-transposed + CM-blocked on the host and loaded via straight
// global memcpy.

#include <cstdint>
#include <cuda_runtime.h>
#include <cuda.h>

// ── Helpers ─────────────────────────────────────────────────────────────────

__device__ __forceinline__ uint64_t make_desc(
    const void* p, uint32_t lead, uint32_t stride, uint32_t swiz) {
    uint32_t a = (uint32_t)__cvta_generic_to_shared(p);
    uint64_t d = 0;
    d |= ((uint64_t)a >> 4) & 0x3FFFULL;
    d |= ((uint64_t)(lead   >> 4) & 0x3FFFULL) << 16;
    d |= ((uint64_t)(stride >> 4) & 0x3FFFULL) << 32;
    (void)swiz;
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

__device__ __forceinline__ void wgmma_go  (int32_t acc[32], uint64_t da, uint64_t db) { WGMMA_B1(0); }
__device__ __forceinline__ void wgmma_fence()  { asm volatile("wgmma.fence.sync.aligned;\n" ::: "memory"); }
__device__ __forceinline__ void wgmma_commit() { asm volatile("wgmma.commit_group.sync.aligned;\n" ::: "memory"); }
__device__ __forceinline__ void wgmma_wait()   { asm volatile("wgmma.wait_group.sync.aligned 0;\n" ::: "memory"); }

constexpr int TM = 64, TK = 256, KL = TK/64, TILE = TM*KL; // 256 u64s
constexpr int NG = 4;

// ── Kernel ──────────────────────────────────────────────────────────────────

extern "C" __global__ void matmul_b1_kernel(
    const __grid_constant__ CUtensorMap tma_a,
    uint32_t m_tiles,
    const uint64_t* __restrict__ Bt,
    uint32_t M, uint32_t K, uint32_t nlim,
    uint64_t* __restrict__ C)
{
    __shared__ alignas(128) uint64_t sA[TILE];   // 2048 B — filled by TMA
    __shared__ alignas(128) uint64_t sB[TILE];   // 2048 B — filled by threads
    __shared__ uint64_t sC[NG][TM];
    __shared__ alignas(8) uint64_t mbar[1];

    const int bi = blockIdx.y, bj = blockIdx.x, t = threadIdx.x;
    const int row0 = bi * TM, col0 = bj * NG;
    if (row0 >= (int)M) return;

    if (t == 0) mbar_init(mbar, 1);
    for (int g = 0; g < NG; ++g)
        if (t < TM) sC[g][t] = 0;
    __syncthreads();

    const int nchunks = (K + TK - 1) / TK;

    for (int g = 0; g < NG; ++g) {
        int col = col0 + g;
        if (col >= (int)nlim) continue;

        int32_t tot[32];
        #pragma unroll
        for (int r = 0; r < 32; ++r) tot[r] = 0;

        for (int kk = 0; kk < nchunks; ++kk) {
            uint32_t phase = kk & 1;

            // TMA load A (thread 0 only, all others just wait).
            if (t == 0) {
                mbar_tx(mbar, 2048);
                tma_2d(sA, &tma_a, 0, (kk * m_tiles + bi) * 16, mbar);
            }

            // Load pre-transposed B tile (all threads).
            const uint64_t* tile = &Bt[(kk * nlim + col) * TILE];
            for (int i = t; i < TILE; i += blockDim.x)
                sB[i] = tile[i];

            // Wait for TMA to finish writing sA.
            mbar_wait(mbar, phase);
            // Ensure thread stores to sB are visible to wgmma async proxy.
            __syncthreads();
            asm volatile("fence.proxy.async.shared::cta;\n" ::: "memory");

            // Fire wgmma. A uses 128B swizzle (layout_type=1), B uses none.
            int32_t acc[32];
            #pragma unroll
            for (int r = 0; r < 32; ++r) acc[r] = 0;
            uint64_t da = make_desc(sA, 128, 256, 0);
            uint64_t db = make_desc(sB, 128, 256, 0);  // no swizzle
            wgmma_fence();
            wgmma_go(acc, da, db);
            wgmma_commit();
            wgmma_wait();
            wgmma_fence();

            #pragma unroll
            for (int r = 0; r < 32; ++r) tot[r] += acc[r];
        }

        // Pack this column group's output.
        const int wid = t >> 5, lane = t & 31;
        const int rb = wid*16 + (lane>>2), cb = (lane&3)*2;
        uint64_t b0 = 0, b8 = 0;
        #pragma unroll
        for (int gi = 0; gi < 8; ++gi) {
            int c0 = cb + gi*8, c1 = c0+1;
            b0 |= (uint64_t)(tot[gi*4+0]&1) << c0;
            b0 |= (uint64_t)(tot[gi*4+1]&1) << c1;
            b8 |= (uint64_t)(tot[gi*4+2]&1) << c0;
            b8 |= (uint64_t)(tot[gi*4+3]&1) << c1;
        }
        uint32_t* c32 = reinterpret_cast<uint32_t*>(sC[g]);
        atomicXor(&c32[rb*2],     (uint32_t)b0);
        atomicXor(&c32[rb*2+1],   (uint32_t)(b0>>32));
        atomicXor(&c32[(rb+8)*2], (uint32_t)b8);
        atomicXor(&c32[(rb+8)*2+1],(uint32_t)(b8>>32));
    }
    __syncthreads();

    for (int g = 0; g < NG; ++g) {
        int col = col0 + g;
        if (t < TM && row0+t < (int)M && col < (int)nlim)
            C[(row0+t)*nlim + col] = sC[g][t];
    }
}
