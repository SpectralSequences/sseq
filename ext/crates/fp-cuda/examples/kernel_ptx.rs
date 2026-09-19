//! Compile the kernel and print the PTX on stdout.

/// Print the PTX NVRTC generates for `cuda_kernels/matmul_b1.cu`.
///
/// Needs libnvrtc but no GPU. `ptxas` reads the output back, which is where the per-thread
/// register counts EXPERIMENTS.md tunes against come from:
///
/// ```text
/// cargo run --example kernel_ptx > matmul_b1.ptx
/// ptxas -arch=sm_90a -v matmul_b1.ptx -o /dev/null
/// ```
fn main() -> anyhow::Result<()> {
    print!("{}", fp_cuda::compile_kernel()?.to_src());
    Ok(())
}
