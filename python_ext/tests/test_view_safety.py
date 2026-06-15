"""Safety tests for the FpVector view system.

These tests exercise:

1. Slice arithmetic & bounds checking.
2. Read-only enforcement (writes through `View` raise).
3. Lifetime / GC (parent stays alive via view; cyclic refs).
4. Mutation visibility (parent and view see each other's writes).
5. Aliasing read-write semantics (overlapping ViewMuts).
6. Re-entrancy via a Rust-side test hook.
7. Stress / fuzz of random view operations.

The view API is:

- ``v.const`` / ``v.mut`` — view over the whole vector.
- ``v.const[a:b]`` / ``v.mut[a:b]`` — sub-view.
- ``view[a:b]`` — sub-view (mutability inherited from ``view``).
- ``m.const[row]`` / ``m.mut[row]`` — matrix row view.
- ``am.const[row, seg]`` / ``am.mut[row, seg]`` — augmented-matrix
  segment view. Pass ``(start_seg, end_seg)`` for a range of segments.
"""

from __future__ import annotations

import gc
import random
import weakref

import pytest

import sseq_ext as ext

P2 = 2
P3 = 3


# ---------------------------------------------------------------------------
# 1. Slice arithmetic & bounds
# ---------------------------------------------------------------------------


def test_slice_bounds_zero_length():
    v = ext.FpVector.from_slice(P2, [1, 0, 1])
    z = v.const[2:2]
    assert len(z) == 0
    assert z.to_list() == []


def test_slice_full_range():
    v = ext.FpVector.from_slice(P2, [1, 0, 1])
    full = v.const[0:3]
    assert full.to_list() == [1, 0, 1]


def test_slice_out_of_bounds_clamps_like_python_lists():
    """Python slice semantics: out-of-range stop is clamped, not an error."""
    v = ext.FpVector.from_slice(P2, [1, 0, 1])
    # Python lists clamp, so we do too:
    assert v.const[0:99].to_list() == [1, 0, 1]
    assert v.const[5:7].to_list() == []


def test_slice_inverted_returns_empty():
    """`v.const[2:1]` is empty (Python slice semantics)."""
    v = ext.FpVector.from_slice(P2, [1, 0, 1])
    assert v.const[2:1].to_list() == []


def test_owned_direct_slice_rejected():
    """`v[a:b]` on an owned vector should explicitly direct the user to
    `v.const[a:b]` / `v.mut[a:b]`."""
    v = ext.FpVector.from_slice(P2, [1, 0, 1])
    with pytest.raises(ValueError, match=r"v\.const|v\.mut"):
        _ = v[0:2]


def test_slice_of_slice_arithmetic():
    """Composed sub-views index into the original storage correctly."""
    v = ext.FpVector.from_slice(P2, [1, 1, 0, 1, 0, 1, 1])
    outer = v.const[1:6]        # [1, 0, 1, 0, 1]
    inner = outer[1:4]          # outer[1:4] = [0, 1, 0]
    assert outer.to_list() == [1, 0, 1, 0, 1]
    assert inner.to_list() == [0, 1, 0]
    deep = inner[1:2]           # [1]
    assert deep.to_list() == [1]


def test_slice_of_slice_clamping():
    v = ext.FpVector.from_slice(P2, list(range(5)))
    outer = v.const[1:4]        # length 3
    assert outer[0:99].to_list() == outer.to_list()


def test_matrix_row_bounds():
    m = ext.Matrix.from_vec(P2, [[1, 0], [0, 1]])
    with pytest.raises(IndexError):
        _ = m.const[2]
    with pytest.raises(IndexError):
        _ = m.mut[99]


def test_augmented_row_bounds():
    am = ext.AugmentedMatrix(P2, 2, [3, 2])
    # Out-of-range *row* (the matrix has 2 rows), valid segment.
    with pytest.raises(IndexError):
        _ = am.const[5, 0]


def test_augmented_segment_bounds():
    """Out-of-range or reversed *segment* keys must raise IndexError, not
    panic. (Regression test: this path previously bypassed validation and
    panicked / underflowed the view length.)"""
    am = ext.AugmentedMatrix(P2, 2, [3, 2])  # 2 segments
    # Segment index too large.
    with pytest.raises(IndexError):
        _ = am.const[0, 5]
    with pytest.raises(IndexError):
        _ = am.mut[0, 5]
    # Reversed segment range.
    with pytest.raises(IndexError):
        _ = am.const[0, (1, 0)]
    # Out-of-range end of a range.
    with pytest.raises(IndexError):
        _ = am.const[0, (0, 9)]
    # The segment accessor path must reject the same keys.
    with pytest.raises(IndexError):
        _ = am.segment_const[5]
    with pytest.raises(IndexError):
        _ = am.segment_const[1, 0]


def test_augmented_three_segments():
    """3-segment AugmentedMatrix: segment views over all three segments and
    the compute_image rejection path. (Note: segments are limb-padded, so
    the flat column count is larger than the sum of segment widths.)"""
    am = ext.AugmentedMatrix(P2, 2, [2, 2, 2])  # 3 segments, each 2 cols
    # Each individual segment has its logical width.
    assert am.segment_const[0].columns() == 2
    assert am.segment_const[1].columns() == 2
    assert am.segment_const[2].columns() == 2
    # A multi-segment span includes inter-segment padding.
    assert am.segment_const[0, 2].columns() >= 6
    # The last segment is square (2x2): write the identity and read it back
    # in segment-local coordinates.
    am.segment_mut[2].add_identity()
    assert am.const[0, 2].to_list() == [1, 0]
    assert am.const[1, 2].to_list() == [0, 1]
    # Mutable single-segment view, segment-local coordinates.
    s0 = am.mut[0, 0]
    s0[1] = 1
    assert am.const[0, 0].to_list() == [0, 1]
    # compute_image is only defined for 2-segment matrices.
    with pytest.raises(ValueError):
        am.compute_image()


def test_augmented_segment_bad_key():
    """Passing a non-int / non-tuple key should raise TypeError."""
    am = ext.AugmentedMatrix(P2, 2, [3, 2])
    with pytest.raises(TypeError):
        _ = am.const["nope"]


def test_slice_index_set_get():
    v = ext.FpVector.from_slice(P2, [1, 0, 0, 0, 1])
    sub = v.mut[1:4]
    assert sub[0] == 0  # was v[1]
    sub[1] = 1          # writes v[2]
    assert sub.to_list() == [0, 1, 0]
    assert v.to_list() == [1, 0, 1, 0, 1]


def test_view_indexing_int_returns_entry():
    v = ext.FpVector.from_slice(P2, [1, 0, 1])
    view = v.const
    assert view[0] == 1
    assert view[2] == 1


# ---------------------------------------------------------------------------
# 2. Read-only enforcement
# ---------------------------------------------------------------------------


def _writes_to_attempt(v):
    """Yield callables that attempt to write through `v`."""
    def _setitem():
        v[0] = 1

    yield ("__setitem__", _setitem)
    yield ("set_to_zero", lambda: v.set_to_zero())
    yield ("add_basis_element", lambda: v.add_basis_element(0, 1))


def test_read_only_view_rejects_all_writes():
    v = ext.FpVector.from_slice(P2, [1, 0, 1])
    view = v.const
    assert not view.writable
    for name, op in _writes_to_attempt(view):
        with pytest.raises(Exception, match="read-only|cannot mutate"):
            op()


def test_mut_on_read_only_view_rejected():
    """`view.mut` from a read-only view should raise."""
    v = ext.FpVector.from_slice(P2, [1, 0, 1])
    view = v.const
    with pytest.raises(Exception, match="read-only|Cannot derive"):
        _ = view.mut


def test_read_only_matrix_row_rejects_writes():
    m = ext.Matrix.from_vec(P2, [[1, 0], [0, 1]])
    row = m.const[0]
    assert not row.writable
    with pytest.raises(Exception):
        row[0] = 1


def test_read_only_segment_view_rejects_writes():
    am = ext.AugmentedMatrix(P2, 2, [2, 2])
    seg = am.const[0, 0]
    with pytest.raises(Exception):
        seg.set_to_zero()


# ---------------------------------------------------------------------------
# 3. Lifetime / GC
# ---------------------------------------------------------------------------


def test_view_keeps_parent_alive():
    """If the only Python reference to the parent is held by a view, the
    parent must not be deallocated."""
    v = ext.FpVector.from_slice(P2, [1, 0, 1])
    parent_ref = weakref.ref(v)
    view = v.const
    del v
    gc.collect()
    # Parent should still be alive (view holds a Py<FpVector>).
    assert parent_ref() is not None
    # And the view should still work.
    assert view.to_list() == [1, 0, 1]
    del view
    gc.collect()
    assert parent_ref() is None


def test_view_of_view_keeps_root_alive():
    v = ext.FpVector.from_slice(P2, [1, 0, 1, 0, 1])
    outer = v.const
    inner = outer[1:4]
    parent_ref = weakref.ref(v)
    del v
    del outer
    gc.collect()
    # `inner` holds a Py<FpVector> handle to the root, so it must stay alive.
    assert parent_ref() is not None
    assert inner.to_list() == [0, 1, 0]
    del inner
    gc.collect()
    assert parent_ref() is None


def test_matrix_view_keeps_matrix_alive():
    m = ext.Matrix.from_vec(P2, [[1, 0, 1], [1, 1, 0]])
    parent_ref = weakref.ref(m)
    row = m.mut[0]
    del m
    gc.collect()
    assert parent_ref() is not None
    row[0] = 0
    assert row.to_list() == [0, 0, 1]
    del row
    gc.collect()
    assert parent_ref() is None


def test_matrix_accessor_object_keeps_matrix_alive():
    """Holding `m.const` (a `MatrixView` object) keeps the matrix alive."""
    m = ext.Matrix.from_vec(P2, [[1, 0]])
    parent_ref = weakref.ref(m)
    accessor = m.const
    del m
    gc.collect()
    assert parent_ref() is not None
    assert accessor[0].to_list() == [1, 0]
    del accessor
    gc.collect()
    assert parent_ref() is None


# ---------------------------------------------------------------------------
# 4. Mutation visibility
# ---------------------------------------------------------------------------


def test_parent_writes_visible_through_view():
    """Mutating the parent should be reflected on subsequent reads through
    a previously-created view (no stale snapshot)."""
    v = ext.FpVector(P2, 5)
    view = v.const[1:4]
    assert view.to_list() == [0, 0, 0]
    v[2] = 1
    v[3] = 1
    assert view.to_list() == [0, 1, 1]


def test_view_writes_visible_through_parent():
    v = ext.FpVector(P2, 5)
    view = v.mut[1:4]
    view[0] = 1
    view[2] = 1
    assert v.to_list() == [0, 1, 0, 1, 0]


def test_matrix_row_view_reflects_set_entry():
    m = ext.Matrix.from_vec(P2, [[0, 0], [0, 0]])
    row0 = m.const[0]
    m[0, 1] = 1
    assert row0.to_list() == [0, 1]


def test_matrix_row_view_mut_writes_visible_in_matrix():
    m = ext.Matrix.from_vec(P2, [[0, 0], [0, 0]])
    row0 = m.mut[0]
    row0[0] = 1
    assert m[0, 0] == 1


# ---------------------------------------------------------------------------
# 5. Aliasing read-write semantics
# ---------------------------------------------------------------------------


def test_two_view_muts_alias_same_storage():
    """Two ViewMuts pointing at the same range share storage."""
    v = ext.FpVector(P2, 5)
    a = v.mut
    b = v.mut
    a[2] = 1
    assert b[2] == 1
    b[3] = 1
    assert a[3] == 1
    assert v.to_list() == [0, 0, 1, 1, 0]


def test_two_view_muts_overlapping_ranges():
    v = ext.FpVector(P2, 6)
    a = v.mut[0:4]      # [0,4)
    b = v.mut[2:6]      # [2,6) – overlaps in [2,4)
    a[3] = 1            # writes v[3]
    assert b[1] == 1    # b[1] is v[3]
    b[0] = 1            # writes v[2]
    assert a[2] == 1    # a[2] is v[2]


def test_view_and_view_mut_alias():
    """Read-only view sees writes from a sibling ViewMut."""
    v = ext.FpVector.from_slice(P2, [0, 0, 0, 0])
    ro = v.const[1:3]
    rw = v.mut[0:4]
    rw[1] = 1
    assert ro.to_list() == [1, 0]


def test_matrix_segment_aliasing():
    am = ext.AugmentedMatrix(P2, 2, [2, 2])
    a = am.mut[0, 0]              # row 0, segment 0
    b = am.mut[0, (0, 1)]         # row 0, segments 0-1 (all 4 cols, padded)
    a[1] = 1
    bits = b.to_list()
    assert sum(bits) == 1, f"expected exactly one set bit, got {bits}"


# ---------------------------------------------------------------------------
# 6. Re-entrancy via Rust-side test hook
#
# The `_test_op_during_self_borrow_mut` hook is gated behind the `test-hooks`
# cargo feature (on by default for dev builds, off for release wheels). Skip
# these tests if the extension was built without it.
# ---------------------------------------------------------------------------

_has_test_hook = hasattr(ext.Matrix, "_test_op_during_self_borrow_mut")
requires_test_hook = pytest.mark.skipif(
    not _has_test_hook,
    reason="extension built without the `test-hooks` feature",
)


@requires_test_hook
def test_borrow_check_fires_on_self_view():
    """The Rust test hook holds borrow_mut on the matrix, then tries to
    write through a view of itself. The view's borrow_mut should fail."""
    m = ext.Matrix.from_vec(P2, [[0, 0], [0, 0]])
    view = m.mut[0]
    with pytest.raises(BufferError, match="already borrowed|borrow"):
        m._test_op_during_self_borrow_mut(view)


@requires_test_hook
def test_borrow_check_does_not_fire_on_unrelated_view():
    """If the view points at a different parent, the test hook on `m1`
    only borrows `m1`, leaving `m2`-views fully usable."""
    m1 = ext.Matrix.from_vec(P2, [[0, 0]])
    m2 = ext.Matrix.from_vec(P2, [[0, 0]])
    view2 = m2.mut[0]
    # Should succeed: borrow_mut is on m1, view2 borrows m2.
    m1._test_op_during_self_borrow_mut(view2)
    assert m2[0, 0] == 1


@requires_test_hook
def test_borrow_check_with_owned_vector():
    """Owned vectors have no parent, so the test hook on a matrix should
    succeed regardless."""
    m = ext.Matrix.from_vec(P2, [[0, 0]])
    v = ext.FpVector(P2, 4)
    m._test_op_during_self_borrow_mut(v)
    assert v[0] == 1


# ---------------------------------------------------------------------------
# 7. Stress / fuzz
# ---------------------------------------------------------------------------


def _random_op(rng: random.Random, v: ext.FpVector, snapshot: list[int]) -> None:
    """Apply a random op to `v` and update `snapshot` to match."""
    p = int(v.prime)
    n = len(v)
    if n == 0:
        return
    op = rng.randrange(4)
    if op == 0:  # set_entry
        i = rng.randrange(n)
        val = rng.randrange(p)
        v[i] = val
        snapshot[i] = val
    elif op == 1:  # add_basis_element
        i = rng.randrange(n)
        c = rng.randrange(p)
        v.add_basis_element(i, c)
        snapshot[i] = (snapshot[i] + c) % p
    elif op == 2:  # set_to_zero
        v.set_to_zero()
        for i in range(len(snapshot)):
            snapshot[i] = 0
    else:  # read-back consistency
        assert v.to_list() == snapshot


@pytest.mark.parametrize("seed", range(10))
def test_stress_owned_vector(seed):
    rng = random.Random(seed)
    n = rng.randint(1, 16)
    v = ext.FpVector(P3, n)
    snapshot = [0] * n
    for _ in range(100):
        _random_op(rng, v, snapshot)
    assert v.to_list() == snapshot


@pytest.mark.parametrize("seed", range(10))
def test_stress_matrix_row_views(seed):
    """Random sequence of row mut/const accessor uses; verify the matrix
    matches a Python-side snapshot."""
    rng = random.Random(seed)
    rows, cols = rng.randint(1, 4), rng.randint(1, 8)
    m = ext.Matrix(P3, rows, cols)
    snap = [[0] * cols for _ in range(rows)]
    for _ in range(200):
        op = rng.randrange(3)
        r = rng.randrange(rows)
        if op == 0:
            # set entry directly via __setitem__
            c = rng.randrange(cols)
            v = rng.randrange(3)
            m[r, c] = v
            snap[r][c] = v
        elif op == 1:
            # write through a mut[r]
            view = m.mut[r]
            c = rng.randrange(cols)
            v = rng.randrange(3)
            view[c] = v
            snap[r][c] = v
        else:
            # read back via const[r]
            view = m.const[r]
            assert view.to_list() == snap[r]
    assert m.to_list() == snap


@pytest.mark.parametrize("seed", range(5))
def test_stress_overlapping_slices(seed):
    """Compose random sub-slices and check that writes through one are
    visible through another."""
    rng = random.Random(seed)
    n = 32
    v = ext.FpVector(P2, n)
    snap = [0] * n
    for _ in range(200):
        s = rng.randrange(n)
        e = rng.randint(s, n)
        is_mut = rng.random() < 0.5 and (e > s)
        if is_mut and e > s:
            view = v.mut[s:e]
            i = rng.randrange(e - s)
            val = rng.randrange(2)
            view[i] = val
            snap[s + i] = val
        else:
            view = v.const[s:e]
            assert view.to_list() == snap[s:e]
    assert v.to_list() == snap
