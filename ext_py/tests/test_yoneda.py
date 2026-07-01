"""Tests for ``ext.yoneda_representative_element``.

The Yoneda representative of an Ext class is a quasi-isomorphic finite quotient
of the resolution that the class factors through (the geometric representative).
We copy the canonical construction from the upstream ``examples/yoneda.rs`` /
``examples/steenrod.rs``: resolve ``S_2`` through a small stem, pick the class
``h_0`` at Adams bidegree ``(s, t) = (1, 1)`` with coordinate ``[1]``, and compute
its representative.

Structural invariants asserted (derived from the upstream examples, which print
``module(s).total_dimension`` for ``s`` in ``0..=b.s()`` and the upstream
Euler-characteristic sanity assert ``euler_characteristic(t) == target_dim(t)``):

* the result is a ``FiniteAugmentedChainComplex`` over ``p = 2``;
* it has ``b.s() + 1`` modules (``max_s() == 2`` for ``b.s() == 1``);
* its augmentation ``target()`` is the original ``S_2`` complex (1-dimensional
  in internal degree 0);
* the bottom module ``C_0`` is the point in internal degree 0.

Yoneda operates on the *standard* backend only; a Nassau-backed ``Resolution`` is
rejected with a ``ValueError`` (mirroring ``ResolutionHomomorphism`` /
``SecondaryResolution`` / ``chain_complex()``).
"""

import pytest

import ext
from ext import sseq

# h_0 lives at Adams bidegree (s, t) = (1, 1); resolving through stem (8, 4)
# covers it comfortably and stays fast.
H0 = sseq.Bidegree.s_t(1, 1)


def standard_s2():
    r = ext.Resolution("S_2", "standard")
    r.compute_through_stem(sseq.Bidegree.n_s(8, 4))
    return r


# --- the representative itself ---------------------------------------------


def test_h0_representative_structure():
    r = standard_s2()
    y = ext.yoneda_representative_element(r, H0, [1])
    assert isinstance(y, ext.FiniteAugmentedChainComplex)
    assert y.prime == 2
    # s_max = b.s() = 1, so modules C_0, C_1 => max_s() = modules.len = 2.
    assert y.max_s == 2
    # The augmentation target is the original S_2 complex: 1-dimensional in
    # internal degree 0.
    target = y.target
    assert target.prime == 2
    assert target.module(0).dimension(0) == 1
    # The bottom cell C_0 is a point in internal degree 0.
    assert y.module(0).dimension(0) == 1
    # The s=0 augmentation chain map is defined.
    assert y.chain_map(0).prime == 2


def test_chain_map_index_out_of_range_raises_index_error():
    r = standard_s2()
    y = ext.yoneda_representative_element(r, H0, [1])
    with pytest.raises(IndexError):
        y.chain_map(2)
    with pytest.raises(IndexError):
        y.chain_map(-1)


# --- backend rejection ------------------------------------------------------


def test_rejects_nassau_backend():
    r = ext.Resolution("S_2", "nassau")
    r.compute_through_stem(sseq.Bidegree.n_s(8, 4))
    with pytest.raises(ValueError):
        ext.yoneda_representative_element(r, H0, [1])


# --- panic guards -----------------------------------------------------------


def test_negative_bidegree_raises_value_error():
    r = standard_s2()
    with pytest.raises(ValueError):
        ext.yoneda_representative_element(r, sseq.Bidegree.s_t(-1, 0), [1])
    with pytest.raises(ValueError):
        ext.yoneda_representative_element(r, sseq.Bidegree.s_t(0, -1), [1])


def test_unresolved_bidegree_raises_value_error():
    r = standard_s2()
    # Far outside the resolved region -> "resolution not resolved through ...".
    with pytest.raises(ValueError):
        ext.yoneda_representative_element(r, sseq.Bidegree.s_t(50, 50), [])


def test_wrong_class_length_raises_value_error():
    r = standard_s2()
    # (1, 1) has exactly one generator.
    with pytest.raises(ValueError):
        ext.yoneda_representative_element(r, H0, [1, 0])
    with pytest.raises(ValueError):
        ext.yoneda_representative_element(r, H0, [])


def test_class_entry_out_of_range_raises_value_error():
    r = standard_s2()
    # Entry must be in [0, p) = [0, 2).
    with pytest.raises(ValueError):
        ext.yoneda_representative_element(r, H0, [2])
