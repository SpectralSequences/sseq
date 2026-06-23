import pytest

from ext import fp


def test_subspace_construction_and_queries():
    s = fp.Subspace(3, 3)
    assert s.prime() == 3
    assert s.ambient_dimension() == 3
    assert s.dimension() == 0
    assert len(s) == 0
    assert repr(s) == "Subspace(3, dim=0, ambient=3)"


def test_subspace_entire_space():
    s = fp.Subspace.entire_space(2, 3)
    assert s.dimension() == 3
    assert s.ambient_dimension() == 3


def test_subspace_from_matrix():
    m = fp.Matrix.from_vec(3, [[1, 0, 0], [0, 1, 2]])
    s = fp.Subspace.from_matrix(m)
    assert s.dimension() == 2
    assert s.ambient_dimension() == 3


def test_add_vector_and_contains():
    s = fp.Subspace(3, 3)
    v = fp.FpVector.from_slice(3, [1, 0, 0])
    assert s.add_vector(v) == 1
    assert s.contains(v)
    assert v in s

    w = fp.FpVector.from_slice(3, [0, 1, 0])
    assert not s.contains(w)
    assert w not in s

    # A scalar multiple is still contained.
    twice = fp.FpVector.from_slice(3, [2, 0, 0])
    assert s.contains(twice)


def test_contains_space_and_sum():
    a = fp.Subspace(3, 3)
    a.add_vector(fp.FpVector.from_slice(3, [1, 0, 0]))
    b = fp.Subspace(3, 3)
    b.add_vector(fp.FpVector.from_slice(3, [0, 1, 0]))

    assert not a.contains_space(b)

    # Incompatible prime/ambient dimension raise ValueError rather than panic.
    with pytest.raises(ValueError):
        a.contains_space(fp.Subspace(5, 3))
    with pytest.raises(ValueError):
        a.contains_space(fp.Subspace(3, 4))

    # The sum of two complementary lines is their 2-dimensional span.
    s = a.sum(b)
    assert s.dimension() == 2
    assert s.contains_space(a)
    assert s.contains_space(b)
    assert s.contains(fp.FpVector.from_slice(3, [1, 0, 0]))
    assert s.contains(fp.FpVector.from_slice(3, [0, 1, 0]))
    assert s.dimension() <= s.ambient_dimension()

    # Overlapping subspaces: the sum's dimension is the union's rank.
    c = fp.Subspace(3, 3)
    c.add_vector(fp.FpVector.from_slice(3, [1, 0, 0]))
    overlap = a.sum(c)
    assert overlap.dimension() == 1
    assert overlap.contains_space(a)


def test_reduce_in_place():
    s = fp.Subspace(3, 3)
    s.add_vector(fp.FpVector.from_slice(3, [1, 0, 0]))

    v = fp.FpVector.from_slice(3, [2, 1, 0])
    s.reduce(v)
    assert v[0] == 0
    assert v[1] == 1


def test_set_to_zero_and_entire():
    s = fp.Subspace.entire_space(2, 3)
    assert s.dimension() == 3
    s.set_to_zero()
    assert s.dimension() == 0
    s.set_to_entire()
    assert s.dimension() == 3


def test_iter_returns_basis_vectors():
    m = fp.Matrix.from_vec(3, [[1, 0, 0], [0, 1, 2]])
    s = fp.Subspace.from_matrix(m)
    basis = s.iter()
    assert [list(v) for v in basis] == [[1, 0, 0], [0, 1, 2]]


def test_iter_all_vectors():
    m = fp.Matrix.from_vec(3, [[1, 0, 0], [0, 1, 2]])
    s = fp.Subspace.from_matrix(m)

    # iter_all_vectors returns a lazy iterator, not a list.
    it = s.iter_all_vectors()
    assert not isinstance(it, list)
    assert iter(it) is it

    # Iterating via a for-loop yields every vector in the subspace.
    collected = []
    for v in s.iter_all_vectors():
        collected.append(list(v))
    assert len(collected) == 9

    # list(...) over the iterator yields the same set/count.
    vectors = sorted(list(v) for v in s.iter_all_vectors())
    assert len(vectors) == 9
    assert sorted(collected) == vectors
    assert [0, 0, 0] in vectors
    assert [1, 1, 2] in vectors


def test_bytes_roundtrip():
    s = fp.Subspace(3, 3)
    s.add_vector(fp.FpVector.from_slice(3, [1, 0, 0]))
    data = s.to_bytes()
    assert isinstance(data, bytes)
    restored = fp.Subspace.from_bytes(3, data)
    assert restored.dimension() == 1
    assert restored.contains(fp.FpVector.from_slice(3, [1, 0, 0]))


def test_invalid_inputs_raise():
    s = fp.Subspace(3, 3)

    with pytest.raises(ValueError):
        s.contains(fp.FpVector.from_slice(5, [1, 0, 0]))

    with pytest.raises(ValueError):
        s.add_vector(fp.FpVector.from_slice(3, [1, 0]))

    with pytest.raises(ValueError):
        fp.Subspace(1, 3)


def test_bytes_error_taxonomy_is_consistent():
    # Subspace mirrors FpVector/Matrix: both to_bytes and from_bytes map
    # serialization failures to RuntimeError, so malformed bytes raise
    # RuntimeError (not ValueError).
    with pytest.raises(RuntimeError):
        fp.Subspace.from_bytes(3, b"\x00\x01\x02")
