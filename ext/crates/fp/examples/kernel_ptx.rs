//! Compile the kernel and print the PTX on stdout.

/// Print the PTX NVRTC generates for `matmul_b1.cu`.
///
/// To see register and SMEM usage:
///
/// ```bash
/// cargo run -p fp --features gpu --example kernel_ptx > matmul_b1.ptx
/// ptxas -arch=sm_90a -v matmul_b1.ptx -o /dev/null
/// ```
fn main() -> anyhow::Result<()> {
    print!("{}", fp::blas::cuda::compile_kernel()?.to_src());
    Ok(())
}
