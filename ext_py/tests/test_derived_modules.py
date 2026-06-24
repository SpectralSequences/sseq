import pytest

from ext import algebra, fp


# The C2 module: a generator x0 in degree 0 and x1 in degree 1 with Sq1 x0 = x1.
C2_JSON = {
    "p": 2,
    "type": "finite dimensional module",
    "gens": {"x0": 0, "x1": 1},
    "actions": ["Sq1 x0 = x1"],
}


def milnor(p=2):
    return algebra.SteenrodAlgebra.milnor(p)


def adem(p=2):
    return algebra.SteenrodAlgebra.adem(p)


def make_c2(alg):
    """Build a C2 SteenrodModule over the given algebra via an FDModuleBuilder."""
    m = algebra.FDModuleBuilder(alg, "C2", [1, 1])
    m.set_action(1, 0, 0, 0, [1])
    return m.build()


# --- TensorModule ---------------------------------------------------------


def test_tensor_module_dimensions():
    alg = milnor(2)
    t = algebra.TensorModule(make_c2(alg), make_c2(alg))
    t.compute_basis(4)
    assert isinstance(t.prime(), int)
    assert t.prime() == 2
    assert t.min_degree() == 0
    # C2 (x) C2: convolution of [1, 1] with [1, 1] = [1, 2, 1].
    assert t.dimension(0) == 1
    assert t.dimension(1) == 2
    assert t.dimension(2) == 1
    assert t.dimension(3) == 0
    assert t.max_degree() == 2
    assert t.total_dimension() == 4


def test_tensor_module_action_decomposable():
    alg = milnor(2)
    t = algebra.TensorModule(make_c2(alg), make_c2(alg))
    t.compute_basis(4)
    # Sq1 . (x0 (x) x0) = x1 (x) x0 + x0 (x) x1 (both entries 1 at p = 2).
    res = fp.FpVector(2, t.dimension(1))
    t.act_on_basis(res, 1, 1, 0, 0, 0)
    assert res[0] == 1
    assert res[1] == 1
    assert sum(res) == 2


def test_tensor_module_seek_and_offset():
    alg = milnor(2)
    t = algebra.TensorModule(make_c2(alg), make_c2(alg))
    t.compute_basis(4)
    # Thin extras are callable and consistent.
    assert isinstance(t.seek_module_num(1, 0), int)
    assert isinstance(t.offset(1, 0), int)
    # Out-of-range basis index / left_degree raise rather than panic.
    with pytest.raises(IndexError):
        t.seek_module_num(1, 9)
    with pytest.raises(IndexError):
        t.offset(1, 9)


def test_tensor_module_offset_empty_block_raises():
    # A left factor with an internal degree gap (graded dims [1, 0, 1]) has
    # dimension 0 in degree 1. `offset(2, 1)` is inside the accepted left-degree
    # range [0, 2] but addresses an empty block; upstream would index
    # `blocks[1][0]` out of bounds. We must raise IndexError, not panic.
    alg = milnor(2)
    gap = algebra.FDModuleBuilder(alg, "g", [1, 0, 1]).build()
    t = algebra.TensorModule(gap, make_c2(alg))
    t.compute_basis(4)
    assert gap.dimension(1) == 0
    with pytest.raises(IndexError):
        t.offset(2, 1)
    # A non-empty block still returns the correct offset. The block structure in
    # total degree 2 lays out left_degree 0 (dim 1) then left_degree 2 (dim 1),
    # each tensored against the right factor's degree.
    assert t.offset(2, 0) == 0
    assert t.offset(2, 2) == t.dimension(2) - 1


def test_tensor_module_act_by_element_known_result():
    alg = milnor(2)
    t = algebra.TensorModule(make_c2(alg), make_c2(alg))
    t.compute_basis(4)
    # Sq1 as an algebra element in degree 1 (the unique basis element).
    op = fp.FpVector(2, alg.dimension(1))
    op[0] = 1
    # input = x0 (x) x0 (the single basis element in degree 0).
    inp = fp.FpVector(2, t.dimension(0))
    inp[0] = 1
    res = fp.FpVector(2, t.dimension(1))
    t.act_by_element(res, 1, 1, op, 0, inp)
    # Sq1 . (x0 (x) x0) = x1 (x) x0 + x0 (x) x1.
    assert res[0] == 1
    assert res[1] == 1
    assert sum(res) == 2


def test_tensor_module_act_by_element_length_mismatch_raises():
    alg = milnor(2)
    t = algebra.TensorModule(make_c2(alg), make_c2(alg))
    t.compute_basis(4)
    op = fp.FpVector(2, alg.dimension(1))
    op[0] = 1
    # Input vector of the wrong length for degree 0 (dimension 1).
    bad_input = fp.FpVector(2, 5)
    res = fp.FpVector(2, t.dimension(1))
    with pytest.raises(ValueError):
        t.act_by_element(res, 1, 1, op, 0, bad_input)


def test_suspension_module_act_by_element_prime_mismatch_raises():
    alg = milnor(2)
    s = algebra.SuspensionModule(make_c2(alg), 3)
    s.compute_basis(8)
    # Operation vector at the wrong prime.
    op = fp.FpVector(3, alg.dimension(1))
    inp = fp.FpVector(2, s.dimension(3))
    inp[0] = 1
    res = fp.FpVector(2, s.dimension(4))
    with pytest.raises(ValueError):
        s.act_by_element(res, 1, 1, op, 3, inp)


def test_tensor_module_odd_prime_action_runs():
    # p = 3 exercises the coproduct sign path. The Milnor algebra panics with
    # "Coproduct at odd primes not supported", so we use the Adem (generic)
    # algebra, whose coproduct is defined at odd primes. We assert the action
    # runs without panic and dimensions are consistent rather than pinning a
    # hand value.
    alg = adem(3)
    # A two-cell module with a degree-1 generator carrying a Bockstein action.
    m = algebra.FDModuleBuilder(alg, "M", [1, 1])
    m.set_action(1, 0, 0, 0, [1])  # beta . x0 = x1
    m = m.build()
    t = algebra.TensorModule(m, m)
    t.compute_basis(6)
    # Dimensions: convolution of [1, 1] with [1, 1] = [1, 2, 1].
    assert t.dimension(0) == 1
    assert t.dimension(1) == 2
    assert t.dimension(2) == 1
    res = fp.FpVector(3, t.dimension(1))
    t.act_on_basis(res, 1, 1, 0, 0, 0)
    assert len(res) == t.dimension(1)


def test_derived_modules_string_and_is_unit():
    alg = milnor(2)
    t = algebra.TensorModule(make_c2(alg), make_c2(alg))
    t.compute_basis(4)
    s = algebra.SuspensionModule(make_c2(alg), 3)
    s.compute_basis(8)
    z = algebra.ZeroModule(alg, 0)
    rp = algebra.RealProjectiveSpace(adem(2), 1, 4)
    rp.compute_basis(6)
    # basis_element_to_string success paths.
    assert isinstance(t.basis_element_to_string(0, 0), str)
    assert isinstance(s.basis_element_to_string(3, 0), str)
    assert isinstance(rp.basis_element_to_string(1, 0), str)
    # is_unit is callable and returns a bool on each derived class.
    assert isinstance(t.is_unit(), bool)
    assert isinstance(s.is_unit(), bool)
    assert isinstance(z.is_unit(), bool)
    assert isinstance(rp.is_unit(), bool)


def test_tensor_module_prime_mismatch_raises():
    left = make_c2(milnor(3))
    right = make_c2(milnor(2))
    with pytest.raises(ValueError):
        algebra.TensorModule(left, right)


def test_tensor_module_distinct_algebra_raises():
    # Same prime but two distinct algebra objects are incompatible.
    left = make_c2(milnor(2))
    right = make_c2(milnor(2))
    with pytest.raises(ValueError):
        algebra.TensorModule(left, right)


def test_tensor_module_into_steenrod_module_roundtrip():
    alg = milnor(2)
    t = algebra.TensorModule(make_c2(alg), make_c2(alg))
    t.compute_basis(4)
    boxed = t.into_steenrod_module()
    assert boxed.prime() == t.prime()
    assert boxed.dimension(1) == t.dimension(1)
    assert boxed.total_dimension() == t.total_dimension()


def test_tensor_module_out_of_range_no_panic():
    alg = milnor(2)
    t = algebra.TensorModule(make_c2(alg), make_c2(alg))
    assert t.dimension(-5) == 0
    assert t.dimension(100) == 0
    with pytest.raises(IndexError):
        t.basis_element_to_string(100, 0)
    with pytest.raises(IndexError):
        t.basis_element_to_string(-1, 0)


# --- SuspensionModule -----------------------------------------------------


def test_suspension_module_shifts_degrees():
    alg = milnor(2)
    s = algebra.SuspensionModule(make_c2(alg), 3)
    s.compute_basis(8)
    assert s.shift() == 3
    assert isinstance(s.prime(), int)
    assert s.prime() == 2
    assert s.min_degree() == 3
    assert s.dimension(0) == 0
    assert s.dimension(3) == 1
    assert s.dimension(4) == 1
    assert s.dimension(5) == 0
    assert s.max_degree() == 4


def test_suspension_module_preserves_action():
    alg = milnor(2)
    s = algebra.SuspensionModule(make_c2(alg), 3)
    s.compute_basis(8)
    # Sq1 . (shifted x0, in degree 3) = shifted x1 (degree 4).
    res = fp.FpVector(2, s.dimension(4))
    s.act_on_basis(res, 1, 1, 0, 3, 0)
    assert res[0] == 1


def test_suspension_module_into_steenrod_module_roundtrip():
    alg = milnor(2)
    s = algebra.SuspensionModule(make_c2(alg), 3)
    s.compute_basis(8)
    boxed = s.into_steenrod_module()
    assert boxed.min_degree() == 3
    assert boxed.dimension(3) == 1
    assert boxed.dimension(4) == 1


def test_suspension_module_negative_shift():
    alg = milnor(2)
    s = algebra.SuspensionModule(make_c2(alg), -2)
    s.compute_basis(4)
    assert s.shift() == -2
    assert s.min_degree() == -2
    assert s.dimension(-2) == 1
    assert s.dimension(-1) == 1


# --- ZeroModule -----------------------------------------------------------


def test_zero_module_is_empty():
    z = algebra.ZeroModule(milnor(2), 0)
    z.compute_basis(8)
    assert isinstance(z.prime(), int)
    assert z.prime() == 2
    assert z.min_degree() == 0
    for d in range(-2, 9):
        assert z.dimension(d) == 0
    assert z.total_dimension() == 0


def test_zero_module_into_steenrod_module_roundtrip():
    z = algebra.ZeroModule(milnor(2), 0)
    boxed = z.into_steenrod_module()
    assert boxed.dimension(0) == 0
    assert boxed.total_dimension() == 0


def test_zero_module_default_min_degree():
    z = algebra.ZeroModule(milnor(2))
    assert z.min_degree() == 0


# --- RealProjectiveSpace --------------------------------------------------


def test_rp_dimensions():
    rp = algebra.RealProjectiveSpace(adem(2), 1, 4)
    rp.compute_basis(6)
    assert isinstance(rp.prime(), int)
    assert rp.prime() == 2
    assert rp.min_degree() == 1
    assert rp.min == 1
    assert rp.max == 4
    assert rp.clear_bottom is False
    for d in range(1, 5):
        assert rp.dimension(d) == 1
    assert rp.dimension(0) == 0
    assert rp.dimension(5) == 0
    assert rp.total_dimension() == 4


def test_rp_action():
    rp = algebra.RealProjectiveSpace(adem(2), 1, 4)
    rp.compute_basis(6)
    # Sq1 . x^1 = x^2 (binomial(1, 1) = 1 mod 2).
    res = fp.FpVector(2, rp.dimension(2))
    rp.act_on_basis(res, 1, 1, 0, 1, 0)
    assert res[0] == 1


def test_rp_infinite():
    # max = None gives RP_min^oo.
    rp = algebra.RealProjectiveSpace(adem(2), 1)
    rp.compute_basis(6)
    assert rp.max is None
    for d in range(1, 7):
        assert rp.dimension(d) == 1


def test_rp_rejects_odd_prime():
    with pytest.raises(ValueError):
        algebra.RealProjectiveSpace(milnor(3), 1, 4)


def test_rp_rejects_bad_range():
    with pytest.raises(ValueError):
        algebra.RealProjectiveSpace(adem(2), 4, 1)


def test_rp_into_steenrod_module_roundtrip():
    rp = algebra.RealProjectiveSpace(adem(2), 1, 4)
    rp.compute_basis(6)
    boxed = rp.into_steenrod_module()
    assert boxed.min_degree() == 1
    assert boxed.dimension(2) == 1
