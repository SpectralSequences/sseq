# fp-cuda

CUDA backend for the F₂ matrix multiplication implemented in `crates/fp/src/blas/`.

## Why a binary tensor-core kernel?

Computing Ext is, at the bottom, F₂ linear algebra: products and row reductions of
matrices over the two-element field. Those matrices are bit-packed, 64 entries to a
`u64`, so a product is a question about bits — entry `(i, j)` of `A·B` is the
*parity* of the number of positions where row `i` of A and column `j` of B are both
set. There is no multiplication to do, only AND and a popcount taken mod 2.

That is precisely what Hopper's binary tensor-core instruction computes.
`wgmma.mma_async…s32.b1.b1.and.popc` reads its operands as bit matrices, ANDs them
elementwise, and accumulates the popcounts into s32 registers, 256 bits of K per
issue. Take the low bit of each accumulator and you have the F₂ entry — no
conversion, and no arithmetic that F₂ does not need.

NVIDIA did not add these instructions for algebraic topology; they exist for
binarized neural networks. But the operation is the same one, and the fit is exact
rather than approximate. It is also the cheapest thing the tensor cores can be asked
to do, so binary is the highest-throughput mode the hardware has, and bit-packing
keeps operand traffic far below what any wider datatype would move.

## Why not cuBLAS or CUTLASS?

- **cuBLAS cannot express a binary GEMM.** Every cuBLAS multiply takes its operand
  types from `cudaDataType`, and that enum has no 1-bit member — the narrowest are
  the 4-bit `CUDA_R_4I` / `CUDA_R_4U`. No `CUBLAS_COMPUTE_*` mode corresponds to a
  binary accumulate either. The 1-bit tensor-core mode is simply not reachable
  through the library.
- **CUTLASS has 1-bit MMA, but not on Hopper.** It supports `b1` — both the
  AND-popcount and XOR-popcount forms, at shape 16×8×256 — for SM75 and SM80,
  through the warp-level `mma.b1` instruction in its 2.x kernel families. The 3.x
  kernels that target SM90 list no `b1` type at all, and the SM90 `wgmma` atoms
  cover the floating-point and 8-bit integer types only. Building on CUTLASS here
  would mean running the Ampere-era warp-level instruction on Hopper rather than the
  warpgroup-wide `wgmma.b1` this kernel is built around — and vendoring a large
  template library to reach an instruction we emit in a few lines of inline PTX.
- **The epilogue is the real obstacle.** A general GEMM returns s32 accumulators —
  32× the bytes of the bit-packed answer — and we would still need a separate pass to
  reduce mod 2 and re-pack the bits. This kernel takes the parity and packs the
  result in shared memory, then stores packed limbs straight back, so the output
  never exists in expanded form.
- **The operands are already in our layout.** `fp::Matrix` stores row-major `u64`
  limbs; a library GEMM would want a conversion on both sides of every call, on data
  that is bit-packed precisely so it stays small.

## How it works

The Hopper memory pipeline is used end-to-end: kernel written in CUDA C++ with
inline PTX for **TMA bulk tensor loads** with **128B swizzle**
(`cp.async.bulk.tensor.2d`), **mbarrier**-based completion sync, and the binary
tensor cores
(`wgmma.mma_async.sync.aligned.m64n128k256.row.col.s32.b1.b1.and.popc`).
Both operands are pre-arranged into plain row-major K-major tiles on the host;
the TMA applies the swizzle that the wgmma matrix descriptors expect.
Rust-side glue uses [`cudarc`](https://crates.io/crates/cudarc) for the host
driver-API surface (module load, device buffers, typed launch) and its
`driver::sys` raw bindings for the `cuTensorMapEncodeTiled` call that builds the
TMA descriptors. `cudarc` is stable Rust and dynamically loads the CUDA driver
at runtime, so the Rust side builds with no CUDA present.

This crate is **excluded from the workspace**, so every workspace-wide command ignores it. It is
opt-in: building requires nvcc on `PATH` and a Hopper-class GPU at runtime.

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
# nvcc lives in the opt-in dev shell, not the default one.
nix develop ./ext#gpu

# The crate is excluded from the workspace, so run cargo from its own directory
# rather than selecting it with -p from the root.
cd ext/crates/fp-cuda
cargo build
```

`build.rs` invokes nvcc on `cuda_kernels/matmul_b1.cu` and emits
`matmul_b1.ptx` into the cargo `OUT_DIR`. `src/lib.rs` embeds it via
`include_bytes!` and loads it at runtime through cudarc.

The kernel's tuning knobs live in `cuda_kernels/params.h`. The kernel includes
that header directly, and `build.rs` parses it to generate the Rust mirror, so
the host and device cannot disagree about a tile size.

**When nvcc is absent** (CI, or a contributor without the CUDA Toolkit) the
build fails. Nothing in the workspace's own `just` recipes builds this crate:
the workspace `exclude` entry keeps it out, so even `--workspace` commands skip
it. Callers reach the backend through `fp`'s optional `gpu` feature, which is
off by default and falls back to the CPU path when it is not enabled.

## Running

All from `ext/crates/fp-cuda`:

```bash
# Smoke test (multiplies a few small shapes, asserts CPU↔GPU equality):
cargo run --example matmul_b1_demo

# Kernel-only throughput (the number to quote):
cargo run --release --example bench_kernel_only

# Fast throughput sweep, and the L2-residency check:
cargo run --release --example tune
cargo run --release --example bench_shapes

# Benchmark against the CPU AVX-512 path in fp::blas:
cargo bench
```

The bench compares each square size in `{128, 256, 512, 1024, 2048, 4096, 8192}`
against `fp::blas::fast_mul_concurrent`, asserts bit-equality of the outputs,
and prints binary TOPS for both backends.

Note that the end-to-end `cargo bench` figures (≤30 TOPS) are dominated by host
serialization and the TMA-layout pre-arrangement, not by the kernel; use
`bench_kernel_only` for the kernel number.

## Using it from `fp`

The `fp` crate has an optional `gpu` feature (`cargo … --features fp/gpu`)
that pulls in `fp-cuda` and dispatches large `p = 2` products from
`<&Matrix as Mul>::mul` to the GPU. It falls back to the CPU BLAS kernel when no
device is present, when the size is below `FP_CUDA_THRESHOLD` (default 2048), or
when a launch fails; `FP_CUDA_DISABLE` forces the CPU path, and `FP_CUDA_DEBUG`
prints the launch parameters. See the `gpu_dispatch_matches_cpu` integration
test.

`fp-cuda`'s library API is fp-agnostic (raw row-major limb slices) so the
dependency is acyclic; the `Matrix` glue lives on the `fp` side
(`src/blas/cuda.rs`) and in the examples/benches, which take a dev-dependency on
`fp`.

## Why excluded from the workspace?

Contributors without nvcc would otherwise see this crate's build fail every time
they run a workspace-wide command. Leaving it out of `default-members` is not
enough: path dependencies residing in the workspace directory are auto-discovered
as members, and `--workspace` selects every member, so `cargo check --workspace`
and rust-analyzer's flycheck would still build it. Only `exclude` keeps it out.

That means:

- `cargo build`, `cargo test`, `cargo check --workspace`, `nix run .#test` and
  rust-analyzer all ignore the crate.
- `fp`'s `gpu` feature still reaches it as a path dependency, so enabling the
  backend works as before.
- It is no longer addressable as `-p fp-cuda` from the workspace root. Build and
  test it from its own directory: `cd crates/fp-cuda && cargo test`.

## Status

Validated on an H200 NVL (CUDA 12.4 toolkit) and earlier on an H100 NVL. Outputs
are **bit-exact** against the CPU `fp::blas` path across `matmul_b1_demo`
(64…8192) and the kernel-only bench (4096…32768, including a full 32768³ CPU
cross-check).

Throughput, **kernel-only** (host setup + H2D/D2H excluded), H200 NVL:

| size (M=K=N) | binary TOPS | ms/launch |
|--------------|-------------|-----------|
| 4096         | ~4,100      | 0.033     |
| 8192         | ~7,000      | 0.158     |
| 16384        | ~8,500      | 1.04      |
| 32768        | ~9,600      | 7.35      |

## Debugging notes

If a correctness regression appears, start with the **64×256×64 identity /
small product** — the smallest path that exercises one swizzled tile end to end,
and the first `matmul_b1_demo` case. A failure there points at the swizzled wgmma
descriptor constants (`DESC_LBO`, `DESC_SBO`, and the per-k256 advance), which
derive from CUTLASS `make_gmma_desc<Major::K>` (`LayoutType::B128`), or at a
host-layout / TMA-box mismatch. Widen to the full `matmul_b1_demo` sweep only
once that passes.

Two further invariants are worth a trace if loads fault rather than miscompute:
the dynamic-SMEM base must be 128-byte aligned for TMA (declared
`extern __shared__ __align__(128)`), and the per-stage
`expect_tx = (TILE_A + TILE_B) * 8` bytes must match the
`cp.async.bulk.tensor.complete_tx::bytes` notifications from the two issued TMA
loads.

## Not yet done

- Keep operands resident on the device across `step_resolution`'s successive
  multiplications (the current dispatch re-marshals and re-copies each product).
- Speed up the host-side bit transpose in `transpose_b`, which dominates
  end-to-end time at large sizes.
