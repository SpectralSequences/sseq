"""Tests for the unstable (``U = true``) resolution-homomorphism family:

  - ``ext.UnstableResolutionHomomorphism`` (the unstable analogue of
    ``ResolutionHomomorphism``),
  - ``algebra.UnstableFreeModuleHomomorphism`` (its ``get_map(s)`` return —
    the unstable free -> free chain map),
  - ``algebra.UnstableFreeModule`` (the unstable resolution's modules, also
    handed back by ``UnstableResolution.module(s)``).

These mirror the *stable* family bound in ``test_resolution_homomorphism.py``
but live over ``MuFreeModule<true, _>`` / ``MuFreeModuleHomomorphism<true, _>``
— distinct const-generic monomorphisations from the stable ``U = false`` types,
so they need their own pyclasses.

Known value: ``from_class`` with shift ``(0, 0)`` and class ``[1]`` is the
identity chain map lifting the identity on the augmentation, so its ``s``-th map
sends generator ``idx`` to the indicator at
``operation_generator_to_index(0, 0, t, idx)`` — exactly the invariant
``ext/tests/extend_identity.rs`` asserts (and the same invariant the stable
``test_from_class_identity_matches_basis`` checks), here for the unstable
resolution of ``S_2``.

The unsuspended unstable ``S_2`` chart is trivial (only the ``(0, 0)`` unit), so
``act`` is exercised on that unit; the structural / guard tests cover the rest.

All bad degree/index/bidegree inputs are pre-checked and raise
``ValueError``/``IndexError`` rather than panicking across the FFI boundary.
"""

import pytest

import ext
from ext import algebra, fp, sseq

Bidegree = sseq.Bidegree
BidegreeGenerator = sseq.BidegreeGenerator
FpVector = fp.FpVector


def us2_rect(max_st=6):
    """An unstable resolution of S_2 computed through the full bidegree
    rectangle (max_st, max_st), so an extend over the same range is fully
    resolved."""
    r = ext.construct_unstable("S_2")
    r.compute_through_bidegree(Bidegree.s_t(max_st, max_st))
    return r


def identity_hom(r, max_st=6):
    hom = ext.UnstableResolutionHomomorphism.from_class(
        "id", r, r, Bidegree.s_t(0, 0), [1]
    )
    hom.extend(Bidegree.s_t(max_st, max_st))
    return hom


# --- construction & accessors ---------------------------------------------


def test_new_accessors_roundtrip():
    r = us2_rect(4)
    hom = ext.UnstableResolutionHomomorphism("f", r, r, Bidegree.s_t(1, 1))
    assert hom.name == "f"
    assert hom.prime == 2
    assert hom.shift.s == 1
    assert hom.shift.t == 1
    assert hom.source.prime == 2
    assert hom.target.prime == 2
    assert hom.source.graded_dimension_string() == r.graded_dimension_string()
    # A freshly constructed hom defines no maps yet (next_hom_degree == shift.s).
    assert hom.next_homological_degree == 1
    assert hom.save_dir is None
    assert hom.algebra.prime == 2


# --- known value: from_class([1]) at (0,0) is the identity chain map ------


def test_from_class_identity_matches_basis():
    # The basis-indicator invariant from ext/tests/extend_identity.rs: the
    # from_class([1]) identity chain map sends generator (t, idx) of module(s)
    # to the indicator vector with a single 1 at
    # operation_generator_to_index(0, 0, t, idx).
    #
    # NOTE on the unstable sphere: from_class([1]) at shift (0,0) requires a
    # degree-0 unit (the augmentation must be 1-dimensional in degree 0) AND a
    # source generator at bidegree (0,0). The only unstable resolution meeting
    # both is the *unsuspended* sphere S_2, whose chart is the single (0,0) unit
    # -- so this invariant necessarily visits exactly one generator here. The
    # suspended sphere S^3 = S_2[3] HAS a non-trivial unstable chart, but its
    # bottom cell sits in degree 3 (no (0,0) generator and no degree-0 unit), so
    # from_class cannot express its identity at all; the genuine
    # unstable-vs-stable contrast lives in test_unstable.py's
    # test_unstable_vs_stable_s3_charts_differ instead.
    r = us2_rect(6)
    hom = identity_hom(r, 6)
    assert hom.name == "id"
    assert hom.shift.s == 0 and hom.shift.t == 0

    visited = 0
    for s in range(0, 7):
        m = hom.get_map(s)
        assert isinstance(m, algebra.UnstableFreeModuleHomomorphism)
        src = r.module(s)
        assert isinstance(src, algebra.UnstableFreeModule)
        for t in range(0, 7):
            for idx in range(src.number_of_gens_in_degree(t)):
                visited += 1
                out = m.output(t, idx)
                j = src.operation_generator_to_index(0, 0, t, idx)
                for i in range(len(out)):
                    assert out[i] == (1 if i == j else 0), (
                        f"identity mismatch at s={s} t={t} idx={idx} i={i}"
                    )
    # The unstable S_2 chart is exactly the (0,0) unit (see the docstring note).
    assert visited == 1


def test_get_map_is_unstable_free_to_free():
    r = us2_rect(4)
    hom = identity_hom(r, 4)
    m = hom.get_map(0)
    assert m.degree_shift == 0
    assert m.prime == 2
    # source()/target() are unstable free modules sharing the resolution's Arc.
    assert isinstance(m.source, algebra.UnstableFreeModule)
    assert isinstance(m.target, algebra.UnstableFreeModule)
    assert m.source.prime == 2
    assert m.next_degree > 0
    # s=0 map is the identity on the unit: output(0,0) == [1].
    assert list(m.output(0, 0)) == [1]


# --- extend_all / extend_through_stem -------------------------------------


def test_extend_all_then_get_map():
    r = us2_rect(6)
    hom = ext.UnstableResolutionHomomorphism.from_class(
        "id", r, r, Bidegree.s_t(0, 0), [1]
    )
    hom.extend_all()
    assert hom.get_map(0).output(0, 0)[0] == 1


def test_extend_through_stem():
    r = ext.construct_unstable("S_2")
    r.compute_through_stem(Bidegree.n_s(8, 4))
    hom = ext.UnstableResolutionHomomorphism.from_class(
        "id", r, r, Bidegree.s_t(0, 0), [1]
    )
    hom.extend_through_stem(Bidegree.n_s(4, 4))
    assert hom.get_map(0).output(0, 0)[0] == 1


# --- act ------------------------------------------------------------------


def test_act_identity_picks_out_unit():
    # The unsuspended unstable S_2 chart is trivial (only the (0,0) unit), so
    # the identity hom acts as the identity on that single generator.
    r = us2_rect(6)
    hom = identity_hom(r, 6)
    b = Bidegree.s_t(0, 0)
    n_src = r.number_of_gens_in_bidegree(b)
    assert n_src == 1
    result = FpVector(2, n_src)
    hom.act(result, 1, BidegreeGenerator.s_t(0, 0, 0))
    assert result[0] == 1


def test_act_guards():
    r = us2_rect(6)
    hom = identity_hom(r, 6)
    result = FpVector(2, 1)
    # Negative generator -> ValueError.
    with pytest.raises(ValueError):
        hom.act(result, 1, BidegreeGenerator.s_t(-1, 0, 0))
    # Wrong result length -> ValueError.
    bad = FpVector(2, 5)
    with pytest.raises(ValueError):
        hom.act(bad, 1, BidegreeGenerator.s_t(0, 0, 0))
    # Generator index out of range at the target bidegree -> IndexError.
    with pytest.raises(IndexError):
        hom.act(result, 1, BidegreeGenerator.s_t(0, 0, 99))


def test_act_prime_mismatch_errors():
    r = us2_rect(6)
    hom = identity_hom(r, 6)
    wrong_prime = FpVector(3, 1)
    with pytest.raises(ValueError):
        hom.act(wrong_prime, 1, BidegreeGenerator.s_t(0, 0, 0))


def test_act_map_undefined_at_s_errors():
    r = us2_rect(6)
    hom = identity_hom(r, 6)
    result = FpVector(2, 1)
    with pytest.raises(ValueError):
        hom.act(result, 1, BidegreeGenerator.s_t(99, 99, 0))


def test_act_not_extended_far_enough_errors():
    # The `src_t >= map.next_degree` guard ("map not extended far enough"):
    # the hom IS defined at homological degree src_s (so the src_s guard does
    # NOT fire) and the source IS computed at the source bidegree, but the map
    # has only been extended through a smaller t. Resolve S_2 through (6, 6) so
    # the source bidegree (0, 3) is computed, but extend the identity hom only
    # through (0, 0); then act at g = (0, 3, 0): src = (0, 3) with src_t = 3 >=
    # map(0).next_degree == 1.
    r = us2_rect(6)
    hom = ext.UnstableResolutionHomomorphism.from_class(
        "id", r, r, Bidegree.s_t(0, 0), [1]
    )
    hom.extend(Bidegree.s_t(0, 0))
    assert hom.next_homological_degree == 1  # s = 0 map is defined ...
    assert hom.get_map(0).next_degree == 1  # ... but only extended through t=0
    result = FpVector(2, 1)
    with pytest.raises(ValueError, match="not extended through"):
        hom.act(result, 1, BidegreeGenerator.s_t(0, 3, 0))


def test_act_target_not_computed_guard_is_shadowed():
    # The `target.has_computed_bidegree(g.degree())` guard is intentionally
    # over-strict relative to the stable ResolutionHomomorphism.act (see the
    # NOTE at that guard in src/lib.rs): it is a conservative superset of the
    # `src_t >= map.next_degree` guard and is UNREACHABLE from Python. The hom
    # cannot be extended past the target's computed range (extend itself guards
    # `target.has_computed_bidegree(input - shift)`), so whenever g.degree() is
    # uncomputed in the target the earlier "not extended through" guard fires
    # first. We therefore exercise the reachable path and assert that guard.
    #
    # Setup: source resolved further (through t = 9) than target (through t = 6),
    # shift (0, 0). The hom can only be extended through (6, 6) (target's range),
    # so act at g = (0, 7, 0) -- where the target is NOT computed but the source
    # IS -- trips `src_t >= map.next_degree` rather than the target guard.
    src = ext.construct_unstable("S_2")
    src.compute_through_bidegree(Bidegree.s_t(6, 9))
    tgt = ext.construct_unstable("S_2")
    tgt.compute_through_bidegree(Bidegree.s_t(6, 6))
    hom = ext.UnstableResolutionHomomorphism.from_class(
        "id", src, tgt, Bidegree.s_t(0, 0), [1]
    )
    hom.extend(Bidegree.s_t(6, 6))
    assert src.has_computed_bidegree(Bidegree.s_t(0, 7)) is True
    assert tgt.has_computed_bidegree(Bidegree.s_t(0, 7)) is False
    # g.s = 0 < tgt.next_homological_degree, so the target-s guard is passed;
    # the "not extended through" guard is what actually fires.
    result = FpVector(2, 1)
    with pytest.raises(ValueError, match="not extended through"):
        hom.act(result, 1, BidegreeGenerator.s_t(0, 7, 0))


def test_act_degree_overflow_errors():
    r = us2_rect(4)
    hom = ext.UnstableResolutionHomomorphism("f", r, r, Bidegree.s_t(1, 1))
    result = FpVector(2, 1)
    with pytest.raises(ValueError):
        hom.act(result, 1, BidegreeGenerator.s_t(2147483647, 0, 0))
    with pytest.raises(ValueError):
        hom.act(result, 1, BidegreeGenerator.s_t(0, 2147483647, 0))


# --- panic guards: construction -------------------------------------------


def test_new_prime_mismatch_errors():
    s2 = us2_rect(4)
    s3 = ext.construct_unstable("S_3")
    with pytest.raises(ValueError):
        ext.UnstableResolutionHomomorphism("f", s2, s3, Bidegree.s_t(0, 0))


def test_from_class_prime_mismatch_errors():
    s2 = us2_rect(4)
    s3 = ext.construct_unstable("S_3")
    with pytest.raises(ValueError):
        ext.UnstableResolutionHomomorphism.from_class(
            "f", s2, s3, Bidegree.s_t(0, 0), [1]
        )


def test_new_negative_shift_errors():
    r = us2_rect(4)
    with pytest.raises(ValueError):
        ext.UnstableResolutionHomomorphism("f", r, r, Bidegree.s_t(-1, 0))
    with pytest.raises(ValueError):
        ext.UnstableResolutionHomomorphism("f", r, r, Bidegree.s_t(0, -1))


def test_from_class_guards():
    r = us2_rect(4)
    # Wrong class length (source has 1 generator at (0,0)).
    with pytest.raises(ValueError):
        ext.UnstableResolutionHomomorphism.from_class(
            "id", r, r, Bidegree.s_t(0, 0), [1, 1]
        )
    # Class at an uncomputed bidegree.
    with pytest.raises(ValueError):
        ext.UnstableResolutionHomomorphism.from_class(
            "id", r, r, Bidegree.s_t(100, 100), []
        )


def test_get_map_out_of_range_errors():
    r = us2_rect(4)
    hom = identity_hom(r, 4)
    with pytest.raises(IndexError):
        hom.get_map(-1)
    with pytest.raises(IndexError):
        hom.get_map(1000)


def test_extend_guards():
    r = us2_rect(4)
    hom = ext.UnstableResolutionHomomorphism.from_class(
        "id", r, r, Bidegree.s_t(0, 0), [1]
    )
    with pytest.raises(ValueError):
        hom.extend(Bidegree.s_t(-1, 0))
    # Beyond the resolved range (source only computed through (4, 4)).
    with pytest.raises(ValueError):
        hom.extend(Bidegree.s_t(100, 100))
    with pytest.raises(ValueError):
        hom.extend_through_stem(Bidegree.n_s(100, 4))


def test_extend_all_unresolved_errors():
    r = ext.construct_unstable("S_2")
    hom = ext.UnstableResolutionHomomorphism("f", r, r, Bidegree.s_t(0, 0))
    with pytest.raises(ValueError):
        hom.extend_all()


# --- UnstableFreeModule accessor guards -----------------------------------


def test_unstable_resolution_module_accessor_and_guards():
    r = us2_rect(4)
    m = r.module(0)
    assert isinstance(m, algebra.UnstableFreeModule)
    assert m.prime == 2
    assert m.min_degree == 0
    assert m.number_of_gens_in_degree(0) == 1
    assert m.dimension(0) == 1
    assert isinstance(m.basis_element_to_string(0, 0), str)
    # module(s) out of range -> IndexError, no panic.
    with pytest.raises(IndexError):
        r.module(-1)
    with pytest.raises(IndexError):
        r.module(10_000)


def test_unstable_free_module_operation_generator_to_index_unstable_bound():
    # THE unstable-specific guard: the number of admissible operations on a
    # degree-0 generator in op_degree 1 is dimension_unstable(1, 0) == 0 (Sq^1
    # has excess 1 > generator degree 0), so op_index 0 is OUT of range and must
    # raise IndexError -- NOT silently read a neighbour's basis element as the
    # stable `dimension(1) == 1` bound would permit. The identity operation
    # (op_degree 0, op_index 0) is always valid.
    r = us2_rect(6)
    m = r.module(0)
    assert m.operation_generator_to_index(0, 0, 0, 0) == 0
    with pytest.raises(IndexError):
        m.operation_generator_to_index(1, 0, 0, 0)
    # Negative op_degree -> IndexError (degree treated as out-of-range index).
    with pytest.raises(IndexError):
        m.operation_generator_to_index(-1, 0, 0, 0)
    # Out-of-range generator index -> IndexError.
    with pytest.raises(IndexError):
        m.operation_generator_to_index(0, 0, 0, 5)
    # Generator degree below min_degree -> ValueError.
    with pytest.raises(ValueError):
        m.operation_generator_to_index(0, 0, -1, 0)


def test_unstable_free_module_homomorphism_output_guards():
    r = us2_rect(4)
    hom = identity_hom(r, 4)
    m = hom.get_map(0)
    # Valid generator output.
    assert list(m.output(0, 0)) == [1]
    # Generator degree beyond the defined range -> ValueError.
    with pytest.raises(ValueError):
        m.output(10_000, 0)
    # Out-of-range generator index -> IndexError.
    with pytest.raises(IndexError):
        m.output(0, 99)
    # Below min_degree -> IndexError.
    with pytest.raises(IndexError):
        m.output(-1, 0)
