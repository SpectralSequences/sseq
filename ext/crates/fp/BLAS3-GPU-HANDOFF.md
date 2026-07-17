# BLAS3 F₂ row reduction — GPU handoff (for an H200-enabled agent)

> **STATUS (device port done, H200-validated).** Phases 3–5 and 7 are
> implemented and validated bit-exact on an H200 NVL (sm_90):
> - **Phase 3** — `DeviceMatrix` + persistent-buffer glue; `matmul_b1_dev`
>   (device-in/device-out GEMM, on-device packing) + `xor_into_region`/
>   `gemm_xor_into` fused epilogue. Gates: `matmul_b1_dev_demo`,
>   `gemm_xor_into_demo`.
> - **Phase 4** — `panel_factor` base kernel (b=64, virtual `perm`). Gate:
>   `panel_factor_demo` (vs a CPU transliteration; full- and low-rank panels).
> - **Phase 5** — `forward_reduce` (forward echelon) + `back_substitute` → full
>   RREF `row_reduce_dev`, all device-resident. Gates: `forward_reduce_demo`,
>   `row_reduce_demo` (`row_reduce_dev == fp::Matrix::row_reduce`, bit-exact).
> - **Phase 7** — `fp::Matrix::row_reduce` dispatches to the GPU above
>   `FP_CUDA_THRESHOLD`. Test: `tests/cuda_dispatch::gpu_row_reduce_matches_cpu`.
>
> Rough timing (`reduce_timing`, half-rank square, single-thread CPU baseline):
> device beats CPU BLAS3 from n≥2048 with a widening lead (1.5× @ 2048 → 3.4× @
> 8192). **Still open: Phase 6** — active-row compaction (§8.2), the wider-panel
> intra-panel GEMM (§5(2)), fusing the XOR into the GEMM store, and profiling the
> single-CTA panel/promote/back-sub kernels (they use 1 SM; lower-order today but
> the next optimization target). All build with CUDA 12.4 nvcc; `cargo run -p
> fp-cuda --example <name>` for each gate.

---

> **TL;DR.** The CPU algorithm is done, proptested bit-exact against
> `Matrix::row_reduce`, and benchmarked: on 4 CPU cores its cost crosses M4RI
> around `n ≈ 1.2·10⁵` (the real workload). Your job is the **device-resident
> port** — run the whole reduction as a sequence of kernels over *one* persistent
> GPU buffer, so PCIe transfer is paid twice (upload + download), not once per
> panel. The F₂ GEMM kernel you need already exists and is **H200-validated**;
> what's missing is a device-resident API around it and one new panel kernel.
>
> Read `BLAS3-ROW-REDUCTION.md` first — this file is the execution wrapper, that
> file is the design (§ references below point into it).

---

## 0. What's already true (don't redo)

- **CPU algorithm** (`src/matrix/blas3.rs`, `Matrix::row_reduce_blas3`): blocked
  forward-echelon + blocked back-substitution, limb-wise panel. Proptested
  bit-for-bit against `row_reduce` (RREF **and** pivots) incl. rank-deficient
  generators (`src/matrix/blas3.rs` tests + `proptest-regressions/`). **This is
  your executable spec and your oracle.**
- **F₂ GEMM kernel** (`fp_cuda_hopper` branch, `crates/fp-cuda`): Hopper
  `wgmma.b1` matmul, **validated on H200 NVL** (Phases 3–12), ~thousands of
  binary TOPS kernel-only. But its only entry point, `matmul_b1_raw`, does
  `clone_htod(A) + clone_htod(B) + alloc(C) + clone_dtoh(C)` **per call** — the
  crate's own README notes end-to-end throughput is "dominated by host
  marshalling." That per-call marshalling is exactly what you must NOT loop over
  (design §1.1).
- **Bench harness**: `examples/reduce_scaling.rs` (one-shot large sizes),
  `benches/reduce_blas3.rs` (criterion). Reuse for GPU numbers.

Current CPU trend (4 threads, half-rank, AVX-512 host), for context on what
"good" looks like — the GPU should blow past M4RI here:

| n | M4RI | `row_reduce_blas3` | ratio |
|---|---|---|---|
| 16384 | ~2 s | 4.0 s | 1.8× |
| 32768 | ~26–31 s | 45.9 s | 1.46× |

---

## 1. Repo setup (combine the two branches)

You need the CPU algorithm **and** the GEMM kernel in one tree:

- CPU algorithm: branch `claude/blas3-row-reduction-lgjz3y` (off `master`).
- GEMM kernel: branch `fp_cuda_hopper` (off `master`).

They're disjoint except `ext/crates/fp/Cargo.toml` (one adds a `[[bench]]`, the
other adds `fp-cuda` deps + a `cuda` feature — a trivial both-sides merge). Merge
`fp_cuda_hopper` into `claude/blas3-row-reduction-lgjz3y` (or a fresh integration
branch), resolve the `Cargo.toml` union, and you have `row_reduce_blas3` next to
`fp_cuda::matmul_b1_raw`.

Sanity gates before any new work (H200 box):

```sh
cargo build -p fp-cuda                       # PTX JITs on the device
cargo run -p fp-cuda --example matmul_b1_demo # CPU↔GPU bit-exact GEMM sweep
cargo test -p fp --features proptest blas3   # CPU oracle still green
```

If the GEMM demo is bit-exact, the kernel is sound on your H200 and every number
below has a trustworthy oracle.

---

## 2. The one rule

**One device allocation for the whole reduction.** Upload the matrix once, run
every step as an in-place kernel over that buffer, download once. Host↔device
traffic must stay ≈ 2× the matrix regardless of panel count. If you find yourself
calling `matmul_b1_raw` (which round-trips the host) inside the panel loop, stop
— that re-marshals overlapping slabs `n/b` times and PCIe becomes the ceiling
(design §1.1). Everything else follows from this.

Corollary constraints (design §1.2): confine all **column**-indexed work to the
narrow panel; make wide work either a GEMM or a contiguous whole-limb row-XOR;
never permute columns; and make row "swaps" **virtual** via a `perm` index vector
(design §4.3) so the multi-hundred-MB buffer never physically shuffles. (The CPU
code physically swaps rows — fine for the oracle, wrong at GPU scale. This is the
one place the GPU port intentionally diverges from `blas3.rs`.)

---

## 3. Device API to build in `fp-cuda` (design §6)

Add a handle-based, device-resident layer beside the existing one-shot function:

```rust
pub struct DeviceMatrix { /* CudaSlice<u64> + rows, cols, stride, on-device perm */ }

impl GpuContext {
    fn upload(&self, m: &Matrix) -> DeviceMatrix;      // one H2D
    fn download(&self, dm: &DeviceMatrix) -> Matrix;   // one D2H, applies perm

    // in-place device kernels over the persistent buffer:
    fn panel_factor(&self, dm, limb_lo, limb_hi, col_hi, r, /*out*/ l, pivcols) -> pr; // §5
    fn gemm_xor_into(&self, dm, factors, rows, cols, dst_limb); // wgmma.b1 + fused XOR epilogue
    fn gather_bits(&self, dm, rows, cols) -> DeviceMatrix;      // column-gather for X / back-sub
}
```

Two deltas from today's kernel do most of the work:

1. **Persistent buffers.** Reuse the *compute* core of `cuda_kernels/matmul_b1.cu`
   unchanged; change only the host glue so operands are device pointers into the
   resident buffer, not fresh `htod` copies.
2. **Fused XOR-accumulate epilogue.** Both trailing update (§4.4) and back-sub
   (§4.6) are `M[region] ^= factors · rows`. Have the GEMM store step XOR its
   result into an existing device region instead of writing a fresh C and copying
   it back — saves an alloc + a D2H per panel and does the update for free.

Everything the CPU driver does in `row_reduce_blas3` maps onto these: the forward
`L·U` trailing GEMM and the back-sub `X·U` GEMM both become `gemm_xor_into`.

---

## 4. The one genuinely new kernel: `panel_factor` (design §5)

The panel is `m × (b/64)` limbs — small (a few MB), stays in L2 across the sweep.
It's the only place with column-indexed / sequential work. Port the CPU Step A
loop (`blas3.rs`, the `for q in col_lo..col_hi` block) to a device kernel:

- **Base case `b = 64` (one limb wide):** each active row contributes one `u64`;
  factor by sweeping the 64 bit-positions — for bit `j`, a `find-first` reduction
  over `m` rows picks the pivot (mark its `perm` slot), then a **row-parallel
  masked XOR** clears bit `j` from every other row and records the multiplier
  into `L`. 64 coalesced passes over `m` words.
- **Wider `b` (256–1024):** split into 64-wide sub-panels, factor the leftmost
  with the base kernel, update the rest of the *panel* with a small GEMM, recurse
  (§5(2)). Start with the base kernel at modest `b`; add this only if the panel
  shows up in profiles.
- Emit to host only `pr` + the pivot columns (a few hundred ints/panel). `L`, the
  reduced pivot rows, and `perm` stay on device.

The CPU `blas3.rs` promotion/deferral logic (why forward-only elimination keeps
the pivot trailings stable) is documented at §4.6 — preserve that structure; it's
what makes the deferred GEMM correct.

---

## 5. Order of work + validation gates (design §10, Phases 3–7)

Each phase has a bit-exact oracle: the GPU result, downloaded, must equal
`row_reduce_blas3` on the same input (which is itself `== row_reduce`).

| Phase | Deliverable | Gate |
|---|---|---|
| 3 | `DeviceMatrix` + persistent-buffer glue; `gemm_xor_into` fused epilogue | a single trailing-update GEMM matches CPU Step B bit-for-bit |
| 4 | `panel_factor` base kernel (b=64) + virtual `perm` | one panel factorization matches CPU Step A (pivots + `L`) |
| 5 | wire the §4 driver end-to-end on device (forward + blocked back-sub) | full `row_reduce_blas3` GPU == CPU, bit-exact, across the shape grid |
| 6 | active-row compaction (§8.2); wider panel GEMM (§5(2)) if profiled | still bit-exact; measure speedup |
| 7 | dispatch in `fp`: `row_reduce` picks GPU above a size/rank threshold | pick threshold from the crossover (§10.5) |

Recommended micro-oracles while building: extend `examples/reduce_scaling.rs` to
run the GPU path beside `row_reduce_blas3` and `assert_eq!` the full matrices, at
sizes from 64 up to 2^15+. Bit-exactness at small sizes catches ~every kernel
bug before the big runs.

---

## 6. Risks / gotchas

- **Marshalling creep** (§2) — the failure mode. Audit that the panel loop issues
  zero `htod`/`dtoh` beyond the initial upload and the scalar pivot-column
  read-backs.
- **`perm` indirection** — every kernel dereferences `M[perm[i]]`. Get this
  uniform early; retrofitting it is painful.
- **Padding is your friend** (design, "WLOG pad to a limb"): columns are stored
  padded to a limb boundary and rows to a multiple of 64; treat the padding as
  genuine zeros so the last partial limb needs no special case. The CPU code
  already relies on this.
- **GEMM operand layout** — `matmul_b1.cu` expects a specific TMA/bit-pack layout
  (`af251a5`, the n256 fragment). `gemm_xor_into` must feed `L`/`U`/`X` in that
  layout from device memory; reuse `matmul_b1_raw`'s packing logic, just without
  the host round-trip.
- **fp-cuda's own `HANDOFF.md`** documents an older Phase 8–9 cluster/multicast
  batch; the README says Phases 8–12 are since H200-validated. If a GEMM shape
  regresses, that's the kernel's bisect trail, not yours.
- **Rank deficiency (§8)** — the workload is rank ≈ m/2. Work is already ∝ rank
  (empty panels cost ~0); add active-row compaction (§8.2) via `perm` partition
  so the GEMM's tall dimension shrinks toward R. It's a `perm` reshuffle, no byte
  movement.

---

## 7. H200 specifics

- Compute capability sm_90a — the kernel targets exactly this; already validated
  on H200 NVL. No kernel changes needed to *run*.
- 141 GB HBM3e, ~4.8 TB/s. A 100 000² F₂ matrix is ~1.25 GB — fits comfortably
  with room for the panel/`L`/`U`/`G` temporaries, so the whole reduction can be
  device-resident with no tiling of the *matrix* itself.
- The kernel's known throughput cliff was L2-residency of B beyond 16384 (fp-cuda
  README/HANDOFF); the row-reduction GEMMs are tall-skinny × wide (`m × pr` ·
  `pr × T`), a different shape — profile them specifically, and lean on the
  Phase 8–12 rasterization/cluster work already in the kernel.

---

## 8. Definition of done

1. `row_reduce_blas3` runs entirely on the H200 over one resident buffer,
   bit-exact vs the CPU version at every tested size (64 … ≥ 2^15).
2. Host↔device traffic is ≈ 2× the matrix (verify with a profiler / counters),
   independent of panel count.
3. On the target regime (n ~ 10⁵, rank ~ n/2) the GPU path is decisively faster
   than M4RI *and* than CPU `row_reduce_blas3`; report the numbers next to the
   §10.4 table.
4. `row_reduce` dispatches to it above a size/rank threshold (§10.5), M4RI below.

The CPU side is the finished skeleton and the oracle; you're moving it onto
silicon that's ~1000× more parallel than the 4 cores that already put the
crossover at the real workload size.
