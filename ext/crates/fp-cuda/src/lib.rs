//! CUDA backend for `fp::blas` F_2 matrix multiplication on Hopper.
//!
//! Both operands are pre-arranged on the host as plain row-major K-major tiles
//! and loaded via TMA with 128B swizzle, which lands them in the SMEM layout the
//! swizzled wgmma matrix descriptors expect. The kernel register-blocks a
//! TILE_M×(NG*64) output tile per CTA out of MSTRIPS m64n128 wgmma.b1 strips
//! that share each loaded B tile (cuts operand-refill bandwidth, the bottleneck).

use std::{ffi::c_void, mem::MaybeUninit, sync::Arc, time::Instant};

use cudarc::{
    driver::{
        CudaContext, CudaFunction, CudaModule, CudaStream, DevicePtr, DeviceRepr, LaunchConfig,
        PushKernelArg, sys,
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

pub struct GpuContext {
    ctx: Arc<CudaContext>,
    #[allow(dead_code)]
    module: Arc<CudaModule>,
    kernel: CudaFunction,
}

impl GpuContext {
    pub fn new(device_id: usize) -> Result<Self, Box<dyn std::error::Error>> {
        let ctx = CudaContext::new(device_id)?;
        let ptx = Ptx::from_src(String::from_utf8(PTX_IMAGE.to_vec())?);
        let module = ctx.load_module(ptx)?;
        let kernel = module.load_function("matmul_b1_kernel")?;
        Ok(Self {
            ctx,
            module,
            kernel,
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
    let m_tiles = m_padded / TILE_M;
    let k_chunks = k_padded / TILE_K;
    // Each CTA computes a TILE_M×(NG*64) output block via MSTRIPS m64n128 wgmmas,
    // so B (and the C output) are grouped/padded to whole NG-limb column tiles.
    let n_groups = n_lim.div_ceil(NG as usize);
    let n_padded_lim = n_groups * NG as usize;

    let stream = gpu.ctx.default_stream();

    let a_padded = pad_2d(a, m, k.div_ceil(64), m_padded, k_padded / 64);
    let b_padded = pad_2d(b, k, n_lim, k_padded, n_lim);

    // Gather A into row-major K-major tiles; the TMA applies the 128B swizzle.
    let a_interleaved = interleave_a(&a_padded, m_padded, k_padded);
    // Pre-transpose B into row-major K-major tiles (swizzled by the TMA).
    let bt = transpose_b(&b_padded, k_padded, n_lim);

    let a_dev = stream.clone_htod(&a_interleaved)?;
    let bt_dev = stream.clone_htod(&bt)?;
    let c_dev = stream.alloc_zeros::<u64>(m_padded * n_padded_lim)?;

    // Raw device addresses for the TMA descriptors. The returned guards keep the
    // reads ordered on the stream; hold them until after the launch.
    let (a_ptr, _ga) = a_dev.device_ptr(&stream);
    let (b_ptr, _gb) = bt_dev.device_ptr(&stream);
    let (c_ptr, _gc) = c_dev.device_ptr(&stream);

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

    let c_all = stream.clone_dtoh(&c_dev)?;
    let c_limbs: Vec<u64> = c_all
        .chunks_exact(n_padded_lim)
        .take(m)
        .flat_map(|row| row[..n_lim].iter().copied())
        .collect();
    Ok((c_limbs, kernel_secs))
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
