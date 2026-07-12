# C-motivic Ext: end-to-end implementation plan

Computing the **C-motivic Adams E₂ page** — `Ext` over the C-motivic mod-2
Steenrod algebra, `p = 2`, over `ℂ` — as a first-class computation that reuses
the existing `ext` engine without disturbing the classical path.

> Base: this branch starts fresh from `master`. The earlier exploratory work
> (a generic-ground-ring engine + `FpTau` `Solvable`) is on
> `73de90ee4671068b3a018640e993ad1a61f0d272` and is recoverable for porting the
> validated *algebra* (`motivic_milnor.rs`), which is the only piece we reuse.

> **Status (implemented).** Phases 0–4 are done. Phase 0: `algebra::motivic`
> (`Tau`, `MotivicMilnorAlgebra`, `CTauAlgebra`). Phase 1:
> `examples/resolve_motivic_ctau.rs` + `tests/motivic_ctau.rs`. Phases 2–3:
> `ext::motivic` (weight-homogeneous lift, `H(δ)`). Phase 4:
> `examples/resolve_motivic.rs` + the `examples/benchmarks/motivic-S_2` golden
> fixture. **All three anchors pass**: invert `τ` reproduces classical `Ext_A`
> rank-for-rank; `τ = 0` gives the algebraic Novikov ranks; keeping `τ` exhibits
> the `h₁`-tower `τ`-torsion (`h₁⁴`, `h₁⁵`). The classical path is untouched and
> the 69 resolution benchmarks stay bit-identical. One deviation from §2.5: the
> lift and `H(δ)` live in `ext::motivic` (they consume the resolution's
> quasi-inverses, populated by the `ext` engine) rather than the `algebra` crate;
> the pure `Tau` linear algebra could still be factored down later.

---

## 0. Success criteria (the three anchors)

The motivic E₂ is pinned by three views of one object, so a correct
implementation must reproduce all three:

1. **invert τ → classical Adams E₂.** Regress against
   `ext/examples/benchmarks/num_gens-S_2`.
2. **kill τ (τ = 0) → algebraic Novikov E₂.** The generator counts of the
   resolution; equivalently the motivic Adams E₂ of `Cτ` (Gheorghe–Wang–Xu).
3. **keep τ → motivic Adams E₂ with τ-torsion.** The full bigraded answer,
   including genuine τ-torsion classes (e.g. `h₁⁵` at `(s,t,w)`), regressed
   against a published C-motivic chart (Isaksen–Wang–Xu).

**Guardrail:** the classical path stays **bit-identical** — all 69 resolution
benchmarks unchanged. We achieve this by *not modifying the `ext` engine at all*
(see §2).

---

## 1. The mathematics we are implementing

Three algebras, not two. Let `A_C` be the C-motivic dual Steenrod algebra
(Voevodsky):

```
A_C = F₂[τ][ξ₁, ξ₂, …, τ₀, τ₁, …] / (τᵢ² = τ·ξᵢ₊₁),      |τ| = (0, −1)   (stem, weight)
```

- **`A_C`** — over `F₂[τ]`, bigraded by (stem, weight). Resolving the sphere
  `F₂[τ]` over it gives the **motivic Adams E₂**.
- **`A_C/τ`** — set `τ = 0`: `A_C/τ ≅ F₂[ξᵢ] ⊗ Λ(τᵢ)`. This is a *connected,
  finite-type `F₂`-algebra* (every generator has positive stem once τ is gone),
  and it is **strictly bigger than the classical `A_* = F₂[ξᵢ]`**. Its Ext is
  the **algebraic Novikov E₂**.
- **`A`** — the classical Steenrod algebra. A *further* collapse of `A_C`
  (invert τ, or run the algebraic Novikov SS off `A_C/τ`). This is what `ext`
  resolves today.

### Why this makes motivic first-class *and* fast

The minimal resolution over `A_C` is a genuine object over the motivic Steenrod
algebra. We compute it the way you compute over any τ-adically filtered ring —
**associated graded first, then deform** — which here means:

- **Leading term:** resolve `A_C/τ` over the field `F₂`. This is a *motivic*
  algebra (not classical `A`), resolved by the ordinary field engine. Fast.
- **Deform:** lift the differentials to `A_C` over `F₂[τ]`; the lifted `d∘d` is
  zero only mod τ, and a small τ-power (Gaussian-elimination) correction
  restores `d² = 0`. This is the honest `A_C` minimal resolution.

Because the mod-τ object is `A_C/τ` (motivic), this is **not** "compute
classical and correct" — both objects are motivic. First-class semantics, and
the heavy combinatorics live over a field. (This is Guozhen Wang's Adams–Novikov
algorithm, module side; see §7.)

### Representation: one `F₂[τ]`-matrix per stem, valuation entries

A homogeneous element of `F₂[τ]` in a fixed bidegree is exactly `τᵏ` for a
unique `k` — store it as the integer `k` (`τ`-valuation), with a sentinel for 0.
Homogeneity fixes every differential entry: generator `j` at weight `wⱼ` maps to
generator `i` at weight `wᵢ` via `τ^{wᵢ − wⱼ}`. So the resolution differential at
each stem is **one matrix over `F₂[τ]`, the same size as the classical `F₂`
matrix**, with integer entries — *not* a sequence of matrices per weight. The
τ-tower is stored once. Linear algebra over it is `F₂` elimination with graded-PID
pivoting (pick the lowest-valuation entry in a column) plus integer bookkeeping —
a small constant over `F₂`, never `×(#weights)`, never polynomial arithmetic.

### The E₂ is a cohomology, not a generator count

`Ext_{A_C}(F₂[τ], F₂[τ]) = H(Hom_{A_C}(F_•, F₂[τ]), δ)`, where
`Hom_{A_C}(F_s, F₂[τ]) = Vₛ*` is a **free `F₂[τ]`-module** (rank = # generators)
and `δ` is the **identity-operation part** of the resolution differentials —
apply the augmentation `A_C → F₂[τ]`, which kills positive Steenrod operations
and leaves the τ-power on the unit. Every entry of `δ` is `τᵏ` (or 0), and
`δ² = 0`. The three anchors are three operations on this one complex:

| operation on τ | result |
|---|---|
| invert τ | classical Adams E₂ |
| set τ = 0 | `δ ≡ 0` ⟹ generator counts = algebraic Novikov E₂ |
| keep τ, take `H(δ)` | motivic Adams E₂, with `F₂[τ]/τᵏ` torsion |

The first (exploratory) implementation printed the middle row — generator counts
— and never applied `δ`. The real work is `H(δ)`, organised efficiently by the
**τ-Bockstein spectral sequence / Curtis table** (Wang §6): leading-term
cycle/tag table, `a` = permanent cycle, `a→b` = differential whose *length* is
the τ-filtration difference (= torsion order). This keeps the heavy pass over
`F₂` while producing honest `F₂[τ]`-cohomology.

---

## 2. Architecture decisions (committed)

1. **Do not genericize the `ext` engine.** Motivic factors through the *field*
   engine via `A_C/τ`, so the engine stays `F₂`-only and classical stays
   bit-identical by construction. (This supersedes / closes PR #263 — the
   ground-ring genericization is unnecessary under this factoring.)
2. **Bigrading = internal degree `t` (engine) + weight carried alongside.**
   `A_C/τ` is single-`t`-graded with weight a bounded per-basis label; `A_C`
   carries weight in its `F₂[τ]` coefficients (the τ-tower). No ℤ²-graded engine.
3. **The deformation lives in `algebra`/the motivic layer, never in `ext`.**
4. **Module picture now; comodules deferred.** BP/Adams–Novikov would add a
   *parallel* comodule engine later (§8) — additive, not a rewrite.
5. **Crate placement.** New code in the `algebra` crate (algebras, coefficient
   rep, lift) plus, on the `ext` side, the cohomology (`H(δ)`) as a
   generalization of the existing `ExtAlgebra` bridge (§6), and a thin
   driver/example. The `ext` *resolution engine* and all classical modules are
   untouched; `ExtAlgebra` is *extended*, with the classical `δ = 0` path
   preserved.

---

## 3. Phase 0 — Coefficient ring and the two algebras

**Goal:** the `F₂[τ]` valuation ring and the algebras `A_C` (over `F₂[τ]`) and
`A_C/τ` (over `F₂`), with no resolution yet.

- `algebra/src/algebra/motivic/tau.rs` — `Tau`: homogeneous `F₂[τ]` as a
  `(coeff ∈ F₂, valuation ∈ u32)` with a zero sentinel. Ops: add (same
  valuation), multiply (add valuations), `valuation`, `shift`, `pow`. Vector /
  matrix helpers over `Tau` for the motivic layer only (small, self-contained;
  **not** an `fp` replacement threaded through `ext`).
- `algebra/src/algebra/motivic/milnor.rs` — port `MotivicMilnorAlgebra` from
  `73de90ee46` (validated: bigrading, Milnor product, coproduct, antipode,
  cross-checked against classical). Strip its dependence on the old
  `Ring`/`Solvable` tower; it needs only `Tau` arithmetic and the (stem, weight)
  bigrading.
- `A_C/τ`: expose the mod-τ reduction as an ordinary `F₂` `Algebra`
  (implements the existing `algebra::Algebra` trait), basis by internal degree
  `t`, each basis element tagged with its weight. This is the object `ext`
  resolves in Phase 1.

**Tests (port + extend):** product vs classical Milnor after inverting τ;
associativity; coproduct is an algebra map; antipode; `A_C/τ` basis dimensions
per (t, weight) against hand computation for small t.

---

## 4. Phase 1 — Resolve `A_C/τ` over `F₂` → algebraic Novikov E₂

**Goal:** first end-to-end numbers, using the **unmodified** `ext` engine.

- Present `A_C/τ` to `ext` as an `Algebra`; resolve the trivial module `F₂` over
  it with the existing minimal-resolution engine. Generators per (s, t) =
  algebraic Novikov E₂ ranks; carry the weight tag per generator (needed by the
  lift in Phase 2).
- `ext/examples/resolve_motivic_ctau.rs` — a `resolve`-style driver printing the
  `A_C/τ` chart.

**Milestone value:** a real, publishable computation (algebraic Novikov E₂ /
motivic Ext of `Cτ`) with essentially zero engine risk, and it validates the
`A_C/τ` algebra structure before the harder τ work.

**Tests:** generator counts contain the classical ranks (each classical class
survives); spot-check low-degree algebraic Novikov ranks against Wang/known
values; `d² = 0` in the `F₂` resolution (engine already guarantees this).

---

## 5. Phase 2 — Lift to `A_C` over `F₂[τ]` → the motivic minimal resolution

**Goal:** the honest minimal resolution over `A_C`, in the valuation rep.

- Take the Phase-1 `A_C/τ` resolution as the **model**: the generator skeleton
  (ranks, weights, and the mod-τ differentials) is fixed.
- Lift each differential entry from `A_C/τ` to `A_C` (choose the `F₂[τ]` lift of
  the mod-τ operation). Compute `d∘d`; it is `≡ 0 mod τ`, so it equals τ·(error).
  Solve the small `F₂[τ]` linear system (graded-PID elimination on valuation
  matrices) that adjusts the lift so `d² = 0` exactly — Wang §4–5. Optionally use
  the "project to cogenerators" size reduction later as an optimization.
- Store differentials as per-stem `Tau` matrices (§1 representation).

**Tests:** generator counts identical to Phase 1 (the lift preserves the
skeleton); `d² = 0` over `F₂[τ]` on every generator through a fixed range;
reducing the lifted differentials mod τ reproduces Phase 1 exactly.

---

## 6. Phase 3 — `H(δ)` in `ExtAlgebra` → the motivic Adams E₂

`ExtAlgebra` (`ext/src/ext_algebra/`) is already the intended **resolution → Ext
bridge**: it wraps a resolution and presents `Ext(M, k)` as a bigraded algebra.
Today that bridge is *trivial* — `dimension(b) = number_of_gens_in_bidegree(b)`
— because over a field the Hom-complex differential vanishes (`δ = 0`), so
`Ext = generators`, read off. The motivic case is the **same bridge finally doing
the real work**: `Ext = H(δ)`, with the classical field case (`δ = 0`) recovering
today's behavior. Phase 3 is therefore a *generalization of `ExtAlgebra`*, not a
new extractor.

**Goal:** teach `ExtAlgebra` that "Ext from a resolution" means taking cohomology,
then read the motivic E₂ off it.

- Give `ExtAlgebra` a δ-aware Ext: extract `δ` (the identity-operation, τ-power
  part of the resolution differentials — apply the augmentation `A_C → F₂[τ]`), a
  complex of free `F₂[τ]`-modules with `δ² = 0`, and compute `Ext = H(δ)` as
  f.g. `F₂[τ]`-modules (free ⊕ `F₂[τ]/τᵏ`). The current `dimension`/`basis`
  become the `δ = 0` specialization.
- Organize the cohomology by the τ-Bockstein Curtis table (Wang §6): leading-term
  cycle/tag, `diff_length` = τ-filtration difference = torsion order. This lands
  next to the existing `secondary` (d₂) submodule — the τ-BSS / algebraic-Novikov
  differentials are *more differentials on Ext*, the same family `ExtAlgebra`
  already hosts — so products, d₂, and Massey products keep operating on the
  motivic Ext through one surface.
- The two derived views fall out: τ = 0 (drop δ) = algebraic Novikov (Phase 1);
  invert τ (rank of δ over `F₂(τ)`) = classical.

**Tests — the three anchors:**
1. invert τ reproduces `num_gens-S_2` exactly, *including killing τ-torsion*
   (`h₁⁵` disappears).
2. τ = 0 reproduces the Phase-1 algebraic Novikov ranks.
3. the full E₂ matches a published C-motivic chart over a fixed range: the
   `h₁`-tower is τ-torsion-free/periodic as expected, and named τ-torsion classes
   (`h₁⁵` etc.) appear with the correct `F₂[τ]/τᵏ` order.

---

## 7. Phase 4 — Presentation, fixtures, docs

- `ext/examples/resolve_motivic.rs` — print the bigraded motivic E₂ (ASCII, or
  emit a SeqSee/JSON chart with weight and τ-torsion annotations).
- Golden fixtures for the motivic chart (small range) under
  `ext/examples/benchmarks/` so the anchors regress in CI.
- Doc pass: a `motivic` module doc explaining the three algebras, the three
  pages, and where the deformation lives; keep the model identifier out of any
  committed text.

---

## 8. Out of scope (future)

- **BP / Adams–Novikov.** Wang's algorithm is natively about `BP∗BP`-comodules;
  motivic is the `A_C/τ ↔ P` special case. Reaching BP means adding a *parallel*
  comodule / Hopf-algebroid resolver (cofree resolutions, cobar `δ`) that shares
  the coefficient layer (extended to p-adic `Z/pᵏ`) and the Curtis/SS extractor.
  Additive; the module engine here is untouched.
- **McTague's Steenrod-algebra data structure.** A faster cycle/tag reduction
  for the `F₂` skeleton (Phase 1). Slot it behind the leading-term-reduction
  interface if/when Phase 1 becomes the bottleneck; not on the critical path.

---

## 9. Reference

- G. Wang, *Computations of the Adams–Novikov E₂-term* (2020) — the algorithm
  (`MinimalResolution.pdf` in `joeybf/minimalresolution`; §4–5 model-and-lift,
  §6 Curtis table).
- Gheorghe–Wang–Xu, *The special fiber of the motivic deformation…*
  (arXiv:1809.09290) — algebraic Novikov E₂ = motivic Adams E₂ of `Cτ`.
- Isaksen–Wang–Xu — C-motivic charts for the anchor regressions.
