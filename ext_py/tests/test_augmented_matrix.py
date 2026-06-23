import pytest

from ext import fp


def test_import_both_classes():
    assert hasattr(fp, "AugmentedMatrix2")
    assert hasattr(fp, "AugmentedMatrix3")


def test_construction_and_queries():
    m = fp.AugmentedMatrix2(3, 2, [2, 2])
    # Prime is passed/queried as a plain int.
    assert m.prime() == 3
    assert m.rows() == 2
    assert m.segments() == 2
    assert m.is_zero()
    # The first segment starts at column 0; the matrix has as many columns as
    # the final segment's end.
    starts = m.segment_starts()
    ends = m.segment_ends()
    assert starts[0] == 0
    assert ends[-1] == m.columns()
    assert len(starts) == 2
    assert len(ends) == 2

    m3 = fp.AugmentedMatrix3(3, 2, [2, 2, 2])
    assert m3.segments() == 3
    assert len(m3.segment_starts()) == 3


def test_invalid_segment_widths_raise():
    # Wrong number of segment widths.
    with pytest.raises(ValueError):
        fp.AugmentedMatrix2(3, 2, [2])
    with pytest.raises(ValueError):
        fp.AugmentedMatrix2(3, 2, [2, 2, 2])
    with pytest.raises(ValueError):
        fp.AugmentedMatrix3(3, 2, [2, 2])
    # Invalid prime.
    with pytest.raises(ValueError):
        fp.AugmentedMatrix2(4, 2, [2, 2])


def test_add_identity_and_invalid_segments():
    m = fp.AugmentedMatrix2(2, 2, [2, 2])
    m.add_identity(1, 1)
    start1 = m.segment_starts()[1]
    rows = m.to_vec()
    assert rows[0][start1] == 1
    assert rows[1][start1 + 1] == 1

    # Out-of-range segment indices raise IndexError.
    with pytest.raises(IndexError):
        m.add_identity(0, 2)
    with pytest.raises(IndexError):
        m.row_segment(0, 0, 2)

    # Non-square segment (2 rows, width-3 segment) raises ValueError.
    wide = fp.AugmentedMatrix2(2, 2, [3, 3])
    with pytest.raises(ValueError):
        wide.add_identity(0, 0)


def test_row_segment_returns_owned_fpvector():
    m = fp.AugmentedMatrix2(2, 2, [2, 2])
    m.add_identity(1, 1)
    seg = m.row_segment(0, 1, 1)
    assert isinstance(seg, fp.FpVector)
    assert len(seg) == 2
    assert seg[0] == 1
    assert seg[1] == 0


def test_into_matrix_returns_matrix():
    m = fp.AugmentedMatrix2(2, 2, [2, 2])
    m.add_identity(1, 1)
    inner = m.into_matrix()
    assert isinstance(inner, fp.Matrix)
    assert inner.rows() == 2
    assert inner.columns() == m.columns()
    # The augmented matrix is still usable (into_matrix clones the inner matrix).
    assert m.rows() == 2


def test_augmented_matrix2_compute_image_and_quasi_inverse():
    # [I | I] over F2: A is the 2x2 identity, the second block is the identity.
    m = fp.AugmentedMatrix2(2, 2, [2, 2])
    m.add_identity(0, 0)
    m.add_identity(1, 1)
    m.row_reduce()

    image = m.compute_image()
    assert isinstance(image, fp.Subspace)
    assert image.prime() == 2
    assert image.dimension() == 2

    qi = m.compute_quasi_inverse()
    assert isinstance(qi, fp.QuasiInverse)
    assert qi.prime() == 2
    assert qi.source_dimension() == 2
    # A is the identity, so its quasi-inverse preimage is the identity too.
    assert qi.preimage().to_vec() == [[1, 0], [0, 1]]

    kernel = m.compute_kernel()
    assert isinstance(kernel, fp.Subspace)
    # A is full rank, so the kernel is trivial.
    assert kernel.dimension() == 0


def test_augmented_matrix3_compute_kernel_and_quasi_inverses():
    # [A | B | I] with all square identity blocks over F3.
    m = fp.AugmentedMatrix3(3, 2, [2, 2, 2])
    m.add_identity(0, 0)
    m.add_identity(1, 1)
    m.add_identity(2, 2)
    m.row_reduce()

    kernel = m.compute_kernel()
    assert isinstance(kernel, fp.Subspace)
    assert kernel.prime() == 3

    a, b = m.compute_quasi_inverses()
    assert isinstance(a, fp.QuasiInverse)
    assert isinstance(b, fp.QuasiInverse)
    assert a.prime() == 3
    assert b.prime() == 3
    # A = I is full-rank 2->2, so its quasi-inverse maps F3^2 -> F3^2.
    assert a.source_dimension() == 2
    assert a.target_dimension() == 2
    # The residual quasi-inverse inverts B (= I) and is itself a full-rank
    # 2->2 map.
    assert b.source_dimension() == 2
    assert b.target_dimension() == 2


def test_compute_methods_require_row_reduce():
    # Reproduction from the review: calling compute_* on a freshly constructed
    # (not row-reduced) augmented matrix must raise ValueError, not panic.
    m2 = fp.AugmentedMatrix2(2, 2, [2, 2])
    with pytest.raises(ValueError):
        m2.compute_kernel()
    with pytest.raises(ValueError):
        m2.compute_image()
    with pytest.raises(ValueError):
        m2.compute_quasi_inverse()

    m3 = fp.AugmentedMatrix3(3, 2, [2, 2, 2])
    with pytest.raises(ValueError):
        m3.compute_kernel()
    with pytest.raises(ValueError):
        m3.compute_quasi_inverses()


def test_repr():
    m = fp.AugmentedMatrix2(3, 2, [2, 2])
    assert repr(m).startswith("AugmentedMatrix2(3, 2x")
    m3 = fp.AugmentedMatrix3(2, 1, [1, 1, 1])
    assert repr(m3).startswith("AugmentedMatrix3(2, 1x")
