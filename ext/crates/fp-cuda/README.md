# fp-cuda

CUDA backend for the F₂ matrix multiplication implemented in `crates/fp/src/blas/`.
The Hopper memory pipeline is used end-to-end: kernel written in CUDA C++ with
inline PTX for **TMA bulk tensor loads** with **128B swizzle**
(`cp.async.bulk.tensor.2d`), **mbarrier**-based completion sync, and the binary
tensor cores
(`wgmma.mma_async.sync.aligned.m64n64k256.row.col.s32.b1.b1.and.popc`).
Both operands are pre-arranged into plain row-major K-major tiles on the host;
the TMA applies the swizzle that the wgmma matrix descriptors expect.
Rust-side glue uses [`cudarc`](https://crates.io/crates/cudarc) for the host
driver-API surface (module load, device buffers, typed launch) and its
`driver::sys` raw bindings for the `cuTensorMapEncodeTiled` call that builds the
TMA descriptors. `cudarc` is stable Rust and dynamically loads the CUDA driver
at runtime, so the Rust side builds with no CUDA present.

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

Builds on **stable** Rust — no nightly toolchain required. (`nvcc` is still
needed at build time to compile the kernel to PTX, and a CUDA driver at runtime.)

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

The full Phase 3–7 pipeline (host row-major pre-arrangement → TMA 128B-swizzle
loads → mbarrier sync → pipelined `m64n256k256` wgmma.b1 → bit-pack → TMA bulk
output store) is **validated on an H100 NVL (sm_90, CUDA 13.0 driver / 12.8
toolkit, 2026-06-15)**. The PTX JITs at module load, the dynamic-SMEM opt-in and
all three TMA descriptors are accepted, and outputs are **bit-exact** against the
CPU `fp::blas` path across `matmul_b1_demo` (64…8192) and the kernel-only bench
(4096…32768, including a full 32768³ CPU cross-check).

Throughput, **kernel-only** (host setup + H2D/D2H excluded — the comparison the
~100-TOPS pre-swizzle baseline was measured at):

| size (M=K=N) | binary TOPS | ms/launch |
|--------------|-------------|-----------|
| 4096         | ~3,600      | 0.038     |
| 8192         | ~5,200      | 0.211     |
| 16384        | ~5,800      | 1.52      |
| 32768        | ~2,200      | 32.1      |

i.e. roughly a **50–58× kernel speedup** over the ~100-TOPS pre-swizzle state.

The drop past 16384 is **not** a power/compute bound (measured: 136 W of the
310 W cap, SM 0–12 %, memory clock pinned at max) — the kernel is
**memory-bandwidth bound on L2 residency of B**. Each B column-panel is reused
across every M-tile, so the whole B matrix (`K*N/8` bytes) wants to fit in the
50 MB L2: at 16384² B is 33.6 MB (fits, ~5,800 TOPS), at 32768² it is 134 MB
(spills → re-streamed from HBM per M-tile → ~2,300 TOPS). `bench_shapes`
confirms this with equal-FLOPs shapes: M=65536/K=N=16384 (B fits) hits 5,386
TOPS while M=16384/K=16384/N=65536 (same FLOPs, B spills) gets 2,272 TOPS, and
M=131072 (8× the FLOPs, B still fits) sustains 5,275 TOPS — so total size is not
the limiter, L2 residency is. This is exactly what the remaining rungs target:
**persistent kernel + tile rasterization** (keep the active tile working set in
L2 at large N) and **clusters + TMA multicast** (one HBM read of a B-panel feeds
a whole cluster). Run `cargo run --release -p fp-cuda --example bench_shapes` to
reproduce.

The end-to-end `cargo bench` figures (≤30 TOPS) are dominated by host
serialization and the TMA-layout pre-arrangement; use `cargo run --release -p
fp-cuda --example bench_kernel_only` for the kernel number.

Reproduce in this order (each gates the next):

1. **64×256×64 identity / small product first.** Smallest path that exercises one
   swizzled tile end-to-end (the first `matmul_b1_demo` case). A failure here
   points at the swizzled wgmma descriptor constants (`DESC_LBO = 16`,
   `DESC_SBO = 1024`, per-k256 advance of 32 bytes), or a host-layout / TMA-box
   mismatch. These derive from CUTLASS `make_gmma_desc<Major::K>`
   (`LayoutType::B128`).
2. **Full size sweep** via `cargo run -p fp-cuda --example matmul_b1_demo`
   (bit-exact CPU↔GPU for 64…8192).
3. **Kernel-only throughput + correctness** via
   `cargo run --release -p fp-cuda --example bench_kernel_only`; compare binary
   TOPS against the ~100-TOPS pre-swizzle baseline.

Other points worth a trace once: the dynamic-SMEM base must be 128-byte aligned
for TMA (declared `extern __shared__ __align__(128)`), and the per-stage
`expect_tx = (TILE + TILE_B) * 8` bytes (one A tile + one B tile) must match
the `cp.async.bulk.tensor.complete_tx::bytes` notifications from the two issued
TMA loads.

## Roadmap

Following the optimization ladder of Pranjal Shankhdhar's "Outperforming cuBLAS
on H100" worklog, adapted to the binary (`b1`) GF(2) kernel.

Done:

- **128B swizzle** (Phase 3) on both operands — TMA loads with
  `CU_TENSOR_MAP_SWIZZLE_128B`; wgmma descriptors set `layout_type = 1`
  (LBO = 16 B, SBO = 1024 B), avoiding bank conflicts. The SMEM K-tile is 1024
  bits (a full 128B K-major swizzle atom = 4 k256 sub-chunks); per-stage wgmmas
  run behind one `commit_group`/`wait_group` and accumulate in-hardware
  (`scale-D = 1`). Operands moved to dynamic shared memory. Host pre-arrangement
  is plain row-major tiles (no `cm()` interleave).
- **Widest binary MMA** (Phase 4) — one `m64n256k256` per k-step covering all
  NG = 4 output limbs, replacing four `m64n64k256`. Same registers/SMEM, 1/4 the
  instructions; B is one 256-column tile per CTA.
- **Register reallocation** (Phase 5) — `setmaxnreg.dec(40)` in the producer,
  `setmaxnreg.inc(216)` in the consumer.
- **Deeper pipeline** (Phase 6) — `STAGES = 3` (latency-vs-occupancy knob).
- **TMA output store** (Phase 7) — the packed `sC` tile is written back with a
  single `cp.async.bulk.tensor.2d.global.shared::cta`; C is padded to whole
  NG-limb column groups so every stored tile is complete.

Remaining:

- Thread-block **clusters + TMA multicast** to share operand loads across CTAs.
- **Persistent kernel + tile scheduler** (rasterization) for L2 reuse.
- Add a `cuda` feature on the `fp` crate that pulls in `fp-cuda` and
  inserts a runtime device check at the top of `impl Mul for &Matrix`,
  dispatching to the GPU for matrices above a size threshold and keeping
  operands resident on the device across `step_resolution`'s successive
  multiplications.
- Extend the parent Nix flake to provide nvcc when the user opts in.
