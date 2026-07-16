//! CUDA backend for `fp::blas` F_2 matrix multiplication on Hopper.
//!
//! Both operands are pre-arranged on the host as plain row-major K-major tiles
//! and loaded via TMA with 128B swizzle, which lands them in the SMEM layout the
//! swizzled wgmma matrix descriptors expect. The kernel register-blocks a
//! TILE_M×(NG*64) output tile per CTA out of MSTRIPS m64n128 wgmma.b1 strips
//! that share each loaded B tile (cuts operand-refill bandwidth, the bottleneck).

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

/// Opt-in per-phase GPU timing for the row reduction (`FP_CUDA_PROF=1`).
///
/// When enabled, [`timed_phase!`] brackets each kernel group with a stream
/// synchronize + host `Instant` and accumulates the elapsed time by label; the
/// accumulator is printed by [`GpuContext::row_reduce_dev`]. The extra syncs
/// serialize the pipeline, so the *absolute* numbers under `FP_CUDA_PROF` run
/// slower than production — only the **relative** per-phase split is meaningful.
/// When disabled the macro expands to the bare body: zero overhead, no extra
/// syncs, the exact pipelined hot path.
mod prof {
    use std::sync::{Mutex, OnceLock};

    static ENABLED: OnceLock<bool> = OnceLock::new();
    static ACC: OnceLock<Mutex<Vec<(String, u64)>>> = OnceLock::new();

    pub fn enabled() -> bool {
        *ENABLED.get_or_init(|| std::env::var("FP_CUDA_PROF").is_ok())
    }

    pub fn add(label: &str, nanos: u64) {
        let mut v = ACC.get_or_init(|| Mutex::new(Vec::new())).lock().unwrap();
        match v.iter_mut().find(|(l, _)| l == label) {
            Some(e) => e.1 += nanos,
            None => v.push((label.to_string(), nanos)),
        }
    }

    pub fn report(tag: &str) {
        if !enabled() {
            return;
        }
        let v = ACC.get_or_init(|| Mutex::new(Vec::new())).lock().unwrap();
        let total: u64 = v.iter().map(|(_, n)| *n).sum::<u64>().max(1);
        eprintln!("[fp-cuda PROF] {tag} — per-phase GPU time (ms), extra syncs on:");
        for (l, n) in v.iter() {
            eprintln!(
                "    {:<16} {:>10.3}  {:>5.1}%",
                l,
                *n as f64 / 1e6,
                100.0 * *n as f64 / total as f64
            );
        }
        eprintln!("    {:<16} {:>10.3}", "TOTAL", total as f64 / 1e6);
    }

    pub fn reset() {
        if let Some(m) = ACC.get() {
            m.lock().unwrap().clear();
        }
    }
}

/// Bracket a kernel group with a sync + timer when `FP_CUDA_PROF` is set,
/// accumulating GPU time under `$label`; otherwise expand to the bare body with
/// no added synchronization. The body may use `?` (it runs in the caller's
/// `Result` context).
macro_rules! timed_phase {
    ($stream:expr, $label:expr, $body:expr) => {{
        if $crate::prof::enabled() {
            $stream.synchronize()?;
            let __t = std::time::Instant::now();
            let __r = $body;
            $stream.synchronize()?;
            $crate::prof::add($label, __t.elapsed().as_nanos() as u64);
            __r
        } else {
            $body
        }
    }};
}

const TILE_M: usize = 192; // MW*MSTRIPS in the kernel; must match
const TILE_K: usize = 1024;
const KL: usize = TILE_K / 64; // 16
const THREADS: u32 = 256; // 2 warpgroups: producer (0..128) + consumer (128..256)
const NG: u32 = 2; // output column-limbs per CTA (NB/64 = 128/64); must match the kernel
const STAGES: usize = 4; // K-loop pipeline depth; must match the kernel
const CLUSTER: usize = 2; // CTAs per cluster along M (multicast B); must match the kernel

/// Lets us pass a `CUtensorMap` by value as a (grid-constant) kernel argument
/// through cudarc's typed launch builder. `repr(transparent)` so the pointer
/// cudarc pushes is the address of the 128-byte descriptor itself.
#[repr(transparent)]
struct TmaArg(sys::CUtensorMap);
unsafe impl DeviceRepr for TmaArg {}

/// A bit-packed F₂ matrix resident in device memory, in the natural row-major,
/// K-major limb layout `fp::Matrix` uses: `stride = cols.div_ceil(64)` u64 per
/// row, `rows * stride` u64 total, one bit per entry, bits past `cols` zero.
///
/// This is the persistent buffer the row-reduction port operates over: uploaded
/// once, mutated in place by device kernels, downloaded once (design §6). It is
/// `fp`-agnostic (raw limbs) so `fp-cuda` stays free of a dependency on `fp`.
pub struct DeviceMatrix {
    pub buf: CudaSlice<u64>,
    pub rows: usize,
    pub cols: usize,
    pub stride: usize,
}

impl DeviceMatrix {
    pub fn stride(&self) -> usize {
        self.stride
    }
}

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
    promote_pivots: CudaFunction,
    zero_pivot_l: CudaFunction,
    gather_rows: CudaFunction,
    block_reduce_rref: CudaFunction,
    gather_cols: CudaFunction,
    xor_into_perm: CudaFunction,
}

impl GpuContext {
    pub fn new(device_id: usize) -> Result<Self, Box<dyn std::error::Error>> {
        let ctx = CudaContext::new(device_id)?;
        let ptx = Ptx::from_src(String::from_utf8(PTX_IMAGE.to_vec())?);
        let module = ctx.load_module(ptx)?;
        let kernel = module.load_function("matmul_b1_kernel")?;
        let pack_a = module.load_function("pack_a")?;
        let pack_b = module.load_function("pack_b")?;
        let xor_into = module.load_function("xor_into")?;
        let panel_factor = module.load_function("panel_factor")?;
        let panel_factor_coop = module.load_function("panel_factor_coop")?;
        let promote_pivots = module.load_function("promote_pivots")?;
        let zero_pivot_l = module.load_function("zero_pivot_l")?;
        let gather_rows = module.load_function("gather_rows")?;
        let block_reduce_rref = module.load_function("block_reduce_rref")?;
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
            promote_pivots,
            zero_pivot_l,
            gather_rows,
            block_reduce_rref,
            gather_cols,
            xor_into_perm,
        })
    }

    pub fn compute_capability(&self) -> Result<(i32, i32), Box<dyn std::error::Error>> {
        let major = self.ctx.attribute(
            sys::CUdevice_attribute_enum::CU_DEVICE_ATTRIBUTE_COMPUTE_CAPABILITY_MAJOR,
        )?;
        let minor = self.ctx.attribute(
            sys::CUdevice_attribute_enum::CU_DEVICE_ATTRIBUTE_COMPUTE_CAPABILITY_MINOR,
        )?;
        Ok((major, minor))
    }

    pub fn default_stream(&self) -> Arc<CudaStream> {
        self.ctx.default_stream()
    }

    /// A CUDA stream **private to the calling OS thread**, created lazily on first use and reused
    /// thereafter (cached per thread in this context's `streams` map). Submitting through this instead of the context's
    /// single `default_stream()` lets calls from different threads run on distinct streams —
    /// overlapping transfers and kernels instead of serializing — while all sub-steps of one call
    /// share one stream (correct ordering within a thread). This is what lets `try_mul` (and the
    /// row-reduce) run lock-free from many threads at once.
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

    pub fn kernel(&self) -> &CudaFunction {
        &self.kernel
    }
}

/// Multiply two bit-packed F₂ matrices on the GPU.
///
/// Operands are plain **row-major, K-major** limb arrays — the exact layout
/// `fp::Matrix::to_bytes` produces (little-endian `u64` limbs, one bit per
/// entry, `columns.div_ceil(64)` limbs per row, no inter-row padding):
///
/// - `a`: the `m`×`k` left operand, `m * k.div_ceil(64)` limbs.
/// - `b`: the `k`×`n` right operand, `k * n.div_ceil(64)` limbs.
///
/// Returns C = A·B as `m * n.div_ceil(64)` limbs in the same layout, ready to
/// hand to `fp::Matrix::from_data`. This keeps `fp-cuda` free of any dependency
/// on `fp` (the higher-level crate depends on this one, not the reverse).
pub fn matmul_b1_raw(
    gpu: &GpuContext,
    a: &[u64],
    m: usize,
    k: usize,
    b: &[u64],
    n: usize,
) -> Result<Vec<u64>, Box<dyn std::error::Error>> {
    Ok(matmul_b1_inner(gpu, a, m, k, b, n, 1)?.0)
}

/// Like [`matmul_b1_raw`], but also returns the average **kernel-only** wall
/// time (seconds) over `time_iters` back-to-back launches, excluding host
/// (de)serialization, the TMA-layout pre-arrangement, and the H2D/D2H copies.
///
/// The kernel zeroes its SMEM accumulator and writes C with a bulk-tensor
/// *store* (overwrite, not accumulate), so repeated launches against the same
/// device buffers are idempotent and the returned limbs are the correct
/// product. Use this to compare against the ~100-binary-TOPS pre-swizzle
/// kernel baseline; the end-to-end `cargo bench` figures are dominated by host
/// serialization and understate kernel throughput.
pub fn matmul_b1_raw_timed(
    gpu: &GpuContext,
    a: &[u64],
    m: usize,
    k: usize,
    b: &[u64],
    n: usize,
    time_iters: usize,
) -> Result<(Vec<u64>, f64), Box<dyn std::error::Error>> {
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
) -> Result<(Vec<u64>, f64), Box<dyn std::error::Error>> {
    let n_lim = n.div_ceil(64);
    assert_eq!(a.len(), m * k.div_ceil(64), "A limb count mismatch");
    assert_eq!(b.len(), k * n_lim, "B limb count mismatch");

    let k_padded = k.next_multiple_of(TILE_K);
    // Pad M to a whole number of clusters (CLUSTER M-tiles) so every cluster
    // rank has a valid M-tile; the extra padded rows produce zeros that the
    // `take(m)` readback trims.
    let m_padded = m.next_multiple_of(TILE_M * CLUSTER);
    // Each CTA computes a TILE_M×(NG*64) output block via MSTRIPS m64n128 wgmmas,
    // so B (and the C output) are grouped/padded to whole NG-limb column tiles.
    let n_groups = n_lim.div_ceil(NG as usize);
    let n_padded_lim = n_groups * NG as usize;

    let stream = gpu.stream();

    let a_padded = pad_2d(a, m, k.div_ceil(64), m_padded, k_padded / 64);
    let b_padded = pad_2d(b, k, n_lim, k_padded, n_lim);

    // Gather A into row-major K-major tiles; the TMA applies the 128B swizzle.
    let a_interleaved = interleave_a(&a_padded, m_padded, k_padded);
    // Pre-transpose B into row-major K-major tiles (swizzled by the TMA).
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
) -> Result<f64, Box<dyn std::error::Error>> {
    let m_tiles = m_padded / TILE_M;
    let k_chunks = k_padded / TILE_K;

    // Raw device addresses for the TMA descriptors. The returned guards keep the
    // reads ordered on the stream; hold them until after the launch.
    let (a_ptr, _ga) = a_dev.device_ptr(stream);
    let (b_ptr, _gb) = bt_dev.device_ptr(stream);
    let (c_ptr, _gc) = c_dev.device_ptr(stream);

    // TMA tensor maps. A: TILE_M-row block per (k_chunk, M-block), split into
    // MSTRIPS m64 strips by the consumer. B: (NG*64)-column tile per (k_chunk,
    // column group), reused across the strips. Both have a 128-byte inner dim
    // (= the 128B swizzle width). C: TILE_M-row × NG-limb output blocks, no
    // swizzle, for the bulk store.
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

    // Persistent grid: co-resident CTAs = (occupancy per SM) × SM count, so the
    // grid exactly fills the machine and the kernel's persistent loop sweeps all
    // output tiles in grouped-rasterized order. Rounded to a whole number of
    // clusters (`__cluster_dims__` requires gridDim.x % CLUSTER == 0); surplus
    // clusters run an empty loop.
    //
    // This is 1 CTA/SM at the register-blocked config, and that is optimal:
    // 2 CTAs/SM was measured (2026-07-07 H200) and loses badly. Two CTAs need the
    // compiled register count ≤128/thread (2·256·128 = the 64K reg file), but the
    // resident accumulator that gives the kernel its arithmetic intensity is
    // 192 regs/thread at MSTRIPS=3. Shrinking it to fit two CTAs (MSTRIPS=1,
    // 64-reg acc → occ=2) collapses AI and drops 16384 from ~8,600 to ~5,500
    // TOPS. High AI (big accumulator) and high occupancy compete for the same
    // register file and AI wins, so we run 1 CTA/SM with the largest accumulator
    // that fits under the 255-reg cap.
    let sms = gpu
        .ctx
        .attribute(sys::CUdevice_attribute_enum::CU_DEVICE_ATTRIBUTE_MULTIPROCESSOR_COUNT)?
        as u32;
    let occ = gpu
        .kernel
        .occupancy_max_active_blocks_per_multiprocessor(THREADS, smem_bytes as usize, None)?
        .max(1);
    let num_ctas = (occ * sms / CLUSTER as u32).max(1) * CLUSTER as u32;
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
        unsafe { lb.launch(cfg) }?;
        Ok(())
    };

    // Warm up once (untimed) when measuring, so the timed loop excludes any
    // first-launch JIT/allocation costs.
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
    /// Device-resident F₂ GEMM. Both operands live on device in the **natural**
    /// row-major, K-major limb layout that `fp::Matrix` stores (`a_dev`: `m ×
    /// k.div_ceil(64)` u64; `b_dev`: `k × n.div_ceil(64)` u64). The pack into the
    /// wgmma tile layout (interleave A, bit-transpose B) runs on device, so there
    /// is no host round-trip — this is the persistent-buffer primitive the
    /// row-reduction port needs (design §6).
    ///
    /// Returns `C = A·B` as a **padded** device buffer of `m_padded ×
    /// n_padded_lim` u64 (valid data in the first `m` rows and first
    /// `n.div_ceil(64)` limbs of each row), plus its row stride `n_padded_lim`.
    /// The padded stride is what [`GpuContext::xor_into_region`] consumes; a
    /// standalone caller trims it on readback.
    pub fn matmul_b1_dev(
        &self,
        a_dev: &CudaSlice<u64>,
        m: usize,
        k: usize,
        b_dev: &CudaSlice<u64>,
        n: usize,
    ) -> Result<(CudaSlice<u64>, usize), Box<dyn std::error::Error>> {
        let sa = k.div_ceil(64);
        let n_lim = n.div_ceil(64);
        assert_eq!(a_dev.len(), m * sa, "A limb count mismatch");
        assert_eq!(b_dev.len(), k * n_lim, "B limb count mismatch");

        let k_padded = k.next_multiple_of(TILE_K);
        let m_padded = m.next_multiple_of(TILE_M * CLUSTER);
        let m_tiles = m_padded / TILE_M;
        let k_chunks = k_padded / TILE_K;
        let n_groups = n_lim.div_ceil(NG as usize);
        let n_padded_lim = n_groups * NG as usize;

        let stream = self.ctx.default_stream();

        // Pack A → interleaved row-major K-major tiles (m_padded × k_padded/64).
        let a_int_len = m_padded * (k_padded / 64);
        let a_int = stream.alloc_zeros::<u64>(a_int_len)?;
        {
            let (m_orig, sa_orig, mt, total) =
                (m as u32, sa as u32, m_tiles as u32, a_int_len as u32);
            let mut lb = stream.launch_builder(&self.pack_a);
            lb.arg(&a_int)
                .arg(a_dev)
                .arg(&m_orig)
                .arg(&sa_orig)
                .arg(&mt)
                .arg(&total);
            unsafe { lb.launch(cfg_1d(a_int_len)) }?;
        }

        // Pack B → bit-transposed K-major tiles.
        let bt_len = k_chunks * n_groups * (NG as usize * 64 * KL);
        let bt = stream.alloc_zeros::<u64>(bt_len)?;
        {
            let (k_orig, nl, ng, total) = (k as u32, n_lim as u32, n_groups as u32, bt_len as u32);
            let mut lb = stream.launch_builder(&self.pack_b);
            lb.arg(&bt)
                .arg(b_dev)
                .arg(&k_orig)
                .arg(&nl)
                .arg(&ng)
                .arg(&total);
            unsafe { lb.launch(cfg_1d(bt_len)) }?;
        }

        let c_dev = stream.alloc_zeros::<u64>(m_padded * n_padded_lim)?;
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
    /// width`. This is the fused trailing-update / back-sub epilogue (design
    /// §4.4/§4.6): the `M[region] ^= L·U` update with no fresh C alloc + D2H.
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
    ) -> Result<(), Box<dyn std::error::Error>> {
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
    ) -> Result<Vec<u64>, Box<dyn std::error::Error>> {
        let n_lim = n.div_ceil(64);
        let stream = self.ctx.default_stream();
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
    pub fn upload(
        &self,
        data: &[u64],
        rows: usize,
        cols: usize,
    ) -> Result<DeviceMatrix, Box<dyn std::error::Error>> {
        let stride = cols.div_ceil(64);
        assert_eq!(data.len(), rows * stride, "limb count mismatch");
        let buf = self.ctx.default_stream().clone_htod(data)?;
        Ok(DeviceMatrix {
            buf,
            rows,
            cols,
            stride,
        })
    }

    /// Download a [`DeviceMatrix`] back to host limbs (natural layout). One D2H.
    pub fn download(&self, dm: &DeviceMatrix) -> Result<Vec<u64>, Box<dyn std::error::Error>> {
        Ok(self.ctx.default_stream().clone_dtoh(&dm.buf)?)
    }

    /// Download a device `u32` buffer (e.g. a `perm` vector) to host.
    pub fn download_u32(&self, s: &CudaSlice<u32>) -> Result<Vec<u32>, Box<dyn std::error::Error>> {
        Ok(self.ctx.default_stream().clone_dtoh(s)?)
    }

    /// The fused trailing-update / back-substitution epilogue over persistent
    /// device buffers (design §4.4/§4.6): `dst[:, col_off:] ^= L · U`, in place,
    /// where `L` is `dst.rows × k` and `U` is `k × t` (`k = l.cols = u.rows`,
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
    ) -> Result<(), Box<dyn std::error::Error>> {
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
        let stream = self.ctx.default_stream();
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

    /// Allocate the identity virtual-row permutation `perm = [0, 1, …, m-1]` on
    /// device (design §4.3). Kernels dereference rows as `M[perm[i]]`; row swaps
    /// are `perm` swaps, so the matrix bytes never move.
    pub fn identity_perm(&self, m: usize) -> Result<CudaSlice<u32>, Box<dyn std::error::Error>> {
        let host: Vec<u32> = (0..m as u32).collect();
        Ok(self.ctx.default_stream().clone_htod(&host)?)
    }

    /// Factor one 64-bit column panel (limb `plimb`) in place over the
    /// persistent buffer, forward-only, starting from pivot row `r` (design §5,
    /// the b=64 base kernel). Rows are addressed through `perm`; a pivot is
    /// promoted by swapping its `perm` entry to position `r + pivot_index`. The
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
    ) -> Result<(usize, Vec<u32>), Box<dyn std::error::Error>> {
        assert_eq!(perm.len(), m.rows, "perm length must equal rows");
        assert_eq!(l.rows, m.rows, "L rows must equal M rows");
        let stream = self.ctx.default_stream();

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
        plimb: usize,
        r: usize,
    ) -> Result<(usize, Vec<u32>), Box<dyn std::error::Error>> {
        assert_eq!(perm.len(), m.rows, "perm length must equal rows");
        assert_eq!(l.rows, m.rows, "L rows must equal M rows");
        let stream = self.ctx.default_stream();

        const THREADS: u32 = 256;
        let smem = THREADS * std::mem::size_of::<i32>() as u32;
        let pivcols = stream.alloc_zeros::<u32>(64)?;
        let pr_out = stream.alloc_zeros::<u32>(1)?;
        // scratch = [barrier(0), g_min, g_pr]; alloc_zeros gives barrier == 0.
        let scratch = stream.alloc_zeros::<u32>(3)?;
        let g_pivword = stream.alloc_zeros::<u64>(1)?;

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
        let rows_worth = (m.rows as u32).div_ceil(THREADS).max(1);
        let num_ctas = (occ * sms).min(rows_worth).max(1);

        let (plimb_u, r_u, n_u, m_u, stride_u, l_stride_u, tc) = (
            plimb as u32,
            r as u32,
            m.cols as u32,
            m.rows as u32,
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
            .arg(&plimb_u)
            .arg(&r_u)
            .arg(&n_u)
            .arg(&m_u)
            .arg(&stride_u)
            .arg(&l_stride_u)
            .arg(&tc);
        unsafe { lb.launch_cooperative(cfg) }?;

        let pr = stream.clone_dtoh(&pr_out)?[0] as usize;
        let cols = stream.clone_dtoh(&pivcols)?;
        Ok((pr, cols[..pr].to_vec()))
    }

    /// Forward pass of the blocked row reduction over the persistent device
    /// buffer (design §4): sweep 64-bit panels left to right, and for each —
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
    ) -> Result<(CudaSlice<u32>, usize, Vec<usize>), Box<dyn std::error::Error>> {
        let stream = self.ctx.default_stream();
        let (rows, stride, n) = (m.rows, m.stride, m.cols);
        let mut perm = self.identity_perm(rows)?;
        let mut r = 0usize;
        let mut pivot_cols = Vec::new();
        // The cooperative multi-CTA panel factor is the default; set
        // FP_CUDA_NO_COOP=1 to fall back to the single-CTA kernel (A/B testing).
        let use_coop = std::env::var("FP_CUDA_NO_COOP").is_err();

        for plimb in 0..stride {
            // Fresh multiplier matrix L (m × 64, one limb/row) per panel.
            let mut l = DeviceMatrix {
                buf: stream.alloc_zeros::<u64>(rows)?,
                rows,
                cols: 64,
                stride: 1,
            };
            let (pr, pivcols) = timed_phase!(
                stream,
                "panel_factor",
                if use_coop {
                    self.panel_factor_coop(m, &mut perm, &mut l, plimb, r)
                } else {
                    self.panel_factor(m, &mut perm, &mut l, plimb, r)
                }
            )?;
            if pr == 0 {
                continue;
            }
            let first_limb = plimb + 1;
            let trailing_limbs = stride - first_limb;
            if trailing_limbs > 0 {
                let t = n - first_limb * 64;
                // (1) promote pivot rows' trailing (triangular, one CTA), then
                // (2) zero pivot rows' L so the GEMM leaves them untouched.
                timed_phase!(stream, "promote", {
                    let (r_u, pr_u, fl, tl, st, ls) = (
                        r as u32,
                        pr as u32,
                        first_limb as u32,
                        trailing_limbs as u32,
                        stride as u32,
                        l.stride as u32,
                    );
                    // Grid-strided over trailing limbs (columns are independent).
                    let mut lb = stream.launch_builder(&self.promote_pivots);
                    lb.arg(&mut m.buf)
                        .arg(&perm)
                        .arg(&l.buf)
                        .arg(&r_u)
                        .arg(&pr_u)
                        .arg(&fl)
                        .arg(&tl)
                        .arg(&st)
                        .arg(&ls);
                    unsafe { lb.launch(cfg_1d(trailing_limbs)) }?;
                    let (r_u, pr_u, ls) = (r as u32, pr as u32, l.stride as u32);
                    let mut lb = stream.launch_builder(&self.zero_pivot_l);
                    lb.arg(&perm).arg(&mut l.buf).arg(&r_u).arg(&pr_u).arg(&ls);
                    unsafe { lb.launch(cfg_1d(pr)) }?;
                });
                // (3) gather U = pivot rows' trailing (pr × trailing_limbs).
                let u_buf = stream.alloc_zeros::<u64>(pr * trailing_limbs)?;
                timed_phase!(stream, "gather_u", {
                    let (r_u, fl, pr_u, nc, st) = (
                        r as u32,
                        first_limb as u32,
                        pr as u32,
                        trailing_limbs as u32,
                        stride as u32,
                    );
                    let mut lb = stream.launch_builder(&self.gather_rows);
                    lb.arg(&u_buf)
                        .arg(&m.buf)
                        .arg(&perm)
                        .arg(&r_u)
                        .arg(&fl)
                        .arg(&pr_u)
                        .arg(&nc)
                        .arg(&st);
                    unsafe { lb.launch(cfg_1d(pr * trailing_limbs)) }?;
                });
                // (4) trailing GEMM: C = L(m×pr)·U(pr×t); M[:, first_limb:] ^= C.
                let (c_dev, n_padded_lim) = timed_phase!(
                    stream,
                    "gemm",
                    self.matmul_b1_dev(&l.buf, rows, pr, &u_buf, t)
                )?;
                timed_phase!(
                    stream,
                    "xor",
                    self.xor_into_region(
                        &stream,
                        &mut m.buf,
                        &c_dev,
                        rows,
                        trailing_limbs,
                        stride,
                        first_limb,
                        n_padded_lim,
                    )
                )?;
            }
            for &q in &pivcols {
                pivot_cols.push(q as usize);
            }
            r += pr;
        }
        stream.synchronize()?;
        Ok((perm, r, pivot_cols))
    }

    /// Back-substitution: turn the row-echelon form left by
    /// [`forward_reduce`](Self::forward_reduce) into full RREF (design §4.6),
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
    ) -> Result<(), Box<dyn std::error::Error>> {
        if r == 0 {
            return Ok(());
        }
        let stream = self.ctx.default_stream();
        let (stride, n) = (m.stride, m.cols);
        let piv_dev =
            stream.clone_htod(&pivot_cols.iter().map(|&q| q as u32).collect::<Vec<_>>())?;

        let one_cta = LaunchConfig {
            grid_dim: (1, 1, 1),
            block_dim: (256, 1, 1),
            shared_mem_bytes: 0,
        };
        const BP: usize = 64; // pivots per block (bl * 64 with bl = 1)
        let mut e = r;
        while e > 0 {
            let s = e - e.min(BP);
            let bp_eff = e - s;

            // (1) reduce the block [s, e) to RREF among itself.
            timed_phase!(stream, "bs.block_reduce", {
                let (s_u, e_u, st) = (s as u32, e as u32, stride as u32);
                let mut lb = stream.launch_builder(&self.block_reduce_rref);
                lb.arg(&mut m.buf)
                    .arg(perm)
                    .arg(&piv_dev)
                    .arg(&s_u)
                    .arg(&e_u)
                    .arg(&st);
                unsafe { lb.launch(one_cta) }?;
            });

            // (2) clear the block's pivot columns from all rows above [0, s).
            if s > 0 {
                let start_limb = pivot_cols[s] / 64;
                let trailing_limbs = stride - start_limb;
                let width_cols = n - start_limb * 64;
                let x_stride = bp_eff.div_ceil(64);

                // X = above rows gathered at the block's pivot columns (s × bp_eff).
                let x_buf = stream.alloc_zeros::<u64>(s * x_stride)?;
                timed_phase!(stream, "bs.gather_x", {
                    let (cs, s_u, cnt, st, xs) = (
                        s as u32,
                        s as u32,
                        bp_eff as u32,
                        stride as u32,
                        x_stride as u32,
                    );
                    let mut lb = stream.launch_builder(&self.gather_cols);
                    lb.arg(&x_buf)
                        .arg(&m.buf)
                        .arg(perm)
                        .arg(&piv_dev)
                        .arg(&cs)
                        .arg(&s_u)
                        .arg(&cnt)
                        .arg(&st)
                        .arg(&xs);
                    unsafe { lb.launch(cfg_1d(s * x_stride)) }?;
                });

                // U = the (now RREF) block rows, limbs [start_limb, stride).
                let u_buf = stream.alloc_zeros::<u64>(bp_eff * trailing_limbs)?;
                timed_phase!(stream, "bs.gather_u", {
                    let (s_u, fl, pr_u, nc, st) = (
                        s as u32,
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
                    unsafe { lb.launch(cfg_1d(bp_eff * trailing_limbs)) }?;
                });

                // G = X · U (s × width_cols); scatter-XOR into rows above via perm.
                let (c_dev, n_padded_lim) = timed_phase!(
                    stream,
                    "bs.gemm",
                    self.matmul_b1_dev(&x_buf, s, bp_eff, &u_buf, width_cols)
                )?;
                timed_phase!(stream, "bs.xor", {
                    let (s_u, w, st, fl, cs) = (
                        s as u32,
                        trailing_limbs as u32,
                        stride as u32,
                        start_limb as u32,
                        n_padded_lim as u32,
                    );
                    let mut lb = stream.launch_builder(&self.xor_into_perm);
                    lb.arg(&mut m.buf)
                        .arg(&c_dev)
                        .arg(perm)
                        .arg(&s_u)
                        .arg(&w)
                        .arg(&st)
                        .arg(&fl)
                        .arg(&cs);
                    unsafe { lb.launch(cfg_1d(s * trailing_limbs)) }?;
                });
            }
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
    ) -> Result<(CudaSlice<u32>, usize, Vec<usize>), Box<dyn std::error::Error>> {
        prof::reset();
        let (perm, r, pivot_cols) = self.forward_reduce(m)?;
        self.back_substitute(m, &perm, r, &pivot_cols)?;
        prof::report("row_reduce_dev");
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
) -> Result<sys::CUtensorMap, Box<dyn std::error::Error>> {
    let gstride = [row_stride_bytes];
    let elemstride = [1u32, 1u32];
    let mut tmap = MaybeUninit::<sys::CUtensorMap>::uninit();
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
/// Output: contiguous tiles, each TILE_M rows × KL u64s (64 × 128 bytes). The
/// TMA applies the 128B swizzle on load, so the host layout is the natural
/// row-major sub-block: tile row `row` holds K bits `kk*TILE_K .. +TILE_K` of
/// global row `bi*TILE_M + row`, zero-padded out of bounds.
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
/// Each (k_chunk, column group) tile is NB = NG*64 rows (= the NG*64 output
/// columns of the group) × KL u64s (= TILE_K K bits); the consumer feeds it to
/// MSTRIPS m64n128 wgmmas that share it. Operand row `lg*64 + jj` is output
/// column `cg*NG*64 + lg*64 + jj`; element `[..][kl] bit` is bit `jj` of
/// `B[k_chunk*TILE_K + kl*64 + bit][cg*NG + lg]`.
/// Groups whose limb runs past `n_lim` are left zero-padded. Output is
/// row-major; the TMA applies the swizzle on load.
fn transpose_b(b: &[u64], k: usize, n_lim: usize) -> Vec<u64> {
    let k_chunks = k / TILE_K;
    let ng = NG as usize;
    let n_groups = n_lim.div_ceil(ng);
    let tile = ng * 64 * KL; // 256 rows × KL u64
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
                    let j = lg * 64 + jj; // operand row within the 256-col tile
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

    /// Regression for the per-thread stream cache being scoped to its context. A single thread that
    /// builds two `GpuContext`s must get two *distinct* streams — the cache was once keyed by thread
    /// alone, so the second context silently reused the first's stream (wrong context/device). The
    /// same context must still reuse its own cached stream. (Uses device 0 twice: distinct instances,
    /// so distinct streams, without needing a second GPU.)
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
