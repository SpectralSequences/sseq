# algebra experiment log

What we tried, what it gained, and why the rejected alternatives were rejected. This is the place
for that record — the code comments are not, and neither is the README, which describes the crate
as it is rather than how it got there.

## Hash-free Milnor basis indexing ("seqno")

`MilnorAlgebra::basis_element_to_index` is a `HashMap<MilnorBasisElement, usize>` lookup. The
table-based alternative, `MilnorAlgebra::seqno`, ranks a `p_part` by summing differences of a
precomputed `g` array, with no hash.

Which one wins depends on the degree, and the crossover is a cache effect. The hashmap is
per-degree, so its working set grows with the dimension of that degree and eventually falls out of
cache. The `g` table is shared across degrees and grows only linearly in the degree, so it stays
resident and its cost is flat. The hashmap wins while it is cache-resident and loses once it is
not; `benches/seqno.rs` sweeps a range of degrees that spans the crossover.

Because of that, `compute_basis` deliberately does not build the seqno tables: a resolution that
never reaches the crossover should not pay for them. Callers that want the hash-free index — a GPU
backend, or a high-degree CPU run — call `compute_seqno_tables` themselves.

### Rejected: `OnceVec<Vec<_>>` storage

**Rejected.** The first version stored the tables in a `OnceVec<Vec<usize>>`. That paid two atomics
*per table access*, which was enough to make the table lose to the hashmap at every degree measured.
Storing one flat, row-major `Vec` behind an `arc_swap::ArcSwapOption` reduced a read to a single
guard load followed by direct indexing.

### Rejected: re-deriving the degree inside `rank`

**Rejected.** `rank` could recover the degree as `Σ rᵢ·ξᵢ` instead of taking it as an argument, but
every caller already knows it, and the hashmap it competes with reads it straight off the basis
element. Re-deriving it would have put a loop in the measurement that the competing path does not
pay. It survives as a `debug_assert!`.

### Hoisting the `arc_swap` guard

**Kept.** `seqno` acquires the guard on every call, which is one atomic per lookup and pure overhead
in a loop that ranks many elements. `seqno_ranker` hoists the acquisition out of the loop; the
`seqno` vs `seqno_naive` gap in `benches/seqno.rs` is what that is worth.
