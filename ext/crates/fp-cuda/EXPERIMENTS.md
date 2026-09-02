# fp-cuda experiment log

What we tried, what it gained, and why the rejected alternatives were rejected. This is the place
for that record — the code comments are not, and neither is the README, which describes the crate
as it is rather than how it got there.

Entries are dated and name the hardware they were measured on: the conclusions are Hopper-specific
and several do not transfer between H100 and H200.

The knobs referred to below live in `cuda_kernels/params.h`, which both the kernel and the Rust
host read. The CPU-side counterpart to this file is `crates/fp/EXPERIMENTS.md`.

The overall shape of the work follows the optimization ladder in Pranjal Shankhdhar's
"Outperforming cuBLAS on H100" worklog, adapted to the binary (`b1`) GF(2) kernel.

## Measurement note

`ncu` cannot attach through the CUDA 13 driver on these boxes, so everything below was measured
on-device: kernel-only timing loops (`examples/bench_kernel_only`, `examples/tune`), equal-FLOPs
shape comparisons (`examples/bench_shapes`), and standalone wgmma/L2 microbenchmarks. Where a
number is a ceiling rather than a measurement, it says so.

## 128B swizzle + pipelined wgmma, ~100 → ~5,800 TOPS (2026-06-15, H100 NVL)

**Kept.** This is the change that made the backend worth having.

The starting point was a naive kernel at ~100 binary TOPS. Putting both operands in the 128B
K-major swizzle layout, pipelining the wgmmas one commit/wait per stage with scale-D = 1
accumulation, widening the MMA shape, and storing C through a TMA bulk (S2G) store took
kernel-only throughput to ~5,800 binary TOPS at 16384³ — 50–58× the baseline — bit-exact against
the CPU path at every size from 64 to 32768.

The operand layout is the load-bearing part: the TMA hardware applies the swizzle on the way into
SMEM, so the host emits a natural row-major tile and no hand-rolled interleave exists to get wrong.

Two supporting changes landed alongside it: `setmaxnreg` register reallocation between the producer
and consumer warpgroups, and moving the operands into dynamic shared memory so the pipeline depth
`STAGES` became a knob at all.

## Above 16384, the kernel was L2-residency bound on B (2026-06-15, H100 NVL)

**Diagnosis, not a change.** It is what motivated the persistent grid.

Throughput fell off sharply above 16384³, and the cause was not power or compute: the card drew
136 W of 310 W at 0–12% SM utilization. It was HBM bandwidth, spent re-reading B.

Each B column-panel is reused across every M-tile, so it wants to stay in L2. Once `K*N/8` exceeds
the 50 MB L2 — true at 32768², where B is 134 MB — each panel is evicted before the next M-tile
needs it and gets re-streamed from HBM once per M-tile. `examples/bench_shapes` isolates this with
equal-FLOPs shapes that differ only in how much B they touch.

## Persistent grid + grouped rasterization closed the L2 cliff (2026-07-07, H200 NVL)

**Kept.** One change against the diagnosis above, and it removed it.

A persistent 1-D grid of roughly SM-count CTAs sweeps the output tiles in a grouped-along-M order
(`GROUP_M` M-tiles per band), which shortens each B-panel's reuse distance enough to keep it
L2-resident and cuts B's HBM re-reads by about `GROUP_M`.

`bench_shapes` is the check: equal-FLOPs shapes where B fits L2 and where B spills now run at the
*same* throughput. The cliff is gone, and throughput climbs with size rather than falling off.

## Thread-block clusters + TMA multicast, removed (2026-08-14, H200 NVL)

**Rejected, after having been kept.** A revision on top of the persistent grid grouped `CLUSTER`
CTAs along M into a `__cluster_dims__` thread-block cluster that shared a single HBM read of each
B-panel via TMA multicast, with a cluster-wide empty barrier reached through `mapa`.

`__cluster_dims__` makes a cluster's CTAs co-resident by construction — a hard placement
constraint. The launch therefore does not queue for SMs the way an ordinary grid does: when another
runtime holds them it fails outright with `CUDA_ERROR_LAUNCH_FAILED`. That is the fault
compute-sanitizer could never attribute (0 invalid accesses across a whole run — it was never a
memory bug).

The multicast did not pay for that. `bench_kernel_only` on an idle H200, cluster-free vs cluster,
binary TOPS, `correct=true idempotent=true` throughout:

| size (M=K=N) | cluster-free | cluster |
|--------------|--------------|---------|
| 4096         | 4091         | 4071    |
| 8192         | 6687         | 6778    |
| 16384        | 8501         | 8632    |
| 32768        | 9608         | 9664    |

Within 1.5% everywhere, because `GROUP_M` rasterization already keeps B L2-resident, so the
second-order saving multicast adds does not show up at these shapes. The CTAs are now independent,
which also makes the grid size a throughput parameter only rather than a placement demand.

## Register-blocking, the refill-bandwidth fix (2026-07-07, H200 NVL)

**Kept.** After the cliff closed, the kernel was L2→SMEM refill bound — roughly 8 TB/s sustained
against a single-warpgroup wgmma ceiling near 12,500 TOPS.

Each CTA now computes a `TM×NB` output block as `MSTRIPS` `m64n128` strips that all reuse one
loaded B sub-tile, so a single L2→SMEM read of B feeds every strip. At `MSTRIPS = 3` that is a
192×128 block and 33% fewer operand-refill bytes per MAC.

This is also what fixed the MMA shape at `m64n128k256`: the kernel had briefly used the wider
`m64n256k256`, but halving N is what makes strips share a B tile, and the trade is strongly
favourable.

## Epilogue overlap and fence hoisting (2026-07-07, H200 NVL)

**Kept.** Two smaller rungs after register-blocking.

Double-buffering `sC` and deferring the store wait to `cp.async.bulk.wait_group.read 1` lets tile
T's output store drain during tile T+1's compute. Hoisting the `wgmma.fence` to once before the
K-loop, rather than twice per k-chunk, removes a warpgroup-wide sync from the inner loop.

After these, skipping an entire operand load barely changes throughput, which says the kernel is
now **TMA-latency bound**: the consumer out-runs per-tile TMA latency. Deeper pipelines would help,
but `STAGES` is capped by shared memory — see below.

## 1 CTA/SM beats 2 CTAs/SM (2026-07-07, H200)

**Chosen:** 1 CTA/SM, with the largest accumulator that fits under the 255-register cap
(`MSTRIPS = 3` → 192 accumulator registers per thread).

Two co-resident CTAs need the compiled register count ≤ 128/thread (2 · 256 · 128 = the 64K
register file). But the resident accumulator is exactly what gives the kernel its arithmetic
intensity, and shrinking it to fit two CTAs (`MSTRIPS = 1`, 64-register accumulator → occupancy 2)
collapses that intensity: 16384³ dropped from ~8,600 to ~5,500 TOPS.

High arithmetic intensity (a big accumulator) and high occupancy compete for the same register
file, and on this kernel intensity wins.

## STAGES is capped at 4 (2026-07-07, H200)

**Chosen:** `STAGES = 4`.

Each pipeline stage is 40 KB, so `STAGES = 5` needs ~206 KB. That is under the 227 KB opt-in cap
and it runs, but it was measured to give no large-N speedup, so it buys nothing.

(While the cluster layer was in place, `STAGES = 5` was not merely useless but unstable: a cluster
launch reserves additional shared memory for distributed-SMEM and cluster-barrier bookkeeping, so
206 KB intermittently over-committed and the kernel faulted with a flaky "unspecified launch
failure" that the sanitizers missed, being an async resource fault rather than a memory error. That
constraint went with the clusters; the "no speedup" one is what still fixes the value.)

Earlier in development the tradeoff was pure latency-vs-occupancy and the shared memory bill set
the occupancy directly: `STAGES = 2` fit two CTAs/SM at 82 KB, `STAGES = 3` one CTA at 122 KB.

## The epilogue fence goes before the barrier, at ~1% (2026-08-15, H200)

**Chosen:** `fence_async_shared()` then `__syncthreads()`, not the reverse.

`fence.proxy.async.shared::cta` orders only the *executing thread's* prior generic-proxy writes.
With the fence after the barrier, only `t == 0` executed it, so the other consumers' `atomicXor`
packing was never fenced into the async proxy before the TMA store read `sC` — correct in practice
on this hardware, but not guaranteed by the model.

Output stays bit-exact either way. The reorder costs ~0.96% at 4096 and ~0.73% at 8192 (every
post-change run below the lowest pre-change run, so the loss is real), and is at noise level at
16384 and 32768, the sizes the kernel is actually used at. Paid.

## Grid caps for the barrier-bound row-reduce kernels (2026-07-28, H200)

**Chosen:** cap the grid well below full occupancy for the three grid-barrier-bound kernels, with
env overrides. Defaults live in `src/lib.rs` as `DEFAULT_PF_CTAS`, `DEFAULT_PROM_CTAS` and
`DEFAULT_BR_CTAS`.

A `grid_sync`'s cost scales with the CTA count, and these kernels sync per pivot, so past the point
where the grid covers the per-pivot work more CTAs only buy more expensive barriers.

| kernel | syncs | gain from the cap | override |
|---|---|---|---|
| `panel_factor_coop` | 1 per pivot bit | +3% at 2^16 | `FP_CUDA_PF_CTAS` |
| `promote_coop` | 1 per pivot | +6% at 2^16, +5% at 2^17 | `FP_CUDA_PROM_CTAS` |
| `block_reduce_coop` (TRSM) | 2 per pivot | 334 -> 115 ms (2.9x) at 2^16, +12% end-to-end | `FP_CUDA_BR_CTAS` |

The block-reduce cap applies **only under TRSM**, where the reduces are narrow base blocks whose
per-pivot XOR needs little width. Without TRSM the reduce is the full block and its XOR genuinely
needs the whole grid, so full occupancy stands.

## Forward-pass panel width scales with stride (2026-07-20, H200)

**Chosen:** `adaptive_bl` in `src/lib.rs` — `stride/256` without the cooperative promote,
`stride/128` with it, capped at `MAX_BL`. Override with `FP_CUDA_BL`.

Wider panels raise the trailing GEMM's contraction dimension, reclaiming the K-padding waste. The
counter-pressure is the promotion cost: the single-CTA `promote_pivots` is O(bl) total, so narrow
panels win there, while the cooperative `promote_coop` (used at stride >= 1024) is ~bl-independent,
so the panel can widen until the forward GEMM stops padding.

Measured optima, half-rank inputs:

| n | stride | promote | optimal bl |
|---|---|---|---|
| 2^15 | 512 | single-CTA | 2 |
| 2^16 | 1024 | cooperative | 8 |
| 2^17 | 2048 | cooperative | 16 |

## The row-reduce CPU crossover sits just below 8192 (2026-08-02, H200)

Half-rank square inputs, device time including upload, against single-threaded M4RI `row_reduce`:

| n | GPU vs M4RI |
|---|---|
| 4096 | 0.57x (a loss) |
| 8192 | 1.57x (a win) |

Hence `DEFAULT_RR_THRESHOLD = 8192`, well above the matmul threshold — a full reduction is many
dependent panel steps, not one GEMM, so its crossover is later.

The small-n crossover is bound by fixed launch and transfer overhead rather than the trailing GEMM,
so the throughput wins above (which scale with n^2) did not move it. The comparison is against
single-threaded M4RI; the concurrent CPU path is faster, which only pushes the crossover up, so
8192 is a floor rather than a fitted optimum.

## Multi-CTA block reduction pays only on wide matrices (2026-07-30, H200)

**Chosen:** gate the grid-parallel back-substitution reduce on `stride >= 1024`; below that the
single-CTA `block_reduce_rref` wins.

Spreading each block's per-pivot clear across the whole grid only pays once the block work
(~`bp · stride`) is large enough to cover the extra barrier traffic:

| n | stride | grid-parallel vs single-CTA |
|---|---|---|
| 2^15 | 512 | neutral |
| 2^16 | 1024 | +6% |
| 2^17 | 2048 | +18% |

Both grid-parallel variants are gated the same way — the cooperative `block_reduce_coop` and the
kernel-boundary `br_cond`/`br_xor` pair that is the default.
