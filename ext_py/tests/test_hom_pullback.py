"""Tests for `HomPullback`: the induced map `Hom(B, X) -> Hom(A, X)` of a
free -> free map `map: A -> B`.

The end-to-end example mirrors upstream `hom_pullback.rs::test_pullback_id`
(`NUM_GENS = [1]`, `SHIFT = 0`): `map` is the iso `F1 -> F0` matching the single
generators, so the pullback is the identity in every degree. Expected values are
derived from that upstream test (which asserts the matrix equals the identity).

Note: the two Hom modules must share the *identical* target module `X` (the
upstream constructor asserts `Arc::ptr_eq`). Because the dynamic monomorphisation
wraps `X` behind a per-instance outer `Arc`, the second Hom module must be built
with `HomModule.with_source` (reusing the first's `X`), not an independent
`HomModule(f1, X)` construction.
"""

import pytest

from ext import algebra, fp


def milnor(p=2):
    return algebra.SteenrodAlgebra.milnor(p)


def make_c2(alg):
    """The bounded module X = C2: x0 in degree 0, x1 in degree 1, Sq1 x0 = x1."""
    m = algebra.FDModuleBuilder(alg, "C2", [1, 1])
    m.set_action(1, 0, 0, 0, [1])
    return m.build()


def free_one_gen(alg, name):
    """A FreeModule with a single generator in degree 0."""
    b = algebra.FPModuleBuilder(alg, name, 0)
    b.add_generators(0, [name + "g"])
    b.add_relations(0, [])
    f = b.build().generators()
    f.compute_basis(4)
    return f


def identity_pullback(alg):
    """The identity pullback Hom(F0, C2) -> Hom(F1, C2) of d: F1 -> F0, d(b)=a."""
    f0 = free_one_gen(alg, "F0")
    f1 = free_one_gen(alg, "F1")
    d = algebra.FreeModuleHomomorphismToFree(f1, f0, 0)
    row = fp.FpVector(2, f0.dimension(0))  # f0.dimension(0) == 1
    row[0] = 1
    d.add_generators_from_rows(0, [row])

    x = make_c2(alg)
    source = algebra.HomModule(f0, x)  # Hom(F0, C2)
    target = source.with_source(f1)  # Hom(F1, C2), sharing X
    pb = algebra.HomPullback(source, target, d)
    return pb, source, target, f0, f1, d, x


# --- construction / accessors ---------------------------------------------


def test_construct_and_invariants():
    pb, source, target, _f0, _f1, _d, _x = identity_pullback(milnor(2))
    assert isinstance(pb.prime(), int)
    assert pb.prime() == 2
    assert pb.degree_shift() == 0
    # Hom(F<g deg 0>, C2).min_degree() = 0 - C2.max_degree()(=1) = -1.
    assert pb.min_degree() == -1
    assert repr(pb).startswith("HomPullback(")


def test_source_target_roundtrip():
    pb, _source, _target, _f0, _f1, _d, _x = identity_pullback(milnor(2))
    s = pb.source()
    t = pb.target()
    assert isinstance(s, algebra.HomModule)
    assert isinstance(t, algebra.HomModule)
    assert s.min_degree() == -1
    assert t.min_degree() == -1
    s.compute_basis(0)
    t.compute_basis(0)
    # Hom(F<g>, C2) dims: dim(-1) = C2.dim(1) = 1, dim(0) = C2.dim(0) = 1.
    assert s.dimension(-1) == 1
    assert s.dimension(0) == 1
    assert t.dimension(-1) == 1
    assert t.dimension(0) == 1


# --- known values (identity pullback) -------------------------------------


def test_identity_partial_matrices():
    pb, _source, _target, _f0, _f1, _d, _x = identity_pullback(milnor(2))
    # The pullback of the iso d is the identity in every degree.
    assert pb.get_partial_matrix(-1, [0]).to_vec() == [[1]]
    assert pb.get_partial_matrix(0, [0]).to_vec() == [[1]]


def test_apply_to_basis_element_known_value():
    pb, _source, _target, _f0, _f1, _d, _x = identity_pullback(milnor(2))
    # apply_to_basis_element(-1, 0): identity -> [1] in target degree -1.
    res = fp.FpVector(2, 1)
    pb.apply_to_basis_element(res, 1, -1, 0)
    assert res[0] == 1
    res0 = fp.FpVector(2, 1)
    pb.apply_to_basis_element(res0, 1, 0, 0)
    assert res0[0] == 1


def test_apply_general_element():
    pb, _source, _target, _f0, _f1, _d, _x = identity_pullback(milnor(2))
    inp = fp.FpVector(2, 1)
    inp[0] = 1
    res = fp.FpVector(2, 1)
    pb.apply(res, 1, -1, inp)
    assert res[0] == 1


def test_apply_aliasing_raises_runtimeerror():
    pb, _source, _target, _f0, _f1, _d, _x = identity_pullback(milnor(2))
    v = fp.FpVector(2, 1)
    v[0] = 1
    with pytest.raises(RuntimeError):
        pb.apply(v, 1, -1, v)


# --- auxiliary data (genuinely computed, not trivial defaults) -------------


def test_auxiliary_data_dimensions():
    pb, _source, _target, _f0, _f1, _d, _x = identity_pullback(milnor(2))
    pb.compute_auxiliary_data_through_degree(0)
    # Identity is an iso in each degree: image dim 1, kernel dim 0.
    for deg in (-1, 0):
        image = pb.image(deg)
        kernel = pb.kernel(deg)
        assert isinstance(image, fp.Subspace)
        assert isinstance(kernel, fp.Subspace)
        assert image.dimension() == 1
        assert kernel.dimension() == 0
    assert pb.quasi_inverse(0) is not None


def test_apply_quasi_inverse_round_trip():
    pb, _source, _target, _f0, _f1, _d, _x = identity_pullback(milnor(2))
    pb.compute_auxiliary_data_through_degree(0)
    inp = fp.FpVector(2, 1)
    inp[0] = 1
    res = fp.FpVector(2, 1)
    applied = pb.apply_quasi_inverse(res, 0, inp)
    assert applied is True
    assert res[0] == 1


def test_uncomputed_aux_data_reads_none():
    pb, _source, _target, _f0, _f1, _d, _x = identity_pullback(milnor(2))
    assert pb.kernel(7) is None
    assert pb.image(7) is None
    assert pb.quasi_inverse(7) is None
    # apply_quasi_inverse with no computed data -> False (not an error).
    res = fp.FpVector(2, 1)
    inp = fp.FpVector(2, 1)
    assert pb.apply_quasi_inverse(res, 7, inp) is False


# --- get_partial_matrix guards --------------------------------------------


def test_get_partial_matrix_out_of_range_is_zero_matrix():
    pb, _source, _target, _f0, _f1, _d, _x = identity_pullback(milnor(2))
    # Degree 5 is above the target Hom module's computed range -> dimension 0,
    # so the (0 x 0) zero matrix is returned (no panic).
    m = pb.get_partial_matrix(5, [])
    assert m.columns() == 0


def test_get_partial_matrix_below_min_degree_raises():
    pb, _source, _target, _f0, _f1, _d, _x = identity_pullback(milnor(2))
    # min_degree() == -1, so degree -5 is below the source.
    with pytest.raises(IndexError):
        pb.get_partial_matrix(-5, [0])


def test_apply_length_and_prime_mismatch_raises():
    pb, _source, _target, _f0, _f1, _d, _x = identity_pullback(milnor(2))
    bad_len = fp.FpVector(2, 3)
    with pytest.raises(ValueError):
        pb.apply_to_basis_element(bad_len, 1, -1, 0)
    bad_prime = fp.FpVector(3, 1)
    with pytest.raises(ValueError):
        pb.apply_to_basis_element(bad_prime, 1, -1, 0)


def test_apply_out_of_range_index_raises():
    pb, _source, _target, _f0, _f1, _d, _x = identity_pullback(milnor(2))
    res = fp.FpVector(2, 1)
    with pytest.raises(IndexError):
        pb.apply_to_basis_element(res, 1, -1, 9)


# --- construction assertion guards (ValueError, not panic) -----------------


def test_assertion_target_source_mismatch_raises():
    # target.source() must equal map.source() (= f1); passing Hom(f0, X) as the
    # target violates this (its source is f0).
    alg = milnor(2)
    f0 = free_one_gen(alg, "F0")
    f1 = free_one_gen(alg, "F1")
    d = algebra.FreeModuleHomomorphismToFree(f1, f0, 0)
    row = fp.FpVector(2, 1)
    row[0] = 1
    d.add_generators_from_rows(0, [row])
    x = make_c2(alg)
    source = algebra.HomModule(f0, x)
    bad_target = source.with_source(f0)  # Hom(f0, X), wrong source
    with pytest.raises(ValueError):
        algebra.HomPullback(source, bad_target, d)


def test_assertion_source_source_mismatch_raises():
    # source.source() must equal map.target() (= f0); passing Hom(f1, X) as the
    # source violates this (its source is f1).
    alg = milnor(2)
    f0 = free_one_gen(alg, "F0")
    f1 = free_one_gen(alg, "F1")
    d = algebra.FreeModuleHomomorphismToFree(f1, f0, 0)
    row = fp.FpVector(2, 1)
    row[0] = 1
    d.add_generators_from_rows(0, [row])
    x = make_c2(alg)
    target = algebra.HomModule(f1, x)
    bad_source = target.with_source(f1)  # Hom(f1, X), wrong source
    with pytest.raises(ValueError):
        algebra.HomPullback(bad_source, target, d)


def test_assertion_distinct_X_raises():
    # source.target() must equal target.target(): two independently built Hom
    # modules over distinct (even if equal) X objects fail the identity check.
    alg = milnor(2)
    f0 = free_one_gen(alg, "F0")
    f1 = free_one_gen(alg, "F1")
    d = algebra.FreeModuleHomomorphismToFree(f1, f0, 0)
    row = fp.FpVector(2, 1)
    row[0] = 1
    d.add_generators_from_rows(0, [row])
    source = algebra.HomModule(f0, make_c2(alg))
    target = algebra.HomModule(f1, make_c2(alg))  # distinct X object
    with pytest.raises(ValueError):
        algebra.HomPullback(source, target, d)


def test_independent_hommodules_over_same_x_still_mismatch():
    # Even over the *same* SteenrodModule object, two independent HomModule(...)
    # constructions wrap X in distinct outer Arcs and fail the identity check;
    # `with_source` is the supported way to build a compatible pair.
    alg = milnor(2)
    f0 = free_one_gen(alg, "F0")
    f1 = free_one_gen(alg, "F1")
    d = algebra.FreeModuleHomomorphismToFree(f1, f0, 0)
    row = fp.FpVector(2, 1)
    row[0] = 1
    d.add_generators_from_rows(0, [row])
    x = make_c2(alg)
    source = algebra.HomModule(f0, x)
    target = algebra.HomModule(f1, x)  # same x, but independent outer Arc
    with pytest.raises(ValueError):
        algebra.HomPullback(source, target, d)
    # The supported construction (with_source) succeeds.
    target_ok = source.with_source(f1)
    pb = algebra.HomPullback(source, target_ok, d)
    assert pb.get_partial_matrix(0, [0]).to_vec() == [[1]]
