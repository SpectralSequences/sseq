"""Tests for API fixes: negative indexing, getter consistency, typed
exceptions in place of panics, and coverage of coordinate/fp types."""

from __future__ import annotations

import pytest

import sseq_ext as ext

P2 = 2


# ---------------------------------------------------------------------------
# Negative / out-of-range indexing
# ---------------------------------------------------------------------------


def test_fpvector_negative_index():
    v = ext.FpVector.from_slice(P2, [1, 0, 1, 1, 0])
    view = v.const[:]
    assert view[-1] == 0
    assert view[-2] == 1
    assert view[-5] == 1
    with pytest.raises(IndexError):
        _ = view[-6]
    with pytest.raises(IndexError):
        _ = view[5]


def test_fpvector_negative_setitem():
    v = ext.FpVector(P2, 4)
    v.mut[:][-1] = 1
    assert v.to_list() == [0, 0, 0, 1]


def test_matrix_negative_index():
    m = ext.Matrix.from_vec(P2, [[1, 0], [0, 1]])
    assert m[-1, -1] == 1
    assert m[-2, 0] == 1
    m[-1, -2] = 1
    assert m[1, 0] == 1
    with pytest.raises(IndexError):
        _ = m[-3, 0]
    with pytest.raises(IndexError):
        _ = m[0, 5]


def test_matrix_row_view_negative():
    m = ext.Matrix.from_vec(P2, [[1, 0], [0, 1]])
    assert m.const[-1].to_list() == [0, 1]


# ---------------------------------------------------------------------------
# prime / name are getters (attributes), not methods
# ---------------------------------------------------------------------------


def test_prime_is_attribute():
    v = ext.FpVector(P2, 3)
    assert v.prime == 2
    m = ext.Matrix(P2, 1, 1)
    assert m.prime == 2
    a = ext.MilnorAlgebra(2)
    assert a.prime == 2
    # Calling as a method should fail (it's an int now).
    with pytest.raises(TypeError):
        v.prime()


def test_milnor_dimension_no_panic_without_compute_basis():
    """dimension() must not panic even if compute_basis wasn't called for
    that degree."""
    a = ext.MilnorAlgebra(2)
    # Should be safe for a large, not-yet-computed degree.
    assert a.dimension(20) >= 1
    assert a.dimension(-3) == 0


# ---------------------------------------------------------------------------
# Typed exceptions instead of panics
# ---------------------------------------------------------------------------


def test_secondary_requires_milnor():
    res = ext.construct("S_2", algebra="adem")
    with pytest.raises(ValueError, match="Milnor"):
        ext.SecondaryResolution(res)


def test_from_class_wrong_length():
    res = ext.construct("S_2", algebra="milnor")
    shift = ext.Bidegree.s_t(0, 0)
    res.compute_through_bidegree(shift)
    # The unit has 1 generator in (0, 0); supplying 3 entries must error.
    with pytest.raises(ValueError, match="length"):
        ext.ResolutionHomomorphism.from_class("f", res, res, shift, [1, 1, 1])


def test_chain_homotopy_mismatched_maps():
    res1 = ext.construct("S_2", algebra="milnor")
    res2 = ext.construct("S_2", algebra="milnor")
    shift = ext.Bidegree.s_t(0, 0)
    f1 = ext.ResolutionHomomorphism("f1", res1, res1, shift)
    f2 = ext.ResolutionHomomorphism("f2", res2, res2, shift)
    with pytest.raises(ValueError):
        ext.ChainHomotopy(f1, f2)


def test_construct_invalid_module():
    with pytest.raises(ValueError):
        ext.construct("not_a_real_module_name")


# ---------------------------------------------------------------------------
# Subspace
# ---------------------------------------------------------------------------


def test_subspace_basics():
    # [target | source] with the target block full rank.
    am = ext.AugmentedMatrix(P2, 2, [2, 2])
    am[0, 0] = 1
    am[1, 1] = 1
    am.segment_mut[1].add_identity()  # source block = identity
    am.row_reduce()
    img = am.compute_image()
    assert img.ambient_dimension() == 2
    assert img.dimension() == 2
    one = ext.FpVector.from_slice(P2, [1, 0])
    assert img.contains(one)
    assert len(img.basis()) == img.dimension()


def test_subspace_contains_length_mismatch():
    am = ext.AugmentedMatrix(P2, 2, [2, 2])
    am.segment_mut[1].add_identity()
    am.row_reduce()
    sub = am.compute_image()
    bad = ext.FpVector(P2, sub.ambient_dimension() + 1)
    with pytest.raises(ValueError):
        sub.contains(bad)


# ---------------------------------------------------------------------------
# Coordinates
# ---------------------------------------------------------------------------


def test_bidegree_arithmetic_and_eq():
    a = ext.Bidegree.n_s(3, 1)
    b = ext.Bidegree.n_s(2, 2)
    assert (a + b) == ext.Bidegree.n_s(5, 3)
    assert (a - b) == ext.Bidegree.n_s(1, -1)
    assert a == ext.Bidegree.n_s(3, 1)
    assert a != b
    assert hash(a) == hash(ext.Bidegree.n_s(3, 1))
    assert a.n == 3 and a.s == 1 and a.t == 4
    assert ext.Bidegree.zero() == ext.Bidegree.n_s(0, 0)


def test_bidegree_generator_getitem_and_unpack():
    g = ext.BidegreeGenerator.n_s(3, 1, 2)
    assert len(g) == 2
    deg, idx = g
    assert deg == ext.Bidegree.n_s(3, 1)
    assert idx == 2
    assert g[0] == deg
    assert g[1] == 2
    assert g[-1] == 2
    assert g[-2] == deg
    with pytest.raises(IndexError):
        _ = g[2]


def test_bidegree_element_eq_hash():
    deg = ext.Bidegree.n_s(0, 0)
    v1 = ext.FpVector.from_slice(P2, [1, 0])
    v2 = ext.FpVector.from_slice(P2, [1, 0])
    v3 = ext.FpVector.from_slice(P2, [0, 1])
    e1 = ext.BidegreeElement(deg, v1)
    e2 = ext.BidegreeElement(deg, v2)
    e3 = ext.BidegreeElement(deg, v3)
    assert e1 == e2
    assert e1 != e3
    assert hash(e1) == hash(e2)
    assert e1.vec.to_list() == [1, 0]
