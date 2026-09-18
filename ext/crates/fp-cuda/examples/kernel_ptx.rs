//! Compile the kernel and print the PTX on stdout.
//!
//! Needs libnvrtc but no GPU, which makes it both a compile check and the way to read the
//! generated code — in particular the per-thread register counts the tuning in EXPERIMENTS.md
//! turns on:
//!
//! ```text
//! cargo run --example kernel_ptx > matmul_b1.ptx
//! ptxas -arch=sm_90a -v matmul_b1.ptx -o /dev/null
//! ```

fn main() -> anyhow::Result<()> {
    print!("{}", fp_cuda::compile_kernel()?.to_src());
    Ok(())
}
