"""Tests for the §6.2 spectral-sequence bindings in ``sseq``.

Covers the ``Sseq`` spectral sequence (monomorphized as ``Sseq<2, Adams>``),
the ``Differential`` and ``Product`` helper types, and the ``Adams`` /
``SseqProfile`` profile markers. Each guarded precondition is exercised to
confirm a clean exception is raised rather than a panic crossing the FFI
boundary.
"""

import pytest

from ext import fp, sseq

Bidegree = sseq.Bidegree
BidegreeElement = sseq.BidegreeElement
FpVector = fp.FpVector
Matrix = fp.Matrix


def vec(p, entries):
    return FpVector.from_slice(p, entries)


def elem(b, p, entries):
    return BidegreeElement(b, vec(p, entries))


# --------------------------------------------------------------------------
# Adams / SseqProfile profile markers
# --------------------------------------------------------------------------


def test_adams_profile_arithmetic():
    assert sseq.Adams.MIN_R == 2
    b = Bidegree.x_y(3, 1)
    target = sseq.Adams.profile(2, b)
    assert target == Bidegree.x_y(2, 3)
    assert sseq.Adams.profile_inverse(2, target) == b
    assert sseq.Adams.differential_length(Bidegree.x_y(-1, 2)) == 2


def test_sseq_profile_default_is_adams():
    default = sseq.SseqProfile.default()
    assert isinstance(default, sseq.Adams)


# --------------------------------------------------------------------------
# Sseq: construction and dimensions
# --------------------------------------------------------------------------


def test_sseq_valid_and_invalid_prime():
    s = sseq.Sseq(2)
    assert s.prime == 2
    with pytest.raises(ValueError):
        sseq.Sseq(4)
    with pytest.raises(ValueError):
        sseq.Sseq(0)


def test_set_get_dimension():
    s = sseq.Sseq(2)
    s.set_dimension(Bidegree.x_y(1, 0), 2)
    assert s.dimension(Bidegree.x_y(1, 0)) == 2
    assert s.get_dimension(Bidegree.x_y(1, 0)) == 2
    assert s.defined(Bidegree.x_y(1, 0))

    # Undefined bidegree: get_dimension is None, dimension raises.
    assert s.get_dimension(Bidegree.x_y(9, 9)) is None
    assert not s.defined(Bidegree.x_y(9, 9))
    with pytest.raises(IndexError):
        s.dimension(Bidegree.x_y(9, 9))

    # Re-defining a bidegree is a clean error, not a panic.
    with pytest.raises(ValueError):
        s.set_dimension(Bidegree.x_y(1, 0), 3)


def test_min_max_iter_degrees():
    s = sseq.Sseq(2)
    for b in [Bidegree.x_y(0, 0), Bidegree.x_y(2, 1), Bidegree.x_y(1, 0)]:
        s.set_dimension(b, 1)
    assert s.min() == Bidegree.x_y(0, 0)
    assert s.max() == Bidegree.x_y(2, 1)
    degrees = s.iter_degrees()
    assert len(degrees) == 3
    assert Bidegree.x_y(1, 0) in degrees


def test_clear():
    s = sseq.Sseq(2)
    s.set_dimension(Bidegree.x_y(0, 0), 1)
    s.add_permanent_class(elem(Bidegree.x_y(0, 0), 2, [1]))
    s.clear()
    # After clear, the bidegree is still defined but the page data is invalid.
    assert s.defined(Bidegree.x_y(0, 0))
    assert s.invalid(Bidegree.x_y(0, 0))


# --------------------------------------------------------------------------
# Sseq: a small worked example (mirrors upstream test_sseq_differential_2)
# --------------------------------------------------------------------------


def make_small_sseq():
    s = sseq.Sseq(2)
    s.set_dimension(Bidegree.x_y(0, 0), 0)
    s.set_dimension(Bidegree.x_y(1, 0), 2)
    s.set_dimension(Bidegree.x_y(0, 1), 0)
    s.set_dimension(Bidegree.x_y(0, 2), 2)
    # d_2([1,0]) = [1,0], d_2([0,1]) = [1,1] from (1,0) to (0,2).
    assert s.add_differential(2, elem(Bidegree.x_y(1, 0), 2, [1, 0]), vec(2, [1, 0]))
    assert s.add_differential(2, elem(Bidegree.x_y(1, 0), 2, [0, 1]), vec(2, [1, 1]))
    s.update()
    return s


def test_add_differential_and_page_data():
    s = make_small_sseq()
    # E_2 at (1,0) is the full 2-dimensional space; E_3 collapses to 0.
    assert s.page_data(Bidegree.x_y(1, 0), 2).dimension() == 2
    assert s.page_data(Bidegree.x_y(1, 0), 3).dimension() == 0
    # (0,2) is killed too.
    assert s.page_data(Bidegree.x_y(0, 2), 2).dimension() == 2
    assert s.page_data(Bidegree.x_y(0, 2), 3).dimension() == 0


def test_differentials_and_hitting():
    s = make_small_sseq()
    diffs = s.differentials(Bidegree.x_y(1, 0))
    assert len(diffs) >= 1
    assert isinstance(diffs[0], sseq.Differential)
    # (0,2) is hit by the d_2 out of (1,0).
    hitting = s.differentials_hitting(Bidegree.x_y(0, 2))
    assert any(r == 2 for (r, _d) in hitting)


def test_update_degree_returns_drawn_differentials():
    s = sseq.Sseq(2)
    s.set_dimension(Bidegree.x_y(0, 0), 0)
    s.set_dimension(Bidegree.x_y(1, 0), 2)
    s.set_dimension(Bidegree.x_y(0, 1), 0)
    s.set_dimension(Bidegree.x_y(0, 2), 2)
    s.add_differential(2, elem(Bidegree.x_y(1, 0), 2, [1, 0]), vec(2, [1, 0]))
    drawn = s.update_degree(Bidegree.x_y(1, 0))
    # A list (per page) of lists (per generator) of target coordinate lists.
    assert isinstance(drawn, list)


def test_complete_and_inconsistent():
    s = make_small_sseq()
    # complete returns a bool without panicking on a defined degree.
    assert isinstance(s.complete(Bidegree.x_y(1, 0)), bool)
    assert s.inconsistent(Bidegree.x_y(1, 0)) is False


def test_permanent_classes():
    s = sseq.Sseq(2)
    s.set_dimension(Bidegree.x_y(0, 0), 2)
    assert s.add_permanent_class(elem(Bidegree.x_y(0, 0), 2, [1, 0]))
    # Adding the same class again is not new.
    assert not s.add_permanent_class(elem(Bidegree.x_y(0, 0), 2, [1, 0]))
    classes = s.permanent_classes(Bidegree.x_y(0, 0))
    assert isinstance(classes, fp.Subspace)
    assert classes.dimension() == 1
    assert classes.contains(vec(2, [1, 0]))


# --------------------------------------------------------------------------
# Sseq: guarded preconditions raise clean exceptions
# --------------------------------------------------------------------------


def test_add_differential_guards():
    s = sseq.Sseq(2)
    s.set_dimension(Bidegree.x_y(1, 0), 2)
    s.set_dimension(Bidegree.x_y(0, 2), 2)
    src = elem(Bidegree.x_y(1, 0), 2, [1, 0])

    # Page below MIN_R.
    with pytest.raises(ValueError):
        s.add_differential(1, src, vec(2, [1, 0]))
    # Target length mismatch (target dim is 2).
    with pytest.raises(ValueError):
        s.add_differential(2, src, vec(2, [1, 0, 1]))
    # Undefined source bidegree.
    with pytest.raises(IndexError):
        s.add_differential(2, elem(Bidegree.x_y(5, 0), 2, [1, 0]), vec(2, [1, 0]))


def test_add_permanent_class_guards():
    s = sseq.Sseq(2)
    s.set_dimension(Bidegree.x_y(0, 0), 2)
    # Undefined bidegree.
    with pytest.raises(IndexError):
        s.add_permanent_class(elem(Bidegree.x_y(9, 9), 2, [1, 0]))
    # Length mismatch.
    with pytest.raises(ValueError):
        s.add_permanent_class(elem(Bidegree.x_y(0, 0), 2, [1, 0, 1]))


def test_page_data_out_of_range():
    s = make_small_sseq()
    with pytest.raises(IndexError):
        s.page_data(Bidegree.x_y(1, 0), 99)
    with pytest.raises(IndexError):
        s.page_data(Bidegree.x_y(9, 9), 2)


# --------------------------------------------------------------------------
# Product and multiply
# --------------------------------------------------------------------------


def test_product_construction_and_getters():
    m = Matrix.from_vec(2, [[1]])
    prod = sseq.Product(Bidegree.x_y(1, 1), True, [(Bidegree.x_y(0, 0), m)])
    assert prod.b == Bidegree.x_y(1, 1)
    assert prod.left is True
    mats = prod.matrices
    assert len(mats) == 1
    assert mats[0][0] == Bidegree.x_y(0, 0)
    assert prod.get_matrix(Bidegree.x_y(0, 0)) is not None
    assert prod.get_matrix(Bidegree.x_y(5, 5)) is None

    # Duplicate bidegree -> ValueError.
    with pytest.raises(ValueError):
        sseq.Product(
            Bidegree.x_y(1, 1),
            True,
            [(Bidegree.x_y(0, 0), Matrix.from_vec(2, [[1]])),
             (Bidegree.x_y(0, 0), Matrix.from_vec(2, [[1]]))],
        )


def test_multiply():
    s = sseq.Sseq(2)
    s.set_dimension(Bidegree.x_y(0, 0), 1)
    s.set_dimension(Bidegree.x_y(1, 1), 1)
    # Multiplication by a class in bidegree (1,1): 1x1 identity at (0,0).
    prod = sseq.Product(
        Bidegree.x_y(1, 1), True, [(Bidegree.x_y(0, 0), Matrix.from_vec(2, [[1]]))]
    )
    result = s.multiply(elem(Bidegree.x_y(0, 0), 2, [1]), prod)
    assert result is not None
    assert result.degree == Bidegree.x_y(1, 1)
    assert result.vec().entry(0) == 1

    # No matrix at this source bidegree -> None.
    assert s.multiply(elem(Bidegree.x_y(1, 1), 2, [1]), prod) is None


# --------------------------------------------------------------------------
# Differential helper type
# --------------------------------------------------------------------------


def test_differential_round_trip():
    d = sseq.Differential(2, 2, 1)
    assert d.prime == 2
    assert d.source_dim == 2
    assert d.target_dim == 1

    # d([1,0]) = [1].
    assert d.add(vec(2, [1, 0]), vec(2, [1]))
    # Same differential again is not new.
    assert not d.add(vec(2, [1, 0]), vec(2, [1]))

    # evaluate writes into the target.
    out = FpVector(2, 1)
    d.evaluate(vec(2, [1, 0]), out)
    assert out.entry(0) == 1

    pairs = d.get_source_target_pairs()
    assert len(pairs) == 1
    src, tgt = pairs[0]
    assert [src.entry(0), src.entry(1)] == [1, 0]
    assert tgt.entry(0) == 1

    # quasi_inverse returns a preimage of length source_dim.
    preimage = d.quasi_inverse(vec(2, [1]))
    assert preimage.len() == 2
    check = FpVector(2, 1)
    d.evaluate(preimage, check)
    assert check.entry(0) == 1


def test_differential_set_to_zero_and_inconsistent():
    d = sseq.Differential(2, 1, 1)
    d.add(vec(2, [1]), vec(2, [1]))
    assert d.get_source_target_pairs()
    d.set_to_zero()
    assert d.get_source_target_pairs() == []
    assert d.inconsistent() is False


def test_differential_guards():
    d = sseq.Differential(2, 2, 1)
    # Wrong source length.
    with pytest.raises(ValueError):
        d.add(vec(2, [1, 0, 1]), None)
    # Wrong target length.
    with pytest.raises(ValueError):
        d.add(vec(2, [1, 0]), vec(2, [1, 1]))
    # Prime mismatch.
    with pytest.raises(ValueError):
        d.add(vec(3, [1, 0]), None)
    # quasi_inverse with wrong value length.
    with pytest.raises(ValueError):
        d.quasi_inverse(vec(2, [1, 1]))


def test_differential_invalid_prime():
    with pytest.raises(ValueError):
        sseq.Differential(4, 1, 1)


# --------------------------------------------------------------------------
# Leibniz rule
# --------------------------------------------------------------------------


def test_leibniz_permanent_product():
    # A permanent class times a permanent product yields a new permanent class.
    s = sseq.Sseq(2)
    s.set_dimension(Bidegree.x_y(0, 0), 1)
    s.set_dimension(Bidegree.x_y(1, 1), 1)
    s.add_permanent_class(elem(Bidegree.x_y(0, 0), 2, [1]))
    prod = sseq.Product(
        Bidegree.x_y(1, 1), True, [(Bidegree.x_y(0, 0), Matrix.from_vec(2, [[1]]))]
    )
    # r = i32::MAX signals a permanent source; target_product=None means the
    # product is permanent too.
    result = s.leibniz((1 << 31) - 1, elem(Bidegree.x_y(0, 0), 2, [1]), prod, None)
    assert result is not None
    r, new_class = result
    assert new_class.degree == Bidegree.x_y(1, 1)
    assert s.permanent_classes(Bidegree.x_y(1, 1)).dimension() == 1


def test_leibniz_guards():
    s = sseq.Sseq(2)
    s.set_dimension(Bidegree.x_y(0, 0), 1)
    prod = sseq.Product(Bidegree.x_y(1, 1), True, [])
    # Undefined source bidegree.
    with pytest.raises(IndexError):
        s.leibniz(2, elem(Bidegree.x_y(9, 9), 2, [1]), prod, None)


def test_leibniz_product_prime_mismatch():
    # A product matrix over the wrong prime raises a clear ValueError
    # ("product prime mismatch") rather than an opaque catch_unwind message.
    s = sseq.Sseq(2)
    s.set_dimension(Bidegree.x_y(0, 0), 1)
    s.set_dimension(Bidegree.x_y(1, 1), 1)
    # Matrix over F_3 but the Sseq is over F_2.
    bad_prod = sseq.Product(
        Bidegree.x_y(1, 1), True, [(Bidegree.x_y(0, 0), Matrix.from_vec(3, [[1]]))]
    )
    with pytest.raises(ValueError, match="product prime mismatch"):
        s.leibniz((1 << 31) - 1, elem(Bidegree.x_y(0, 0), 2, [1]), bad_prod, None)
    # The source-product check also covers the differential (target) product.
    good_prod = sseq.Product(
        Bidegree.x_y(1, 1), True, [(Bidegree.x_y(0, 0), Matrix.from_vec(2, [[1]]))]
    )
    with pytest.raises(ValueError, match="product prime mismatch"):
        s.leibniz(
            2,
            elem(Bidegree.x_y(0, 0), 2, [1]),
            good_prod,
            bad_prod,
        )
    # The Sseq is untouched: the wrong-prime product was rejected before any
    # mutation, so a subsequent read still works.
    assert s.dimension(Bidegree.x_y(0, 0)) == 1


# --------------------------------------------------------------------------
# Fix 1: d_r for r >= 3 must guard *intermediate* target bidegrees
# --------------------------------------------------------------------------


def test_add_differential_r3_undefined_intermediate_raises_index_error():
    # A d_3 out of (1,0) makes upstream index profile(2,(1,0)) = (0,2) and
    # profile(3,(1,0)) = (0,3). With (0,2) undefined (but (0,3) defined!), the
    # binding used to pass its single final-target check and then panic in
    # MultiIndexed. It must now raise a clean IndexError naming (0, 2).
    s = sseq.Sseq(2)
    s.set_dimension(Bidegree.x_y(1, 0), 2)
    # (0,2) deliberately NOT defined.
    s.set_dimension(Bidegree.x_y(0, 3), 2)

    assert sseq.Adams.profile(2, Bidegree.x_y(1, 0)) == Bidegree.x_y(0, 2)
    assert sseq.Adams.profile(3, Bidegree.x_y(1, 0)) == Bidegree.x_y(0, 3)

    src = elem(Bidegree.x_y(1, 0), 2, [1, 0])
    with pytest.raises(IndexError):
        s.add_differential(3, src, vec(2, [1, 0]))


def test_add_differential_r3_all_defined_succeeds():
    # With every intermediate (profile(2,..)=(0,2)) and final
    # (profile(3,..)=(0,3)) target defined, the d_3 is recorded and shows up
    # as a differential hitting (0,3); the source class is killed on E_4.
    s = sseq.Sseq(2)
    s.set_dimension(Bidegree.x_y(1, 0), 1)
    s.set_dimension(Bidegree.x_y(0, 2), 1)
    s.set_dimension(Bidegree.x_y(0, 3), 1)

    assert s.add_differential(3, elem(Bidegree.x_y(1, 0), 2, [1]), vec(2, [1]))
    s.update()

    hitting = s.differentials_hitting(Bidegree.x_y(0, 3))
    assert any(r == 3 for (r, _d) in hitting)
    # E_3 at (1,0) still has the class; E_4 has it killed (it supports a d_3).
    assert s.page_data(Bidegree.x_y(1, 0), 3).dimension() == 1
    assert s.page_data(Bidegree.x_y(1, 0), 4).dimension() == 0
    # (0,3) is a boundary on E_4.
    assert s.page_data(Bidegree.x_y(0, 3), 4).dimension() == 0


# --------------------------------------------------------------------------
# Other review gaps: shape-mismatched multiply, aliased evaluate
# --------------------------------------------------------------------------


def test_multiply_shape_mismatch_raises():
    # A product matrix whose row/column count disagrees with the source/target
    # dimensions must raise a clean error instead of panicking in Matrix::apply.
    s = sseq.Sseq(2)
    s.set_dimension(Bidegree.x_y(0, 0), 2)
    s.set_dimension(Bidegree.x_y(1, 1), 2)
    # Source dim is 2 but the matrix has a single row.
    prod = sseq.Product(
        Bidegree.x_y(1, 1), True, [(Bidegree.x_y(0, 0), Matrix.from_vec(2, [[1, 0]]))]
    )
    with pytest.raises(ValueError):
        s.multiply(elem(Bidegree.x_y(0, 0), 2, [1, 0]), prod)


def test_differential_evaluate_aliased_source_target_raises():
    # Passing the same FpVector as both source and target violates PyO3's
    # borrow rules (one shared, one exclusive borrow of the same object) and
    # must surface as a RuntimeError, not UB.
    d = sseq.Differential(2, 2, 2)
    d.add(vec(2, [1, 0]), vec(2, [1, 0]))
    shared = vec(2, [1, 0])
    with pytest.raises(RuntimeError):
        d.evaluate(shared, shared)
