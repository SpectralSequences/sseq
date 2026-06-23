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
    assert list(s.iter()) == [1, 2, 0]
    assert s.iter_nonzero() == [(0, 1), (1, 2)]
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


def test_fp_slice_handles_parent_lifetime_after_original_reference_deleted():
    v = fp.FpVector.from_slice(5, [1, 2, 3, 4])
    s = v.slice(1, 3)
    sm = v.slice_mut(2, 4)

    del v

    assert list(s.iter()) == [2, 3]
    sm.add_basis_element(0, 4)
    assert s[1] == 2
    assert list(sm.as_slice().iter()) == [2, 4]


def test_stale_slice_handles_raise_python_exception_after_parent_shrink():
    v = fp.FpVector.from_slice(5, [1, 2, 3, 4])
    s = v.slice(1, 4)
    sm = v.slice_mut(1, 4)

    v.set_scratch_vector_size(2)

    with pytest.raises(IndexError):
        s.prime()
    with pytest.raises(IndexError):
        s[0]
    with pytest.raises(IndexError):
        repr(s)
    with pytest.raises(IndexError):
        sm[0]
    with pytest.raises(IndexError):
        sm.set_entry(0, 1)
    with pytest.raises(IndexError):
        sm.set_to_zero()


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


def test_fp_slice_mut_add_assign_and_masked_methods():
    v = fp.FpVector.from_slice(5, [1, 0, 0, 0, 0, 0])
    target = v.slice_mut(1, 4)
    source_v = fp.FpVector.from_slice(5, [1, 2, 3, 4])
    source = source_v.slice(0, 3)

    target.add(source, 2)
    assert [v[i] for i in range(len(v))] == [1, 2, 4, 1, 0, 0]

    target.add_offset(source, 1, 1)
    assert [v[i] for i in range(len(v))] == [1, 2, 1, 4, 0, 0]

    target.assign(fp.FpVector.from_slice(5, [4, 0, 1]).slice(0, 3))
    assert [v[i] for i in range(len(v))] == [1, 4, 0, 1, 0, 0]

    target.add_masked(source_v.slice(0, 4), 2, [3, 0, 2])
    assert [v[i] for i in range(len(v))] == [1, 2, 2, 2, 0, 0]

    target.add_unmasked(fp.FpVector.from_slice(5, [1, 0, 4]).slice(0, 3), 3, [2, 0, 1])
    assert [v[i] for i in range(len(v))] == [1, 2, 4, 0, 0, 0]


def test_fp_slice_mut_add_tensor():
    v = fp.FpVector(5, 6)

    v.slice_mut(0, 6).add_tensor(
        1,
        2,
        fp.FpVector.from_slice(5, [1, 2]).slice(0, 2),
        fp.FpVector.from_slice(5, [3, 4]).slice(0, 2),
    )

    assert [v[i] for i in range(len(v))] == [0, 1, 3, 2, 1, 0]


def test_fp_slice_mut_new_method_errors_are_python_exceptions():
    target_v = fp.FpVector(5, 3)
    target = target_v.slice_mut(0, 3)
    same_prime_short = fp.FpVector(5, 2).slice(0, 2)
    other_prime = fp.FpVector(7, 3).slice(0, 3)

    with pytest.raises(ValueError):
        target.add(same_prime_short, 1)
    with pytest.raises(ValueError):
        target.add(other_prime, 1)
    with pytest.raises(IndexError):
        target.add_offset(fp.FpVector(5, 3).slice(0, 3), 1, 4)
    with pytest.raises(ValueError):
        target.add_masked(fp.FpVector(5, 3).slice(0, 3), 1, [0, 1])
    with pytest.raises(IndexError):
        target.add_masked(fp.FpVector(5, 3).slice(0, 3), 1, [0, 1, 3])
    with pytest.raises(ValueError):
        target.add_unmasked(fp.FpVector(5, 3).slice(0, 3), 1, [0, 1])
    with pytest.raises(IndexError):
        target.add_unmasked(fp.FpVector(5, 3).slice(0, 3), 1, [0, 1, 3])
    with pytest.raises(IndexError):
        target.add_tensor(
            1,
            1,
            fp.FpVector.from_slice(5, [1, 2]).slice(0, 2),
            fp.FpVector.from_slice(5, [1, 2]).slice(0, 2),
        )


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
