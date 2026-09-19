//! Tuning knobs shared by the Rust host code and the CUDA kernel.

/// m64 row-strips per CTA (block knob).
///
/// Each k256 step issues `MSTRIPS` m64n128 wgmmas that share one loaded B sub-tile, so raising it
/// cuts operand-refill bytes per MAC and costs accumulator registers: the consumer holds
/// `MSTRIPS * ACC_N` of them, which the register file bounds. See EXPERIMENTS.md for the
/// measurements behind the value.
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

/// The knobs as NVRTC `-D` options.
///
/// The kernel defines none of them itself and `#error`s on a missing one, so a knob this list
/// omits stops the compile rather than silently taking a default.
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
