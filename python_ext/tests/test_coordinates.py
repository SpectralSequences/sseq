"""Tests for the `Bidegree` / `BidegreeGenerator` Python API.

These cover the string-protocol behaviors of `BidegreeGenerator` introduced
when removing the bespoke ``to_string_compact()`` method:

- ``__str__`` / ``__repr__``;
- ``__format__`` with the ``""``, ``"full"``, and ``"compact"`` specs;
- iteration / unpacking into ``(degree, idx)``;
- equality, hashability, and the named constructors.
"""

from __future__ import annotations

import pytest

import sseq_ext as ext


# ---------------------------------------------------------------------------
# Construction
# ---------------------------------------------------------------------------


def test_construct_two_arg():
    b = ext.Bidegree.n_s(3, 2)
    g = ext.BidegreeGenerator(b, 5)
    assert g.degree == b
    assert g.idx == 5
    assert g.n == 3
    assert g.s == 2
    assert g.t == 5  # t = n + s


def test_construct_n_s_classmethod():
    g = ext.BidegreeGenerator.n_s(3, 2, 5)
    assert g.n == 3
    assert g.s == 2
    assert g.idx == 5


def test_construct_s_t_classmethod():
    # s_t(s, t, idx): n = t - s
    g = ext.BidegreeGenerator.s_t(2, 5, 7)
    assert g.s == 2
    assert g.t == 5
    assert g.n == 3
    assert g.idx == 7


def test_construct_rejects_tuple():
    """Constructor takes (Bidegree, int), not a single tuple."""
    b = ext.Bidegree.n_s(3, 2)
    with pytest.raises(TypeError):
        ext.BidegreeGenerator((b, 5))  # type: ignore[arg-type]


def test_construct_rejects_wrong_arity():
    b = ext.Bidegree.n_s(3, 2)
    with pytest.raises(TypeError):
        ext.BidegreeGenerator(b)  # type: ignore[call-arg]
    with pytest.raises(TypeError):
        ext.BidegreeGenerator(b, 5, 7)  # type: ignore[call-arg]


# ---------------------------------------------------------------------------
# String protocols: __str__, __repr__, __format__
# ---------------------------------------------------------------------------


def test_str_is_full_form():
    """``str(g)`` produces the full ``(n, s, idx)`` form (with spaces)."""
    g = ext.BidegreeGenerator.n_s(3, 2, 5)
    assert str(g) == "(3, 2, 5)"


def test_repr_is_round_trippable_looking():
    g = ext.BidegreeGenerator.n_s(3, 2, 5)
    assert repr(g) == "BidegreeGenerator(n=3, s=2, idx=5)"


def test_format_default_matches_str():
    g = ext.BidegreeGenerator.n_s(3, 2, 5)
    assert f"{g}" == str(g) == "(3, 2, 5)"


def test_format_full_spec():
    g = ext.BidegreeGenerator.n_s(3, 2, 5)
    assert f"{g:full}" == "(3, 2, 5)"


def test_format_compact_spec():
    g = ext.BidegreeGenerator.n_s(3, 2, 5)
    assert f"{g:compact}" == "(3,2,5)"


def test_format_compact_negative_coords():
    """Compact form must still separate every coordinate, even if negative."""
    g = ext.BidegreeGenerator.n_s(-1, 2, 0)
    # The Display impl writes coords as i32, so a leading '-' on n is fine.
    assert f"{g:compact}" == "(-1,2,0)"
    assert f"{g:full}" == "(-1, 2, 0)"


def test_format_unknown_spec_raises():
    g = ext.BidegreeGenerator.n_s(3, 2, 5)
    with pytest.raises(ValueError, match="Unknown format spec"):
        f"{g:weird}"


def test_to_string_compact_removed():
    """The legacy method is gone; only ``__format__`` should provide it."""
    g = ext.BidegreeGenerator.n_s(3, 2, 5)
    assert not hasattr(g, "to_string_compact")


# ---------------------------------------------------------------------------
# Iteration / unpacking
# ---------------------------------------------------------------------------


def test_unpack_into_degree_and_idx():
    b = ext.Bidegree.n_s(3, 2)
    g = ext.BidegreeGenerator(b, 5)
    deg, idx = g
    assert isinstance(deg, ext.Bidegree)
    assert deg == b
    assert idx == 5


def test_iter_yields_two_items():
    g = ext.BidegreeGenerator.n_s(3, 2, 5)
    items = list(g)
    assert len(items) == 2
    deg, idx = items
    assert deg == ext.Bidegree.n_s(3, 2)
    assert idx == 5


def test_len_is_two():
    g = ext.BidegreeGenerator.n_s(3, 2, 5)
    assert len(g) == 2


def test_unpack_too_many_fails():
    g = ext.BidegreeGenerator.n_s(3, 2, 5)
    with pytest.raises(ValueError):
        a, b, c = g  # noqa: F841


def test_iter_returns_fresh_iterator_each_call():
    """``__iter__`` should give a new iterator, so re-iterating works."""
    g = ext.BidegreeGenerator.n_s(3, 2, 5)
    first = list(g)
    second = list(g)
    assert first == second


# ---------------------------------------------------------------------------
# Equality / hashing
# ---------------------------------------------------------------------------


def test_equality_and_hash():
    g1 = ext.BidegreeGenerator.n_s(3, 2, 5)
    g2 = ext.BidegreeGenerator.n_s(3, 2, 5)
    g3 = ext.BidegreeGenerator.n_s(3, 2, 6)
    assert g1 == g2
    assert hash(g1) == hash(g2)
    assert g1 != g3
    # usable as a dict / set key
    d = {g1: "first", g3: "third"}
    assert d[g2] == "first"


def test_n_s_and_s_t_constructors_agree():
    """``n_s(n, s, idx)`` and ``s_t(s, n+s, idx)`` describe the same gen."""
    g1 = ext.BidegreeGenerator.n_s(3, 2, 5)
    g2 = ext.BidegreeGenerator.s_t(2, 5, 5)
    assert g1 == g2
