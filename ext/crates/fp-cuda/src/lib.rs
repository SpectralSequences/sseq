//! CUDA backend for `fp::blas` F_2 matrix multiplication on Hopper.
//!
//! Both operands are pre-arranged on the host as plain row-major K-major tiles
//! and loaded via TMA with 128B swizzle, which lands them in the SMEM layout the
//! swizzled wgmma matrix descriptors expect. The kernel is a thin wrapper around
//! wgmma.b1 m64n64k256.

use std::{ffi::c_void, mem::MaybeUninit, sync::Arc};

use cuda_core::{
    CudaContext, CudaFunction, CudaModule, DeviceBuffer, launch_kernel_on_stream,
    sys::{
        CUdeviceptr,
        CUfunction_attribute_enum_CU_FUNC_ATTRIBUTE_MAX_DYNAMIC_SHARED_SIZE_BYTES as FUNC_ATTR_MAX_DSMEM,
        CUresult, CUtensorMap,
        CUtensorMapDataType_enum_CU_TENSOR_MAP_DATA_TYPE_UINT32 as DATA_UINT32,
        CUtensorMapFloatOOBfill_enum_CU_TENSOR_MAP_FLOAT_OOB_FILL_NONE as OOB_NONE,
        CUtensorMapInterleave_enum_CU_TENSOR_MAP_INTERLEAVE_NONE as INTERLEAVE_NONE,
        CUtensorMapL2promotion_enum_CU_TENSOR_MAP_L2_PROMOTION_NONE as L2_NONE,
        CUtensorMapSwizzle_enum_CU_TENSOR_MAP_SWIZZLE_128B as SWIZZLE_128B, cuFuncSetAttribute,
        cuTensorMapEncodeTiled, cudaError_enum_CUDA_SUCCESS as CUDA_SUCCESS,
    },
};
use fp::{matrix::Matrix, prime::TWO};

static PTX_IMAGE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/matmul_b1.ptx"));

const TILE_M: usize = 64;
const TILE_K: usize = 1024;
const KL: usize = TILE_K / 64; // 16
const THREADS: u32 = 256; // 2 warpgroups: producer (0..128) + consumer (128..256)
const NG: u32 = 4;
const STAGES: usize = 2; // K-loop pipeline depth; must match the kernel

pub struct GpuContext {
    ctx: Arc<CudaContext>,
    #[allow(dead_code)]
    module: Arc<CudaModule>,
    kernel: CudaFunction,
}

impl GpuContext {
    pub fn new(device_id: usize) -> Result<Self, Box<dyn std::error::Error>> {
        let ctx = CudaContext::new(device_id)?;
        let module = ctx.load_module_from_image(PTX_IMAGE)?;
        let kernel = module.load_function("matmul_b1_kernel")?;
        Ok(Self {
            ctx,
            module,
            kernel,
        })
    }

    pub fn compute_capability(&self) -> Result<(i32, i32), Box<dyn std::error::Error>> {
        Ok(self.ctx.compute_capability()?)
    }

    pub fn default_stream(&self) -> Arc<cuda_core::CudaStream> {
        self.ctx.default_stream()
    }

    pub fn kernel(&self) -> &CudaFunction {
        &self.kernel
    }
}

pub fn matmul_b1(
    gpu: &GpuContext,
    a: &Matrix,
    b: &Matrix,
) -> Result<Matrix, Box<dyn std::error::Error>> {
    assert_eq!(a.prime(), TWO);
    assert_eq!(b.prime(), TWO);
    assert_eq!(a.columns(), b.rows());

    let m = a.rows();
    let k = a.columns();
    let n = b.columns();
    let n_lim = n.div_ceil(64);

    let k_padded = k.next_multiple_of(TILE_K);
    let m_padded = m.next_multiple_of(TILE_M);
    let m_tiles = m_padded / TILE_M;
    let k_chunks = k_padded / TILE_K;
    // Each CTA computes a 256-column (NG-limb) group with one m64n256 wgmma, so
    // B is grouped/padded to whole 256-column tiles.
    let n_groups = n_lim.div_ceil(NG as usize);

    let stream = gpu.ctx.default_stream();

    let a_limbs = matrix_to_u64s(a);
    let b_limbs = matrix_to_u64s(b);

    let a_padded = pad_2d(&a_limbs, m, k.div_ceil(64), m_padded, k_padded / 64);
    let b_padded = pad_2d(&b_limbs, k, n_lim, k_padded, n_lim);

    // Gather A into row-major K-major tiles; the TMA applies the 128B swizzle.
    let a_interleaved = interleave_a(&a_padded, m_padded, k_padded);
    // Pre-transpose B into row-major K-major tiles (swizzled by the TMA).
    let bt = transpose_b(&b_padded, k_padded, n_lim);

    let a_dev = DeviceBuffer::from_host(&stream, &a_interleaved)?;
    let bt_dev = DeviceBuffer::from_host(&stream, &bt)?;
    let c_dev = DeviceBuffer::<u64>::zeroed(&stream, m_padded * n_lim)?;

    // TMA tensor maps for A and B. Both views are plain row-major tiles whose
    // inner dim is 128 bytes (32 UINT32 elements = one 128B swizzle row); the
    // TMA applies the swizzle on load. They differ in tile height: A is a
    // 64-row tile per (k_chunk, M-tile); B is a 256-column tile per
    // (k_chunk, 256-col group), fed to one m64n256 wgmma.
    let encode_tile_tma = |dev_ptr: CUdeviceptr,
                           outer_tiles: u64,
                           box_rows: u32|
     -> Result<CUtensorMap, Box<dyn std::error::Error>> {
        let mut tmap = MaybeUninit::<CUtensorMap>::uninit();
        let gdim: [u64; 2] = [32, outer_tiles * box_rows as u64];
        let gstride: [u64; 1] = [128]; // bytes per row
        let boxdim: [u32; 2] = [32, box_rows];
        let elemstride: [u32; 2] = [1, 1];
        let res: CUresult = unsafe {
            cuTensorMapEncodeTiled(
                tmap.as_mut_ptr(),
                DATA_UINT32,
                2,
                dev_ptr as *mut c_void,
                gdim.as_ptr(),
                gstride.as_ptr(),
                boxdim.as_ptr(),
                elemstride.as_ptr(),
                INTERLEAVE_NONE,
                SWIZZLE_128B,
                L2_NONE,
                OOB_NONE,
            )
        };
        if res != CUDA_SUCCESS {
            return Err(format!("cuTensorMapEncodeTiled failed: {res:?}").into());
        }
        Ok(unsafe { tmap.assume_init() })
    };

    let tma_a = encode_tile_tma(
        a_dev.cu_deviceptr(),
        (k_chunks * m_tiles) as u64,
        TILE_M as u32,
    )?;
    let tma_b = encode_tile_tma(
        bt_dev.cu_deviceptr(),
        (k_chunks * n_groups) as u64,
        (NG as usize * 64) as u32,
    )?;

    let mut tma_a_storage = tma_a;
    let mut tma_b_storage = tma_b;
    let mut mt: u32 = m_tiles as u32;
    let mut m_val: u32 = m_padded as u32;
    let mut k_val: u32 = k_padded as u32;
    let mut nl: u32 = n_lim as u32;
    let mut c_ptr: CUdeviceptr = c_dev.cu_deviceptr();

    let mut params: [*mut c_void; 7] = [
        &mut tma_a_storage as *mut _ as *mut c_void,
        &mut tma_b_storage as *mut _ as *mut c_void,
        &mut mt as *mut _ as *mut c_void,
        &mut m_val as *mut _ as *mut c_void,
        &mut k_val as *mut _ as *mut c_void,
        &mut nl as *mut _ as *mut c_void,
        &mut c_ptr as *mut _ as *mut c_void,
    ];

    let grid_x = (nl + NG - 1) / NG;
    let grid_y = m_val / TILE_M as u32;

    // Dynamic SMEM per CTA: sA + sB + sC + 2*STAGES mbarriers (see kernel).
    let tile_a = TILE_M * KL; // 64-row A tile
    let tile_b = NG as usize * 64 * KL; // 256-col B tile
    let smem_u64 = STAGES * tile_a + STAGES * tile_b + NG as usize * TILE_M + 2 * STAGES;
    let smem_bytes = (smem_u64 * std::mem::size_of::<u64>()) as u32;

    // Opt in to >48 KB shared memory (Hopper static default cap).
    let res: CUresult = unsafe {
        cuFuncSetAttribute(
            gpu.kernel.cu_function(),
            FUNC_ATTR_MAX_DSMEM as _,
            smem_bytes as i32,
        )
    };
    if res != CUDA_SUCCESS {
        return Err(format!("cuFuncSetAttribute(MAX_DYNAMIC_SHARED) failed: {res:?}").into());
    }

    unsafe {
        launch_kernel_on_stream(
            &gpu.kernel,
            (grid_x, grid_y, 1),
            (THREADS, 1, 1),
            smem_bytes,
            &stream,
            &mut params,
        )?;
    }
    stream.synchronize()?;

    let c_all = c_dev.to_host_vec(&stream)?;
    let c_limbs: Vec<u64> = c_all
        .chunks_exact(n_lim)
        .take(m)
        .flat_map(|row| row.iter().copied())
        .collect();
    Ok(Matrix::from_data(TWO, m, n, c_limbs))
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
/// Each (k_chunk, 256-col group) tile is NB = NG*64 = 256 rows (= the 256 output
/// columns of the group) × KL u64s (= TILE_K K bits), fed to one m64n256 wgmma.
/// Operand row `lg*64 + jj` is output column `cg*256 + lg*64 + jj`; element
/// `[..][kl] bit` is bit `jj` of `B[k_chunk*TILE_K + kl*64 + bit][cg*NG + lg]`.
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
                for i in 0..TILE_K {
                    let br = kk * TILE_K + i;
                    buf[i] = if br < k { b[br * n_lim + limb] } else { 0 };
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

fn matrix_to_u64s(m: &Matrix) -> Vec<u64> {
    let stride = m.columns().div_ceil(64);
    let mut bytes = Vec::with_capacity(m.rows() * stride * 8);
    m.to_bytes(&mut bytes).expect("Vec writes never fail");
    bytes
        .chunks_exact(8)
        .map(|c| u64::from_le_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]))
        .collect()
}
