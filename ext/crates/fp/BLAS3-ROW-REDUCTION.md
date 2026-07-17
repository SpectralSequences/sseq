# BLAS3 (GEMM-based) row reduction over F₂ — design plan

> Status: **Phase 1 implemented** (`src/matrix/blas3.rs`,
> `Matrix::row_reduce_blas3`); the device-resident GPU phases remain design,
> to be executed once a Hopper GPU is available.
>
> Goal: reduce a large, highly rank-deficient dense matrix over F₂ to reduced
> row echelon form (RREF) — the semantics of `Matrix::row_reduce` — with the
> bulk of the work expressed as F₂ matrix–matrix products (`wgmma.b1`), so it
> rides the `fp-cuda` GEMM kernel instead of the scalar/AVX M4RI path.
>
> Target workload: `m ≈ 100 000` rows, `n` comparable, **rank `R ≈ m/2`**.
>
> **What Phase 1 built.** A CPU-resident blocked reduction that runs against the
> generic `<&Matrix as Mul>::mul` (AVX-512 tiled kernel today, GPU once wired
> in), proptested bit-for-bit against `Matrix::row_reduce` (RREF *and* pivots),
> including rank-deficient generators. It is structured as **forward blocked
> elimination to echelon form** (the GEMM-heavy part, §4) followed by a
> **back-substitution** pass to RREF (§4.6). The Gauss–Jordan-in-one-pass sketch
> the earlier draft of §4 described does *not* work with a deferred trailing
> update: back-substitution keeps mutating the pivot rows' trailing, so they are
> not stable operands for the GEMM. Forward-only elimination only ever reduces a
> pivot by an *earlier* (stable) pivot, which is what makes the deferred GEMM
> correct — see §4.6.

---

## 1. The two constraints that dictate the design

Everything below is shaped by two facts the naive "call the fast GEMM in a loop"
approach gets wrong.

### 1.1 The computation must stay **GPU-resident**

The current GPU entry point is `fp_cuda::matmul_b1_raw`, dispatched per-multiply
from `fp/src/blas/cuda.rs::try_mul`. Reading `matmul_b1_inner` (fp-cuda
`src/lib.rs`), **every call**:

1. host-repacks A into TMA interleave and transposes+packs B (`bt`),
2. `clone_htod(A)`, `clone_htod(B)` — two H2D copies,
3. `alloc_zeros(C)` + launch,
4. `clone_dtoh(C)` — one D2H copy.

For an `M×N` product the transfers are `Θ(MK + KN + MN)` bytes. A blocked row
reduction performs `Θ(n/b)` trailing GEMMs. If each is an independent
`matmul_b1_raw`, we pay a **full PCIe round trip per panel**, re-uploading
overlapping regions of the same matrix `n/b` times. At 1.25 GB for the working
matrix and ~tens of GB/s over PCIe, transfer — not compute — sets the ceiling.
The Phase 3–9 kernel peaks at ~5 800 binary TOPS on H100; that throughput is
unreachable if the operands are re-marshalled from the host each panel.

**Consequence.** The matrix must live in **one persistent device allocation**
for the whole reduction. Panel factorization, the trailing GEMM, and the
in-place update all run as device kernels over that buffer. The host sees only:
the initial H2D of the matrix, a trickle of scalars per panel (pivot columns +
rank, ~kilobytes), and one final D2H. Total bus traffic ≈ **2× the matrix**,
independent of `n/b`. This is a new device-resident API surface in `fp-cuda`
(§6), *not* a caller of the existing `try_mul`.

### 1.2 Column operations are **expensive**; row operations are cheap

Layout (`matrix_inner.rs`): row-major, each row a run of 64-bit limbs; column
`j` is bit `j%64` of limb `j/64`. Therefore:

| Operation | Cost | Why |
|---|---|---|
| XOR one row-slice into another (contiguous limbs) | **cheap** | coalesced; one thread per limb-column streams contiguous memory |
| Whole-matrix GEMM `C = A·B` | **cheap** (relative to work) | the `wgmma.b1` kernel |
| Read/scan/gather a single **column** across rows | **expensive** | strided by `stride` limbs, one useful *bit* per limb loaded, no coalescing; pivot search and column-gather live here |
| Swap two **columns** | **expensive** | strided RMW of single bits |
| Swap two **rows** | cheap-ish, but moves `stride` limbs × … | avoid at scale — see virtual permutation (§4.3) |

So the design must (a) confine every column-indexed access to the **narrow
panel** (width `b`, a few limbs), never the full width, and (b) express all
wide, dominant work as GEMM or as contiguous row-XOR. It must **never** permute
columns, and should avoid physically moving rows in the 1.25 GB buffer.

---

## 2. Why the existing code can't be reused as-is

- `Matrix::row_reduce` (`matrix_inner.rs:674`) is **M4RI** (Method of Four
  Russians): build a table of `2^k` combinations of `k` pivot rows, reuse it to
  reduce the rest. Arithmetic intensity is ~O(1) per bit touched — a BLAS2-class
  algorithm. It cannot feed a tensor-core GEMM and it is inherently host/CPU.
- `fp/src/blas` (the AVX-512 tiled GEMM) and `fp-cuda` give us a *fast GEMM*,
  but no *elimination*. The gap is precisely a row-reduction whose inner loop is
  GEMM.
- `cuda.rs::try_mul` is the wrong granularity (per-multiply marshalling, §1.1).

The literature calls the object we want the **PLE decomposition** `A = P·L·E`
(P a row permutation, L unit lower-triangular on the pivot rows/cols, E a row
echelon form). It is the rank-revealing generalization of PLU, and the standard
result is that it **reduces to matrix multiplication** — this is exactly the
BLAS3 property we need. References in §8. RREF is a cheap back-substitution away
from PLE (and, in the Gauss–Jordan variant below, produced directly).

---

## 3. Cost model and the rank-deficiency win

Let `m` rows, `n` cols, rank `R`. Right-looking blocked elimination with panel
width `b`:

- **Trailing GEMM per panel:** `L (m × pr) · U (pr × (n−c−b))`, where `pr ≤ b` is
  the number of pivots found in that panel. Summed over panels, the GEMM inner
  dimension totals `R` (every pivot is used exactly once as a GEMM row), and the
  trailing width averages `~n/2`. **Total GEMM work `≈ m · n · R / 64` bit-ops**
  (the `/64` is limb packing; tensor cores add another large constant factor).
  Because the cost carries a factor `R`, not `n`, **a rank-½ matrix is ~2× cheaper
  than full rank for free** — the single most important property for this
  workload. Empty column panels (no pivots) contribute *zero* GEMM.

- **Panel factorization per panel:** confined to width `b`. Total
  `≈ m · R · b / 64` for the eliminations plus `≈ m · R` bit-reads for pivot
  search (the only column-indexed term). With `b ≪ n` this is a lower-order term
  vs. the trailing GEMM (ratio `≈ b / n`). Choosing `b` in `[256, 1024]` keeps
  it negligible while keeping the GEMM's inner dimension `pr` large enough to
  saturate the kernel.

Net: **`O(m·n·R)` total, GEMM-dominated, ∝ rank.** The recursive PLE variant
(§7) lowers the panel term further and reaches the sub-cubic
`O(m·n·R^{ω−2})` shape, but the iterative scheme already puts essentially all
the flops in the GEMM.

---

## 4. The algorithm: right-looking blocked Gauss–Jordan, GPU-resident

Produces global RREF directly (clears above *and* below pivots), matching
`row_reduce` semantics (reduced form + pivot list). One device buffer `M`
holds the matrix throughout.

State: `r` = pivots found so far (also the number of finished pivot rows, kept
at the top via the virtual permutation of §4.3). Panels sweep columns left to
right; `c` is always a multiple of 64 and `b` a multiple of 64, so a panel
occupies whole limbs `[c/64, (c+b)/64)` of each row — critical for §4.4.

For each panel `[c, c+b)`:

### 4.1 Step A — panel factorization (the only column-op region)

Reduce the sub-block `M[active, c:c+b]` to RREF **within the panel columns
only**, discovering up to `b` pivots. Row operations here touch **only the panel
limbs** `[c/64, (c+b)/64)` — the trailing columns are deliberately left stale
and fixed up by the GEMM in Step B. This is where all the (narrow) column work
lives; §5 details the device kernel. Outputs:

- `pr` new pivots and their absolute pivot columns `q_0 < … < q_{pr−1} ∈ [c,c+b)`;
- the `pr` pivot rows logically placed at positions `[r, r+pr)` (via the
  permutation, §4.3), each `1` at its own pivot column and `0` at the other
  pivot columns (RREF among pivots);
- the **multiplier matrix** `L`: for every non-pivot row `j`, an `pr`-bit row
  recording which pivot rows were added to `j` during the panel sweep. This is
  captured *as the sweep runs* (§4.2) and equals the panel-pivot-column bits of
  the pristine rows.

### 4.2 Why the multipliers `L` are exactly the pivot-column bits

The panel sweep is Gauss–Jordan restricted to panel limbs. When it clears pivot
column `q_k` from row `j`, it adds pivot row `k` iff the current bit `M[j, q_k]`
is 1 — record that bit as `L[j, k]`, then clear it. Because the pivot rows are
kept mutually reduced (each pivot row is 0 at the *other* pivot columns), adding
pivot row `k′≠k` to row `j` never disturbs `M[j, q_k]`. Hence each captured bit
equals the **pristine** `M[j, q_k]`, and the `L[j, ·]` are independent — no
ordering hazard. A row never adds itself, so the pivot rows get `0` on their own
column and are correctly reduced by *later* pivots in the same panel. The trailing
GEMM then applies these same multipliers to the trailing columns. This is the
in-place-`L` trick of blocked LU, specialized to F₂ (the multiplier is a single
bit; "keep `L`" means "read the bit before you clear it").

### 4.3 Virtual row permutation (avoid moving 1.25 GB)

"Move the pivot row up to position `r`" must **not** physically copy rows in the
device buffer. Keep an on-device permutation/index vector `perm` (length `m`).
"Swap rows" swaps two entries of `perm`; kernels dereference `M[perm[i]]`. The
matrix bytes never move during the sweep. A single optional physical compaction
(gather by `perm`) can be done once at the very end when materializing the
result — or never, if the caller reads rows through `perm`. Row *swaps* are thus
`O(1)` index writes, not `O(stride)` limb moves.

### 4.4 Step B — trailing update (the GEMM), in place on device

Apply the recorded eliminations to the trailing columns in one GF(2) product:

```
U = pivot rows [r, r+pr) restricted to columns [c+b, n)     # pr × T, T = n−c−b
G = L · U                                                    # m × T  (wgmma.b1)
M[:, c+b:n] ^= G                                             # in-place XOR
```

`L` is `m × pr` (pr ≤ b, narrow). The product `G` is `m × T`. Because `c+b` is a
multiple of 64, the trailing region begins at a limb boundary: `U` is a
contiguous limb-slice of the pivot rows, and the update is a **whole-limb XOR**
of `G` into `M`'s trailing limbs — `G.stride == (#trailing limbs of M)` exactly,
so it is a coalesced row-parallel XOR, no column indexing. Pivot rows get `L`-row
= 0 on their own future and are left untouched by the XOR (their panel columns
were already finished in Step A), so they stay correctly reduced.

Then `r += pr` and advance to the next panel. After the last panel, `M`
(read through `perm`) is in global RREF; the pivot list is `q_*` gathered across
panels.

### 4.5 Correctness argument (for validation against `row_reduce`)

Invariant after processing columns `[0, c+b)`: `M` is in global RREF with respect
to the pivots found so far, over columns `[0, c+b)`, and rows `[0, r)` are those
pivots. Step A extends the reduced region across the panel columns and records
`L`; Step B extends it across the trailing columns with the identical multipliers
(the split is only *where* the same row-combinations are applied — panel limbs by
XOR in A, trailing limbs by GEMM in B). No double application: A touches only
panel limbs, B only trailing limbs. Therefore the invariant holds inductively and
the final matrix is the RREF. This is checkable exhaustively on CPU (§9) with no
GPU.

### 4.6 What Phase 1 actually implements: forward echelon + back-substitution

§4.1–4.5 describe a one-pass Gauss–Jordan (clear above *and* below per panel).
That does **not** compose with a deferred trailing GEMM, and the reason is worth
recording. If a panel clears its pivot columns from rows *above* (earlier
pivots), it mutates those pivot rows' trailing. But a later panel's pivot may be
a row that an earlier panel reduced — and its deferred trailing has to be
"realized" from the earlier pivots' trailing at GEMM time. If those trailings
keep changing (because of ongoing back-substitution), the realized value is
wrong. The 3×65 counterexample that surfaced this: three pivots that reduce each
other, and only the trailing (limb 1) column comes out wrong.

The fix — standard for blocked LU/PLE — is to split the two directions:

- **Forward pass (blocked, the GEMM part).** Each panel does forward elimination
  only: clear pivot columns from rows *below* the pivots, deferring their
  trailing to the `L·U` GEMM of §4.4. A new pivot row is "promoted" by replaying
  the earlier *this-panel* pivots into its trailing — and those earlier pivots
  are **stable**, because forward elimination never reduces a pivot by a *later*
  one. This yields row-echelon form: pivot rows at `[0, R)`, each reduced by
  earlier pivots; rows `[R, m)` zero. This is where the `O(m·n·R)` GEMM work is.
- **Back-substitution pass → RREF.** Clear each pivot column from the rows above
  it, processing pivots high-to-low so every source row is already fully reduced
  before it is used (adding it back can't reintroduce a later pivot column).

Phase 1 does the back-substitution with plain full-width row operations —
`O(R²·n)`, correct and simple, but **not yet BLAS3**. Because it runs on only the
`R` pivot rows and has the identical `L·U` structure as the forward update
(mirror image, right-to-left over the pivot rows), it blocks into GEMM the same
way; that is the first Phase-2 task (and for `R ≈ m/2` it is ~half the total
work, so it must be blocked before the GPU port pays off). The forward pass — the
novel, dominant, GEMM-based part — is what Phase 1 validates.

---

## 5. The panel-factorization kernel (design of the hard part)

The panel `P = M[active, c:c+b]` is `m × (b/64)` limbs — e.g. `m=1e5`, `b=256`
→ `1e5 × 4` u64 = 3.2 MB, a small working set that stays in device memory (and
largely in L2) across the sweep. Two implementation strata:

1. **`b = 64` (one limb wide) base kernel.** Each active row contributes one
   u64. Factoring is "reduce `m` u64 words to echelon over GF(2)": iterate the 64
   bit-positions; for bit `j`, find a not-yet-pivot word with bit `j` set (a
   `find-first` reduction over `m` bits — the lone column op), mark its `perm`
   slot as pivot `k`, then in a **row-parallel masked XOR** add that pivot word
   into every other word whose bit `j` is set, capturing the cleared bit into
   column `k` of `L`. 64 bit-steps, each a coalesced pass over `m` words.

2. **`b = 256…1024` via the base kernel + intra-panel GEMM (recursive/blocked
   within the panel).** Split the panel into 64-wide sub-panels; factor the
   leftmost with (1), then update the rest of the *panel* with a small GEMM
   (same L·U shape, but width `≤ b`), recursing. This is the panel-level image of
   §7 and keeps even the panel's dominant work in GEMM. Start with (1) at modest
   `b`; add (2) only if the panel term shows up in profiles.

Design points:
- **No column swaps ever.** Pivot selection permutes *rows* (via `perm`), and
  free (non-pivot) columns are simply skipped and recorded — the pivot column
  list `q_*` carries the column identity; columns are never physically moved.
- **`find-first` down a 64-bit column** is the only irreducible column read.
  It is `m` bit-reads per column, `b` columns per panel, `n/b` panels → `m·R`
  total (§3) — lower order, and itself a coalesced reduction (load each active
  row's panel limb once; test the bit).
- The kernel emits to host only `pr` and `q_0..q_{pr−1}` (a few hundred ints per
  panel). Everything else (`L`, the reduced pivot rows, `perm`) stays on device.

---

## 6. Required `fp-cuda` device-resident API (new surface)

The current crate exposes one shot `matmul_b1_raw(ctx, a, m, k, b, n) -> Vec<u64>`
(host in, host out). Row reduction needs a **handle-based device API** so buffers
persist across kernels. Sketch:

```rust
pub struct DeviceMatrix { ptr: CudaSlice<u64>, rows, cols, stride, /* on device */ }

impl GpuContext {
    fn upload(&self, m: &Matrix) -> DeviceMatrix;           // one H2D
    fn download(&self, dm: &DeviceMatrix) -> Matrix;        // one D2H (applies perm)

    // in-place device kernels over a persistent buffer:
    fn panel_factor(&self, dm, c, b, r, perm, out_L, out_pivcols) -> pr;   // §5
    fn gemm_xor_into(&self, dm, L, pivot_rows, c_plus_b, r, pr);          // §4.4, in place
    // (gemm_xor_into wraps the existing wgmma.b1 kernel but writes C by XOR
    //  into an existing device region instead of alloc+dtoh)
}
```

Key deltas from today's kernel:
- **Persistent allocation** for `M`; kernels take device pointers, not host
  slices. Reuse the *compute* core of `matmul_b1.cu` unchanged; change only the
  host glue (no per-call `htod`/`dtoh`/`alloc_zeros`).
- **Fused epilogue: XOR-accumulate C into a target region** rather than writing a
  fresh C and copying it back — saves an allocation and a D2H per panel and does
  the `M[:,c+b:] ^= G` update for free inside the GEMM store.
- The `wgmma.b1` op is AND+popc→`s32`; the panel bit-pack/`n256` fragment layout
  already solved in Phase 4 (`af251a5`) is reused for `L`/`U`.

This is the bulk of the *new* GPU work. The CPU/algorithm side (§4) is
implementable and testable first (§9) against the existing AVX GEMM.

---

## 7. Alternative: recursive PLE (most GEMM-heavy)

Instead of a fixed panel width, split the column range in two and recurse
(`A = [A_L | A_R]`): PLE-factor `A_L`, update `A_R` by the resulting transform
(a GEMM), PLE-factor the trailing part of `A_R`, and combine the permutations.
Base case: a ~64-wide panel via §5(1). This is M4RI's `_mzd_pluq` / the
Dumas–Pernet–Sultan CUP decomposition. Pros: the panel term shrinks to another
level of GEMM, giving the sub-cubic `O(m·n·R^{ω−2})` shape and maximal tensor-core
occupancy. Cons: permutation bookkeeping (two-sided, rank-profile tracking) and a
triangular-solve-shaped update (`TRSM`), all as device kernels — materially more
to get right and to validate. **Recommendation:** ship the iterative blocked
scheme (§4) first — it already puts ~all flops in GEMM and is far easier to prove
correct — then graft recursion onto the *panel* (§5(2)) and, if profiling
warrants, onto the whole column split.

---

## 8. Rank-deficiency specifics (the `R ≈ m/2` regime)

1. **Work already ∝ R** (§3): the GEMM inner dimension is the pivots actually
   found; half-rank ⇒ ~half the flops, automatically.
2. **Active-row compaction.** As columns are consumed, rows that are zero across
   all remaining columns are dead weight in the GEMM's `m` dimension. Periodically
   partition `perm` so the GEMM operates on `~R` active rows instead of `m`.
   "Is this row now zero in `[c, n)`?" is an OR-reduce over a contiguous limb
   range — a **row op**, cheap and coalesced. Compacting is a `perm` partition,
   no byte movement (§4.3). For `R = m/2` this trims the trailing GEMM's tall
   dimension toward `R`, compounding with point 1.
3. **Skip empty panels cheaply.** A panel with `pr = 0` (no pivots) does no GEMM
   and no update — the sweep just advances `c`. Wide zero regions cost only their
   `find-first` scans.

---

## 9. Validation without a GPU (do this first)

The **algorithm** (§4) is independent of the device; only the *residency* is
GPU-specific. So it can be built and proven correct entirely on CPU:

- Implement §4 in `fp` against the existing `&Matrix * &Matrix` (AVX-512 tiled
  GEMM) as the Step-B primitive, with a CPU panel factorization for Step A.
- **proptest** the result against `Matrix::row_reduce` for equality of the RREF
  *and* the pivot list, over random `p=2` matrices across the shape grid already
  used in `blas/mod.rs` tests, plus deliberately rank-deficient generators
  (random rank `≤ min(m,n)/2`, duplicated/zero rows, zero columns).
- Add a criterion bench (mirroring the migrated algebra suite) measuring the
  M4RI-vs-blocked crossover as a function of `(m, n, R, b)` to pick a default `b`.

Once green on CPU, the GPU work (§6) is a *residency/perf* port with a
bit-exactness oracle already in hand — the CPU blocked result is the reference,
and the GPU path must match it bit-for-bit (the same discipline `fp-cuda` used
for the GEMM: CPU↔GPU bit-equality at every size).

Crucially, this de-risks the expensive GPU effort: the design is falsifiable on a
laptop before a single device kernel is written.

---

## 10. Concrete work plan

| Phase | Where | Deliverable | Status |
|---|---|---|---|
| 0 | `fp` | This document | ✅ done |
| 1 | `fp` | CPU blocked forward-echelon (§4) + back-sub (§4.6) vs `row_reduce`; proptest + rank-deficient generators | ✅ done (`blas3.rs`) |
| 1b | `fp` | Block the back-substitution (§4.6) into the same `L·U` GEMM shape (right-to-left over pivot rows) | next |
| 2 | `fp` | criterion bench; choose default `b`; measure M4RI crossover | gated by 1 |
| 3 | `fp-cuda` | Device-resident `DeviceMatrix` + persistent-buffer glue; `gemm_xor_into` fused epilogue (§6) | gated by 1,1b |
| 4 | `fp-cuda` | `panel_factor` kernel §5(1) (b=64) + virtual `perm` | gated by 3 |
| 5 | `fp-cuda` | Wire §4 to run entirely on device; bit-exact vs CPU blocked oracle | gated by 3,4 |
| 6 | `fp-cuda` | Active-row compaction (§8.2); panel-level GEMM §5(2) if profiled | gated by 5 |
| 7 | `fp` | Dispatch: `row_reduce` picks GPU path above a size/rank threshold, else M4RI | gated by 5 |

Phase 1 is landed and proptested against `row_reduce`. Phase 1b (blocking the
back-substitution) and Phase 2 (the bench) need no GPU and come next. Phases 3–6
are the residency port that turns the two constraints of §1 into the speedup.

### 10.1 Phase 2 bench results (CPU, `benches/reduce_blas3.rs`)

Measured on an AVX-512 host (so the GEMM is the tiled AVX kernel, not a scalar
fallback), half-rank inputs (`rank = n/2`, the target regime), criterion
median:

| n    | `row_reduce` (M4RI) | `row_reduce_blas3` | ratio |
|------|--------------------:|-------------------:|------:|
| 512  | 274 µs              | 3.28 ms            | 12×   |
| 1024 | 941 µs              | 14.8 ms            | 16×   |
| 2048 | 4.62 ms             | 64.3 ms            | 14×   |
| 4096 | 27.2 ms             | 307 ms             | 11×   |

Panel-width sweep at `n = 2048` (half-rank): **64 → 1024 all land at 64–69 ms**,
i.e. within noise of each other.

**Thread scaling** (half-rank `n = 2048`, `--features concurrent`, so the GEMM's
`maybe_rayon::join` is live; the earlier table above was built with `concurrent`
*off*, i.e. a serial GEMM — a methodology caveat):

| threads | `row_reduce` (M4RI) | `row_reduce_blas3` |
|---------|--------------------:|-------------------:|
| 1       | 4.68 ms             | 68.6 ms            |
| 4       | 4.77 ms (**flat**)  | 60.8 ms (**1.13×**)|

This is the crux of *why* the approach matters and *why* this prototype does not
yet show it. M4RI is a **sequential dependency chain** — build the `2^k` table,
reduce, advance — so it is flat in the core count and cannot be sped up by
throwing cores at it. `L·U` GEMM is **data-parallel** and is the whole reason
the GPU kernel exists. But blas3 only scales 1.13× here because the GEMM is a
**minority** of its runtime; the majority is serial work this prototype left
naive, and Amdahl's law caps the speedup at `1/serial_fraction`.

Which serial parts can actually be parallelized, and how:

- **Trailing GEMM** — already parallel via `maybe_rayon` (needs `concurrent`).
- **Back-substitution** — blocking it into the mirror `L·U` GEMM (Phase 1b)
  turns `O(R²·n)` serial row ops into a data-parallel GEMM. Biggest single lever
  for the parallel fraction.
- **Panel below-row elimination** — embarrassingly parallel across rows *in
  principle*, but only `≈ b/64` limbs of work per row per pivot column, so a
  per-column `rayon` fork/join is swamped by its own overhead. It parallelizes
  only if the whole panel is factored by a coarse limb-wise kernel (§5), not row
  by row. On CPU the bigger win is just making it limb-wise (drop the per-bit
  `entry()` accessor) to shrink the serial fraction; it stays hard to parallelize
  cheaply.
- **Pivot search** — a `find-first` reduction; parallelizable but fine-grained,
  same caveat.

Sober extrapolation: even fully blocked, on **4 cores** blas3 will not cross
M4RI at these *small* sizes — the crossover needs either many-core CPUs or the
GPU's thousands of lanes, which is exactly the regime the design targets. The
value of Phase 1b + a limb-wise panel on CPU is to raise the parallel fraction so
the *scaling trend* is visible (and to serve as the GPU oracle), not to win on a
4-core box.

### 10.2 Large-matrix trend — the regime that matters

`examples/reduce_scaling.rs`, release, half-rank, one-shot per size, 4-core
AVX-512 host, 4 threads:

| n     | M4RI    | blas3   | ratio  | M4RI vs prev | blas3 vs prev |
|-------|--------:|--------:|-------:|-------------:|--------------:|
| 4096  | 28 ms   | 299 ms  | 10.6×  | —            | —             |
| 8192  | 224 ms  | 1.12 s  | 5.0×   | 8.0×         | 3.7×          |
| 16384 | 1.92 s  | 8.16 s  | 4.2×   | 8.6×         | 7.3×          |
| 32768 | 24.8 s  | 76.0 s  | **3.07×** | **12.9×** | 9.3×          |

Two effects both favour blas3 as `n` grows, and both are visible here:

1. **The ratio collapses monotonically** (10.6× → 3.07×). blas3's GEMM amortizes
   better on larger operands while its fixed per-`entry()` panel overhead becomes
   a vanishing fraction.
2. **M4RI degrades superlinearly at 32768** — the step is 12.9×, well past the
   ~8× that `n³` predicts, because the 134 MB matrix overflows cache and M4RI's
   table method turns memory-bound. blas3's *tiled* GEMM tracks `n³` (9.3×) — it
   was built for the memory hierarchy. So large matrices punish M4RI twice: no
   parallelism **and** worse locality.

Extrapolating the ratio (~0.72× per doubling in the last step) puts the 4-core
crossover in the few-hundred-thousand range — and the real workload, 100 000
rows, is ~27× more work than 2^15, squarely into that regime. On the GPU
(thousands of lanes, HBM bandwidth) the crossover moves down by orders of
magnitude. This is the quantitative case for the whole approach: **M4RI is a
sequential, cache-bound dead-end at scale; the GEMM path is not.**

Caveats: single-shot (not averaged); blas3 is still slower than M4RI at every
size *measured on 4 cores*; and its serial `O(R²·n)` back-substitution (Phase 1b)
is an increasing drag — blocking it would bend the blas3 curve further down.

**Reading this honestly:** on CPU the blocked algorithm is ~an order of magnitude
*slower* than M4RI, and it is **not GEMM-bound** — the block-width independence is
the tell (if the trailing GEMM dominated, wider panels would move the number a
lot). The time is in the parts this prototype left naive:

1. **Panel factorization by per-`entry()` scanning.** Each pivot scans `O(m)`
   rows through the bit-at-a-time `entry()` accessor, and every *free* column
   (half of them, at half rank) triggers a full failed scan. That is `O(m·n)`
   scalar accessor calls — exactly the work M4RI avoids with its table, and
   exactly what §5's limb-wise panel kernel replaces on the GPU.
2. **Unblocked back-substitution** (§4.6), `O(R²·n)` row ops.

Neither is the GEMM. This is consistent with — not a counterexample to — the
thesis: the approach is a *GPU* technique. On CPU, M4RI's cache-friendly table
already extracts most of the available speed, and the modest AVX-GEMM advantage
cannot pay for the scalar panel/back-sub overhead. The win requires the GPU,
where the `wgmma.b1` GEMM is ~50–100× a CPU pass and the panel/back-sub costs
move on-device (§5) or get blocked (Phase 1b). **Do not route CPU `row_reduce`
to this path** (§10 Phase 7's threshold should stay GPU-only). Phase 1b + a
limb-wise CPU panel kernel would narrow the gap but are unlikely to beat M4RI on
CPU; their real purpose is to be the correctness oracle and the algorithmic
skeleton for the GPU port.

---

## 11. Summary

- Do **not** loop over `matmul_b1_raw`: it re-marshals the matrix every panel and
  makes PCIe the ceiling (§1.1). Keep the matrix in **one device buffer** and run
  a sequence of in-place kernels; bus traffic stays ≈ 2× the matrix (§6).
- Confine every **column** access to the narrow panel; express all wide work as
  **GEMM** or contiguous **row-XOR**; never move columns; make row swaps virtual
  via `perm` so the 1.25 GB buffer never shuffles (§1.2, §4.3).
- Use right-looking **blocked Gauss–Jordan** (PLE family): panel-factor →
  capture multipliers `L` → one trailing **GEMM** `L·U` XOR-ed in place (§4).
  Total cost `O(m·n·R)`, GEMM-dominated, **∝ rank** — the rank-½ workload is
  ~2× cheaper for free (§3), with compaction trimming the GEMM's tall dimension
  toward `R` (§8).
- Prove it on CPU against `row_reduce` before touching the GPU (§9).

### References

- M. Albrecht, G. Bard, C. Pernet — *Efficient Dense Gaussian Elimination over
  the Finite Field with Two Elements* (arXiv:1111.6549). The M4RI block-iterative
  **PLE**; the basis for §4/§7.
- J.-G. Dumas, C. Pernet, Z. Sultan — *Rank-profile revealing Gaussian
  elimination and the CUP matrix decomposition* (arXiv:1112.5717).
- *Fast matrix decomposition in F₂* (arXiv:1209.5198).
- J.-G. Dumas, C. Pernet — *Computational linear algebra over finite fields*
  (arXiv:1204.3735) — survey; reduction of elimination to matmul.
- *A Study on Optimization of Sparse and Dense Linear System Solver over GF(2)
  on GPUs*, Springer (LNCS) — GPU bit-packed blocked elimination.
