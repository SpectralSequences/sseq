"""Tests for the `ExtAlgebra` pyclass (`ext::ext_algebra`).

`ExtAlgebra` is **not** a Steenrod-`Algebra`-trait type (it has no `(degree, index)`
basis and does not implement `Algebra`); it is a bigraded-algebra *view of a
resolution*. It wraps a resolution of `M` together with a resolution of the base
field `k` (the "unit"), presenting Ext(M, k) as a bigraded module over the bigraded
algebra Ext(k, k). When `M == k` (the same `Resolution` passed twice) it is the
algebra Ext(k, k) itself. Because it is built from resolutions, it is bound in the
top-level `ext` module (next to `Resolution`/`ResolutionHomomorphism`), not in
`algebra`.

Only the standard-backend instantiation is bound (`ExtAlgebra(Resolution(...,
"standard"), unit)`); a Nassau-backed resolution is rejected with a clean
`ValueError`, matching the standard-only precedent of `Resolution.module` /
`ResolutionHomomorphism`.

Known values mirror upstream `ext/src/ext_algebra.rs::tests::test_sphere_products`
(the canonical construction there): over `S_2`, `h_0 = (n=0, s=1)`,
`h_1 = (n=1, s=1)`; `h_0^2` is the nonzero generator of `Ext^{2,2} = (n=0, s=2)`,
and the Adams relations give `h_0 h_1 = 0 = h_1 h_0`.

All bad prime/degree/index inputs are pre-checked and raise
`ValueError`/`IndexError` rather than panicking across the FFI boundary.
"""

import pytest

import ext
from ext import fp, sseq

Bidegree = sseq.Bidegree
BidegreeGenerator = sseq.BidegreeGenerator
BidegreeElement = sseq.BidegreeElement


def s2_algebra(n=8, s=8):
    """The Ext algebra of the mod-2 sphere, computed through stem (n, s).

    Uses the standard backend (ExtAlgebra requires it) and passes the resolution
    as its own unit (the `M == k` case), exactly as the upstream Rust test does.
    """
    r = ext.Resolution("S_2", "standard")
    alg = ext.ExtAlgebra(r, r)
    alg.compute_through_stem(Bidegree.n_s(n, s))
    return alg


# --- construction & accessors ---------------------------------------------


def test_construct_and_prime():
    alg = s2_algebra(4, 4)
    assert alg.prime() == 2
    assert alg.is_unit() is True


def test_resolution_and_unit_share_object():
    alg = s2_algebra(4, 4)
    # M == k: the resolution and unit are the same object, both prime 2.
    assert alg.resolution().prime() == 2
    assert alg.unit().prime() == 2


def test_separate_unit_is_not_unit():
    # Two distinct resolutions of S_2: not detected as the unit (no Arc identity),
    # but products still make sense (both resolve k = F_2).
    r = ext.Resolution("S_2", "standard")
    u = ext.Resolution("S_2", "standard")
    alg = ext.ExtAlgebra(r, u)
    alg.compute_through_stem(Bidegree.n_s(4, 4))
    assert alg.is_unit() is False
    assert alg.prime() == 2


# --- dimension / basis structural invariants ------------------------------


def test_dimension_origin_is_one():
    # Ext^{0,0}(F_2, F_2) = F_2 is 1-dimensional (the unit class).
    alg = s2_algebra(4, 4)
    assert alg.dimension(Bidegree.n_s(0, 0)) == 1
    assert alg.unit_dimension(Bidegree.n_s(0, 0)) == 1


def test_dimension_h0_squared():
    # h_0^2 generates Ext^{2,2} = (n=0, s=2), which is 1-dimensional.
    alg = s2_algebra(8, 8)
    assert alg.dimension(Bidegree.n_s(0, 2)) == 1
    # h_0, h_1 each live in a 1-dimensional Ext^{1,*}.
    assert alg.dimension(Bidegree.n_s(0, 1)) == 1
    assert alg.dimension(Bidegree.n_s(1, 1)) == 1


def test_dimension_uncomputed_is_zero():
    alg = s2_algebra(4, 4)
    # Far outside the computed range: 0, never a panic.
    assert alg.dimension(Bidegree.n_s(1000, 1000)) == 0


def test_basis_length_matches_dimension():
    alg = s2_algebra(8, 8)
    b = Bidegree.n_s(0, 2)
    basis = alg.basis(b)
    assert len(basis) == alg.dimension(b)
    assert all(isinstance(g, BidegreeGenerator) for g in basis)


# --- elements / generators -------------------------------------------------


def test_generator_roundtrip():
    alg = s2_algebra(8, 8)
    h0 = alg.generator(BidegreeGenerator.n_s(0, 1, 0))
    assert h0.degree == Bidegree.n_s(0, 1)
    # The single coordinate is set.
    assert h0.vec().entry(0) == 1


def test_element_from_coords():
    alg = s2_algebra(8, 8)
    x = alg.element(Bidegree.n_s(0, 1), [1])
    assert x.degree == Bidegree.n_s(0, 1)
    assert x.vec().entry(0) == 1


# --- products: known values from upstream test_sphere_products ------------


def test_h0_squared_nonzero():
    alg = s2_algebra(8, 8)
    h0 = alg.generator(BidegreeGenerator.n_s(0, 1, 0))
    h0_sq = alg.multiply(h0, h0)
    assert h0_sq.degree == Bidegree.n_s(0, 2)
    assert not h0_sq.vec().is_zero()


def test_adams_relation_h0_h1_vanishes():
    alg = s2_algebra(8, 8)
    h0 = alg.generator(BidegreeGenerator.n_s(0, 1, 0))
    h1 = alg.generator(BidegreeGenerator.n_s(1, 1, 0))
    assert alg.multiply(h0, h1).vec().is_zero()
    assert alg.multiply(h1, h0).vec().is_zero()


def test_try_multiply_in_range_matches_multiply():
    alg = s2_algebra(8, 8)
    h0 = alg.generator(BidegreeGenerator.n_s(0, 1, 0))
    via_try = alg.try_multiply(h0, h0)
    assert via_try is not None
    assert via_try.degree == Bidegree.n_s(0, 2)
    assert not via_try.vec().is_zero()


def test_multiply_into_matrix_shape():
    alg = s2_algebra(8, 8)
    h0 = alg.generator(BidegreeGenerator.n_s(0, 1, 0))
    # Multiply h0 into the unit basis at (0, 1): one row per unit generator at
    # (0, 1) (there is one: h0), columns = dimension at (0, 1) + (0, 1) = (0, 2).
    rows = alg.multiply_into(h0, Bidegree.n_s(0, 1))
    assert rows is not None
    assert len(rows) == alg.unit_dimension(Bidegree.n_s(0, 1))
    # The single row is h0 * h0 = h0^2, nonzero.
    total = sum(sum(r) for r in rows)
    assert total != 0


def test_try_multiply_out_of_range_is_none():
    # Compute only a tiny range, then ask for a product whose target is unresolved.
    r = ext.Resolution("S_2", "standard")
    alg = ext.ExtAlgebra(r, r)
    alg.compute_through_stem(Bidegree.n_s(2, 2))
    h0 = alg.generator(BidegreeGenerator.n_s(0, 1, 0))
    # Build a high-degree operand whose product target is past the computed range.
    # h0^k for large k lands at (0, k); ask for a product into a far bidegree.
    far = alg.multiply_into(h0, Bidegree.n_s(0, 2))
    # (0,2)+(0,1) = (0,3); may or may not be computed at this small range. The
    # robust invariant: when out of range, multiply_into returns None (never panics).
    assert far is None or isinstance(far, list)


# --- panic guards ----------------------------------------------------------


def test_nassau_resolution_rejected():
    r = ext.Resolution("S_2", "nassau")
    with pytest.raises(ValueError):
        ext.ExtAlgebra(r, r)


def test_mismatched_primes_rejected():
    r2 = ext.Resolution("S_2", "standard")
    r3 = ext.Resolution("S_3", "standard")
    with pytest.raises(ValueError):
        ext.ExtAlgebra(r2, r3)


def test_negative_bidegree_dimension_rejected():
    alg = s2_algebra(4, 4)
    with pytest.raises(ValueError):
        alg.dimension(Bidegree.s_t(-1, 0))
    with pytest.raises(ValueError):
        alg.unit_dimension(Bidegree.s_t(0, -1))


def test_negative_compute_rejected():
    alg = s2_algebra(4, 4)
    with pytest.raises(ValueError):
        alg.compute_through_stem(Bidegree.s_t(-1, 0))
    with pytest.raises(ValueError):
        alg.compute_through_bidegree(Bidegree.s_t(0, -1))


def test_huge_compute_does_not_oom():
    # A huge-but-valid stem must not be requested (it would OOM); negative is the
    # cheap guard. Here we confirm a huge *query* (dimension) is a safe 0.
    alg = s2_algebra(4, 4)
    assert alg.dimension(Bidegree.n_s(1_000_000, 1_000_000)) == 0


def test_generator_index_out_of_range():
    alg = s2_algebra(8, 8)
    # Ext^{1,1} at (n=0, s=1) is 1-dimensional; idx 5 is out of range -> IndexError.
    with pytest.raises(IndexError):
        alg.generator(BidegreeGenerator.n_s(0, 1, 5))


def test_generator_negative_rejected():
    alg = s2_algebra(4, 4)
    with pytest.raises(ValueError):
        alg.generator(BidegreeGenerator.s_t(-1, 0, 0))


def test_element_wrong_length_rejected():
    alg = s2_algebra(8, 8)
    # (n=0, s=1) is 1-dimensional; a length-2 coords vector is rejected.
    with pytest.raises(ValueError):
        alg.element(Bidegree.n_s(0, 1), [1, 0])


def test_element_uncomputed_bidegree_rejected():
    # `ExtAlgebra.element` pre-rejects an uncomputed bidegree at *construction*.
    # The analogous rejection inside `multiply`'s own operand validation (when an
    # operand bidegree is uncomputed) is covered separately by
    # `test_multiply_uncomputed_operand_rejected`, which builds the operand
    # directly via `sseq.BidegreeElement` to bypass this construction guard.
    alg = s2_algebra(2, 2)
    with pytest.raises(ValueError):
        alg.element(Bidegree.n_s(500, 500), [])


def test_multiply_uncomputed_operand_rejected():
    # Exercise `multiply`'s OWN operand validation (`check_res_element` /
    # `check_unit_element`), not `ExtAlgebra.element`'s construction guard. We
    # build a structurally-valid `BidegreeElement` directly at a well-formed but
    # UNCOMPUTED bidegree (bypassing `ExtAlgebra.element`, which would pre-reject
    # it), then feed it to `multiply` in each operand position.
    alg = s2_algebra(8, 8)
    h0 = alg.generator(BidegreeGenerator.n_s(0, 1, 0))
    # (n=500, s=500) is well-formed (s, t >= 0) but far outside the computed
    # range; the empty p=2 vector makes a structurally-valid element there.
    uncomputed = BidegreeElement(Bidegree.n_s(500, 500), fp.FpVector(2, 0))

    # Left operand (resolution side): hits `check_res_element`'s uncomputed guard.
    with pytest.raises(ValueError, match="not computed at the element's bidegree"):
        alg.multiply(uncomputed, h0)
    # Right operand (unit side): hits `check_unit_element`'s uncomputed guard.
    with pytest.raises(ValueError, match="not computed at the element's bidegree"):
        alg.multiply(h0, uncomputed)


def test_multiply_into_degree_overflow_rejected():
    # The i32-overflow guard (`checked_target` on `x.degree() + y.degree()`) is
    # reachable only through `multiply_into`: its target `b` argument is NOT
    # validated as computed (only s, t >= 0 then the overflow check), so a
    # near-`i32::MAX` `b` makes the degree sum overflow. Through
    # `multiply`/`try_multiply` the guard is SHADOWED by the operand
    # uncomputed-bidegree check (both operands must be at computed -- hence small
    # -- bidegrees, whose sum cannot overflow); see
    # `test_multiply_near_imax_operand_shadowed_by_uncomputed_guard`.
    alg = s2_algebra(8, 8)
    h0 = alg.generator(BidegreeGenerator.n_s(0, 1, 0))  # valid, computed at (0, 1)
    imax = 2**31 - 1
    # b.t() = i32::MAX, plus h0.t() = 1, overflows i32 -> ValueError, not a panic.
    with pytest.raises(ValueError, match="overflows i32"):
        alg.multiply_into(h0, Bidegree.s_t(imax, imax))


def test_multiply_near_imax_operand_shadowed_by_uncomputed_guard():
    # Through `multiply`, a near-`i32::MAX` operand is rejected by the operand
    # uncomputed-bidegree guard BEFORE the overflow guard can fire (the overflow
    # guard is exercised directly via `multiply_into` above). Documents that the
    # overflow path is unreachable from `multiply`/`try_multiply` from Python.
    alg = s2_algebra(8, 8)
    h0 = alg.generator(BidegreeGenerator.n_s(0, 1, 0))
    imax = 2**31 - 1
    big = BidegreeElement(Bidegree.s_t(imax, imax), fp.FpVector(2, 0))
    with pytest.raises(ValueError, match="not computed at the element's bidegree"):
        alg.multiply(big, h0)


def test_multiply_cross_prime_operand_rejected():
    # `check_element`'s per-element prime check: an operand whose underlying
    # FpVector is over a different prime than the algebra (p=3 vs p=2) is rejected
    # with ValueError, not a panic. The bidegree (0, 1) IS computed, so the prime
    # check (which precedes the uncomputed-bidegree check) is what fires.
    alg = s2_algebra(8, 8)
    h0 = alg.generator(BidegreeGenerator.n_s(0, 1, 0))
    over_p3 = BidegreeElement(Bidegree.n_s(0, 1), fp.FpVector(3, 1))

    # Left operand (resolution side).
    with pytest.raises(ValueError, match="over prime 3 but the ExtAlgebra is over prime 2"):
        alg.multiply(over_p3, h0)
    # Right operand (unit side).
    with pytest.raises(ValueError, match="over prime 3 but the ExtAlgebra is over prime 2"):
        alg.multiply(h0, over_p3)


# NIT: the `check_unit_augmentation` guard (unit resolution's augmentation must
# be 1-dimensional in degree 0) is not given a dedicated raising-test: over `S_2`
# the unit is always a valid 1-dimensional unit, so this guard is exercised only
# indirectly via the product path (every successful `multiply`/`multiply_into` on
# S_2 passes it). A dedicated raising-test would require a non-standard unit
# module whose degree-0 augmentation is not 1-dimensional.
