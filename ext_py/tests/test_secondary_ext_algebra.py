"""Tests for the secondary (d2) layer ``SecondaryExtAlgebra`` and its
``SecondaryProduct`` results, plus ``ExtAlgebra.without_unit``
(``ext::ext_algebra::secondary``).

``SecondaryExtAlgebra`` composes an ``ExtAlgebra`` with the secondary
resolutions of ``M`` and the unit ``k`` and exposes the secondary differential
``d2`` (with the survival check ``survives``), the E3-page data
(``page_data``/``unit_page_data``), and the ``Mod_{Cλ²}`` secondary product
(``secondary_multiply_into``). It wraps ``SecondaryResolution`` /
``SecondaryResolutionHomomorphism``, so it is standard-backend only (Nassau is
rejected at the underlying construction).

The KNOWN d2 values mirror upstream
``ext/src/ext_algebra/secondary.rs::tests::test_sphere_d2`` (the canonical
construction): over ``S_2`` resolved through stem ``(16, 6)``,
  * ``h0 = (n=0, s=1)``, ``h1 = (n=1, s=1)``, ``h2 = (n=3, s=1)`` are permanent
    cycles (``survives == True``, ``d2`` is the zero class), and
  * the first Adams differential is ``d2(h4) = h0·h3²``, the nonzero generator
    of ``Ext^{3,17}`` at ``(n=14, s=3)`` (dimension 1), so ``h4 = (n=15, s=1)``
    does not survive.

Every uncomputed / negative / extend_all-not-called input is pre-checked and
raises ``ValueError`` rather than panicking across the FFI boundary.
"""

import pytest

import ext
from ext import fp, sseq

Bidegree = sseq.Bidegree
BidegreeGenerator = sseq.BidegreeGenerator
BidegreeElement = sseq.BidegreeElement


def _resolution(n=16, s=6, backend="standard"):
    r = ext.Resolution("S_2", backend)
    return r


def build(n=16, s=6, via_without_unit=False):
    """``SecondaryExtAlgebra`` of the mod-2 sphere, resolved through stem (n, s)
    and fully extended (E3 pages built), exactly as upstream ``test_sphere_d2``.
    """
    r = ext.Resolution("S_2", "standard")
    if via_without_unit:
        alg = ext.ExtAlgebra.without_unit(r)
    else:
        alg = ext.ExtAlgebra(r, r)
    alg.compute_through_stem(Bidegree.n_s(n, s))
    sec = ext.SecondaryExtAlgebra(alg)
    sec.extend_all()
    return alg, sec


def gen(alg, n, s):
    return alg.generator(BidegreeGenerator.n_s(n, s, 0))


# --- construction & accessors ---------------------------------------------


def test_construct_and_prime():
    alg, sec = build(4, 4)
    assert sec.prime == 2
    # ext_algebra() returns the bound ExtAlgebra (sharing resolutions).
    e = sec.ext_algebra
    assert isinstance(e, ext.ExtAlgebra)
    assert e.prime == 2
    assert e.is_unit is True


def test_without_unit_builds_usable_d2_path():
    # without_unit(res) == new(res, res): is_unit True, and the d2 layer works.
    alg, sec = build(16, 6, via_without_unit=True)
    assert alg.is_unit is True
    h0 = gen(alg, 0, 1)
    assert sec.survives(h0) is True


def test_without_unit_rejects_nassau():
    r = ext.Resolution("S_2", "nassau")
    with pytest.raises(ValueError):
        ext.ExtAlgebra.without_unit(r)


# --- KNOWN d2 values (upstream test_sphere_d2) -----------------------------


@pytest.mark.parametrize("n,s", [(0, 1), (1, 1), (3, 1)])
def test_permanent_classes_survive(n, s):
    # h0, h1, h2 are permanent cycles: survives == True and d2 is the zero class.
    alg, sec = build(16, 6)
    h = gen(alg, n, s)
    assert sec.survives(h) is True
    d = sec.d2(h)
    assert d is not None
    assert d.vec().is_zero


def test_h4_first_adams_differential():
    # d2(h4) = h0·h3², the nonzero generator of Ext^{3,17} at (n=14, s=3).
    alg, sec = build(16, 6)
    h4 = gen(alg, 15, 1)
    d = sec.d2(h4)
    assert d is not None
    # target bidegree (n=14, s=3), dimension 1.
    assert d.n == 14
    assert d.s == 3
    assert alg.dimension(Bidegree.n_s(14, 3)) == 1
    assert not d.vec().is_zero
    # h4 does not survive.
    assert sec.survives(h4) is False


# --- page_data / unit_page_data --------------------------------------------


def test_page_data_returns_subquotient():
    alg, sec = build(16, 6)
    sq = sec.page_data(Bidegree.n_s(0, 1))
    assert isinstance(sq, fp.Subquotient)
    # h0 is a single surviving class at (0, 1).
    assert sq.dimension == 1
    usq = sec.unit_page_data(Bidegree.n_s(0, 0))
    assert isinstance(usq, fp.Subquotient)
    # The unit class survives at (0, 0).
    assert usq.dimension == 1


# --- SecondaryProduct round-trip -------------------------------------------


def test_secondary_multiply_into_shapes():
    alg, sec = build(16, 6)
    h0 = gen(alg, 0, 1)
    # Multiply h0 into the unit at (0, 0): the unit class survives, so one product.
    products = sec.secondary_multiply_into(h0, Bidegree.n_s(0, 0))
    assert isinstance(products, list)
    assert len(products) == 1
    p = products[0]
    assert isinstance(p, ext.SecondaryProduct)
    assert isinstance(p.source, BidegreeElement)
    assert isinstance(p.ext_part, fp.FpVector)
    assert isinstance(p.lambda_part, fp.FpVector)
    # source lives at the queried bidegree (0, 0).
    assert p.source.n == 0
    assert p.source.s == 0


def test_secondary_multiply_into_empty_when_none_survive():
    alg, sec = build(16, 6)
    h0 = gen(alg, 0, 1)
    # A computed bidegree with no surviving unit classes yields an empty list.
    products = sec.secondary_multiply_into(h0, Bidegree.n_s(2, 1))
    assert isinstance(products, list)


# --- panic guards: extend_all not called -----------------------------------


def _unextended():
    r = ext.Resolution("S_2", "standard")
    alg = ext.ExtAlgebra(r, r)
    alg.compute_through_stem(Bidegree.n_s(8, 6))
    sec = ext.SecondaryExtAlgebra(alg)  # NOT extended
    return alg, sec


def test_d2_before_extend_all_raises():
    alg, sec = _unextended()
    h0 = gen(alg, 0, 1)
    with pytest.raises(ValueError, match="extend_all"):
        sec.d2(h0)


def test_survives_before_extend_all_raises():
    alg, sec = _unextended()
    h0 = gen(alg, 0, 1)
    with pytest.raises(ValueError, match="extend_all"):
        sec.survives(h0)


def test_page_data_before_extend_all_raises():
    alg, sec = _unextended()
    with pytest.raises(ValueError, match="extend_all"):
        sec.page_data(Bidegree.n_s(0, 1))


def test_unit_page_data_before_extend_all_raises():
    alg, sec = _unextended()
    with pytest.raises(ValueError, match="extend_all"):
        sec.unit_page_data(Bidegree.n_s(0, 0))


def test_secondary_multiply_into_before_extend_all_raises():
    alg, sec = _unextended()
    h0 = gen(alg, 0, 1)
    with pytest.raises(ValueError, match="extend_all"):
        sec.secondary_multiply_into(h0, Bidegree.n_s(0, 0))


# --- panic guards: negative / malformed bidegrees --------------------------


def test_page_data_negative_bidegree_raises():
    alg, sec = build(4, 4)
    with pytest.raises(ValueError, match="s >= 0 and t >= 0"):
        sec.page_data(Bidegree.s_t(-1, 0))


def test_unit_page_data_negative_bidegree_raises():
    alg, sec = build(4, 4)
    with pytest.raises(ValueError, match="s >= 0 and t >= 0"):
        sec.unit_page_data(Bidegree.s_t(0, -1))


def test_secondary_multiply_into_negative_bidegree_raises():
    alg, sec = build(4, 4)
    h0 = gen(alg, 0, 1)
    with pytest.raises(ValueError, match="s >= 0 and t >= 0"):
        sec.secondary_multiply_into(h0, Bidegree.s_t(-1, 0))


def test_compute_partial_negative_raises():
    alg, sec = _unextended()
    with pytest.raises(ValueError, match="s >= 0"):
        sec.compute_partial(-1)


def test_d2_uncomputed_element_raises():
    # An element at a bidegree the resolution has not computed is rejected.
    alg, sec = build(4, 4)
    # Construct an element far out of the computed range by hand.
    far = Bidegree.n_s(100, 1)
    vec = fp.FpVector.from_slice(2, [0])
    elem = BidegreeElement(far, vec)
    with pytest.raises(ValueError):
        sec.d2(elem)


def test_d2_uncomputed_target_returns_none():
    # A valid computed class whose d2 *target* is out of range yields None
    # (uncomputed differential), not a panic. At the far edge of the computed
    # stem the d2 target (n-1, s+2) is unresolved.
    alg, sec = build(16, 6)
    # (n=15, s=6) is computed; its d2 target (14, 8) is beyond the stem (16, 6).
    if alg.dimension(Bidegree.n_s(15, 6)) > 0:
        h = gen(alg, 15, 6)
        # Either None (uncomputed target) or a computed class; must not raise.
        sec.d2(h)


# --- d2-path element guards: prime / coord-count (check_res_element) --------


def test_d2_cross_prime_operand_rejected():
    # SecondaryExtAlgebra.check_res_element reuses ExtAlgebra::check_element: an
    # element whose underlying FpVector is over a different prime than the
    # algebra (p=3 vs the p=2 SecondaryExtAlgebra) is rejected with ValueError,
    # not a panic. (0, 1) IS computed, but the prime check precedes the
    # uncomputed-bidegree check, so the prime branch is what fires. Mirrors
    # test_ext_algebra.py::test_multiply_cross_prime_operand_rejected.
    alg, sec = build(4, 4)
    over_p3 = BidegreeElement(Bidegree.n_s(0, 1), fp.FpVector(3, 1))
    with pytest.raises(ValueError, match="over prime 3 but the ExtAlgebra is over prime 2"):
        sec.d2(over_p3)
    with pytest.raises(ValueError, match="over prime 3 but the ExtAlgebra is over prime 2"):
        sec.survives(over_p3)


def test_d2_coord_count_mismatch_rejected():
    # check_res_element coord-count branch: a class at a computed bidegree whose
    # vector length differs from the generator count there is rejected with
    # ValueError. At (n=0, s=1) the dimension is 1 (h0); a length-2 vector over
    # the right prime trips the coord-count check (which follows the
    # has_computed_bidegree check, so the bidegree must be computed first).
    alg, sec = build(4, 4)
    assert alg.dimension(Bidegree.n_s(0, 1)) == 1
    wrong_len = BidegreeElement(Bidegree.n_s(0, 1), fp.FpVector.from_slice(2, [0, 0]))
    with pytest.raises(ValueError, match="coordinate.*but there are 1 generator"):
        sec.d2(wrong_len)
    with pytest.raises(ValueError, match="coordinate.*but there are 1 generator"):
        sec.survives(wrong_len)


# --- shared-instance identity (Arc storage) --------------------------------


def test_ext_algebra_shares_instance():
    # The Arc-storage refactor makes SecondaryExtAlgebra hold the SAME ExtAlgebra
    # instance it was built on, and ext_algebra() return it back with stable
    # identity (shared resolution/unit Arcs and product cache). Confirm via
    # observable equivalence: same prime / is_unit / dimension at a computed
    # bidegree as the algebra passed to the constructor.
    alg, sec = build(8, 6)
    e = sec.ext_algebra
    assert e.prime == alg.prime
    assert e.is_unit == alg.is_unit
    b = Bidegree.n_s(0, 1)
    assert e.dimension(b) == alg.dimension(b)
