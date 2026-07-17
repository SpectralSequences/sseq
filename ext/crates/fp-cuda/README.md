# fp-cuda

CUDA backend for the F₂ matrix multiplication implemented in `crates/fp/src/blas/`.
The Hopper memory pipeline is used end-to-end: kernel written in CUDA C++ with
inline PTX for **TMA bulk tensor loads** (`cp.async.bulk.tensor.2d`),
**mbarrier**-based completion sync, a warp-level `__ballot_sync` bit-transpose
for B, and the binary tensor cores
(`wgmma.mma_async.sync.aligned.m64n64k256.row.col.s32.b1.b1.s32.and.popc`).
Rust-side glue uses [NVlabs/cuda-oxide](https://github.com/NVlabs/cuda-oxide)'s
`cuda-core` crate for the host driver-API surface (untyped module loading +
raw kernel launch) and its `sys` re-export of `cuda-bindings` for the
`cuTensorMapEncodeTiled` call that builds the TMA descriptors.

This crate is **excluded from the workspace's `default-members`**, so plain
`cargo build` / `nix run .#test` ignore it. It is opt-in: building requires
nvcc on `PATH` and (at runtime) a Hopper-class GPU.

## Prerequisites

1. **nvcc** (CUDA Toolkit 12.x+, since TMA + wgmma require 12.0+) on `PATH`,
   with Hopper (sm_90a) support. Override the binary location with the
   `NVCC` env var if needed.
2. A **Hopper or newer GPU** at runtime (sm_90 / sm_90a / sm_100). PTX load
   will fail on pre-Hopper devices because the kernel emits `wgmma.*` and
   `cp.async.bulk.tensor.*` instructions that only exist on sm_90+.

No cargo-oxide CLI, no special Rust toolchain, no LLVM 21 — `cuda-core`
compiles with stable rustc.

## Building

```bash
# From the workspace root; the leading -p selects this crate explicitly.
cargo build -p fp-cuda
```

`build.rs` invokes nvcc on `cuda_kernels/matmul_b1.cu` and emits
`matmul_b1.ptx` into the cargo `OUT_DIR`. `src/lib.rs` embeds it via
`include_bytes!` and loads it at runtime through `cuda-core`'s
`CudaContext::load_module_from_image`.

## Running

```bash
# Smoke test (multiplies a few small shapes, asserts CPU↔GPU equality):
cargo run -p fp-cuda --example matmul_b1_demo

# Benchmark against the CPU AVX-512 path in fp::blas:
cargo bench -p fp-cuda
```

The bench compares each square size in `{128, 256, 512, 1024, 2048, 4096, 8192}`
against `fp::blas::fast_mul_concurrent`, asserts bit-equality of the outputs,
and prints binary TOPS for both backends.

## Why excluded from `default-members`?

Contributors without nvcc would otherwise see this crate's build fail every
time they run `cargo build`. Keeping it out of the default member set means:

- `cargo build`, `cargo test`, `cargo fmt`, `nix run .#test` from the
  workspace root behave exactly as before.
- Tooling that wants this crate explicitly opts in with `-p fp-cuda`.

The crate is still a workspace **member**, so `cargo metadata` sees it,
`rust-analyzer` indexes it, and shared dependency resolution works.

## Status

Phase 1 is **structurally complete but untested on hardware**. The wgmma
pipeline (TMA + mbarrier + warp-shuffle transpose + wgmma + bit-pack) is all
wired up; the host-side `CUtensorMap` build matches the kernel's `boxDim`.
Calibration points needing on-hardware verification before the bench will
pass bit-equality:

1. **wgmma SMEM descriptor `leading_dim` / `stride`** — currently encoded
   with swizzle=0 and `leading = stride = 32` bytes. PTX manual §9.7.13.2
   has worked examples for the swizzle-0 case; CUTLASS's
   `cute::SM90_64x64x256_S32_TN_B1B1` atom is the canonical reference.
2. **Per-thread accumulator → output bit mapping** — derived from the PTX
   manual's "Matrix Fragments for WGMMA" for `m64n64.s32`. Verify with a
   64×64 identity-matrix product before benching larger sizes.
3. **mbarrier transaction-count semantics** — kernel uses
   `expect_tx = TMA_BYTES_A + TMA_BYTES_B` (4096 bytes). Confirm both TMA
   loads finalize a single `cp.async.bulk.tensor.complete_tx::bytes`
   notification each, not a different multiple.
4. **`__ballot_sync` B-transpose** — the `atomicOr` write-back in
   `transpose_b_warp` assumes disjoint `(dst_idx, shift)` regions across
   warps; this holds for the two-pass layout but is worth tracing once.

## Phase 1.5 / Phase 2 roadmap

- Switch TMA + SMEM descriptors to `CU_TENSOR_MAP_SWIZZLE_128B` to
  eliminate bank conflicts on the wgmma operand reads. Likely requires
  growing the SMEM tile to keep 128-byte alignment along the innermost
  dim (e.g. 16 u64s per row for the A tile, expanding the per-CTA output
  to 16 column-limbs and 16 wgmma instructions per K chunk).
- Replace single-buffered SMEM tiles with **double-buffered** TMA loads
  so the next K-chunk transfers overlap with the current wgmma.
- Try larger wgmma shapes (`m64n128k256`, `m64n256k256`) for higher
  accumulator reuse per instruction.
- Migrate the output write to TMA bulk store
  (`cp.async.bulk.tensor.2d.global.shared::cta`).
- Add a `cuda` feature on the `fp` crate that pulls in `fp-cuda` and
  inserts a runtime device check at the top of `impl Mul for &Matrix`,
  dispatching to the GPU for matrices above a size threshold and keeping
  operands resident on the device across `step_resolution`'s successive
  multiplications.
- Extend the parent Nix flake to provide nvcc when the user opts in.
