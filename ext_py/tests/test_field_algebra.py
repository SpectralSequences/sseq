"""Tests for `algebra.Field`: the ground field F_p viewed as the trivial
1-dimensional algebra (concentrated in degree 0), and for `HomModule.algebra`,
which returns that ground-field algebra.

Note: `algebra.Field` (this trivial *algebra*) is distinct from `fp.Fp`
(the *field type* / scalars). See the `algebra.Field` docstring.
"""

import pytest

from ext import algebra, fp


def test_construction_valid_and_invalid_prime():
    assert algebra.Field(2).prime == 2
    assert algebra.Field(3).prime == 3

    # A non-prime must raise ValueError, never panic.
    for bad in (4, 0, 1):
        with pytest.raises(ValueError):
            algebra.Field(bad)


def test_prime_is_plain_int():
    p = algebra.Field(2).prime
    assert isinstance(p, int)
    assert p == 2


def test_dimension_degree_zero_and_nonzero():
    f = algebra.Field(2)
    # 1-dimensional, concentrated in degree 0.
    assert f.dimension(0) == 1
    assert f.dimension(1) == 0
    assert f.dimension(5) == 0
    # Negative degrees are empty, not errors.
    assert f.dimension(-1) == 0


def test_compute_basis_is_noop():
    # The field needs no book-keeping; compute_basis is a harmless no-op.
    f = algebra.Field(2)
    f.compute_basis(10)
    assert f.dimension(0) == 1


def test_basis_element_to_string_unit():
    f = algebra.Field(2)
    assert f.basis_element_to_string(0, 0) == "1"
    # Anything other than the single unit basis element raises IndexError.
    with pytest.raises(IndexError):
        f.basis_element_to_string(0, 1)
    with pytest.raises(IndexError):
        f.basis_element_to_string(1, 0)
    with pytest.raises(IndexError):
        f.basis_element_to_string(-1, 0)


def test_multiply_basis_elements_unit_times_unit():
    f = algebra.Field(2)
    # unit * unit = unit (coeff 1).
    v = fp.FpVector(2, f.dimension(0))
    f.multiply_basis_elements(v, 1, 0, 0, 0, 0)
    assert list(v) == [1]

    # Accumulates: doing it twice at p = 2 cancels to zero.
    f.multiply_basis_elements(v, 1, 0, 0, 0, 0)
    assert list(v) == [0]


def test_multiply_basis_elements_odd_prime_coeff():
    f = algebra.Field(3)
    v = fp.FpVector(3, 1)
    f.multiply_basis_elements(v, 2, 0, 0, 0, 0)
    assert list(v) == [2]
    # Large coeff is reduced mod p (7 % 3 == 1), never overflowing.
    v2 = fp.FpVector(3, 1)
    f.multiply_basis_elements(v2, 7, 0, 0, 0, 0)
    assert list(v2) == [1]


def test_multiply_basis_elements_accepts_fpslicemut():
    f = algebra.Field(2)
    vec = fp.FpVector(2, 1)
    f.multiply_basis_elements(vec.slice_mut(0, 1), 1, 0, 0, 0, 0)
    assert list(vec) == [1]


def test_multiply_basis_elements_out_of_range_and_errors():
    f = algebra.Field(2)

    # Out-of-range basis index (only (0, 0) is valid) -> IndexError, no panic.
    ok = fp.FpVector(2, 1)
    with pytest.raises(IndexError):
        f.multiply_basis_elements(ok, 1, 0, 1, 0, 0)
    with pytest.raises(IndexError):
        f.multiply_basis_elements(ok, 1, 1, 0, 0, 0)

    # Prime mismatch on the result -> ValueError.
    wrong_prime = fp.FpVector(3, 1)
    with pytest.raises(ValueError):
        f.multiply_basis_elements(wrong_prime, 1, 0, 0, 0, 0)

    # Result too short -> ValueError.
    short = fp.FpVector(2, 0)
    with pytest.raises(ValueError):
        f.multiply_basis_elements(short, 1, 0, 0, 0, 0)


def test_element_to_string():
    f = algebra.Field(3)
    v = fp.FpVector.from_slice(3, [2])
    assert f.element_to_string(0, v) == "2"

    # Length mismatch raises ValueError, not a panic.
    with pytest.raises(ValueError):
        f.element_to_string(0, fp.FpVector(3, 2))
    # Negative degree raises.
    with pytest.raises(IndexError):
        f.element_to_string(-1, v)


def test_coproduct_and_decompose():
    f = algebra.Field(2)
    assert isinstance(f.coproduct(0, 0), list)
    assert isinstance(f.decompose(0, 0), list)
    # Outside the single basis element -> raises, no panic.
    with pytest.raises(IndexError):
        f.coproduct(0, 1)
    with pytest.raises(IndexError):
        f.decompose(1, 0)


def test_field_is_not_fp():
    # `algebra.Field` is the trivial algebra; `fp.Fp` is the field type.
    f = algebra.Field(2)
    assert not isinstance(f, fp.Fp)
    assert isinstance(f, algebra.Field)


# --- HomModule.algebra ---------------------------------------------------


def milnor(p=2):
    return algebra.SteenrodAlgebra.milnor(p)


def make_c2(alg):
    """The bounded module X = C2: x0 in degree 0, x1 in degree 1, Sq1 x0 = x1."""
    m = algebra.FDModuleBuilder(alg, "C2", [1, 1])
    m.set_action(1, 0, 0, 0, [1])
    return m.build()


def free_one_gen(alg, name):
    """A FreeModule with a single generator in degree 0."""
    b = algebra.FPModuleBuilder(alg, name, 0)
    b.add_generators(0, [name + "g"])
    b.add_relations(0, [])
    f = b.build().generators()
    f.compute_basis(4)
    return f


def test_hom_module_algebra_returns_ground_field():
    alg = milnor(2)
    source = free_one_gen(alg, "F")
    x = make_c2(alg)
    hom = algebra.HomModule(source, x)

    field = hom.algebra
    # The Hom space is a module over the ground field, not the Steenrod algebra.
    assert isinstance(field, algebra.Field)
    assert field.prime == hom.prime == 2
    assert field.dimension(0) == 1
    assert field.dimension(1) == 0


def test_hom_module_algebra_prime_matches_at_odd_prime():
    alg = milnor(3)
    source = free_one_gen(alg, "F")
    x = algebra.FDModuleBuilder(alg, "pt", [1]).build()
    hom = algebra.HomModule(source, x)
    field = hom.algebra
    assert field.prime == 3
    assert field.dimension(0) == 1


def test_hom_module_has_no_into_steenrod_module():
    # A HomModule's algebra is the ground field, not the Steenrod algebra, so it
    # is not a SteenrodModule and exposes no into_steenrod_module().
    assert not hasattr(algebra.HomModule, "into_steenrod_module")
