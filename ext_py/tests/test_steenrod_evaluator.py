import pytest

from ext import algebra, fp


def test_construction_valid_and_invalid_prime():
    ev = algebra.SteenrodEvaluator(2)
    assert ev.prime == 2
    assert algebra.SteenrodEvaluator(3).prime == 3

    # A non-prime must raise ValueError, never panic.
    with pytest.raises(ValueError):
        algebra.SteenrodEvaluator(4)
    with pytest.raises(ValueError):
        algebra.SteenrodEvaluator(0)
    with pytest.raises(ValueError):
        algebra.SteenrodEvaluator(1)


def test_evaluate_algebra_adem_known_value():
    ev = algebra.SteenrodEvaluator(2)
    degree, vec = ev.evaluate_algebra_adem("Sq2 * Sq2")
    assert isinstance(degree, int)
    assert degree == 4
    assert isinstance(vec, fp.FpVector)
    # Sq2 Sq2 = Sq3 Sq1 (one admissible monomial); the Adem element is nonzero.
    a = algebra.AdemAlgebra(2)
    a.compute_basis(degree)
    assert a.element_to_string(degree, vec) == "Sq3 Sq1"


def test_evaluate_algebra_adem_zero():
    ev = algebra.SteenrodEvaluator(2)
    # Sq1 Sq1 = 0 by the Adem relations, but it still lives in degree 2.
    degree, vec = ev.evaluate_algebra_adem("Sq1 * Sq1")
    assert degree == 2
    assert vec.is_zero()


def test_evaluate_algebra_milnor_known_value():
    ev = algebra.SteenrodEvaluator(2)
    degree, vec = ev.evaluate_algebra_milnor("Sq2 * Sq2")
    assert degree == 4
    m = algebra.MilnorAlgebra(2)
    m.compute_basis(degree)
    assert m.element_to_string(degree, vec) == "P(1, 1)"


def test_evaluate_module_adem_returns_dict():
    ev = algebra.SteenrodEvaluator(2)
    result = ev.evaluate_module_adem("Sq2 * x0 + x1")
    assert isinstance(result, dict)
    assert set(result) == {"x0", "x1"}
    deg0, vec0 = result["x0"]
    assert deg0 == 2
    assert isinstance(vec0, fp.FpVector)
    deg1, vec1 = result["x1"]
    assert deg1 == 0


def test_adem_milnor_roundtrip():
    ev = algebra.SteenrodEvaluator(2)
    degree, adem_vec = ev.evaluate_algebra_adem("Sq2 * Sq2")
    milnor_vec = ev.adem_to_milnor(degree, adem_vec)
    assert isinstance(milnor_vec, fp.FpVector)
    m = algebra.MilnorAlgebra(2)
    m.compute_basis(degree)
    assert m.element_to_string(degree, milnor_vec) == "P(1, 1)"

    back = ev.milnor_to_adem(degree, milnor_vec)
    a = algebra.AdemAlgebra(2)
    a.compute_basis(degree)
    assert a.element_to_string(degree, back) == "Sq3 Sq1"


def test_change_basis_validates():
    ev = algebra.SteenrodEvaluator(2)
    # Negative degree raises.
    with pytest.raises(Exception):
        ev.adem_to_milnor(-1, fp.FpVector(2, 0))
    # Length mismatch raises ValueError.
    with pytest.raises(ValueError):
        ev.adem_to_milnor(4, fp.FpVector(2, 99))
    # Prime mismatch raises ValueError.
    with pytest.raises(ValueError):
        ev.adem_to_milnor(4, fp.FpVector(3, 1))


def test_malformed_input_raises_value_error():
    ev = algebra.SteenrodEvaluator(2)
    with pytest.raises(ValueError):
        ev.evaluate_algebra_adem("Sqx")
    with pytest.raises(ValueError):
        ev.evaluate_algebra_adem("Sq2 +")
    with pytest.raises(ValueError):
        ev.evaluate_algebra_milnor("not an expression!!")
    with pytest.raises(ValueError):
        ev.evaluate_module_adem("x0 + ")


def test_parse_algebra_tree():
    node = algebra.parse_algebra("Sq2 * Sq2")
    assert node.kind() == "Product"
    left = node.left()
    right = node.right()
    assert left.kind() == "BasisElt"
    assert right.kind() == "BasisElt"
    be = left.basis_element()
    assert be.kind() == "P"
    assert be.p() == 2
    # Wrong-shape accessors raise.
    with pytest.raises(ValueError):
        be.q()
    with pytest.raises(ValueError):
        node.scalar()
    assert repr(node)


def test_parse_algebra_scalar_and_qlist():
    node = algebra.parse_algebra("3")
    assert node.kind() == "Scalar"
    assert node.scalar() == 3

    q = algebra.parse_algebra("Q2")
    be = q.basis_element()
    assert be.kind() == "Q"
    assert be.q() == 2

    plist = algebra.parse_algebra("P(1, 0)")
    be = plist.basis_element()
    assert be.kind() == "PList"
    assert be.p_list() == [1, 0]


def test_parse_module_tree():
    tree = algebra.parse_module("Sq2 * x0 + x1")
    assert isinstance(tree, list)
    assert len(tree) == 2
    node0, gen0 = tree[0]
    assert gen0 == "x0"
    assert node0.kind() == "BasisElt"
    node1, gen1 = tree[1]
    assert gen1 == "x1"


def test_parse_errors_raise():
    with pytest.raises(ValueError):
        algebra.parse_algebra("Sq2 +")
    with pytest.raises(ValueError):
        algebra.parse_module("x0 + ")


def test_bockstein_or_sq_variants():
    node = algebra.parse_algebra("A(2 b 5)")
    be = node.basis_element()
    assert be.kind() == "AList"
    items = be.a_list()
    assert len(items) == 3
    assert all(isinstance(x, algebra.BocksteinOrSq) for x in items)
    # Sq(2), Bockstein, Sq(5)
    assert isinstance(items[0], algebra.BocksteinOrSq.Sq)
    assert items[0]._0 == 2
    assert isinstance(items[1], algebra.BocksteinOrSq.Bockstein)
    assert isinstance(items[2], algebra.BocksteinOrSq.Sq)
    assert items[2]._0 == 5

    # Variants are directly constructible (mirroring PorBockstein).
    assert isinstance(algebra.BocksteinOrSq.Sq(7), algebra.BocksteinOrSq)
    assert isinstance(algebra.BocksteinOrSq.Bockstein(), algebra.BocksteinOrSq)
