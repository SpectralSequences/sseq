//! Host side of the device row reduction to reduced row echelon form.

use std::sync::{Arc, OnceLock};

use anyhow::{anyhow, bail};
use cudarc::{
    driver::{
        CudaContext, CudaFunction, CudaModule, CudaSlice, DevicePtr, DevicePtrMut, LaunchConfig,
        PushKernelArg,
    },
    nvrtc::{CompileError, CompileOptions, Ptx, compile_ptx_with_opts},
};

use super::{
    kernel::{
        ARCH, DeviceMatrix, GemmTarget, GpuContext, NG, TILE_M, cfg_1d, device_stride,
        note_product_fallback, tma_xor_supported,
    },
    params::GROUP_M,
};

/// The reduction's CUDA C++ source, compiled to PTX at runtime by NVRTC — see
/// [`compile_row_reduce`].
static ROW_REDUCE_SRC: &str = include_str!("row_reduce.cu");

/// Compile the row-reduction kernels to PTX with NVRTC.
///
/// A program of its own, separate from the GEMM's: the two share no device code and no knobs, so
/// it takes no defines. Needs `libnvrtc`, but no GPU.
pub fn compile_row_reduce() -> anyhow::Result<Ptx> {
    // SAFETY: as in `compile_kernel`: the probe only `dlopen`s the candidate library names, and has
    // to come before any other nvrtc call because cudarc panics on a missing symbol.
    if !unsafe { cudarc::nvrtc::sys::is_culib_present() } {
        bail!("libnvrtc was not found, so row_reduce.cu cannot be compiled");
    }
    let opts = CompileOptions {
        arch: Some(ARCH),
        name: Some("row_reduce.cu".to_string()),
        ..Default::default()
    };
    compile_ptx_with_opts(ROW_REDUCE_SRC, opts).map_err(|e| match &e {
        CompileError::CompileError { log, .. } => anyhow!(
            "NVRTC failed to compile row_reduce.cu for {ARCH}:\n{}",
            log.to_string_lossy()
        ),
        _ => anyhow!("NVRTC failed to compile row_reduce.cu: {e}"),
    })
}

/// The compiled reduction, compiled once per process and reused.
fn row_reduce_ptx() -> anyhow::Result<Ptx> {
    static PTX: OnceLock<Ptx> = OnceLock::new();
    if let Some(ptx) = PTX.get() {
        return Ok(ptx.clone());
    }
    let ptx = compile_row_reduce()?;
    Ok(PTX.get_or_init(|| ptx).clone())
}

/// The kernels of `row_reduce.cu`, loaded onto one context.
pub(super) struct RrKernels {
    /// Held so the loaded module outlives the [`CudaFunction`]s taken from it; never read.
    #[allow(dead_code)]
    module: Arc<CudaModule>,
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

impl RrKernels {
    /// Compile (once per process) and load the reduction onto `ctx`.
    pub(super) fn load(ctx: &Arc<CudaContext>) -> anyhow::Result<Self> {
        let module = ctx.load_module(row_reduce_ptx()?)?;
        let f = |name: &str| module.load_function(name);
        Ok(Self {
            panel_factor: f("panel_factor")?,
            panel_factor_coop: f("panel_factor_coop")?,
            pf_find_swap: f("pf_find_swap")?,
            pf_step: f("pf_step")?,
            pf_xor: f("pf_xor")?,
            mark_live: f("mark_live")?,
            promote_pivots: f("promote_pivots")?,
            promote_coop: f("promote_coop")?,
            zero_pivot_l: f("zero_pivot_l")?,
            gather_rows: f("gather_rows")?,
            block_reduce_rref: f("block_reduce_rref")?,
            block_reduce_coop: f("block_reduce_coop")?,
            br_cond: f("br_cond")?,
            br_xor: f("br_xor")?,
            gather_cols: f("gather_cols")?,
            xor_into_perm: f("xor_into_perm")?,
            module,
        })
    }
}

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
/// single-CTA `promote_pivots` but ~bl-independent for the cooperative `promote_coop`. So the
/// divisor follows [`promote_is_coop`], the same condition the reduction selects the promote by,
/// and not the stride alone: [`rr_coop`] is off by default, and keying on stride made every large
/// reduction pick the wide cooperative panel while paying the O(bl) single-CTA promotion for it.
///
/// See EXPERIMENTS.md for the measured optima. Override with `FP_CUDA_BL`.
fn adaptive_bl(stride: usize) -> usize {
    let div = if promote_is_coop(stride) { 128 } else { 256 };
    (stride / div).clamp(1, MAX_BL)
}

/// Whether the reduction promotes pivots with the cooperative `promote_coop` kernel.
///
/// Below this stride the grid barrier's fixed cost outweighs the single-CTA promotion it replaces.
fn promote_is_coop(stride: usize) -> bool {
    rr_coop() && stride >= 1024
}

/// Whether the row reduction uses its **cooperative** kernels.
///
/// Those are `panel_factor_coop`, `promote_coop` and `block_reduce_coop`, launched with
/// `cuLaunchCooperativeKernel` and synchronized by a hand-rolled grid-wide spin barrier.
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

impl GpuContext {
    pub fn identity_perm(&self, m: usize) -> anyhow::Result<CudaSlice<u32>> {
        let host: Vec<u32> = (0..m as u32).collect();
        Ok(self.stream().clone_htod(&host)?)
    }

    /// Factor one 64-bit column panel in place over the persistent buffer — the b=64 base kernel.
    ///
    /// Forward-only over limb `plimb`, starting from pivot row `r`. Rows are addressed through
    /// `perm`; a pivot is promoted by swapping its `perm` entry to position `r + pivot_index`. The
    /// multiplier bits captured while clearing rows *below* each pivot are ORed into `l` (indexed
    /// by original row id, so `l` needs no permutation).
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
        let mut lb = stream.launch_builder(&self.rr.panel_factor);
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
        let sms = self.sm_count()?;
        let occ = self
            .rr
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
        let mut lb = stream.launch_builder(&self.rr.panel_factor_coop);
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
        let sms = self.sm_count()?;
        let occ = self
            .rr
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
            let mut lb = stream.launch_builder(&self.rr.pf_find_swap);
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
            let mut lb = stream.launch_builder(&self.rr.pf_step);
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
            let mut lb = stream.launch_builder(&self.rr.pf_xor);
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
            let mut lb = stream.launch_builder(&self.rr.mark_live);
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
        let mut lb = stream.launch_builder(&self.rr.promote_coop);
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
        let (rows, stride) = (m.rows, m.stride);
        let trailing_limbs = end_limb - first_limb;
        if pr == 0 || trailing_limbs == 0 {
            return Ok(());
        }

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
                let mut lb = stream.launch_builder(&self.rr.promote_pivots);
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
            let mut lb = stream.launch_builder(&self.rr.zero_pivot_l);
            lb.arg(perm).arg(&mut l.buf).arg(&r_u).arg(&pr_u).arg(&ls);
            // SAFETY: launch contract for `zero_pivot_l`.
            unsafe { lb.launch(cfg_1d(pr)) }?;
        }

        // (3) gather U = pivot rows' trailing (pr × trailing_limbs), the padding limbs included:
        // they are zero columns. SAFETY: uninitialized is sound because `gather_rows` below writes
        // all `pr * trailing_limbs` elements before the GEMM reads them.
        let u_buf = unsafe { stream.alloc::<u64>(pr * trailing_limbs) }?;
        {
            let (r_u, fl, pr_u, nc, st) = (
                r_piv as u32,
                first_limb as u32,
                pr as u32,
                trailing_limbs as u32,
                stride as u32,
            );
            let mut lb = stream.launch_builder(&self.rr.gather_rows);
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

        // (4) GEMM L(m×pr)·U(pr×trailing); M[:, first_limb:] ^= the product, XORed into M by the
        // GEMM itself, which M's stride invariant allows. Every box of that reduce store must start
        // on an even limb, so an odd first limb starts the product one limb early, against a zero
        // leading limb of U: that product limb is L·0 = 0.
        if tma_xor_supported(stride) {
            let lead = first_limb % 2;
            let (a_int, m_padded) =
                self.pack_a_dev(&stream, &l.buf.slice(..), rows, pr, l.stride)?;
            let pb = self.pack_b_dev(
                &stream,
                &u_buf.slice(..),
                pr,
                trailing_limbs,
                trailing_limbs,
                lead,
            )?;
            drop(u_buf);
            let (dst, _gd) = m.buf.device_ptr_mut(&stream);
            self.gemm_packed(
                &stream,
                &a_int,
                m_padded,
                &pb,
                GemmTarget::TmaXor {
                    dst,
                    dst_rows: rows,
                    dst_stride: stride,
                    x_limb: first_limb - lead,
                },
            )?;
        } else {
            note_product_fallback(rows, stride);
            let (c_dev, n_padded_lim) = self.matmul_b1_dev_strided(
                &l.buf,
                rows,
                pr,
                l.stride,
                &u_buf,
                trailing_limbs * 64,
            )?;
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
        }
        Ok(())
    }

    /// Forward pass of the blocked row reduction over the persistent device buffer.
    ///
    /// Sweeps 64-bit panels left to right, and for each — factor it
    /// ([`panel_factor`](Self::panel_factor)), promote the pivot rows' deferred trailing, drop the
    /// pivots from the multiplier matrix, and apply the trailing update `M[:, c+b:] ^= L·U` as one
    /// wgmma GEMM. Leaves `m` in
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
        let use_promote_coop = promote_is_coop(stride);
        let (pc_ctas, mut pc_barrier, pc_cond) = if use_promote_coop {
            let sms = self.sm_count()?;
            let occ = self
                .rr
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

        // Panels cover the real columns only: the stride's padding limbs are zero columns, which
        // can hold no pivot, though the trailing updates carry them along.
        let width = m.cols.div_ceil(64);
        let mut ppanel = 0usize;
        let mut panel_idx = 0usize;
        while ppanel < width {
            let bl_eff = bl.min(width - ppanel);
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
            let l_stride = device_stride(bl_eff * 64);
            let mut l = DeviceMatrix::from_parts(
                stream.alloc_zeros::<u64>(rows * l_stride)?,
                rows,
                bl_eff * 64,
                l_stride,
            );
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
                // Past the last real limb the trailing region is padding alone: nothing to update.
                let first = ppanel + bl_eff;
                self.trailing_update(
                    m,
                    &perm,
                    &mut l,
                    r - pr,
                    pr,
                    first,
                    if first < width { stride } else { first },
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

    /// Clear a pivot block's columns from the rows *above* it, as one `X·U` GEMM.
    ///
    /// The back-substitution Schur update. Clears the pivot columns of block
    /// `[block_s, block_e)` from the rows at perm positions
    /// `[above_start, above_start+above_count)`: gather `X` = those rows' bits at the block's pivot
    /// columns, `U` = the (already-RREF) block rows over their trailing, `G = X·U`, then
    /// scatter-XOR `G` into the above rows.
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
        slab: usize,
    ) -> anyhow::Result<()> {
        if above_count == 0 || block_e <= block_s {
            return Ok(());
        }
        let stream = self.stream();
        let stride = m.stride;
        let bp_eff = block_e - block_s;
        let start_limb = pivot_cols[block_s] / 64;
        let trailing_limbs = stride - start_limb;
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
            let mut lb = stream.launch_builder(&self.rr.gather_cols);
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
            let mut lb = stream.launch_builder(&self.rr.gather_rows);
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

        // G = X · U (above_count × trailing limbs), XORed into the above rows. Those rows are permuted,
        // which a TMA tile cannot address, so G goes through a product buffer: one row slab of it
        // at a time ([`bs_slab_rows`]), with U packed once for all of them.
        let pb = self.pack_b_dev(
            &stream,
            &u_buf.slice(..),
            bp_eff,
            trailing_limbs,
            trailing_limbs,
            0,
        )?;
        drop(u_buf);
        let slab = slab.min(above_count);
        // SAFETY: uninitialized is sound because each slab's GEMM stores every tile of the rows
        // `xor_into_perm` then reads, before it reads them.
        let c_dev =
            unsafe { stream.alloc::<u64>(slab.next_multiple_of(TILE_M) * pb.n_padded_lim) }?;
        let mut a0 = 0;
        while a0 < above_count {
            let rows_here = slab.min(above_count - a0);
            let x_view = x_buf.slice(a0 * x_stride..(a0 + rows_here) * x_stride);
            let (a_int, m_padded) =
                self.pack_a_dev(&stream, &x_view, rows_here, bp_eff, x_stride)?;
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
            let (s_u, w, st, fl, cs) = (
                rows_here as u32,
                trailing_limbs as u32,
                stride as u32,
                start_limb as u32,
                pb.n_padded_lim as u32,
            );
            let perm_slab = perm_above.slice(a0..a0 + rows_here);
            let mut lb = stream.launch_builder(&self.rr.xor_into_perm);
            lb.arg(&mut m.buf)
                .arg(&c_dev)
                .arg(&perm_slab)
                .arg(&s_u)
                .arg(&w)
                .arg(&st)
                .arg(&fl)
                .arg(&cs);
            // SAFETY: launch contract for `xor_into_perm`.
            unsafe { lb.launch(cfg_1d(rows_here * trailing_limbs)) }?;
            a0 += rows_here;
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
                let mut lb = stream.launch_builder(&self.rr.block_reduce_coop);
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
                        let mut lb = stream.launch_builder(&self.rr.br_cond);
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
                        let mut lb = stream.launch_builder(&self.rr.br_xor);
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
                let mut lb = stream.launch_builder(&self.rr.block_reduce_rref);
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

    /// Recursive **blocked TRSM** reduction of a pivot block `[s, e)` to RREF among itself.
    ///
    /// Splits at the midpoint, recurses on the right half, clears its pivots from the left half
    /// with one large `X·U` GEMM ([`bs_clear_above`](Self::bs_clear_above)), then recurses on the
    /// left half.
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
        slab: usize,
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
            br_ctas, slab,
        )?;
        self.bs_clear_above(m, perm, piv_dev, pivot_cols, mid, e, s, mid - s, slab)?;
        // Left half to RREF (its rows now carry no right-half pivot bits).
        self.block_reduce_rec(
            m, perm, piv_dev, pivot_cols, s, mid, base_bp, use_coop, streamed, br_barrier, br_cond,
            br_ctas, slab,
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
        self.back_substitute_with(m, perm, r, pivot_cols, &RrOptions::default())
    }

    /// [`Self::back_substitute`] under explicit [`RrOptions`].
    pub fn back_substitute_with(
        &self,
        m: &mut DeviceMatrix,
        perm: &CudaSlice<u32>,
        r: usize,
        pivot_cols: &[usize],
        opts: &RrOptions,
    ) -> anyhow::Result<()> {
        if r == 0 {
            return Ok(());
        }
        let stream = self.stream();
        let stride = m.stride;
        let slab = bs_slab_rows(m.rows, stride, opts.bs_slab_bytes);
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
        let sms = self.sm_count()?;
        let br_occ = self
            .rr
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
                    slab,
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
            self.bs_clear_above(m, perm, &piv_dev, pivot_cols, s, e, 0, s, slab)?;

            e = s;
        }
        stream.synchronize()?;
        Ok(())
    }

    /// Full device-resident F₂ row reduction to RREF over one persistent buffer:
    /// [`forward_reduce`](Self::forward_reduce) then
    /// [`back_substitute`](Self::back_substitute). Mutates `m` to the reduced row
    /// echelon form (addressed through the returned `perm`) and returns
    /// `(perm, rank, pivot_cols)`. Bit-for-bit equal to `fp::Matrix::row_reduce`.
    pub fn row_reduce_dev(
        &self,
        m: &mut DeviceMatrix,
    ) -> anyhow::Result<(CudaSlice<u32>, usize, Vec<usize>)> {
        self.row_reduce_dev_with(m, &RrOptions::default())
    }

    /// [`Self::row_reduce_dev`] under explicit [`RrOptions`].
    pub fn row_reduce_dev_with(
        &self,
        m: &mut DeviceMatrix,
        opts: &RrOptions,
    ) -> anyhow::Result<(CudaSlice<u32>, usize, Vec<usize>)> {
        let t0 = std::time::Instant::now();
        let (perm, r, pivot_cols) = self.forward_reduce(m)?;
        let t1 = std::time::Instant::now();
        self.back_substitute_with(m, &perm, r, &pivot_cols, opts)?;
        if std::env::var_os("FP_CUDA_RR_TIMING").is_some() {
            eprintln!(
                "[rr_timing] forward={:.3}s back={:.3}s (r={r}, fused={})",
                (t1 - t0).as_secs_f64(),
                t1.elapsed().as_secs_f64(),
                tma_xor_supported(m.stride),
            );
        }
        Ok((perm, r, pivot_cols))
    }

    /// Download limbs `[first_limb, first_limb + ncols)` of the rows at `perm` positions
    /// `[k0, k1)` of a reduced matrix, packed tight, into `out`.
    ///
    /// The rows come out in `perm` order — pivot order after [`Self::row_reduce_dev`] — and only
    /// the slice asked for crosses to the host.
    #[allow(clippy::too_many_arguments)]
    pub fn download_rows_into(
        &self,
        m: &DeviceMatrix,
        perm: &CudaSlice<u32>,
        k0: usize,
        k1: usize,
        first_limb: usize,
        ncols: usize,
        out: &mut [u64],
    ) -> anyhow::Result<()> {
        let pr = k1 - k0;
        assert_eq!(
            out.len(),
            pr * ncols,
            "download_rows_into: buffer is {} limbs, the rows need {}",
            out.len(),
            pr * ncols
        );
        assert!(
            first_limb + ncols <= m.stride,
            "download_rows_into: limbs out of range"
        );
        assert!(k1 <= perm.len(), "download_rows_into: rows out of range");
        if pr == 0 || ncols == 0 {
            return Ok(());
        }
        let stream = self.stream();
        // SAFETY: uninitialized is sound because `gather_rows` writes all `pr * ncols` elements
        // before the copy below reads them.
        let dst = unsafe { stream.alloc::<u64>(pr * ncols) }?;
        let (r_u, fl, pr_u, nc, st) = (
            k0 as u32,
            first_limb as u32,
            pr as u32,
            ncols as u32,
            m.stride as u32,
        );
        let mut lb = stream.launch_builder(&self.rr.gather_rows);
        lb.arg(&dst)
            .arg(&m.buf)
            .arg(perm)
            .arg(&r_u)
            .arg(&fl)
            .arg(&pr_u)
            .arg(&nc)
            .arg(&st);
        // SAFETY: launch contract for `gather_rows`; the checks above keep its reads inside `m`
        // and `perm`.
        unsafe { lb.launch(cfg_1d(pr * ncols)) }?;
        stream.memcpy_dtoh(&dst, out)?;
        Ok(stream.synchronize()?)
    }
}

/// Knobs of one device row reduction ([`GpuContext::row_reduce_dev_with`]).
///
/// Every setting gives the same result; they differ in time and device memory.
#[derive(Clone, Copy, Debug, Default)]
pub struct RrOptions {
    /// Byte budget of one back-substitution product slab ([`bs_slab_rows`]); `None` for the
    /// default.
    pub bs_slab_bytes: Option<usize>,
}

/// Rows of one back-substitution product slab for a matrix of `rows × stride` limbs.
///
/// `budget` bytes of product rows (by default a sixteenth of the matrix, but at least 256 MiB),
/// rounded down to a whole GROUP_M rasterization band of M-tiles so that each slab keeps the GEMM's
/// B-panel reuse, and at least one band. A smaller slab bounds the product buffer tighter and
/// launches more, smaller GEMMs.
pub fn bs_slab_rows(rows: usize, stride: usize, budget: Option<usize>) -> usize {
    let band = TILE_M * GROUP_M;
    let n_padded = stride.max(1).next_multiple_of(NG as usize);
    let budget = budget.unwrap_or_else(|| (rows * stride * 8 / 16).max(256 << 20));
    ((budget / (n_padded * 8)) / band * band).max(band)
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::{
        matrix::{Matrix, arbitrary::MatrixArbParams},
        prime::TWO,
    };

    /// A context on device 0 shared by the test binary, or `None` without a usable GPU (see the
    /// GEMM tests' `gpu`, which this mirrors).
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

    /// The reduction compiles.
    #[test]
    fn row_reduce_compiles() {
        // SAFETY: see `compile_row_reduce`; the probe only tries to `dlopen` the library.
        if !unsafe { cudarc::nvrtc::sys::is_culib_present() } {
            return; // no CUDA Toolkit in this environment — nothing to compile with
        }
        let src = compile_row_reduce()
            .expect("row_reduce.cu must compile")
            .to_src();
        for kernel in [
            "panel_factor",
            "panel_factor_coop",
            "pf_find_swap",
            "pf_step",
            "pf_xor",
            "mark_live",
            "promote_pivots",
            "promote_coop",
            "zero_pivot_l",
            "gather_rows",
            "block_reduce_rref",
            "block_reduce_coop",
            "br_cond",
            "br_xor",
            "gather_cols",
            "xor_into_perm",
        ] {
            assert!(
                src.contains(kernel),
                "{kernel} is missing from the compiled PTX"
            );
        }
    }

    /// `m`'s rows as tight limbs.
    fn tight_limbs(m: &Matrix) -> Vec<u64> {
        let limbs = m.columns().div_ceil(64);
        (0..m.rows())
            .flat_map(|i| m.data()[i * m.stride()..][..limbs].iter().copied())
            .collect()
    }

    /// Reduce `base` on the device and check the pivot rows, rank and pivot columns against the
    /// CPU M4RI reduction, bit for bit, and that no trailing update went through a product
    /// buffer.
    fn check(gpu: &GpuContext, base: &Matrix, opts: &RrOptions) {
        let (rows, cols) = (base.rows(), base.columns());
        let stride = cols.div_ceil(64);
        let mut cpu = base.clone();
        let rank = cpu.row_reduce_cpu();
        let expected: Vec<usize> = (0..cols).filter(|&c| cpu.pivots()[c] >= 0).collect();

        let mut dm = gpu.upload(&tight_limbs(base), rows, cols).unwrap();
        assert_eq!(dm.stride(), device_stride(cols));
        let (perm, r, pivot_cols) = gpu.row_reduce_dev_with(&mut dm, opts).unwrap();
        assert_eq!(r, rank, "rank of {rows}x{cols}");
        assert_eq!(pivot_cols, expected, "pivot columns of {rows}x{cols}");
        let mut got = vec![0u64; r * stride];
        gpu.download_rows_into(&dm, &perm, 0, r, 0, stride, &mut got)
            .unwrap();
        assert_eq!(
            got,
            tight_limbs(&cpu)[..r * stride],
            "pivot rows of {rows}x{cols}"
        );
        assert_eq!(
            super::super::product_fallbacks(),
            0,
            "a trailing update went through a product buffer"
        );
    }

    /// An arbitrary `rows × columns` matrix over F₂.
    fn arb_matrix(rows: usize, columns: usize) -> BoxedStrategy<Matrix> {
        Matrix::arbitrary_with(MatrixArbParams {
            p: Some(TWO),
            rows: Just(rows).boxed(),
            columns: Just(columns).boxed(),
        })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(48))]

        /// Small shapes, an odd or even number of limbs: the reduction is the CPU's.
        #[test]
        fn row_reduce_dev_is_row_reduce(
            m in (1..=3 * TILE_M + 1, 1..=40 * 64 + 1usize)
                .prop_flat_map(|(rows, cols)| arb_matrix(rows, cols)),
        ) {
            let Some(gpu) = gpu() else {
                return Ok(()); // no usable GPU/driver in this environment — nothing to exercise
            };
            check(gpu, &m, &RrOptions::default());
        }
    }

    /// A pseudorandom `rows × cols` matrix of rank at most `rank`, `0` for full random.
    fn random_matrix(rows: usize, cols: usize, rank: usize, seed: u64) -> Matrix {
        use rand::{Rng, SeedableRng, rngs::StdRng};
        let mut rng = StdRng::seed_from_u64(seed);
        let mut rand = |r: usize, c: usize| {
            let v: Vec<Vec<u32>> = (0..r)
                .map(|_| (0..c).map(|_| rng.random::<bool>() as u32).collect())
                .collect();
            Matrix::from_vec(TWO, &v)
        };
        if rank == 0 {
            rand(rows, cols)
        } else {
            &rand(rows, rank) * &rand(rank, cols)
        }
    }

    /// Shapes wide enough for the grid-parallel back-substitution, its blocked TRSM and the
    /// multi-slab product, with odd and even limb counts.
    #[test]
    fn row_reduce_dev_wide() {
        let Some(gpu) = gpu() else {
            return; // no usable GPU/driver in this environment — nothing to exercise
        };
        // One band per slab, so every clear of more rows than that is split.
        let slabbed = RrOptions {
            bs_slab_bytes: Some(1),
        };
        for (rows, cols, rank, seed) in [
            (4096, 1100 * 64, 0, 1),
            (4096, 1101 * 64 - 7, 0, 2),
            (7000, 1024 * 64 + 3, 0, 3),
            (6000, 1200 * 64, 900, 4),
        ] {
            let m = random_matrix(rows, cols, rank, seed);
            check(gpu, &m, &RrOptions::default());
            check(gpu, &m, &slabbed);
        }
    }
}
