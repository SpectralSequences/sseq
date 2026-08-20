# fp experiment log

What we tried, what it gained, and why the rejected alternatives were rejected. This is the place
for that record — the code comments are not, and neither is the README, which describes the crate
as it is rather than how it got there.

The GPU-side counterpart is `crates/fp-cuda/EXPERIMENTS.md`.

## Measurement note

Everything below is half-rank square input (`rank = n/2`, the target regime) on a 4-core AVX-512
host, so the GEMM is the tiled AVX kernel rather than a scalar fallback. Criterion medians up to
n = 4096; the larger sizes are single-shot runs of `examples/reduce_scaling.rs`, not averaged.

**A methodology trap worth naming:** the first thread-scaling numbers were taken with `concurrent`
*off*, so the GEMM's `maybe_rayon::join` was inert and the "parallel" column measured a serial
GEMM. Any scaling claim about `row_reduce_blas3` has to be built with `--features concurrent`.

## GEMM row reduction is not a GPU-only technique (2026-06-20)

**The first reading was wrong, and it was wrong because the matrices were small.** At n <= 4096
`row_reduce_blas3` measured ~10-16x slower than M4RI `row_reduce`, which read as "not GEMM-bound, a
GPU-only technique — don't route CPU `row_reduce` here."

| n | M4RI | blas3 | ratio |
|---|---|---|---|
| 512 | 274 us | 3.28 ms | 12x |
| 1024 | 941 us | 14.8 ms | 16x |
| 2048 | 4.62 ms | 64.3 ms | 14x |
| 4096 | 27.2 ms | 307 ms | 11x |

That conclusion is an artefact of the size. At small n the `O(m·n)` panel scan and the serial
back-substitution dominate a GEMM that is itself small and overhead-heavy. Extending the sweep
upward reverses it — the ratio collapses monotonically, and M4RI degrades *superlinearly* at 32768
(a 12.9x step where `n^3` predicts ~8x) because the 134 MB matrix overflows cache and the table
method turns memory-bound, while the tiled GEMM tracks `n^3` at 9.3x.

Large matrices punish M4RI twice: no parallelism **and** worse locality. M4RI is a sequential
dependency chain — build the `2^k` table, reduce, advance — so it is flat in the core count; the
`L·U` GEMM is data-parallel. That is the whole quantitative case for the approach.

## Two constant-factor fixes moved the CPU crossover onto the target size (2026-06-24)

Full progression, blas3 wall time and (ratio to M4RI):

| n | M4RI | original | + blocked back-sub | + limb-wise panel |
|---|---|---|---|---|
| 4096 | 28 ms | 299 ms (10.6x) | 207 ms (7.3x) | 209 ms (7.5x) |
| 8192 | 224 ms | 1.12 s (5.0x) | 1.30 s (6.0x) | **1.02 s (4.6x)** |
| 16384 | ~2 s | 8.16 s (4.2x) | 5.48 s (2.8x) | **4.00 s (1.8x)** |
| 32768 | ~26-31 s | 76.0 s (3.07x) | 57.2 s (2.22x) | **45.9 s (1.46x)** |

**Blocked back-substitution** turns the serial `O(R^2·n)` row operations into the mirror `X·U`
GEMM. At 16384 it took 8.16 s -> 5.48 s and raised 4-core scaling 1.23x -> 1.89x (~63% parallel
fraction by Amdahl). The apparent regression at 8192 sits between two points that both improved and
is single-shot noise.

**Limb-wise panel** replaces the per-bit `entry()` / `add_basis_element()` accessors in the panel
pivot search, the forward elimination and the back-substitution gather with direct limb reads and
writes. It helps *more* at large n, because the `O(m·n)` panel scan the accessor overhead sat on
top of is a growing fraction there.

Together the ratio at 2^15 falls 3.07x -> 1.46x, dropping ~0.81x per doubling. Extrapolated, blas3
crosses M4RI on **four cores at around n ~ 1.2e5** — the size of the real workload. Two small
constant-factor rewrites moved the crossover off "few x 10^5" and onto the target.

The consequence for the code: dispatch is a **size threshold**, not "GPU-only". Below the crossover
M4RI stays.

## Panel width is not a tuning knob (2026-06-20)

At n = 2048, half-rank, panel widths of 64, 128, 256, 512 and 1024 all land at 64-69 ms — within
noise of each other. `DEFAULT_BLOCK_COLS` is therefore chosen for shape rather than fitted: wide
enough that the trailing GEMM's inner dimension is a healthy number of pivots, narrow enough that
the panel factorization stays a lower-order term. `row_reduce_blas3_block` exposes the width so the
sweep can be repeated, not because a caller should be picking it.

## Deferred: recursive PLE (2026-06-18)

**Deferred, not rejected.** Instead of a fixed panel width, split the column range in two and
recurse: PLE-factor the left half, update the right by the resulting transform (a GEMM), PLE-factor
the trailing part, combine the permutations, with a ~64-wide panel as the base case. This is M4RI's
`_mzd_pluq` and the Dumas-Pernet-Sultan CUP decomposition.

It shrinks the panel term into another level of GEMM, giving the sub-cubic `O(m·n·R^(w-2))` shape
and better tensor-core occupancy. The cost is two-sided permutation bookkeeping with rank-profile
tracking, and a triangular-solve-shaped update — materially more to get right and to validate.

The iterative blocked scheme already puts nearly all flops in GEMM and is far easier to prove
correct, so it ships first. Recursion can be grafted onto the panel later, and onto the whole
column split if profiling warrants it.
