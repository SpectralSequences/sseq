# fp

Vectors and matrices over finite fields: the linear algebra `ext` is built on.

## GPU backend

With the `gpu` feature, `<&Matrix as Mul>::mul` sends large F₂ products to a Hopper GPU
(`blas::cuda`), and falls back to the CPU kernel when there is no device, the product is small, or
a launch fails.

An F₂ product needs only AND and a popcount mod 2, which is exactly what Hopper's binary tensor-core
instruction `wgmma…b1.b1.and.popc` computes. No library reaches it: cuBLAS has no 1-bit data type,
and CUTLASS supports `b1` only through Ampere's warp-level `mma`, not SM90's `wgmma`. The kernel
also reduces mod 2 and packs the bits on the device, so the product never exists as s32
accumulators.

For algorithms built from many dependent products over the same data, `blas::cuda::DeviceMatrix`
keeps a matrix in device memory between calls: `GpuContext::upload` once, `matmul_b1_dev` and
`gemm_xor_into` (`dst[:, c:] ^= l·u`, XORed into `dst` without a product buffer where its alignment
allows) as often as needed, `download` once.

Every submission goes through one driver thread, so the process has a single owner of the device:
a persistent grid fills the machine, and concurrent callers would only queue on it.
`FP_CUDA_DEVICE` picks which GPU that is.

**Requirements.** libnvrtc (CUDA 12+) on the loader path, since the kernel is compiled at runtime
(`nix develop ./ext#gpu` provides it), and a Hopper GPU, since the kernel targets `compute_90a`.
Without them `fp` still builds, and the GPU tests pass vacuously.

**Environment.** `FP_CUDA_THRESHOLD` is the smallest `min(m, k, n)` sent to the GPU,
`FP_CUDA_DISABLE` forces the CPU, and `FP_CUDA_DEBUG` prints each launch's parameters.

From `ext/crates/fp`:

```bash
cargo test --release --features gpu -- cuda                    # kernel proptests
cargo test --release --features gpu --test cuda_dispatch       # `Mul` dispatch proptests
cargo run --release --features gpu --example bench_kernel_only # kernel-only throughput
cargo run --release --features gpu --example bench_shapes      # the L2-residency check
cargo bench --features gpu --bench matmul_b1                   # end to end, against the CPU
```

What was tried, and what it measured, is in `src/blas/cuda/EXPERIMENTS.md`.
