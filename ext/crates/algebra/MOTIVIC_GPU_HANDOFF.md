# Handoff: CubeCL kernel for the C-motivic Milnor multiply (ℂ, p = 2)

A self-contained brief for building a GPU kernel that offloads the product in the
C-motivic Steenrod algebra `A_C` (and its mod-τ reduction `A_C/τ`). It mirrors the
classical Nassau handoff (`GPU_KERNEL_HANDOFF.md`, branch `nassau_gpu`) and reuses
as much of that machinery as the math allows. Written from a **CPU-only container
that cannot run GPU code**, so everything a GPU box needs — the exact CPU
reference to port, the output encoding, the validation oracle — is captured here.

The motivic layer lives on branch **`claude/awesome-davinci-khkr8x`** of
`JoeyBF/sseq` (`ext/` workspace), in `ext/crates/algebra/src/algebra/motivic/`.
Pull that first. Read the classical `GPU_KERNEL_HANDOFF.md` alongside this — the
device plumbing (CubeCL toolchain, batching boundary, atomic-XOR output, hash-free
indexing, staging, gotchas) is shared and not repeated in full here.

---

## 0. TL;DR — what to build, and the two facts that make it tractable

Build a CubeCL kernel that computes, in one launch, the batch of `Q(E₁)P(R₁) · s`
products the motivic resolution issues at a differential-assembly step (the same
"fixed operation × general element" unit as classical; §3). Port the CPU
**two-matrix closed form** `multiply_closed` (Kong–Lin Theorem 5.1, ρ = 0) in
`motivic/milnor.rs`. Two facts collapse most of the apparent extra complexity:

1. **The device output stays F₂ atomic-XOR — the τ never touches it.** Every
   surviving `(X, Y)` pair contributes coefficient exactly `1`, and by weight-
   homogeneity a fixed output monomial `z` carries a *single, determined* τ-power
   `k(z)` — the weight difference between `a·b` and `z` (magnitude
   `|w(a)+w(b)−w(z)|`; `bidegree(...).1` uses a negated-weight convention, so pin
   the exact expression against `multiply_closed`, which the §7 oracle checks). So
   the kernel computes **F₂ presence** of each output index (atomic XOR, identical
   to classical); the τ-power is a host-side weight lookup, not part of the device
   accumulator. **You do not need `Tau` arithmetic on the GPU.**

2. **The resolution's products (the bottleneck) are the mod-τ product, which is
   odd-primary-Milnor-structured at p = 2 — so the classical admissible-matrix
   kernel ports almost directly.** `A_C/τ ≅ 𝔽₂[ξᵢ] ⊗ E(τᵢ)` has the odd-primary
   dual's shape (with `2ⁱ` powers, not `pⁱ`), and mod τ the X-matrix constraint
   collapses from `S(X) ≤ R₁` to `S(X) = R₁` exactly — **the classical admissible
   matrix**. The only additions over the classical kernel are the second matrix
   `Y` (the `τᵢ` / Q-part interaction) and `2ⁱ` weights. The *full* `A_C` product
   (with τ, needed only by the Phase-2 lift) needs both matrices and the τ-power.

So: start from the classical `nassau_gpu` kernel, keep its admissible-matrix X
enumeration and atomic-XOR output, add the Y enumeration and the weight table.
Validate bit-for-bit against `multiply_closed`.

---

## 1. Environment reality

Same as the classical handoff §1 — everything there applies verbatim (CubeCL
`0.10` + `wgpu` compiles headless; `cubecl-cpu` needs a network fetch that was
403-blocked in the sandbox but works on a normal box and gives a GPU-less
correctness backend; pick `wgpu`/`cuda`/`hip` per hardware; ignore the sandbox's
`-C target-cpu=native` SIGILL). **Nothing in this doc was executed on a GPU.**

---

## 2. Where things live (branch `claude/awesome-davinci-khkr8x`)

Key file: **`ext/crates/algebra/src/algebra/motivic/milnor.rs`**

| Item | Symbol | Line (approx.) |
|---|---|---|
| **CPU reference to port** | `multiply_closed` (returns `DualElement`; the fuzz-validated product) | ~851 |
| X enumeration (ξ-matrix) | `Closed::enum_x` | ~702 |
| X→Y bridge (S′, RY targets, T(X)) | `Closed::on_x` | ~734 |
| Y enumeration (τ/Q-matrix) | `enum_y` | ~788 |
| Incremental b / T / S state | `struct Acc` | ~ (near `Closed`) |
| τ-rewrite coefficient `c(S, R)` | `c_coeff` | ~82 |
| Column enumerators | `columns_eq`, `columns_le`, `col0_x_options` | ~588, ~609, ~632 |
| Buffer bound (rows/anti-diagonals) | `const NB` (= 64) | ~656 |
| Product used by the engine | `MotivicMilnorAlgebra::product_indexed` | (returns `Vec<(Tau, usize)>`) |
| Basis index (binary search) | `index_of`, `index_of_ref` | ~945 |
| **Weight of a basis element** | `MotivicMilnorAlgebra::bidegree` (`.1` = weight) | ~950 |
| Product cache (replace on GPU) | `products: RwLock<FxHashMap<…>>` | struct field |
| CPU profiling counters | `PRODUCT_HITS/MISSES/NANOS` (env `MOT_PROFILE`) | ~52 |

Mod-τ reduction (**the resolution algebra**):
**`ext/crates/algebra/src/algebra/motivic/ctau.rs`**

| Item | Symbol | Line |
|---|---|---|
| `A_C/τ` product = τ⁰ part of `A_C` | `CTauAlgebra::multiply_basis_elements` | ~98 |
| Weight label per basis element | `CTauAlgebra::weight` | ~64 |
| Underlying `A_C` engine | `CTauAlgebra::engine` | ~55 |

Coefficient ring (host-side only; **not** needed on device): `motivic/tau.rs`
(`Tau` = a τ-valuation, `Option<u32>`).

Full-product consumer (Phase-2 lift): **`ext/src/motivic/mod.rs`** —
`MotivicResolution::lift`/`correct`/`composite` call `engine().product_indexed`
and *do* use the τ-powers. Driver + profiling: `ext/examples/resolve_motivic.rs`.

Validation oracle (already in the crate): `multiply` (duality product) in
`motivic/milnor.rs`, and the test `test_closed_form_matches_duality_oracle`.

---

## 3. The call chain the products flow through

Two distinct consumers, different needs:

```
Phase 1 — RESOLUTION of A_C/τ  (the bottleneck: ~97% of run time; MOD-τ products)
  Resolution engine assembles a differential
    └ module action → CTauAlgebra::multiply_basis_elements(out, coeff, R_deg, R_idx, s_deg, s_idx)
         └ engine.product_indexed(...)  → multiply_closed(a, b)  [FULL A_C product]
         └ keep only terms with τ-valuation 0   (drop τ-divisible)   ← mod-τ

Phase 2 — LIFT to A_C  (correcting d²=0; FULL products with τ-powers)
  MotivicResolution::correct → composite → engine.product_indexed(...) → multiply_closed
```

The fundamental kernel unit is the classical one: **a fixed operation
`a = Q(E₁)P(R₁)` × a general element `s` (a sum of `Q(E₂)P(R₂)` terms) → add
`a·s` to an F₂ output slice.** `a` repeats across many `s` in one assembly step —
that is the amortization a workgroup exploits (one workgroup per distinct `(E₁,
R₁)`; parallelize the term tests across `s`).

> **Batching boundary — determine on the GPU box.** The classical kernel batches
> per Nassau `get_partial_matrix` (a proven-maximal, non-redundant unit; classical
> handoff §5). The motivic resolution currently uses the **standard** `Resolution`
> engine (`ext/src/motivic/mod.rs`, via `Resolution::new`), not Nassau, so the
> exact assembly boundary that maximizes same-`a` amortization must be located
> there (it is wherever a differential column is built from many algebra actions).
> Two options: (a) find/instrument the standard engine's equivalent of
> `get_partial_matrix`; or (b) port the motivic resolution to the Nassau
> signature algorithm and inherit its boundary and the classical §5 analysis
> wholesale. (b) is the cleaner long-term path and the one that reuses the most.

> **CPU-side batch primitive already in place.** `MotivicMilnorAlgebra` now caches
> products as one dense [`ProductBlock`] per topological-degree pair `(t1, t2)`
> (`entry (idx1, idx2) = basis[t1][idx1] · basis[t2][idx2]`), with per-entry
> `OnceLock` fill. `MotivicMilnorAlgebra::fill_block(t1, t2)` computes an entire
> block in one (parallel) pass — this is the ready-made host handle for the GPU
> launch: hand the kernel a `(t1, t2)` block, get all `dim1 × dim2` structure
> constants back, store into the block's cells.
>
> **But do not fill whole blocks blindly.** Measured block *density* during a real
> resolution is only **~50%** (stem 30 and 40 both ≈ 50%): the standard engine
> requests roughly half of each touched block's `(idx1, idx2)` grid. So a GPU (or
> CPU) pass that fills the *entire* block does ~2× the necessary
> `multiply_closed` work — and since product compute is ~95% of resolution time,
> eager full-block fill is a **net CPU loss** (confirmed by measurement; that is
> why the CPU path stays lazy-per-entry). For the GPU this means either (i) pass
> the kernel only the *requested* `(idx1, idx2)` sublist of the block (keep the
> engine's demand-driven set), accepting a ragged batch; or (ii) fill the full
> block and eat the 2× overcompute, betting GPU throughput makes it a wash. Decide
> with an A/B on the GPU box; the ragged-batch path (i) is the safer default and
> mirrors what the lazy CPU cache already does.

---

## 4. The math to port (`multiply_closed`, ρ = 0)

`Q(E₁)P(R₁) · Q(E₂)P(R₂)` is a sum over **two** non-negative integer matrices
`X` (the ξ-part) and `Y` (the τ/Q-part), Kong–Lin Theorem 5.1 at ρ = 0. Sequences
are indexed from 0; **ξ-type** sequences (`R₁, R₂, S(X), R(X), T(X)`) drop index 0
(ξ₀ = 1), **τ-type** sequences (`E, S(Y), T(Y)`) keep it (τ₀ ≠ 1). This ξ₀ = 1 / τ₀
≠ 1 asymmetry is the whole subtlety — get it wrong and terms silently vanish.

Matrix functionals (per matrix `M`, entries `x_{i,j}`, i = row, j = column):
- `S(M)_r = Σⱼ x_{r,j}` (row sum), `R(M)_j = Σᵢ 2ⁱ x_{i,j}` (column weighted sum),
  `T(M)_r = Σ_{i+j=r} x_{i,j}` (anti-diagonal sum).
- `b(M) = Πᵣ multinomial(anti-diagonal r) mod 2` = **1 iff the entries on each
  anti-diagonal are bitwise disjoint** (no carries). The CPU keeps a running XOR
  per anti-diagonal and **rejects a column the instant an entry collides**, so
  only `b = 1` matrices are ever visited and `b`/`T`/`S` are never recomputed
  (`struct Acc`, `enum_x`/`enum_y`). Port this pruning — it is what makes the
  enumeration cheap and it maps to per-anti-diagonal bit masks exactly like the
  classical `masks[j]`.

Enumeration (see `enum_x` → `on_x` → `enum_y`):
- **X** (`enum_x`): column j ≥ 1 ranges over `Σᵢ 2ⁱ x_{i,j} ≤ R₂[j]`; column 0 over
  `x_{i,0} ≤ R₁[i]`; prune on the row budget `S(X)_r ≤ R₁[r]` (r ≥ 1) and on
  b-collision. **For the mod-τ product the row budget is an equality
  `S(X) = R₁` — i.e. the classical admissible matrix of `Sq(R₁)`.**
- **`on_x`**: `S′ = R₁ − S(X)` (ξ-part, index 0 = 0); τ-power `k = Σ(S′)`; the Y
  column targets `R(Y)[j] = R₂[j] − R(X)[j]` (j ≥ 1) and the forced
  `R(Y)[0] = Σ₂(E₁) + Σ₂(S′) − Σ_{j≥1} R(Y)[j]`; output P-part `T(X)` (drop index 0).
- **Y** (`enum_y`): each column an exact weighted sum `R(Y)[j]`, pruned on
  b-collision. At a leaf the pair contributes iff the ρ = 0 degree equation
  `popcount(E₁) + 2Σ(S′) = Σ(S(Y))` holds **and** `c(S(Y), S′) ≠ 0` (the τ-rewrite
  coefficient `c_coeff`, Theorem 3.4 — pure integer/`binomial2` arithmetic, a
  direct GPU port). Output monomial: `Q(E₂ + T(Y)) P(T(X))`, and it must be
  square-free (`E₂ + T(Y) ∈ Seq₁`).

**Coefficient and τ (the §0 fact 1 in detail).** Each surviving `(X, Y)`
contributes `b(X)·b(Y)·c(S(Y),S′)·binom(E₂+T(Y), E₂) = 1` — every factor is 0/1 and
we only reach the leaf when all are 1. So the coefficient of an output monomial is
`#{surviving (X,Y) landing on it} mod 2` — **an F₂ count = atomic XOR**, exactly
classical. The τ-power `k = Σ(S′)` is the *same* for every `(X,Y)` landing on a
given `z` (weight-homogeneity; `k = Σ(S′)`), so recover it host-side from the
weight table (`bidegree(...).1`, sign calibrated against `multiply_closed`) rather
than tracking it on device. **Mod-τ (resolution): keep only `z` with `k(z) = 0`**
— equivalently only enumerate `X` with `S(X) = R₁`.

**Special case:** `a = 1` (E₁ = 0, R₁ = ∅) ⇒ output is `s` unchanged. Common
(identity operation).

---

## 5. Batching / occupancy

The classical §5 analysis (batch per assembly launch, one workgroup per distinct
operation, ~60% of term-work fills a 256-wide workgroup on same-operation
amortization at high stems, "widen the scope buys nothing") **carries over** —
the kernel unit is identical. Re-measure for the motivic workload once the
boundary of §3 is fixed; the motivic algebra is ~2× larger and the algebraic-
Novikov E₂ has more generators, so expect *denser* launches than classical at the
same stem (more `s` terms per operation), which only helps occupancy.

CPU baseline to beat (this sandbox, `--features concurrent`, ~3–4 cores; use for
harness sanity, not as an absolute target): report `(40, 8)` resolution ≈ 2.1 s,
`(50, 10)` ≈ 18 s; the closed-form product is ~62 µs/call at stem 40, ~157 µs at
stem 50, and is ~100% of resolution time (linear algebra is ~3% — **the product is
the bottleneck, exactly as classical**). Regenerate with:

```
MOT_PROFILE=1 cargo run --release --no-default-features --features concurrent \
    --example resolve_motivic          # prints resolution / products / lift / cohomology
```

`PRODUCT_HITS/MISSES/NANOS` (in `motivic/milnor.rs`) report the closed-form call
count and time; the `MOT_PROFILE` env gate is read in `MotivicResolution::new`.

---

## 6. Data layout for the device

Upload once per algebra (rebuild as `compute_basis` grows):

- **Basis table + index.** The `A_C/τ` (= `A_C`) basis in a degree is a contiguous
  sorted array of monomials `(e_mask: u32, R: [u32; MAX_XI])`, sorted by
  `(e_mask, R)`. `index_of_ref` is a **binary search** over it — directly GPU-
  portable (no hashing), unlike the classical `HashMap`. That is the simplest
  device index and is enough to start. A hash-free *`seqno`-style* rank (classical
  handoff §6) is a later optimization: the motivic rank factors as
  `(rank of E among Seq₁ subsets at that weight) × (classical-seqno rank of R)` —
  more arithmetic than classical but still hash-free; only build it if the binary
  search shows up in the profile.
- **Weight table.** One `i32` per basis element = `bidegree(degree, idx).1`. The
  host multiplies each present output index `z` by `τ^{k(z)}`, `k(z)` the weight
  difference (magnitude `|w(a)+w(b)−w(z)|`; fix the sign once against
  `multiply_closed`). For the mod-τ (resolution) kernel, keep only the `k(z) = 0`
  slice (`w(z) = w(a)+w(b)`) and the output is pure F₂ — no weight multiply at all.
- **Per-launch inputs.** For each product: the operation `(E₁, R₁)` (fixed within a
  workgroup) and `s` as a list of `(E₂, R₂)` terms (or indices into the shared
  basis table), each padded to `MAX_XI` `u32`s for coalesced access.
- **Output.** F₂ vectors bit-packed into `Atomic<u32>` arrays, one per matrix row;
  `atomic_xor(word, 1 << bit)` at `idx = index_of_ref(z)`. Match the crate's `fp`
  p = 2 bit order (LSB-first within a limb — verify against `fp`). Multiple
  `(X, Y)` pairs, possibly across threads, land on the same index and must cancel
  mod 2 — **atomic XOR, not add** (classical gotcha §10 applies identically).

---

## 7. Validation & benchmarking (on the GPU box)

**Correctness oracle — already in the crate.**
`test_closed_form_matches_duality_oracle` (`motivic/milnor.rs` tests) checks
`multiply_closed` bit-for-bit against the independent duality product `multiply`
for **every** basis pair up to `deg(a)+deg(b) ≤ 18`, including the τ-carrying
(`Sq²·Sq² ∋ τQ₀Q₁`) and multi-Q cases. **Mirror it for the kernel:** for every
`(a, s)` in a degree range, compare the kernel's F₂ output slice (per output
index, i.e. the presence bits) against `multiply_closed(a, bⱼ)` reduced to
presence, and separately check the host-rebuilt τ-powers against
`multiply_closed`'s `Tau` coefficients. With the `cubecl-cpu` runtime this runs
without a GPU. Add a mod-τ variant comparing against
`CTauAlgebra::multiply_basis_elements`.

**End-to-end A/B.** CPU baseline is the current default (`multiply_closed` behind
the `RwLock` cache). Feature-flag the GPU path in the resolution's product call,
keep device buffers resident across the resolution (upload only the degrees that
grew), and A/B `resolve_motivic` wall time at e.g. `(50,10)` and `(60,10)`. The
GPU must beat the CPU path *including* host↔device transfer.

---

## 8. Suggested staging (each stage compiles + validates before the next)

1. **Toolchain slice.** `cubecl` behind a `gpu` feature on `algebra` (classical
   handoff §9.1 skeleton — a trivial F₂-XOR `#[cube]` kernel + host launch + test).
2. **Index on device.** Port `index_of_ref` as a `#[cube]` binary search over the
   uploaded sorted basis; test it is the identity `index_of_ref(basisₖ) == k` over
   a full degree.
3. **Single-`(E₁,R₁)` mod-τ multiply.** One workgroup: enumerate the classical
   admissible matrices of `Sq(R₁)` (`S(X) = R₁`) — reuse the classical
   `AdmissibleMatrix` port from `nassau_gpu` — then the Y (Q-part) enumeration and
   the `c_coeff`/ρ leaf test, atomic-XOR the F₂ output. Validate one `(a, s)`
   against `CTauAlgebra::multiply_basis_elements`.
4. **Full `A_C` multiply.** Relax `S(X) = R₁` to `S(X) ≤ R₁`, emit at all
   τ-powers, rebuild `Tau` on the host from the weight table. Validate against
   `multiply_closed` (§7 oracle).
5. **Batched launch + wire into the resolution + A/B** (§3 boundary, §5 occupancy,
   §7 A/B). Keep buffers resident.

---

## 9. Gotchas (motivic-specific; the classical §10 all still apply)

- **ξ₀ = 1 but τ₀ ≠ 1.** ξ-sequences compare on indices ≥ 1, τ-sequences on ≥ 0.
  This is the single most error-prone point — the naive Theorem 5.1 reading drops
  the `Q₀·P(ξ₁) → Q₀P(ξ₁)` term because it treats `S(X)`'s index 0 as constrained.
  The CPU reference and its oracle test pin it; keep the test.
- **τ never goes on the device.** Coefficients are F₂ presence; the τ-power is a
  weight-table lookup on the host. Don't build `Tau` arithmetic into the kernel.
- **Two matrices, but Y is small.** X is the classical admissible matrix; Y
  handles only the `τᵢ` interaction and is typically tiny (often trivial when
  E₁ = E₂ = 0 and the product is pure-ξ). Enumerate X per workgroup (as classical);
  the Y loop is a short inner sweep per `(X, term)`.
- **Mod-τ vs full.** The resolution wants `S(X) = R₁` (τ⁰) — a *restriction* of the
  full enumeration, so a mod-τ-only kernel is strictly smaller. Ship it first (it
  is the bottleneck); the full product is only for the Phase-2 lift.
- **Square-free output.** `E₂ + T(Y)` must be ∈ Seq₁ (each τᵢ at most once); a
  collision kills the term. Q indices fit a `u32` mask.
- **`c_coeff` needs `r₀ = 0`.** `S′` has index 0 = 0 by construction; assert it.
- **Trailing-zero canonicalization.** Output P-parts are trimmed before indexing;
  the sorted basis assumes trimmed monomials (same as classical §10).

---

*Written from a sandbox without GPU access. The math, the CPU reference, and the
oracle are exercised on CPU (all `motivic` tests green); nothing here ran on a
GPU. Validate stage-by-stage on real hardware, starting from the classical
`nassau_gpu` admissible-matrix kernel — most of it is reused.*
