//! Tuning knobs shared by the Rust host code and the CUDA kernel.

/// m64 row-strips per CTA, trading operand-refill bytes per MAC against accumulator registers;
/// see EXPERIMENTS.md.
pub const MSTRIPS: usize = 3;

/// wgmma M extent, fixed for binary wgmma.
pub const MW: usize = 64;

/// K bits per loaded tile.
pub const TK: usize = 1024;

/// n128 output width (columns) per CTA, fixed by the wgmma_n128 shape.
pub const NB: usize = 128;

/// K-loop pipeline depth (full/empty buffers).
///
/// Bounded by the SMEM a stage costs; see EXPERIMENTS.md.
pub const STAGES: usize = 4;

/// M-tiles per rasterization group (L2 reuse knob). Device-side only.
pub const GROUP_M: usize = 16;

/// Threads per warpgroup. A CTA runs two: producer and consumer.
pub const THREADS_PER_WG: usize = 128;

/// The knobs as NVRTC `-D` options; the kernel defines none itself and `#error`s on a missing one.
pub fn defines() -> Vec<String> {
    [
        ("MSTRIPS", MSTRIPS),
        ("MW", MW),
        ("TK", TK),
        ("NB", NB),
        ("STAGES", STAGES),
        ("GROUP_M", GROUP_M),
        ("THREADS_PER_WG", THREADS_PER_WG),
    ]
    .iter()
    .map(|(name, value)| format!("-D{name}={value}"))
    .collect()
}
