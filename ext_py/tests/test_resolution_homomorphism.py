"""Tests for the `ResolutionHomomorphism` pyclass (`ext::resolution_homomorphism`).

A `ResolutionHomomorphism` is a lifted chain map between two free resolutions —
the resolution-level realisation of a map of `Ext` modules. Only the
*standard*-backend Standard->Standard instantiation is bound (both source and
target are `Resolution(..., "standard")`); a Nassau-backed resolution is rejected
with a clean `ValueError`, because Nassau resolves over the concrete
`MilnorAlgebra` whose maps the bound homomorphism pyclasses cannot represent
(mirroring the standard-only `Resolution.module`/`chain_complex`).

The canonical known value: `from_class` with shift `(0, 0)` and class `[1]` is the
identity chain map lifting the identity on the augmentation, so its `s`-th map
sends generator `idx` to the corresponding basis element (a single `1` at
`operation_generator_to_index(0, 0, t, idx)`). This is exactly the invariant
`ext/tests/extend_identity.rs` asserts for `from_module_homomorphism(identity)`.

All bad degree/index/bidegree inputs are pre-checked and raise
`ValueError`/`IndexError` rather than panicking across the FFI boundary.
"""

import pytest

import ext
from ext import fp, sseq

Bidegree = sseq.Bidegree
BidegreeGenerator = sseq.BidegreeGenerator
FpVector = fp.FpVector


def s2_rect(max_st=8):
    """A standard-backend resolution of S_2 computed through the full bidegree
    rectangle (max_st, max_st), so an extend over the same range is fully
    resolved."""
    r = ext.Resolution("S_2", "standard")
    r.compute_through_bidegree(Bidegree.s_t(max_st, max_st))
    return r


def identity_hom(r, max_st=8):
    hom = ext.ResolutionHomomorphism.from_class("id", r, r, Bidegree.s_t(0, 0), [1])
    hom.extend(Bidegree.s_t(max_st, max_st))
    return hom


# --- construction & accessors ---------------------------------------------


def test_new_accessors_roundtrip():
    r = s2_rect(4)
    hom = ext.ResolutionHomomorphism("f", r, r, Bidegree.s_t(1, 1))
    assert hom.name() == "f"
    assert hom.prime() == 2
    assert hom.shift().s == 1
    assert hom.shift().t == 1
    # source()/target() share the underlying resolution.
    assert hom.source().prime() == 2
    assert hom.target().prime() == 2
    assert hom.source().graded_dimension_string() == r.graded_dimension_string()
    # A freshly constructed hom defines no maps yet.
    assert hom.next_homological_degree() == 1  # = shift.s
    assert hom.save_dir() is None
    assert hom.algebra().prime() == 2


# --- known value: from_class([1]) at (0,0) is the identity chain map ------


def test_from_class_identity_matches_basis():
    r = s2_rect(8)
    hom = identity_hom(r, 8)
    assert hom.name() == "id"
    assert hom.shift().s == 0 and hom.shift().t == 0

    for s in range(0, 9):
        m = hom.get_map(s)
        src = r.module(s)
        for t in range(0, 9):
            for idx in range(src.number_of_gens_in_degree(t)):
                out = m.output(t, idx)
                j = src.operation_generator_to_index(0, 0, t, idx)
                # Identity: a single 1 at the generator's own basis index.
                for i in range(len(out)):
                    assert out[i] == (1 if i == j else 0), (
                        f"identity mismatch at s={s} t={t} idx={idx} i={i}"
                    )


def test_get_map_is_free_to_free_homomorphism():
    r = s2_rect(4)
    hom = identity_hom(r, 4)
    m = hom.get_map(1)
    # source = r.module(1), target = r.module(0) (output_s = input_s - shift.s = 1).
    assert m.degree_shift() == 0
    assert m.prime() == 2


# --- extend_all -----------------------------------------------------------


def test_extend_all_then_get_map():
    r = s2_rect(6)
    hom = ext.ResolutionHomomorphism.from_class("id", r, r, Bidegree.s_t(0, 0), [1])
    hom.extend_all()
    out = hom.get_map(0).output(0, 0)
    assert out[0] == 1


def test_extend_through_stem():
    r = ext.Resolution("S_2", "standard")
    r.compute_through_stem(Bidegree.n_s(8, 4))
    hom = ext.ResolutionHomomorphism.from_class("id", r, r, Bidegree.s_t(0, 0), [1])
    hom.extend_through_stem(Bidegree.n_s(4, 4))
    # The s=0 map is the identity on the unit.
    assert hom.get_map(0).output(0, 0)[0] == 1


# --- act ------------------------------------------------------------------


def test_act_identity_picks_out_generator():
    # For the identity resolution homomorphism, acting on a target generator g
    # at (s, t) writes the indicator of that generator into the result vector of
    # length = number of source generators at (s, t) (shift = 0).
    r = s2_rect(6)
    hom = identity_hom(r, 6)
    # h_0 lives at (n, s) = (0, 1), i.e. (s, t) = (1, 1), with one generator.
    b = Bidegree.s_t(1, 1)
    n_src = r.number_of_gens_in_bidegree(b)
    assert n_src == 1
    result = FpVector(2, n_src)
    hom.act(result, 1, BidegreeGenerator.s_t(1, 1, 0))
    # Identity acts as the identity on Ext: the generator maps to itself.
    assert result[0] == 1


def test_act_guards():
    r = s2_rect(6)
    hom = identity_hom(r, 6)
    result = FpVector(2, 1)
    # Negative generator -> ValueError.
    with pytest.raises(ValueError):
        hom.act(result, 1, BidegreeGenerator.s_t(-1, 0, 0))
    # Wrong result length -> ValueError.
    bad = FpVector(2, 5)
    with pytest.raises(ValueError):
        hom.act(bad, 1, BidegreeGenerator.s_t(1, 1, 0))
    # Generator index out of range at the target bidegree -> IndexError.
    with pytest.raises(IndexError):
        hom.act(result, 1, BidegreeGenerator.s_t(1, 1, 99))


# --- backend rejection ----------------------------------------------------


def test_rejects_nassau_backend():
    standard = s2_rect(4)
    nassau = ext.Resolution("S_2", "nassau")
    nassau.compute_through_stem(Bidegree.n_s(8, 4))
    with pytest.raises(ValueError):
        ext.ResolutionHomomorphism("f", nassau, standard, Bidegree.s_t(0, 0))
    with pytest.raises(ValueError):
        ext.ResolutionHomomorphism("f", standard, nassau, Bidegree.s_t(0, 0))
    with pytest.raises(ValueError):
        ext.ResolutionHomomorphism.from_class(
            "f", nassau, nassau, Bidegree.s_t(0, 0), [1]
        )


# --- panic guards ---------------------------------------------------------


def test_new_negative_shift_errors():
    r = s2_rect(4)
    with pytest.raises(ValueError):
        ext.ResolutionHomomorphism("f", r, r, Bidegree.s_t(-1, 0))
    with pytest.raises(ValueError):
        ext.ResolutionHomomorphism("f", r, r, Bidegree.s_t(0, -1))


def test_from_class_guards():
    r = s2_rect(4)
    # Wrong class length (source has 1 generator at (0,0)).
    with pytest.raises(ValueError):
        ext.ResolutionHomomorphism.from_class("id", r, r, Bidegree.s_t(0, 0), [1, 1])
    # Class at an uncomputed bidegree.
    with pytest.raises(ValueError):
        ext.ResolutionHomomorphism.from_class(
            "id", r, r, Bidegree.s_t(100, 100), []
        )


def test_get_map_out_of_range_errors():
    r = s2_rect(4)
    hom = identity_hom(r, 4)
    with pytest.raises(IndexError):
        hom.get_map(-1)
    with pytest.raises(IndexError):
        hom.get_map(1000)


def test_extend_guards():
    r = s2_rect(4)
    hom = ext.ResolutionHomomorphism.from_class("id", r, r, Bidegree.s_t(0, 0), [1])
    # Negative target.
    with pytest.raises(ValueError):
        hom.extend(Bidegree.s_t(-1, 0))
    # Beyond the resolved range (source only computed through (4, 4)).
    with pytest.raises(ValueError):
        hom.extend(Bidegree.s_t(100, 100))
    with pytest.raises(ValueError):
        hom.extend_through_stem(Bidegree.n_s(100, 4))


def test_extend_all_unresolved_errors():
    r = ext.Resolution("S_2", "standard")
    hom = ext.ResolutionHomomorphism("f", r, r, Bidegree.s_t(0, 0))
    with pytest.raises(ValueError):
        hom.extend_all()
