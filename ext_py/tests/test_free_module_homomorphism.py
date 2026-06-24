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


def free_module_one_generator(alg):
    """A FreeModule with a single generator in degree 0, obtained as the
    generators of a one-generator finitely presented module (FreeModule has no
    Python-side mutators)."""
    b = algebra.FPModuleBuilder(alg, "F", 0)
    b.add_generators(0, ["g"])
    b.add_relations(0, [])
    return b.build().generators()


def c2_differential(alg):
    """f: F -> C2 with F = <g> in degree 0 and f(g) = x0."""
    source = free_module_one_generator(alg)
    target = algebra.steenrod_module_from_json(alg, C2_JSON)
    hom = algebra.FreeModuleHomomorphism(source, target, 0)
    row = fp.FpVector(2, target.dimension(0))
    row[0] = 1
    hom.add_generators_from_rows(0, [row])
    return hom


# --- construction / accessors ---------------------------------------------


def test_construct_and_invariants():
    alg = milnor(2)
    hom = c2_differential(alg)
    assert isinstance(hom.prime(), int)
    assert hom.prime() == 2
    assert hom.degree_shift() == 0
    assert hom.min_degree() == 0
    assert hom.next_degree() == 1
    assert repr(hom).startswith("FreeModuleHomomorphism(")


def test_source_and_target_types_and_state():
    alg = milnor(2)
    hom = c2_differential(alg)
    source = hom.source()
    target = hom.target()
    assert isinstance(source, algebra.FreeModule)
    assert isinstance(target, algebra.SteenrodModule)
    assert source.number_of_gens_in_degree(0) == 1
    assert target.dimension(0) == 1
    assert target.dimension(1) == 1
    assert source.prime() == target.prime() == 2


def test_construct_requires_same_algebra():
    a1 = milnor(2)
    a2 = milnor(2)  # distinct algebra object
    source = free_module_one_generator(a1)
    target = algebra.steenrod_module_from_json(a2, C2_JSON)
    with pytest.raises(ValueError):
        algebra.FreeModuleHomomorphism(source, target, 0)


# --- apply / apply_to_basis_element / apply_to_generator / output ----------


def test_apply_to_basis_element_known_values():
    alg = milnor(2)
    hom = c2_differential(alg)

    # f(g) = x0: basis element (degree 0, idx 0) -> [1].
    res = fp.FpVector(2, 1)
    hom.apply_to_basis_element(res, 1, 0, 0)
    assert res[0] == 1

    # f(Sq1 . g) = Sq1 . x0 = x1: basis element (degree 1, idx 0) -> [1].
    res1 = fp.FpVector(2, 1)
    hom.apply_to_basis_element(res1, 1, 1, 0)
    assert res1[0] == 1


def test_apply_general_element():
    alg = milnor(2)
    hom = c2_differential(alg)
    inp = fp.FpVector(2, 1)
    inp[0] = 1
    res = fp.FpVector(2, 1)
    hom.apply(res, 1, 0, inp)
    assert res[0] == 1


def test_apply_to_generator_and_output():
    alg = milnor(2)
    hom = c2_differential(alg)
    res = fp.FpVector(2, 1)
    hom.apply_to_generator(res, 1, 0, 0)
    assert res[0] == 1

    out = hom.output(0, 0)
    assert isinstance(out, fp.FpVector)
    assert out[0] == 1


def test_apply_aliasing_input_and_target_raises():
    alg = milnor(2)
    hom = c2_differential(alg)
    v = fp.FpVector(2, 1)
    v[0] = 1
    # Same object as both input and mutable target -> RuntimeError.
    with pytest.raises(RuntimeError):
        hom.apply(v, 1, 0, v)


# --- auxiliary data: kernel / image / quasi_inverse ------------------------


def test_auxiliary_data_dimensions_and_types():
    alg = milnor(2)
    hom = c2_differential(alg)
    hom.compute_auxiliary_data_through_degree(0)

    image = hom.image(0)
    kernel = hom.kernel(0)
    qi = hom.quasi_inverse(0)
    assert isinstance(image, fp.Subspace)
    assert isinstance(kernel, fp.Subspace)
    assert isinstance(qi, fp.QuasiInverse)
    # f is an iso k -> k in degree 0.
    assert image.dimension() == 1
    assert kernel.dimension() == 0


def test_apply_quasi_inverse_round_trip():
    alg = milnor(2)
    hom = c2_differential(alg)
    hom.compute_auxiliary_data_through_degree(0)
    # qi(x0) should recover g = [1] in the source degree 0.
    inp = fp.FpVector(2, 1)
    inp[0] = 1
    res = fp.FpVector(2, 1)
    applied = hom.apply_quasi_inverse(res, 0, inp)
    assert applied is True
    assert res[0] == 1


def test_apply_quasi_inverse_returns_false_when_uncomputed():
    alg = milnor(2)
    hom = c2_differential(alg)
    res = fp.FpVector(2, 1)
    inp = fp.FpVector(2, 1)
    # Degree 0 quasi-inverse not computed yet -> False, not an error.
    assert hom.apply_quasi_inverse(res, 0, inp) is False


def test_get_partial_matrix():
    alg = milnor(2)
    hom = c2_differential(alg)
    m = hom.get_partial_matrix(0, [0])
    assert isinstance(m, fp.Matrix)
    assert m.rows() == 1
    assert m.columns() == 1


# --- guards: errors instead of panics --------------------------------------


def test_uncomputed_aux_data_reads_none():
    alg = milnor(2)
    hom = c2_differential(alg)
    assert hom.kernel(7) is None
    assert hom.image(7) is None
    assert hom.quasi_inverse(7) is None


def test_apply_out_of_range_index_raises():
    alg = milnor(2)
    hom = c2_differential(alg)
    res = fp.FpVector(2, 1)
    with pytest.raises(IndexError):
        hom.apply_to_basis_element(res, 1, 0, 9)


def test_apply_length_and_prime_mismatch_raises():
    alg = milnor(2)
    hom = c2_differential(alg)
    bad_len = fp.FpVector(2, 3)
    with pytest.raises(ValueError):
        hom.apply_to_basis_element(bad_len, 1, 0, 0)
    bad_prime = fp.FpVector(3, 1)
    with pytest.raises(ValueError):
        hom.apply_to_basis_element(bad_prime, 1, 0, 0)


def test_add_generators_from_rows_non_consecutive_raises():
    alg = milnor(2)
    hom = c2_differential(alg)
    # Degree 0 already defined; the next expected degree is 1, not 5.
    row = fp.FpVector(2, 1)
    with pytest.raises(ValueError):
        hom.add_generators_from_rows(5, [row])


def test_add_generators_from_rows_wrong_count_raises():
    alg = milnor(2)
    source = free_module_one_generator(alg)
    target = algebra.steenrod_module_from_json(alg, C2_JSON)
    hom = algebra.FreeModuleHomomorphism(source, target, 0)
    # Degree 0 has exactly one generator; supplying two rows is an error.
    r1 = fp.FpVector(2, 1)
    r2 = fp.FpVector(2, 1)
    with pytest.raises(ValueError):
        hom.add_generators_from_rows(0, [r1, r2])


def test_add_generators_from_matrix_rows():
    alg = milnor(2)
    source = free_module_one_generator(alg)
    target = algebra.steenrod_module_from_json(alg, C2_JSON)
    hom = algebra.FreeModuleHomomorphism(source, target, 0)
    matrix = fp.Matrix.from_vec(2, [[1]])
    hom.add_generators_from_matrix_rows(0, matrix)
    res = fp.FpVector(2, 1)
    hom.apply_to_basis_element(res, 1, 0, 0)
    assert res[0] == 1


def test_differential_density_undefined_degree_raises():
    alg = milnor(2)
    hom = c2_differential(alg)
    with pytest.raises(ValueError):
        hom.differential_density(9)


def test_differential_density_defined_degree():
    alg = milnor(2)
    hom = c2_differential(alg)
    # Degree 0 has one generator whose output [1] is fully dense.
    assert hom.differential_density(0) == pytest.approx(1.0)


def test_set_kernel_non_consecutive_raises():
    alg = milnor(2)
    hom = c2_differential(alg)
    sub = fp.Subspace(2, 1)
    # Kernel table is empty (length min_degree = 0); degree 3 is non-consecutive.
    with pytest.raises(ValueError):
        hom.set_kernel(3, sub)
