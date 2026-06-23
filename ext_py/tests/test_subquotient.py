import pytest

from ext import fp


def test_construction_and_queries():
    sq = fp.Subquotient(3, 5)
    assert sq.prime() == 3
    assert isinstance(sq.prime(), int)
    assert sq.ambient_dimension() == 5
    assert sq.dimension() == 0
    assert len(sq) == 0
    assert sq.is_empty()
    assert repr(sq) == "Subquotient(3, dim=0, ambient=5)"


def test_new_full():
    sq = fp.Subquotient.new_full(2, 4)
    assert sq.dimension() == 4
    assert sq.ambient_dimension() == 4
    assert sq.quotient_dimension() == 4
    assert len(sq.gens()) == 4


def test_invalid_prime_raises():
    with pytest.raises(ValueError):
        fp.Subquotient(4, 3)
    with pytest.raises(ValueError):
        fp.Subquotient.new_full(4, 3)


def test_add_gen_quotient_reduce_and_gens():
    # Mirrors the upstream `test_add_gen` example at p = 3, dim = 5.
    sq = fp.Subquotient(3, 5)
    sq.quotient(fp.FpVector.from_slice(3, [1, 1, 0, 0, 1]))
    sq.quotient(fp.FpVector.from_slice(3, [0, 2, 0, 0, 1]))
    sq.add_gen(fp.FpVector.from_slice(3, [1, 1, 0, 0, 0]))
    sq.add_gen(fp.FpVector.from_slice(3, [0, 1, 0, 0, 0]))

    assert sq.dimension() == 1
    gens = sq.gens()
    assert len(gens) == 1
    assert list(gens[0]) == [0, 0, 0, 0, 1]

    zeros = sq.zeros()
    assert isinstance(zeros, fp.Subspace)
    assert zeros.dimension() == 2

    # reduce returns the coefficients w.r.t. the generators and mutates the
    # vector in place.
    elt = fp.FpVector.from_slice(3, [2, 0, 0, 0, 0])
    assert sq.reduce(elt) == [2]

    # complement + quotient + gens cover the ambient space.
    assert (
        sq.zeros().dimension() + len(sq.gens()) + len(sq.complement_pivots())
        == sq.ambient_dimension()
    )


def test_clear_gens_keeps_quotient():
    sq = fp.Subquotient(3, 5)
    sq.quotient(fp.FpVector.from_slice(3, [1, 1, 0, 0, 1]))
    sq.add_gen(fp.FpVector.from_slice(3, [0, 1, 0, 0, 0]))
    assert sq.dimension() >= 1
    sq.clear_gens()
    assert sq.dimension() == 0
    assert sq.zeros().dimension() == 1


def test_set_to_full():
    sq = fp.Subquotient(2, 3)
    sq.set_to_full()
    # `set_to_full` makes the gens the entire space and clears the quotient,
    # but (matching upstream) does not update the cached `dimension` counter.
    assert sq.zeros().dimension() == 0
    assert len(sq.gens()) == 3


def test_from_parts():
    sub = fp.Subspace(2, 3)
    sub.add_vector(fp.FpVector.from_slice(2, [1, 0, 0]))
    sub.add_vector(fp.FpVector.from_slice(2, [0, 1, 0]))
    quot = fp.Subspace(2, 3)
    quot.add_vector(fp.FpVector.from_slice(2, [1, 0, 0]))

    sq = fp.Subquotient.from_parts(sub, quot)
    assert sq.dimension() == 1
    assert sq.ambient_dimension() == 3


def test_from_parts_mismatch_raises():
    sub = fp.Subspace(2, 3)
    bad = fp.Subspace(2, 4)
    with pytest.raises(ValueError):
        fp.Subquotient.from_parts(sub, bad)
    other_prime = fp.Subspace(3, 3)
    with pytest.raises(ValueError):
        fp.Subquotient.from_parts(sub, other_prime)


def test_invalid_vector_inputs_raise():
    sq = fp.Subquotient(3, 3)
    with pytest.raises(ValueError):
        sq.quotient(fp.FpVector.from_slice(5, [1, 0, 0]))
    with pytest.raises(ValueError):
        sq.add_gen(fp.FpVector.from_slice(3, [1, 0]))
    with pytest.raises(ValueError):
        sq.reduce(fp.FpVector.from_slice(3, [1, 0]))


def test_reduce_by_quotient():
    sq = fp.Subquotient(3, 3)
    sq.quotient(fp.FpVector.from_slice(3, [1, 0, 0]))
    v = fp.FpVector.from_slice(3, [1, 1, 0])
    sq.reduce_by_quotient(v)
    assert list(v) == [0, 1, 0]


def test_reduce_matrix():
    source = fp.Subquotient.new_full(3, 2)
    target = fp.Subquotient.new_full(3, 2)
    # identity matrix maps source ambient (rows) to target ambient (cols).
    m = fp.Matrix.from_vec(3, [[1, 0], [0, 1]])
    result = fp.Subquotient.reduce_matrix(m, source, target)
    assert len(result) == source.dimension()
