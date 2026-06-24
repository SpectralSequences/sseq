import pytest

from ext import algebra, fp


def milnor(p=2):
    return algebra.SteenrodAlgebra.milnor(p)


def make_c2(alg):
    """Build a C2 SteenrodModule over the given algebra via an FDModuleBuilder."""
    m = algebra.FDModuleBuilder(alg, "C2", [1, 1])
    m.set_action(1, 0, 0, 0, [1])  # Sq1 x0 = x1
    return m.build()


def free_one_gen(alg):
    """A FreeModule with a single generator in degree 0.

    `FreeModule` is query-only (no Python mutators), so build it through the
    remaining path: an FPModule's `generators()` FreeModule.
    """
    b = algebra.FPModuleBuilder(alg, "F", 0)
    b.add_generators(0, ["x0"])
    f = b.build().generators()
    f.compute_basis(4)
    return f


# --- QuotientModule -------------------------------------------------------


def test_quotient_module_basic_dimensions():
    alg = milnor(2)
    q = algebra.QuotientModule(make_c2(alg), 1)
    q.compute_basis(2)
    assert isinstance(q.prime(), int)
    assert q.prime() == 2
    assert q.min_degree() == 0
    assert q.truncation == 1
    # Nothing quotiented yet: same dims as C2 ([1, 1]).
    assert q.dimension(0) == 1
    assert q.dimension(1) == 1
    assert q.dimension(2) == 0
    assert q.max_degree() == 1
    assert q.total_dimension() == 2


def test_quotient_module_truncation_zeroes_above():
    alg = milnor(2)
    # Truncate C2 at degree 0: degree 1 is quotiented away entirely.
    q = algebra.QuotientModule(make_c2(alg), 0)
    q.compute_basis(2)
    assert q.dimension(0) == 1
    assert q.dimension(1) == 0
    assert q.max_degree() == 0
    assert q.total_dimension() == 1


def test_quotient_module_quotient_basis_elements():
    alg = milnor(2)
    q = algebra.QuotientModule(make_c2(alg), 1)
    q.compute_basis(2)
    # Quotient out x1 (the unique basis element in degree 1).
    q.quotient_basis_elements(1, [0])
    assert q.dimension(1) == 0
    assert q.dimension(0) == 1
    assert q.total_dimension() == 1


def test_quotient_module_quotient_vector_and_reduce():
    alg = milnor(2)
    q = algebra.QuotientModule(make_c2(alg), 1)
    q.compute_basis(2)
    # Quotient out x1 by supplying its coefficient vector in degree 1.
    elt = fp.FpVector(2, q.dimension(1))  # original dim in degree 1 is 1
    elt[0] = 1
    q.quotient(1, elt)
    assert q.dimension(1) == 0
    # Reducing [1] in degree 1 now projects onto the (empty) complement -> 0.
    v = fp.FpVector(2, 1)
    v[0] = 1
    q.reduce(1, v)
    assert v[0] == 0


def test_quotient_module_reduce_above_truncation_zeroes():
    alg = milnor(2)
    q = algebra.QuotientModule(make_c2(alg), 0)
    q.compute_basis(2)
    # Degree 1 is above the truncation: reduce zeroes any vector.
    v = fp.FpVector(2, 1)
    v[0] = 1
    q.reduce(1, v)
    assert v[0] == 0


def test_quotient_module_old_basis_to_new():
    alg = milnor(2)
    q = algebra.QuotientModule(make_c2(alg), 1)
    q.compute_basis(2)
    q.quotient_basis_elements(1, [0])
    # In degree 0, x0 survives, so the original [1] maps to the new [1].
    new = fp.FpVector(2, q.dimension(0))
    old = fp.FpVector(2, 1)
    old[0] = 1
    q.old_basis_to_new(0, new, old)
    assert new[0] == 1


def test_quotient_module_old_basis_to_new_aliasing_raises_runtimeerror():
    # In degree 0 the original and quotient dimensions are both 1, so a single
    # length-1 vector is shape-valid as both `old` (input) and `new` (target).
    # Aliasing them must raise RuntimeError (borrow conflict), NOT ValueError.
    alg = milnor(2)
    q = algebra.QuotientModule(make_c2(alg), 1)
    q.compute_basis(2)
    q.quotient_basis_elements(1, [0])
    v = fp.FpVector(2, q.dimension(0))
    v[0] = 1
    with pytest.raises(RuntimeError):
        q.old_basis_to_new(0, v, v)
    with pytest.raises(Exception) as excinfo:
        q.old_basis_to_new(0, v, v)
    assert not isinstance(excinfo.value, ValueError)


def test_quotient_module_old_basis_to_new_wrong_type_is_valueerror():
    alg = milnor(2)
    q = algebra.QuotientModule(make_c2(alg), 1)
    q.compute_basis(2)
    q.quotient_basis_elements(1, [0])
    new = fp.FpVector(2, q.dimension(0))
    old = fp.FpVector(2, 1)
    old[0] = 1
    with pytest.raises(ValueError):
        q.old_basis_to_new(0, 123, old)
    with pytest.raises(ValueError):
        q.old_basis_to_new(0, new, 123)


def test_quotient_module_action_is_reduced():
    alg = milnor(2)
    q = algebra.QuotientModule(make_c2(alg), 1)
    q.compute_basis(2)
    # Before quotienting: Sq1 . x0 = x1.
    res = fp.FpVector(2, q.dimension(1))
    q.act_on_basis(res, 1, 1, 0, 0, 0)
    assert res[0] == 1
    # After quotienting out x1, Sq1 . x0 reduces to 0 in the quotient.
    q.quotient_basis_elements(1, [0])
    assert q.dimension(1) == 0
    res = fp.FpVector(2, q.dimension(1))  # length 0
    q.act_on_basis(res, 1, 1, 0, 0, 0)
    assert sum(res) == 0


def test_quotient_module_out_of_range_raises():
    alg = milnor(2)
    q = algebra.QuotientModule(make_c2(alg), 1)
    q.compute_basis(2)
    # Degree outside [min_degree, truncation] raises for the subspace setters.
    with pytest.raises(IndexError):
        q.quotient_basis_elements(5, [0])
    with pytest.raises(IndexError):
        q.quotient_all(-1)
    # Out-of-range basis index raises.
    with pytest.raises(IndexError):
        q.quotient_basis_elements(0, [9])
    # reduce below min_degree raises.
    v = fp.FpVector(2, 1)
    with pytest.raises(IndexError):
        q.reduce(-1, v)


def test_quotient_module_construct_below_min_degree_raises():
    alg = milnor(2)
    with pytest.raises(ValueError):
        algebra.QuotientModule(make_c2(alg), -5)


def test_quotient_module_free_inner_uncomputed_algebra_no_panic():
    # A FreeModule inner whose Steenrod algebra has NOT been computed: a
    # truncation above the algebra's computed degree used to make upstream
    # `module.compute_basis(truncation)` index past the empty algebra basis
    # and panic. `QuotientModule.new` now pre-extends the inner module.
    alg = milnor(2)
    # One generator in degree 0; the algebra is NOT computed up to truncation.
    b = algebra.FPModuleBuilder(alg, "F", 0)
    b.add_generators(0, ["x0"])
    f = b.build().generators()
    q = algebra.QuotientModule(f.into_steenrod_module(), 20)
    assert q.prime() == 2
    assert q.truncation == 20
    assert q.min_degree() == 0
    # F<x0> over A: dim in degree t equals the algebra dimension in t.
    assert q.dimension(0) == 1  # 1 (unit)
    assert q.dimension(1) == 1  # Sq1
    assert q.dimension(2) == 1  # Sq2
    assert q.dimension(3) == 2  # Sq3, Sq2 Sq1


def test_quotient_module_mutation_works_again_after_box_dropped():
    # The mutation lock is only active while a boxed SteenrodModule from this
    # module is alive; dropping it restores unique ownership and mutation.
    alg = milnor(2)
    q = algebra.QuotientModule(make_c2(alg), 1)
    q.compute_basis(2)
    boxed = q.into_steenrod_module()
    with pytest.raises(RuntimeError):
        q.quotient_basis_elements(1, [0])
    # Drop the only outstanding box; the Arc is unique again.
    del boxed
    q.quotient_basis_elements(1, [0])
    assert q.dimension(1) == 0


def test_quotient_module_into_steenrod_module_roundtrip_and_locks():
    alg = milnor(2)
    q = algebra.QuotientModule(make_c2(alg), 1)
    q.compute_basis(2)
    boxed = q.into_steenrod_module()
    assert boxed.prime() == q.prime()
    assert boxed.dimension(0) == q.dimension(0)
    assert boxed.dimension(1) == q.dimension(1)
    # After boxing the Arc is shared, so mutation now raises RuntimeError.
    with pytest.raises(RuntimeError):
        q.quotient_basis_elements(0, [0])


def test_quotient_module_out_of_range_dimension_no_panic():
    alg = milnor(2)
    q = algebra.QuotientModule(make_c2(alg), 1)
    assert q.dimension(-5) == 0
    assert q.dimension(100) == 0
    with pytest.raises(IndexError):
        q.basis_element_to_string(-1, 0)


# --- HomModule ------------------------------------------------------------


def test_hom_module_dimensions():
    alg = milnor(2)
    source = free_one_gen(alg)
    target = make_c2(alg)
    hom = algebra.HomModule(source, target)
    assert isinstance(hom.prime(), int)
    assert hom.prime() == 2
    # min_degree = source.min_degree() - target.max_degree() = 0 - 1.
    assert hom.min_degree() == -1
    hom.compute_basis(0)
    # Hom(F<x0>, C2) graded opposite: dim in degree d = target.dim(-d).
    assert hom.dimension(-1) == 1  # target.dim(1)
    assert hom.dimension(0) == 1  # target.dim(0)
    assert hom.dimension(1) == 0  # target.dim(-1)


def test_hom_module_source_target_roundtrip():
    alg = milnor(2)
    source = free_one_gen(alg)
    target = make_c2(alg)
    hom = algebra.HomModule(source, target)
    s = hom.source()
    assert s.min_degree() == 0
    assert s.number_of_gens_in_degree(0) == 1
    t = hom.target()
    assert t.dimension(0) == 1
    assert t.dimension(1) == 1


def test_hom_module_scalar_action():
    alg = milnor(2)
    hom = algebra.HomModule(free_one_gen(alg), make_c2(alg))
    hom.compute_basis(0)
    # The field unit (op degree 0, index 0) fixes a basis element.
    res = fp.FpVector(2, hom.dimension(-1))
    hom.act_on_basis(res, 1, 0, 0, -1, 0)
    assert res[0] == 1


def test_hom_module_basis_element_to_string():
    alg = milnor(2)
    hom = algebra.HomModule(free_one_gen(alg), make_c2(alg))
    hom.compute_basis(0)
    assert isinstance(hom.basis_element_to_string(-1, 0), str)
    assert isinstance(hom.is_unit(), bool)


def test_hom_module_prime_mismatch_raises():
    source = free_one_gen(milnor(2))
    target = make_c2(milnor(3))
    with pytest.raises(ValueError):
        algebra.HomModule(source, target)


def test_hom_module_distinct_algebra_raises():
    # Same prime but two distinct algebra objects are incompatible.
    source = free_one_gen(milnor(2))
    target = make_c2(milnor(2))
    with pytest.raises(ValueError):
        algebra.HomModule(source, target)


def test_hom_module_unbounded_target_raises():
    alg = milnor(2)
    source = free_one_gen(alg)
    # A FreeModule is unbounded above; Hom requires a bounded target.
    unbounded = free_one_gen(alg).into_steenrod_module()
    with pytest.raises(ValueError):
        algebra.HomModule(source, unbounded)


def test_hom_module_out_of_range_no_panic():
    alg = milnor(2)
    hom = algebra.HomModule(free_one_gen(alg), make_c2(alg))
    assert hom.dimension(-5) == 0
    assert hom.dimension(5) == 0
    with pytest.raises(IndexError):
        hom.basis_element_to_string(-5, 0)


def test_hom_module_overflow_degree_no_panic():
    # target.max_degree() == 1, so the upstream compute_basis would add
    # i32::MAX + 1 and overflow. The degree-touching methods short-circuit
    # cleanly instead of panicking.
    alg = milnor(2)
    hom = algebra.HomModule(free_one_gen(alg), make_c2(alg))
    IMAX = 2147483647
    assert hom.dimension(IMAX) == 0
    with pytest.raises(IndexError):
        hom.basis_element_to_string(IMAX, 0)
    res = fp.FpVector(2, 0)
    with pytest.raises((IndexError, ValueError)):
        hom.act_on_basis(res, 1, 0, 0, IMAX, 0)


def test_hom_module_total_dimension_unbounded_raises():
    alg = milnor(2)
    hom = algebra.HomModule(free_one_gen(alg), make_c2(alg))
    # Hom over a free source is unbounded above.
    with pytest.raises(ValueError):
        hom.total_dimension()


def test_hom_module_has_no_into_steenrod_module():
    # HomModule is over the ground field, not the Steenrod algebra, so it is
    # deliberately not a SteenrodModule and exposes no into_steenrod_module().
    alg = milnor(2)
    hom = algebra.HomModule(free_one_gen(alg), make_c2(alg))
    assert not hasattr(hom, "into_steenrod_module")
