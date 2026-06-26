import pytest

from ext import fp


def test_construction_and_queries():
    sq = fp.Subquotient(3, 5)
    assert sq.prime == 3
    assert isinstance(sq.prime, int)
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


def test_subspace_gens_quotient_pivots_and_dimension():
    # Non-trivial subquotient (non-empty quotient and one generator), mirroring
    # the upstream `test_add_gen` example at p = 3, dim = 5. After the calls the
    # upstream `Display` is:
    #   Generators: [0, 0, 0, 0, 1]
    #   Zeros:      [1, 0, 0, 0, 2]
    #               [0, 1, 0, 0, 2]
    sq = fp.Subquotient(3, 5)
    sq.quotient(fp.FpVector.from_slice(3, [1, 1, 0, 0, 1]))
    sq.quotient(fp.FpVector.from_slice(3, [0, 2, 0, 0, 1]))
    sq.add_gen(fp.FpVector.from_slice(3, [1, 1, 0, 0, 0]))
    sq.add_gen(fp.FpVector.from_slice(3, [0, 1, 0, 0, 0]))

    # dimension is the subspace-part generator count; quotient (zeros) dim is 2.
    assert sq.dimension() == 1
    assert sq.zeros().dimension() == 2

    # subspace_dimension == self.dimension + quotient.dimension() per upstream
    # `subquotient.rs::subspace_dimension`.
    assert sq.subspace_dimension() == sq.dimension() + sq.zeros().dimension()
    assert sq.subspace_dimension() == 3

    # subspace_gens chains gens() with the quotient's basis vectors (upstream
    # `subspace_gens` = `gens().chain(quotient.iter())`).
    subspace_gens = [list(v) for v in sq.subspace_gens()]
    assert subspace_gens == [
        [0, 0, 0, 0, 1],
        [1, 0, 0, 0, 2],
        [0, 1, 0, 0, 2],
    ]

    # quotient_pivots is the quotient subspace's pivot table: pivots[col] = row
    # index of the pivot in that column, else -1. Quotient pivots are in cols 0,1.
    assert sq.quotient_pivots() == [0, 1, -1, -1, -1]


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

    # Stale-`dimension` quirk: `set_to_full` makes gens the entire space and
    # clears the quotient, but upstream does NOT update the cached `dimension`
    # counter. So on a fresh Subquotient these are inconsistent today:
    #   dimension()/len(sq) report 0 (stale), while gens() actually has 3 rows.
    # Pin the surprising current behavior so a future upstream fix (syncing the
    # cached dimension) trips this test and prompts a revisit.
    assert sq.dimension() == 0
    assert len(sq) == 0
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


def test_reduce_by_quotient_slice_mut():
    sq = fp.Subquotient(3, 3)
    sq.quotient(fp.FpVector.from_slice(3, [1, 0, 0]))
    # Full-row slice: reduction must write through to the matrix.
    m = fp.Matrix.from_vec(3, [[1, 1, 0]])
    sq.reduce_by_quotient(m.row_mut(0))
    assert list(m.row(0)) == [0, 1, 0]
    # Sub-range slice: only the targeted columns are reduced in place.
    m2 = fp.Matrix.from_vec(3, [[2, 1, 1, 0]])
    row = m2.row_mut(0)
    sq.reduce_by_quotient(row.slice_mut(1, 4))
    assert list(m2.row(0)) == [2, 0, 1, 0]


def test_reduce_matrix():
    source = fp.Subquotient.new_full(3, 2)
    target = fp.Subquotient.new_full(3, 2)
    # identity matrix maps source ambient (rows) to target ambient (cols).
    m = fp.Matrix.from_vec(3, [[1, 0], [0, 1]])
    result = fp.Subquotient.reduce_matrix(m, source, target)
    assert len(result) == source.dimension()


def test_reduce_matrix_values_with_nontrivial_quotient():
    # source = full space of dim 2 at p = 3, so gens() = [1, 0] and [0, 1].
    source = fp.Subquotient.new_full(3, 2)

    # target has a non-trivial quotient: quotient kills coordinate 1, generator
    # is [1, 0]. So target.reduce projects out column 1 and reads coeff at col 0.
    target = fp.Subquotient(3, 2)
    target.quotient(fp.FpVector.from_slice(3, [0, 1]))
    target.add_gen(fp.FpVector.from_slice(3, [1, 0]))

    # Non-identity matrix. `Matrix.apply` computes (input row-vector) * matrix:
    # result = sum_i input[i] * row(i). So gen [1,0] -> row 0 = [2, 1];
    # gen [0,1] -> row 1 = [0, 1].
    m = fp.Matrix.from_vec(3, [[2, 1], [0, 1]])

    # Reducing images in target: drop col 1 then read coeff at col 0.
    #   [2, 1] -> quotient -> [2, 0] -> coeff [2]
    #   [0, 1] -> quotient -> [0, 0] -> coeff [0]
    result = fp.Subquotient.reduce_matrix(m, source, target)
    assert result == [[2], [0]]


def test_reduce_matrix_dimension_mismatches_raise():
    source = fp.Subquotient.new_full(3, 2)
    target = fp.Subquotient.new_full(3, 2)

    # rows != source.ambient_dimension (3 rows vs ambient 2).
    bad_rows = fp.Matrix.from_vec(3, [[1, 0], [0, 1], [0, 0]])
    with pytest.raises(ValueError):
        fp.Subquotient.reduce_matrix(bad_rows, source, target)

    # columns != target.ambient_dimension (3 cols vs ambient 2).
    bad_cols = fp.Matrix.from_vec(3, [[1, 0, 0], [0, 1, 0]])
    with pytest.raises(ValueError):
        fp.Subquotient.reduce_matrix(bad_cols, source, target)

    # prime mismatch between matrix and subquotients.
    bad_prime = fp.Matrix.from_vec(5, [[1, 0], [0, 1]])
    with pytest.raises(ValueError):
        fp.Subquotient.reduce_matrix(bad_prime, source, target)
