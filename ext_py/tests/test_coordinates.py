import pytest

from ext import fp, sseq


def test_bidegree_constructors_and_coords():
    # s_t(s, t): n = t - s.
    b = sseq.Bidegree.s_t(2, 5)
    assert b.s == 2
    assert b.t == 5
    assert b.n == 3
    assert b.x == 3
    assert b.y == 2
    assert b.coords == (3, 2)

    # n_s(n, s): t = n + s.
    b = sseq.Bidegree.n_s(3, 2)
    assert (b.n, b.s, b.t) == (3, 2, 5)

    # x_y(x, y) == n_s(x, y).
    b = sseq.Bidegree.x_y(3, 2)
    assert (b.n, b.s) == (3, 2)

    # The constructors agree.
    assert sseq.Bidegree.s_t(2, 5) == sseq.Bidegree.n_s(3, 2)
    assert sseq.Bidegree.n_s(3, 2) == sseq.Bidegree.x_y(3, 2)


def test_bidegree_add_sub():
    a = sseq.Bidegree.n_s(3, 2)
    b = sseq.Bidegree.n_s(1, 1)
    assert a + b == sseq.Bidegree.n_s(4, 3)
    assert a - b == sseq.Bidegree.n_s(2, 1)


def test_bidegree_str():
    b = sseq.Bidegree.n_s(3, 2)
    assert str(b) == "(3, 2)"
    assert "n_s(3, 2)" in repr(b)


def test_bidegree_eq_hash_and_as_keys():
    a = sseq.Bidegree.n_s(3, 2)
    b = sseq.Bidegree.s_t(2, 5)  # same as a
    c = sseq.Bidegree.n_s(0, 0)

    assert a == b
    assert a != c
    assert hash(a) == hash(b)

    # Usable as dict keys / set members.
    d = {a: "x"}
    assert d[b] == "x"
    s = {a, b, c}
    assert len(s) == 2


def test_bidegree_element_roundtrip_vec():
    deg = sseq.Bidegree.n_s(23, 9)
    vec = fp.FpVector.from_slice(2, [0, 1])
    el = sseq.BidegreeElement(deg, vec)

    assert el.degree == deg
    assert (el.n, el.s, el.t) == (23, 9, 32)
    assert el.x == 23
    assert el.y == 9

    out = el.vec()
    assert isinstance(out, fp.FpVector)
    assert [out[i] for i in range(len(out))] == [0, 1]

    out2 = el.into_vec()
    assert [out2[i] for i in range(len(out2))] == [0, 1]

    assert el.to_basis_string() == "x_(23, 9, 1)"


def test_bidegree_element_basis_string_multiple_terms():
    deg = sseq.Bidegree.n_s(23, 9)
    vec = fp.FpVector.from_slice(2, [1, 0, 1])
    el = sseq.BidegreeElement(deg, vec)
    assert el.to_basis_string() == "x_(23, 9, 0) + x_(23, 9, 2)"


def test_bidegree_generator_constructors_and_into_element():
    g = sseq.BidegreeGenerator(sseq.Bidegree.n_s(3, 2), 1)
    assert g.idx == 1
    assert g.degree == sseq.Bidegree.n_s(3, 2)
    assert (g.n, g.s, g.t) == (3, 2, 5)

    assert sseq.BidegreeGenerator.n_s(3, 2, 1).degree == sseq.Bidegree.n_s(3, 2)
    assert sseq.BidegreeGenerator.s_t(2, 5, 1).degree == sseq.Bidegree.s_t(2, 5)

    el = g.into_element(2, 4)
    assert isinstance(el, sseq.BidegreeElement)
    assert el.degree == sseq.Bidegree.n_s(3, 2)
    assert el.to_basis_string() == "x_(3, 2, 1)"

    assert str(g) == "(3, 2, 1)"


def test_bidegree_generator_into_element_bad_input():
    g = sseq.BidegreeGenerator(sseq.Bidegree.n_s(3, 2), 3)
    # idx >= ambient -> IndexError (no panic).
    with pytest.raises(IndexError):
        g.into_element(2, 1)
    # non-prime -> ValueError.
    with pytest.raises(ValueError):
        g.into_element(4, 8)


def test_bidegree_range():
    rng = sseq.BidegreeRange(3, lambda s: 4)
    assert rng.s == 3
    assert rng.t(0) == 4
    assert rng.t(2) == 4

    smaller = rng.restrict(2)
    assert smaller.s == 2
    assert smaller.t(1) == 4

    # restricting to a larger s -> ValueError.
    with pytest.raises(ValueError):
        rng.restrict(5)


def test_iter_s_t_visits_expected_bidegrees():
    visited = []

    def callback(b):
        visited.append((b.n, b.s))
        return None  # empty range -> no cascade

    rng = sseq.BidegreeRange(3, lambda s: 4)
    sseq.iter_s_t(callback, sseq.Bidegree.n_s(0, 0), rng)

    assert set(visited) == {(0, 0), (1, 0), (2, 0), (3, 0), (-1, 1), (-2, 2)}


def test_iter_s_t_callback_exception_propagates():
    def callback(b):
        raise RuntimeError("boom")

    rng = sseq.BidegreeRange(3, lambda s: 4)
    with pytest.raises(RuntimeError, match="boom"):
        sseq.iter_s_t(callback, sseq.Bidegree.n_s(0, 0), rng)


def test_iter_s_t_empty_range_raises():
    rng = sseq.BidegreeRange(0, lambda s: 4)
    with pytest.raises(ValueError):
        sseq.iter_s_t(lambda b: None, sseq.Bidegree.n_s(0, 0), rng)


def test_iter_s_t_t_callback_exception_propagates():
    # An exception raised by the range's `t(s)` callback (not the per-bidegree
    # callback) is propagated as a Python exception.
    def bad_t(s):
        raise RuntimeError("t boom")

    rng = sseq.BidegreeRange(3, bad_t)
    with pytest.raises(RuntimeError, match="t boom"):
        sseq.iter_s_t(lambda b: None, sseq.Bidegree.n_s(0, 0), rng)


def test_iter_s_t_non_callable_t_raises_cleanly():
    # A non-callable `t` -> clean Python exception (TypeError), not a panic.
    rng = sseq.BidegreeRange(3, 4)
    with pytest.raises(TypeError):
        sseq.iter_s_t(lambda b: None, sseq.Bidegree.n_s(0, 0), rng)


def test_iter_s_t_wrong_arity_t_raises_cleanly():
    # A `t` callback with the wrong arity -> clean Python exception, not a panic.
    rng = sseq.BidegreeRange(3, lambda: 4)
    with pytest.raises(TypeError):
        sseq.iter_s_t(lambda b: None, sseq.Bidegree.n_s(0, 0), rng)


def test_iter_s_t_non_callable_callback_raises_cleanly():
    # A non-callable per-bidegree `callback` -> clean Python exception, not a
    # panic.
    rng = sseq.BidegreeRange(3, lambda s: 4)
    with pytest.raises(TypeError):
        sseq.iter_s_t(42, sseq.Bidegree.n_s(0, 0), rng)


def test_iter_s_t_wrong_arity_callback_raises_cleanly():
    # A per-bidegree callback with the wrong arity -> clean Python exception.
    rng = sseq.BidegreeRange(3, lambda s: 4)
    with pytest.raises(TypeError):
        sseq.iter_s_t(lambda: None, sseq.Bidegree.n_s(0, 0), rng)


def test_iter_s_t_callback_bad_return_string_raises_valueerror():
    # callback returns a malformed value (a string) -> clean ValueError from
    # extract_callback_range, not a panic.
    rng = sseq.BidegreeRange(3, lambda s: 4)
    with pytest.raises(ValueError):
        sseq.iter_s_t(lambda b: "nope", sseq.Bidegree.n_s(0, 0), rng)


def test_iter_s_t_callback_bad_return_wrong_length_tuple_raises_valueerror():
    # callback returns a wrong-length tuple -> clean ValueError, not a panic.
    rng = sseq.BidegreeRange(3, lambda s: 4)
    with pytest.raises(ValueError):
        sseq.iter_s_t(lambda b: (1, 2, 3), sseq.Bidegree.n_s(0, 0), rng)
