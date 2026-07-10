import pytest

from ext import fp


def test_fp_vector_slice_queries_and_to_owned():
    v = fp.FpVector.from_py(5, [0, 1, 7, 0, 4])

    s = v.slice(1, 4)

    assert s.prime == 5
    assert len(s) == 3
    assert not s.is_empty
    assert s.entry(1) == 2
    assert s[1] == 2
    assert s[-1] == 0
    assert not s.is_zero
    assert s.first_nonzero == (0, 1)
    assert list(s.iter()) == [1, 2, 0]
    assert s.iter_nonzero() == [(0, 1), (1, 2)]
    assert repr(s) == "FpSlice(5, [1, 2, 0])"

    restricted = s.restrict(1, 3)
    assert len(restricted) == 2
    assert [restricted[i] for i in range(len(restricted))] == [2, 0]

    owned = s.to_owned()
    assert isinstance(owned, fp.FpVector)
    assert repr(owned) == "FpVector(5, [1, 2, 0])"


def test_fp_vector_restrict_returns_subslice():
    v = fp.FpVector.from_py(5, [0, 1, 7, 0, 4])

    r = v.restrict(1, 4)
    assert isinstance(r, fp.FpSlice)
    assert len(r) == 3
    assert r.iter_nonzero() == [(0, 1), (1, 2)]
    # Mirrors slice(start, end) for an FpVector.
    assert r.iter_nonzero() == v.slice(1, 4).iter_nonzero()
    assert repr(r.to_owned()) == "FpVector(5, [1, 2, 0])"

    with pytest.raises(IndexError):
        v.restrict(0, 6)
    with pytest.raises(IndexError):
        v.restrict(3, 2)


def test_fp_vector_iter_nonzero():
    v = fp.FpVector.from_py(5, [0, 1, 7, 0, 4])
    assert v.iter_nonzero() == [(1, 1), (2, 2), (4, 4)]
    # Matches the full-vector slice.
    assert v.iter_nonzero() == v.slice(0, len(v)).iter_nonzero()


def test_fp_vector_to_owned_is_independent_clone():
    v = fp.FpVector.from_py(5, [0, 1, 7, 0, 4])
    owned = v.to_owned()
    assert isinstance(owned, fp.FpVector)
    assert list(owned) == list(v)
    # Mutating the source does not affect the clone.
    v.set_entry(0, 3)
    assert owned[0] == 0


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
    v = fp.FpVector.from_py(5, [1, 2, 3, 4])
    s = v.slice(1, 3)
    sm = v.slice_mut(2, 4)

    del v

    assert list(s.iter()) == [2, 3]
    sm.add_basis_element(0, 4)
    assert s[1] == 2
    assert list(sm.iter()) == [2, 4]


def test_stale_slice_handles_raise_python_exception_after_parent_shrink():
    v = fp.FpVector.from_py(5, [1, 2, 3, 4])
    s = v.slice(1, 4)
    sm = v.slice_mut(1, 4)

    v.set_scratch_vector_size(2)

    with pytest.raises(IndexError):
        s.prime
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
    v = fp.FpVector.from_py(5, [1, 2, 3, 4, 0])
    s = v.slice_mut(1, 4)

    assert s.prime == 5
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

    assert len(s) == 3
    assert [s[i] for i in range(len(s))] == [4, 3, 4]

    sub = s.slice_mut(1, 3)
    sub.set_to_zero()
    assert [v[i] for i in range(len(v))] == [1, 4, 0, 0, 0]


def test_fp_slice_mut_add_assign_and_masked_methods():
    v = fp.FpVector.from_py(5, [1, 0, 0, 0, 0, 0])
    target = v.slice_mut(1, 4)
    source_v = fp.FpVector.from_py(5, [1, 2, 3, 4])
    source = source_v.slice(0, 3)

    target.add(source, 2)
    assert [v[i] for i in range(len(v))] == [1, 2, 4, 1, 0, 0]

    target.assign(fp.FpVector.from_py(5, [4, 0, 1]).slice(0, 3))
    assert [v[i] for i in range(len(v))] == [1, 4, 0, 1, 0, 0]


def test_fp_slice_mut_add_tensor():
    v = fp.FpVector(5, 6)

    v.slice_mut(0, 6).add_tensor(
        1,
        2,
        fp.FpVector.from_py(5, [1, 2]).slice(0, 2),
        fp.FpVector.from_py(5, [3, 4]).slice(0, 2),
    )

    assert [v[i] for i in range(len(v))] == [0, 1, 3, 2, 1, 0]

def test_fp_slice_mut_assign_distinct_and_aliased():
    vt = fp.FpVector.from_py(5, [1, 2, 3])
    vo = fp.FpVector.from_py(5, [4, 0, 1])
    vt.slice_mut(0, 3).assign(vo.slice(0, 3))
    assert [vt[i] for i in range(3)] == [4, 0, 1]

    # Aliased clone fallback over the same vector (non-overlapping halves).
    v = fp.FpVector.from_py(5, [1, 2, 3, 4, 0, 1])
    v.slice_mut(0, 3).assign(v.slice(3, 6))
    assert [v[i] for i in range(6)] == [4, 0, 1, 4, 0, 1]


def test_fp_slice_mut_add_tensor_distinct_and_aliased():
    # Distinct operands, distinct target (no-clone path).
    vt = fp.FpVector(5, 6)
    left = fp.FpVector.from_py(5, [1, 2])
    right = fp.FpVector.from_py(5, [3, 4])
    vt.slice_mut(0, 6).add_tensor(1, 2, left.slice(0, 2), right.slice(0, 2))
    assert [vt[i] for i in range(6)] == [0, 1, 3, 2, 1, 0]

    # Dual-operand left == right over the SAME vector (two shared borrows of
    # one operand coexist; neither aliases the distinct target).
    vt2 = fp.FpVector(5, 6)
    w = fp.FpVector.from_py(5, [1, 2])
    vt2.slice_mut(0, 6).add_tensor(1, 2, w.slice(0, 2), w.slice(0, 2))
    assert [vt2[i] for i in range(6)] == [0, 2, 4, 4, 3, 0]

    # An operand aliasing the target: the clone fallback must read the operand's
    # ORIGINAL contents even though the target overwrites them mid-op.
    v = fp.FpVector.from_py(5, [1, 2, 0, 0, 0, 0])
    r = fp.FpVector.from_py(5, [3, 4])
    v.slice_mut(0, 6).add_tensor(1, 2, v.slice(0, 2), r.slice(0, 2))
    assert [v[i] for i in range(6)] == [1, 3, 3, 2, 1, 0]


def test_fp_slice_mut_matrix_row_aliasing_clone_fallback():
    # Two slices over different rows of the SAME matrix are conservatively
    # treated as aliased (the whole Matrix pyclass is borrowed as a unit), so
    # this takes the clone fallback. The result must still be correct.
    m = fp.Matrix.from_py(5, [[1, 1, 1], [2, 2, 2]])
    m.row_mut(0).add(m.row(1), 1)
    assert m.to_py() == [[3, 3, 3], [2, 2, 2]]

    # assign across rows of one matrix likewise clones but stays correct.
    m2 = fp.Matrix.from_py(5, [[1, 2, 3], [4, 0, 1]])
    m2.row_mut(0).assign(m2.row(1))
    assert m2.to_py() == [[4, 0, 1], [4, 0, 1]]


def test_fp_slice_mut_accepts_fp_vector_operand():
    # The operand-taking FpSliceMut methods accept either an FpSlice or a
    # (full) FpVector, coercing the FpVector via its full-vector slice.
    v = fp.FpVector.from_py(5, [1, 0, 0, 0, 0, 0])
    target = v.slice_mut(1, 4)
    operand = fp.FpVector.from_py(5, [1, 2, 3])

    target.add(operand, 2)
    assert [v[i] for i in range(len(v))] == [1, 2, 4, 1, 0, 0]

    target.assign(operand)
    assert [v[i] for i in range(len(v))] == [1, 1, 2, 3, 0, 0]

    # An FpVector operand whose backing object IS the target's parent takes the
    # clone fallback and must still be correct.
    w = fp.FpVector.from_py(5, [1, 2, 3])
    w.slice_mut(0, 3).add(w, 1)
    assert [w[i] for i in range(len(w))] == [2, 4, 1]

    # add_tensor accepts FpVector operands too.
    out = fp.FpVector(5, 6)
    left = fp.FpVector.from_py(5, [1, 2])
    right = fp.FpVector.from_py(5, [3, 4])
    out.slice_mut(0, 6).add_tensor(1, 2, left, right)
    assert [out[i] for i in range(6)] == [0, 1, 3, 2, 1, 0]

    # A matrix row_mut accepting an FpVector operand (the secondary_massey.py
    # pattern: out.slice_mut(...).add(g, 1) where g is an FpVector).
    m = fp.Matrix.from_py(5, [[1, 1, 1], [2, 2, 2]])
    m.row_mut(0).add(fp.FpVector.from_py(5, [3, 0, 1]), 1)
    assert m.to_py() == [[4, 1, 2], [2, 2, 2]]


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
        target.add_tensor(
            1,
            1,
            fp.FpVector.from_py(5, [1, 2]).slice(0, 2),
            fp.FpVector.from_py(5, [1, 2]).slice(0, 2),
        )


def test_fp_slice_mut_to_owned():
    v = fp.FpVector.from_py(5, [1, 2, 3, 4, 0])
    s = v.slice_mut(1, 4)

    owned = s.to_owned()
    assert isinstance(owned, fp.FpVector)
    assert owned.prime == 5
    assert repr(owned) == "FpVector(5, [2, 3, 4])"

    # to_owned is a copy: mutating the source does not change the owned vector.
    s.set_entry(0, 0)
    assert repr(owned) == "FpVector(5, [2, 3, 4])"


def test_fp_slice_len_revalidates_after_parent_shrink():
    v = fp.FpVector.from_py(5, [1, 2, 3, 4])
    s = v.slice(1, 4)
    sm = v.slice_mut(1, 4)

    assert len(s) == 3
    assert len(sm) == 3
    assert not s.is_empty

    v.set_scratch_vector_size(2)

    with pytest.raises(IndexError):
        len(s)
    with pytest.raises(IndexError):
        s.is_empty
    with pytest.raises(IndexError):
        len(sm)
    with pytest.raises(IndexError):
        sm.is_empty


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
