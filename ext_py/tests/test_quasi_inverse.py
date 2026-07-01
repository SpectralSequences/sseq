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
    assert qi.prime == 2
    assert qi.image_dimension == 4
    assert qi.source_dimension == 4
    assert qi.target_dimension == 6
    assert "QuasiInverse(2" in repr(qi)


def test_pivots_and_preimage():
    qi = make_qi()
    assert qi.pivots == [0, -1, 1, -1, 2, 3]
    assert qi.preimage.to_vec() == [
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


def make_identity_qi():
    # Square identity preimage at p=3, so source==target==3 and a single
    # length-3 vector is a valid argument both as input and as output target.
    preimage = fp.Matrix.from_vec(3, [[1, 0, 0], [0, 1, 0], [0, 0, 1]])
    return fp.QuasiInverse([0, 1, 2], preimage)


def test_apply_input_target_aliasing_raises_runtimeerror():
    # Passing the SAME bare FpVector as both the mutable target and the input
    # is an aliasing conflict: it must raise RuntimeError (borrow conflict),
    # NOT the generic ValueError reserved for wrong-type arguments.
    qi = make_identity_qi()
    v = fp.FpVector.from_slice(3, [1, 2, 1])
    with pytest.raises(RuntimeError):
        qi.apply(v, 1, v)
    with pytest.raises(Exception) as excinfo:
        qi.apply(v, 1, v)
    assert not isinstance(excinfo.value, ValueError)


def test_apply_wrong_type_is_valueerror_not_runtimeerror():
    # A genuine wrong-type argument still raises ValueError (not RuntimeError).
    qi = make_identity_qi()
    good = fp.FpVector(3, 3)
    with pytest.raises(ValueError):
        qi.apply(123, 1, good)
    with pytest.raises(ValueError):
        qi.apply(fp.Matrix.from_vec(3, [[1, 0, 0]]), 1, good)


def test_apply_distinct_objects_regression():
    # The normal distinct-objects call still produces the known value.
    qi = make_identity_qi()
    v = fp.FpVector.from_slice(3, [1, 2, 1])
    out = fp.FpVector(3, 3)
    qi.apply(out, 2, v)
    assert list(out) == [2, 1, 2]


def test_bytes_roundtrip():
    qi = make_qi()
    data = qi.to_bytes()
    assert isinstance(data, bytes)
    restored = fp.QuasiInverse.from_bytes(2, data)
    assert restored.source_dimension == qi.source_dimension
    assert restored.target_dimension == qi.target_dimension
    assert restored.image_dimension == qi.image_dimension

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
    assert qi.prime == 3
    assert qi.source_dimension == 3
    assert qi.preimage.to_vec() == [[0, 1, 0], [0, 2, 2]]


def test_compute_quasi_inverse_out_of_range():
    _, m = fp.Matrix.augmented_from_vec(3, [[1, 0, 1], [0, 1, 1]])
    # Row reduce first so we exercise the column-range check rather than the
    # not-row-reduced guard (which would otherwise fire first).
    m.row_reduce()
    with pytest.raises(IndexError):
        m.compute_quasi_inverse(2, 999)


def test_compute_quasi_inverse_requires_row_reduce():
    # Without row_reduce the matrix has uninitialized (empty) pivots, which
    # upstream would slice out of bounds and panic. The guard turns that into
    # a clean ValueError.
    padded_cols, m = fp.Matrix.augmented_from_vec(3, [[1, 0, 1], [0, 1, 1]])
    with pytest.raises(ValueError):
        m.compute_quasi_inverse(2, padded_cols)


def test_inconsistent_image_raises():
    # preimage has 4 rows; supply an image with 5 non-negative pivots.
    preimage = fp.Matrix.from_vec(
        2,
        [
            [1, 0, 1, 1],
            [1, 1, 0, 0],
            [0, 1, 0, 1],
            [1, 1, 1, 0],
        ],
    )
    with pytest.raises(ValueError):
        fp.QuasiInverse([0, 1, 2, 3, 0], preimage)


def test_pivot_out_of_range_raises():
    # preimage has 4 rows; a non-negative pivot of 4 is an invalid row index.
    preimage = fp.Matrix.from_vec(
        2,
        [
            [1, 0, 1, 1],
            [1, 1, 0, 0],
            [0, 1, 0, 1],
            [1, 1, 1, 0],
        ],
    )
    with pytest.raises(ValueError):
        fp.QuasiInverse([0, 1, 2, 4], preimage)


def test_apply_with_coeff_odd_prime():
    # At p = 3, identity-style preimage so the result is `coeff * input`.
    preimage = fp.Matrix.from_vec(
        3,
        [
            [1, 0, 0],
            [0, 1, 0],
            [0, 0, 1],
        ],
    )
    qi = fp.QuasiInverse([0, 1, 2], preimage)
    v = fp.FpVector.from_slice(3, [1, 2, 1])

    out = fp.FpVector(3, 3)
    qi.apply(out, 2, v)
    # 2 * [1, 2, 1] mod 3 = [2, 1, 2]
    assert list(out) == [2, 1, 2]

    # A large coeff must reduce mod p and not overflow/panic.
    out2 = fp.FpVector(3, 3)
    qi.apply(out2, 0xFFFF_FFFF, v)
    # 0xFFFFFFFF mod 3 == 0, so result is the zero vector.
    assert list(out2) == [0, 0, 0]

    out3 = fp.FpVector(3, 3)
    qi.apply(out3, 2 + 3 * 10, v)  # coeff = 32 ≡ 2 mod 3
    assert list(out3) == [2, 1, 2]


def test_none_image_construction_and_roundtrip():
    preimage = fp.Matrix.from_vec(
        2,
        [
            [1, 0, 0],
            [0, 1, 0],
            [0, 0, 1],
        ],
    )
    qi = fp.QuasiInverse(None, preimage)
    assert qi.pivots is None
    # With a None (identity) image, target_dimension == image_dimension.
    assert qi.image_dimension == 3
    assert qi.source_dimension == 3
    assert qi.target_dimension == 3

    v = fp.FpVector.from_slice(2, [1, 0, 1])
    out = fp.FpVector(2, 3)
    qi.apply(out, 1, v)
    assert list(out) == [1, 0, 1]

    # A None image is serialized as an explicit identity pivot list and so
    # round-trips to Some([0, 1, 2, ...]) rather than None.
    restored = fp.QuasiInverse.from_bytes(2, qi.to_bytes())
    assert restored.pivots == [0, 1, 2]
    assert restored.target_dimension == 3
    out2 = fp.FpVector(2, 3)
    restored.apply(out2, 1, v)
    assert list(out2) == [1, 0, 1]
