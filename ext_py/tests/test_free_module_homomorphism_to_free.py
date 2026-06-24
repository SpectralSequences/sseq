"""Tests for the free -> free `FreeModuleHomomorphismToFree` variant.

This is the `FreeModuleHomomorphism` whose *target* is itself a concrete
`FreeModule` (rather than a boxed dynamic `SteenrodModule`). It is the variant
`HomPullback` requires, and it additionally exposes `hom_k`.

Expected values are derived from upstream `free_module_homomorphism.rs`
(`output_degree = input_degree - degree_shift`; the `hom_k` impl on the
`MuFreeModuleHomomorphism<U, MuFreeModule<U, A>>` block).
"""

import gc

import pytest

from ext import algebra, fp


def milnor(p=2):
    return algebra.SteenrodAlgebra.milnor(p)


def free_gen_in_degree(alg, name, gen_degree, min_degree=0):
    """A FreeModule whose only generator lives in `gen_degree`.

    FreeModule has no Python-side mutators, so it is obtained as the generators
    of a one-generator finitely presented module. Generators must be added at
    consecutive degrees from `min_degree`.
    """
    b = algebra.FPModuleBuilder(alg, name, min_degree)
    for d in range(min_degree, gen_degree):
        b.add_generators(d, [])
    b.add_generators(gen_degree, [name + "_g"])
    b.add_relations(min_degree, [])
    f = b.build().generators()
    f.compute_basis(4)
    return f


def free_to_free_id(alg):
    """f: F1 -> F0 with F1 = <g> and F0 = <a> both in degree 0, f(g) = a."""
    source = free_gen_in_degree(alg, "F1", 0)
    target = free_gen_in_degree(alg, "F0", 0)
    hom = algebra.FreeModuleHomomorphismToFree(source, target, 0)
    row = fp.FpVector(2, target.dimension(0))  # target dim in degree 0 is 1
    row[0] = 1
    hom.add_generators_from_rows(0, [row])
    return hom


# --- construction / accessors ---------------------------------------------


def test_construct_and_invariants():
    hom = free_to_free_id(milnor(2))
    assert isinstance(hom.prime(), int)
    assert hom.prime() == 2
    assert hom.degree_shift() == 0
    assert hom.min_degree() == 0
    assert hom.next_degree() == 1
    assert repr(hom).startswith("FreeModuleHomomorphismToFree(")


def test_source_and_target_are_both_free_modules():
    hom = free_to_free_id(milnor(2))
    source = hom.source()
    target = hom.target()
    assert isinstance(source, algebra.FreeModule)
    assert isinstance(target, algebra.FreeModule)
    assert source.number_of_gens_in_degree(0) == 1
    assert target.number_of_gens_in_degree(0) == 1
    assert source.prime() == target.prime() == 2


def test_construct_requires_same_algebra():
    a1 = milnor(2)
    a2 = milnor(2)  # distinct algebra object
    source = free_gen_in_degree(a1, "F1", 0)
    target = free_gen_in_degree(a2, "F0", 0)
    with pytest.raises(ValueError):
        algebra.FreeModuleHomomorphismToFree(source, target, 0)


# --- apply / apply_to_basis_element / apply_to_generator / output ----------


def test_apply_to_basis_element_known_values():
    hom = free_to_free_id(milnor(2))
    # f(g) = a: basis element (degree 0, idx 0) -> [1].
    res = fp.FpVector(2, 1)
    hom.apply_to_basis_element(res, 1, 0, 0)
    assert res[0] == 1
    # f(Sq1 . g) = Sq1 . a = [1] in target degree 1 (dimension 1).
    res1 = fp.FpVector(2, hom.target().dimension(1))
    hom.apply_to_basis_element(res1, 1, 1, 0)
    assert res1[0] == 1


def test_apply_general_element():
    hom = free_to_free_id(milnor(2))
    inp = fp.FpVector(2, 1)
    inp[0] = 1
    res = fp.FpVector(2, 1)
    hom.apply(res, 1, 0, inp)
    assert res[0] == 1


def test_apply_to_generator_and_output():
    hom = free_to_free_id(milnor(2))
    res = fp.FpVector(2, 1)
    hom.apply_to_generator(res, 1, 0, 0)
    assert res[0] == 1
    out = hom.output(0, 0)
    assert isinstance(out, fp.FpVector)
    assert out[0] == 1


def test_apply_aliasing_input_and_target_raises():
    hom = free_to_free_id(milnor(2))
    v = fp.FpVector(2, 1)
    v[0] = 1
    with pytest.raises(RuntimeError):
        hom.apply(v, 1, 0, v)


# --- hom_k -----------------------------------------------------------------


def test_hom_k_known_value():
    hom = free_to_free_id(milnor(2))
    # The dual of the iso F1 -> F0 in degree 0 is the 1x1 identity.
    assert hom.hom_k(0) == [[1]]
    # No target generators in degree 1 -> empty list.
    assert hom.hom_k(1) == []


def test_hom_k_source_above_max_computed_degree_no_panic():
    # Target has generators up to degree 5, but the source only up to degree 2
    # (degree_shift == 0). Upstream `hom_k` reads
    # `source.number_of_gens_in_degree(t + shift)` before any early return, which
    # panics for a degree above the source's computed range. The binding must
    # instead return the correct upstream result: with no source generators in
    # `t + shift`, `source_dim` is morally 0, so the dual matrix is `target_dim`
    # rows of length 0. Here `target_dim == 1`, so the result is `[[]]`.
    alg = milnor(2)
    source = free_gen_in_degree(alg, "F1", 2)
    target = free_gen_in_degree(alg, "F0", 5)
    assert source.max_computed_degree() == 2
    assert target.number_of_gens_in_degree(5) == 1
    hom = algebra.FreeModuleHomomorphismToFree(source, target, 0)
    assert hom.hom_k(5) == [[]]


def test_hom_k_target_above_max_computed_degree_is_empty():
    # `t` above the target's computed range morally has 0 target generators, so
    # the empty list is returned rather than panicking (matches upstream's
    # `target_dim == 0 => vec![]`).
    alg = milnor(2)
    source = free_gen_in_degree(alg, "F1", 2)
    target = free_gen_in_degree(alg, "F0", 5)
    assert target.max_computed_degree() == 5
    hom = algebra.FreeModuleHomomorphismToFree(source, target, 0)
    assert hom.hom_k(6) == []


def test_hom_k_undefined_outputs_raises():
    alg = milnor(2)
    source = free_gen_in_degree(alg, "F1", 0)
    target = free_gen_in_degree(alg, "F0", 0)
    hom = algebra.FreeModuleHomomorphismToFree(source, target, 0)
    # Outputs on the degree-0 generator are not yet defined -> ValueError.
    with pytest.raises(ValueError):
        hom.hom_k(0)


# --- auxiliary data: kernel / image / quasi_inverse ------------------------


def test_auxiliary_data_dimensions_and_types():
    hom = free_to_free_id(milnor(2))
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
    hom = free_to_free_id(milnor(2))
    hom.compute_auxiliary_data_through_degree(0)
    inp = fp.FpVector(2, 1)
    inp[0] = 1
    res = fp.FpVector(2, 1)
    applied = hom.apply_quasi_inverse(res, 0, inp)
    assert applied is True
    assert res[0] == 1


def test_apply_quasi_inverse_returns_false_when_uncomputed():
    hom = free_to_free_id(milnor(2))
    res = fp.FpVector(2, 1)
    inp = fp.FpVector(2, 1)
    assert hom.apply_quasi_inverse(res, 0, inp) is False


def test_uncomputed_aux_data_reads_none():
    hom = free_to_free_id(milnor(2))
    assert hom.kernel(7) is None
    assert hom.image(7) is None
    assert hom.quasi_inverse(7) is None


# --- get_partial_matrix ----------------------------------------------------


def test_get_partial_matrix_in_range():
    hom = free_to_free_id(milnor(2))
    m = hom.get_partial_matrix(0, [0])
    assert isinstance(m, fp.Matrix)
    assert m.to_vec() == [[1]]


def test_get_partial_matrix_out_of_range_target_is_zero_matrix():
    # source.min_degree() = 0 but target.min_degree() = 1, so the output degree
    # 0 is below the target's range -> target dimension 0 -> the (1 x 0) zero
    # matrix is returned rather than panicking.
    alg = milnor(2)
    source = free_gen_in_degree(alg, "F1", 0)
    target = free_gen_in_degree(alg, "F0", 1, min_degree=1)
    hom = algebra.FreeModuleHomomorphismToFree(source, target, 0)
    m = hom.get_partial_matrix(0, [0])
    assert m.rows() == 1
    assert m.columns() == 0


# --- guards: errors instead of panics --------------------------------------


def test_apply_out_of_range_index_raises():
    hom = free_to_free_id(milnor(2))
    res = fp.FpVector(2, 1)
    with pytest.raises(IndexError):
        hom.apply_to_basis_element(res, 1, 0, 9)


def test_apply_length_and_prime_mismatch_raises():
    hom = free_to_free_id(milnor(2))
    bad_len = fp.FpVector(2, 3)
    with pytest.raises(ValueError):
        hom.apply_to_basis_element(bad_len, 1, 0, 0)
    bad_prime = fp.FpVector(3, 1)
    with pytest.raises(ValueError):
        hom.apply_to_basis_element(bad_prime, 1, 0, 0)


def test_apply_below_min_degree_raises():
    hom = free_to_free_id(milnor(2))
    res = fp.FpVector(2, 1)
    with pytest.raises(IndexError):
        hom.apply_to_basis_element(res, 1, -1, 0)


def test_add_generators_from_rows_non_consecutive_raises():
    hom = free_to_free_id(milnor(2))
    row = fp.FpVector(2, 1)
    with pytest.raises(ValueError):
        hom.add_generators_from_rows(5, [row])


def test_add_generators_from_matrix_rows():
    alg = milnor(2)
    source = free_gen_in_degree(alg, "F1", 0)
    target = free_gen_in_degree(alg, "F0", 0)
    hom = algebra.FreeModuleHomomorphismToFree(source, target, 0)
    matrix = fp.Matrix.from_vec(2, [[1]])
    hom.add_generators_from_matrix_rows(0, matrix)
    res = fp.FpVector(2, 1)
    hom.apply_to_basis_element(res, 1, 0, 0)
    assert res[0] == 1


def test_extend_by_zero_past_max_computed_degree_raises():
    hom = free_to_free_id(milnor(2))
    with pytest.raises(ValueError):
        hom.extend_by_zero(50)


def test_set_kernel_non_consecutive_raises():
    hom = free_to_free_id(milnor(2))
    sub = fp.Subspace(2, 1)
    with pytest.raises(ValueError):
        hom.set_kernel(3, sub)


def test_compute_auxiliary_data_out_of_sync_raises():
    hom = free_to_free_id(milnor(2))
    hom.set_image(0, None)
    with pytest.raises(ValueError):
        hom.compute_auxiliary_data_through_degree(0)


def test_differential_density_known_and_undefined():
    hom = free_to_free_id(milnor(2))
    assert hom.differential_density(0) == pytest.approx(1.0)
    with pytest.raises(ValueError):
        hom.differential_density(9)


# --- degree_shift != 0 -----------------------------------------------------


def c2_like_shift(alg):
    """f: F1 -> F0 with degree_shift = 1, F1 = <g> in degree 1, f(g) = a."""
    source = free_gen_in_degree(alg, "F1", 1)
    target = free_gen_in_degree(alg, "F0", 0)
    hom = algebra.FreeModuleHomomorphismToFree(source, target, 1)
    row = fp.FpVector(2, target.dimension(0))  # lands in target degree 1-1=0
    row[0] = 1
    hom.add_generators_from_rows(1, [row])
    return hom


def test_degree_shift_invariants_and_apply():
    hom = c2_like_shift(milnor(2))
    assert hom.degree_shift() == 1
    assert hom.min_degree() == 1
    assert hom.next_degree() == 2
    # output(1, 0) = a = [1] in target degree 0.
    assert hom.output(1, 0)[0] == 1
    # apply_to_basis_element(degree 1, idx 0) = f(g) = a = [1] in target deg 0.
    res = fp.FpVector(2, 1)
    hom.apply_to_basis_element(res, 1, 1, 0)
    assert res[0] == 1


def test_degree_shift_hom_k_known_value():
    hom = c2_like_shift(milnor(2))
    # f*: source generators in degree t + shift map to target gens in degree t.
    # In t = 0: target has gen a in degree 0, source has g in degree 1.
    assert hom.hom_k(0) == [[1]]


# --- state sharing ---------------------------------------------------------


def test_target_state_is_shared_not_snapshotted():
    alg = milnor(2)
    source = free_gen_in_degree(alg, "F1", 0)
    target = free_gen_in_degree(alg, "F0", 0)
    hom = algebra.FreeModuleHomomorphismToFree(source, target, 0)
    row = fp.FpVector(2, 1)
    row[0] = 1
    hom.add_generators_from_rows(0, [row])
    del target
    gc.collect()
    t = hom.target()
    assert t.number_of_gens_in_degree(0) == 1
    assert t.dimension(0) == 1
