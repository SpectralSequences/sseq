import pytest

from ext import algebra, fp


def make(variant, p=2, degree=8):
    if variant == "adem":
        a = algebra.SteenrodAlgebra.adem(p)
    else:
        a = algebra.SteenrodAlgebra.milnor(p)
    a.compute_basis(degree)
    return a


VARIANTS = ["adem", "milnor"]


def test_construction_and_variant():
    adem = algebra.SteenrodAlgebra.adem(2)
    assert adem.prime() == 2
    assert adem.algebra_type() == algebra.AlgebraType.Adem

    milnor = algebra.SteenrodAlgebra.milnor(3)
    assert milnor.prime() == 3
    assert milnor.algebra_type() == algebra.AlgebraType.Milnor


def test_invalid_prime_raises():
    for bad in (4, 0, 1):
        with pytest.raises(ValueError):
            algebra.SteenrodAlgebra.adem(bad)
        with pytest.raises(ValueError):
            algebra.SteenrodAlgebra.milnor(bad)


def test_prime_is_plain_int():
    a = algebra.SteenrodAlgebra.milnor(2)
    p = a.prime()
    assert isinstance(p, int)
    assert p == 2


@pytest.mark.parametrize("variant", VARIANTS)
def test_compute_basis_and_dimension(variant):
    a = make(variant, 2, 8)
    assert a.dimension(0) == 1
    assert a.dimension(1) == 1
    assert a.dimension(2) == 1
    assert a.dimension(3) == 2
    # Negative degree is empty, not an error.
    assert a.dimension(-2) == 0


@pytest.mark.parametrize("variant", VARIANTS)
def test_multiply_basis_elements_known_results(variant):
    a = make(variant, 2, 8)

    # Sq1 * Sq1 = 0 in degree 2.
    v = fp.FpVector(2, a.dimension(2))
    a.multiply_basis_elements(v, 1, 1, 0, 1, 0)
    assert list(v) == [0]

    # Sq1 * Sq2 = Sq3 in degree 3 (a single basis term in either basis).
    deg3, sq3_idx = a.basis_element_from_string("Sq3")
    assert deg3 == 3
    v = fp.FpVector(2, a.dimension(3))
    a.multiply_basis_elements(v, 1, 1, 0, 2, 0)
    assert v[sq3_idx] == 1
    assert sum(v) == 1


@pytest.mark.parametrize("variant", VARIANTS)
def test_multiply_element_families_via_fpvector(variant):
    a = make(variant, 2, 8)

    # multiply_element_by_element using full FpVector inputs: Sq1 * Sq2 = Sq3.
    r = fp.FpVector.from_slice(2, [1])  # Sq1 (degree 1, dim 1)
    s = fp.FpVector.from_slice(2, [1])  # Sq2 (degree 2, dim 1)
    out = fp.FpVector(2, a.dimension(3))
    a.multiply_element_by_element(out, 1, 1, r, 2, s)
    _, sq3_idx = a.basis_element_from_string("Sq3")
    assert out[sq3_idx] == 1

    # multiply_basis_element_by_element with an FpVector element.
    s = fp.FpVector.from_slice(2, [1])
    out = fp.FpVector(2, a.dimension(3))
    a.multiply_basis_element_by_element(out, 1, 1, 0, 2, s)
    assert out[sq3_idx] == 1

    # multiply_element_by_basis_element.
    r = fp.FpVector.from_slice(2, [1])
    out = fp.FpVector(2, a.dimension(3))
    a.multiply_element_by_basis_element(out, 1, 1, r, 2, 0)
    assert out[sq3_idx] == 1


@pytest.mark.parametrize("variant", VARIANTS)
def test_multiply_accepts_fpslice_and_fpslicemut(variant):
    a = make(variant, 2, 8)

    s_vec = fp.FpVector.from_slice(2, [1])  # Sq2
    s_slice = s_vec.slice(0, 1)

    result_vec = fp.FpVector(2, a.dimension(3))
    result_slice = result_vec.slice_mut(0, a.dimension(3))
    a.multiply_basis_element_by_element(result_slice, 1, 1, 0, 2, s_slice)
    _, sq3_idx = a.basis_element_from_string("Sq3")
    assert result_vec[sq3_idx] == 1


@pytest.mark.parametrize("variant", VARIANTS)
def test_multiply_prime_and_length_errors(variant):
    a = make(variant, 2, 8)

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


@pytest.mark.parametrize("variant", VARIANTS)
def test_multiply_large_coeff_does_not_overflow(variant):
    a = make(variant, 2, 8)
    _, sq3_idx = a.basis_element_from_string("Sq3")

    big = fp.FpVector(2, a.dimension(3))
    a.multiply_basis_elements(big, 0xFFFFFFFF, 1, 0, 2, 0)  # odd coeff -> 1
    assert big[sq3_idx] == 1

    even = fp.FpVector(2, a.dimension(3))
    a.multiply_basis_elements(even, 0xFFFFFFFE, 1, 0, 2, 0)  # even coeff -> 0
    assert list(even) == [0, 0]


@pytest.mark.parametrize("variant", VARIANTS)
def test_basis_element_string_roundtrip(variant):
    a = make(variant, 2, 8)
    for d in range(9):
        for i in range(a.dimension(d)):
            s = a.basis_element_to_string(d, i)
            assert a.basis_element_from_string(s) == (d, i)

    with pytest.raises(ValueError):
        a.basis_element_from_string("not a valid element ###")


@pytest.mark.parametrize("variant", VARIANTS)
def test_basis_element_from_string_absent_names_raise(variant):
    # Parseable-but-absent names must raise ValueError, never panic across the
    # FFI boundary.
    a = make(variant, 2, 8)
    with pytest.raises(ValueError):
        a.basis_element_from_string("Sq0")


@pytest.mark.parametrize("variant", VARIANTS)
def test_basis_element_to_string_out_of_range(variant):
    a = make(variant, 2, 8)
    with pytest.raises(IndexError):
        a.basis_element_to_string(2, 99)
    with pytest.raises(IndexError):
        a.basis_element_to_string(-1, 0)


@pytest.mark.parametrize("variant", VARIANTS)
def test_decompose_basis_element_guards(variant):
    a = make(variant, 2, 8)
    # The degree-0 unit is indecomposable -> ValueError, not a panic.
    with pytest.raises(ValueError):
        a.decompose_basis_element(0, 0)
    # A non-generator decomposes into a list of triples.
    decomp = a.decompose_basis_element(3, 0)
    assert isinstance(decomp, list)
    assert all(len(t) == 3 for t in decomp)


@pytest.mark.parametrize("variant", VARIANTS)
def test_decompose_generator_raises_consistently(variant):
    # Both variants reject generators identically: Sq^2 / P(2) is the degree-2
    # generator (idx 0). Previously the Milnor variant returned a degenerate
    # self-term while Adem raised; the generators-based guard unifies them.
    a = make(variant, 2, 8)
    assert 0 in a.generators(2)
    with pytest.raises(ValueError):
        a.decompose_basis_element(2, 0)
    # A non-generator decomposable element still decomposes.
    assert isinstance(a.decompose_basis_element(3, 0), list)


def test_milnor_q0_decompose_raises():
    # Q_0 (degree 1, idx 0) at an odd prime used to underflow-panic through the
    # union (`prime().pow(i - 1)` with i == 0). It must raise ValueError.
    a = make("milnor", 3, 8)
    assert 0 in a.generators(1)
    with pytest.raises(ValueError):
        a.decompose_basis_element(1, 0)


@pytest.mark.parametrize("variant", VARIANTS)
def test_generated_algebra_surface(variant):
    a = make(variant, 2, 8)
    assert a.generators(2) == [0]
    assert isinstance(a.generator_to_string(2, 0), str)
    assert isinstance(a.generating_relations(4), list)
    assert a.generators(-1) == []
    assert a.generating_relations(-1) == []


@pytest.mark.parametrize("variant", VARIANTS)
def test_default_filtration_one_products(variant):
    a = make(variant, 2, 8)
    products = a.default_filtration_one_products()
    assert all(len(triple) == 3 for triple in products)
    assert all(isinstance(name, str) for name, _, _ in products)


@pytest.mark.parametrize("variant", VARIANTS)
def test_element_to_string(variant):
    a = make(variant, 2, 8)
    v = fp.FpVector.from_slice(2, [1, 1])  # degree 3 has dim 2
    text = a.element_to_string(3, v)
    assert isinstance(text, str)

    with pytest.raises(ValueError):
        a.element_to_string(3, fp.FpVector(2, 5))


def test_coproduct_p2():
    a = make("adem", 2, 8)
    assert a.coproduct(2, 0) == [(0, 0, 2, 0), (1, 0, 1, 0), (2, 0, 0, 0)]
    assert a.decompose(2, 0) == [(2, 0)]


def test_coproduct_milnor_odd_prime_raises():
    a = make("milnor", 3, 8)
    with pytest.raises(ValueError):
        a.coproduct(0, 0)


def test_from_json_constructs_known_algebra():
    spec = {"p": 2}
    adem = algebra.SteenrodAlgebra.from_json(spec, algebra.AlgebraType.Adem, False)
    assert adem.prime() == 2
    assert adem.algebra_type() == algebra.AlgebraType.Adem

    milnor = algebra.SteenrodAlgebra.from_json(spec, algebra.AlgebraType.Milnor, False)
    assert milnor.prime() == 2
    assert milnor.algebra_type() == algebra.AlgebraType.Milnor

    # from_json defaults unstable to False.
    again = algebra.SteenrodAlgebra.from_json(spec, algebra.AlgebraType.Adem)
    assert again.prime() == 2

    # The "algebra" allow-list is respected by the underlying constructor; a
    # spec listing only milnor falls back to milnor even if adem is requested.
    listed = {"p": 2, "algebra": ["milnor"]}
    fallback = algebra.SteenrodAlgebra.from_json(
        listed, algebra.AlgebraType.Adem, False
    )
    assert fallback.algebra_type() == algebra.AlgebraType.Milnor


def test_from_json_accepts_string_algebra_type():
    spec = {"p": 2}

    # Plain strings are accepted, case-insensitively, as an alternative to the
    # AlgebraType enum.
    for s in ("adem", "ADEM", "Adem"):
        a = algebra.SteenrodAlgebra.from_json(spec, s)
        assert a.algebra_type() == algebra.AlgebraType.Adem

    for s in ("milnor", "MILNOR", "Milnor"):
        m = algebra.SteenrodAlgebra.from_json(spec, s)
        assert m.algebra_type() == algebra.AlgebraType.Milnor

    # The enum still works exactly as before.
    enum_adem = algebra.SteenrodAlgebra.from_json(spec, algebra.AlgebraType.Adem)
    assert enum_adem.algebra_type() == algebra.AlgebraType.Adem


def test_from_json_rejects_invalid_algebra_string():
    with pytest.raises(ValueError):
        algebra.SteenrodAlgebra.from_json({"p": 2}, "foo")


def test_from_json_rejects_non_string_non_enum_algebra_type():
    with pytest.raises(TypeError):
        algebra.SteenrodAlgebra.from_json({"p": 2}, 42)


def test_from_json_bad_prime_raises():
    # Upstream returns an opaque anyhow::Error for a bad prime, which the
    # binding maps to RuntimeError (documented behavior); the Python value
    # itself converts fine, so this is not a ValueError.
    with pytest.raises(RuntimeError):
        algebra.SteenrodAlgebra.from_json({"p": 4}, algebra.AlgebraType.Adem, False)


def test_from_json_rejects_non_dict():
    # A list converts to JSON fine, but upstream rejects the shape -> RuntimeError.
    with pytest.raises(RuntimeError):
        algebra.SteenrodAlgebra.from_json([1, 2, 3], algebra.AlgebraType.Adem, False)


def test_from_json_large_int_in_spec():
    # py_to_json now accepts ints in (i64::MAX, u64::MAX] via the u64 path
    # rather than raising OverflowError. Such a value is not a valid prime, so
    # upstream rejects it with a RuntimeError (the int conversion succeeds).
    big = 2**63 + 1  # > i64::MAX, <= u64::MAX
    with pytest.raises(RuntimeError):
        algebra.SteenrodAlgebra.from_json({"p": big}, algebra.AlgebraType.Adem, False)

    # An int outside [i64::MIN, u64::MAX] is rejected by py_to_json itself with
    # a ValueError (taxonomy), not OverflowError.
    too_big = 2**64
    with pytest.raises(ValueError):
        algebra.SteenrodAlgebra.from_json(
            {"p": too_big}, algebra.AlgebraType.Adem, False
        )


def test_repr():
    a = algebra.SteenrodAlgebra.milnor(2)
    assert isinstance(repr(a), str)
    assert repr(a)
