//! Compile the CUDA C++ kernel to PTX via nvcc.
//!
//! The emitted `matmul_b1.ptx` is picked up by `src/lib.rs` via
//! `include_bytes!(concat!(env!("OUT_DIR"), "/matmul_b1.ptx"))`. Builders
//! without nvcc see a clear error; this crate is opt-in (excluded from the
//! workspace `default-members`) so contributors who don't have CUDA installed
//! never hit this path.

use std::{env, path::PathBuf, process::Command};

const KERNEL_SRC: &str = "cuda_kernels/matmul_b1.cu";
const PTX_NAME: &str = "matmul_b1.ptx";
const ARCH: &str = "sm_90a";

fn main() {
    println!("cargo:rerun-if-changed={KERNEL_SRC}");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=NVCC");

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR not set by cargo"));
    let ptx_out = out_dir.join(PTX_NAME);

    let nvcc = env::var("NVCC").unwrap_or_else(|_| "nvcc".to_string());

    let status = Command::new(&nvcc)
        .args([
            "-ptx",
            "-O3",
            "-std=c++17",
            "--use_fast_math",
            &format!("-arch={ARCH}"),
            KERNEL_SRC,
            "-o",
        ])
        .arg(&ptx_out)
        .status();

    let status = match status {
        Ok(s) => s,
        Err(e) => {
            panic!(
                "failed to invoke nvcc ('{nvcc}'): {e}.\nfp-cuda requires the CUDA Toolkit (12.x \
                 or newer) on PATH.\nSet the NVCC env var to override the binary location."
            );
        }
    };

    if !status.success() {
        panic!(
            "nvcc failed to compile {KERNEL_SRC} (exit status: {status}).\nCheck that your CUDA \
             Toolkit supports {ARCH} (Hopper sm_90a)."
        );
    }
}
