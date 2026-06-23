import pytest

from ext import fp


def make_subspace(p, rows):
    s = fp.Subspace(p, len(rows[0]))
    for row in rows:
        s.add_vector(fp.FpVector.from_slice(p, row))
    return s


def test_construction_and_queries():
    linear = make_subspace(2, [[0, 1, 0], [0, 0, 1]])
    offset = fp.FpVector.from_slice(2, [1, 0, 0])
    aff = fp.AffineSubspace(offset, linear)

    # Prime is passed/queried as a plain int.
    assert aff.prime() == 2
    assert aff.ambient_dimension() == 3
    assert aff.dimension() == 2
    assert repr(aff) == "AffineSubspace([1, 0, 0] + {[0, 1, 0], [0, 0, 1]})"


def test_offset_and_linear_part_roundtrip():
    linear = make_subspace(2, [[0, 1, 0], [0, 0, 1]])
    offset = fp.FpVector.from_slice(2, [1, 0, 0])
    aff = fp.AffineSubspace(offset, linear)

    # offset() returns an owned FpVector (the input reduced against the
    # linear part); here it is unchanged because [1,0,0] is already reduced.
    stored = aff.offset()
    assert isinstance(stored, fp.FpVector)
    assert list(stored) == [1, 0, 0]

    # linear_part() returns an owned Subspace clone.
    lp = aff.linear_part()
    assert isinstance(lp, fp.Subspace)
    assert lp.dimension() == 2
    assert lp.ambient_dimension() == 3
    assert [list(v) for v in lp.iter()] == [[0, 1, 0], [0, 0, 1]]


def test_offset_is_reduced():
    # Offset [1, 1, 0] reduces to [1, 0, 0] against span{[0,1,0],[0,0,1]}.
    linear = make_subspace(2, [[0, 1, 0], [0, 0, 1]])
    offset = fp.FpVector.from_slice(2, [1, 1, 0])
    aff = fp.AffineSubspace(offset, linear)
    assert list(aff.offset()) == [1, 0, 0]


def test_contains_membership():
    linear = make_subspace(2, [[0, 1, 0], [0, 0, 1]])
    offset = fp.FpVector.from_slice(2, [1, 0, 0])
    aff = fp.AffineSubspace(offset, linear)

    # [1,1,0] = offset + [0,1,0] is in the affine subspace.
    assert aff.contains(fp.FpVector.from_slice(2, [1, 1, 0]))
    assert fp.FpVector.from_slice(2, [1, 1, 0]) in aff

    # [0,1,0] has the wrong first coordinate, so it is not contained.
    assert not aff.contains(fp.FpVector.from_slice(2, [0, 1, 0]))
    assert fp.FpVector.from_slice(2, [0, 1, 0]) not in aff


def test_contains_accepts_slice():
    linear = make_subspace(2, [[0, 1, 0], [0, 0, 1]])
    offset = fp.FpVector.from_slice(2, [1, 0, 0])
    aff = fp.AffineSubspace(offset, linear)

    v = fp.FpVector.from_slice(2, [1, 1, 0])
    assert aff.contains(v.slice(0, 3))


def test_contains_space():
    # a = origin + span{[0,1,0],[0,0,1]} (a linear subspace through 0).
    a = fp.AffineSubspace(
        fp.FpVector.from_slice(2, [0, 0, 0]),
        make_subspace(2, [[0, 1, 0], [0, 0, 1]]),
    )
    # b = [0,1,0] + span{[0,1,0]}: both its linear part and offset lie in a.
    b = fp.AffineSubspace(
        fp.FpVector.from_slice(2, [0, 1, 0]),
        make_subspace(2, [[0, 1, 0]]),
    )
    # c = [1,0,0] + span{[0,1,0]}: its offset is outside a.
    c = fp.AffineSubspace(
        fp.FpVector.from_slice(2, [1, 0, 0]),
        make_subspace(2, [[0, 1, 0]]),
    )

    assert a.contains_space(b)
    assert not a.contains_space(c)


def test_sum_semantics():
    # a = [1,0,0] + span{[0,1,0]}, b = [0,0,1] + span{[0,0,1]}.
    a = fp.AffineSubspace(
        fp.FpVector.from_slice(2, [1, 0, 0]),
        make_subspace(2, [[0, 1, 0]]),
    )
    b = fp.AffineSubspace(
        fp.FpVector.from_slice(2, [0, 0, 1]),
        make_subspace(2, [[0, 0, 1]]),
    )

    # sum: linear part = span{[0,1,0],[0,0,1]} (dim 2); offset =
    # [1,0,0] + [0,0,1] = [1,0,1] reduced against the linear part -> [1,0,0].
    s = a.sum(b)
    assert s.dimension() == 2
    assert list(s.offset()) == [1, 0, 0]
    # [1,1,1] = offset + [0,1,1] is contained; [0,0,0] is not.
    assert s.contains(fp.FpVector.from_slice(2, [1, 1, 1]))
    assert not s.contains(fp.FpVector.from_slice(2, [0, 0, 0]))


def test_invalid_construction_raises():
    linear = make_subspace(2, [[0, 1, 0]])

    # Offset length must match the linear part's ambient dimension.
    with pytest.raises(ValueError):
        fp.AffineSubspace(fp.FpVector.from_slice(2, [1, 0]), linear)

    # Offset prime must match the linear part's prime.
    with pytest.raises(ValueError):
        fp.AffineSubspace(fp.FpVector.from_slice(3, [1, 0, 0]), linear)


def test_invalid_inputs_raise():
    aff = fp.AffineSubspace(
        fp.FpVector.from_slice(2, [1, 0, 0]),
        make_subspace(2, [[0, 1, 0]]),
    )

    # contains: wrong prime / dimension.
    with pytest.raises(ValueError):
        aff.contains(fp.FpVector.from_slice(3, [1, 0, 0]))
    with pytest.raises(ValueError):
        aff.contains(fp.FpVector.from_slice(2, [1, 0]))

    # contains_space / sum: incompatible prime or ambient dimension.
    other_prime = fp.AffineSubspace(
        fp.FpVector.from_slice(3, [1, 0, 0]),
        make_subspace(3, [[0, 1, 0]]),
    )
    other_dim = fp.AffineSubspace(
        fp.FpVector.from_slice(2, [1, 0, 0, 0]),
        make_subspace(2, [[0, 1, 0, 0]]),
    )
    with pytest.raises(ValueError):
        aff.contains_space(other_prime)
    with pytest.raises(ValueError):
        aff.contains_space(other_dim)
    with pytest.raises(ValueError):
        aff.sum(other_prime)
    with pytest.raises(ValueError):
        aff.sum(other_dim)
