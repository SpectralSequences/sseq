# fp-cuda experiment log

What we tried, what it gained, and why the rejected alternatives were rejected. This is the place
for that record — the code comments are not, and neither is the README, which describes the crate
as it is rather than how it got there.

Entries are dated and name the hardware they were measured on: the conclusions are Hopper-specific
and several do not transfer between H100 and H200.

The knobs referred to below live in `cuda_kernels/params.h`, which both the kernel and the Rust
host read.

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

**Diagnosis, not a change.** It is what motivated the persistent grid and the clusters.

Throughput fell off sharply above 16384³, and the cause was not power or compute: the card drew
136 W of 310 W at 0–12% SM utilization. It was HBM bandwidth, spent re-reading B.

Each B column-panel is reused across every M-tile, so it wants to stay in L2. Once `K*N/8` exceeds
the 50 MB L2 — true at 32768², where B is 134 MB — each panel is evicted before the next M-tile
needs it and gets re-streamed from HBM once per M-tile. `examples/bench_shapes` isolates this with
equal-FLOPs shapes that differ only in how much B they touch.

## Persistent grid + clusters closed the L2 cliff (2026-07-07, H200 NVL)

**Kept.** Two changes against the diagnosis above, and together they removed it.

A persistent 1-D grid of roughly SM-count CTAs sweeps the output tiles in a grouped-along-M order
(`GROUP_M` M-tiles per band), which shortens each B-panel's reuse distance enough to keep it
L2-resident and cuts B's HBM re-reads by about `GROUP_M`. On top of that, `CLUSTER` CTAs along M
form a thread-block cluster and share a single HBM read of each B-panel via TMA multicast, each
computing a different M-tile.

`bench_shapes` is the check: equal-FLOPs shapes where B fits L2 and where B spills now run at the
*same* throughput. The cliff is gone, and throughput climbs with size rather than falling off.

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

## STAGES is capped at 4 under CLUSTER > 1 (2026-07-07, H200)

**Chosen:** `STAGES = 4`.

Each pipeline stage is 40 KB, so `STAGES = 5` needs ~206 KB. That is under the 227 KB opt-in cap,
but a cluster launch reserves additional shared memory for distributed-SMEM and cluster-barrier
bookkeeping, so 206 KB intermittently over-commits and the kernel faults with a flaky "unspecified
launch failure". The sanitizers miss it, because it is an async resource fault rather than a memory
error.

`STAGES = 5` is stable with `CLUSTER = 1`, but gives no large-N speedup there, so it buys nothing.

Earlier in development, before clusters, the tradeoff was pure latency-vs-occupancy and the shared
memory bill set the occupancy directly: `STAGES = 2` fit two CTAs/SM at 82 KB, `STAGES = 3` one CTA
at 122 KB.
