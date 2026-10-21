//! Host side of the Hopper `matmul_b1` kernel: compilation, marshalling and launch.

use std::{
    collections::HashMap,
    ffi::c_void,
    mem::MaybeUninit,
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    thread::ThreadId,
    time::Instant,
};

use anyhow::{anyhow, bail};
use cudarc::{
    driver::{
        CudaContext, CudaFunction, CudaModule, CudaSlice, CudaStream, CudaView, DevicePtr,
        DevicePtrMut, DeviceRepr, LaunchConfig, PushKernelArg, sys,
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
    /// Held so the loaded module outlives the [`CudaFunction`]s taken from it; never read.
    #[allow(dead_code)]
    module: Arc<CudaModule>,
    kernel: CudaFunction,
    /// The GEMM with the reduce-store epilogue, XORing its product into a matrix (`EPI_TMA_XOR`).
    kernel_xor: CudaFunction,
    transpose_kernel: CudaFunction,
    pack_a: CudaFunction,
    pack_b: CudaFunction,
    xor_into: CudaFunction,
    restride_rows: CudaFunction,
}

impl GpuContext {
    /// Open device `device_id` and load the kernel onto it.
    pub fn new(device_id: usize) -> anyhow::Result<Self> {
        let ptx = kernel_ptx()?;
        let ctx = CudaContext::new(device_id)?;
        let module = ctx.load_module(ptx)?;
        let kernel = module.load_function("matmul_b1_kernel")?;
        let kernel_xor = module.load_function("matmul_b1_kernel_xor")?;
        let transpose_kernel = module.load_function("transpose_tile_b1_kernel")?;
        let pack_a = module.load_function("pack_a")?;
        let pack_b = module.load_function("pack_b")?;
        let xor_into = module.load_function("xor_into")?;
        let restride_rows = module.load_function("restride_rows")?;
        Ok(Self {
            ctx,
            streams: Mutex::new(HashMap::new()),
            module,
            kernel,
            kernel_xor,
            transpose_kernel,
            pack_a,
            pack_b,
            xor_into,
            restride_rows,
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
    let kernel_secs = {
        let (c_ptr, _gc) = c_dev.device_ptr(&stream);
        run_gemm_kernel(
            gpu,
            &stream,
            &a_dev,
            &bt_dev,
            GemmTarget::Store {
                c: c_ptr,
                n_padded_lim,
            },
            m_padded,
            k_padded,
            n_groups,
            time_iters,
        )?
    };

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

/// Where a GEMM's product goes ([`run_gemm_kernel`]).
///
/// Raw device addresses: the caller holds the buffers' stream guards across the launch.
#[derive(Clone, Copy, Debug)]
enum GemmTarget {
    /// Store the padded product `C` (`m_padded × n_padded_lim` limbs).
    Store {
        c: sys::CUdeviceptr,
        n_padded_lim: usize,
    },
    /// XOR the product into the matrix `dst` (`dst_rows × dst_stride` limbs), product limb 0
    /// landing on limb `x_limb` of each row (`EPI_TMA_XOR`). Rows past `dst_rows` are dropped.
    TmaXor {
        dst: sys::CUdeviceptr,
        dst_rows: usize,
        dst_stride: usize,
        x_limb: usize,
    },
}

/// Whether the reduce-store epilogue can address a matrix of row stride `stride` limbs.
///
/// It needs 16-byte rows, i.e. an even limb count, and a width in u32s that fits the kernel's
/// signed 32-bit coordinates.
pub fn tma_xor_supported(stride: usize) -> bool {
    stride > 0 && stride.is_multiple_of(2) && stride * 2 < i32::MAX as usize
}

/// Launch the GEMM over operands already packed into its tiles and resident on the device.
///
/// `target` says where the product goes. Returns the average kernel-only wall time over
/// `time_iters` launches.
#[allow(clippy::too_many_arguments)]
fn run_gemm_kernel(
    gpu: &GpuContext,
    stream: &Arc<CudaStream>,
    a_dev: &CudaSlice<u64>,
    bt_dev: &CudaSlice<u64>,
    target: GemmTarget,
    m_padded: usize,
    k_padded: usize,
    n_groups: usize,
    time_iters: usize,
) -> anyhow::Result<f64> {
    let m_tiles = m_padded / TILE_M;
    let k_chunks = k_padded / TILE_K;

    // Raw device addresses for the TMA descriptors. The returned guards keep the
    // reads ordered on the stream; hold them until after the launch.
    let (a_ptr, _ga) = a_dev.device_ptr(stream);
    let (b_ptr, _gb) = bt_dev.device_ptr(stream);

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
    // The third map is the epilogue's: C for a store, the destination matrix itself for the reduce.
    // The latter spans the matrix's whole row stride, so that box starts and the tensor width are
    // the even limb counts `EPI_TMA_XOR` needs.
    let (tma_c, x_off, kernel) = match target {
        GemmTarget::Store { c, n_padded_lim } => {
            debug_assert_eq!(n_padded_lim, n_groups * NG as usize);
            let map = encode_tma(
                c,
                [(n_padded_lim * 2) as u64, m_padded as u64],
                [(NG as usize * 2) as u32, TILE_M as u32],
                (n_padded_lim * 8) as u64,
                sys::CUtensorMapSwizzle_enum::CU_TENSOR_MAP_SWIZZLE_NONE,
            )?;
            (map, 0i32, &gpu.kernel)
        }
        GemmTarget::TmaXor {
            dst,
            dst_rows,
            dst_stride,
            x_limb,
        } => {
            // Violating any of these is an encode failure, an illegal instruction, or a silent XOR
            // past the end of a row, so none of them may reach the launch.
            assert!(
                tma_xor_supported(dst_stride),
                "the reduce-store epilogue needs an even row stride, got {dst_stride}"
            );
            assert!(
                x_limb.is_multiple_of(2),
                "the reduce-store epilogue needs an even first limb, got {x_limb}"
            );
            assert!(
                x_limb + n_groups * NG as usize <= dst_stride,
                "the reduce-store epilogue would write limbs {x_limb}..{} of a {dst_stride}-limb \
                 row",
                x_limb + n_groups * NG as usize
            );
            assert!(dst_rows > 0 && dst_rows <= u32::MAX as usize);
            let map = encode_tma(
                dst,
                [(dst_stride * 2) as u64, dst_rows as u64],
                [(NG as usize * 2) as u32, TILE_M as u32],
                (dst_stride * 8) as u64,
                sys::CUtensorMapSwizzle_enum::CU_TENSOR_MAP_SWIZZLE_NONE,
            )?;
            (map, (x_limb * 2) as i32, &gpu.kernel_xor)
        }
    };

    // Dynamic SMEM per CTA: sA + sB + 2*sC (double-buffered) + 2*STAGES mbarriers.
    let tile_a = TILE_M * KL; // TILE_M-row A block
    let tile_b = NG as usize * 64 * KL; // (NG*64)-col B tile
    let smem_u64 = STAGES * tile_a + STAGES * tile_b + 2 * NG as usize * TILE_M + 2 * STAGES;
    let smem_bytes = (smem_u64 * std::mem::size_of::<u64>()) as u32;

    // Opt in to >48 KB shared memory (Hopper static default cap).
    kernel.set_attribute(
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
    let occ = kernel
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
        let mut lb = stream.launch_builder(kernel);
        lb.arg(&ta)
            .arg(&tb)
            .arg(&tc)
            .arg(&mt)
            .arg(&ng)
            .arg(&m_val)
            .arg(&k_val)
            .arg(&x_off);
        // SAFETY: the eight pushed arguments match the parameter list both GEMM entry points share,
        // in order and type; `smem_bytes` is the size the kernel was granted above via
        // MAX_DYNAMIC_SHARED_SIZE_BYTES, and the device buffers behind the tensor maps outlive this
        // closure (their guards are held until after the final synchronize).
        unsafe { lb.launch(cfg) }?;
        Ok(())
    };

    // Relaunching is idempotent only for the store; the reduce accumulates.
    debug_assert!(time_iters == 1 || matches!(target, GemmTarget::Store { .. }));
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
    Ok(start.elapsed().as_secs_f64() / time_iters as f64)
}

/// A bit-packed F₂ matrix resident in device memory.
///
/// Rows are `stride` limbs apart and hold one bit per entry, bits past `cols` zero: the layout
/// [`Matrix`] uses, as raw limbs. The point of it is persistence: uploaded once, mutated in place by
/// device kernels and downloaded once, so an algorithm made of many dependent launches pays for the
/// transfers only at its ends.
///
/// The row stride is always [`device_stride`]`(cols)`, a multiple of [`DEVICE_LIMB_MULTIPLE`]
/// limbs, whatever the column count: every constructor pads to it, and the limbs past `cols` are
/// zero. That invariant is what lets a GEMM always XOR its product straight into the matrix (see
/// [`tma_xor_supported`]), so the fields are private and [`Self::from_parts`] checks it.
pub struct DeviceMatrix {
    pub(super) buf: CudaSlice<u64>,
    pub(super) rows: usize,
    pub(super) cols: usize,
    pub(super) stride: usize,
}

impl DeviceMatrix {
    /// Assemble a matrix from a device buffer of at least `rows * stride` limbs, checking the
    /// stride invariant.
    pub(super) fn from_parts(buf: CudaSlice<u64>, rows: usize, cols: usize, stride: usize) -> Self {
        assert!(
            stride.is_multiple_of(DEVICE_LIMB_MULTIPLE) && stride >= cols.div_ceil(64),
            "DeviceMatrix: a {cols}-column matrix cannot have row stride {stride}"
        );
        assert!(buf.len() >= rows * stride, "DeviceMatrix: buffer too small");
        Self {
            buf,
            rows,
            cols,
            stride,
        }
    }

    /// Rows.
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Columns, not counting the zero padding up to the stride.
    pub fn cols(&self) -> usize {
        self.cols
    }

    /// Row stride in 64-bit limbs; see the type's documentation.
    pub fn stride(&self) -> usize {
        self.stride
    }

    /// The device buffer, `rows * stride` limbs (at least).
    pub fn buf(&self) -> &CudaSlice<u64> {
        &self.buf
    }
}

/// Row stride multiple, in limbs, of every [`DeviceMatrix`].
///
/// Two limbs make every row 16 bytes, what the GEMM's reduce-store epilogue needs to XOR into it.
pub const DEVICE_LIMB_MULTIPLE: usize = 2;

/// The row stride of a `cols`-column [`DeviceMatrix`]: `cols.div_ceil(64)` rounded up to
/// [`DEVICE_LIMB_MULTIPLE`], and never zero.
pub fn device_stride(cols: usize) -> usize {
    cols.div_ceil(64)
        .max(1)
        .next_multiple_of(DEVICE_LIMB_MULTIPLE)
}

/// Device staging per [`GpuContext::upload_rows`] chunk.
pub const UPLOAD_CHUNK_BYTES: usize = 256 << 20;

/// GEMMs whose product could not be XORed into its destination and went through a buffer instead.
static PRODUCT_FALLBACKS: AtomicU64 = AtomicU64::new(0);

/// How many GEMM-XOR updates have taken the product-buffer path in this process.
///
/// Every [`DeviceMatrix`] meets the reduce-store epilogue's alignment, so this stays zero; a
/// nonzero count means that invariant was broken somewhere.
pub fn product_fallbacks() -> u64 {
    PRODUCT_FALLBACKS.load(Ordering::Relaxed)
}

/// Count a product-buffer fallback, say so once per process, and fail a debug build: it cannot
/// happen while every destination is a [`DeviceMatrix`].
pub(super) fn note_product_fallback(rows: usize, stride: usize) {
    PRODUCT_FALLBACKS.fetch_add(1, Ordering::Relaxed);
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        eprintln!(
            "[fp-cuda] a {rows}-row matrix of row stride {stride} limbs cannot take a GEMM's \
             product into itself, so the product goes through a buffer of up to one more matrix"
        );
    });
    debug_assert!(
        false,
        "a {rows}-row matrix of row stride {stride} limbs broke the DeviceMatrix stride invariant"
    );
}

/// The GEMM's right operand, packed into its tiles ([`GpuContext::pack_b_dev`]).
struct PackedB {
    bt: CudaSlice<u64>,
    k_padded: usize,
    n_groups: usize,
    n_padded_lim: usize,
}

/// A 1-D launch config: `ceil(total / 256)` blocks of 256 threads.
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
    /// Device-resident F₂ GEMM over operands in the natural limb layout.
    ///
    /// `a_dev` is `m × k.div_ceil(64)` limbs and `b_dev` is `k × n.div_ceil(64)`. Both are packed
    /// into the wgmma tiles on the device, so nothing crosses to the host.
    ///
    /// Returns `C = A·B` as a **padded** buffer of `m_padded × n_padded_lim` limbs — valid data in
    /// the first `m` rows and first `n.div_ceil(64)` limbs of each — plus its row stride. The
    /// padding is what [`Self::xor_into_region`] consumes; a standalone caller trims it on readback.
    pub fn matmul_b1_dev(
        &self,
        a_dev: &CudaSlice<u64>,
        m: usize,
        k: usize,
        b_dev: &CudaSlice<u64>,
        n: usize,
    ) -> anyhow::Result<(CudaSlice<u64>, usize)> {
        self.matmul_b1_dev_strided(a_dev, m, k, k.div_ceil(64), b_dev, n)
    }

    /// Like [`Self::matmul_b1_dev`], but A may be a column block of a wider device buffer.
    ///
    /// `a_row_stride` is A's actual per-row limb stride (≥ `k.div_ceil(64)`); only the first
    /// `k.div_ceil(64)` limbs of each row are the operand. This lets a caller holding one large
    /// persistent matrix multiply a column block of it in place, without first copying the block
    /// out to a tightly packed buffer.
    pub fn matmul_b1_dev_strided(
        &self,
        a_dev: &CudaSlice<u64>,
        m: usize,
        k: usize,
        a_row_stride: usize,
        b_dev: &CudaSlice<u64>,
        n: usize,
    ) -> anyhow::Result<(CudaSlice<u64>, usize)> {
        assert!(
            a_row_stride >= k.div_ceil(64),
            "a_row_stride must cover k limbs"
        );
        assert_eq!(a_dev.len(), m * a_row_stride, "A limb count mismatch");
        assert_eq!(b_dev.len(), k * n.div_ceil(64), "B limb count mismatch");

        let stream = self.stream();
        let (a_int, m_padded) = self.pack_a_dev(&stream, &a_dev.slice(..), m, k, a_row_stride)?;
        let n_lim = n.div_ceil(64);
        let pb = self.pack_b_dev(&stream, &b_dev.slice(..), k, n_lim, n_lim, 0)?;

        // SAFETY: uninitialized is sound because the GEMM's bulk-tensor store overwrites every
        // output tile, padding included, before anything reads the buffer.
        let c_dev = unsafe { stream.alloc::<u64>(m_padded * pb.n_padded_lim) }?;
        {
            let (c_ptr, _gc) = c_dev.device_ptr(&stream);
            self.gemm_packed(
                &stream,
                &a_int,
                m_padded,
                &pb,
                GemmTarget::Store {
                    c: c_ptr,
                    n_padded_lim: pb.n_padded_lim,
                },
            )?;
        }
        Ok((c_dev, pb.n_padded_lim))
    }

    /// Pack the `m × k` left operand into the GEMM's interleaved K-major tiles.
    ///
    /// Rows of `a` are `a_row_stride` limbs apart, the first `k.div_ceil(64)` of them the operand.
    /// Returns the packed buffer and `m_padded`, the row count padded to whole M-tiles.
    fn pack_a_dev(
        &self,
        stream: &Arc<CudaStream>,
        a: &CudaView<'_, u64>,
        m: usize,
        k: usize,
        a_row_stride: usize,
    ) -> anyhow::Result<(CudaSlice<u64>, usize)> {
        let k_padded = k.next_multiple_of(TILE_K);
        let m_padded = m.next_multiple_of(TILE_M);
        let a_int_len = m_padded * (k_padded / 64);
        assert!(a_int_len <= u32::MAX as usize, "pack_a: packed A too large");
        // SAFETY: uninitialized is sound because `pack_a` writes all `a_int_len` elements, the
        // padding as explicit zeros, before anything reads the buffer.
        let a_int = unsafe { stream.alloc::<u64>(a_int_len) }?;
        let (m_orig, sa_orig, a_str, mt, total) = (
            m as u32,
            k.div_ceil(64) as u32,
            a_row_stride as u32,
            (m_padded / TILE_M) as u32,
            a_int_len as u32,
        );
        let mut lb = stream.launch_builder(&self.pack_a);
        lb.arg(&a_int)
            .arg(a)
            .arg(&m_orig)
            .arg(&sa_orig)
            .arg(&a_str)
            .arg(&mt)
            .arg(&total);
        // SAFETY: launch contract for `pack_a`.
        unsafe { lb.launch(cfg_1d(a_int_len)) }?;
        Ok((a_int, m_padded))
    }

    /// Pack the `k`-row right operand into the GEMM's bit-transposed K-major tiles.
    ///
    /// The operand is `lead` zero limbs followed by the first `src_limbs` limbs of each row of
    /// `b`, whose rows are `b_stride` limbs apart; its product limb `j` is product column limb
    /// `j - lead` of `b`'s. Packed once, it can feed several GEMMs.
    fn pack_b_dev(
        &self,
        stream: &Arc<CudaStream>,
        b: &CudaView<'_, u64>,
        k: usize,
        src_limbs: usize,
        b_stride: usize,
        lead: usize,
    ) -> anyhow::Result<PackedB> {
        assert!(
            b_stride >= src_limbs,
            "pack_b: stride narrower than the operand"
        );
        assert!(
            k == 0 || b.len() >= (k - 1) * b_stride + src_limbs,
            "pack_b: B limb count mismatch"
        );
        let n_lim = lead + src_limbs;
        let k_padded = k.next_multiple_of(TILE_K);
        let n_groups = n_lim.div_ceil(NG as usize);
        let n_padded_lim = n_groups * NG as usize;
        let bt_len = (k_padded / TILE_K) * n_groups * (NG as usize * 64 * KL);
        assert!(bt_len <= u32::MAX as usize, "pack_b: packed B too large");
        // SAFETY: as for `pack_a_dev`: `pack_b` writes all `bt_len` elements before any read.
        let bt = unsafe { stream.alloc::<u64>(bt_len) }?;
        let (k_orig, nl, bs, ld, ng, total) = (
            k as u32,
            n_lim as u32,
            b_stride as u32,
            lead as u32,
            n_groups as u32,
            bt_len as u32,
        );
        let mut lb = stream.launch_builder(&self.pack_b);
        lb.arg(&bt)
            .arg(b)
            .arg(&k_orig)
            .arg(&nl)
            .arg(&bs)
            .arg(&ld)
            .arg(&ng)
            .arg(&total);
        // SAFETY: launch contract for `pack_b`; the checks above keep its reads, at limbs below
        // `src_limbs` of rows below `k`, inside `b`.
        unsafe { lb.launch(cfg_1d(bt_len)) }?;
        Ok(PackedB {
            bt,
            k_padded,
            n_groups,
            n_padded_lim,
        })
    }

    /// One GEMM over operands packed by [`Self::pack_a_dev`] and [`Self::pack_b_dev`].
    fn gemm_packed(
        &self,
        stream: &Arc<CudaStream>,
        a_int: &CudaSlice<u64>,
        m_padded: usize,
        pb: &PackedB,
        target: GemmTarget,
    ) -> anyhow::Result<()> {
        run_gemm_kernel(
            self,
            stream,
            a_int,
            &pb.bt,
            target,
            m_padded,
            pb.k_padded,
            pb.n_groups,
            1,
        )?;
        Ok(())
    }

    /// XOR a padded GEMM output into a region of a persistent device matrix.
    ///
    /// `c_dev` is `m × c_stride` limbs, as [`Self::matmul_b1_dev`] returns it, and the update is
    /// `dst[j][dst_limb + col] ^= c_dev[j][col]` for `j < m`, `col < width`.
    ///
    /// Panics unless the region lies inside both buffers.
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
        // `xor_into` bounds its thread index and nothing else, so an index in range still
        // dereferences `j * stride + dst_limb + col`, which only these checks constrain. Getting
        // it wrong is an illegal device access, which kills the context for the whole process
        // rather than failing this call.
        let dst_end = dst_limb
            .checked_add(width)
            .expect("xor_into_region: destination region overflows");
        assert!(
            dst_end <= dst_stride,
            "xor_into_region: region ends at limb {dst_end}, past the {dst_stride}-limb row"
        );
        assert!(
            c_stride >= width,
            "xor_into_region: source stride {c_stride} is narrower than the {width}-limb update"
        );
        let need_dst = m
            .checked_mul(dst_stride)
            .expect("xor_into_region: destination extent overflows");
        let need_c = m
            .checked_mul(c_stride)
            .expect("xor_into_region: source extent overflows");
        let total = m
            .checked_mul(width)
            .expect("xor_into_region: launch size overflows");
        assert!(
            dst.len() >= need_dst,
            "xor_into_region: destination is {} limbs, region needs {need_dst}",
            dst.len()
        );
        assert!(
            c_dev.len() >= need_c,
            "xor_into_region: source is {} limbs, region needs {need_c}",
            c_dev.len()
        );
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
        // SAFETY: launch contract for `xor_into`; the checks above keep every dereference inside
        // both buffers.
        unsafe { lb.launch(cfg_1d(total)) }?;
        Ok(())
    }

    /// Upload `rows × cols` bits, `rows` tight rows of `cols.div_ceil(64)` limbs, to the device.
    ///
    /// The device copy's rows are padded to [`device_stride`]; the device peak is the matrix plus
    /// one staging chunk ([`Self::upload_rows`]).
    pub fn upload(&self, data: &[u64], rows: usize, cols: usize) -> anyhow::Result<DeviceMatrix> {
        let tight = cols.div_ceil(64);
        assert_eq!(data.len(), rows * tight, "limb count mismatch");
        let mut dm = self.alloc_matrix(rows, cols)?;
        if tight > 0 {
            self.upload_rows(&mut dm, 0, data, tight)?;
        } else {
            self.stream().memset_zeros(&mut dm.buf)?;
        }
        Ok(dm)
    }

    /// Allocate an uninitialized `rows × cols` [`DeviceMatrix`], for the caller to fill.
    pub fn alloc_matrix(&self, rows: usize, cols: usize) -> anyhow::Result<DeviceMatrix> {
        let stride = device_stride(cols);
        // SAFETY: uninitialized is sound because the matrix is handed out only to be filled, by
        // `upload_rows` or by the caller, before anything reads it.
        let buf = unsafe { self.stream().alloc::<u64>((rows * stride).max(1)) }?;
        Ok(DeviceMatrix::from_parts(buf, rows, cols, stride))
    }

    /// Write rows `[row0, row0 + n)` of `dm` from `data`, `n` rows of `src_stride` limbs,
    /// zero-filling each row past `src_stride` limbs.
    ///
    /// When the strides differ the rows go through one device staging buffer of at most
    /// [`UPLOAD_CHUNK_BYTES`], so the device never holds a second copy of the matrix; equal strides
    /// copy straight in. The limbs of `data` past `dm.cols()` must be zero, as a [`Matrix`]'s are.
    pub fn upload_rows(
        &self,
        dm: &mut DeviceMatrix,
        row0: usize,
        data: &[u64],
        src_stride: usize,
    ) -> anyhow::Result<()> {
        assert!(
            src_stride >= 1 && src_stride <= dm.stride,
            "upload_rows: source stride {src_stride} does not fit a {}-limb row",
            dm.stride
        );
        assert!(
            data.len().is_multiple_of(src_stride),
            "upload_rows: partial row"
        );
        let n = data.len() / src_stride;
        assert!(row0 + n <= dm.rows, "upload_rows: rows out of range");
        if n == 0 {
            return Ok(());
        }
        let stream = self.stream();
        let ds = dm.stride;
        if src_stride == ds {
            let mut view = dm.buf.slice_mut(row0 * ds..(row0 + n) * ds);
            stream.memcpy_htod(data, &mut view)?;
            return Ok(stream.synchronize()?);
        }
        let chunk = (UPLOAD_CHUNK_BYTES / (8 * src_stride)).clamp(1, n);
        // SAFETY: uninitialized is sound because each chunk's rows are copied in before
        // `restride_rows` reads them, and it reads no further.
        let mut staging = unsafe { stream.alloc::<u64>(chunk * src_stride) }?;
        let mut a = 0;
        while a < n {
            let k = chunk.min(n - a);
            {
                let mut sv = staging.slice_mut(..k * src_stride);
                stream.memcpy_htod(&data[a * src_stride..(a + k) * src_stride], &mut sv)?;
            }
            let mut dv = dm.buf.slice_mut((row0 + a) * ds..(row0 + a + k) * ds);
            let (nr, ss, dst_s) = (k as u32, src_stride as u32, ds as u32);
            let mut lb = stream.launch_builder(&self.restride_rows);
            lb.arg(&mut dv).arg(&staging).arg(&nr).arg(&ss).arg(&dst_s);
            // SAFETY: launch contract for `restride_rows`; `dv` is exactly `k` rows of `ds` limbs
            // and the staging buffer holds `k` rows of `src_stride`.
            unsafe { lb.launch(cfg_1d(k * ds)) }?;
            a += k;
        }
        Ok(stream.synchronize()?)
    }

    /// Wait for this thread's stream to drain.
    ///
    /// `cudarc`'s `memcpy_dtoh` is `cuMemcpyDtoHAsync`: a download appears synchronous only
    /// because a device-to-host copy into pageable memory blocks until it completes, which a
    /// page-locked destination would silently stop doing. Every download ends here instead.
    fn sync(&self) -> anyhow::Result<()> {
        Ok(self.stream().synchronize()?)
    }

    /// Download a [`DeviceMatrix`]'s limbs, `rows × stride` of them.
    pub fn download(&self, dm: &DeviceMatrix) -> anyhow::Result<Vec<u64>> {
        let out = self.stream().clone_dtoh(&dm.buf)?;
        self.sync()?;
        Ok(out)
    }

    /// Download into an existing host buffer, allocating nothing.
    ///
    /// [`Self::download`] allocates a fresh `Vec<u64>` per call, which at large sizes is a
    /// multi-GiB allocation every time. Callers in a loop should hold one buffer and reuse it.
    pub fn download_into(&self, dm: &DeviceMatrix, out: &mut [u64]) -> anyhow::Result<()> {
        assert_eq!(
            out.len(),
            dm.rows * dm.stride,
            "download_into: buffer is {} limbs, matrix needs {}",
            out.len(),
            dm.rows * dm.stride
        );
        self.stream().memcpy_dtoh(&dm.buf, out)?;
        self.sync()
    }

    /// Download a device `u32` buffer (e.g. a `perm` vector).
    pub fn download_u32(&self, s: &CudaSlice<u32>) -> anyhow::Result<Vec<u32>> {
        let out = self.stream().clone_dtoh(s)?;
        self.sync()?;
        Ok(out)
    }

    /// Accumulate a product into a region of a persistent device matrix: `dst[:, col_off:] ^= l·u`.
    ///
    /// `l` is `dst.rows × k` and `u` is `k × t`, with `k = l.cols = u.rows` and `t = u.cols`.
    /// `col_off` must be a limb boundary, and the trailing region `[col_off, dst.cols)` must be
    /// exactly `t` columns wide, so that the XOR lands on whole limbs.
    ///
    /// The GEMM XORs its product straight into `dst`, which its stride invariant makes possible,
    /// starting one limb early against a zero leading limb of `u` when `col_off` is an odd limb.
    /// The product never exists as a buffer, nor reaches the host.
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
        let x_limb = col_off / 64;
        let t_lim = t.div_ceil(64);
        let stream = self.stream();
        let (a_int, m_padded) = self.pack_a_dev(&stream, &l.buf.slice(..), m, k, l.stride)?;
        // The reduce store needs every box to start on an even limb, and writes the product's
        // padding to whole NG-limb column groups too (as zeros): `dst`'s even stride leaves room.
        if tma_xor_supported(dst.stride) {
            let lead = x_limb % 2;
            let pb = self.pack_b_dev(&stream, &u.buf.slice(..), k, t_lim, u.stride, lead)?;
            let (dst_ptr, _gd) = dst.buf.device_ptr_mut(&stream);
            return self.gemm_packed(
                &stream,
                &a_int,
                m_padded,
                &pb,
                GemmTarget::TmaXor {
                    dst: dst_ptr,
                    dst_rows: m,
                    dst_stride: dst.stride,
                    x_limb: x_limb - lead,
                },
            );
        }
        note_product_fallback(m, dst.stride);
        let pb = self.pack_b_dev(&stream, &u.buf.slice(..), k, t_lim, u.stride, 0)?;
        // SAFETY: uninitialized is sound because the GEMM's bulk-tensor store overwrites every
        // output tile before `xor_into` reads it.
        let c_dev = unsafe { stream.alloc::<u64>(m_padded * pb.n_padded_lim) }?;
        {
            let (c_ptr, _gc) = c_dev.device_ptr(&stream);
            self.gemm_packed(
                &stream,
                &a_int,
                m_padded,
                &pb,
                GemmTarget::Store {
                    c: c_ptr,
                    n_padded_lim: pb.n_padded_lim,
                },
            )?;
        }
        self.xor_into_region(
            &stream,
            &mut dst.buf,
            &c_dev,
            m,
            t_lim,
            dst.stride,
            x_limb,
            pb.n_padded_lim,
        )?;
        Ok(stream.synchronize()?)
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
        for kernel in [
            "matmul_b1_kernel",
            "matmul_b1_kernel_xor",
            "transpose_tile_b1_kernel",
            "pack_a",
            "pack_b",
            "xor_into",
            "restride_rows",
        ] {
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

    /// `m`'s rows as tight limbs, `m.columns().div_ceil(64)` per row.
    fn tight_limbs(m: &Matrix) -> Vec<u64> {
        let limbs = m.columns().div_ceil(64);
        (0..m.rows())
            .flat_map(|i| m.data()[i * m.stride()..][..limbs].iter().copied())
            .collect()
    }

    /// `a · b` on the CPU.
    fn cpu_mul(a: &Matrix, b: &Matrix) -> Matrix {
        if a.physical_rows().is_multiple_of(64) && b.physical_rows().is_multiple_of(64) {
            a.fast_mul_sequential(b)
        } else {
            a.naive_mul(b)
        }
    }

    /// Operands for `dst[:, x_limb*64:] ^= l·u`: `dst` (`m × (x_limb*64 + t)`), `l` (`m × k`) and
    /// `u` (`k × t`).
    ///
    /// `x_limb` and `t` together give the logical width and first limb either parity, so both an
    /// even and an odd first limb (the zero lead limb) are drawn, over a padded stride.
    fn arb_xor_operands() -> impl Strategy<Value = (Matrix, Matrix, Matrix, usize)> {
        (
            1..=2 * TILE_M + 1,
            1..=TILE_K + 65,
            1..=2 * NB + 1,
            0..=3usize,
        )
            .prop_flat_map(|(m, k, t, x_limb)| {
                (
                    arb_matrix(m, x_limb * 64 + t),
                    arb_matrix(m, k),
                    arb_matrix(k, t),
                    Just(x_limb),
                )
            })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        /// The device-resident GEMM is the product, with A read through a row stride wider than
        /// the operand.
        #[test]
        fn matmul_b1_dev_is_mul((a, b, spare, _iters) in arb_operands()) {
            let Some(gpu) = gpu() else {
                return Ok(()); // no usable GPU/driver in this environment — nothing to exercise
            };
            let reference = cpu_mul(&a, &b);
            let (m, k, n) = (a.rows(), a.columns(), b.columns());
            let wide = with_spare_columns(&a, spare);
            let stream = gpu.stream();
            let a_dev = stream.clone_htod(&wide.data()[..m * wide.stride()]).unwrap();
            let b_dev = stream.clone_htod(&tight_limbs(&b)).unwrap();
            let (c_dev, c_stride) =
                gpu.matmul_b1_dev_strided(&a_dev, m, k, wide.stride(), &b_dev, n).unwrap();
            let c_all = stream.clone_dtoh(&c_dev).unwrap();
            stream.synchronize().unwrap();
            let n_lim = n.div_ceil(64);
            let c: Vec<u64> = c_all
                .chunks_exact(c_stride)
                .take(m)
                .flat_map(|row| row[..n_lim].iter().copied())
                .collect();
            prop_assert_eq!(Matrix::from_data(TWO, m, n, c), reference);
        }

        /// `gemm_xor_into` XORs exactly the product into the trailing region and leaves the
        /// leading columns alone, without ever taking the product-buffer fallback.
        #[test]
        fn gemm_xor_into_xors_the_product((dst, l, u, x_limb) in arb_xor_operands()) {
            let Some(gpu) = gpu() else {
                return Ok(()); // no usable GPU/driver in this environment — nothing to exercise
            };
            let product = tight_limbs(&cpu_mul(&l, &u));
            let mut expected = tight_limbs(&dst);
            let (stride, t_lim) = (dst.columns().div_ceil(64), u.columns().div_ceil(64));
            for (row, prow) in expected.chunks_exact_mut(stride).zip(product.chunks_exact(t_lim)) {
                for (d, p) in row[x_limb..].iter_mut().zip(prow) {
                    *d ^= p;
                }
            }

            let mut d = gpu.upload(&tight_limbs(&dst), dst.rows(), dst.columns()).unwrap();
            let l_dev = gpu.upload(&tight_limbs(&l), l.rows(), l.columns()).unwrap();
            let u_dev = gpu.upload(&tight_limbs(&u), u.rows(), u.columns()).unwrap();
            prop_assert_eq!(d.stride(), device_stride(dst.columns()));
            gpu.gemm_xor_into(&mut d, &l_dev, &u_dev, x_limb * 64).unwrap();
            let got: Vec<u64> = gpu
                .download(&d)
                .unwrap()
                .chunks_exact(d.stride())
                .flat_map(|row| row[..stride].iter().copied())
                .collect();
            prop_assert_eq!(got, expected);
            prop_assert_eq!(product_fallbacks(), 0, "an update went through a product buffer");
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
