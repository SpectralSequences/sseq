import pytest

from ext import fp


def make_qi():
    # Mirrors the Rust `test_stream_qi` example at p = 2.
    preimage = fp.Matrix.from_vec(
        2,
        [
            [1, 0, 1, 1],
            [1, 1, 0, 0],
            [0, 1, 0, 1],
            [1, 1, 1, 0],
        ],
    )
    return fp.QuasiInverse([0, -1, 1, -1, 2, 3], preimage)


def test_import_and_construction():
    qi = make_qi()
    assert qi.prime() == 2
    assert qi.image_dimension() == 4
    assert qi.source_dimension() == 4
    assert qi.target_dimension() == 6
    assert "QuasiInverse(2" in repr(qi)


def test_pivots_and_preimage():
    qi = make_qi()
    assert qi.pivots() == [0, -1, 1, -1, 2, 3]
    assert qi.preimage().to_vec() == [
        [1, 0, 1, 1],
        [1, 1, 0, 0],
        [0, 1, 0, 1],
        [1, 1, 1, 0],
    ]


def test_apply_known_example():
    qi = make_qi()
    v = fp.FpVector.from_slice(2, [1, 1, 0, 0, 1, 0])
    out = fp.FpVector(2, 4)
    qi.apply(out, 1, v)
    assert list(out) == [1, 1, 1, 0]


def test_apply_accepts_slices():
    qi = make_qi()
    v = fp.FpVector.from_slice(2, [1, 1, 0, 0, 1, 0])
    out = fp.FpVector(2, 4)
    qi.apply(out.slice_mut(0, 4), 1, v.slice(0, 6))
    assert list(out) == [1, 1, 1, 0]


def test_apply_dimension_and_prime_mismatch():
    qi = make_qi()
    out = fp.FpVector(2, 4)
    # Wrong input length (target_dimension is 6).
    bad_input = fp.FpVector(2, 5)
    with pytest.raises(ValueError):
        qi.apply(out, 1, bad_input)
    # Wrong output length (source_dimension is 4).
    good_input = fp.FpVector(2, 6)
    with pytest.raises(ValueError):
        qi.apply(fp.FpVector(2, 3), 1, good_input)
    # Prime mismatch on input.
    with pytest.raises(ValueError):
        qi.apply(out, 1, fp.FpVector(3, 6))


def test_apply_wrong_type():
    qi = make_qi()
    with pytest.raises(ValueError):
        qi.apply(123, 1, fp.FpVector(2, 6))


def test_bytes_roundtrip():
    qi = make_qi()
    data = qi.to_bytes()
    assert isinstance(data, bytes)
    restored = fp.QuasiInverse.from_bytes(2, data)
    assert restored.source_dimension() == qi.source_dimension()
    assert restored.target_dimension() == qi.target_dimension()
    assert restored.image_dimension() == qi.image_dimension()

    v = fp.FpVector.from_slice(2, [1, 1, 0, 0, 1, 0])
    out = fp.FpVector(2, 4)
    restored.apply(out, 1, v)
    assert list(out) == [1, 1, 1, 0]


def test_from_bytes_malformed():
    with pytest.raises(RuntimeError):
        fp.QuasiInverse.from_bytes(2, b"\x00\x01\x02")


def test_compute_quasi_inverse_from_matrix():
    # Example from the Rust `compute_quasi_inverse` doc at p = 3.
    rows = [
        [1, 2, 1, 1, 0],
        [1, 0, 2, 1, 1],
        [2, 2, 0, 2, 1],
    ]
    padded_cols, m = fp.Matrix.augmented_from_vec(3, rows)
    m.row_reduce()
    qi = m.compute_quasi_inverse(len(rows[0]), padded_cols)
    assert qi.prime() == 3
    assert qi.source_dimension() == 3
    assert qi.preimage().to_vec() == [[0, 1, 0], [0, 2, 2]]


def test_compute_quasi_inverse_out_of_range():
    _, m = fp.Matrix.augmented_from_vec(3, [[1, 0, 1], [0, 1, 1]])
    with pytest.raises(IndexError):
        m.compute_quasi_inverse(2, 999)
