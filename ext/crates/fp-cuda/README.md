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
2. A **Hopper GPU** at runtime (sm_90a). The kernel is built for `sm_90a`, an
   architecture-specific target that is **not** forward-compatible, so the PTX
   runs only on Hopper — not on pre-Hopper devices (which lack the `wgmma.*` and
   `cp.async.bulk.tensor.*` instructions it emits) nor on newer architectures
   such as Blackwell (`sm_100`).

Builds on **stable** Rust — no nightly toolchain required. (`nvcc` is still
needed at build time to compile the kernel to PTX, and a CUDA driver at runtime.)

## Building

```bash
# From the workspace root; the leading -p selects this crate explicitly.
cargo build -p fp-cuda
```

`build.rs` invokes nvcc on `cuda_kernels/matmul_b1.cu` and emits
`matmul_b1.ptx` into the cargo `OUT_DIR`. `src/lib.rs` embeds it via
`include_bytes!` and loads it at runtime through cudarc.

**When nvcc is absent** (CI, or a contributor without the CUDA Toolkit)
`build.rs` emits a *stub* PTX and prints a warning instead of failing, so the
crate stays `cargo check`/`clippy`-able everywhere — including under
`cargo check --workspace --all-features`, which the workspace CI runs on a
machine with no CUDA. The stub carries no kernel, so at runtime
`GpuContext::new` returns an error and callers (e.g. `fp`'s `cuda` dispatch)
fall back to the CPU path. An nvcc that *is* present but fails to compile is
still a hard build error.

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

The full pipeline (host row-major pre-arrangement → TMA 128B-swizzle loads →
mbarrier sync → persistent grid + clusters/multicast → register-blocked
`m64n128k256` wgmma.b1 strips → bit-pack → double-buffered TMA bulk output
store) is **validated on an H200 NVL (sm_90, CUDA 13.0 driver / 12.4 toolkit,
2026-07-07)** and earlier on an H100 NVL. The PTX JITs at module load, the
dynamic-SMEM opt-in (~166 KB) and all three TMA descriptors are accepted, and
outputs are **bit-exact** against the CPU `fp::blas` path across `matmul_b1_demo`
(64…8192) and the kernel-only bench (4096…32768, incl. a full 32768³ CPU
cross-check).

Throughput, **kernel-only** (host setup + H2D/D2H excluded — the comparison the
~100-TOPS pre-swizzle baseline was measured at), H200 NVL:

| size (M=K=N) | binary TOPS | ms/launch |
|--------------|-------------|-----------|
| 4096         | ~4,000      | 0.034     |
| 8192         | ~6,700      | 0.163     |
| 16384        | ~8,600      | 1.02      |
| 32768        | ~9,600      | 7.33      |

i.e. roughly a **~96× kernel speedup** over the ~100-TOPS pre-swizzle state, and
the >16384 L2 cliff is **gone** — throughput now *climbs* with size.

Getting there took closing an L2-residency-of-B cliff, then a bandwidth wall,
then a latency wall (all measured on-device, since ncu can't attach through the
CUDA-13 driver — see the standalone wgmma/L2 microbenchmark approach):
- **Persistent grid + grouped rasterization + clusters/TMA-multicast** (Phases
  8–9) keep B's reuse distance short so it stays L2-resident. `bench_shapes` now
  shows equal-FLOPs B-fits and B-spills shapes running at the *same* throughput —
  the cliff is closed.
- **Register-blocking** (Phase 10): each CTA computes a 192×128 output block as 3
  `m64n128` strips sharing one loaded B sub-tile, cutting operand-refill
  bytes/MAC by 33% — the kernel had become L2→SMEM-refill bound (~8 TB/s
  sustained vs a ~12.5k-TOPS single-warpgroup wgmma ceiling).
- **Epilogue overlap** (Phase 11): double-buffered `sC` + a deferred
  `cp.async.bulk.wait_group.read 1` so tile T's output store drains during tile
  T+1's compute.
- **Fence hoisting** (Phase 12): one `wgmma.fence` before the K-loop instead of
  two per k-chunk (a warpgroup-wide sync). After blocking, skipping an entire
  operand load barely changes throughput — the kernel is now **TMA-latency
  bound** (deeper pipelines help; pipeline depth is capped by SMEM at STAGES=4).

Run `cargo run --release -p fp-cuda --example bench_shapes` (L2-residency check)
or `--example tune` (fast throughput sweep) to reproduce.

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
- **Wide binary MMA** (Phase 4) — began with one `m64n256k256` per k-step
  (replacing four `m64n64k256`); the current kernel uses `m64n128k256` strips
  (see Phase 10).
- **Register reallocation** (Phase 5) — `setmaxnreg.dec(40)` in the producer,
  `setmaxnreg.inc(N)` in the consumer (N sized to the accumulator block).
- **Deeper pipeline** (Phase 6) — `STAGES` full/empty buffers (now 4).
- **TMA output store** (Phase 7) — the packed `sC` tile is written back with a
  single `cp.async.bulk.tensor.2d.global.shared::cta`; C is padded to whole
  NG-limb column groups so every stored tile is complete.
- **Persistent kernel + grouped rasterization** (Phase 8) — a 1-D grid of
  ~SM-count CTAs sweeps the output tiles in a grouped-along-M order (`GROUP_M`
  M-tiles per band), shortening each B-panel's reuse distance to keep it
  L2-resident. Cuts B's HBM re-reads by ~`GROUP_M`.
- **Clusters + TMA multicast** (Phase 9) — `CLUSTER` CTAs along M form a
  thread-block cluster and share one HBM read of each B-panel via
  `cp.async.bulk.tensor…multicast::cluster` (each computes a different M-tile).
  Cluster-wide empty barrier; the pipeline barriers init once and flow
  continuously across tiles. Mirrors the proven `pranjalssh/fast.cu` matmul_9.
- **Register-blocking** (Phase 10) — each CTA computes a `TM×NB` output block
  (`MSTRIPS` `m64n128` strips × `NB`=128 cols) whose strips all reuse one loaded
  B sub-tile, cutting operand-refill bytes/MAC (the bottleneck after Phase 9).
  `MSTRIPS`=3 → a 192×128 block, −33% bytes/MAC. `MSTRIPS`/`TILE_M`/`NG` in the
  kernel and `src/lib.rs` must match.
- **Epilogue overlap** (Phase 11) — double-buffered `sC` + deferred
  `cp.async.bulk.wait_group.read 1` so tile T's output store overlaps tile T+1's
  compute.
- **Fence hoisting** (Phase 12) — one `wgmma.fence` before the K-loop instead of
  two per k-chunk.

Phases 8–9 were validated on hardware (H200 NVL) alongside 10–12.

- **Wired into `fp`** — the `fp` crate has an optional `gpu` feature
  (`cargo …--features fp/gpu`) that pulls in `fp-cuda` and dispatches large
  `p = 2` products from `<&Matrix as Mul>::mul` to the GPU, falling back to the
  CPU BLAS kernel when no device is present, the size is below
  `FP_CUDA_THRESHOLD` (default 2048), or a launch fails. `fp-cuda`'s library API
  is fp-agnostic (raw row-major limb slices) so the dependency is acyclic; the
  `Matrix` glue lives on the `fp` side (`src/blas/cuda.rs`) and in the
  examples/benches (a dev-dependency on `fp`). `FP_CUDA_DISABLE` forces the CPU
  path. See the `gpu_dispatch_matches_cpu` integration test.

Remaining (now TMA-latency bound — the consumer out-runs per-tile TMA latency and
pipeline depth is SMEM-capped at `STAGES`=4):

- Keep operands resident on the device across `step_resolution`'s successive
  multiplications (the current dispatch re-marshals and re-copies each product).
- Extend the parent Nix flake to provide nvcc when the user opts in.
