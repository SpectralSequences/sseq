import pytest

from ext import algebra, fp


def make_algebra(p=2, degree=8):
    a = algebra.MilnorAlgebra(p)
    a.compute_basis(degree)
    return a


def test_construction_valid_and_invalid_prime():
    a = algebra.MilnorAlgebra(2)
    assert a.prime() == 2
    assert algebra.MilnorAlgebra(3).prime() == 3

    # A non-prime must raise ValueError, never panic.
    with pytest.raises(ValueError):
        algebra.MilnorAlgebra(4)
    with pytest.raises(ValueError):
        algebra.MilnorAlgebra(0)
    with pytest.raises(ValueError):
        algebra.MilnorAlgebra(1)


def test_prime_is_plain_int():
    a = algebra.MilnorAlgebra(2)
    p = a.prime()
    assert isinstance(p, int)
    assert p == 2


def test_compute_basis_and_dimension():
    a = make_algebra(2, 8)
    assert a.dimension(0) == 1
    assert a.dimension(1) == 1
    assert a.dimension(2) == 1
    assert a.dimension(3) == 2
    assert a.dimension(4) == 2
    # Negative degree is empty, not an error.
    assert a.dimension(-2) == 0


def test_generic_q_and_profile():
    a = algebra.MilnorAlgebra(2)
    assert a.generic() is False
    assert a.q() == 1

    a3 = algebra.MilnorAlgebra(3)
    assert a3.generic() is True
    assert a3.q() == 4

    profile = a.profile()
    assert isinstance(profile, algebra.MilnorProfile)
    assert profile.is_trivial()
    assert profile.truncated is False


def test_multiply_basis_elements_known_results():
    a = make_algebra(2, 8)

    # Sq^1 * Sq^1 = 0 in degree 2.
    v = fp.FpVector(2, a.dimension(2))
    a.multiply_basis_elements(v, 1, 1, 0, 1, 0)
    assert list(v) == [0]

    # Sq^2 * Sq^2 = P(1, 1) in degree 4.
    v = fp.FpVector(2, a.dimension(4))
    a.multiply_basis_elements(v, 1, 2, 0, 2, 0)
    assert list(v) == [0, 1]

    # Sq^2 * Sq^1 in degree 3.
    v = fp.FpVector(2, a.dimension(3))
    a.multiply_basis_elements(v, 1, 2, 0, 1, 0)
    assert list(v) == [1, 1]


def test_multiply_basis_elements_accumulates_into_result():
    a = make_algebra(2, 8)
    # multiply_* adds into the result, so doing Sq^2 * Sq^2 = [0, 1] twice at
    # p = 2 cancels to zero.
    v = fp.FpVector(2, a.dimension(4))
    a.multiply_basis_elements(v, 1, 2, 0, 2, 0)
    assert list(v) == [0, 1]
    a.multiply_basis_elements(v, 1, 2, 0, 2, 0)
    assert list(v) == [0, 0]


def test_multiply_element_families_via_fpvector():
    a = make_algebra(2, 8)

    # multiply_element_by_element using full FpVector inputs.
    r = fp.FpVector.from_slice(2, [1])  # Sq^2 (degree 2, dim 1)
    s = fp.FpVector.from_slice(2, [1])  # Sq^2
    out = fp.FpVector(2, a.dimension(4))
    a.multiply_element_by_element(out, 1, 2, r, 2, s)
    assert list(out) == [0, 1]

    # multiply_basis_element_by_element with an FpVector element.
    s = fp.FpVector.from_slice(2, [1])  # Sq^1
    out = fp.FpVector(2, a.dimension(3))
    a.multiply_basis_element_by_element(out, 1, 2, 0, 1, s)
    assert list(out) == [1, 1]


def test_multiply_accepts_fpslice_and_fpslicemut():
    a = make_algebra(2, 8)

    # Pass an FpSlice as an input element, and an FpSliceMut as the result.
    s_vec = fp.FpVector.from_slice(2, [1])
    s_slice = s_vec.slice(0, 1)

    result_vec = fp.FpVector(2, a.dimension(3))
    result_slice = result_vec.slice_mut(0, a.dimension(3))
    a.multiply_basis_element_by_element(result_slice, 1, 2, 0, 1, s_slice)
    assert list(result_vec) == [1, 1]


def test_multiply_milnor_basis_elements():
    a = make_algebra(2, 8)
    m1 = a.basis_element_from_index(2, 0)
    m2 = a.basis_element_from_index(2, 0)
    out = fp.FpVector(2, a.dimension(4))
    a.multiply(out, 1, m1, m2)
    assert list(out) == [0, 1]


def test_multiply_prime_and_length_errors():
    a = make_algebra(2, 8)

    # Prime mismatch on the result.
    wrong_prime = fp.FpVector(3, a.dimension(2))
    with pytest.raises(ValueError):
        a.multiply_basis_elements(wrong_prime, 1, 1, 0, 1, 0)

    # Result too short.
    short = fp.FpVector(2, 0)
    with pytest.raises(ValueError):
        a.multiply_basis_elements(short, 1, 2, 0, 2, 0)

    # Out-of-range basis index.
    ok = fp.FpVector(2, a.dimension(2))
    with pytest.raises(IndexError):
        a.multiply_basis_elements(ok, 1, 1, 99, 1, 0)


def test_basis_element_to_from_string_roundtrip():
    a = make_algebra(2, 6)
    for d in range(7):
        for i in range(a.dimension(d)):
            s = a.basis_element_to_string(d, i)
            assert a.basis_element_from_string(s) == (d, i)

    with pytest.raises(ValueError):
        a.basis_element_from_string("not a valid element ###")


def test_element_to_string():
    a = make_algebra(2, 6)
    v = fp.FpVector.from_slice(2, [1, 1])  # P(3) + P(0, 1) in degree 3
    text = a.element_to_string(3, v)
    assert "P(3)" in text
    assert "P(0, 1)" in text

    # Length mismatch raises.
    with pytest.raises(ValueError):
        a.element_to_string(3, fp.FpVector(2, 5))


def test_basis_element_index_roundtrip():
    a = make_algebra(2, 6)
    for d in range(7):
        for i in range(a.dimension(d)):
            elt = a.basis_element_from_index(d, i)
            assert isinstance(elt, algebra.MilnorBasisElement)
            assert a.basis_element_to_index(elt) == i
            assert a.try_basis_element_to_index(elt) == i

    with pytest.raises(IndexError):
        a.basis_element_from_index(2, 99)


def test_basis_element_to_index_not_found():
    a = make_algebra(2, 6)
    bogus = algebra.MilnorBasisElement([99], q_part=0, degree=2)
    assert a.try_basis_element_to_index(bogus) is None
    with pytest.raises(ValueError):
        a.basis_element_to_index(bogus)


def test_milnor_basis_element_fields():
    elt = algebra.MilnorBasisElement([2, 1], q_part=3, degree=10)
    assert elt.p_part == [2, 1]
    assert elt.q_part == 3
    assert elt.degree == 10

    elt.p_part = [1]
    elt.q_part = 0
    assert elt.p_part == [1]
    assert elt.q_part == 0

    # compute_degree fills in the degree from the parts.
    e = algebra.MilnorBasisElement([1])
    e.compute_degree(2)
    assert e.degree == 1

    assert algebra.MilnorBasisElement([1]) == algebra.MilnorBasisElement([1])


def test_milnor_profile_fields_and_methods():
    profile = algebra.MilnorProfile(truncated=True, q_part=0b1111, p_part=[3, 2, 1])
    assert profile.truncated is True
    assert profile.q_part == 0b1111
    assert profile.p_part == [3, 2, 1]
    assert profile.get_p_part(0) == 3
    assert profile.is_valid()

    default = algebra.MilnorProfile()
    assert default.is_trivial()
    assert default.q_part == 0xFFFFFFFF


def test_new_with_profile_valid_and_invalid():
    profile = algebra.MilnorProfile(truncated=True, q_part=0xFFFFFFFF, p_part=[2, 1])
    a = algebra.MilnorAlgebra.new_with_profile(2, profile)
    a.compute_basis(8)
    assert a.prime() == 2

    # An invalid profile raises ValueError instead of panicking.
    bad = algebra.MilnorProfile(truncated=True, q_part=0xFFFFFFFF, p_part=[1, 5])
    if not bad.is_valid():
        with pytest.raises(ValueError):
            algebra.MilnorAlgebra.new_with_profile(2, bad)


def test_generated_algebra_surface():
    a = make_algebra(2, 8)
    assert a.generators(2) == [0]
    assert isinstance(a.generator_to_string(2, 0), str)
    assert isinstance(a.decompose_basis_element(4, 1), list)
    assert isinstance(a.generating_relations(4), list)
    assert a.decompose(2, 0) == [(2, 0)]


def test_default_filtration_one_products():
    a = make_algebra(2, 8)
    products = a.default_filtration_one_products()
    assert all(len(triple) == 3 for triple in products)
    assert all(isinstance(name, str) for name, _, _ in products)


def test_coproduct_p2_and_odd_prime_error():
    a = make_algebra(2, 8)
    assert a.coproduct(2, 0) == [(0, 0, 2, 0), (1, 0, 1, 0), (2, 0, 0, 0)]

    a3 = make_algebra(3, 32)
    with pytest.raises(ValueError):
        a3.coproduct(4, 0)


def test_beps_pn_and_ppart_table():
    a = make_algebra(2, 8)
    assert a.beps_pn(0, 1) == (1, 0)
    assert a.ppart_table(0) == [[]]
    with pytest.raises(IndexError):
        a.ppart_table(-1)
