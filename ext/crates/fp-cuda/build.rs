//! Compile the CUDA C++ kernel to PTX via nvcc.
//!
//! The emitted `matmul_b1.ptx` is picked up by `src/lib.rs` via
//! `include_bytes!(concat!(env!("OUT_DIR"), "/matmul_b1.ptx"))`.
//!
//! When `nvcc` is **not on PATH**, we emit a *stub* PTX instead of failing the build. This keeps
//! the crate `cargo check`/`clippy`-able everywhere, which CI relies on: it runs `cargo check
//! --workspace --all-features` on a machine with no CUDA. The stub carries no kernel, so at runtime
//! `GpuContext::new` fails cleanly and callers fall back to the CPU path. An `nvcc` that *is*
//! present but fails to compile is still a hard error.

use std::{env, fs, path::PathBuf, process::Command};

const KERNEL_SRC: &str = "cuda_kernels/matmul_b1.cu";
const PTX_NAME: &str = "matmul_b1.ptx";
const ARCH: &str = "sm_90a";

/// A minimal, valid PTX module with no kernel.
///
/// `include_bytes!` needs the file to exist, so one is always written.
const STUB_PTX: &str = "//\n// fp-cuda stub: nvcc was unavailable at build time; no GPU kernel \
                        was\n// compiled. Install the CUDA Toolkit (12.x+) and rebuild to enable \
                        the GPU\n// backend.\n//\n.version 7.8\n.target sm_90a\n.address_size 64\n";

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

    match status {
        // nvcc ran and compiled the kernel
        Ok(s) if s.success() => {}
        // nvcc exists but failed to compile
        Ok(s) => panic!(
            "nvcc failed to compile {KERNEL_SRC} (exit status: {s}).\nCheck that your CUDA \
             Toolkit supports {ARCH} (Hopper sm_90a)."
        ),
        // nvcc absent, degrade to a stub so the crate still builds
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            println!(
                "cargo:warning=fp-cuda: nvcc not found ('{nvcc}': {e}); emitting a stub PTX. The \
                 GPU backend will be unavailable at runtime. Install the CUDA Toolkit (12.x+) and \
                 rebuild to enable it."
            );
            fs::write(&ptx_out, STUB_PTX).expect("failed to write stub PTX to OUT_DIR");
        }
        // nvcc is present but couldn't be executed
        Err(e) => panic!(
            "failed to run nvcc ('{nvcc}': {e}). Set the NVCC env var to a working nvcc, or unset \
             it to fall back to a stub only when nvcc is absent."
        ),
    }
}
