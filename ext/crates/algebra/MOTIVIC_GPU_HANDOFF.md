# GPU port notes: the C-motivic Milnor multiply (ℂ, p = 2)

The C-motivic product (`multiply_closed`, Kong–Lin Theorem 5.1 at ρ = 0, in
`motivic/milnor.rs`) is the arithmetic bottleneck of the deformation pipeline and
the natural GPU-offload target. This note records the two facts that make a kernel
tractable and where the batch boundary is; the device plumbing (CubeCL toolchain,
atomic-XOR output, hash-free indexing) mirrors the classical Nassau kernel and is
not repeated here.

## The batch unit

`MotivicMilnorAlgebra::fill_block(t1, t2)` computes an entire `ProductBlock` — every
structure constant `Q(E₁)P(R₁) · Q(E₂)P(R₂)` for a pair of topological degrees — in
one shot, filling independent `OnceLock` cells in parallel. That block is the shape a
GPU kernel would fill in a single launch: the resolution asks for the same
degree-pair block repeatedly, so the per-degree-pair block is the batching boundary.

## Two facts that collapse the apparent complexity

1. **The device output stays F₂ atomic-XOR — τ never touches it.** Every surviving
   matrix pair `(X, Y)` contributes coefficient exactly `1`, and by weight-homogeneity
   a fixed output monomial `z` carries a single, determined τ-power — the weight
   difference between `a·b` and `z`. So the kernel computes only **F₂ presence** of
   each output index (identical to the classical kernel); the τ-power is a host-side
   weight lookup, not part of the device accumulator. No `Tau` arithmetic on the GPU.
   (Pin the exact τ-power expression against `multiply_closed` — `bidegree(...)` uses a
   negated-weight convention.)

2. **The bottleneck products are the mod-τ product, which is odd-primary-Milnor
   structured at p = 2.** `A_C/τ ≅ 𝔽₂[ξᵢ] ⊗ E(τᵢ)` has the odd-primary dual's shape
   (with `2ⁱ` powers, not `pⁱ`), and mod τ the X-matrix constraint collapses from
   `S(X) ≤ R₁` to `S(X) = R₁` — exactly the classical admissible matrix. The only
   additions over the classical kernel are the second matrix `Y` (the `τᵢ`/Q-part
   interaction) and the `2ⁱ` weights. The full `A_C` product (with τ, needed only by
   the Phase-2 lift) needs both matrices plus the τ-power.

## Validation

Validate any kernel bit-for-bit against `multiply_closed`, which is itself checked
exhaustively against the duality oracle `multiply` in the crate's tests. Start from
the classical admissible-matrix X enumeration, add the Y enumeration and the weight
table.
