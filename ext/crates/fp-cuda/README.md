# fp-cuda

CUDA backend for the F₂ matrix multiplication implemented in `crates/fp/src/blas/`.
The Hopper memory pipeline is used end-to-end: kernel written in CUDA C++ with
inline PTX for **TMA bulk tensor loads** with **128B swizzle**
(`cp.async.bulk.tensor.2d`), **mbarrier**-based completion sync, and the binary
tensor cores
(`wgmma.mma_async.sync.aligned.m64n64k256.row.col.s32.b1.b1.and.popc`).
Both operands are pre-arranged into plain row-major K-major tiles on the host;
the TMA applies the swizzle that the wgmma matrix descriptors expect.
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

The full Phase 3 pipeline (host row-major pre-arrangement → TMA 128B-swizzle
loads → mbarrier sync → pipelined wgmma.b1 → bit-pack) compiles and is wired
end-to-end; the host-side `CUtensorMap` build matches the kernel's `boxDim` and
the swizzle mode. The most recent change (128B swizzle + wgmma pipelining) has
**not yet been re-validated on hardware** — verify in this order:

1. **64×256×64 identity product first.** Smallest path that exercises one
   swizzled tile end-to-end. A failure here points at the swizzled wgmma
   descriptor constants (`DESC_LBO = 16`, `DESC_SBO = 1024`, per-k256 advance
   of 32 bytes), or a host-layout / TMA-box mismatch. These derive from
   CUTLASS `make_gmma_desc<Major::K>` (`LayoutType::B128`) but are not
   hardware-checked here.
2. **Full size sweep** via `cargo run -p fp-cuda --example matmul_b1_demo`
   (bit-exact CPU↔GPU for 64…8192).
3. **Bench** with `cargo bench -p fp-cuda`; compare binary TOPS against the
   ~100 TOPS pre-swizzle baseline and confirm outputs stay bit-equal.

Other points worth a trace once: the dynamic-SMEM base must be 128-byte aligned
for TMA (declared `extern __shared__ __align__(128)`), and the per-stage
`expect_tx = (1 + active_ng) * 8192` bytes must match exactly one
`cp.async.bulk.tensor.complete_tx::bytes` notification per issued TMA.

## Phase 3 roadmap

Done (Phase 3): **128B swizzle** on both operands — the TMA loads with
`CU_TENSOR_MAP_SWIZZLE_128B` and the wgmma matrix descriptors set
`layout_type = 1` (LBO = 16 B, SBO = 1024 B), so operand reads avoid bank
conflicts. The SMEM K-tile was grown to 1024 bits (a full 128B K-major swizzle
atom = 4 k256 sub-chunks) and moved to dynamic shared memory (opt-in via
`CU_FUNC_ATTRIBUTE_MAX_DYNAMIC_SHARED_SIZE_BYTES`). The per-stage wgmmas now run
behind a single `commit_group`/`wait_group` and accumulate in-hardware
(`scale-D = 1`) into one resident accumulator per column group, instead of
serializing each wgmma behind its own `commit`/`wait`. The host pre-arrangement
is now plain row-major tiles (the hand-rolled `cm()` interleave is gone).

Remaining:

- Try larger wgmma shapes (`m64n128k256`, `m64n256k256`) for higher
  accumulator reuse per instruction (requires re-deriving the fragment →
  output bit-pack for the wider N).
- Migrate the output write to TMA bulk store
  (`cp.async.bulk.tensor.2d.global.shared::cta`).
- Add a `cuda` feature on the `fp` crate that pulls in `fp-cuda` and
  inserts a runtime device check at the top of `impl Mul for &Matrix`,
  dispatching to the GPU for matrices above a size threshold and keeping
  operands resident on the device across `step_resolution`'s successive
  multiplications.
- Extend the parent Nix flake to provide nvcc when the user opts in.
