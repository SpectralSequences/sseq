//! [`multiply_batch`](super::batch::multiply_batch) on an NVIDIA GPU.
//!
//! Every (product, term) pair of a batch is one item of a persistent kernel, which enumerates the
//! terms of `P(R) P(S)` with the anti-diagonal walk of `antidiagonal.rs` and XORs each into its row
//! at the index the seqno tables give it (`multiply.cu`). The kernel needs no table of admissible
//! matrices and no state beyond the Milnor basis and the seqno tables, which are uploaded once and
//! extended when a batch reaches a higher degree.
//!
//! The kernel is compiled at runtime by NVRTC for the device it runs on, so any card the driver
//! supports works, and a binary built without a CUDA toolkit still runs; it needs `libnvrtc` at
//! runtime. `MILNOR_CUDA_DISABLE` turns the device off and `MILNOR_CUDA_DEVICE` (default 0) picks
//! it.
//!
//! Memory: a launch's output is split into blocks of whole rows of at most [`BLOCK_BYTES`], and
//! its per-item arrays into chunks of at most [`ARRAYS_BYTES`], all accumulating into the block's
//! output. Each block reserves its bytes from a per-device budget before allocating, so concurrent
//! callers wait for room instead of failing; a block that runs out of device memory anyway is
//! retried once with the device to itself.

use std::{
    borrow::Cow,
    ops::RangeInclusive,
    sync::{
        Arc, Condvar, Mutex, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
};

use anyhow::{Context, anyhow, bail};
use cudarc::{
    driver::{
        CudaContext, CudaFunction, CudaSlice, CudaStream, DriverError, LaunchConfig,
        PinnedHostSlice, PushKernelArg, sys,
    },
    nvrtc::{CompileError, CompileOptions, compile_ptx_with_opts},
};

use super::{
    MilnorAlgebra, PPart,
    batch::{BatchOutput, BatchProduct, LimbBlock, num_limbs},
};
use crate::algebra::Algebra;

const SRC: &str = include_str!("multiply.cu");

/// Walk widths the kernel is instantiated for: `multiply_antidiag_6` to `_10`.
const WIDTHS: RangeInclusive<usize> = 6..=PPart::MAX_LEN;

/// Threads per block. Measured on an H200 against 64 and 128.
const THREADS: u32 = 96;

/// Batches of at most this many terms run on the CPU. A launch costs ~0.1 ms however little it
/// carries, against ~0.15 ms for ~300 terms on one CPU thread (H200, stem-150 batches).
pub const CPU_TERMS: usize = 300;

/// Output bytes of one row block.
const BLOCK_BYTES: usize = 1 << 30;

/// Bytes of per-item arrays one kernel launch uploads.
const ARRAYS_BYTES: usize = 1 << 30;

/// Device memory the launch budget leaves to the tables, `fp` and anything else on the card.
const RESERVED_BYTES: usize = 6 << 30;

/// Kernel launches, process-wide. A test comparing the device with the CPU checks that this moved,
/// so that a batch which silently ran on the CPU cannot pass by agreeing with itself.
pub static LAUNCHES: AtomicU64 = AtomicU64::new(0);

/// Batches that failed on the device and were recomputed on the CPU, process-wide.
pub static FAILURES: AtomicU64 = AtomicU64::new(0);

/// The walk width of a product of `degree`: the number of $\xi_i$ that can occur, at least 6.
fn width_for_degree(degree: i32) -> usize {
    let mut n = 0;
    while n < PPart::MAX_LEN && (1i64 << (n + 1)) - 1 <= i64::from(degree) {
        n += 1;
    }
    n.max(*WIDTHS.start())
}

/// Shared memory of one block of the width-`n` kernel: the walk state (`STATE_BYTES`) and a
/// queue of 64 terms per warp (`QCAP`) with its count.
fn shared_bytes(n: usize) -> usize {
    let state = (n * (n - 1) + 2 * (n + 1)) * size_of::<u16>() * THREADS as usize;
    let warps = THREADS as usize / 32;
    state.next_multiple_of(16) + warps * (64 * 16 + 4)
}

/// Evaluate a batch on the device, or `None` if it should run on the CPU: no device, an algebra
/// the kernel does not support, or a batch too small to be worth a launch.
pub(super) fn multiply_batch(
    algebra: &MilnorAlgebra,
    num_cols: usize,
    col_map: Option<&[u32]>,
    num_rows: usize,
    products: &[BatchProduct],
) -> Option<anyhow::Result<BatchOutput>> {
    if !super::batch::batch_applicable(algebra) {
        return None;
    }
    let terms: usize = products.iter().map(|p| p.term_indices.len()).sum();
    if terms <= CPU_TERMS {
        return None;
    }
    let device = device()?;
    Some(device.run(algebra, num_cols, col_map, num_rows, products))
}

/// Whether a device was found (opening it on first use).
pub(super) fn available() -> bool {
    device().is_some()
}

/// Report a batch that failed on the device and is being recomputed on the CPU.
pub(super) fn report_failure(e: &anyhow::Error) {
    FAILURES.fetch_add(1, Ordering::Relaxed);
    eprintln!("[milnor_cuda] a batch failed on the device, computing it on the CPU: {e:#}");
}

/// The process's device, opened on first use. `None`, reported once, if there is none.
fn device() -> Option<&'static Device> {
    static DEVICE: OnceLock<Option<Device>> = OnceLock::new();
    DEVICE
        .get_or_init(|| {
            if std::env::var_os("MILNOR_CUDA_DISABLE").is_some() {
                return None;
            }
            let ordinal = match std::env::var("MILNOR_CUDA_DEVICE") {
                Ok(v) => v
                    .parse()
                    .unwrap_or_else(|_| panic!("MILNOR_CUDA_DEVICE={v:?} is not a device ordinal")),
                Err(_) => 0,
            };
            // cudarc panics, rather than returning an error, when the driver library is absent.
            match std::panic::catch_unwind(|| Device::open(ordinal)) {
                Ok(Ok(device)) => Some(device),
                Ok(Err(e)) => {
                    eprintln!("[milnor_cuda] no GPU multiply: {e:#}");
                    None
                }
                Err(_) => {
                    eprintln!("[milnor_cuda] no GPU multiply: the CUDA driver is not available");
                    None
                }
            }
        })
        .as_ref()
}

/// One width's kernel: the function, its shared memory, and the grid that fills the device.
struct Kernel {
    function: CudaFunction,
    shared_bytes: usize,
    grid: u32,
}

struct Device {
    ctx: Arc<CudaContext>,
    /// Indexed by width minus [`WIDTHS`]`.start()`.
    kernels: Vec<Kernel>,
    tables: Mutex<Option<Arc<Tables>>>,
    budget: Budget,
    pinned: PinnedPool,
}

impl Device {
    fn open(ordinal: usize) -> anyhow::Result<Self> {
        // SAFETY: only `dlopen`s the candidate library names. cudarc panics at the first missing
        // symbol, so this has to come before any other NVRTC call.
        if !unsafe { cudarc::nvrtc::sys::is_culib_present() } {
            bail!("libnvrtc was not found, so the kernel cannot be compiled");
        }
        let ctx = CudaContext::new(ordinal)?;
        let attribute = |a| ctx.attribute(a).map(|v| v as u32);
        let major =
            attribute(sys::CUdevice_attribute::CU_DEVICE_ATTRIBUTE_COMPUTE_CAPABILITY_MAJOR)?;
        let minor =
            attribute(sys::CUdevice_attribute::CU_DEVICE_ATTRIBUTE_COMPUTE_CAPABILITY_MINOR)?;
        let sm_count =
            attribute(sys::CUdevice_attribute::CU_DEVICE_ATTRIBUTE_MULTIPROCESSOR_COUNT)?;

        let brace = |values: Vec<u32>| {
            let values: Vec<String> = values.iter().map(u32::to_string).collect();
            format!("{{{}}}", values.join(","))
        };
        let shifts = (0..PPart::MAX_LEN).map(PPart::shift).collect();
        let masks = (0..PPart::MAX_LEN).map(PPart::max_entry).collect();
        let opts = CompileOptions {
            arch: Some(Box::leak(
                format!("compute_{major}{minor}").into_boxed_str(),
            )),
            name: Some("multiply.cu".to_owned()),
            options: vec![
                format!("-DPPART_MAX_LEN={}", PPart::MAX_LEN),
                format!("-DPP_SHIFTS={}", brace(shifts)),
                format!("-DPP_MASKS={}", brace(masks)),
                format!("-DNT={THREADS}"),
            ],
            ..Default::default()
        };
        let ptx = compile_ptx_with_opts(SRC, opts).map_err(|e| match &e {
            CompileError::CompileError { log, .. } => {
                anyhow!(
                    "NVRTC failed to compile multiply.cu:\n{}",
                    log.to_string_lossy()
                )
            }
            _ => anyhow!("NVRTC failed to compile multiply.cu: {e}"),
        })?;
        let module = ctx.load_module(ptx)?;

        let mut kernels = Vec::new();
        for n in WIDTHS {
            let name = format!("multiply_antidiag_{n}");
            let function = module.load_function(&name)?;
            let shared_bytes = shared_bytes(n);
            function.set_attribute(
                sys::CUfunction_attribute::CU_FUNC_ATTRIBUTE_MAX_DYNAMIC_SHARED_SIZE_BYTES,
                shared_bytes as i32,
            )?;
            let per_sm = function.occupancy_max_active_blocks_per_multiprocessor(
                THREADS,
                shared_bytes,
                None,
            )?;
            if per_sm == 0 {
                bail!("{name}: a block of {THREADS} threads does not fit on an SM");
            }
            kernels.push(Kernel {
                function,
                shared_bytes,
                grid: per_sm * sm_count,
            });
        }

        let total = ctx.total_mem()?;
        Ok(Self {
            budget: Budget::new(total.saturating_sub(RESERVED_BYTES).max(1 << 30)),
            pinned: PinnedPool::new(ctx.clone()),
            tables: Mutex::new(None),
            kernels,
            ctx,
        })
    }

    /// Device tables covering products of degree up to `max_degree` with right factors up to
    /// `max_s_degree`, uploading new ones if the current ones fall short.
    fn tables(
        &self,
        algebra: &MilnorAlgebra,
        max_degree: i32,
        max_s_degree: i32,
    ) -> anyhow::Result<Arc<Tables>> {
        let mut current = self.tables.lock().unwrap();
        if let Some(t) = &*current
            && t.seqno_degree >= max_degree
            && t.basis_degree >= max_s_degree
        {
            return Ok(t.clone());
        }
        let tables = Arc::new(Tables::upload(&self.ctx.default_stream(), algebra)?);
        if tables.seqno_degree < max_degree || tables.basis_degree < max_s_degree {
            bail!(
                "the seqno tables reach degree {}, but the batch needs {max_degree}",
                tables.seqno_degree
            );
        }
        *current = Some(tables.clone());
        Ok(tables)
    }

    fn run(
        &'static self,
        algebra: &MilnorAlgebra,
        num_cols: usize,
        col_map: Option<&[u32]>,
        num_rows: usize,
        products: &[BatchProduct],
    ) -> anyhow::Result<BatchOutput> {
        let num_limbs = num_limbs(num_cols);
        let products: Cow<[BatchProduct]> = if products.is_sorted_by_key(|p| p.row) {
            Cow::Borrowed(products)
        } else {
            let mut v = products.to_vec();
            v.sort_by_key(|p| p.row);
            Cow::Owned(v)
        };
        let products = &*products;

        // The products that contribute, and each one's R, packed. Products arrive grouped by R, so
        // the previous one is usually reused.
        let live: Vec<bool> = products
            .iter()
            .map(|p| algebra.dimension(p.r_degree + p.s_degree) > 0 && !p.term_indices.is_empty())
            .collect();
        let mut r_bits = Vec::with_capacity(products.len());
        let mut last: Option<((i32, usize), u64)> = None;
        for p in products {
            let key = (p.r_degree, p.r_idx);
            let bits = match last {
                Some((k, b)) if k == key => b,
                _ => algebra.basis_element_from_index(key.0, key.1).p_part.bits(),
            };
            last = Some((key, bits));
            r_bits.push(bits);
        }
        let (max_degree, max_s_degree) = products
            .iter()
            .zip(&live)
            .filter(|(_, l)| **l)
            .fold((0, 0), |(o, s), (p, _)| {
                (o.max(p.r_degree + p.s_degree), s.max(p.s_degree))
            });
        let tables = self.tables(algebra, max_degree, max_s_degree)?;

        let stream = self.ctx.per_thread_stream();
        let col_map_dev = stream
            .clone_htod(col_map.unwrap_or(&[0]))
            .context("uploading the column map")?;
        let launch = Launch {
            device: self,
            tables: &tables,
            stream: &stream,
            products,
            r_bits: &r_bits,
            col_map: &col_map_dev,
            col_map_len: col_map.map_or(0, |m| m.len() as u32),
            use_col_map: u32::from(col_map.is_some()),
            num_limbs,
        };

        let mut blocks: Vec<Box<dyn LimbBlock>> = Vec::new();
        for (row0, row1, p0, p1) in row_blocks(products, num_rows, num_limbs) {
            let block_len = (row1 - row0) * num_limbs;
            // The live products of the block, by walk width, then cut into chunks of arrays.
            let mut groups: Vec<Vec<usize>> = WIDTHS.map(|_| Vec::new()).collect();
            for i in (p0..p1).filter(|&i| live[i]) {
                let p = &products[i];
                groups[width_for_degree(p.r_degree + p.s_degree) - WIDTHS.start()].push(i);
            }
            let chunks: Vec<Vec<std::ops::Range<usize>>> = groups
                .iter()
                .map(|g| array_chunks(g.iter().map(|&i| products[i].term_indices.len())))
                .collect();
            let max_chunk = chunks
                .iter()
                .zip(&groups)
                .flat_map(|(cs, g)| cs.iter().map(|c| launch.chunk_bytes(&g[c.clone()])))
                .max()
                .unwrap_or(0);
            let need =
                block_len * size_of::<u32>() + max_chunk + size_of_val(col_map.unwrap_or(&[]));

            let result = {
                let _permit = self.budget.acquire(need);
                launch.block(row0, block_len, &groups, &chunks)
            };
            let result = match result {
                Err(e) if is_out_of_memory(&e) => {
                    eprintln!(
                        "[milnor_cuda] out of device memory on a {:.2} GB block; retrying with \
                         the device to itself",
                        need as f64 / 1e9
                    );
                    let _ = stream.synchronize();
                    let _permit = self.budget.acquire_all();
                    trim_memory_pool(&self.ctx);
                    launch.block(row0, block_len, &groups, &chunks)
                }
                r => r,
            };
            blocks.push(Box::new(result?));
        }
        Ok(BatchOutput::from_blocks(blocks, num_limbs))
    }
}

/// The Milnor basis and the seqno tables, on the device.
struct Tables {
    /// The seqno tables cover products of degree at most this.
    seqno_degree: i32,
    /// The basis covers right factors of degree at most this.
    basis_degree: i32,
    /// Index into `pp` of element 0 of each degree.
    basis_start: Vec<u32>,
    /// Every basis element through `basis_degree`, as its packed p-part.
    pp: CudaSlice<u64>,
    /// `SeqnoTables::g` and `xi`, padded to [`PPart::MAX_LEN`] entries.
    g: CudaSlice<u32>,
    xi: CudaSlice<u32>,
    width: u32,
}

impl Tables {
    /// Upload the tables, through the degree the algebra's seqno tables reach.
    fn upload(stream: &Arc<CudaStream>, algebra: &MilnorAlgebra) -> anyhow::Result<Self> {
        let seqno = algebra.seqno_tables();
        let degree = seqno.max_degree();
        let mut basis_start = Vec::with_capacity(degree as usize + 1);
        let mut pp = Vec::new();
        for d in 0..=degree {
            basis_start.push(u32::try_from(pp.len()).context("the basis has 2^32 elements")?);
            pp.extend(algebra.ppart_table(d).iter().map(|p| p.bits()));
        }
        let g: Vec<u32> = seqno
            .g()
            .iter()
            .map(|&v| u32::try_from(v))
            .collect::<Result<_, _>>()
            .context("a seqno table entry exceeds u32")?;
        let mut xi: Vec<u32> = seqno.xi().iter().map(|&x| x as u32).collect();
        xi.resize(xi.len().max(PPart::MAX_LEN), 0);
        let tables = Self {
            seqno_degree: degree,
            basis_degree: degree,
            basis_start,
            pp: stream.clone_htod(&pp)?,
            g: stream.clone_htod(&g)?,
            xi: stream.clone_htod(&xi)?,
            width: seqno.width() as u32,
        };
        stream.synchronize()?;
        Ok(tables)
    }
}

/// What one batch shares across its blocks.
struct Launch<'a> {
    device: &'static Device,
    tables: &'a Tables,
    stream: &'a Arc<CudaStream>,
    products: &'a [BatchProduct],
    r_bits: &'a [u64],
    col_map: &'a CudaSlice<u32>,
    col_map_len: u32,
    use_col_map: u32,
    num_limbs: usize,
}

/// The kernel's per-item and per-product arrays for one chunk of products.
struct Arrays {
    /// Per item: the term's index into the basis table, and its product.
    term_gei: Vec<u32>,
    term_prod: Vec<u32>,
    /// Per product: R, the first limb of its row in the block, and its column offset.
    prod_r: Vec<u64>,
    prod_row_base: Vec<u32>,
    prod_out_offset: Vec<u32>,
}

impl Launch<'_> {
    /// Device bytes of the arrays of `chunk`.
    fn chunk_bytes(&self, chunk: &[usize]) -> usize {
        chunk
            .iter()
            .map(|&i| 8 * self.products[i].term_indices.len() + 16)
            .sum()
    }

    fn arrays(&self, chunk: &[usize], row0: usize) -> anyhow::Result<Arrays> {
        let terms = chunk
            .iter()
            .map(|&i| self.products[i].term_indices.len())
            .sum();
        let mut a = Arrays {
            term_gei: Vec::with_capacity(terms),
            term_prod: Vec::with_capacity(terms),
            prod_r: Vec::with_capacity(chunk.len()),
            prod_row_base: Vec::with_capacity(chunk.len()),
            prod_out_offset: Vec::with_capacity(chunk.len()),
        };
        for &i in chunk {
            let p = &self.products[i];
            let local = a.prod_r.len() as u32;
            let base = self.tables.basis_start[p.s_degree as usize];
            a.term_gei
                .extend(p.term_indices.iter().map(|&t| base + t as u32));
            a.term_prod
                .extend(std::iter::repeat_n(local, p.term_indices.len()));
            a.prod_r.push(self.r_bits[i]);
            a.prod_row_base
                .push(((p.row - row0) * self.num_limbs) as u32);
            a.prod_out_offset
                .push(u32::try_from(p.out_offset).context("a column offset exceeds u32")?);
        }
        Ok(a)
    }

    /// Run the products of one row block, starting at row `row0` and `len` limbs long, and read
    /// its output back.
    fn block(
        &self,
        row0: usize,
        len: usize,
        groups: &[Vec<usize>],
        chunks: &[Vec<std::ops::Range<usize>>],
    ) -> anyhow::Result<Pinned> {
        let stream = self.stream;
        let t = self.tables;
        let mut out = stream
            .alloc_zeros::<u32>(len.max(1))
            .context("allocating the output")?;
        let mut counter = stream
            .alloc_zeros::<u32>(1)
            .context("allocating the counter")?;
        let num_limbs = self.num_limbs as u32;
        let out_len = len as u64;
        for (gi, (group, chunks)) in groups.iter().zip(chunks).enumerate() {
            let kernel = &self.device.kernels[gi];
            for chunk in chunks {
                let a = self.arrays(&group[chunk.clone()], row0)?;
                let num_items = a.term_gei.len() as u32;
                if num_items == 0 {
                    continue;
                }
                let upload = "uploading the arrays";
                let term_gei = stream.clone_htod(&a.term_gei).context(upload)?;
                let term_prod = stream.clone_htod(&a.term_prod).context(upload)?;
                let prod_r = stream.clone_htod(&a.prod_r).context(upload)?;
                let prod_row_base = stream.clone_htod(&a.prod_row_base).context(upload)?;
                let prod_out_offset = stream.clone_htod(&a.prod_out_offset).context(upload)?;
                stream.memset_zeros(&mut counter).context(upload)?;
                let cfg = LaunchConfig {
                    grid_dim: (kernel.grid.min(num_items.div_ceil(THREADS)), 1, 1),
                    block_dim: (THREADS, 1, 1),
                    shared_mem_bytes: kernel.shared_bytes as u32,
                };
                let mut builder = stream.launch_builder(&kernel.function);
                builder
                    .arg(&t.pp)
                    .arg(&term_gei)
                    .arg(&term_prod)
                    .arg(&prod_r)
                    .arg(&prod_row_base)
                    .arg(&prod_out_offset)
                    .arg(&t.g)
                    .arg(&t.xi)
                    .arg(&mut out)
                    .arg(self.col_map)
                    .arg(&self.col_map_len)
                    .arg(&self.use_col_map)
                    .arg(&num_items)
                    .arg(&mut counter)
                    .arg(&t.width)
                    .arg(&num_limbs)
                    .arg(&out_len);
                // SAFETY: the arguments match the kernel's parameters in order and type, and every
                // array is as long as the kernel indexes it: items and products by construction,
                // `out` checked against `out_len` and `num_limbs`, `pp` by the basis covering
                // every term's degree, `g` and `xi` by the seqno tables covering every product's.
                unsafe { builder.launch(cfg) }
                    .with_context(|| format!("launching {:?}", cfg.grid_dim))?;
                LAUNCHES.fetch_add(1, Ordering::Relaxed);
            }
        }
        let mut host = self
            .device
            .pinned
            .take(len)
            .context("allocating page-locked memory")?;
        // Exactly `len` limbs: the copy takes its size from the destination, and a pooled
        // buffer may be longer than this block.
        let dst = &mut host.buf.as_mut().unwrap().0.as_mut_slice()?[..len];
        stream
            .memcpy_dtoh(&out.slice(..len), dst)
            .context("reading the output back")?;
        stream.synchronize().context("completing the block")?;
        Ok(host)
    }
}

/// Split the products, sorted by row, into blocks of whole rows whose output fits
/// [`BLOCK_BYTES`]: `(row0, row1, p0, p1)` per block, the products of rows `row0..row1` being
/// `p0..p1`. A single row larger than that is a block of its own.
fn row_blocks(
    products: &[BatchProduct],
    num_rows: usize,
    num_limbs: usize,
) -> Vec<(usize, usize, usize, usize)> {
    let max_rows = (BLOCK_BYTES / (num_limbs * size_of::<u32>())).max(1);
    let mut blocks = Vec::new();
    let (mut row0, mut p0) = (0, 0);
    for (i, p) in products.iter().enumerate() {
        if p.row >= row0 + max_rows {
            blocks.push((row0, p.row, p0, i));
            (row0, p0) = (p.row, i);
        }
    }
    blocks.push((row0, num_rows, p0, products.len()));
    blocks
}

/// Cut products with the given term counts into consecutive chunks whose arrays fit
/// [`ARRAYS_BYTES`] and whose items fit a `u32`, at least one product each.
fn array_chunks(terms: impl Iterator<Item = usize>) -> Vec<std::ops::Range<usize>> {
    let mut chunks = Vec::new();
    let (mut start, mut bytes, mut items, mut n) = (0, 0, 0, 0);
    for t in terms {
        let b = 8 * t + 16;
        if n > start && (bytes + b > ARRAYS_BYTES || items + t > u32::MAX as usize) {
            chunks.push(start..n);
            (start, bytes, items) = (n, 0, 0);
        }
        bytes += b;
        items += t;
        n += 1;
    }
    if n > start {
        chunks.push(start..n);
    }
    chunks
}

fn is_out_of_memory(e: &anyhow::Error) -> bool {
    e.chain().any(|c| {
        c.downcast_ref::<DriverError>()
            .is_some_and(|d| d.0 == sys::CUresult::CUDA_ERROR_OUT_OF_MEMORY)
    })
}

/// Return the memory the stream-ordered allocator keeps cached to the device.
fn trim_memory_pool(ctx: &Arc<CudaContext>) {
    let mut pool: sys::CUmemoryPool = std::ptr::null_mut();
    let _ = ctx.bind_to_thread();
    // SAFETY: an out-pointer to a local and the context's own device; trimming releases only
    // memory no allocation holds.
    unsafe {
        if sys::cuDeviceGetDefaultMemPool(&mut pool, ctx.cu_device()) == sys::CUresult::CUDA_SUCCESS
        {
            sys::cuMemPoolTrimTo(pool, 0);
        }
    }
}

/// Device memory the launches in flight may hold together. A block takes its bytes before
/// allocating, so callers wait for room rather than failing; a request larger than the whole
/// budget is clamped to it and runs alone.
struct Budget {
    cap: usize,
    avail: Mutex<usize>,
    /// Callers waiting for the whole budget. Ordinary requests wait while there are any, so the
    /// budget drains for them instead of being refilled by smaller requests.
    draining: AtomicU64,
    freed: Condvar,
}

struct Permit<'a> {
    budget: &'a Budget,
    bytes: usize,
}

impl Drop for Permit<'_> {
    fn drop(&mut self) {
        *self.budget.avail.lock().unwrap() += self.bytes;
        self.budget.freed.notify_all();
    }
}

impl Budget {
    fn new(cap: usize) -> Self {
        Self {
            cap,
            avail: Mutex::new(cap),
            draining: AtomicU64::new(0),
            freed: Condvar::new(),
        }
    }

    fn acquire(&self, bytes: usize) -> Permit<'_> {
        let bytes = bytes.min(self.cap);
        let mut avail = self.avail.lock().unwrap();
        while *avail < bytes || self.draining.load(Ordering::Relaxed) > 0 {
            avail = self.freed.wait(avail).unwrap();
        }
        *avail -= bytes;
        Permit {
            budget: self,
            bytes,
        }
    }

    /// The whole budget, once every other holder has let go.
    fn acquire_all(&self) -> Permit<'_> {
        let mut avail = self.avail.lock().unwrap();
        self.draining.fetch_add(1, Ordering::Relaxed);
        while *avail < self.cap {
            avail = self.freed.wait(avail).unwrap();
        }
        *avail = 0;
        self.draining.fetch_sub(1, Ordering::Relaxed);
        self.freed.notify_all();
        Permit {
            budget: self,
            bytes: self.cap,
        }
    }
}

/// Page-locked host buffers for reading outputs back, reused because locking pages costs about
/// ten times the copy itself (69 ms against 7 ms for a 338 MB output). At most
/// [`PinnedPool::MAX`] are kept, the smallest being freed first.
struct PinnedPool {
    ctx: Arc<CudaContext>,
    free: Mutex<Vec<PinnedSlice>>,
}

/// A pooled buffer: `PinnedHostSlice` behind a newtype, so the pool can hold it.
struct PinnedSlice(PinnedHostSlice<u32>);

impl PinnedSlice {
    fn len(&self) -> usize {
        cudarc::driver::HostSlice::len(&self.0)
    }
}

/// An output block in a pooled buffer, returned to the pool when the output is dropped.
struct Pinned {
    buf: Option<PinnedSlice>,
    len: usize,
    pool: &'static PinnedPool,
}

impl PinnedPool {
    const MAX: usize = 4;

    fn new(ctx: Arc<CudaContext>) -> Self {
        Self {
            ctx,
            free: Mutex::new(Vec::new()),
        }
    }

    fn take(&'static self, len: usize) -> anyhow::Result<Pinned> {
        let reused = {
            let mut free = self.free.lock().unwrap();
            free.iter()
                .position(|b| b.len() >= len)
                .map(|i| free.swap_remove(i))
        };
        let buf = match reused {
            Some(b) => b,
            // SAFETY: the memory is uninitialized until the device writes the output into it,
            // which happens before anything reads it. Flags 0: cached, because the host reads it.
            None => PinnedSlice(unsafe { self.ctx.alloc_pinned_with_flags(len.max(1), 0) }?),
        };
        Ok(Pinned {
            buf: Some(buf),
            len,
            pool: self,
        })
    }

    fn put(&self, buf: PinnedSlice) {
        let mut free = self.free.lock().unwrap();
        free.push(buf);
        if free.len() > Self::MAX {
            let smallest = (0..free.len()).min_by_key(|&i| free[i].len()).unwrap();
            free.swap_remove(smallest);
        }
    }
}

impl Drop for Pinned {
    fn drop(&mut self) {
        if let Some(buf) = self.buf.take() {
            self.pool.put(buf);
        }
    }
}

impl LimbBlock for Pinned {
    fn limbs(&self) -> &[u32] {
        let buf = self.buf.as_ref().unwrap();
        // The copy into it completed before the block was returned (`Launch::block` synchronizes),
        // so this does not wait.
        &buf.0.as_slice().expect("pinned output buffer")[..self.len]
    }
}

#[cfg(test)]
mod tests {
    use super::{super::batch::tests::*, *};
    use crate::algebra::milnor_algebra::batch::{COL_MAP_DROP, cpu_multiply_batch};

    fn device_or_fail() -> &'static Device {
        device().expect("these tests need a CUDA device")
    }

    /// Run on the device and compare with the CPU, checking the device was used.
    fn check(
        algebra: &MilnorAlgebra,
        num_cols: usize,
        col_map: Option<&[u32]>,
        num_rows: usize,
        products: &[BatchProduct],
    ) {
        let before = LAUNCHES.load(Ordering::Relaxed);
        let got = device_or_fail()
            .run(algebra, num_cols, col_map, num_rows, products)
            .unwrap();
        assert!(
            LAUNCHES.load(Ordering::Relaxed) > before,
            "nothing was launched"
        );
        let want = cpu_multiply_batch(algebra, num_cols, col_map, num_rows, products);
        assert_eq!(got, want);
    }

    #[test]
    fn device_matches_cpu() {
        let max_degree = 300;
        let algebra = algebra(max_degree);
        let mut rng = TestRng::new(7);
        for (degree, rows, blocks) in [(40, 50, 4), (130, 40, 3), (300, 12, 2)] {
            let (products, width) = random_batch(&algebra, &mut rng, degree, rows, blocks);
            check(&algebra, width, None, rows, &products);
        }
    }

    #[test]
    fn device_matches_cpu_col_map() {
        let max_degree = 160;
        let algebra = algebra(max_degree);
        let mut rng = TestRng::new(8);
        let (products, width) = random_batch(&algebra, &mut rng, max_degree, 30, 3);
        let kept: Vec<usize> = (0..width).filter(|_| rng.chance(0.2)).collect();
        let mut map = vec![COL_MAP_DROP; width];
        for (j, &c) in kept.iter().enumerate() {
            map[c] = j as u32;
        }
        check(&algebra, kept.len(), Some(&map), 30, &products);
    }

    /// A prefix narrower than the products' blocks keeps only its columns.
    #[test]
    fn device_matches_cpu_prefix() {
        let max_degree = 120;
        let algebra = algebra(max_degree);
        let mut rng = TestRng::new(9);
        let (products, width) = random_batch(&algebra, &mut rng, max_degree, 20, 3);
        check(&algebra, width / 3 + 5, None, 20, &products);
    }

    /// A smaller batch after a larger one reads back into the larger one's pooled buffer.
    #[test]
    fn device_reuses_a_larger_buffer() {
        let algebra = algebra(100);
        let mut rng = TestRng::new(11);
        for rows in [60, 20, 5] {
            let (products, width) = random_batch(&algebra, &mut rng, 100, rows, 3);
            check(&algebra, width, None, rows, &products);
        }
    }

    #[test]
    fn row_blocks_partition() {
        let algebra = algebra(60);
        let mut rng = TestRng::new(10);
        let (products, _) = random_batch(&algebra, &mut rng, 60, 40, 2);
        // An absurdly wide row makes every row a block of its own.
        let wide = BLOCK_BYTES / 4;
        let blocks = row_blocks(&products, 40, wide);
        assert_eq!(blocks.len(), 40);
        let mut next = (0, 0);
        for &(r0, r1, p0, p1) in &blocks {
            assert_eq!((r0, p0), next);
            assert!(products[p0..p1].iter().all(|p| (r0..r1).contains(&p.row)));
            next = (r1, p1);
        }
        assert_eq!(next, (40, products.len()));
    }

    #[test]
    fn array_chunks_respect_the_budget() {
        // Each of the first four takes a third of the budget: two fit, a third does not.
        let per = ARRAYS_BYTES / 8 / 3;
        let chunks = array_chunks([per, per, per, per, 1, 1].into_iter());
        assert_eq!(chunks, vec![0..2, 2..6]);
        // One product larger than the budget still runs, alone.
        let chunks = array_chunks([ARRAYS_BYTES, 1].into_iter());
        assert_eq!(chunks, vec![0..1, 1..2]);
    }
}
