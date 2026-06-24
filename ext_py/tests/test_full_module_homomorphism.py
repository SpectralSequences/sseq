import pytest

from ext import algebra, fp


# The C2 module: x0 in degree 0, x1 in degree 1, with Sq1 x0 = x1.
C2_JSON = {
    "p": 2,
    "type": "finite dimensional module",
    "gens": {"x0": 0, "x1": 1},
    "actions": ["Sq1 x0 = x1"],
}


def milnor(p=2):
    return algebra.SteenrodAlgebra.milnor(p)


def c2_module(alg):
    return algebra.steenrod_module_from_json(alg, C2_JSON)


def free_module_one_generator(alg):
    """A FreeModule with a single generator in degree 0 (unbounded above)."""
    b = algebra.FPModuleBuilder(alg, "F", 0)
    b.add_generators(0, ["g"])
    b.add_relations(0, [])
    return b.build().generators()


# --- binding presence ------------------------------------------------------


def test_full_module_homomorphism_in_module():
    assert "FullModuleHomomorphism" in dir(algebra)


# --- construction / accessors ----------------------------------------------


def test_zero_construct_and_invariants():
    alg = milnor(2)
    m = c2_module(alg)
    hom = algebra.FullModuleHomomorphism(m, m, 0)
    assert isinstance(hom.prime(), int)
    assert hom.prime() == 2
    assert hom.degree_shift() == 0
    assert hom.min_degree() == 0
    assert repr(hom).startswith("FullModuleHomomorphism(")


def test_source_and_target_types_and_state():
    alg = milnor(2)
    m = c2_module(alg)
    hom = algebra.FullModuleHomomorphism(m, m, 0)
    source = hom.source()
    target = hom.target()
    assert isinstance(source, algebra.SteenrodModule)
    assert isinstance(target, algebra.SteenrodModule)
    assert source.dimension(0) == 1
    assert source.dimension(1) == 1
    assert target.dimension(1) == 1
    assert source.prime() == target.prime() == 2


def test_construct_requires_same_algebra():
    a1 = milnor(2)
    a2 = milnor(2)  # distinct algebra object
    m1 = c2_module(a1)
    m2 = c2_module(a2)
    with pytest.raises(ValueError):
        algebra.FullModuleHomomorphism(m1, m2, 0)


# --- zero / identity static constructors -----------------------------------


def test_zero_maps_everything_to_zero():
    alg = milnor(2)
    m = c2_module(alg)
    hom = algebra.FullModuleHomomorphism.zero(m, m, 0)
    assert isinstance(hom, algebra.FullModuleHomomorphism)
    res = fp.FpVector(2, 1)
    hom.apply_to_basis_element(res, 1, 0, 0)
    assert res[0] == 0
    res1 = fp.FpVector(2, 1)
    hom.apply_to_basis_element(res1, 1, 1, 0)
    assert res1[0] == 0


def test_identity_is_identity():
    alg = milnor(2)
    m = c2_module(alg)
    hom = algebra.FullModuleHomomorphism.identity(m)
    assert hom.degree_shift() == 0
    # identity: x0 -> x0 (degree 0), x1 -> x1 (degree 1).
    res0 = fp.FpVector(2, 1)
    hom.apply_to_basis_element(res0, 1, 0, 0)
    assert res0[0] == 1
    res1 = fp.FpVector(2, 1)
    hom.apply_to_basis_element(res1, 1, 1, 0)
    assert res1[0] == 1


def test_identity_requires_bounded_module():
    alg = milnor(2)
    free = free_module_one_generator(alg)
    free.compute_basis(2)
    unbounded = free.into_steenrod_module()
    with pytest.raises(ValueError):
        algebra.FullModuleHomomorphism.identity(unbounded)


# --- from_matrices ---------------------------------------------------------


def from_matrices_bottom_cell(alg):
    """f: C2 -> C2, degree_shift 0: identity on the bottom cell (degree 0),
    zero on the top cell (degree 1)."""
    m = c2_module(alg)
    m0 = fp.Matrix.from_vec(2, [[1]])
    m1 = fp.Matrix.from_vec(2, [[0]])
    return algebra.FullModuleHomomorphism.from_matrices(m, m, [m0, m1], 0)


def test_from_matrices_known_values():
    alg = milnor(2)
    hom = from_matrices_bottom_cell(alg)
    # apply_to_basis_element: degree 0 -> [1], degree 1 -> [0].
    r0 = fp.FpVector(2, 1)
    hom.apply_to_basis_element(r0, 1, 0, 0)
    assert r0[0] == 1
    r1 = fp.FpVector(2, 1)
    hom.apply_to_basis_element(r1, 1, 1, 0)
    assert r1[0] == 0
    # apply on a general element [1] in degree 0 agrees.
    inp = fp.FpVector(2, 1)
    inp[0] = 1
    r2 = fp.FpVector(2, 1)
    hom.apply(r2, 1, 0, inp)
    assert r2[0] == 1


def test_from_matrices_dimension_mismatch_raises():
    alg = milnor(2)
    m = c2_module(alg)
    # Degree 0 of C2 has dimension 1, so a 2-column matrix is wrong.
    bad = fp.Matrix.from_vec(2, [[1, 0]])
    with pytest.raises(ValueError):
        algebra.FullModuleHomomorphism.from_matrices(m, m, [bad], 0)


def test_from_matrices_prime_mismatch_raises():
    alg = milnor(2)
    m = c2_module(alg)
    bad = fp.Matrix.from_vec(3, [[1]])
    with pytest.raises(ValueError):
        algebra.FullModuleHomomorphism.from_matrices(m, m, [bad], 0)


# --- auxiliary data: kernel / image / quasi_inverse ------------------------


def test_auxiliary_data_dimensions_and_types():
    alg = milnor(2)
    hom = from_matrices_bottom_cell(alg)
    hom.compute_auxiliary_data_through_degree(1)

    image0 = hom.image(0)
    kernel0 = hom.kernel(0)
    qi0 = hom.quasi_inverse(0)
    assert isinstance(image0, fp.Subspace)
    assert isinstance(kernel0, fp.Subspace)
    assert isinstance(qi0, fp.QuasiInverse)
    # degree 0 is an iso k -> k.
    assert image0.dimension() == 1
    assert kernel0.dimension() == 0
    # degree 1 is the zero map k -> k.
    assert hom.image(1).dimension() == 0
    assert hom.kernel(1).dimension() == 1


def test_apply_quasi_inverse_round_trip():
    alg = milnor(2)
    hom = from_matrices_bottom_cell(alg)
    hom.compute_auxiliary_data_through_degree(1)
    # qi(x0) recovers x0 = [1] in the source degree 0.
    inp = fp.FpVector(2, 1)
    inp[0] = 1
    res = fp.FpVector(2, 1)
    applied = hom.apply_quasi_inverse(res, 0, inp)
    assert applied is True
    assert res[0] == 1


def test_apply_quasi_inverse_returns_false_when_uncomputed():
    alg = milnor(2)
    hom = from_matrices_bottom_cell(alg)
    res = fp.FpVector(2, 1)
    inp = fp.FpVector(2, 1)
    assert hom.apply_quasi_inverse(res, 0, inp) is False


def test_get_partial_matrix():
    alg = milnor(2)
    hom = from_matrices_bottom_cell(alg)
    m = hom.get_partial_matrix(0, [0])
    assert isinstance(m, fp.Matrix)
    assert m.rows() == 1
    assert m.columns() == 1
    assert m.to_vec() == [[1]]


# --- guards: errors instead of panics --------------------------------------


def test_uncomputed_aux_data_reads_none():
    alg = milnor(2)
    hom = from_matrices_bottom_cell(alg)
    assert hom.kernel(7) is None
    assert hom.image(7) is None
    assert hom.quasi_inverse(7) is None


def test_apply_out_of_range_index_raises():
    alg = milnor(2)
    hom = algebra.FullModuleHomomorphism.identity(c2_module(alg))
    res = fp.FpVector(2, 1)
    with pytest.raises(IndexError):
        hom.apply_to_basis_element(res, 1, 0, 9)


def test_apply_length_and_prime_mismatch_raises():
    alg = milnor(2)
    hom = algebra.FullModuleHomomorphism.identity(c2_module(alg))
    bad_len = fp.FpVector(2, 3)
    with pytest.raises(ValueError):
        hom.apply_to_basis_element(bad_len, 1, 0, 0)
    bad_prime = fp.FpVector(3, 1)
    with pytest.raises(ValueError):
        hom.apply_to_basis_element(bad_prime, 1, 0, 0)


def test_apply_aliasing_input_and_target_raises():
    alg = milnor(2)
    hom = algebra.FullModuleHomomorphism.identity(c2_module(alg))
    v = fp.FpVector(2, 1)
    v[0] = 1
    # Same object as both input and mutable target -> RuntimeError.
    with pytest.raises(RuntimeError):
        hom.apply(v, 1, 0, v)
