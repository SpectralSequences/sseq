//! Host side of the Hopper `matmul_b1` kernel: compilation, marshalling and launch.

use std::{
    collections::HashMap,
    ffi::c_void,
    mem::MaybeUninit,
    sync::{Arc, Mutex, OnceLock},
    thread::ThreadId,
    time::Instant,
};

use anyhow::{anyhow, bail};
use cudarc::{
    driver::{
        CudaContext, CudaFunction, CudaModule, CudaStream, DevicePtr, DeviceRepr, LaunchConfig,
        PushKernelArg, sys,
    },
    nvrtc::{CompileError, CompileOptions, Ptx, compile_ptx_with_opts},
};

use super::params::{self, MSTRIPS, MW, NB, STAGES, THREADS_PER_WG, TK};
use crate::{matrix::Matrix, prime::TWO};

/// The CUDA C++ kernel, compiled to PTX at runtime by NVRTC — see [`compile_kernel`].
static KERNEL_SRC: &str = include_str!("matmul_b1.cu");

/// The virtual architecture the kernel is compiled for.
///
/// `90a` is architecture-*specific* and deliberately not forward-compatible: the kernel emits
/// `wgmma.*` and `cp.async.bulk.tensor.*`, which exist on Hopper and nowhere else.
const ARCH: &str = "compute_90a";

/// Compile the kernel to PTX with NVRTC, passing the knobs from [`params::defines`].
///
/// Needs `libnvrtc`, but no GPU.
pub fn compile_kernel() -> anyhow::Result<Ptx> {
    // SAFETY: `is_culib_present` only `dlopen`s the candidate library names and reports whether
    // one resolved; it dereferences nothing. cudarc panics on the first missing symbol, so the
    // probe has to come before any other nvrtc call.
    if !unsafe { cudarc::nvrtc::sys::is_culib_present() } {
        bail!(
            "libnvrtc was not found, so the CUDA kernel cannot be compiled. Install the CUDA \
             Toolkit (12.x+) and make sure its lib directory is on the loader path; in this repo, \
             `nix develop ./ext#gpu` does both."
        );
    }

    let opts = CompileOptions {
        arch: Some(ARCH),
        // Names the program, so NVRTC's diagnostics say matmul_b1.cu rather than "default_program".
        name: Some("matmul_b1.cu".to_string()),
        options: params::defines(),
        ..Default::default()
    };

    compile_ptx_with_opts(KERNEL_SRC, opts).map_err(|e| match &e {
        // `CompileError`'s `Display` is a `Debug` dump that buries the log in escapes.
        CompileError::CompileError { log, .. } => anyhow!(
            "NVRTC failed to compile matmul_b1.cu for {ARCH}:\n{}",
            log.to_string_lossy()
        ),
        _ => anyhow!("NVRTC failed to compile matmul_b1.cu: {e}"),
    })
}

/// The compiled kernel, compiled once per process and reused.
fn kernel_ptx() -> anyhow::Result<Ptx> {
    static PTX: OnceLock<Ptx> = OnceLock::new();
    if let Some(ptx) = PTX.get() {
        return Ok(ptx.clone());
    }
    let ptx = compile_kernel()?;
    Ok(PTX.get_or_init(|| ptx).clone())
}

const TILE_M: usize = MW * MSTRIPS; // output rows per CTA
const TILE_K: usize = TK;
const KL: usize = TILE_K / 64;
const THREADS: u32 = (2 * THREADS_PER_WG) as u32; // producer warpgroup + consumer warpgroup
const NG: u32 = (NB / 64) as u32; // output column-limbs per CTA

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

/// A CUDA device with the `matmul_b1` kernel loaded, ready to launch.
pub struct GpuContext {
    ctx: Arc<CudaContext>,
    /// Per-thread CUDA streams for *this* context, created lazily (see [`Self::stream`]). Owned by
    /// the context so a thread using several contexts (e.g. one per device) gets a distinct stream
    /// per context. The mutex is held only for the map lookup, never across a GPU submission, so it
    /// does not serialize device work.
    streams: Mutex<HashMap<ThreadId, Arc<CudaStream>>>,
    #[allow(dead_code)]
    module: Arc<CudaModule>,
    kernel: CudaFunction,
    transpose_kernel: CudaFunction,
}

impl GpuContext {
    /// Open device `device_id` and load the kernel onto it.
    pub fn new(device_id: usize) -> anyhow::Result<Self> {
        let ptx = kernel_ptx()?;
        let ctx = CudaContext::new(device_id)?;
        let module = ctx.load_module(ptx)?;
        let kernel = module.load_function("matmul_b1_kernel")?;
        let transpose_kernel = module.load_function("transpose_tile_b1_kernel")?;
        Ok(Self {
            ctx,
            streams: Mutex::new(HashMap::new()),
            module,
            kernel,
            transpose_kernel,
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

    /// A CUDA stream **private to the calling OS thread**.
    ///
    /// Created lazily on first use and reused thereafter, cached per thread in this context's
    /// `streams` map. Submitting through this instead of the context's single default stream
    /// lets calls from different threads run on distinct streams — overlapping transfers and
    /// kernels instead of serializing — while all sub-steps of one call share one stream, which
    /// keeps ordering correct within a thread. This is what lets `try_mul` run lock-free from many
    /// threads at once.
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
}

impl Matrix {
    /// Compute `self · rhs` on the GPU.
    ///
    /// Both operands must be over F₂. The operands are read in place: A is gathered into the
    /// kernel's tile layout straight from its limbs, and B is uploaded as it stands, to be
    /// rearranged K-major on the device by `transpose_tile_b1_kernel`.
    pub fn cuda_mul(&self, gpu: &GpuContext, rhs: &Self) -> anyhow::Result<Self> {
        Ok(matmul_b1(gpu, self, rhs, 1)?.0)
    }

    /// Like [`Self::cuda_mul`], but also returns the average **kernel-only** wall time in seconds.
    ///
    /// The time is averaged over `time_iters` back-to-back launches, and excludes host
    /// marshalling and the H2D/D2H copies.
    ///
    /// The kernel zeroes its SMEM accumulator and writes C with a bulk-tensor *store* (overwrite,
    /// not accumulate), so repeated launches against the same device buffers are idempotent and the
    /// returned matrix is the correct product.
    pub fn cuda_mul_timed(
        &self,
        gpu: &GpuContext,
        rhs: &Self,
        time_iters: usize,
    ) -> anyhow::Result<(Self, f64)> {
        matmul_b1(gpu, self, rhs, time_iters.max(1))
    }
}

/// Multiply `a · b` over F₂ on the GPU, launching the kernel `time_iters` times.
fn matmul_b1(
    gpu: &GpuContext,
    a: &Matrix,
    b: &Matrix,
    time_iters: usize,
) -> anyhow::Result<(Matrix, f64)> {
    assert_eq!(a.prime(), TWO);
    assert_eq!(b.prime(), TWO);
    assert_eq!(a.columns(), b.rows());

    let (m, k, n) = (a.rows(), a.columns(), b.columns());
    let mut c = Matrix::new(TWO, m, n);
    if m == 0 || k == 0 || n == 0 {
        return Ok((c, 0.0));
    }
    let n_lim = n.div_ceil(64);

    let k_padded = k.next_multiple_of(TILE_K);
    // Pad M to a whole number of M-tiles; the extra padded rows produce zeros
    // that the `take(m)` readback trims.
    let m_padded = m.next_multiple_of(TILE_M);
    let m_tiles = m_padded / TILE_M;
    let k_chunks = k_padded / TILE_K;
    // Each CTA computes a TILE_M×(NG*64) output block via MSTRIPS m64n128 wgmmas,
    // so B (and the C output) are grouped/padded to whole NG-limb column tiles.
    let n_groups = n_lim.div_ceil(NG as usize);
    let n_padded_lim = n_groups * NG as usize;

    let stream = gpu.stream();

    let a_interleaved = interleave_a(a, m_tiles, k_chunks);
    let a_dev = stream.clone_htod(&a_interleaved)?;

    // B goes up as its first `k` rows, row stride and all; the kernel reads rows past `k` as zeros,
    // so the K padding costs no host copy.
    let b_stride = b.stride();
    let b_dev = stream.clone_htod(&b.data()[..k * b_stride])?;
    let bt_dev = stream.alloc_zeros::<u64>(k_chunks * n_groups * (NG as usize * 64) * KL)?;
    {
        let cfg = LaunchConfig {
            grid_dim: ((KL * NG as usize) as u32, n_groups as u32, k_chunks as u32),
            block_dim: (64, 1, 1),
            shared_mem_bytes: 0,
        };
        let mut lb = stream.launch_builder(&gpu.transpose_kernel);
        let (n_lim_i, b_stride_i, k_i, n_groups_i) =
            (n_lim as i32, b_stride as i32, k as i32, n_groups as i32);
        lb.arg(&b_dev)
            .arg(&bt_dev)
            .arg(&n_lim_i)
            .arg(&b_stride_i)
            .arg(&k_i)
            .arg(&n_groups_i);
        // SAFETY: the six pushed arguments match `transpose_tile_b1_kernel`'s parameter list in
        // order and type; `b_dev` holds `k * b_stride` limbs and the kernel indexes it only where
        // `row < k` and `limb < n_lim <= b_stride`; `bt_dev` is exactly the tile count the grid
        // covers; both buffers outlive the launch, their guards being held until the final
        // synchronize.
        unsafe { lb.launch(cfg) }?;
    }

    let c_dev = stream.alloc_zeros::<u64>(m_padded * n_padded_lim)?;

    // Raw device addresses for the TMA descriptors. The returned guards keep the
    // reads ordered on the stream; hold them until after the launch.
    let (a_ptr, _ga) = a_dev.device_ptr(&stream);
    let (b_ptr, _gb) = bt_dev.device_ptr(&stream);
    let (c_ptr, _gc) = c_dev.device_ptr(&stream);

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
    let num_ctas = (occ * sms).max(1);
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

    let c_all = stream.clone_dtoh(&c_dev)?;
    let c_stride = c.stride();
    for (dst, src) in c
        .data_mut()
        .chunks_exact_mut(c_stride)
        .zip(c_all.chunks_exact(n_padded_lim))
        .take(m)
    {
        dst[..n_lim].copy_from_slice(&src[..n_lim]);
    }
    Ok((c, kernel_secs))
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
/// Output: contiguous tiles, each TILE_M rows × KL u64s, so a row is exactly the swizzle width.
/// The TMA applies the 128B swizzle on load, so the host layout is the natural row-major sub-block:
/// tile row `row` holds K bits `kk*TILE_K .. +TILE_K` of global row `bi*TILE_M + row`, zero-padded
/// out of bounds.
///
/// Tiles are ordered: for K-chunk kk=0..k_chunks-1, then M-tile bi=0..m_tiles-1.
fn interleave_a(a: &Matrix, m_tiles: usize, k_chunks: usize) -> Vec<u64> {
    let (m, k_lim, stride) = (a.rows(), a.columns().div_ceil(64), a.stride());
    let data = a.data();
    let tile_u64s = TILE_M * KL;
    let mut out = vec![0u64; k_chunks * m_tiles * tile_u64s];

    for kk in 0..k_chunks {
        let (kl_start, kl_end) = (kk * KL, ((kk + 1) * KL).min(k_lim));
        for bi in 0..m_tiles {
            let base = (kk * m_tiles + bi) * tile_u64s;
            for row in 0..TILE_M.min(m.saturating_sub(bi * TILE_M)) {
                let src = (bi * TILE_M + row) * stride;
                out[base + row * KL..][..kl_end - kl_start]
                    .copy_from_slice(&data[src + kl_start..src + kl_end]);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::sync::OnceLock;

    use proptest::prelude::*;

    use super::*;
    use crate::matrix::arbitrary::MatrixArbParams;

    /// A context on device 0 shared by the whole test binary, or `None` if there is no usable GPU.
    ///
    /// `GpuContext::new` initializes the CUDA driver through cudarc, which *panics* (rather than
    /// returning `Err`) when no driver library is present — as on GPU-less CI. We silence the panic
    /// hook and catch the unwind so the probe reports "no GPU" instead of aborting the run. Every
    /// device-touching test gates on this and returns early on `None`, so the whole GPU test suite
    /// is disabled cleanly (rather than failing) wherever there is no device.
    fn gpu() -> Option<&'static GpuContext> {
        static GPU: OnceLock<Option<GpuContext>> = OnceLock::new();
        GPU.get_or_init(|| {
            let prev_hook = std::panic::take_hook();
            std::panic::set_hook(Box::new(|_| {}));
            let ctx = std::panic::catch_unwind(|| GpuContext::new(0).ok()).unwrap_or(None);
            std::panic::set_hook(prev_hook);
            ctx
        })
        .as_ref()
    }

    /// The kernel compiles.
    #[test]
    fn kernel_compiles() {
        // SAFETY: see `compile_kernel`; the probe only tries to `dlopen` the library.
        if !unsafe { cudarc::nvrtc::sys::is_culib_present() } {
            return; // no CUDA Toolkit in this environment — nothing to compile with
        }
        let ptx = compile_kernel().expect("matmul_b1.cu must compile");
        let src = ptx.to_src();
        for kernel in ["matmul_b1_kernel", "transpose_tile_b1_kernel"] {
            assert!(
                src.contains(kernel),
                "{kernel} is missing from the compiled PTX"
            );
        }
    }

    /// `m` copied into a matrix whose row stride has room for `spare` more columns.
    fn with_spare_columns(m: &Matrix, spare: usize) -> Matrix {
        let (rows, columns) = (m.rows(), m.columns());
        let mut out = Matrix::new_with_capacity(TWO, rows, columns, rows, columns + spare);
        for i in 0..rows {
            out.row_mut(i).assign(m.row(i));
        }
        out
    }

    /// An arbitrary `rows × columns` matrix over F₂.
    fn arb_matrix(rows: usize, columns: usize) -> BoxedStrategy<Matrix> {
        Matrix::arbitrary_with(MatrixArbParams {
            p: Some(TWO),
            rows: Just(rows).boxed(),
            columns: Just(columns).boxed(),
        })
    }

    /// Multipliable operands, a number of spare columns to widen their row stride by, and a launch
    /// count.
    ///
    /// The dimensions straddle the kernel's M, N and K tiles, so every axis sees both whole tiles
    /// and ragged tails.
    fn arb_operands() -> impl Strategy<Value = (Matrix, Matrix, usize, usize)> {
        (1..=2 * TILE_M + 1, 1..=TILE_K + 65, 1..=2 * NB + 1).prop_flat_map(|(m, k, n)| {
            (arb_matrix(m, k), arb_matrix(k, n), 0..=128usize, 1..=3usize)
        })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        /// The GPU product is the product whatever the operands' row stride, and relaunching the
        /// kernel against the same buffers does not change it.
        #[test]
        fn cuda_mul_is_mul((a, b, spare, iters) in arb_operands()) {
            let Some(gpu) = gpu() else {
                return Ok(()); // no usable GPU/driver in this environment — nothing to exercise
            };
            // The tiled CPU kernel needs its operands padded to whole blocks, as `Mul` checks.
            let reference = if a.physical_rows().is_multiple_of(64)
                && b.physical_rows().is_multiple_of(64)
            {
                a.fast_mul_sequential(&b)
            } else {
                a.naive_mul(&b)
            };
            let (a, b) = (with_spare_columns(&a, spare), with_spare_columns(&b, spare));
            let (c, _) = a.cuda_mul_timed(gpu, &b, iters).expect("GPU matmul launch failed");
            prop_assert_eq!(c, reference);
        }
    }

    /// Regression for the per-thread stream cache being scoped to its context.
    ///
    /// A single thread that builds two `GpuContext`s must get two *distinct* streams — the cache
    /// was once keyed by thread alone, so the second context silently reused the first's stream
    /// (wrong context/device). The same context must still reuse its own cached stream. Uses device
    /// 0 twice: distinct instances, so distinct streams, without needing a second GPU.
    #[test]
    fn stream_is_scoped_per_context() {
        if gpu().is_none() {
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
