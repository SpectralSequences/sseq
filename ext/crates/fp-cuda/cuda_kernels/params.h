// Tuning knobs shared by the CUDA kernel and the Rust host code.
//
// This is the single source of truth: `matmul_b1.cu` includes this header and derives its
// `constexpr`s from it, while `build.rs` parses the `#define`s and emits the Rust mirror that
// `src/lib.rs` reads. Everything else in either language is derived, so the two cannot drift.
//
// Only `#define <NAME> <integer>` lines are parsed; keep the values plain decimal integers.

#pragma once

// m64 row-strips per CTA (block knob). Each k256 step issues MSTRIPS m64n128 wgmmas that share one
// B sub-tile: 2 → 128×128 block (−20% bytes/MAC, 128 acc regs), 3 → 192×128 (−33%, 192).
// Accumulator regs per thread = MSTRIPS*ACC_N ≤ 240; see EXPERIMENTS.md for why we spend the
// register file on the accumulator rather than on a second resident CTA.
#define MSTRIPS 3

// wgmma M extent, fixed for binary wgmma.
#define MW 64

// K bits per loaded tile.
#define TK 1024

// n128 output width (columns) per CTA, fixed by the wgmma_n128 shape.
#define NB 128

// K-loop pipeline depth (full/empty buffers). Capped at 4; see EXPERIMENTS.md.
#define STAGES 4

// M-tiles per rasterization group (L2 reuse knob).
#define GROUP_M 16

// Threads per warpgroup. A CTA runs two: producer and consumer.
#define THREADS_PER_WG 128

// Cross-check of the Rust mirror. `build.rs` re-exports every knob it parsed out of this file as
// -DFP_CUDA_VERIFY_<NAME> and compiles with -DFP_CUDA_VERIFY, so the C++ front end — the only thing
// that authoritatively knows what these macros expand to — decides whether the parse was right. A
// misread value fails the static_assert; a knob the parser skipped entirely leaves
// FP_CUDA_VERIFY_<NAME> undefined and fails to compile. Either way the build stops here rather than
// shipping a host that disagrees with the kernel.
//
// The list below is the authoritative set of shared knobs: add a check whenever you add a knob.
#ifdef FP_CUDA_VERIFY
#define FP_CUDA_CHECK(name) \
    static_assert((name) == (FP_CUDA_VERIFY_##name), "build.rs misread " #name " from params.h")
FP_CUDA_CHECK(MSTRIPS);
FP_CUDA_CHECK(MW);
FP_CUDA_CHECK(TK);
FP_CUDA_CHECK(NB);
FP_CUDA_CHECK(STAGES);
FP_CUDA_CHECK(GROUP_M);
FP_CUDA_CHECK(THREADS_PER_WG);
#undef FP_CUDA_CHECK
#endif
