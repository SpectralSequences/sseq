//! CUDA backend for `fp::blas` F_2 matrix multiplication on Hopper.

use std::{
    collections::HashMap,
    ffi::c_void,
    mem::MaybeUninit,
    sync::{Arc, Mutex},
    thread::ThreadId,
    time::Instant,
};

use cudarc::{
    driver::{
        CudaContext, CudaFunction, CudaModule, CudaSlice, CudaStream, DevicePtr, DeviceRepr,
        LaunchConfig, PushKernelArg, sys,
    },
    nvrtc::Ptx,
};

static PTX_IMAGE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/matmul_b1.ptx"));

/// The tuning knobs from `cuda_kernels/params.h`, mirrored into Rust by `build.rs`.
///
/// The kernel includes the same header, so these cannot drift from it.
mod params {
    #![allow(dead_code)]
    include!(concat!(env!("OUT_DIR"), "/params.rs"));
}
use params::{MSTRIPS, MW, NB, STAGES, THREADS_PER_WG, TK};

const TILE_M: usize = MW * MSTRIPS; // output rows per CTA
const TILE_K: usize = TK;
const KL: usize = TILE_K / 64;
const THREADS: u32 = (2 * THREADS_PER_WG) as u32; // producer warpgroup + consumer warpgroup
const NG: u32 = (NB / 64) as u32; // output column-limbs per CTA

/// Widest forward-pass panel, in limbs: at this width the forward GEMM stops padding K.
const MAX_BL: usize = 16;

/// Default grid cap for `panel_factor_coop`, overridden by `FP_CUDA_PF_CTAS`.
const DEFAULT_PF_CTAS: u32 = 128;
/// Default grid cap for `promote_coop`, overridden by `FP_CUDA_PROM_CTAS`.
const DEFAULT_PROM_CTAS: u32 = 384;
/// Default grid cap for `block_reduce_coop` under TRSM, overridden by `FP_CUDA_BR_CTAS`.
const DEFAULT_BR_CTAS: u32 = 128;
/// Default TRSM base-block width. Bounded by `block_reduce_rref`'s shared `cond[]`.
const DEFAULT_BS_BASE: usize = 64;

/// Adaptive forward-pass panel width in limbs (`b = 64·bl` columns).
///
/// Wider panels raise the trailing GEMM's contraction dimension toward `b`, reclaiming the
/// K-padding waste (Loss 1); the counter-pressure is the promotion cost, which is O(bl) for the
/// single-CTA `promote_pivots` but ~bl-independent for the cooperative `promote_coop` used at
/// stride ≥ 1024. See EXPERIMENTS.md for the measured optima. Override with `FP_CUDA_BL`.
fn adaptive_bl(stride: usize) -> usize {
    let div = if stride >= 1024 { 128 } else { 256 };
    (stride / div).clamp(1, MAX_BL)
}

/// Whether the row reduction uses its **cooperative** kernels — `panel_factor_coop`,
/// `promote_coop`, `block_reduce_coop` — launched with `cuLaunchCooperativeKernel` and
/// synchronized by a hand-rolled grid-wide spin barrier.
///
/// **Off by default.** The barrier spins until every CTA arrives, so the launch needs the whole
/// grid co-resident, which holds only when this process owns the GPU outright: any other CUDA
/// runtime occupying SMs strands the missing CTAs and the resident ones spin forever. The default
/// path keeps the same all-SM parallelism but replaces the in-grid `grid_sync` with kernel-boundary
/// synchronization, so nothing launches cooperatively and nothing can deadlock; it costs the
/// forward pass a per-column relaunch, converging with size as the O(cols) launch term is dwarfed
/// by the O(cols²) work. Set `FP_CUDA_RR_COOP=1` to opt in on a dedicated GPU.
fn rr_coop() -> bool {
    std::env::var("FP_CUDA_RR_COOP")
        .map(|v| v != "0" && !v.is_empty())
        .unwrap_or(false)
}

/// A `CUtensorMap` passed by value as a (grid-constant) kernel argument.
///
/// `repr(transparent)` so the pointer cudarc's typed launch builder pushes is the address of the
/// 128-byte descriptor itself.
#[repr(transparent)]
struct TmaArg(sys::CUtensorMap);

// SAFETY: `DeviceRepr` requires the type to be plain data that is valid to memcpy into a kernel
// parameter. `CUtensorMap` is an opaque 128-byte POD descriptor with no host pointers or padding
// invariants, and the kernel declares the matching parameter `const __grid_constant__ CUtensorMap`.
unsafe impl DeviceRepr for TmaArg {}

/// A bit-packed F₂ matrix resident in device memory, in the natural row-major,
/// K-major limb layout `fp::Matrix` uses: `stride = cols.div_ceil(64)` u64 per
/// row, `rows * stride` u64 total, one bit per entry, bits past `cols` zero.
///
/// This is the persistent buffer the row-reduction port operates over: uploaded once, mutated in
/// place by device kernels, downloaded once. It is `fp`-agnostic (raw limbs) so `fp-cuda` stays
/// free of a dependency on `fp`.
pub struct DeviceMatrix {
    pub buf: CudaSlice<u64>,
    pub rows: usize,
    pub cols: usize,
    pub stride: usize,
}

impl DeviceMatrix {
    /// Row stride in 64-bit limbs.
    pub fn stride(&self) -> usize {
        self.stride
    }
}

/// A CUDA device with the `matmul_b1` kernel loaded, ready to launch.
pub struct GpuContext {
    ctx: Arc<CudaContext>,
    /// Per-thread CUDA streams for *this* context, created lazily (see [`Self::stream`]). Owned by
    /// the context so a thread using several contexts (e.g. one per device) gets a distinct stream
    /// per context. The mutex is held only for the map lookup, never across a GPU submission, so it
    /// does not serialize device work — unlike the whole-op lock this design replaced.
    streams: Mutex<HashMap<ThreadId, Arc<CudaStream>>>,
    #[allow(dead_code)]
    module: Arc<CudaModule>,
    kernel: CudaFunction,
    // Device-resident packing/epilogue kernels for the row-reduction port.
    pack_a: CudaFunction,
    pack_b: CudaFunction,
    xor_into: CudaFunction,
    panel_factor: CudaFunction,
    panel_factor_coop: CudaFunction,
    pf_find_swap: CudaFunction,
    pf_step: CudaFunction,
    pf_xor: CudaFunction,
    mark_live: CudaFunction,
    promote_pivots: CudaFunction,
    promote_coop: CudaFunction,
    zero_pivot_l: CudaFunction,
    gather_rows: CudaFunction,
    block_reduce_rref: CudaFunction,
    block_reduce_coop: CudaFunction,
    br_cond: CudaFunction,
    br_xor: CudaFunction,
    gather_cols: CudaFunction,
    xor_into_perm: CudaFunction,
}

impl GpuContext {
    /// Open device `device_id` and load the kernel onto it.
    ///
    /// Fails if there is no usable device, or if the embedded PTX is the stub `build.rs` emits when
    /// `nvcc` is absent, in which case the kernel is missing from the module.
    pub fn new(device_id: usize) -> anyhow::Result<Self> {
        // Deliberately the device *primary* context: another CUDA runtime in this process will
        // retain the same one, and a private context does not isolate us from it anyway.
        let ctx = CudaContext::new(device_id)?;
        let ptx = Ptx::from_src(String::from_utf8(PTX_IMAGE.to_vec())?);
        let module = ctx.load_module(ptx)?;
        let kernel = module.load_function("matmul_b1_kernel")?;
        let pack_a = module.load_function("pack_a")?;
        let pack_b = module.load_function("pack_b")?;
        let xor_into = module.load_function("xor_into")?;
        let panel_factor = module.load_function("panel_factor")?;
        let panel_factor_coop = module.load_function("panel_factor_coop")?;
        let pf_find_swap = module.load_function("pf_find_swap")?;
        let pf_step = module.load_function("pf_step")?;
        let pf_xor = module.load_function("pf_xor")?;
        let mark_live = module.load_function("mark_live")?;
        let promote_pivots = module.load_function("promote_pivots")?;
        let promote_coop = module.load_function("promote_coop")?;
        let zero_pivot_l = module.load_function("zero_pivot_l")?;
        let gather_rows = module.load_function("gather_rows")?;
        let block_reduce_rref = module.load_function("block_reduce_rref")?;
        let block_reduce_coop = module.load_function("block_reduce_coop")?;
        let br_cond = module.load_function("br_cond")?;
        let br_xor = module.load_function("br_xor")?;
        let gather_cols = module.load_function("gather_cols")?;
        let xor_into_perm = module.load_function("xor_into_perm")?;
        Ok(Self {
            ctx,
            streams: Mutex::new(HashMap::new()),
            module,
            kernel,
            pack_a,
            pack_b,
            xor_into,
            panel_factor,
            panel_factor_coop,
            pf_find_swap,
            pf_step,
            pf_xor,
            mark_live,
            promote_pivots,
            promote_coop,
            zero_pivot_l,
            gather_rows,
            block_reduce_rref,
            block_reduce_coop,
            br_cond,
            br_xor,
            gather_cols,
            xor_into_perm,
        })
    }

    /// The device's compute capability as `(major, minor)`. The kernel requires 9.0 (Hopper).
    pub fn compute_capability(&self) -> anyhow::Result<(i32, i32)> {
        let major = self.ctx.attribute(
            sys::CUdevice_attribute_enum::CU_DEVICE_ATTRIBUTE_COMPUTE_CAPABILITY_MAJOR,
        )?;
        let minor = self.ctx.attribute(
            sys::CUdevice_attribute_enum::CU_DEVICE_ATTRIBUTE_COMPUTE_CAPABILITY_MINOR,
        )?;
        Ok((major, minor))
    }

    /// The context's shared default stream, for callers that do not need per-thread isolation.
    pub fn default_stream(&self) -> Arc<CudaStream> {
        self.ctx.default_stream()
    }

    /// A CUDA stream **private to the calling OS thread**.
    ///
    /// Created lazily on first use and reused thereafter, cached per thread in this context's
    /// `streams` map. Submitting through this instead of the context's single `default_stream()`
    /// lets calls from different threads run on distinct streams — overlapping transfers and
    /// kernels instead of serializing — while all sub-steps of one call share one stream, which
    /// keeps ordering correct within a thread. This is what lets `try_mul` (and the row-reduce) run
    /// lock-free from many threads at once.
    pub fn stream(&self) -> Arc<CudaStream> {
        self.streams
            .lock()
            .expect("stream cache poisoned")
            .entry(std::thread::current().id())
            // Fall back to the context default stream if creation fails (e.g. a poisoned context),
            // so the caller sees a normal Err and degrades to CPU rather than panicking.
            .or_insert_with(|| {
                self.ctx
                    .new_stream()
                    .unwrap_or_else(|_| self.ctx.default_stream())
            })
            .clone()
    }

    /// The loaded `matmul_b1_kernel` handle.
    pub fn kernel(&self) -> &CudaFunction {
        &self.kernel
    }
}

/// Multiply two bit-packed F₂ matrices on the GPU.
///
/// Operands are plain **row-major, K-major** limb arrays — the exact layout `fp::Matrix::to_bytes`
/// produces (little-endian `u64` limbs, one bit per entry, `columns.div_ceil(64)` limbs per row, no
/// inter-row padding):
///
/// - `a`: the `m`×`k` left operand, `m * k.div_ceil(64)` limbs.
/// - `b`: the `k`×`n` right operand, `k * n.div_ceil(64)` limbs.
///
/// Returns C = A·B as `m * n.div_ceil(64)` limbs in the same layout, ready to hand to
/// `fp::Matrix::from_data`.
pub fn matmul_b1_raw(
    gpu: &GpuContext,
    a: &[u64],
    m: usize,
    k: usize,
    b: &[u64],
    n: usize,
) -> anyhow::Result<Vec<u64>> {
    Ok(matmul_b1_inner(gpu, a, m, k, b, n, 1)?.0)
}

/// Like [`matmul_b1_raw`], but also returns the average **kernel-only** wall time in seconds.
///
/// The time is averaged over `time_iters` back-to-back launches, and excludes host
/// (de)serialization, the TMA-layout pre-arrangement, and the H2D/D2H copies.
///
/// The kernel zeroes its SMEM accumulator and writes C with a bulk-tensor *store* (overwrite, not
/// accumulate), so repeated launches against the same device buffers are idempotent and the
/// returned limbs are the correct product.
pub fn matmul_b1_raw_timed(
    gpu: &GpuContext,
    a: &[u64],
    m: usize,
    k: usize,
    b: &[u64],
    n: usize,
    time_iters: usize,
) -> anyhow::Result<(Vec<u64>, f64)> {
    matmul_b1_inner(gpu, a, m, k, b, n, time_iters.max(1))
}

#[allow(clippy::too_many_arguments)]
fn matmul_b1_inner(
    gpu: &GpuContext,
    a: &[u64],
    m: usize,
    k: usize,
    b: &[u64],
    n: usize,
    time_iters: usize,
) -> anyhow::Result<(Vec<u64>, f64)> {
    let n_lim = n.div_ceil(64);
    assert_eq!(a.len(), m * k.div_ceil(64), "A limb count mismatch");
    assert_eq!(b.len(), k * n_lim, "B limb count mismatch");

    let k_padded = k.next_multiple_of(TILE_K);
    // Pad M to a whole number of M-tiles; the extra padded rows produce zeros
    // that the `take(m)` readback trims.
    let m_padded = m.next_multiple_of(TILE_M);
    // Each CTA computes a TILE_M×(NG*64) output block via MSTRIPS m64n128 wgmmas,
    // so B (and the C output) are grouped/padded to whole NG-limb column tiles.
    let n_groups = n_lim.div_ceil(NG as usize);
    let n_padded_lim = n_groups * NG as usize;

    let stream = gpu.stream();

    let a_padded = pad_2d(a, m, k.div_ceil(64), m_padded, k_padded / 64);
    let b_padded = pad_2d(b, k, n_lim, k_padded, n_lim);

    let a_interleaved = interleave_a(&a_padded, m_padded, k_padded);
    let bt = transpose_b(&b_padded, k_padded, n_lim);

    let a_dev = stream.clone_htod(&a_interleaved)?;
    let bt_dev = stream.clone_htod(&bt)?;
    let c_dev = stream.alloc_zeros::<u64>(m_padded * n_padded_lim)?;

    let kernel_secs = run_gemm_kernel(
        gpu,
        &stream,
        &a_dev,
        &bt_dev,
        &c_dev,
        m_padded,
        k_padded,
        n_groups,
        n_padded_lim,
        time_iters,
    )?;

    let c_all = stream.clone_dtoh(&c_dev)?;
    let c_limbs: Vec<u64> = c_all
        .chunks_exact(n_padded_lim)
        .take(m)
        .flat_map(|row| row[..n_lim].iter().copied())
        .collect();
    Ok((c_limbs, kernel_secs))
}

/// Encode the TMA descriptors and launch the `wgmma.b1` GEMM over operands that
/// are **already interleaved/transposed and resident on device**. Shared by the
/// host round-trip path ([`matmul_b1_inner`]) and the device-resident path
/// ([`GpuContext::matmul_b1_dev`]) — the only difference between them is where
/// the packed operands come from and whether `C` is downloaded. Returns the
/// average kernel-only wall time over `time_iters` launches.
#[allow(clippy::too_many_arguments)]
fn run_gemm_kernel(
    gpu: &GpuContext,
    stream: &Arc<CudaStream>,
    a_dev: &cudarc::driver::CudaSlice<u64>,
    bt_dev: &cudarc::driver::CudaSlice<u64>,
    c_dev: &cudarc::driver::CudaSlice<u64>,
    m_padded: usize,
    k_padded: usize,
    n_groups: usize,
    n_padded_lim: usize,
    time_iters: usize,
) -> anyhow::Result<f64> {
    let m_tiles = m_padded / TILE_M;
    let k_chunks = k_padded / TILE_K;

    // Raw device addresses for the TMA descriptors. The returned guards keep the
    // reads ordered on the stream; hold them until after the launch.
    let (a_ptr, _ga) = a_dev.device_ptr(stream);
    let (b_ptr, _gb) = bt_dev.device_ptr(stream);
    let (c_ptr, _gc) = c_dev.device_ptr(stream);

    let tma_a = encode_tma(
        a_ptr,
        [32, (k_chunks * m_tiles * TILE_M) as u64],
        [32, TILE_M as u32],
        128,
        sys::CUtensorMapSwizzle_enum::CU_TENSOR_MAP_SWIZZLE_128B,
    )?;
    let tma_b = encode_tma(
        b_ptr,
        [32, (k_chunks * n_groups * NG as usize * 64) as u64],
        [32, (NG as usize * 64) as u32],
        128,
        sys::CUtensorMapSwizzle_enum::CU_TENSOR_MAP_SWIZZLE_128B,
    )?;
    let tma_c = encode_tma(
        c_ptr,
        [(n_padded_lim * 2) as u64, m_padded as u64],
        [(NG as usize * 2) as u32, TILE_M as u32],
        (n_padded_lim * 8) as u64,
        sys::CUtensorMapSwizzle_enum::CU_TENSOR_MAP_SWIZZLE_NONE,
    )?;

    // Dynamic SMEM per CTA: sA + sB + 2*sC (double-buffered) + 2*STAGES mbarriers.
    let tile_a = TILE_M * KL; // TILE_M-row A block
    let tile_b = NG as usize * 64 * KL; // (NG*64)-col B tile
    let smem_u64 = STAGES * tile_a + STAGES * tile_b + 2 * NG as usize * TILE_M + 2 * STAGES;
    let smem_bytes = (smem_u64 * std::mem::size_of::<u64>()) as u32;

    // Opt in to >48 KB shared memory (Hopper static default cap).
    gpu.kernel.set_attribute(
        sys::CUfunction_attribute_enum::CU_FUNC_ATTRIBUTE_MAX_DYNAMIC_SHARED_SIZE_BYTES,
        smem_bytes as i32,
    )?;

    // Persistent grid: co-resident CTAs = (occupancy per SM) × SM count, so the grid exactly fills
    // the machine and the kernel's persistent loop sweeps all output tiles in grouped-rasterized
    // order. The CTAs are independent, so the grid size is a throughput parameter only: any size is
    // correct, and fewer CTAs simply do more tile-iterations each.
    let sms = gpu
        .ctx
        .attribute(sys::CUdevice_attribute_enum::CU_DEVICE_ATTRIBUTE_MULTIPROCESSOR_COUNT)?
        as u32;
    let occ = gpu
        .kernel
        .occupancy_max_active_blocks_per_multiprocessor(THREADS, smem_bytes as usize, None)?
        .max(1);
    let mut num_ctas = (occ * sms).max(1);
    // Diagnostic: cap the persistent grid to probe how much of a small GEMM's
    // time is the persistent-grid startup (mbar init + pipeline fill across
    // occ×SMs CTAs). The persistent loop handles any grid size; fewer CTAs just
    // do more tile-iterations each.
    if let Some(cap) = std::env::var("FP_CUDA_GEMM_CTAS")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
    {
        num_ctas = cap.max(1);
    }
    if std::env::var("FP_CUDA_DEBUG").is_ok() {
        eprintln!("[fp-cuda] occ={occ}/SM sms={sms} num_ctas={num_ctas} smem={smem_bytes}B");
    }

    let ta = TmaArg(tma_a);
    let tb = TmaArg(tma_b);
    let tc = TmaArg(tma_c);
    let mt = m_tiles as u32;
    let ng = n_groups as u32;
    let m_val = m_padded as u32;
    let k_val = k_padded as u32;

    let launch = || -> Result<(), cudarc::driver::DriverError> {
        let cfg = LaunchConfig {
            grid_dim: (num_ctas, 1, 1),
            block_dim: (THREADS, 1, 1),
            shared_mem_bytes: smem_bytes,
        };
        let mut lb = stream.launch_builder(&gpu.kernel);
        lb.arg(&ta)
            .arg(&tb)
            .arg(&tc)
            .arg(&mt)
            .arg(&ng)
            .arg(&m_val)
            .arg(&k_val);
        // SAFETY: the seven pushed arguments match `matmul_b1_kernel`'s parameter list in order and
        // type, `smem_bytes` is the size the kernel was granted above via
        // MAX_DYNAMIC_SHARED_SIZE_BYTES, and the device buffers behind the tensor maps outlive this
        // closure (their guards are held until after the final synchronize).
        unsafe { lb.launch(cfg) }?;
        Ok(())
    };

    // Warm up once (untimed) when measuring, so the timed loop excludes any first-launch
    // JIT/allocation costs.
    if time_iters > 1 {
        launch()?;
        stream.synchronize()?;
    }

    let start = Instant::now();
    for _ in 0..time_iters {
        launch()?;
    }
    stream.synchronize()?;
    let kernel_secs = start.elapsed().as_secs_f64() / time_iters as f64;

    Ok(kernel_secs)
}

/// A 1D launch config: `ceil(total / 256)` blocks of 256 threads.
///
/// # The launch contract
///
/// Every kernel launch below is `unsafe` for the same two reasons, so the per-site `SAFETY:` notes
/// cite this contract and name only what that site adds:
///
/// 1. the arguments pushed through the `LaunchBuilder` must match the kernel's parameter list in
///    order and by type — the driver reinterprets them blind;
/// 2. every device buffer they reference must outlive the launch, which here means holding its
///    guard to the end of the enclosing method.
///
/// `launch_cooperative` adds a third: the grid must be able to become co-resident, so its size has
/// to come from an occupancy query rather than from the problem size.
fn cfg_1d(total: usize) -> LaunchConfig {
    const T: u32 = 256;
    let blocks = total.div_ceil(T as usize).max(1) as u32;
    LaunchConfig {
        grid_dim: (blocks, 1, 1),
        block_dim: (T, 1, 1),
        shared_mem_bytes: 0,
    }
}

impl GpuContext {
    /// Device-resident F₂ GEMM: [`matmul_b1_raw`]'s operand layout, but already on device and
    /// packed into the wgmma tiles there, so there is no host round-trip. This is the
    /// persistent-buffer primitive the row-reduction port needs.
    ///
    /// Returns `C = A·B` as a **padded** buffer of `m_padded × n_padded_lim` u64 — valid data in
    /// the first `m` rows and first `n.div_ceil(64)` limbs of each — plus its row stride. The
    /// padding is what [`GpuContext::xor_into_region`] consumes; a standalone caller trims it on
    /// readback.
    pub fn matmul_b1_dev(
        &self,
        a_dev: &CudaSlice<u64>,
        m: usize,
        k: usize,
        b_dev: &CudaSlice<u64>,
        n: usize,
    ) -> anyhow::Result<(CudaSlice<u64>, usize)> {
        let sa = k.div_ceil(64);
        self.matmul_b1_dev_strided(a_dev, m, k, sa, b_dev, n)
    }

    /// Like [`matmul_b1_dev`](Self::matmul_b1_dev) but A may be a row-sub-block of
    /// a wider device buffer: `a_row_stride` is A's actual per-row limb stride
    /// (≥ `k.div_ceil(64)`), while only the first `k.div_ceil(64)` limbs of each
    /// row are the operand. Used by the wide-panel forward update, whose
    /// multiplier matrix L is stored with stride `bl` but has only `ceil(pr/64)`
    /// occupied limbs.
    pub fn matmul_b1_dev_strided(
        &self,
        a_dev: &CudaSlice<u64>,
        m: usize,
        k: usize,
        a_row_stride: usize,
        b_dev: &CudaSlice<u64>,
        n: usize,
    ) -> anyhow::Result<(CudaSlice<u64>, usize)> {
        let sa = k.div_ceil(64);
        let n_lim = n.div_ceil(64);
        assert!(a_row_stride >= sa, "a_row_stride must cover k limbs");
        assert_eq!(a_dev.len(), m * a_row_stride, "A limb count mismatch");
        assert_eq!(b_dev.len(), k * n_lim, "B limb count mismatch");

        let k_padded = k.next_multiple_of(TILE_K);
        let m_padded = m.next_multiple_of(TILE_M);
        let m_tiles = m_padded / TILE_M;
        let k_chunks = k_padded / TILE_K;
        let n_groups = n_lim.div_ceil(NG as usize);
        let n_padded_lim = n_groups * NG as usize;

        let stream = self.stream();

        // Pack A → interleaved row-major K-major tiles (m_padded × k_padded/64).
        // pack_a/pack_b/the GEMM fully overwrite these buffers (padding written as
        // explicit zeros), so allocate uninitialized and skip the memset — the
        // per-call zeroing of the multi-GB C output is pure waste at large n.
        let a_int_len = m_padded * (k_padded / 64);
        // SAFETY: `alloc` hands back uninitialized device memory. `pack_a` below writes all
        // `a_int_len` elements — the padding as explicit zeros — before anything reads the buffer.
        let a_int = unsafe { stream.alloc::<u64>(a_int_len) }?;
        {
            let (m_orig, sa_orig, a_str, mt, total) = (
                m as u32,
                sa as u32,
                a_row_stride as u32,
                m_tiles as u32,
                a_int_len as u32,
            );
            let mut lb = stream.launch_builder(&self.pack_a);
            lb.arg(&a_int)
                .arg(a_dev)
                .arg(&m_orig)
                .arg(&sa_orig)
                .arg(&a_str)
                .arg(&mt)
                .arg(&total);
            // SAFETY: launch contract for `pack_a`.
            unsafe { lb.launch(cfg_1d(a_int_len)) }?;
        }

        // Pack B → bit-transposed K-major tiles.
        let bt_len = k_chunks * n_groups * (NG as usize * 64 * KL);
        // SAFETY: as for `a_int` above: `pack_b` writes all `bt_len` elements before any read.
        let bt = unsafe { stream.alloc::<u64>(bt_len) }?;
        {
            let (k_orig, nl, ng, total) = (k as u32, n_lim as u32, n_groups as u32, bt_len as u32);
            let mut lb = stream.launch_builder(&self.pack_b);
            lb.arg(&bt)
                .arg(b_dev)
                .arg(&k_orig)
                .arg(&nl)
                .arg(&ng)
                .arg(&total);
            // SAFETY: launch contract for `pack_b`.
            unsafe { lb.launch(cfg_1d(bt_len)) }?;
        }

        // The GEMM writes C with a bulk-tensor store (overwrite, not accumulate), covering every
        // output tile; xor_into only reads the first m rows and trailing_limbs columns, all
        // written. So C needs no pre-zeroing. SAFETY: uninitialized is sound because the GEMM's
        // bulk-tensor store overwrites every output tile, as the comment above records.
        let c_dev = unsafe { stream.alloc::<u64>(m_padded * n_padded_lim) }?;
        run_gemm_kernel(
            self,
            &stream,
            &a_int,
            &bt,
            &c_dev,
            m_padded,
            k_padded,
            n_groups,
            n_padded_lim,
            1,
        )?;

        Ok((c_dev, n_padded_lim))
    }

    /// XOR a padded GEMM output `c_dev` (`m × c_stride` u64/row, as returned by
    /// [`GpuContext::matmul_b1_dev`]) into a region of a persistent device matrix
    /// `dst`: `dst[j][dst_limb + col] ^= c_dev[j][col]` for `j < m`, `col <
    /// width`. This is the fused trailing-update / back-sub epilogue: the `M[region] ^= L·U`
    /// update with no fresh C alloc + D2H.
    #[allow(clippy::too_many_arguments)]
    pub fn xor_into_region(
        &self,
        stream: &Arc<CudaStream>,
        dst: &mut CudaSlice<u64>,
        c_dev: &CudaSlice<u64>,
        m: usize,
        width: usize,
        dst_stride: usize,
        dst_limb: usize,
        c_stride: usize,
    ) -> anyhow::Result<()> {
        let total = m * width;
        let (mm, w, ds, dl, cs) = (
            m as u32,
            width as u32,
            dst_stride as u32,
            dst_limb as u32,
            c_stride as u32,
        );
        let mut lb = stream.launch_builder(&self.xor_into);
        lb.arg(dst)
            .arg(c_dev)
            .arg(&mm)
            .arg(&w)
            .arg(&ds)
            .arg(&dl)
            .arg(&cs);
        // SAFETY: launch contract for `xor_into`.
        unsafe { lb.launch(cfg_1d(total)) }?;
        Ok(())
    }

    /// Convenience wrapper: upload two host operands, run [`matmul_b1_dev`], and
    /// download the trimmed `m × n.div_ceil(64)` product. Same signature and
    /// result as [`matmul_b1_raw`] but exercising the device-resident packing +
    /// GEMM path end to end — used to validate that path bit-for-bit against the
    /// host oracle.
    ///
    /// [`matmul_b1_dev`]: GpuContext::matmul_b1_dev
    pub fn matmul_b1_dev_roundtrip(
        &self,
        a: &[u64],
        m: usize,
        k: usize,
        b: &[u64],
        n: usize,
    ) -> anyhow::Result<Vec<u64>> {
        let n_lim = n.div_ceil(64);
        let stream = self.stream();
        let a_dev = stream.clone_htod(a)?;
        let b_dev = stream.clone_htod(b)?;
        let (c_dev, n_padded_lim) = self.matmul_b1_dev(&a_dev, m, k, &b_dev, n)?;
        let c_all = stream.clone_dtoh(&c_dev)?;
        Ok(c_all
            .chunks_exact(n_padded_lim)
            .take(m)
            .flat_map(|row| row[..n_lim].iter().copied())
            .collect())
    }

    /// Upload a bit-packed F₂ matrix (natural row-major limb layout, `rows *
    /// cols.div_ceil(64)` u64) to device memory. One H2D copy.
    pub fn upload(&self, data: &[u64], rows: usize, cols: usize) -> anyhow::Result<DeviceMatrix> {
        let stride = cols.div_ceil(64);
        assert_eq!(data.len(), rows * stride, "limb count mismatch");
        let buf = self.stream().clone_htod(data)?;
        Ok(DeviceMatrix {
            buf,
            rows,
            cols,
            stride,
        })
    }

    /// Download a [`DeviceMatrix`] back to host limbs (natural layout). One D2H.
    pub fn download(&self, dm: &DeviceMatrix) -> anyhow::Result<Vec<u64>> {
        Ok(self.stream().clone_dtoh(&dm.buf)?)
    }

    /// Download a device `u32` buffer (e.g. a `perm` vector) to host.
    pub fn download_u32(&self, s: &CudaSlice<u32>) -> anyhow::Result<Vec<u32>> {
        Ok(self.stream().clone_dtoh(s)?)
    }

    /// The fused trailing-update / back-substitution epilogue over persistent device buffers:
    /// `dst[:, col_off:] ^= L · U`, in place, where `L` is `dst.rows × k` and `U` is `k × t`
    /// (`k = l.cols = u.rows`,
    /// `t = u.cols`). `col_off` must be a limb boundary (multiple of 64) and the
    /// trailing region `[col_off, dst.cols)` must be exactly `t` columns wide —
    /// i.e. `col_off + t == dst.cols` — so the whole-limb XOR lands correctly.
    ///
    /// Runs [`matmul_b1_dev`](Self::matmul_b1_dev) into a scratch device C and
    /// XORs it into `dst` with [`xor_into_region`](Self::xor_into_region) — no
    /// host round-trip, no D2H of the product.
    pub fn gemm_xor_into(
        &self,
        dst: &mut DeviceMatrix,
        l: &DeviceMatrix,
        u: &DeviceMatrix,
        col_off: usize,
    ) -> anyhow::Result<()> {
        assert_eq!(col_off % 64, 0, "col_off must be a limb boundary");
        assert_eq!(l.rows, dst.rows, "L rows must match dst rows");
        assert_eq!(
            l.cols, u.rows,
            "inner dimension mismatch (L.cols != U.rows)"
        );
        assert_eq!(
            col_off + u.cols,
            dst.cols,
            "trailing region must be U.cols wide"
        );
        let (m, k, t) = (dst.rows, l.cols, u.cols);
        if m == 0 || k == 0 || t == 0 {
            return Ok(());
        }
        let (c_dev, _n_padded_lim) = self.matmul_b1_dev(&l.buf, m, k, &u.buf, t)?;
        let width = t.div_ceil(64); // == dst.stride - col_off/64
        let stream = self.stream();
        self.xor_into_region(
            &stream,
            &mut dst.buf,
            &c_dev,
            m,
            width,
            dst.stride,
            col_off / 64,
            _n_padded_lim,
        )?;
        stream.synchronize()?;
        Ok(())
    }

    /// Allocate the identity virtual-row permutation `perm = [0, 1, …, m-1]` on device. Kernels
    /// dereference rows as `M[perm[i]]`; row swaps are `perm` swaps, so the matrix bytes never
    /// move.
    pub fn identity_perm(&self, m: usize) -> anyhow::Result<CudaSlice<u32>> {
        let host: Vec<u32> = (0..m as u32).collect();
        Ok(self.stream().clone_htod(&host)?)
    }

    /// Factor one 64-bit column panel (limb `plimb`) in place over the persistent buffer,
    /// forward-only, starting from pivot row `r` — the b=64 base kernel. Rows are addressed
    /// through `perm`; a pivot is promoted by swapping its `perm` entry to position
    /// `r + pivot_index`. The
    /// multiplier bits captured while clearing rows *below* each pivot are ORed
    /// into `l` (indexed by original row id, so `l` needs no permutation).
    ///
    /// `l` must be an `m × (≥64-column)` device matrix (one limb per row is
    /// enough since a panel yields ≤ 64 pivots); the caller zeroes the panel's
    /// pivot-index bits before the call. Returns `(pr, pivcols)` — the pivots
    /// found and their absolute columns — read back to host (a few hundred
    /// bytes); everything else stays on device.
    pub fn panel_factor(
        &self,
        m: &mut DeviceMatrix,
        perm: &mut CudaSlice<u32>,
        l: &mut DeviceMatrix,
        plimb: usize,
        r: usize,
    ) -> anyhow::Result<(usize, Vec<u32>)> {
        assert_eq!(perm.len(), m.rows, "perm length must equal rows");
        assert_eq!(l.rows, m.rows, "L rows must equal M rows");
        let stream = self.stream();

        const THREADS: u32 = 256;
        let pivcols = stream.alloc_zeros::<u32>(64)?;
        let pr_out = stream.alloc_zeros::<u32>(1)?;

        let (plimb_u, r_u, n_u, m_u, stride_u, l_stride_u) = (
            plimb as u32,
            r as u32,
            m.cols as u32,
            m.rows as u32,
            m.stride as u32,
            l.stride as u32,
        );
        let cfg = LaunchConfig {
            grid_dim: (1, 1, 1),
            block_dim: (THREADS, 1, 1),
            shared_mem_bytes: THREADS * std::mem::size_of::<i32>() as u32,
        };
        let mut lb = stream.launch_builder(&self.panel_factor);
        lb.arg(&mut m.buf)
            .arg(perm)
            .arg(&mut l.buf)
            .arg(&pivcols)
            .arg(&pr_out)
            .arg(&plimb_u)
            .arg(&r_u)
            .arg(&n_u)
            .arg(&m_u)
            .arg(&stride_u)
            .arg(&l_stride_u);
        // SAFETY: launch contract for `panel_factor`.
        // `shared_mem_bytes` is the dynamic SMEM the kernel declares.
        unsafe { lb.launch(cfg) }?;

        let pr = stream.clone_dtoh(&pr_out)?[0] as usize;
        let cols = stream.clone_dtoh(&pivcols)?;
        Ok((pr, cols[..pr].to_vec()))
    }

    /// Grid-parallel (cooperative) equivalent of [`panel_factor`](Self::panel_factor):
    /// identical math, but each of the 64 sequential bit-steps spreads its
    /// find-first and masked-XOR across the whole grid, with a self-contained
    /// atomic grid barrier between steps. The single-CTA `panel_factor` uses one
    /// SM of ~132 and dominates the forward pass (~76% of GPU time profiled); this
    /// removes that bottleneck. Launched via `cuLaunchCooperativeKernel` with all
    /// CTAs co-resident (required by the spin barrier). Same `(pr, pivcols)`
    /// read-back.
    pub fn panel_factor_coop(
        &self,
        m: &mut DeviceMatrix,
        perm: &mut CudaSlice<u32>,
        l: &mut DeviceMatrix,
        ppanel: usize,
        bl: usize,
        r: usize,
        m_active: usize,
    ) -> anyhow::Result<(usize, Vec<u32>)> {
        assert_eq!(perm.len(), m.rows, "perm length must equal rows");
        assert_eq!(l.rows, m.rows, "L rows must equal M rows");
        assert!(l.stride >= bl, "L stride must be at least bl");
        assert!(m_active <= m.rows && m_active >= r, "m_active out of range");
        let stream = self.stream();

        const THREADS: u32 = 256;
        let smem = THREADS * std::mem::size_of::<i32>() as u32;
        // Up to bl·64 pivots in a wide panel.
        let pivcols = stream.alloc_zeros::<u32>(bl * 64)?;
        let pr_out = stream.alloc_zeros::<u32>(1)?;
        // scratch = [barrier(0), g_min, g_pr]; alloc_zeros gives barrier == 0.
        let scratch = stream.alloc_zeros::<u32>(3)?;
        let g_pivword = stream.alloc_zeros::<u64>(bl)?;

        // Co-resident grid: at most occ×SMs CTAs (cooperative launch cap), and no
        // more than needed to cover the rows once.
        let sms = self
            .ctx
            .attribute(sys::CUdevice_attribute_enum::CU_DEVICE_ATTRIBUTE_MULTIPROCESSOR_COUNT)?
            as u32;
        let occ = self
            .panel_factor_coop
            .occupancy_max_active_blocks_per_multiprocessor(THREADS, smem as usize, None)?
            .max(1);
        // Only rows [r, m_active) are scanned/updated (dead rows excluded), so
        // size the grid and the kernel's row bound to m_active.
        let rows_worth = (m_active as u32).div_ceil(THREADS).max(1);
        let num_ctas = (occ * sms).min(rows_worth).max(1);
        // panel_factor is partly grid-barrier-bound (a grid_sync per pivot bit), so a smaller grid
        // gives cheaper barriers while still covering the panel work. See EXPERIMENTS.md.
        let num_ctas = std::env::var("FP_CUDA_PF_CTAS")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(DEFAULT_PF_CTAS)
            .clamp(1, num_ctas);

        let (ppanel_u, bl_u, r_u, n_u, m_u, stride_u, l_stride_u, tc) = (
            ppanel as u32,
            bl as u32,
            r as u32,
            m.cols as u32,
            m_active as u32,
            m.stride as u32,
            l.stride as u32,
            num_ctas,
        );
        let cfg = LaunchConfig {
            grid_dim: (num_ctas, 1, 1),
            block_dim: (THREADS, 1, 1),
            shared_mem_bytes: smem,
        };
        let mut lb = stream.launch_builder(&self.panel_factor_coop);
        lb.arg(&mut m.buf)
            .arg(perm)
            .arg(&mut l.buf)
            .arg(&pivcols)
            .arg(&pr_out)
            .arg(&scratch)
            .arg(&g_pivword)
            .arg(&ppanel_u)
            .arg(&bl_u)
            .arg(&r_u)
            .arg(&n_u)
            .arg(&m_u)
            .arg(&stride_u)
            .arg(&l_stride_u)
            .arg(&tc);
        // SAFETY: launch contract for `panel_factor_coop`. Its grid comes from the occupancy query.
        unsafe { lb.launch_cooperative(cfg) }?;

        let pr = stream.clone_dtoh(&pr_out)?[0] as usize;
        let cols = stream.clone_dtoh(&pivcols)?;
        Ok((pr, cols[..pr].to_vec()))
    }

    /// **Streamed** (kernel-boundary) equivalent of
    /// [`panel_factor_coop`](Self::panel_factor_coop): identical math and the same
    /// all-SM parallelism, but each of the ≤ `bl·64` sequential bit-steps is three
    /// ordinary grid-wide launches (`pf_find` → `pf_swap` → `pf_xor`) whose stream
    /// ordering replaces the cooperative kernel's in-grid `grid_sync`. No
    /// `cuLaunchCooperativeKernel`, so no all-CTAs-co-resident requirement — it
    /// composes with a concurrent kernel from another CUDA runtime instead of
    /// deadlocking the grid barrier. All per-step state (`g_pr`, `g_min`,
    /// `g_pivpos`, `g_pivword`) lives on the device, so the host issues every launch
    /// without a readback and their latency hides behind the GPU work; only the
    /// final `(pr, pivcols)` is copied back. Bit-for-bit equal to the coop kernel.
    #[allow(clippy::too_many_arguments)]
    pub fn panel_factor_streamed(
        &self,
        m: &mut DeviceMatrix,
        perm: &mut CudaSlice<u32>,
        l: &mut DeviceMatrix,
        ppanel: usize,
        bl: usize,
        r: usize,
        m_active: usize,
    ) -> anyhow::Result<(usize, Vec<u32>)> {
        assert_eq!(perm.len(), m.rows, "perm length must equal rows");
        assert_eq!(l.rows, m.rows, "L rows must equal M rows");
        assert!(l.stride >= bl, "L stride must be at least bl");
        assert!(m_active <= m.rows && m_active >= r, "m_active out of range");
        let stream = self.stream();

        const THREADS: u32 = 256;
        const INF: i32 = 0x7fff_ffff;
        let smem = THREADS * std::mem::size_of::<i32>() as u32;

        // Device-resident per-step state: pivot count, find-first result (INF =
        // none), this step's pivot position (pf_xor's guard), the pivot row's bl
        // panel limbs, and the last-CTA-finalize arrival counter. g_min starts INF;
        // g_pr and arrival start 0 (arrival self-resets to 0 each step).
        let pivcols = stream.alloc_zeros::<u32>(bl * 64)?;
        let mut g_pr = stream.alloc_zeros::<u32>(1)?;
        let mut g_min = stream.clone_htod(&[INF])?;
        let g_pivpos = stream.alloc_zeros::<i32>(1)?;
        let mut g_pivword = stream.alloc_zeros::<u64>(bl)?;
        let mut arrival = stream.alloc_zeros::<u32>(1)?;

        // Regular launches wave-schedule, so the grid can be sized purely to cover
        // the active rows once; cap at occ×SMs for launch efficiency.
        let sms = self
            .ctx
            .attribute(sys::CUdevice_attribute_enum::CU_DEVICE_ATTRIBUTE_MULTIPROCESSOR_COUNT)?
            as u32;
        let occ = self
            .pf_xor
            .occupancy_max_active_blocks_per_multiprocessor(THREADS, 0, None)?
            .max(1);
        let rows_worth = (m_active as u32).div_ceil(THREADS).max(1);
        let num_ctas = (occ * sms).min(rows_worth).max(1);
        // Each step's grid-wide min-reduce + last-CTA finalize contends on g_min / arrival across
        // all CTAs, so — as for the cooperative kernel above — a smaller grid makes every step
        // cheaper once it still covers the rows.
        let num_ctas = std::env::var("FP_CUDA_PF_CTAS")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(DEFAULT_PF_CTAS)
            .clamp(1, num_ctas);

        let (r_u, m_u, n_u, stride_u, l_stride_u, ppanel_u, bl_u) = (
            r as u32,
            m_active as u32,
            m.cols as u32,
            m.stride as u32,
            l.stride as u32,
            ppanel as u32,
            bl as u32,
        );
        let find_cfg = LaunchConfig {
            grid_dim: (num_ctas, 1, 1),
            block_dim: (THREADS, 1, 1),
            shared_mem_bytes: smem,
        };
        let grid_cfg = LaunchConfig {
            grid_dim: (num_ctas, 1, 1),
            block_dim: (THREADS, 1, 1),
            shared_mem_bytes: 0,
        };

        // Valid columns in this panel (the last panel may be short).
        let ncols = (bl * 64).min(m.cols - ppanel * 64);

        // (0) find+swap the first column's pivot (no trailing XOR yet).
        {
            let (plimb_u, j_u, cc_u) = (ppanel_u, 0u32, 0u32);
            let mut lb = stream.launch_builder(&self.pf_find_swap);
            lb.arg(&mut m.buf)
                .arg(&mut *perm)
                .arg(&pivcols)
                .arg(&mut g_pivword)
                .arg(&mut g_min)
                .arg(&mut g_pr)
                .arg(&g_pivpos)
                .arg(&mut arrival)
                .arg(&ppanel_u)
                .arg(&bl_u)
                .arg(&j_u)
                .arg(&plimb_u)
                .arg(&cc_u)
                .arg(&r_u)
                .arg(&m_u)
                .arg(&stride_u)
                .arg(&n_u);
            // SAFETY: launch contract for `pf_step`.
            unsafe { lb.launch(find_cfg) }?;
        }

        // (1) fused lookahead: each step clears column cc-1 and finds+swaps column
        // cc — one launch per column instead of two, with the row read once.
        for cc in 1..ncols {
            let cc_u = cc as u32;
            let mut lb = stream.launch_builder(&self.pf_step);
            lb.arg(&mut m.buf)
                .arg(&mut *perm)
                .arg(&mut l.buf)
                .arg(&pivcols)
                .arg(&mut g_pivword)
                .arg(&mut g_min)
                .arg(&mut g_pr)
                .arg(&g_pivpos)
                .arg(&mut arrival)
                .arg(&ppanel_u)
                .arg(&bl_u)
                .arg(&cc_u)
                .arg(&r_u)
                .arg(&m_u)
                .arg(&stride_u)
                .arg(&l_stride_u)
                .arg(&n_u);
            // SAFETY: as for the `pf_step` launch above.
            unsafe { lb.launch(find_cfg) }?;
        }

        // (2) clear the final column's pivot from the rows below.
        {
            let (cc_u, j_u) = ((ncols - 1) as u32, ((ncols - 1) & 63) as u32);
            let mut lb = stream.launch_builder(&self.pf_xor);
            lb.arg(&mut m.buf)
                .arg(&*perm)
                .arg(&mut l.buf)
                .arg(&g_pivword)
                .arg(&g_pivpos)
                .arg(&g_pr)
                .arg(&ppanel_u)
                .arg(&bl_u)
                .arg(&cc_u)
                .arg(&j_u)
                .arg(&r_u)
                .arg(&m_u)
                .arg(&stride_u)
                .arg(&l_stride_u);
            // SAFETY: launch contract for `pf_xor`.
            unsafe { lb.launch(grid_cfg) }?;
        }

        let pr = stream.clone_dtoh(&g_pr)?[0] as usize;
        let cols = stream.clone_dtoh(&pivcols)?;
        Ok((pr, cols[..pr].to_vec()))
    }

    /// Active-row compaction: mark the below rows [r, m_active) that are entirely zero across
    /// the remaining columns [start_limb·64, n) —
    /// permanently dead (they can never pivot and carry no multiplier) — and
    /// stable-partition `perm[r..m_active]` so the live rows come first. Returns
    /// the new active count `r + live`. The dead rows are parked in
    /// [new_active, m_active) and never scanned again; pivot rows [0, r) are
    /// untouched. The partition is done on the host (a few hundred KB of perm +
    /// flags per call), which is negligible beside the panel work it saves.
    fn compact_perm(
        &self,
        m: &DeviceMatrix,
        perm: &mut CudaSlice<u32>,
        r: usize,
        m_active: usize,
        start_limb: usize,
    ) -> anyhow::Result<usize> {
        if m_active <= r {
            return Ok(m_active);
        }
        let stream = self.stream();
        let n_scan = m_active - r;
        // SAFETY: uninitialized is sound because `mark_live` writes all `n_scan` entries before the
        // `clone_dtoh` below reads them.
        let live = unsafe { stream.alloc::<u32>(n_scan) }?;
        {
            let (r_u, m_u, sl, st) = (
                r as u32,
                m_active as u32,
                start_limb as u32,
                m.stride as u32,
            );
            let mut lb = stream.launch_builder(&self.mark_live);
            lb.arg(&live)
                .arg(&m.buf)
                .arg(&*perm)
                .arg(&r_u)
                .arg(&m_u)
                .arg(&sl)
                .arg(&st);
            // SAFETY: launch contract for `mark_live`.
            unsafe { lb.launch(cfg_1d(n_scan)) }?;
        }
        let live_host = stream.clone_dtoh(&live)?;
        let perm_host = stream.clone_dtoh(&perm.slice(r..m_active))?;
        // Stable partition: live rows first (preserve order), dead rows after.
        let mut ordered: Vec<u32> = Vec::with_capacity(n_scan);
        for (i, &lv) in live_host.iter().enumerate() {
            if lv != 0 {
                ordered.push(perm_host[i]);
            }
        }
        let new_active = r + ordered.len();
        for (i, &lv) in live_host.iter().enumerate() {
            if lv == 0 {
                ordered.push(perm_host[i]);
            }
        }
        let mut view = perm.slice_mut(r..m_active);
        stream.memcpy_htod(&ordered, &mut view)?;
        Ok(new_active)
    }

    /// Elementwise promote of the `pr` pivot rows (perm positions
    /// `[r_piv, r_piv+pr)`): the cooperative forward-substitution replay of `L`
    /// onto those rows' trailing. `l_limb_off` shifts the L read so bit `i` maps
    /// to global multiplier bit `l_limb_off·64 + i` (0 for the whole-panel promote).
    #[allow(clippy::too_many_arguments)]
    fn promote_elem(
        &self,
        m: &mut DeviceMatrix,
        perm: &CudaSlice<u32>,
        l: &DeviceMatrix,
        r_piv: usize,
        pr: usize,
        first_limb: usize,
        trailing_limbs: usize,
        l_limb_off: usize,
        pc_barrier: &mut CudaSlice<u32>,
        pc_cond: &CudaSlice<u32>,
        pc_ctas: u32,
    ) -> anyhow::Result<()> {
        if pr == 0 || trailing_limbs == 0 {
            return Ok(());
        }
        let stream = self.stream();
        stream.memset_zeros(pc_barrier)?;
        let (r_u, pr_u, fl, tl, st, ls, llo, tc) = (
            r_piv as u32,
            pr as u32,
            first_limb as u32,
            trailing_limbs as u32,
            m.stride as u32,
            l.stride as u32,
            l_limb_off as u32,
            pc_ctas,
        );
        let cfg = LaunchConfig {
            grid_dim: (pc_ctas, 1, 1),
            block_dim: (256, 1, 1),
            shared_mem_bytes: 0,
        };
        let mut lb = stream.launch_builder(&self.promote_coop);
        lb.arg(&mut m.buf)
            .arg(perm)
            .arg(&l.buf)
            .arg(&r_u)
            .arg(&pr_u)
            .arg(&fl)
            .arg(&tl)
            .arg(&st)
            .arg(&ls)
            .arg(&llo)
            .arg(pc_barrier)
            .arg(pc_cond)
            .arg(&tc);
        // SAFETY: launch contract for `promote_coop`. Its grid comes from the occupancy query.
        unsafe { lb.launch_cooperative(cfg) }?;
        Ok(())
    }

    /// One trailing update `M[:, first_limb·64 : end_limb·64) ^= L · U`: promote the `pr` pivot
    /// rows at perm positions `[r_piv, r_piv+pr)`
    /// over the column range, drop them from `l`, gather them into `U`, run the
    /// GEMM, and XOR the product into the region. Shared by the single-wide-panel
    /// far update and the recursive intra-panel updates; `l` carries the pivots'
    /// multipliers at bits `[0, pr)` (stride `l.stride`). `pc_*` are the
    /// cooperative-promote scaffolding, created once by the caller.
    #[allow(clippy::too_many_arguments)]
    fn trailing_update(
        &self,
        m: &mut DeviceMatrix,
        perm: &CudaSlice<u32>,
        l: &mut DeviceMatrix,
        r_piv: usize,
        pr: usize,
        first_limb: usize,
        end_limb: usize,
        promote_coop: bool,
        pc_barrier: &mut CudaSlice<u32>,
        pc_cond: &CudaSlice<u32>,
        pc_ctas: u32,
    ) -> anyhow::Result<()> {
        let stream = self.stream();
        let (rows, stride, n) = (m.rows, m.stride, m.cols);
        let trailing_limbs = end_limb - first_limb;
        if pr == 0 || trailing_limbs == 0 {
            return Ok(());
        }
        let t = (end_limb * 64).min(n) - first_limb * 64;

        // (1) promote pivot rows' trailing, then (2) zero their L rows.
        {
            let (r_u, pr_u, fl, tl, st, ls) = (
                r_piv as u32,
                pr as u32,
                first_limb as u32,
                trailing_limbs as u32,
                stride as u32,
                l.stride as u32,
            );
            if promote_coop {
                self.promote_elem(
                    m,
                    perm,
                    l,
                    r_piv,
                    pr,
                    first_limb,
                    trailing_limbs,
                    0,
                    pc_barrier,
                    pc_cond,
                    pc_ctas,
                )?;
            } else {
                let mut lb = stream.launch_builder(&self.promote_pivots);
                lb.arg(&mut m.buf)
                    .arg(perm)
                    .arg(&l.buf)
                    .arg(&r_u)
                    .arg(&pr_u)
                    .arg(&fl)
                    .arg(&tl)
                    .arg(&st)
                    .arg(&ls);
                // SAFETY: launch contract for `promote_pivots`.
                unsafe { lb.launch(cfg_1d(trailing_limbs)) }?;
            }
            let (r_u, pr_u, ls) = (r_piv as u32, pr as u32, l.stride as u32);
            let mut lb = stream.launch_builder(&self.zero_pivot_l);
            lb.arg(perm).arg(&mut l.buf).arg(&r_u).arg(&pr_u).arg(&ls);
            // SAFETY: launch contract for `zero_pivot_l`.
            unsafe { lb.launch(cfg_1d(pr)) }?;
        }

        // (3) gather U = pivot rows' trailing (pr × trailing_limbs). SAFETY: uninitialized is sound
        // because `gather_rows` below writes all `pr * trailing_limbs` elements before the GEMM
        // reads them.
        let u_buf = unsafe { stream.alloc::<u64>(pr * trailing_limbs) }?;
        {
            let (r_u, fl, pr_u, nc, st) = (
                r_piv as u32,
                first_limb as u32,
                pr as u32,
                trailing_limbs as u32,
                stride as u32,
            );
            let mut lb = stream.launch_builder(&self.gather_rows);
            lb.arg(&u_buf)
                .arg(&m.buf)
                .arg(perm)
                .arg(&r_u)
                .arg(&fl)
                .arg(&pr_u)
                .arg(&nc)
                .arg(&st);
            // SAFETY: launch contract for `gather_rows`.
            unsafe { lb.launch(cfg_1d(pr * trailing_limbs)) }?;
        }

        // (4) GEMM C = L(m×pr)·U(pr×t); M[:, first_limb:] ^= C.
        let (c_dev, n_padded_lim) =
            self.matmul_b1_dev_strided(&l.buf, rows, pr, l.stride, &u_buf, t)?;
        self.xor_into_region(
            &stream,
            &mut m.buf,
            &c_dev,
            rows,
            trailing_limbs,
            stride,
            first_limb,
            n_padded_lim,
        )?;
        Ok(())
    }

    /// Forward pass of the blocked row reduction over the persistent device
    /// buffer: sweep 64-bit panels left to right, and for each —
    /// factor it ([`panel_factor`](Self::panel_factor)), promote the pivot rows'
    /// deferred trailing, drop the pivots from the multiplier matrix, and apply
    /// the trailing update `M[:, c+b:] ^= L·U` as one wgmma GEMM. Leaves `m` in
    /// **row-echelon** form addressed through `perm`: the `rank` pivot rows at
    /// perm positions `[0, rank)`, each reduced by earlier pivots; the rest zero.
    ///
    /// Returns `(perm, rank, pivot_cols)`. Back-substitution to RREF is a
    /// separate pass (the mirror-image blocked update over the pivot rows).
    pub fn forward_reduce(
        &self,
        m: &mut DeviceMatrix,
    ) -> anyhow::Result<(CudaSlice<u32>, usize, Vec<usize>)> {
        let stream = self.stream();
        let (rows, stride) = (m.rows, m.stride);
        let mut perm = self.identity_perm(rows)?;
        let mut r = 0usize;
        let mut pivot_cols = Vec::new();
        // Cooperative vs. composable kernels (see [`rr_coop`]). The non-cooperative
        // path never launches a cooperative grid, so it composes with concurrent GPU
        // work at the cost of the single-CTA panel factor; the cooperative path is the
        // faster exclusive-GPU mode.
        let coop = rr_coop();

        // Panel width in limbs; see [`adaptive_bl`]. Both panel-factor paths handle wide panels,
        // so it is chosen the same way in either mode.
        let bl = if let Some(v) = std::env::var("FP_CUDA_BL")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
        {
            v.clamp(1, stride.max(1))
        } else {
            adaptive_bl(stride).clamp(1, stride.max(1))
        };

        // Active-row compaction: park permanently-dead below rows past m_active
        // so panel_factor stops scanning them. Re-scanned every COMPACT_PERIOD
        // panels to amortize the mark-and-partition cost.
        const COMPACT_PERIOD: usize = 4;
        let mut m_active = rows;

        // Cooperative multi-CTA promotion (right-looking) replaces the single-CTA
        // triangular replay when the matrix is wide enough to amortize the grid
        // barriers; otherwise the grid-strided promote_pivots kernel is used.
        let use_promote_coop = coop && stride >= 1024;
        let (pc_ctas, mut pc_barrier, pc_cond) = if use_promote_coop {
            let sms = self
                .ctx
                .attribute(sys::CUdevice_attribute_enum::CU_DEVICE_ATTRIBUTE_MULTIPROCESSOR_COUNT)?
                as u32;
            let occ = self
                .promote_coop
                .occupancy_max_active_blocks_per_multiprocessor(256, 0, None)?
                .max(1);
            // promote is a forward-substitution TRSM (1 grid_sync/pivot), partly barrier-bound: a
            // smaller grid gives cheaper barriers while still covering the per-pivot trailing XOR.
            // See EXPERIMENTS.md.
            let full = (occ * sms).max(1);
            let capped = std::env::var("FP_CUDA_PROM_CTAS")
                .ok()
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or(DEFAULT_PROM_CTAS)
                .clamp(1, full);
            // SAFETY: uninitialized is sound because `promote_coop` writes each of the `bl * 64`
            // pivot-word slots before reading it.
            (capped, stream.alloc_zeros::<u32>(1)?, unsafe {
                stream.alloc::<u32>(bl * 64)
            }?)
        } else {
            // SAFETY: the one-element buffer is never read on this branch; it exists only so both
            // arms have the same type.
            (0, stream.alloc_zeros::<u32>(1)?, unsafe {
                stream.alloc::<u32>(1)
            }?)
        };

        let mut ppanel = 0usize;
        let mut panel_idx = 0usize;
        while ppanel < stride {
            let bl_eff = bl.min(stride - ppanel);
            if panel_idx.is_multiple_of(COMPACT_PERIOD) {
                m_active = self.compact_perm(m, &mut perm, r, m_active, ppanel)?;
                // No live below rows left ⇒ no column past here can have a pivot;
                // the remaining panels would all be empty. Stop the forward sweep
                // (huge win for low-rank / rank-deficient inputs).
                if m_active <= r {
                    break;
                }
            }
            // Single wide panel: elementwise cooperative factor + far GEMM.
            let mut l = DeviceMatrix {
                buf: stream.alloc_zeros::<u64>(rows * bl_eff)?,
                rows,
                cols: bl_eff * 64,
                stride: bl_eff,
            };
            let (pr, pivcols) = if coop {
                self.panel_factor_coop(m, &mut perm, &mut l, ppanel, bl_eff, r, m_active)?
            } else {
                // Multi-SM factor via kernel-boundary sync — same math and grid
                // parallelism as the coop kernel, no cooperative launch.
                self.panel_factor_streamed(m, &mut perm, &mut l, ppanel, bl_eff, r, m_active)?
            };
            if pr > 0 {
                for &q in &pivcols {
                    pivot_cols.push(q as usize);
                }
                r += pr;
                self.trailing_update(
                    m,
                    &perm,
                    &mut l,
                    r - pr,
                    pr,
                    ppanel + bl_eff,
                    stride,
                    use_promote_coop,
                    &mut pc_barrier,
                    &pc_cond,
                    pc_ctas,
                )?;
            }
            ppanel += bl_eff;
            panel_idx += 1;
        }
        stream.synchronize()?;
        Ok((perm, r, pivot_cols))
    }

    /// Clear a pivot block's columns from a set of rows *above* it, as one `X·U` GEMM (the
    /// back-substitution Schur update). Clears the pivot columns of block `[block_s, block_e)`
    /// from the rows at perm positions
    /// `[above_start, above_start+above_count)`: gather `X` = those rows' bits at
    /// the block's pivot columns, `U` = the (already-RREF) block rows over their
    /// trailing, `G = X·U`, then scatter-XOR `G` into the above rows.
    ///
    /// Unlike the forward trailing update this needs **no promote**: the source
    /// (block) rows are already fully reduced and disjoint from the target (above)
    /// rows, so it is a clean BLAS3 triangular-solve step. Shared by the outer
    /// back-sub loop (`above = [0, s)`) and the recursive within-block TRSM
    /// (`above` = the left sub-block).
    #[allow(clippy::too_many_arguments)]
    fn bs_clear_above(
        &self,
        m: &mut DeviceMatrix,
        perm: &CudaSlice<u32>,
        piv_dev: &CudaSlice<u32>,
        pivot_cols: &[usize],
        block_s: usize,
        block_e: usize,
        above_start: usize,
        above_count: usize,
    ) -> anyhow::Result<()> {
        if above_count == 0 || block_e <= block_s {
            return Ok(());
        }
        let stream = self.stream();
        let (stride, n) = (m.stride, m.cols);
        let bp_eff = block_e - block_s;
        let start_limb = pivot_cols[block_s] / 64;
        let trailing_limbs = stride - start_limb;
        let width_cols = n - start_limb * 64;
        let x_stride = bp_eff.div_ceil(64);
        // View of the target rows so gather_cols/xor_into_perm read perm at the
        // right offset (they index perm[jpos] for jpos < above_count).
        let perm_above = perm.slice(above_start..above_start + above_count);

        // X = above rows gathered at the block's pivot columns (above_count × bp_eff). SAFETY:
        // uninitialized is sound because `gather_cols` below writes all `above_count * x_stride`
        // elements before the GEMM reads them.
        let x_buf = unsafe { stream.alloc::<u64>(above_count * x_stride) }?;
        {
            let (cs, s_u, cnt, st, xs) = (
                block_s as u32,
                above_count as u32,
                bp_eff as u32,
                stride as u32,
                x_stride as u32,
            );
            let mut lb = stream.launch_builder(&self.gather_cols);
            lb.arg(&x_buf)
                .arg(&m.buf)
                .arg(&perm_above)
                .arg(piv_dev)
                .arg(&cs)
                .arg(&s_u)
                .arg(&cnt)
                .arg(&st)
                .arg(&xs);
            // SAFETY: launch contract for `gather_cols`.
            unsafe { lb.launch(cfg_1d(above_count * x_stride)) }?;
        }

        // U = the (now RREF) block rows, limbs [start_limb, stride). SAFETY: uninitialized is sound
        // because `gather_rows` below writes all `bp_eff * trailing_limbs` elements before the GEMM
        // reads them.
        let u_buf = unsafe { stream.alloc::<u64>(bp_eff * trailing_limbs) }?;
        {
            let (s_u, fl, pr_u, nc, st) = (
                block_s as u32,
                start_limb as u32,
                bp_eff as u32,
                trailing_limbs as u32,
                stride as u32,
            );
            let mut lb = stream.launch_builder(&self.gather_rows);
            lb.arg(&u_buf)
                .arg(&m.buf)
                .arg(perm)
                .arg(&s_u)
                .arg(&fl)
                .arg(&pr_u)
                .arg(&nc)
                .arg(&st);
            // SAFETY: launch contract for `gather_rows`.
            unsafe { lb.launch(cfg_1d(bp_eff * trailing_limbs)) }?;
        }

        // G = X · U (above_count × width_cols); scatter-XOR into the above rows.
        let (c_dev, n_padded_lim) =
            self.matmul_b1_dev(&x_buf, above_count, bp_eff, &u_buf, width_cols)?;
        {
            let (s_u, w, st, fl, cs) = (
                above_count as u32,
                trailing_limbs as u32,
                stride as u32,
                start_limb as u32,
                n_padded_lim as u32,
            );
            let mut lb = stream.launch_builder(&self.xor_into_perm);
            lb.arg(&mut m.buf)
                .arg(&c_dev)
                .arg(&perm_above)
                .arg(&s_u)
                .arg(&w)
                .arg(&st)
                .arg(&fl)
                .arg(&cs);
            // SAFETY: launch contract for `xor_into_perm`.
            unsafe { lb.launch(cfg_1d(above_count * trailing_limbs)) }?;
        }
        Ok(())
    }

    /// Reduce a pivot block `[s, e)` to RREF **among itself**, elementwise: the
    /// triangular solve done as `(e-s)` sequential per-pivot column clears (the
    /// `block_reduce_coop` / `block_reduce_rref` kernels). Cheap for a narrow
    /// block; the base case of [`block_reduce_rec`](Self::block_reduce_rec).
    #[allow(clippy::too_many_arguments)]
    fn block_reduce_elem(
        &self,
        m: &mut DeviceMatrix,
        perm: &CudaSlice<u32>,
        piv_dev: &CudaSlice<u32>,
        s: usize,
        e: usize,
        use_coop: bool,
        streamed: bool,
        br_barrier: &mut CudaSlice<u32>,
        br_cond: &CudaSlice<u32>,
        br_ctas: u32,
    ) -> anyhow::Result<()> {
        if e <= s {
            return Ok(());
        }
        let stream = self.stream();
        let stride = m.stride;
        {
            if use_coop {
                stream.memset_zeros(br_barrier)?;
                let (s_u, e_u, st, tc) = (s as u32, e as u32, stride as u32, br_ctas);
                let cfg = LaunchConfig {
                    grid_dim: (br_ctas, 1, 1),
                    block_dim: (256, 1, 1),
                    shared_mem_bytes: 0,
                };
                let mut lb = stream.launch_builder(&self.block_reduce_coop);
                lb.arg(&mut m.buf)
                    .arg(perm)
                    .arg(piv_dev)
                    .arg(&s_u)
                    .arg(&e_u)
                    .arg(&st)
                    .arg(br_barrier)
                    .arg(br_cond)
                    .arg(&tc);
                // SAFETY: launch contract for `block_reduce_coop`.
                // Its grid comes from the occupancy query.
                unsafe { lb.launch_cooperative(cfg) }?;
            } else if streamed {
                // Kernel-boundary equivalent of block_reduce_coop: per pivot k
                // (high-to-low), br_cond gathers the clear-conditions, then br_xor
                // clears row k from the flagged block rows across the grid. Stream
                // order replaces the cooperative grid barrier.
                let st = stride as u32;
                let xor_cfg = LaunchConfig {
                    grid_dim: (br_ctas, 1, 1),
                    block_dim: (256, 1, 1),
                    shared_mem_bytes: 0,
                };
                let mut k = e;
                while k > s {
                    k -= 1;
                    let (s_u, k_u) = (s as u32, k as u32);
                    let nj = (k - s) as u32;
                    {
                        let mut lb = stream.launch_builder(&self.br_cond);
                        lb.arg(&m.buf)
                            .arg(perm)
                            .arg(piv_dev)
                            .arg(&s_u)
                            .arg(&k_u)
                            .arg(&st)
                            .arg(br_cond);
                        // SAFETY: launch contract for `br_cond`.
                        unsafe { lb.launch(cfg_1d((nj.max(1)) as usize)) }?;
                    }
                    {
                        let mut lb = stream.launch_builder(&self.br_xor);
                        lb.arg(&mut m.buf)
                            .arg(perm)
                            .arg(&s_u)
                            .arg(&k_u)
                            .arg(&st)
                            .arg(br_cond);
                        // SAFETY: launch contract for `br_xor`.
                        unsafe { lb.launch(xor_cfg) }?;
                    }
                }
            } else {
                let (s_u, e_u, st) = (s as u32, e as u32, stride as u32);
                let cfg = LaunchConfig {
                    grid_dim: (1, 1, 1),
                    block_dim: (256, 1, 1),
                    shared_mem_bytes: 0,
                };
                let mut lb = stream.launch_builder(&self.block_reduce_rref);
                lb.arg(&mut m.buf)
                    .arg(perm)
                    .arg(piv_dev)
                    .arg(&s_u)
                    .arg(&e_u)
                    .arg(&st);
                // SAFETY: launch contract for `block_reduce_rref`.
                unsafe { lb.launch(cfg) }?;
            }
        }
        Ok(())
    }

    /// Recursive **blocked TRSM** reduction of a pivot block `[s, e)` to RREF
    /// among itself: split at the midpoint, recurse on the right half, clear its
    /// pivots from the left half with one large `X·U` GEMM
    /// ([`bs_clear_above`](Self::bs_clear_above)), then recurse on the left half.
    /// Below `base_bp` the elementwise
    /// [`block_reduce_elem`](Self::block_reduce_elem) runs.
    ///
    /// This is the BLAS3 form of back-substitution's within-block reduce: it moves the
    /// O(bp²·width) triangular work off the elementwise per-pivot clears and onto the tensor
    /// cores. Because the source and target
    /// rows are disjoint and already reduced, there is **no promote** — the
    /// overhead that made the forward-pass recursion a net loss is absent here.
    #[allow(clippy::too_many_arguments)]
    fn block_reduce_rec(
        &self,
        m: &mut DeviceMatrix,
        perm: &CudaSlice<u32>,
        piv_dev: &CudaSlice<u32>,
        pivot_cols: &[usize],
        s: usize,
        e: usize,
        base_bp: usize,
        use_coop: bool,
        streamed: bool,
        br_barrier: &mut CudaSlice<u32>,
        br_cond: &CudaSlice<u32>,
        br_ctas: u32,
    ) -> anyhow::Result<()> {
        if e - s <= base_bp {
            return self.block_reduce_elem(
                m, perm, piv_dev, s, e, use_coop, streamed, br_barrier, br_cond, br_ctas,
            );
        }
        let mid = s + (e - s) / 2;
        // Right half to RREF, then clear its pivots from the left half (K = e-mid).
        self.block_reduce_rec(
            m, perm, piv_dev, pivot_cols, mid, e, base_bp, use_coop, streamed, br_barrier, br_cond,
            br_ctas,
        )?;
        self.bs_clear_above(m, perm, piv_dev, pivot_cols, mid, e, s, mid - s)?;
        // Left half to RREF (its rows now carry no right-half pivot bits).
        self.block_reduce_rec(
            m, perm, piv_dev, pivot_cols, s, mid, base_bp, use_coop, streamed, br_barrier, br_cond,
            br_ctas,
        )?;
        Ok(())
    }

    /// Back-substitution: turn the row-echelon form left by
    /// [`forward_reduce`](Self::forward_reduce) into full RREF,
    /// blocked right-to-left over pivot blocks. For each block of ≤ 64 pivots
    /// (perm positions `[s, e)`): reduce it among itself, then clear its pivot
    /// columns from every row above `[0, s)` with one `X·U` GEMM. In place over
    /// the persistent buffer through `perm`; `pivot_cols` is the ascending pivot
    /// column list from the forward pass.
    pub fn back_substitute(
        &self,
        m: &mut DeviceMatrix,
        perm: &CudaSlice<u32>,
        r: usize,
        pivot_cols: &[usize],
    ) -> anyhow::Result<()> {
        if r == 0 {
            return Ok(());
        }
        let stream = self.stream();
        let stride = m.stride;
        let piv_dev =
            stream.clone_htod(&pivot_cols.iter().map(|&q| q as u32).collect::<Vec<_>>())?;

        // Multi-CTA block reduction spreads each block's per-pivot clear across the whole grid,
        // which only pays once the block work (≈ bp·stride) is large — hence the width gate; see
        // EXPERIMENTS.md. Two grid-parallel variants (see [`rr_coop`]): `use_coop` is the
        // cooperative kernel for a dedicated GPU, `streamed` the kernel-boundary br_cond/br_xor
        // pair that composes with concurrent work and is the default. Below the gate both fall
        // back to the single-CTA `block_reduce_rref`.
        let wide = stride >= 1024;
        let use_coop = rr_coop() && wide;
        let streamed = !rr_coop() && wide;

        // Pivots per back-substitution block. Wider blocks raise the X·U GEMM's
        // contraction dimension bp toward TILE_K, cutting its K-padding waste
        // (K=64 pads 16×). The grid-parallel base reduces are ~bp-independent (their
        // compute and barrier counts scale with r, not bp), so when either fires we
        // widen bp for free; the single-CTA fallback keeps bp=64 (its shared cond[]
        // is sized 64). Override with FP_CUDA_BP.
        let bp = if use_coop || streamed {
            // K=1024 makes the X·U GEMM's contraction an exact TILE_K multiple —
            // zero K-padding — and block_reduce_coop is bp-independent.
            std::env::var("FP_CUDA_BP")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1024)
        } else {
            64
        }
        .clamp(1, r);

        const BR_THREADS: u32 = 256;
        let sms = self
            .ctx
            .attribute(sys::CUdevice_attribute_enum::CU_DEVICE_ATTRIBUTE_MULTIPROCESSOR_COUNT)?
            as u32;
        let br_occ = self
            .block_reduce_coop
            .occupancy_max_active_blocks_per_multiprocessor(BR_THREADS, 0, None)?
            .max(1);
        // Blocked-TRSM within-block reduce: recurse each block to narrow base blocks
        // + X·U GEMMs. The GEMM path composes regardless, so use it whenever a
        // grid-parallel base reduce is in play (coop or streamed).
        let use_trsm = use_coop || streamed;
        let base_bp: usize = std::env::var("FP_CUDA_BS_BASE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_BS_BASE)
            .clamp(1, DEFAULT_BS_BASE);

        // The block reduce is grid-barrier-bound (2 grid syncs/pivot) and a grid_sync's cost scales
        // with the CTA count, so under TRSM — where the reduces are narrow base blocks whose
        // per-pivot XOR needs little width — a small grid gives far cheaper barriers. Without TRSM
        // the reduce is the full block, whose XOR genuinely needs the whole grid. See
        // EXPERIMENTS.md.
        let br_cap = if use_trsm {
            DEFAULT_BR_CTAS
        } else {
            br_occ * sms
        };
        let br_ctas = std::env::var("FP_CUDA_BR_CTAS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(br_cap)
            .clamp(1, br_occ * sms);
        let mut br_barrier = stream.alloc_zeros::<u32>(1)?;
        // SAFETY: uninitialized is sound because `br_cond` writes each of the `bp` condition slots
        // before `br_xor` reads it.
        let br_cond = unsafe { stream.alloc::<u32>(bp) }?;

        let mut e = r;
        while e > 0 {
            let s = e - e.min(bp);

            // (1) reduce the block [s, e) to RREF among itself.
            if use_trsm {
                self.block_reduce_rec(
                    m,
                    perm,
                    &piv_dev,
                    pivot_cols,
                    s,
                    e,
                    base_bp,
                    use_coop,
                    streamed,
                    &mut br_barrier,
                    &br_cond,
                    br_ctas,
                )?;
            } else {
                self.block_reduce_elem(
                    m,
                    perm,
                    &piv_dev,
                    s,
                    e,
                    use_coop,
                    streamed,
                    &mut br_barrier,
                    &br_cond,
                    br_ctas,
                )?;
            }

            // (2) clear the block's pivot columns from all rows above [0, s).
            self.bs_clear_above(m, perm, &piv_dev, pivot_cols, s, e, 0, s)?;

            e = s;
        }
        stream.synchronize()?;
        Ok(())
    }

    /// Full device-resident F₂ row reduction to RREF over one persistent buffer:
    /// [`forward_reduce`](Self::forward_reduce) then
    /// [`back_substitute`](Self::back_substitute). Mutates `m` to the reduced row
    /// echelon form (addressed through the returned `perm`) and returns
    /// `(perm, rank, pivot_cols)`. Bit-for-bit equal to
    /// `fp::Matrix::row_reduce_blas3` / `row_reduce` (validated in the examples).
    pub fn row_reduce_dev(
        &self,
        m: &mut DeviceMatrix,
    ) -> anyhow::Result<(CudaSlice<u32>, usize, Vec<usize>)> {
        if std::env::var_os("FP_CUDA_RR_TIMING").is_some() {
            let t0 = std::time::Instant::now();
            let (perm, r, pivot_cols) = self.forward_reduce(m)?;
            let t1 = std::time::Instant::now();
            self.back_substitute(m, &perm, r, &pivot_cols)?;
            let t2 = std::time::Instant::now();
            eprintln!(
                "[rr_timing] forward={:.3}s back={:.3}s (r={r})",
                (t1 - t0).as_secs_f64(),
                (t2 - t1).as_secs_f64(),
            );
            return Ok((perm, r, pivot_cols));
        }
        let (perm, r, pivot_cols) = self.forward_reduce(m)?;
        self.back_substitute(m, &perm, r, &pivot_cols)?;
        Ok((perm, r, pivot_cols))
    }
}

/// Encode a 2D row-major TMA tensor map of UINT32 elements.
fn encode_tma(
    dev_ptr: sys::CUdeviceptr,
    gdim: [u64; 2],
    boxdim: [u32; 2],
    row_stride_bytes: u64,
    swizzle: sys::CUtensorMapSwizzle_enum,
) -> anyhow::Result<sys::CUtensorMap> {
    let gstride = [row_stride_bytes];
    let elemstride = [1u32, 1u32];
    let mut tmap = MaybeUninit::<sys::CUtensorMap>::uninit();
    // SAFETY: `gdim`, `gstride`, `boxdim` and `elemstride` are the rank-2 arrays the driver reads
    // (rank is passed as 2, and `gstride` is rank-1 by the API's contract of omitting the innermost
    // dimension). `dev_ptr` is a live device allocation owned by the caller. On success the driver
    // has fully written `tmap`, so the `assume_init` below is sound; on failure we return early.
    unsafe {
        sys::cuTensorMapEncodeTiled(
            tmap.as_mut_ptr(),
            sys::CUtensorMapDataType_enum::CU_TENSOR_MAP_DATA_TYPE_UINT32,
            2,
            dev_ptr as *mut c_void,
            gdim.as_ptr(),
            gstride.as_ptr(),
            boxdim.as_ptr(),
            elemstride.as_ptr(),
            sys::CUtensorMapInterleave_enum::CU_TENSOR_MAP_INTERLEAVE_NONE,
            swizzle,
            sys::CUtensorMapL2promotion_enum::CU_TENSOR_MAP_L2_PROMOTION_NONE,
            sys::CUtensorMapFloatOOBfill_enum::CU_TENSOR_MAP_FLOAT_OOB_FILL_NONE,
        )
        .result()?;
        Ok(tmap.assume_init())
    }
}

/// Gather A into plain row-major K-major tiles for TMA 128B swizzle.
///
/// Output: contiguous tiles, each TILE_M rows × KL u64s, so a row is 128 bytes — the swizzle width.
/// The TMA applies the 128B swizzle on load, so the host layout is the natural row-major sub-block:
/// tile row `row` holds K bits `kk*TILE_K .. +TILE_K` of global row `bi*TILE_M + row`, zero-padded
/// out of bounds.
///
/// Tiles are ordered: for K-chunk kk=0..k_chunks-1, then M-tile bi=0..m_tiles-1.
fn interleave_a(a: &[u64], m: usize, k: usize) -> Vec<u64> {
    let sa = k / 64;
    let k_chunks = k / TILE_K;
    let m_tiles = m / TILE_M;
    let tile_u64s = TILE_M * KL;
    let mut out = vec![0u64; k_chunks * m_tiles * tile_u64s];

    for kk in 0..k_chunks {
        for bi in 0..m_tiles {
            let base = (kk * m_tiles + bi) * tile_u64s;
            for row in 0..TILE_M {
                for kl in 0..KL {
                    let global_row = bi * TILE_M + row;
                    let global_kl = kk * KL + kl;
                    let val = if global_row < m && global_kl < sa {
                        a[global_row * sa + global_kl]
                    } else {
                        0
                    };
                    out[base + row * KL + kl] = val;
                }
            }
        }
    }
    out
}

/// Pre-transpose B into plain row-major K-major tiles for TMA 128B swizzle.
///
/// Each (k_chunk, column group) tile is NB = NG*64 rows (= the NG*64 output columns of the group) ×
/// KL u64s (= TILE_K K bits); the consumer feeds it to MSTRIPS m64n128 wgmmas that share it.
/// Operand row `lg*64 + jj` is output column `cg*NG*64 + lg*64 + jj`; element `[..][kl] bit` is bit
/// `jj` of `B[k_chunk*TILE_K + kl*64 + bit][cg*NG + lg]`.
///
/// Groups whose limb runs past `n_lim` are left zero-padded. Output is row-major; the TMA applies
/// the swizzle on load.
fn transpose_b(b: &[u64], k: usize, n_lim: usize) -> Vec<u64> {
    let k_chunks = k / TILE_K;
    let ng = NG as usize;
    let n_groups = n_lim.div_ceil(ng);
    let tile = ng * 64 * KL; // NB rows × KL u64
    let mut out = vec![0u64; k_chunks * n_groups * tile];
    let mut buf = [0u64; TILE_K];

    for kk in 0..k_chunks {
        for cg in 0..n_groups {
            let base = (kk * n_groups + cg) * tile;
            for lg in 0..ng {
                let limb = cg * ng + lg;
                if limb >= n_lim {
                    continue; // padded column group → leave zeros
                }
                for (i, slot) in buf.iter_mut().enumerate() {
                    let br = kk * TILE_K + i;
                    *slot = if br < k { b[br * n_lim + limb] } else { 0 };
                }
                for jj in 0..64usize {
                    let j = lg * 64 + jj; // operand row within the NB-col tile
                    for kl in 0..KL {
                        let mut val: u64 = 0;
                        for bit in 0..64usize {
                            val |= ((buf[kl * 64 + bit] >> jj) & 1) << bit;
                        }
                        out[base + j * KL + kl] = val;
                    }
                }
            }
        }
    }
    out
}

/// Copy a `rows × stride` limb array into a zero-filled `nr × ns` one, returning it unchanged when
/// the shapes already agree.
fn pad_2d(src: &[u64], rows: usize, stride: usize, nr: usize, ns: usize) -> Vec<u64> {
    if rows == nr && stride == ns {
        return src.to_vec();
    }
    let mut out = vec![0u64; nr * ns];
    for r in 0..rows {
        let n = stride.min(ns);
        out[r * ns..r * ns + n].copy_from_slice(&src[r * stride..r * stride + n]);
    }
    out
}

#[cfg(test)]
mod tests {
    use std::sync::OnceLock;

    use super::*;

    /// `true` iff a usable CUDA device is present, probed once for the whole test binary.
    ///
    /// `GpuContext::new` initializes the CUDA driver through cudarc, which *panics* (rather than
    /// returning `Err`) when no driver library is present — as on GPU-less CI. We silence the panic
    /// hook and catch the unwind so the probe reports "no GPU" instead of aborting the run. Every
    /// device-touching test gates on this and returns early when it is `false`, so the whole GPU
    /// test suite is disabled cleanly (rather than failing) wherever there is no device.
    fn gpu_available() -> bool {
        static AVAIL: OnceLock<bool> = OnceLock::new();
        *AVAIL.get_or_init(|| {
            let prev_hook = std::panic::take_hook();
            std::panic::set_hook(Box::new(|_| {}));
            let ok = std::panic::catch_unwind(|| GpuContext::new(0).is_ok()).unwrap_or(false);
            std::panic::set_hook(prev_hook);
            ok
        })
    }

    /// Regression for the per-thread stream cache being scoped to its context.
    ///
    /// A single thread that builds two `GpuContext`s must get two *distinct* streams — the cache
    /// was once keyed by thread alone, so the second context silently reused the first's stream
    /// (wrong context/device). The same context must still reuse its own cached stream. Uses device
    /// 0 twice: distinct instances, so distinct streams, without needing a second GPU.
    #[test]
    fn stream_is_scoped_per_context() {
        if !gpu_available() {
            return; // no usable GPU/driver in this environment — nothing to exercise
        }
        let a = GpuContext::new(0).expect("GPU is available");
        let b = GpuContext::new(0).expect("GPU is available");
        let (sa, sb) = (a.stream(), b.stream());
        assert!(
            !Arc::ptr_eq(&sa, &sb),
            "distinct GpuContexts must not share a per-thread stream"
        );
        assert!(
            Arc::ptr_eq(&sa, &a.stream()),
            "a context must reuse its own cached stream"
        );
        assert!(
            Arc::ptr_eq(&sb, &b.stream()),
            "a context must reuse its own cached stream"
        );
    }
}
