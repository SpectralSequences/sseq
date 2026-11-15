# groebner

Minimal resolutions over Steenrod algebras by Gröbner bases. It supports every kind of Milnor
algebra in the `algebra` crate:

| kind                         | prime | algebra                                   |
|------------------------------|-------|-------------------------------------------|
| `SteenrodKind::Classical`    | 2     | $\mathcal{A}$                             |
| `SteenrodKind::Classical`    | odd   | $\mathcal{A}$, with the $Q_k$             |
| `SteenrodKind::MotivicModTau`| 2     | $\mathcal{A}^{\mathbb{C}}/\tau$, the odd-primary form at 2 |
| `SteenrodKind::ReducedPowers`| odd   | the reduced powers $\mathcal{P}$          |

No linear algebra is involved: everything is rewriting of sparse module elements.

This crate is a port of Weinan Lin's [SSeqCpp](https://github.com/WayneLin92/SSeqCpp), which is
licensed under the Apache License 2.0. The code is loosely based on it: the method, the packing of
monomials, the Gebauer–Möller pruning and the x2m criterion follow Lin, but the code is written
anew in Rust rather than translated line by line (but `pairs.rs` follows Lin's `AddToBuffers` and `Minimize` very
closely).

## How it works

Every module element is a sparse sum of monomials $Q(E) P(R) g$, where $g$ is a generator. A
monomial is packed into a `u128` (`ModMono`): from the top down, the complement of the index of
$g$, the May weight, the exterior part and the $p$-adic digits $d_{i,n}$ of $R$, one field per
$\xi_n^{p^i}$, ordered by degree. Coefficients are kept in a separate array of bytes. Integer
comparison is the monomial order, and the leading term of a sum is its smallest term. With this
order the leading term of a product is the product of the leading terms in the associated graded
algebra

$$E[Q_0, Q_1, \dots] \otimes \bigotimes_{i,n} \mathbb{F}_p[x_{i,n}]/(x_{i,n}^p)$$

whenever that is non-zero (see the `leading_terms_multiply` test). Divisibility, gcd and lcm there
are digitwise comparisons, done on the whole word at once.

The `DIGIT_BITS` bits of digits are split into fields of equal width, handed out in order of increasing
degree; this bounds the degrees a `Params` supports (`Params::max_degree`). At $p = 2$ a field is
one bit, and the digitwise operations are plain bit operations. At an odd prime a field has
$\lceil \log_2 p \rceil + 1$ bits, the top one a guard bit that is zero in every valid monomial.
Additions and subtractions then cannot carry across fields, and the guard bits record per field
whether $d_a + d_b \ge p$ (the product vanishes) or $d_a \ge d_b$ (divisibility, gcd, lcm).

Level $s$ of a `Resolution` is a Gröbner basis of the graph of $d \colon C_{s+1} \to C_s$, ordered
by the $C_s$ component. In each internal degree $t$:

1. Each level hands out its critical pairs of degree $t$: pairs of leading terms on the same
   generator, and *singles* $x \cdot g$ for $x$ in the annihilator of the leading term of $g$. The
   pairs are pruned by the Gebauer–Möller criteria when they are formed.
2. Installed filters, and Lin's x2m criterion if enabled, may discard pairs or mark them for
   auditing.
3. Each pair is reduced, in parallel. The products $u \cdot g$ of basis elements by monomials
   that the reductions need recur across pairs, so each is computed once per degree and shared
   between threads, until the pairs that can use it are done.
4. Going down from the top level, a result whose $C_s$ component survives is a new basis element.
   Otherwise its $C_{s+1}$ component is a cycle: it is either a boundary or the differential of a
   new generator of $C_{s+2}$.

The module being resolved enters through generators and relations. The `ext` crate turns a finite
module into such a presentation (`ext::groebner`), and `Resolution::sphere` presents the ground
field directly.

### x2m pruning

`Resolution::enable_x2m` turns on Lin's criterion. Every basis element carries a shadow `x2m` in
the associated graded algebra of the May filtration (where $\xi_n^{p^i}$ has filtration $n$ and
$Q_k$ has $k + 1$): an element of a free module on generators attached to the elements whose
leading parts were new. When the leading part of a reduction cancels, its shadow is a relation
among those leading parts, and the relations are kept in a Gröbner basis over the associated
graded algebra. A critical pair whose shadow reduces to zero modulo the relations and the other
pairs of its degree is redundant. Lin uses this at $p = 2$. The audit finds no wrong prediction at
any prime or kind tested, and enforcing it leaves the results unchanged.

## Experiments

`Resolution::stats` counts, for every level and degree, the pairs formed, pruned, reduced, and what
became of them. A reduction is *redundant* (`Outcome::is_redundant`) when skipping it would not have
changed the resolution: both components reduce to zero (`useless`), the cycle it produces bounds
one found in the same degree (`boundary`), or the cycle lies beyond the largest homological degree
(`beyond`). Predicting redundant pairs is the point of filters.

A `PairFilter` sees a `PairContext`: the pair, the leading terms, the generator, the degree, and
derived data such as the lcm and the *depth*. The depth is the smallest $j$ such that every monomial
involved lies in the span of the $\xi_n^{p^i}$ with $i + n \le j$, which at $p = 2$ means the pair
lives in $A(j-1)$. Install a filter with `Resolution::add_filter` in one of two modes:

- `FilterMode::Audit` reduces every pair anyway, after the pairs no filter objects to, records
  whether each prediction was right, and warns about any pair predicted redundant that was not. `Resolution::filter_report` prints the
  record.
- `FilterMode::Enforce` discards the pairs. This changes the result if the filter is wrong.

`FnFilter` turns a closure into a filter. A `PairObserver` sees every reduction. `PairLog` writes
them as JSON lines, ready for analysis in a notebook.

```sh
# The sphere at p = 2, with s <= 20, n <= 40, auditing x2m, and a log of every pair.
X2M=audit cargo run --release -p groebner --example sphere -- 2 classical 20 40 pairs.jsonl
# Any finite module, via the ext crate.
GROEBNER_PAIR_LOG=pairs.jsonl cargo run --release --example groebner_resolve
```

With the `groebner` feature of `ext`, every example of `ext` resolves with this algorithm.
`GROEBNER_KIND=motivic` or `GROEBNER_KIND=reduced` selects the other kinds of algebra, and
`GROEBNER_X2M=audit` or `GROEBNER_X2M=enforce` turns on x2m pruning.

## Tests

The unit tests compare the product with `MilnorAlgebra` for every kind, and check the leading term
property. `ext/tests/milnor_vs_groebner.rs` compares whole resolutions with the standard ones.
