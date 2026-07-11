//! Turns the build-time `MILNOR_PROFILE` environment variable into `cfg(milnor_profile)`, which
//! gates the (otherwise zero-cost) Milnor-multiplication profiling counters in
//! `src/algebra/milnor_algebra.rs`. Build with e.g. `MILNOR_PROFILE=1 cargo build` to enable them.

fn main() {
    // Declare the cfg so `cfg(milnor_profile)` does not trip the `unexpected_cfgs` lint.
    println!("cargo::rustc-check-cfg=cfg(milnor_profile)");
    // Rebuild when the toggle changes.
    println!("cargo::rerun-if-env-changed=MILNOR_PROFILE");
    if std::env::var_os("MILNOR_PROFILE").is_some() {
        println!("cargo::rustc-cfg=milnor_profile");
    }
}
