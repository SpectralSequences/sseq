use std::{ffi::c_void, time::Instant};

use cuda_core::{DeviceBuffer, launch_kernel_on_stream, sys::CUdeviceptr};
use fp::{matrix::Matrix, prime::TWO};
use fp_cuda::GpuContext;
use rand::Rng;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let gpu = GpuContext::new(0)?;
    println!("=== Time breakdown for 131072 x 131072 x 131072 (2.1 GB per matrix) ===\n");

    let mut rng = rand::rng();
    let m: usize = 131072;
    let k: usize = 131072;
    let n: usize = 131072;
    let n_lim = n / 64;
    let k_padded = k; // already multiple of 256
    let m_padded = m; // already multiple of 64
    let sa = k / 64;

    // 1. Generate random data
    let t = Instant::now();
    let a_data: Vec<u64> = (0..m * sa).map(|_| rng.random()).collect();
    let b_data: Vec<u64> = (0..k * n_lim).map(|_| rng.random()).collect();
    println!(
        "  RNG generation:     {:>8.1} ms",
        t.elapsed().as_secs_f64() * 1e3
    );

    // 2. Host transpose
    let t = Instant::now();
    let bt = transpose_b_host(&b_data, k_padded, n_lim);
    println!(
        "  Host B transpose:   {:>8.1} ms",
        t.elapsed().as_secs_f64() * 1e3
    );

    // 3. H2D transfer
    let stream = gpu.default_stream();
    let t = Instant::now();
    let a_dev = DeviceBuffer::from_host(&stream, &a_data)?;
    let bt_dev = DeviceBuffer::from_host(&stream, &bt)?;
    let c_dev = DeviceBuffer::<u64>::zeroed(&stream, m_padded * n_lim)?;
    stream.synchronize()?;
    println!(
        "  H2D transfer:       {:>8.1} ms",
        t.elapsed().as_secs_f64() * 1e3
    );

    // 4. Kernel only (warmup + timed)
    let kernel = &gpu.kernel();
    let mut a_ptr: CUdeviceptr = a_dev.cu_deviceptr();
    let mut sa_val: u32 = sa as u32;
    let mut bt_ptr: CUdeviceptr = bt_dev.cu_deviceptr();
    let mut m_val: u32 = m_padded as u32;
    let mut k_val: u32 = k_padded as u32;
    let mut nlim_val: u32 = n_lim as u32;
    let mut c_ptr: CUdeviceptr = c_dev.cu_deviceptr();

    let mut params: [*mut c_void; 7] = [
        &mut a_ptr as *mut _ as *mut c_void,
        &mut sa_val as *mut _ as *mut c_void,
        &mut bt_ptr as *mut _ as *mut c_void,
        &mut m_val as *mut _ as *mut c_void,
        &mut k_val as *mut _ as *mut c_void,
        &mut nlim_val as *mut _ as *mut c_void,
        &mut c_ptr as *mut _ as *mut c_void,
    ];

    let grid_x = nlim_val;
    let grid_y = m_val / 64;

    // warmup
    unsafe {
        launch_kernel_on_stream(
            kernel,
            (grid_x, grid_y, 1),
            (128, 1, 1),
            0,
            &stream,
            &mut params,
        )?;
    }
    stream.synchronize()?;

    // timed
    let t = Instant::now();
    unsafe {
        launch_kernel_on_stream(
            kernel,
            (grid_x, grid_y, 1),
            (128, 1, 1),
            0,
            &stream,
            &mut params,
        )?;
    }
    stream.synchronize()?;
    let kernel_ms = t.elapsed().as_secs_f64() * 1e3;
    println!("  Kernel execution:   {:>8.1} ms", kernel_ms);

    // 5. D2H transfer
    let t = Instant::now();
    let _c_all = c_dev.to_host_vec(&stream)?;
    println!(
        "  D2H transfer:       {:>8.1} ms",
        t.elapsed().as_secs_f64() * 1e3
    );

    let bit_ops = 2.0 * (m as f64) * (n as f64) * (k as f64);
    println!(
        "\n  Kernel-only TOPS:   {:.1}",
        bit_ops / (kernel_ms / 1e3) / 1e12
    );
    println!("  H100 binary peak:   ~360,000 TOPS");
    println!(
        "  Utilization:        {:.2}%",
        bit_ops / (kernel_ms / 1e3) / 1e12 / 360_000.0 * 100.0
    );

    Ok(())
}

fn cm(row: usize, kl: usize) -> usize {
    (row / 8) * 32 + (kl / 2) * 16 + (row % 8) * 2 + (kl % 2)
}

fn transpose_b_host(b: &[u64], k: usize, n_lim: usize) -> Vec<u64> {
    let k_chunks = k / 256;
    let tile = 64 * 4usize;
    let mut out = vec![0u64; k_chunks * n_lim * tile];
    let mut buf = [0u64; 256];
    for kk in 0..k_chunks {
        for cl in 0..n_lim {
            let base = (kk * n_lim + cl) * tile;
            for i in 0..256usize {
                let br = kk * 256 + i;
                buf[i] = if br < k { b[br * n_lim + cl] } else { 0 };
            }
            for kl in 0..4usize {
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
