import pytest

from ext import fp


def test_matrix_construction_and_queries():
    m = fp.Matrix(7, 2, 3)
    assert m.prime() == 7
    assert m.rows() == 2
    assert m.columns() == 3
    assert m.is_zero()
    assert len(m) == 2
    assert m.to_vec() == [[0, 0, 0], [0, 0, 0]]
    assert repr(m).startswith("Matrix(7, ")


def test_matrix_from_vec_and_identity():
    m = fp.Matrix.from_vec(7, [[1, 3, 6], [0, 3, 4]])
    assert m.to_vec() == [[1, 3, 6], [0, 3, 4]]
    assert not m.is_zero()

    ident = fp.Matrix.identity(5, 3)
    assert ident.to_vec() == [[1, 0, 0], [0, 1, 0], [0, 0, 1]]


def test_matrix_from_rows_and_from_row():
    r0 = fp.FpVector.from_slice(5, [1, 2, 3])
    r1 = fp.FpVector.from_slice(5, [4, 0, 1])
    m = fp.Matrix.from_rows(5, [r0, r1], 3)
    assert m.to_vec() == [[1, 2, 3], [4, 0, 1]]

    single = fp.Matrix.from_row(5, r0, 3)
    assert single.to_vec() == [[1, 2, 3]]


def test_matrix_augmented_from_vec():
    first_source, m = fp.Matrix.augmented_from_vec(7, [[1, 3, 6], [0, 3, 4]])
    assert first_source >= 3
    assert m.rows() == 2


def test_prime_is_int():
    m = fp.Matrix(5, 1, 1)
    assert isinstance(m.prime(), int)


def test_invalid_prime_and_dims():
    with pytest.raises(ValueError):
        fp.Matrix(1, 2, 2)
    with pytest.raises(ValueError):
        fp.Matrix.from_vec(4, [[1, 2]])
    with pytest.raises(ValueError):
        fp.Matrix.from_vec(7, [[1, 2], [3]])


def test_row_access_and_getitem():
    m = fp.Matrix.from_vec(5, [[1, 2, 3], [4, 0, 1]])
    row = m.row(1)
    assert row.prime() == 5
    assert len(row) == 3
    assert row.entry(0) == 4
    assert row[2] == 1
    assert row[-1] == 1
    assert not row.is_zero()
    assert row.first_nonzero() == (0, 4)
    assert list(row.iter()) == [4, 0, 1]
    assert row.iter_nonzero() == [(0, 4), (2, 1)]
    assert m[0].to_owned().prime() == 5
    assert list(m[0].iter()) == [1, 2, 3]

    with pytest.raises(IndexError):
        m.row(2)
    with pytest.raises(IndexError):
        row.entry(3)


def test_row_mut_reflects_in_parent():
    m = fp.Matrix.from_vec(5, [[1, 2, 3], [4, 0, 1]])
    rm = m.row_mut(0)
    rm.set_entry(0, 9)
    assert m.to_vec()[0] == [4, 2, 3]
    rm[1] = 3
    assert m.row(0)[1] == 3
    rm.scale(2)
    assert m.to_vec()[0] == [3, 1, 1]
    rm.set_to_zero()
    assert m.to_vec()[0] == [0, 0, 0]
    rm.add_basis_element(2, 1)
    assert m.to_vec()[0] == [0, 0, 1]

    with pytest.raises(IndexError):
        rm.set_entry(3, 1)


def test_row_mut_add_slice():
    m = fp.Matrix.from_vec(5, [[1, 2, 3]])
    other = fp.FpVector.from_slice(5, [1, 1, 1])
    m.row_mut(0).add(other.slice(0, 3), 2)
    assert m.to_vec()[0] == [3, 4, 0]


def test_mutators():
    m = fp.Matrix.from_vec(5, [[1, 2], [3, 4]])
    m.swap_rows(0, 1)
    assert m.to_vec() == [[3, 4], [1, 2]]
    with pytest.raises(IndexError):
        m.swap_rows(0, 2)

    m.safe_row_op(0, 1, 1)
    assert m.to_vec() == [[4, 1], [1, 2]]
    with pytest.raises(ValueError):
        m.safe_row_op(0, 0, 1)

    m.set_to_zero()
    assert m.is_zero()


def test_add_row_extends_matrix():
    m = fp.Matrix(5, 1, 2)
    new = m.add_row()
    assert m.rows() == 2
    new.set_entry(0, 3)
    assert m.to_vec()[1] == [3, 0]


def test_assign_requires_matching_shape():
    m = fp.Matrix.from_vec(5, [[1, 2], [3, 4]])
    other = fp.Matrix.from_vec(5, [[1, 1], [1, 1]])
    m.assign(other)
    assert m.to_vec() == [[1, 1], [1, 1]]

    mismatch = fp.Matrix(5, 3, 2)
    with pytest.raises(ValueError):
        m.assign(mismatch)
    diff_prime = fp.Matrix(7, 2, 2)
    with pytest.raises(ValueError):
        m.assign(diff_prime)


def test_row_reduce_rank():
    m = fp.Matrix.from_vec(2, [[1, 1, 0], [0, 1, 1], [1, 0, 1]])
    assert m.row_reduce() == 2

    pivots = m.pivots()
    assert isinstance(pivots, list)


def test_extend_columns_and_pivots():
    m = fp.Matrix(5, 2, 2)
    m.extend_column_dimension(4)
    assert m.columns() == 4
    m.initialize_pivots()
    assert m.pivots() == [-1, -1, -1, -1]


def test_trim_and_rotate():
    m = fp.Matrix.from_vec(5, [[1, 2, 3], [4, 0, 1], [2, 2, 2]])
    m.trim(0, 2, 1)
    assert m.to_vec() == [[2, 3], [0, 1]]

    n = fp.Matrix.from_vec(5, [[1, 0], [2, 0], [3, 0]])
    n.rotate_down(0, 3, 1)
    assert n.to_vec() == [[3, 0], [1, 0], [2, 0]]


def test_bytes_roundtrip():
    m = fp.Matrix.from_vec(5, [[1, 2, 3], [4, 0, 2]])
    data = m.to_bytes()
    n = fp.Matrix.from_bytes(5, 2, 3, data)
    assert n.to_vec() == m.to_vec()


def test_stale_row_handle_after_trim_raises():
    m = fp.Matrix.from_vec(5, [[1, 2, 3], [4, 0, 1], [2, 2, 2]])
    row = m.row(2)
    assert row.entry(0) == 2
    m.trim(0, 1, 0)
    assert m.rows() == 1
    with pytest.raises(IndexError):
        row.entry(0)
    with pytest.raises(IndexError):
        row[0]
