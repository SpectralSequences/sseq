import pytest

from ext import fp


def test_fp_vector_slice_queries_and_to_owned():
    v = fp.FpVector.from_slice(5, [0, 1, 7, 0, 4])

    s = v.slice(1, 4)

    assert s.prime() == 5
    assert len(s) == 3
    assert not s.is_empty()
    assert s.entry(1) == 2
    assert s[1] == 2
    assert s[-1] == 0
    assert not s.is_zero()
    assert s.first_nonzero() == (0, 1)
    assert repr(s) == "FpSlice(5, [1, 2, 0])"

    restricted = s.restrict(1, 3)
    assert len(restricted) == 2
    assert [restricted[i] for i in range(len(restricted))] == [2, 0]

    owned = s.to_owned()
    assert isinstance(owned, fp.FpVector)
    assert repr(owned) == "FpVector(5, [1, 2, 0])"


def test_fp_vector_slice_range_and_index_errors():
    v = fp.FpVector(3, 4)
    s = v.slice(1, 3)

    with pytest.raises(IndexError):
        v.slice(3, 2)
    with pytest.raises(IndexError):
        v.slice(0, 5)
    with pytest.raises(IndexError):
        v.slice_mut(0, 5)
    with pytest.raises(IndexError):
        s.entry(2)
    with pytest.raises(IndexError):
        s[2]
    with pytest.raises(IndexError):
        s[-3]
    with pytest.raises(IndexError):
        s.restrict(1, 3)


def test_fp_slice_mut_updates_parent_and_as_slice():
    v = fp.FpVector.from_slice(5, [1, 2, 3, 4, 0])
    s = v.slice_mut(1, 4)

    assert s.prime() == 5
    assert len(s) == 3
    assert s[0] == 2
    assert s[-1] == 4
    assert repr(s) == "FpSliceMut(5, [2, 3, 4])"

    s.set_entry(0, 7)
    assert v[1] == 2

    s[1] = 9
    assert v[2] == 4

    s.add_basis_element(2, 3)
    assert v[3] == 2

    s.scale(2)
    assert [v[i] for i in range(len(v))] == [1, 4, 3, 4, 0]

    as_slice = s.as_slice()
    assert len(as_slice) == 3
    assert [as_slice[i] for i in range(len(as_slice))] == [4, 3, 4]

    sub = s.slice_mut(1, 3)
    sub.set_to_zero()
    assert [v[i] for i in range(len(v))] == [1, 4, 0, 0, 0]


def test_fp_slice_mut_index_and_range_errors():
    v = fp.FpVector(2, 3)
    s = v.slice_mut(1, 3)

    with pytest.raises(IndexError):
        s.set_entry(2, 1)
    with pytest.raises(IndexError):
        s.add_basis_element(2, 1)
    with pytest.raises(IndexError):
        s[2]
    with pytest.raises(IndexError):
        s[-3]
    with pytest.raises(IndexError):
        s[2] = 1
    with pytest.raises(IndexError):
        s.slice_mut(2, 1)
    with pytest.raises(IndexError):
        s.slice_mut(0, 3)
