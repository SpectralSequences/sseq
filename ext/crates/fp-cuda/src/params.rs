//! Tuning knobs shared by the Rust host code and the CUDA kernel.

/// m64 row-strips per CTA (block knob).
///
/// Each k256 step issues `MSTRIPS` m64n128 wgmmas that share one B sub-tile: 2 → 128×128 block
/// (−20% bytes/MAC, 128 acc regs), 3 → 192×128 (−33%, 192). Accumulator regs per thread =
/// `MSTRIPS * ACC_N` ≤ 240; see EXPERIMENTS.md for why we spend the register file on the
/// accumulator rather than on a second resident CTA.
pub const MSTRIPS: usize = 3;

/// wgmma M extent, fixed for binary wgmma.
pub const MW: usize = 64;

/// K bits per loaded tile.
pub const TK: usize = 1024;

/// n128 output width (columns) per CTA, fixed by the wgmma_n128 shape.
pub const NB: usize = 128;

/// K-loop pipeline depth (full/empty buffers).
///
/// Capped at 4; see EXPERIMENTS.md.
pub const STAGES: usize = 4;

/// M-tiles per rasterization group (L2 reuse knob). Device-side only.
pub const GROUP_M: usize = 16;

/// Threads per warpgroup. A CTA runs two: producer and consumer.
pub const THREADS_PER_WG: usize = 128;

/// The knobs as NVRTC `-D` options.
///
/// These constants are the single source of truth: the host reads them directly and the kernel
/// gets these very same values, so the two cannot disagree about a tile size. `matmul_b1.cu`
/// defines none of them itself and `#error`s on a missing one, so a knob added here but left out
/// of this list stops the compile rather than silently taking a default.
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
