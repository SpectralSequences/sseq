import gc

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
    assert isinstance(hom.prime, int)
    assert hom.prime == 2
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
    assert source.prime == target.prime == 2


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


# --- degree_shift != 0 -----------------------------------------------------
#
# Build f: F -> C2 with degree_shift = 1, where F has a single generator g in
# degree 1 and f(g) = x0 (the bottom cell of C2, living in target degree 0).
# Upstream `free_module_homomorphism.rs` defines `output_degree = input_degree
# - degree_shift` (line 64) and acts via
# `target.act(.., generator_degree - degree_shift, output_on_generator)`
# (lines 78-85), so:
#   * min_degree = max(source.min_degree(), target.min_degree() + degree_shift)
#                = max(0, 0 + 1) = 1.
#   * output(1, 0) = x0 = [1] in target degree 1 - 1 = 0 (dimension 1).
#   * apply_to_basis_element(degree 1, idx 0) = f(g) = x0 = [1] (target deg 0).
#   * apply_to_basis_element(degree 2, idx 0) = f(Sq1 . g) = Sq1 . x0 = x1 = [1]
#     in target degree 2 - 1 = 1 (dimension 1).


def free_module_gen_in_degree(alg, gen_degree):
    """A FreeModule whose only generator g lives in `gen_degree`.

    Generators must be added at consecutive degrees from min_degree = 0, so the
    intervening degrees are filled with empty generator lists.
    """
    b = algebra.FPModuleBuilder(alg, "F", 0)
    for d in range(gen_degree):
        b.add_generators(d, [])
    b.add_generators(gen_degree, ["g"])
    b.add_relations(0, [])
    return b.build().generators()


def c2_differential_shift(alg):
    """f: F -> C2 with degree_shift = 1, F = <g> in degree 1 and f(g) = x0."""
    source = free_module_gen_in_degree(alg, 1)
    target = algebra.steenrod_module_from_json(alg, C2_JSON)
    hom = algebra.FreeModuleHomomorphism(source, target, 1)
    # The output on g lands in target.dimension(1 - degree_shift) = dim(0) = 1.
    row = fp.FpVector(2, target.dimension(0))
    row[0] = 1
    hom.add_generators_from_rows(1, [row])
    return hom


def test_degree_shift_invariants():
    alg = milnor(2)
    hom = c2_differential_shift(alg)
    assert hom.degree_shift() == 1
    # min_degree = max(source.min_degree()=0, target.min_degree()=0 + shift=1).
    assert hom.min_degree() == 1
    assert hom.next_degree() == 2


def test_degree_shift_output_and_apply_to_generator():
    alg = milnor(2)
    hom = c2_differential_shift(alg)
    # output(1, 0) = x0 = [1] in target degree 0.
    out = hom.output(1, 0)
    assert isinstance(out, fp.FpVector)
    assert out[0] == 1
    # apply_to_generator lands in target degree 1 - 1 = 0 (dimension 1).
    res = fp.FpVector(2, 1)
    hom.apply_to_generator(res, 1, 1, 0)
    assert res[0] == 1


def test_degree_shift_apply_to_basis_element_known_values():
    alg = milnor(2)
    hom = c2_differential_shift(alg)
    # f(g): basis element (degree 1, idx 0) -> x0 = [1] in target degree 0.
    res0 = fp.FpVector(2, 1)
    hom.apply_to_basis_element(res0, 1, 1, 0)
    assert res0[0] == 1
    # f(Sq1 . g) = Sq1 . x0 = x1 = [1] in target degree 2 - 1 = 1.
    res1 = fp.FpVector(2, 1)
    hom.apply_to_basis_element(res1, 1, 2, 0)
    assert res1[0] == 1


def test_degree_shift_apply_general_element():
    alg = milnor(2)
    hom = c2_differential_shift(alg)
    inp = fp.FpVector(2, 1)
    inp[0] = 1
    res = fp.FpVector(2, 1)
    hom.apply(res, 1, 1, inp)
    assert res[0] == 1


def test_degree_shift_get_partial_matrix_dims_coincide():
    alg = milnor(2)
    hom = c2_differential_shift(alg)
    # degree 1: target.dimension(1) == target.dimension(1 - 1) == 1, so the
    # matrix is well-defined. Its single row is f(g) = x0 = [1].
    m = hom.get_partial_matrix(1, [0])
    assert isinstance(m, fp.Matrix)
    assert m.rows() == 1
    assert m.columns() == 1
    assert m.to_vec() == [[1]]


def test_degree_shift_get_partial_matrix_dims_differ_raises():
    alg = milnor(2)
    hom = c2_differential_shift(alg)
    # degree 2: target.dimension(2) = 0 but target.dimension(2 - 1) = 1, so the
    # documented guard rejects this with a clean ValueError (no panic).
    with pytest.raises(ValueError):
        hom.get_partial_matrix(2, [0])


def test_degree_shift_output_out_of_range_raises():
    alg = milnor(2)
    hom = c2_differential_shift(alg)
    # generator degree below min_degree (= 1) -> IndexError.
    with pytest.raises(IndexError):
        hom.output(0, 0)
    # generator degree at/above next_degree (= 2) -> ValueError.
    with pytest.raises(ValueError):
        hom.output(2, 0)
    # generator index out of range in degree 1 (only 1 generator) -> IndexError.
    with pytest.raises(IndexError):
        hom.output(1, 9)


# --- auxiliary-data sync-check error path -----------------------------------


def test_compute_auxiliary_data_out_of_sync_raises():
    # `compute_auxiliary_data_through_degree` extends the kernels/images/
    # quasi_inverses tables in lock-step (free_module_homomorphism.rs lines
    # 101-108 push all three at the same degree). A manual `set_image` that
    # advances only the images table leaves the three out of sync; the binding
    # detects this (images.len() != kernels.len()) and raises ValueError rather
    # than letting the upstream `push_checked` panic.
    alg = milnor(2)
    hom = c2_differential(alg)
    # Advance only the images table by one (degree 0 == images.len()).
    hom.set_image(0, None)
    with pytest.raises(ValueError):
        hom.compute_auxiliary_data_through_degree(0)


# --- remaining mutator guards -----------------------------------------------


def test_set_image_non_consecutive_raises():
    alg = milnor(2)
    hom = c2_differential(alg)
    # images table is empty (length min_degree = 0); degree 3 is non-consecutive.
    with pytest.raises(ValueError):
        hom.set_image(3, None)


def test_set_quasi_inverse_non_consecutive_raises():
    alg = milnor(2)
    hom = c2_differential(alg)
    # quasi_inverses table is empty; degree 3 is non-consecutive.
    with pytest.raises(ValueError):
        hom.set_quasi_inverse(3, None)


def test_extend_by_zero_past_max_computed_degree_raises():
    alg = milnor(2)
    hom = c2_differential(alg)
    # The source only has generators through degree 0 (max_computed_degree = 0),
    # so extending the outputs to degree 5 leaves a gap -> ValueError.
    with pytest.raises(ValueError):
        hom.extend_by_zero(5)


def test_add_generators_from_matrix_rows_wrong_row_count_raises():
    alg = milnor(2)
    source = free_module_one_generator(alg)
    target = algebra.steenrod_module_from_json(alg, C2_JSON)
    hom = algebra.FreeModuleHomomorphism(source, target, 0)
    # Degree 0 has 1 generator; a matrix with 0 rows is too few.
    empty = fp.Matrix(2, 0, 1)
    with pytest.raises(ValueError):
        hom.add_generators_from_matrix_rows(0, empty)


def test_add_generators_from_matrix_rows_wrong_column_dim_raises():
    alg = milnor(2)
    source = free_module_one_generator(alg)
    target = algebra.steenrod_module_from_json(alg, C2_JSON)
    hom = algebra.FreeModuleHomomorphism(source, target, 0)
    # target.dimension(0) = 1, so a 3-column matrix has the wrong width.
    wide = fp.Matrix.from_vec(2, [[0, 0, 0]])
    with pytest.raises(ValueError):
        hom.add_generators_from_matrix_rows(0, wide)


def test_add_generators_from_rows_wrong_length_raises():
    alg = milnor(2)
    source = free_module_one_generator(alg)
    target = algebra.steenrod_module_from_json(alg, C2_JSON)
    hom = algebra.FreeModuleHomomorphism(source, target, 0)
    # target.dimension(0) = 1, so a length-3 row is the wrong length.
    bad_len = fp.FpVector(2, 3)
    with pytest.raises(ValueError):
        hom.add_generators_from_rows(0, [bad_len])


def test_add_generators_from_rows_wrong_prime_raises():
    alg = milnor(2)
    source = free_module_one_generator(alg)
    target = algebra.steenrod_module_from_json(alg, C2_JSON)
    hom = algebra.FreeModuleHomomorphism(source, target, 0)
    bad_prime = fp.FpVector(3, 1)
    with pytest.raises(ValueError):
        hom.add_generators_from_rows(0, [bad_prime])


# --- output() / apply out-of-range guards (no panic) ------------------------


def test_output_below_min_degree_raises():
    alg = milnor(2)
    hom = c2_differential(alg)
    # min_degree = 0, so degree -1 is below the defined range.
    with pytest.raises(IndexError):
        hom.output(-1, 0)


def test_output_above_next_degree_raises():
    alg = milnor(2)
    hom = c2_differential(alg)
    # next_degree = 1, so degree 5 has no defined outputs yet.
    with pytest.raises(ValueError):
        hom.output(5, 0)


def test_output_generator_index_out_of_range_raises():
    alg = milnor(2)
    hom = c2_differential(alg)
    # Degree 0 has a single generator; index 9 is out of range.
    with pytest.raises(IndexError):
        hom.output(0, 9)


def test_apply_to_basis_element_below_min_degree_raises():
    alg = milnor(2)
    hom = c2_differential(alg)
    res = fp.FpVector(2, 1)
    # source.min_degree() = 0, so input degree -1 is below the source.
    with pytest.raises(IndexError):
        hom.apply_to_basis_element(res, 1, -1, 0)


# --- state sharing through source()/target() --------------------------------


def test_target_state_is_shared_not_snapshotted():
    # The homomorphism holds the target's `Arc`, so it shares state with the
    # built SteenrodModule rather than snapshotting it. A SteenrodModule built
    # via FDModuleBuilder(...).build() works as a homomorphism target (the
    # consumer that previously used into_steenrod_module()).
    alg = milnor(2)
    fd = algebra.FDModuleBuilder(alg, "C2", [1, 1], 0)
    fd.set_action(1, 0, 0, 0, [1])  # Sq1 x0 = x1
    sm = fd.build()
    # After build() the builder is locked (the `built` flag fires first).
    with pytest.raises(RuntimeError):
        fd.add_generator(2, "y")

    source = free_module_one_generator(alg)
    hom = algebra.FreeModuleHomomorphism(source, sm, 0)
    row = fp.FpVector(2, 1)
    row[0] = 1
    hom.add_generators_from_rows(0, [row])  # f(g) = x0

    # Drop the directly held box; the homomorphism keeps its own shared Arc.
    del sm
    gc.collect()

    # The homomorphism's target() handle still reflects the shared module.
    target = hom.target()
    assert target.dimension(0) == 1
    assert target.dimension(1) == 1
    res = fp.FpVector(2, 1)
    hom.apply_to_basis_element(res, 1, 0, 0)
    assert res[0] == 1


def test_source_handle_reflects_underlying_state():
    # FreeModule is query-only from Python (no mutators), so a shared-vs-
    # snapshot distinction cannot be exercised via mutation. The closest
    # meaningful observation is that source() hands back a handle faithfully
    # reflecting the same underlying module (same prime, generator counts and
    # computed-degree bound) as the FreeModule passed to the constructor, and
    # that repeated calls agree.
    alg = milnor(2)
    source = free_module_one_generator(alg)
    target = algebra.steenrod_module_from_json(alg, C2_JSON)
    hom = algebra.FreeModuleHomomorphism(source, target, 0)

    s1 = hom.source()
    s2 = hom.source()
    assert s1.prime == source.prime == 2
    assert s1.number_of_gens_in_degree(0) == source.number_of_gens_in_degree(0) == 1
    assert s1.max_computed_degree() == source.max_computed_degree()
    assert s2.number_of_gens_in_degree(0) == 1
