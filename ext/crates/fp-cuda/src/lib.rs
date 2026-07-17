//! CUDA backend for `fp::blas` F_2 matrix multiplication on Hopper.
//!
//! A is pre-interleaved on the host and loaded via TMA with 128B swizzle.
//! B is pre-transposed + CM-blocked on the host and loaded via memcpy.
//! The kernel is a thin wrapper around wgmma.b1 m64n64k256.

use std::{ffi::c_void, mem::MaybeUninit, sync::Arc};

use cuda_core::{
    CudaContext, CudaFunction, CudaModule, DeviceBuffer, launch_kernel_on_stream,
    sys::{
        CUdeviceptr, CUresult, CUtensorMap,
        CUtensorMapDataType_enum_CU_TENSOR_MAP_DATA_TYPE_UINT32 as DATA_UINT32,
        CUtensorMapFloatOOBfill_enum_CU_TENSOR_MAP_FLOAT_OOB_FILL_NONE as OOB_NONE,
        CUtensorMapInterleave_enum_CU_TENSOR_MAP_INTERLEAVE_NONE as INTERLEAVE_NONE,
        CUtensorMapL2promotion_enum_CU_TENSOR_MAP_L2_PROMOTION_NONE as L2_NONE,
        CUtensorMapSwizzle_enum_CU_TENSOR_MAP_SWIZZLE_128B as SWIZZLE_128B,
        CUtensorMapSwizzle_enum_CU_TENSOR_MAP_SWIZZLE_NONE as SWIZZLE_NONE, cuTensorMapEncodeTiled,
        cudaError_enum_CUDA_SUCCESS as CUDA_SUCCESS,
    },
};
use fp::{matrix::Matrix, prime::TWO};

static PTX_IMAGE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/matmul_b1.ptx"));

const TILE_M: usize = 64;
const TILE_K: usize = 256;
const KL: usize = TILE_K / 64; // 4
const THREADS: u32 = 128;
const NG: u32 = 4;

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

    let stream = gpu.ctx.default_stream();

    let a_limbs = matrix_to_u64s(a);
    let b_limbs = matrix_to_u64s(b);

    let a_padded = pad_2d(&a_limbs, m, k.div_ceil(64), m_padded, k_padded / 64);
    let b_padded = pad_2d(&b_limbs, k, n_lim, k_padded, n_lim);

    // Pre-arrange A into interleaved 128-byte blocks for TMA 128B swizzle.
    let a_interleaved = interleave_a(&a_padded, m_padded, k_padded);
    // Pre-transpose B into CM-blocked tiles.
    let bt = transpose_b(&b_padded, k_padded, n_lim);

    let a_dev = DeviceBuffer::from_host(&stream, &a_interleaved)?;
    let bt_dev = DeviceBuffer::from_host(&stream, &bt)?;
    let c_dev = DeviceBuffer::<u64>::zeroed(&stream, m_padded * n_lim)?;

    // TMA tensor map for A.
    // The interleaved A is a 2D array of UINT32 elements:
    //   dim[0] = 32 (32 × 4 bytes = 128 bytes per super-row)
    //   dim[1] = k_chunks × m_tiles × 16 (16 super-rows per tile)
    //   stride[0] = 128 bytes (tightly packed)
    //   box = [32, 16] → 2048 bytes per TMA load
    let tma_a = {
        let mut tmap = MaybeUninit::<CUtensorMap>::uninit();
        let total_rows = (k_chunks * m_tiles * 16) as u64;
        let gdim: [u64; 2] = [32, total_rows];
        let gstride: [u64; 1] = [128]; // bytes per row
        let boxdim: [u32; 2] = [32, 16];
        let elemstride: [u32; 2] = [1, 1];
        let res: CUresult = unsafe {
            cuTensorMapEncodeTiled(
                tmap.as_mut_ptr(),
                DATA_UINT32,
                2,
                a_dev.cu_deviceptr() as *mut c_void,
                gdim.as_ptr(),
                gstride.as_ptr(),
                boxdim.as_ptr(),
                elemstride.as_ptr(),
                INTERLEAVE_NONE,
                SWIZZLE_NONE,
                L2_NONE,
                OOB_NONE,
            )
        };
        if res != CUDA_SUCCESS {
            return Err(format!("cuTensorMapEncodeTiled failed: {res:?}").into());
        }
        unsafe { tmap.assume_init() }
    };

    let mut tma_storage = tma_a;
    let mut mt: u32 = m_tiles as u32;
    let mut bt_ptr: CUdeviceptr = bt_dev.cu_deviceptr();
    let mut m_val: u32 = m_padded as u32;
    let mut k_val: u32 = k_padded as u32;
    let mut nl: u32 = n_lim as u32;
    let mut c_ptr: CUdeviceptr = c_dev.cu_deviceptr();

    let mut params: [*mut c_void; 7] = [
        &mut tma_storage as *mut _ as *mut c_void,
        &mut mt as *mut _ as *mut c_void,
        &mut bt_ptr as *mut _ as *mut c_void,
        &mut m_val as *mut _ as *mut c_void,
        &mut k_val as *mut _ as *mut c_void,
        &mut nl as *mut _ as *mut c_void,
        &mut c_ptr as *mut _ as *mut c_void,
    ];

    let grid_x = (nl + NG - 1) / NG;
    let grid_y = m_val / TILE_M as u32;

    unsafe {
        launch_kernel_on_stream(
            &gpu.kernel,
            (grid_x, grid_y, 1),
            (THREADS, 1, 1),
            0,
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

/// CM-blocked index within a 64-col × 4-K-limb tile (256 u64s).
fn cm(row: usize, kl: usize) -> usize {
    (row / 8) * 32 + (kl / 2) * 16 + (row % 8) * 2 + (kl % 2)
}

/// Pre-interleave A for TMA 128B swizzle.
///
/// Output: contiguous tiles, each 2048 bytes = 16 super-rows of 128 bytes.
/// Each super-row holds one core matrix: 8 rows × 2 K-limbs = 16 u64s = 128 bytes.
/// Layout within tile matches cm() ordering:
///   super_row[rg*2 + kg], where rg=0..7 (row group) and kg=0..1 (K group).
///   Within super-row: u64 at offset 2*r + kl_sub.
///
/// Tiles are ordered: for K-chunk kk=0..k_chunks-1, then M-tile bi=0..m_tiles-1.
fn interleave_a(a: &[u64], m: usize, k: usize) -> Vec<u64> {
    let sa = k / 64;
    let k_chunks = k / TILE_K;
    let m_tiles = m / TILE_M;
    let tile_u64s = TILE_M * KL; // 256
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
                    out[base + cm(row, kl)] = val;
                }
            }
        }
    }
    out
}

/// Pre-transpose B into CM-blocked tiles.
fn transpose_b(b: &[u64], k: usize, n_lim: usize) -> Vec<u64> {
    let k_chunks = k / TILE_K;
    let tile = 64 * KL;
    let mut out = vec![0u64; k_chunks * n_lim * tile];
    let mut buf = [0u64; 256];

    for kk in 0..k_chunks {
        for cl in 0..n_lim {
            let base = (kk * n_lim + cl) * tile;
            for i in 0..256usize {
                let br = kk * 256 + i;
                buf[i] = if br < k { b[br * n_lim + cl] } else { 0 };
            }
            for kl in 0..KL {
                for j in 0..64usize {
                    let mut val: u64 = 0;
                    for bit in 0..64usize {
                        val |= ((buf[kl * 64 + bit] >> j) & 1) << bit;
                    }
                    out[base + cm(j, kl)] = val;
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
