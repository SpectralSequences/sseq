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


# --- from_matrices min_degree guard ----------------------------------------


def test_from_matrices_rejects_min_degree_below_target_min():
    alg = milnor(2)
    m = c2_module(alg)
    target_min = m.min_degree()
    assert target_min == 0
    # Upstream builds the kernels/images/quasi_inverses tables starting at
    # target.min_degree(), so matrices recorded below it would never get
    # auxiliary data -> rejected with a clear ValueError.
    with pytest.raises(ValueError):
        algebra.FullModuleHomomorphism.from_matrices(
            m, m, [], 0, min_degree=target_min - 1
        )


def test_from_matrices_min_degree_at_target_min_and_default_ok():
    alg = milnor(2)
    m = c2_module(alg)
    target_min = m.min_degree()
    # min_degree == target.min_degree() is accepted.
    hom = algebra.FullModuleHomomorphism.from_matrices(
        m, m, [], 0, min_degree=target_min
    )
    assert isinstance(hom, algebra.FullModuleHomomorphism)
    # The default (None) path is unaffected.
    hom_default = algebra.FullModuleHomomorphism.from_matrices(m, m, [], 0)
    assert isinstance(hom_default, algebra.FullModuleHomomorphism)


def test_from_matrices_explicit_min_degree_multi_degree_apply():
    """`matrices[i]` is the matrix in output degree `min_degree + i`."""
    alg = milnor(2)
    m = c2_module(alg)
    # degree_shift 0; zero on the bottom cell (output degree 0), identity on the
    # top cell (output degree 1). If the ordering were reversed, the asserts
    # below would flip.
    m0 = fp.Matrix.from_vec(2, [[0]])  # output degree 0
    m1 = fp.Matrix.from_vec(2, [[1]])  # output degree 1
    hom = algebra.FullModuleHomomorphism.from_matrices(
        m, m, [m0, m1], 0, min_degree=m.min_degree()
    )
    r0 = fp.FpVector(2, 1)
    hom.apply_to_basis_element(r0, 1, 0, 0)
    assert r0[0] == 0
    r1 = fp.FpVector(2, 1)
    hom.apply_to_basis_element(r1, 1, 1, 0)
    assert r1[0] == 1


# --- degree_shift != 0 -----------------------------------------------------


def shift_one_top_to_bottom(alg):
    """f: C2 -> C2 with degree_shift 1, so output_degree = input_degree - 1.

    `matrices[0]` is the matrix in output degree min_degree (= 0); its rows
    index the source basis in degree output_degree + degree_shift = 1 and its
    columns index the target basis in degree 0. The single matrix [[1]] sends
    the source top cell x1 (degree 1) to the target bottom cell x0 (degree 0).
    """
    m = c2_module(alg)
    m0 = fp.Matrix.from_vec(2, [[1]])
    return algebra.FullModuleHomomorphism.from_matrices(m, m, [m0], 1)


def test_shift_apply_lands_in_shifted_degree():
    alg = milnor(2)
    hom = shift_one_top_to_bottom(alg)
    assert hom.degree_shift() == 1
    # input_degree 1 -> output_degree 0; result lives in target.dim(0) == 1.
    # Per upstream apply_to_basis_element: result += matrices.get(0).row(0) = [1].
    res = fp.FpVector(2, 1)
    hom.apply_to_basis_element(res, 1, 1, 0)
    assert res[0] == 1
    # apply on the general top-cell element [1] in degree 1 agrees.
    inp = fp.FpVector(2, 1)
    inp[0] = 1
    res2 = fp.FpVector(2, 1)
    hom.apply(res2, 1, 1, inp)
    assert res2[0] == 1
    # input_degree 0 -> output_degree -1 (target.dim == 0): no matrix recorded
    # there, so the map contributes nothing (length-0 result, no panic).
    res_empty = fp.FpVector(2, 0)
    hom.apply_to_basis_element(res_empty, 1, 0, 0)


def test_shift_get_partial_matrix_success_and_guard():
    alg = milnor(2)
    hom = shift_one_top_to_bottom(alg)
    # Success case: target.dim(1) == target.dim(0) == 1.
    gm = hom.get_partial_matrix(1, [0])
    assert isinstance(gm, fp.Matrix)
    assert gm.rows() == 1
    assert gm.columns() == 1
    assert gm.to_vec() == [[1]]
    # Guard case: target.dim(0) == 1 != target.dim(-1) == 0 -> ValueError.
    with pytest.raises(ValueError):
        hom.get_partial_matrix(0, [0])


def test_shift_get_partial_matrix_out_of_range_degree():
    alg = milnor(2)
    hom = shift_one_top_to_bottom(alg)
    # Degree beyond the (FD) source's range has dimension 0, so any input index
    # is out of range -> clean IndexError (no panic).
    with pytest.raises((IndexError, ValueError)):
        hom.get_partial_matrix(50, [0])


def test_shift_from_matrices_row_dimension_validation():
    alg = milnor(2)
    m = c2_module(alg)
    # For output degree 0 and degree_shift 1, rows must equal
    # source.dim(0 + 1) == 1. A 2-row matrix is rejected.
    bad = fp.Matrix.from_vec(2, [[1], [1]])
    with pytest.raises(ValueError):
        algebra.FullModuleHomomorphism.from_matrices(m, m, [bad], 1)


# --- empty-matrices aux-data no-op -----------------------------------------


def test_empty_new_aux_data_noop():
    alg = milnor(2)
    m = c2_module(alg)
    hom = algebra.FullModuleHomomorphism(m, m, 0)
    # No matrices were recorded, so the upstream computation is a no-op and must
    # not panic; nothing is cached, so reads return None.
    hom.compute_auxiliary_data_through_degree(2)
    assert hom.kernel(0) is None
    assert hom.image(0) is None
    assert hom.quasi_inverse(0) is None


def test_empty_zero_aux_data_noop():
    alg = milnor(2)
    m = c2_module(alg)
    hom = algebra.FullModuleHomomorphism.zero(m, m, 0)
    hom.compute_auxiliary_data_through_degree(2)
    assert hom.kernel(0) is None
    assert hom.image(0) is None
    assert hom.quasi_inverse(0) is None


# --- apply_quasi_inverse aliasing ------------------------------------------


def test_apply_quasi_inverse_aliasing_raises():
    alg = milnor(2)
    hom = from_matrices_bottom_cell(alg)
    hom.compute_auxiliary_data_through_degree(1)
    v = fp.FpVector(2, 1)
    v[0] = 1
    # Same object as both input and mutable result -> RuntimeError.
    with pytest.raises(RuntimeError):
        hom.apply_quasi_inverse(v, 0, v)


# --- apply degree-range guards ---------------------------------------------


def test_apply_below_min_degree_raises():
    alg = milnor(2)
    hom = algebra.FullModuleHomomorphism.identity(c2_module(alg))
    res = fp.FpVector(2, 1)
    with pytest.raises(IndexError):
        hom.apply_to_basis_element(res, 1, -1, 0)


def test_apply_above_range_raises():
    alg = milnor(2)
    hom = algebra.FullModuleHomomorphism.identity(c2_module(alg))
    res = fp.FpVector(2, 1)
    # Degree above the FD source's range has dimension 0 -> index out of range.
    with pytest.raises((IndexError, ValueError)):
        hom.apply_to_basis_element(res, 1, 50, 0)


# --- overflow guard --------------------------------------------------------


def test_apply_output_degree_overflow_raises():
    alg = milnor(2)
    m = c2_module(alg)
    # degree_shift = i32::MIN; output_degree = input_degree - degree_shift then
    # overflows i32 for any in-range input_degree -> clean ValueError, no panic.
    hom = algebra.FullModuleHomomorphism(m, m, -2147483648)
    res = fp.FpVector(2, 1)
    with pytest.raises(ValueError):
        hom.apply_to_basis_element(res, 1, 0, 0)
