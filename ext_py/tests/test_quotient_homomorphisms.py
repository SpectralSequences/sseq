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
    return algebra.SteenrodModule.from_spec(C2_JSON, alg)


def identity_and_quotient(alg):
    """The identity FullModuleHomomorphism on a single C2 object plus a
    QuotientModule view of the *same* module object (so the Arc-identity guard
    in the quotient-homomorphism constructors is satisfied)."""
    m = c2_module(alg)
    f = algebra.FullModuleHomomorphism.identity(m)
    q = algebra.QuotientModule(m, 1)
    return m, f, q


# --- binding presence ------------------------------------------------------


def test_classes_in_module():
    assert "QuotientHomomorphism" in dir(algebra)
    assert "QuotientHomomorphismSource" in dir(algebra)
    assert "GenericZeroHomomorphism" in dir(algebra)


# --- QuotientHomomorphism --------------------------------------------------


def test_quotient_hom_construct_and_invariants():
    alg = milnor(2)
    _m, f, q = identity_and_quotient(alg)
    qh = algebra.QuotientHomomorphism(f, q, q)
    assert isinstance(qh.prime, int)
    assert qh.prime == 2
    assert qh.degree_shift == 0
    assert qh.min_degree == 0
    assert repr(qh).startswith("QuotientHomomorphism(")
    # source / target are the bound QuotientModule, sharing state.
    assert isinstance(qh.source, algebra.QuotientModule)
    assert isinstance(qh.target, algebra.QuotientModule)
    assert qh.source.dimension(0) == 1
    assert qh.target.dimension(1) == 1


def test_quotient_hom_identity_known_values():
    alg = milnor(2)
    _m, f, q = identity_and_quotient(alg)
    qh = algebra.QuotientHomomorphism(f, q, q)
    # induced identity: x0 -> [1] in degree 0, x1 -> [1] in degree 1.
    r0 = fp.FpVector(2, 1)
    qh.apply_to_basis_element(r0, 1, 0, 0)
    assert r0[0] == 1
    r1 = fp.FpVector(2, 1)
    qh.apply_to_basis_element(r1, 1, 1, 0)
    assert r1[0] == 1
    # apply on a general element [1] in degree 0 agrees.
    inp = fp.FpVector(2, 1)
    inp[0] = 1
    r2 = fp.FpVector(2, 1)
    qh.apply(r2, 1, 0, inp)
    assert r2[0] == 1


def test_quotient_hom_target_quotiented_is_zero():
    alg = milnor(2)
    m = c2_module(alg)
    f = algebra.FullModuleHomomorphism.identity(m)
    q_src = algebra.QuotientModule(m, 1)
    q_tgt = algebra.QuotientModule(m, 1)
    # Quotient out x1 in the target before building the homomorphism.
    q_tgt.quotient_basis_elements(1, [0])
    assert q_tgt.dimension(1) == 0
    qh = algebra.QuotientHomomorphism(f, q_src, q_tgt)
    # x1 lives in source degree 1; its image lands in the now-zero target
    # degree 1. A length-0 result is accepted and left untouched (no panic).
    r = fp.FpVector(2, 0)
    qh.apply_to_basis_element(r, 1, 1, 0)
    assert len(r) == 0
    # A wrong (nonzero) result length raises rather than panicking.
    bad = fp.FpVector(2, 1)
    with pytest.raises(ValueError):
        qh.apply_to_basis_element(bad, 1, 1, 0)


def test_quotient_hom_rejects_foreign_modules():
    alg = milnor(2)
    m = c2_module(alg)
    n = c2_module(alg)
    f = algebra.FullModuleHomomorphism.identity(m)
    # A quotient of a *different* module object is rejected.
    q_other = algebra.QuotientModule(n, 1)
    with pytest.raises(ValueError):
        algebra.QuotientHomomorphism(f, q_other, q_other)


def test_quotient_hom_guards_raise_not_panic():
    alg = milnor(2)
    _m, f, q = identity_and_quotient(alg)
    qh = algebra.QuotientHomomorphism(f, q, q)
    # Out-of-range source index -> IndexError.
    res = fp.FpVector(2, 1)
    with pytest.raises(IndexError):
        qh.apply_to_basis_element(res, 1, 0, 9)
    # Wrong result length -> ValueError.
    bad = fp.FpVector(2, 3)
    with pytest.raises(ValueError):
        qh.apply_to_basis_element(bad, 1, 0, 0)
    # Prime mismatch -> ValueError.
    badp = fp.FpVector(3, 1)
    with pytest.raises(ValueError):
        qh.apply_to_basis_element(badp, 1, 0, 0)
    # Below source min_degree -> IndexError.
    with pytest.raises(IndexError):
        qh.apply_to_basis_element(res, 1, -1, 0)


def test_quotient_hom_no_auxiliary_data():
    alg = milnor(2)
    _m, f, q = identity_and_quotient(alg)
    qh = algebra.QuotientHomomorphism(f, q, q)
    # No auxiliary data is ever stored for a quotient homomorphism.
    qh.compute_auxiliary_data_through_degree(1)
    assert qh.kernel(0) is None
    assert qh.image(0) is None
    assert qh.quasi_inverse(0) is None
    res = fp.FpVector(2, 1)
    inp = fp.FpVector(2, 1)
    assert qh.apply_quasi_inverse(res, 0, inp) is False


def test_quotient_hom_get_partial_matrix():
    alg = milnor(2)
    _m, f, q = identity_and_quotient(alg)
    qh = algebra.QuotientHomomorphism(f, q, q)
    gm = qh.get_partial_matrix(0, [0])
    assert isinstance(gm, fp.Matrix)
    assert gm.rows == 1
    assert gm.columns == 1
    assert gm.to_vec() == [[1]]


def test_quotient_hom_apply_aliasing_raises():
    alg = milnor(2)
    _m, f, q = identity_and_quotient(alg)
    qh = algebra.QuotientHomomorphism(f, q, q)
    v = fp.FpVector(2, 1)
    v[0] = 1
    with pytest.raises(RuntimeError):
        qh.apply(v, 1, 0, v)


def test_quotient_hom_partial_matrix_below_target_min():
    # Finding 1: source/target quotients with asymmetric min_degree
    # (src_min = 0 < target_min = 2). Querying get_partial_matrix at a degree in
    # [src_min, target_min) used to panic in the trait-default's
    # target.dimension(degree) (QuotientModule subspaces BiVec assert); it now
    # returns a correctly-shaped (len(inputs) x 0) zero matrix.
    alg = milnor(2)
    sb = algebra.FDModuleBuilder(alg, "s", [1], 0)
    source = sb.build()
    tb = algebra.FDModuleBuilder(alg, "t", [1], 2)
    target = tb.build()
    f = algebra.FullModuleHomomorphism(source, target, 0)
    q_src = algebra.QuotientModule(source, 3)
    q_tgt = algebra.QuotientModule(target, 3)
    qh = algebra.QuotientHomomorphism(f, q_src, q_tgt)
    assert qh.source.min_degree == 0
    assert qh.target.min_degree == 2
    gm = qh.get_partial_matrix(0, [0])
    assert isinstance(gm, fp.Matrix)
    assert gm.rows == 1
    assert gm.columns == 0


def test_quotient_hom_partial_matrix_degree_shift_guard():
    # With degree_shift != 0, get_partial_matrix raises ValueError when
    # target.dimension(degree) != target.dimension(degree - degree_shift) (both
    # nonzero). dim(0) = 1, dim(1) = 2 makes the two differ.
    alg = milnor(2)
    mb = algebra.FDModuleBuilder(alg, "m", [1, 2], 0)
    m = mb.build()
    f = algebra.FullModuleHomomorphism(m, m, 1)
    q = algebra.QuotientModule(m, 3)
    qh = algebra.QuotientHomomorphism(f, q, q)
    assert qh.degree_shift == 1
    with pytest.raises(ValueError):
        qh.get_partial_matrix(1, [0])


# --- QuotientHomomorphismSource --------------------------------------------


def test_quotient_hom_source_construct_and_types():
    alg = milnor(2)
    _m, f, q = identity_and_quotient(alg)
    qhs = algebra.QuotientHomomorphismSource(f, q)
    assert qhs.prime == 2
    assert qhs.degree_shift == 0
    assert qhs.min_degree == 0
    assert repr(qhs).startswith("QuotientHomomorphismSource(")
    # source is the quotient; target is the plain SteenrodModule.
    assert isinstance(qhs.source, algebra.QuotientModule)
    assert isinstance(qhs.target, algebra.SteenrodModule)
    assert qhs.source.dimension(0) == 1
    assert qhs.target.dimension(1) == 1


def test_quotient_hom_source_known_values():
    alg = milnor(2)
    _m, f, q = identity_and_quotient(alg)
    qhs = algebra.QuotientHomomorphismSource(f, q)
    # Identity into the un-quotiented target: x0 -> [1], x1 -> [1].
    r0 = fp.FpVector(2, 1)
    qhs.apply_to_basis_element(r0, 1, 0, 0)
    assert r0[0] == 1
    r1 = fp.FpVector(2, 1)
    qhs.apply_to_basis_element(r1, 1, 1, 0)
    assert r1[0] == 1


def test_quotient_hom_source_rejects_foreign_module():
    alg = milnor(2)
    m = c2_module(alg)
    n = c2_module(alg)
    f = algebra.FullModuleHomomorphism.identity(m)
    q_other = algebra.QuotientModule(n, 1)
    with pytest.raises(ValueError):
        algebra.QuotientHomomorphismSource(f, q_other)


def test_quotient_hom_source_no_auxiliary_data():
    alg = milnor(2)
    _m, f, q = identity_and_quotient(alg)
    qhs = algebra.QuotientHomomorphismSource(f, q)
    assert qhs.kernel(0) is None
    assert qhs.image(0) is None
    assert qhs.quasi_inverse(0) is None


def test_quotient_hom_source_get_partial_matrix():
    # The induced identity into the un-quotiented target is the 1x1 [[1]].
    alg = milnor(2)
    _m, f, q = identity_and_quotient(alg)
    qhs = algebra.QuotientHomomorphismSource(f, q)
    gm = qhs.get_partial_matrix(0, [0])
    assert isinstance(gm, fp.Matrix)
    assert gm.rows == 1
    assert gm.columns == 1
    assert gm.to_vec() == [[1]]


# --- GenericZeroHomomorphism -----------------------------------------------


def test_generic_zero_construct_and_invariants():
    alg = milnor(2)
    m = c2_module(alg)
    z = algebra.GenericZeroHomomorphism(m, m, 0)
    assert z.prime == 2
    assert z.degree_shift == 0
    assert z.min_degree == 0
    assert repr(z).startswith("GenericZeroHomomorphism(")
    assert isinstance(z.source, algebra.SteenrodModule)
    assert isinstance(z.target, algebra.SteenrodModule)
    assert z.source.dimension(0) == 1
    assert z.target.dimension(1) == 1


def test_generic_zero_maps_everything_to_zero():
    alg = milnor(2)
    m = c2_module(alg)
    z = algebra.GenericZeroHomomorphism(m, m, 0)
    # apply_to_basis_element adds nothing.
    r = fp.FpVector(2, 1)
    r[0] = 1
    z.apply_to_basis_element(r, 1, 0, 0)
    assert r[0] == 1  # unchanged (added 0)
    # apply on a general element adds nothing.
    inp = fp.FpVector(2, 1)
    inp[0] = 1
    out = fp.FpVector(2, 1)
    z.apply(out, 1, 0, inp)
    assert out[0] == 0


def test_generic_zero_default_degree_shift():
    alg = milnor(2)
    m = c2_module(alg)
    z = algebra.GenericZeroHomomorphism(m, m)
    assert z.degree_shift == 0


def test_generic_zero_get_partial_matrix_is_zero():
    alg = milnor(2)
    m = c2_module(alg)
    z = algebra.GenericZeroHomomorphism(m, m, 0)
    gm = z.get_partial_matrix(0, [0])
    assert isinstance(gm, fp.Matrix)
    assert gm.rows == 1
    assert gm.columns == 1
    assert gm.to_vec() == [[0]]


def test_generic_zero_no_auxiliary_data():
    alg = milnor(2)
    m = c2_module(alg)
    z = algebra.GenericZeroHomomorphism(m, m, 0)
    z.compute_auxiliary_data_through_degree(1)
    assert z.kernel(0) is None
    assert z.image(0) is None
    assert z.quasi_inverse(0) is None
    res = fp.FpVector(2, 1)
    inp = fp.FpVector(2, 1)
    assert z.apply_quasi_inverse(res, 0, inp) is False


def test_generic_zero_requires_same_algebra():
    a1 = milnor(2)
    a2 = milnor(2)  # distinct algebra object
    m1 = c2_module(a1)
    m2 = c2_module(a2)
    with pytest.raises(ValueError):
        algebra.GenericZeroHomomorphism(m1, m2, 0)


def test_generic_zero_guards_raise_not_panic():
    alg = milnor(2)
    m = c2_module(alg)
    z = algebra.GenericZeroHomomorphism(m, m, 0)
    # Out-of-range source index -> IndexError.
    res = fp.FpVector(2, 1)
    with pytest.raises(IndexError):
        z.apply_to_basis_element(res, 1, 0, 9)
    # Wrong result length -> ValueError.
    bad = fp.FpVector(2, 3)
    with pytest.raises(ValueError):
        z.apply_to_basis_element(bad, 1, 0, 0)


def test_generic_zero_apply_aliasing_raises():
    alg = milnor(2)
    m = c2_module(alg)
    z = algebra.GenericZeroHomomorphism(m, m, 0)
    v = fp.FpVector(2, 1)
    v[0] = 1
    with pytest.raises(RuntimeError):
        z.apply(v, 1, 0, v)


def test_generic_zero_partial_matrix_beyond_target_range():
    # Finding 2: degree_shift > 0 with a FreeModule-backed target. The binding
    # only ensured the target through the (smaller) output degree, so the
    # trait-default's target.dimension(degree) used to reach beyond the
    # FreeModule's computed range and trip its "not computed through degree"
    # assert. It now returns a clean (len(inputs) x 0) zero matrix.
    alg = milnor(2)
    source = c2_module(alg)
    # An empty FreeModule (no generators) over the same algebra object.
    free = algebra.FPModuleBuilder(alg, "t", 0).build().generators()
    target = free.into_steenrod_module()
    z = algebra.GenericZeroHomomorphism(source, target, 1)
    assert z.degree_shift == 1
    # Source (C2) has dimension 1 in degree 1, so input index 0 is valid.
    gm = z.get_partial_matrix(1, [0])
    assert isinstance(gm, fp.Matrix)
    assert gm.rows == 1
    assert gm.columns == 0
