"""Smoke tests for the FpVector view system."""

from __future__ import annotations

import pytest

import sseq_ext as ext


def test_owned_basics():
    p = 2
    v = ext.FpVector(p, 5)
    assert len(v) == 5
    assert v.is_owned
    assert v.writable
    v[2] = 1
    assert v[2] == 1
    assert v.to_list() == [0, 0, 1, 0, 0]


def test_fpvector_slice_view():
    p = 2
    v = ext.FpVector.from_slice(p, [1, 0, 1, 1, 0])
    view = v.const[1:4]
    assert len(view) == 3
    assert not view.is_owned
    assert not view.writable  # read-only
    assert view.to_list() == [0, 1, 1]
    with pytest.raises(Exception):
        view[0] = 1  # read-only


def test_fpvector_slice_mut_view():
    p = 2
    v = ext.FpVector(p, 5)
    view = v.mut[1:4]
    assert view.writable
    view[0] = 1
    view[2] = 1
    assert v.to_list() == [0, 1, 0, 1, 0]


def test_matrix_row_view():
    p = 2
    m = ext.Matrix.from_vec(p, [[1, 0, 1], [0, 1, 1]])
    row = m.const[0]
    assert row.to_list() == [1, 0, 1]
    row_mut = m.mut[1]
    row_mut[0] = 1
    assert m.to_list() == [[1, 0, 1], [1, 1, 1]]


def test_augmented_matrix_segment_view():
    p = 2
    am = ext.AugmentedMatrix(p, 2, [3, 2])
    am.segment_mut[1].add_identity()  # right block becomes identity
    seg1 = am.const[0, 1]  # row 0, segment 1
    assert seg1.to_list() == [1, 0]
    seg1_mut = am.mut[1, 0]  # row 1, segment 0
    seg1_mut[1] = 1
    # check by reading back
    seg1_read = am.const[1, 0]
    assert seg1_read.to_list() == [0, 1, 0]


def test_view_outlives_python_handle():
    """If the only reference to the parent is held by a view, the view
    keeps the parent alive via the Py<...> handle."""
    p = 2
    v = ext.FpVector.from_slice(p, [3, 1, 4, 1, 5])
    view = v.const[0:5]
    del v
    # accessing the view should still work
    assert view.to_list() == [1, 1, 0, 1, 1]


def test_view_compose():
    p = 2
    v = ext.FpVector.from_slice(p, [1, 1, 1, 1, 1])
    outer = v.const[0:5]
    inner = outer[1:4]
    assert inner.to_list() == [1, 1, 1]
    assert len(inner) == 3
