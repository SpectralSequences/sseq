import pytest

from ext import algebra, fp


def make_algebra(p=2, degree=8):
    a = algebra.AdemAlgebra(p)
    a.compute_basis(degree)
    return a


def test_construction_valid_and_invalid_prime():
    a = algebra.AdemAlgebra(2)
    assert a.prime == 2
    assert algebra.AdemAlgebra(3).prime == 3

    # A non-prime must raise ValueError, never panic.
    for bad in (4, 0, 1):
        with pytest.raises(ValueError):
            algebra.AdemAlgebra(bad)


def test_prime_is_plain_int():
    a = algebra.AdemAlgebra(2)
    p = a.prime
    assert isinstance(p, int)
    assert p == 2


def test_generic_and_q():
    a = algebra.AdemAlgebra(2)
    assert a.generic() is False
    assert a.q() == 1

    a3 = algebra.AdemAlgebra(3)
    assert a3.generic() is True
    assert a3.q() == 4


def test_compute_basis_and_dimension():
    a = make_algebra(2, 8)
    assert a.dimension(0) == 1
    assert a.dimension(1) == 1
    assert a.dimension(2) == 1
    assert a.dimension(3) == 2  # Sq3, Sq2 Sq1
    # Negative degree is empty, not an error.
    assert a.dimension(-2) == 0


def test_multiply_basis_elements_known_results():
    a = make_algebra(2, 8)

    # Sq1 * Sq1 = 0 in degree 2.
    v = fp.FpVector(2, a.dimension(2))
    a.multiply_basis_elements(v, 1, 1, 0, 1, 0)
    assert list(v) == [0]

    # Sq1 * Sq2 = Sq3 in degree 3 (a single admissible basis term).
    deg3, sq3_idx = a.basis_element_from_string("Sq3")
    assert deg3 == 3
    v = fp.FpVector(2, a.dimension(3))
    a.multiply_basis_elements(v, 1, 1, 0, 2, 0)
    assert v[sq3_idx] == 1
    assert sum(v) == 1

    # Sq2 * Sq2 = Sq3 Sq1 (Adem: Sq2 Sq2 = Sq3 Sq1).
    _, sq3sq1_idx = a.basis_element_from_string("Sq3 Sq1")
    v = fp.FpVector(2, a.dimension(4))
    a.multiply_basis_elements(v, 1, 2, 0, 2, 0)
    assert v[sq3sq1_idx] == 1
    assert sum(v) == 1


def test_multiply_accumulates_into_result():
    a = make_algebra(2, 8)
    # multiply_* adds into the result; doing Sq1 * Sq2 = Sq3 twice at p = 2
    # cancels to zero.
    v = fp.FpVector(2, a.dimension(3))
    a.multiply_basis_elements(v, 1, 1, 0, 2, 0)
    assert sum(v) == 1
    a.multiply_basis_elements(v, 1, 1, 0, 2, 0)
    assert list(v) == [0, 0]


def test_multiply_element_families_via_fpvector():
    a = make_algebra(2, 8)

    # multiply_element_by_element using full FpVector inputs: Sq2 * Sq2.
    r = fp.FpVector.from_slice(2, [1])  # Sq2 (degree 2, dim 1)
    s = fp.FpVector.from_slice(2, [1])
    out = fp.FpVector(2, a.dimension(4))
    a.multiply_element_by_element(out, 1, 2, r, 2, s)
    _, sq3sq1_idx = a.basis_element_from_string("Sq3 Sq1")
    assert out[sq3sq1_idx] == 1

    # multiply_basis_element_by_element with an FpVector element: Sq1 * Sq2.
    s = fp.FpVector.from_slice(2, [1])  # Sq2
    out = fp.FpVector(2, a.dimension(3))
    a.multiply_basis_element_by_element(out, 1, 1, 0, 2, s)
    _, sq3_idx = a.basis_element_from_string("Sq3")
    assert out[sq3_idx] == 1

    # multiply_element_by_basis_element: Sq1 (element) * Sq2.
    r = fp.FpVector.from_slice(2, [1])  # Sq1
    out = fp.FpVector(2, a.dimension(3))
    a.multiply_element_by_basis_element(out, 1, 1, r, 2, 0)
    assert out[sq3_idx] == 1


def test_multiply_accepts_fpslice_and_fpslicemut():
    a = make_algebra(2, 8)

    s_vec = fp.FpVector.from_slice(2, [1])  # Sq2
    s_slice = s_vec.slice(0, 1)

    result_vec = fp.FpVector(2, a.dimension(3))
    result_slice = result_vec.slice_mut(0, a.dimension(3))
    a.multiply_basis_element_by_element(result_slice, 1, 1, 0, 2, s_slice)
    _, sq3_idx = a.basis_element_from_string("Sq3")
    assert result_vec[sq3_idx] == 1


def test_multiply_input_target_aliasing_raises_runtimeerror():
    # Passing the SAME bare FpVector as both an input element and the mutable
    # result target is an aliasing conflict -> RuntimeError, NOT ValueError.
    a = make_algebra(2, 8)
    v = fp.FpVector.from_slice(2, [1])  # Sq1 element / result alias
    with pytest.raises(RuntimeError):
        a.multiply_element_by_basis_element(v, 1, 1, v, 2, 0)
    with pytest.raises(Exception) as excinfo:
        a.multiply_element_by_basis_element(v, 1, 1, v, 2, 0)
    assert not isinstance(excinfo.value, ValueError)

    # Dual-input variant: same object as both r and s and result.
    with pytest.raises(RuntimeError):
        a.multiply_element_by_element(v, 1, 1, v, 1, v)


def test_multiply_wrong_type_is_valueerror():
    # A genuine wrong-type argument still raises ValueError.
    a = make_algebra(2, 8)
    s = fp.FpVector.from_slice(2, [1])
    out = fp.FpVector(2, a.dimension(3))
    with pytest.raises(ValueError):
        a.multiply_element_by_basis_element(123, 1, 1, s, 2, 0)
    with pytest.raises(ValueError):
        a.multiply_element_by_basis_element(out, 1, 1, 123, 2, 0)


def test_multiply_distinct_objects_regression():
    # Distinct objects still produce the known value.
    a = make_algebra(2, 8)
    r = fp.FpVector.from_slice(2, [1])  # Sq1
    out = fp.FpVector(2, a.dimension(3))
    a.multiply_element_by_basis_element(out, 1, 1, r, 2, 0)
    _, sq3_idx = a.basis_element_from_string("Sq3")
    assert out[sq3_idx] == 1


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


def test_multiply_large_coeff_does_not_overflow():
    # Upstream computes `coeff * value` before reducing mod p, overflowing for
    # large coeff. The binding reduces coeff mod p first.
    a = make_algebra(2, 8)

    _, sq3_idx = a.basis_element_from_string("Sq3")

    big = fp.FpVector(2, a.dimension(3))
    a.multiply_basis_elements(big, 0xFFFFFFFF, 1, 0, 2, 0)  # odd coeff -> 1
    assert big[sq3_idx] == 1

    even = fp.FpVector(2, a.dimension(3))
    a.multiply_basis_elements(even, 0xFFFFFFFE, 1, 0, 2, 0)  # even coeff -> 0
    assert list(even) == [0, 0]

    # Odd prime: coeff >= p reduces correctly (7 % 3 == 1).
    a3 = make_algebra(3, 32)
    base = fp.FpVector(3, a3.dimension(8))
    a3.multiply_basis_elements(base, 1, 4, 0, 4, 0)
    scaled = fp.FpVector(3, a3.dimension(8))
    a3.multiply_basis_elements(scaled, 7, 4, 0, 4, 0)
    assert list(scaled) == list(base)


def test_basis_element_to_from_string_roundtrip():
    a = make_algebra(2, 8)
    for d in range(9):
        for i in range(a.dimension(d)):
            s = a.basis_element_to_string(d, i)
            assert a.basis_element_from_string(s) == (d, i)

    with pytest.raises(ValueError):
        a.basis_element_from_string("not a valid element ###")


def test_basis_element_from_string_absent_names_raise():
    # Parseable-but-absent / inadmissible names used to panic across the PyO3
    # boundary (upstream `basis_element_to_index`). They must now raise a
    # normal ValueError, never a PanicException.
    a = make_algebra(2, 8)
    for name in ("Sq0", "Sq1 Sq1"):
        with pytest.raises(ValueError):
            a.basis_element_from_string(name)

    # Valid names still round-trip correctly.
    for d in range(7):
        for i in range(a.dimension(d)):
            s = a.basis_element_to_string(d, i)
            assert a.basis_element_from_string(s) == (d, i)


def test_basis_element_to_string_out_of_range():
    a = make_algebra(2, 8)
    with pytest.raises(IndexError):
        a.basis_element_to_string(2, 99)
    with pytest.raises(IndexError):
        a.basis_element_to_string(-1, 0)


def test_basis_element_index_roundtrip():
    a = make_algebra(2, 8)
    for d in range(9):
        for i in range(a.dimension(d)):
            elt = a.basis_element_from_index(d, i)
            assert isinstance(elt, algebra.AdemBasisElement)
            assert a.basis_element_to_index(elt) == i
            assert a.try_basis_element_to_index(elt) == i

    with pytest.raises(IndexError):
        a.basis_element_from_index(2, 99)


def test_basis_element_to_index_not_found():
    a = make_algebra(2, 8)
    # Sq1 Sq2 (ps=[1, 2]) is inadmissible, so it is not a basis element.
    bogus = algebra.AdemBasisElement([1, 2], degree=3)
    assert a.try_basis_element_to_index(bogus) is None
    with pytest.raises(ValueError):
        a.basis_element_to_index(bogus)


def test_decompose_basis_element_and_guards():
    a = make_algebra(2, 8)

    # The degree-0 unit and generators are indecomposable -> ValueError.
    with pytest.raises(ValueError):
        a.decompose_basis_element(0, 0)
    # Sq2 is a generator in degree 2.
    assert a.generators(2) == [0]
    with pytest.raises(ValueError):
        a.decompose_basis_element(2, 0)

    # A non-generator decomposes into a list of triples.
    # Sq2 Sq1 (degree 3, the non-admissible-built element) is decomposable.
    decomp = a.decompose_basis_element(3, 1)
    assert isinstance(decomp, list)
    assert all(len(t) == 3 for t in decomp)


def test_generated_algebra_surface():
    a = make_algebra(2, 8)
    assert a.generators(2) == [0]
    assert isinstance(a.generator_to_string(2, 0), str)
    assert isinstance(a.generating_relations(4), list)
    # Negative degrees are empty, not errors.
    assert a.generators(-1) == []
    assert a.generating_relations(-1) == []


def test_default_filtration_one_products():
    a = make_algebra(2, 8)
    products = a.default_filtration_one_products()
    assert all(len(triple) == 3 for triple in products)
    assert all(isinstance(name, str) for name, _, _ in products)


def test_coproduct_and_decompose():
    a = make_algebra(2, 8)
    # p = 2 coproduct of Sq2 (degree 2, idx 0).
    assert a.coproduct(2, 0) == [(0, 0, 2, 0), (1, 0, 1, 0), (2, 0, 0, 0)]

    # bialgebra decompose: Sq2 decomposes (at p=2) into its Ps reversed.
    assert a.decompose(2, 0) == [(2, 0)]
    assert isinstance(a.decompose(3, 0), list)


def test_coproduct_generic_non_divisible_raises():
    a3 = make_algebra(3, 32)
    # Degree 1 is the bockstein, handled specially. A degree not divisible by
    # q = 4 (other than 1) would trip an upstream assertion -> ValueError.
    # Find a nonzero-dimension degree that is not 1 and not divisible by 4.
    raised = False
    for d in range(2, 16):
        if d % 4 != 0 and a3.dimension(d) > 0:
            try:
                a3.coproduct(d, 0)
            except ValueError:
                raised = True
                break
    assert raised


def test_beps_pn():
    a = make_algebra(2, 8)
    # P^1 = Sq1 lives in degree 1.
    assert a.beps_pn(0, 1) == (1, 0)
    # x == 0 short-circuits.
    assert a.beps_pn(1, 0) == (1, 0)
    assert a.beps_pn(0, 0) == (0, 0)


def test_element_to_string():
    a = make_algebra(2, 8)
    v = fp.FpVector.from_slice(2, [1, 1])  # degree 3 has dim 2
    text = a.element_to_string(3, v)
    assert isinstance(text, str)

    # Length mismatch raises.
    with pytest.raises(ValueError):
        a.element_to_string(3, fp.FpVector(2, 5))


def test_adem_basis_element_fields():
    elt = algebra.AdemBasisElement([2, 1], bocksteins=1, degree=10, p_or_sq=True)
    assert elt.ps == [2, 1]
    assert elt.bocksteins == 1
    assert elt.degree == 10
    assert elt.p_or_sq is True

    elt.ps = [3]
    elt.bocksteins = 0
    elt.degree = 3
    elt.p_or_sq = False
    assert elt.ps == [3]
    assert elt.bocksteins == 0
    assert elt.degree == 3
    assert elt.p_or_sq is False

    # Equality compares ps and bocksteins (matching upstream).
    assert algebra.AdemBasisElement([1]) == algebra.AdemBasisElement([1])
    assert algebra.AdemBasisElement([1]) != algebra.AdemBasisElement([2])


def test_por_bockstein_variants():
    p = algebra.PorBockstein.P(3)
    b = algebra.PorBockstein.Bockstein(True)
    assert isinstance(p, algebra.PorBockstein)
    assert isinstance(b, algebra.PorBockstein)

    # iter_filtered exposes the decomposition as PorBockstein values.
    a = make_algebra(3, 32)
    _, idx = a.basis_element_from_string("b")
    elt = a.basis_element_from_index(1, idx)
    pieces = elt.iter_filtered()
    assert all(isinstance(x, algebra.PorBockstein) for x in pieces)
    assert any(isinstance(x, algebra.PorBockstein.Bockstein) for x in pieces)
