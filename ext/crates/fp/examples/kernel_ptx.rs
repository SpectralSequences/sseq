//! Compile the kernel and print the PTX on stdout.

/// Print the PTX NVRTC generates for `cuda_kernels/matmul_b1.cu`.
fn main() -> anyhow::Result<()> {
    print!("{}", fp::blas::cuda::compile_kernel()?.to_src());
    Ok(())
}
