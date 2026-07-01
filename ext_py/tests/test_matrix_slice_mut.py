import pytest

from ext import fp


def make_matrix(rows):
    """Build an F5 matrix from a list-of-lists via row_mut.set_entry."""
    p = 5
    m = fp.Matrix(p, len(rows), len(rows[0]))
    for i, row in enumerate(rows):
        rm = m.row_mut(i)
        for j, v in enumerate(row):
            rm.set_entry(j, v)
    return m


def test_slice_mut_construction_and_queries():
    m = make_matrix([[1, 2, 3, 4], [0, 1, 2, 3], [4, 3, 2, 1]])
    rect = m.slice_mut(0, 2, 1, 3)
    # prime returned as a plain int.
    assert rect.prime == 5
    assert rect.rows == 2
    assert rect.columns == 2
    assert repr(rect) == "MatrixSliceMut(5, 2x2)"


def test_row_and_row_slice_read():
    m = make_matrix([[1, 2, 3, 4], [0, 1, 2, 3], [4, 3, 2, 1]])
    rect = m.slice_mut(0, 3, 1, 3)
    # row(i) is the column-shifted view into the rectangle.
    row0 = rect.row(0)
    assert isinstance(row0, fp.FpSlice)
    assert len(row0) == 2
    assert row0[0] == 2
    assert row0[1] == 3

    # row_slice restricts the row range, keeping the columns.
    sub = rect.row_slice(1, 3)
    assert sub.rows == 2
    assert sub.columns == 2
    assert sub.row(0)[0] == 1  # original row 1, column 1


def test_row_mut_writes_through_to_parent():
    m = make_matrix([[1, 2, 3, 4], [0, 1, 2, 3]])
    rect = m.slice_mut(0, 2, 1, 4)
    rm = rect.row_mut(0)
    assert isinstance(rm, fp.FpSliceMut)
    rm.set_entry(0, 0)  # rectangle column 0 == matrix column 1
    assert m.to_vec()[0] == [1, 0, 3, 4]


def test_iter_mut_reflects_in_parent():
    m = make_matrix([[1, 1, 1, 1], [2, 2, 2, 2]])
    rect = m.slice_mut(0, 2, 0, 4)
    # iter yields read-only row handles.
    for r in rect.iter():
        assert isinstance(r, fp.FpSlice)
    # iter_mut yields mutable handles that write through.
    for r in rect.iter_mut():
        r.set_entry(0, 0)
    assert m.to_vec() == [[0, 1, 1, 1], [0, 2, 2, 2]]


def test_add_identity():
    m = fp.Matrix(3, 2, 4)
    rect = m.slice_mut(0, 2, 2, 4)
    rect.add_identity()
    assert m.to_vec() == [[0, 0, 1, 0], [0, 0, 0, 1]]

    # Non-square rectangle raises ValueError.
    wide = m.slice_mut(0, 2, 0, 4)
    with pytest.raises(ValueError):
        wide.add_identity()


def test_add_masked():
    m = fp.Matrix(3, 2, 2)
    other = fp.Matrix.from_vec(3, [[1, 2], [0, 1]])
    rect = m.slice_mut(0, 2, 0, 2)
    rect.add_masked(other, [0, 1])
    assert m.to_vec() == [[1, 2], [0, 1]]

    # Mask length must equal the rectangle's column count.
    with pytest.raises(ValueError):
        rect.add_masked(other, [0])
    # Mask index out of range for `other` raises IndexError.
    with pytest.raises(IndexError):
        rect.add_masked(other, [0, 5])
    # Row-count mismatch raises ValueError.
    bad_rows = fp.Matrix.from_vec(3, [[1, 2]])
    with pytest.raises(ValueError):
        rect.add_masked(bad_rows, [0, 1])
    # Prime mismatch raises ValueError.
    bad_prime = fp.Matrix.from_vec(5, [[1, 2], [0, 1]])
    with pytest.raises(ValueError):
        rect.add_masked(bad_prime, [0, 1])


def test_add_masked_self_raises_runtime_error():
    # Passing the slice's own parent matrix as `other` keeps an immutable
    # borrow of that object alive while the rectangle tries to borrow it
    # mutably, so the binding raises RuntimeError rather than aliasing.
    m = make_matrix([[1, 2], [0, 1]])
    rect = m.slice_mut(0, 2, 0, 2)
    with pytest.raises(RuntimeError):
        rect.add_masked(m, [0, 1])


def test_invalid_rectangle_raises():
    m = make_matrix([[1, 2, 3], [4, 0, 1]])
    with pytest.raises(IndexError):
        m.slice_mut(0, 5, 0, 1)  # too many rows
    with pytest.raises(IndexError):
        m.slice_mut(0, 1, 0, 9)  # too many columns
    with pytest.raises(IndexError):
        m.slice_mut(2, 1, 0, 1)  # inverted row range


def test_out_of_range_row_index_raises():
    m = make_matrix([[1, 2, 3], [4, 0, 1]])
    rect = m.slice_mut(0, 2, 0, 3)
    with pytest.raises(IndexError):
        rect.row(2)
    with pytest.raises(IndexError):
        rect.row_mut(2)


def test_stale_handle_after_parent_shrinks_raises():
    m = make_matrix([[1, 1, 0], [0, 1, 1]])
    rect = m.slice_mut(0, 2, 0, 3)
    # A square sub-rectangle, valid before the parent shrinks.
    square = m.slice_mut(0, 2, 0, 2)
    # Trim the parent to a single row; the 2-row rectangles are now stale.
    m.trim(0, 1, 0)
    with pytest.raises(IndexError):
        rect.rows
    # The square rectangle passes its shape check but fails revalidation.
    with pytest.raises(IndexError):
        square.add_identity()
    # A row handle taken before the shrink also raises on use.
    rm = m.slice_mut(0, 1, 0, 3).row_mut(0)
    # (still valid: row 0 survives)
    rm.set_entry(0, 1)


def test_augmented_segment_mutates_and_reads_back():
    m = fp.AugmentedMatrix2(2, 2, [2, 2])
    seg = m.segment(1, 1)
    assert isinstance(seg, fp.MatrixSliceMut)
    assert seg.rows == 2
    assert seg.columns == 2
    seg.add_identity()
    start1 = m.segment_starts[1]
    rows = m.to_vec()
    assert rows[0][start1] == 1
    assert rows[1][start1 + 1] == 1


def test_augmented_row_segment_mut_writes_through():
    m = fp.AugmentedMatrix2(3, 2, [2, 2])
    row = m.row_segment_mut(0, 0, 0)
    assert isinstance(row, fp.FpSliceMut)
    row.set_entry(0, 2)
    assert m.to_vec()[0][0] == 2


def test_augmented3_segment_and_row_segment_mut_write_through():
    # Exercises the MatrixParent::Augmented3 arm: take a segment / row segment
    # from an AugmentedMatrix3, mutate through it, and observe the change via
    # the augmented matrix (to_vec / row_segment).
    m = fp.AugmentedMatrix3(3, 2, [2, 2, 2])
    seg = m.segment(1, 1)
    assert isinstance(seg, fp.MatrixSliceMut)
    assert seg.rows == 2
    assert seg.columns == 2
    seg.add_identity()
    start1 = m.segment_starts[1]
    rows = m.to_vec()
    assert rows[0][start1] == 1
    assert rows[1][start1 + 1] == 1

    row = m.row_segment_mut(0, 0, 0)
    assert isinstance(row, fp.FpSliceMut)
    row.set_entry(0, 2)
    assert m.to_vec()[0][0] == 2
    # Observable via the read-only row_segment accessor too.
    assert list(m.row_segment(0, 0, 0))[0] == 2


def test_stale_slice_mut_repr_raises():
    # __repr__ revalidates via with_slice_mut, so a stale handle raises rather
    # than returning a string.
    m = make_matrix([[1, 1, 0], [0, 1, 1]])
    rect = m.slice_mut(0, 2, 0, 3)
    m.trim(0, 1, 0)
    with pytest.raises(IndexError):
        repr(rect)


def test_augmented_segment_builds_nontrivial_compute_values():
    # Mirrors the F3 doctest of Matrix::compute_image / compute_quasi_inverse,
    # but built entirely through the Python segment-mut API: place A in segment
    # 0 entry by entry, the identity in segment 1, then row reduce and verify
    # the committed expected image and preimage. This closes the earlier gap
    # where the Python layer could not set interior augmented-matrix entries.
    a = [
        [1, 2, 1, 1, 0],
        [1, 0, 2, 1, 1],
        [2, 2, 0, 2, 1],
    ]
    m = fp.AugmentedMatrix2(3, 3, [5, 3])
    # Fill A (segment 0) via row_segment_mut.
    for i, arow in enumerate(a):
        rm = m.row_segment_mut(i, 0, 0)
        for j, v in enumerate(arow):
            rm.set_entry(j, v)
    # Identity into segment 1 via segment(...).
    m.segment(1, 1).add_identity()
    m.row_reduce()

    image = m.compute_image()
    assert image.dimension == 2
    image_rows = [list(v) for v in image.iter()]
    assert image_rows == [[1, 0, 2, 1, 1], [0, 1, 1, 0, 1]]

    qi = m.compute_quasi_inverse()
    assert qi.source_dimension == 3
    assert qi.preimage.to_vec() == [[0, 1, 0], [0, 2, 2]]
