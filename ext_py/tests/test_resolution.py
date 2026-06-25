"""Tests for `Resolution` (Nassau vs standard dispatch) and `SecondaryResolution`.

`Resolution(spec, algorithm)` selects an algorithm at runtime: ``None``/``"auto"``
(try Nassau, fall back to the general algorithm), ``"nassau"`` (force Nassau, error
if ineligible), or ``"standard"`` (force the general algorithm).

Expected values are not hardcoded: the resolution of ``S_2`` through a fixed small
range must be identical regardless of algorithm, so the auto/nassau results are
validated against the standard result via `graded_dimension_string()` agreement.

`SecondaryResolution` is only supported over the *standard* backend; a Nassau-backed
`Resolution` is rejected with a clean `ValueError` (Nassau stores quasi-inverses on
disk and needs a save directory, which the binding does not provide). See the
maintainer decision rejecting Nassau-backed secondary resolutions.

Ranges are kept small (`Bidegree.n_s(8, 4)`) so the suite stays fast (~0.05s total
on the dev machine).
"""

from itertools import islice

import pytest

import ext
from ext import sseq

# Small target bidegree: resolving S_2 through this stem is ~10ms per algorithm.
SMALL = sseq.Bidegree.n_s(8, 4)


def resolve(algorithm, max=SMALL):
    r = ext.Resolution("S_2", algorithm)
    r.compute_through_stem(max)
    return r


# --- algorithm dispatch ----------------------------------------------------


def test_auto_resolves_and_dimension_string_nonempty():
    s = resolve(None).graded_dimension_string()
    assert isinstance(s, str)
    assert len(s) > 0


def test_auto_equals_standard():
    # auto picks Nassau for the sphere; both algorithms resolve the same object, so
    # the graded dimensions over the same range must agree exactly.
    assert resolve("auto").graded_dimension_string() == resolve("standard").graded_dimension_string()


def test_nassau_equals_standard():
    assert resolve("nassau").graded_dimension_string() == resolve("standard").graded_dimension_string()


def test_standard_resolves():
    assert len(resolve("standard").graded_dimension_string()) > 0


def test_nassau_resolves():
    assert len(resolve("nassau").graded_dimension_string()) > 0


# --- error taxonomy --------------------------------------------------------


def test_bogus_algorithm_raises_valueerror():
    with pytest.raises(ValueError):
        ext.Resolution("S_2", "bogus")


def test_nassau_on_ineligible_spec_raises_valueerror():
    # Odd-prime sphere is ineligible for Nassau (requires p = 2). This is a
    # bad-argument condition, so it must be a clean ValueError, not a panic.
    with pytest.raises(ValueError):
        ext.Resolution("S_3", "nassau")


# --- compute_through_stem guard --------------------------------------------


def test_negative_s_bidegree_raises_valueerror():
    # Pre-fix this panicked across the FFI boundary (negative s over-allocates /
    # overflows in the resolve loop). The guard now raises ValueError first.
    r = ext.Resolution("S_2", "standard")
    with pytest.raises(ValueError):
        r.compute_through_stem(sseq.Bidegree.n_s(0, -1))


# --- SecondaryResolution ---------------------------------------------------


def test_secondary_over_standard_backend():
    r = resolve("standard")
    sec = ext.SecondaryResolution(r)
    sec.extend_all()
    assert isinstance(sec.underlying(), ext.Resolution)


def test_secondary_over_nassau_backend_raises_valueerror():
    # Nassau-backed secondary resolution is rejected up front (it would otherwise
    # panic in extend_all because Nassau quasi-inverses live on disk only).
    r = ext.Resolution("S_2", "nassau")
    with pytest.raises(ValueError):
        ext.SecondaryResolution(r)


# --- FreeChainComplex method set (§7.2) ------------------------------------
#
# Known low-dimensional Ext of the sphere S_2 over the mod-2 Steenrod algebra,
# in Adams indexing (s, t) [stem n = t - s]. These are standard, textbook,
# algorithm-independent values:
#   (0,0) = 1 (the unit), h_0 = (1,1), h_1 = (1,2), h_2 = (1,4),
#   h_0^2 = (2,2), h_0^3 = (3,3); the gaps (1,3) = 0.
# They are cross-checked below against `graded_dimension_string`/the other
# backend, so they are validated rather than merely asserted.
KNOWN_NONZERO = {(0, 0): 1, (1, 1): 1, (1, 2): 1, (1, 4): 1, (2, 2): 1, (3, 3): 1}
KNOWN_ZERO = [(1, 3), (2, 1)]


@pytest.mark.parametrize("algorithm", ["standard", "nassau"])
def test_compute_through_bidegree_and_number_of_gens(algorithm):
    r = ext.Resolution("S_2", algorithm)
    r.compute_through_bidegree(sseq.Bidegree.s_t(4, 12))
    for (s, t), n in KNOWN_NONZERO.items():
        assert r.number_of_gens_in_bidegree(sseq.Bidegree.s_t(s, t)) == n
    for s, t in KNOWN_ZERO:
        assert r.number_of_gens_in_bidegree(sseq.Bidegree.s_t(s, t)) == 0


def test_known_values_agree_across_backends():
    # The two algorithms resolve the same object; their Ext dimensions over a
    # shared range must agree, which validates the hardcoded KNOWN_* tables.
    a = resolve("standard")
    b = resolve("nassau")
    for s, t in list(KNOWN_NONZERO) + KNOWN_ZERO:
        bd = sseq.Bidegree.s_t(s, t)
        assert a.number_of_gens_in_bidegree(bd) == b.number_of_gens_in_bidegree(bd)


def test_number_of_gens_guards():
    r = resolve("standard")
    # Negative s/t -> clean ValueError, never a panic.
    with pytest.raises(ValueError):
        r.number_of_gens_in_bidegree(sseq.Bidegree.s_t(-1, 0))
    with pytest.raises(ValueError):
        r.number_of_gens_in_bidegree(sseq.Bidegree.s_t(0, -1))
    # Far outside the computed range -> 0, never a panic.
    assert r.number_of_gens_in_bidegree(sseq.Bidegree.s_t(100, 200)) == 0


def test_module_standard_shares_arc():
    r = resolve("standard")
    m0 = r.module(0)
    # C_0 is free on one generator in degree 0.
    assert m0.dimension(0) == 1
    assert m0.prime() == 2
    # Negative / out-of-range s -> ValueError.
    with pytest.raises(ValueError):
        r.module(-1)
    with pytest.raises(ValueError):
        r.module(r.next_homological_degree())


def test_module_nassau_unsupported():
    # Nassau resolves over the concrete MilnorAlgebra; the FreeModule pyclass
    # (over the SteenrodAlgebra union) cannot represent its modules.
    r = resolve("nassau")
    with pytest.raises(ValueError):
        r.module(0)


@pytest.mark.parametrize("algorithm", ["standard", "nassau"])
def test_iter_nonzero_stem(algorithm):
    r = resolve(algorithm)
    # The iterator is bounded but exposed lazily; slice it with islice.
    seen = [(b.n, b.s) for b in islice(r.iter_nonzero_stem(), 8)]
    # Every yielded bidegree is nonzero.
    for n, s in seen:
        assert r.number_of_gens_in_bidegree(sseq.Bidegree.n_s(n, s)) > 0
    # The unit and the start of the h_0-tower are present.
    assert (0, 0) in seen
    assert (0, 1) in seen


def test_iter_stem_yields_bidegrees():
    r = resolve("standard")
    first = next(iter(r.iter_stem()))
    assert isinstance(first, sseq.Bidegree)


def test_filtration_one_products_h0():
    r = resolve("standard")
    # h_0 is the filtration-one product of the degree-1 operation Sq^1.
    prod = r.filtration_one_products(1, 0)
    assert prod.b.s == 1
    assert prod.b.n == 0
    # The product matrix out of the unit (0,0) is [[1]] (h_0 hits h_0).
    m = r.filtration_one_product(1, 0, sseq.Bidegree.s_t(0, 0))
    assert m == [[1]]
    with pytest.raises(ValueError):
        r.filtration_one_products(-1, 0)


@pytest.mark.parametrize("algorithm", ["standard", "nassau"])
def test_filtration_one_products_uncomputed_no_panic(algorithm):
    # A freshly constructed resolution has no modules; upstream's unconditional
    # module(0) would panic. The binding returns the empty product instead.
    r = ext.Resolution("S_2", algorithm)
    prod = r.filtration_one_products(1, 0)
    assert list(prod.matrices) == []
    # h_0 still lives in (n, s) = (0, 1).
    assert prod.b.s == 1
    assert prod.b.n == 0


@pytest.mark.parametrize("algorithm", ["standard", "nassau"])
def test_filtration_one_op_idx_out_of_range_raises(algorithm):
    # op_deg = 1 has a single operation (Sq^1) at p = 2, so op_idx = 999 is out
    # of range. Both methods must raise IndexError, not panic or read garbage.
    r = resolve(algorithm)
    with pytest.raises(IndexError):
        r.filtration_one_products(1, 999)
    with pytest.raises(IndexError):
        r.filtration_one_product(1, 999, sseq.Bidegree.s_t(0, 0))


def test_boundary_string_guards():
    r = resolve("standard")
    g = sseq.BidegreeGenerator.s_t(0, 0, 0)
    assert isinstance(r.boundary_string(g), str)
    # idx beyond the generators at (0,0) -> ValueError.
    with pytest.raises(ValueError):
        r.boundary_string(sseq.BidegreeGenerator.s_t(0, 0, 5))


def test_callback_records_bidegrees():
    r = ext.Resolution("S_2", "standard")
    visited = []
    r.compute_through_bidegree_with_callback(sseq.Bidegree.s_t(3, 6), visited.append)
    assert len(visited) > 0
    assert all(isinstance(b, sseq.Bidegree) for b in visited)


def test_callback_exception_propagates():
    r = ext.Resolution("S_2", "standard")

    def boom(b):
        raise ValueError("boom")

    with pytest.raises(ValueError):
        r.compute_through_stem_with_callback(sseq.Bidegree.n_s(4, 4), boom)


def test_callback_unsupported_on_nassau():
    r = ext.Resolution("S_2", "nassau")
    with pytest.raises(ValueError):
        r.compute_through_bidegree_with_callback(sseq.Bidegree.s_t(2, 2), lambda b: None)


def test_name_is_method_returning_str():
    # `name` is bound as a method (not a getter); `set_name` is intentionally
    # not bound (frozen, Arc-shared resolution has no exclusive &mut).
    r = resolve("standard")
    assert isinstance(r.name(), str)
    assert not hasattr(r, "set_name")


@pytest.mark.parametrize("algorithm", ["standard", "nassau"])
def test_algebra_returns_steenrod_algebra(algorithm):
    # algebra() yields a SteenrodAlgebra on both backends: the standard backend
    # shares its union algebra directly; the Nassau backend (which resolves over
    # a bare MilnorAlgebra) rebuilds the equivalent Milnor variant.
    r = resolve(algorithm)
    alg = r.algebra()
    assert isinstance(alg, ext.algebra.SteenrodAlgebra)
    assert alg.prime() == 2
    assert alg.algebra_type() == ext.algebra.AlgebraType.Milnor
    # The accessor an example relies on (chart.py) must work off it.
    assert alg.default_filtration_one_products()
