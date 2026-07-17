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
        Ok(Self {
            ctx,
            streams: Mutex::new(HashMap::new()),
            module,
            kernel,
            pack_a,
            pack_b,
            xor_into,
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
