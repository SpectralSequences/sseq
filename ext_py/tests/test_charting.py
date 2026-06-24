"""Tests for the §6.3 charting bindings in ``sseq``.

Covers the ``SvgBackend`` / ``TikzBackend`` backends (driven manually through
the flattened ``Backend`` methods), the ``Orientation`` enum, the
``PyFileWriter`` adapter's error propagation (a Python ``.write`` that raises
surfaces as a Python exception, never a panic), and the end-to-end
``Sseq.write_to_graph`` charting entry point.
"""

import io

import pytest

from ext import fp, sseq

Bidegree = sseq.Bidegree
BidegreeGenerator = sseq.BidegreeGenerator
BidegreeElement = sseq.BidegreeElement
FpVector = fp.FpVector
Matrix = fp.Matrix
SvgBackend = sseq.SvgBackend
TikzBackend = sseq.TikzBackend
Orientation = sseq.Orientation


def vec(p, entries):
    return FpVector.from_slice(p, entries)


def elem(b, p, entries):
    return BidegreeElement(b, vec(p, entries))


# --------------------------------------------------------------------------
# Orientation enum
# --------------------------------------------------------------------------


def test_orientation_variants_exist():
    variants = [Orientation.Left, Orientation.Right,
                Orientation.Above, Orientation.Below]
    # The four variants are distinct.
    for i, a in enumerate(variants):
        for j, b in enumerate(variants):
            assert (a == b) == (i == j)


# --------------------------------------------------------------------------
# SvgBackend: manual drawing
# --------------------------------------------------------------------------


def test_svg_backend_manual_drawing_stringio():
    buf = io.StringIO()
    g = SvgBackend(buf)
    g.header(Bidegree.x_y(4, 4))
    g.node(Bidegree.x_y(1, 1), 1)
    g.node(Bidegree.x_y(2, 2), 1)
    g.structline(BidegreeGenerator.n_s(1, 1, 0), BidegreeGenerator.n_s(2, 2, 0))
    g.line(Bidegree.x_y(0, 0), Bidegree.x_y(4, 0), "grid")
    g.text(Bidegree.x_y(0, 0), "0", Orientation.Below)
    # The closing </svg> is emitted when the backend is dropped.
    del g
    out = buf.getvalue()
    assert "<svg" in out
    assert "<circle" in out
    assert 'class="structline"' in out
    assert "</svg>" in out


def test_svg_backend_init_draws_grid():
    buf = io.StringIO()
    g = SvgBackend(buf)
    g.init(Bidegree.x_y(4, 4))
    del g
    out = buf.getvalue()
    assert "<svg" in out
    assert "major-grid" in out


def test_svg_backend_bytesio():
    # A binary file: the adapter falls back to writing bytes on a TypeError.
    buf = io.BytesIO()
    g = SvgBackend(buf)
    g.header(Bidegree.x_y(2, 2))
    g.node(Bidegree.x_y(1, 1), 1)
    del g
    out = buf.getvalue()
    assert b"<svg" in out
    assert b"</svg>" in out


def test_svg_backend_unsupported_orientation_raises():
    # SvgBackend only supports Left/Below; Right/Above are contained as a
    # RuntimeError (no panic crosses the FFI boundary).
    buf = io.StringIO()
    g = SvgBackend(buf)
    g.header(Bidegree.x_y(2, 2))
    with pytest.raises(RuntimeError):
        g.text(Bidegree.x_y(0, 0), "x", Orientation.Right)


def test_svg_backend_structline_matrix():
    buf = io.StringIO()
    g = SvgBackend(buf)
    g.header(Bidegree.x_y(3, 3))
    g.node(Bidegree.x_y(1, 1), 1)
    g.node(Bidegree.x_y(2, 2), 1)
    g.structline_matrix(Bidegree.x_y(1, 1), Bidegree.x_y(2, 2), [[1]], "d2")
    del g
    out = buf.getvalue()
    assert "d2" in out


# --------------------------------------------------------------------------
# TikzBackend: manual drawing
# --------------------------------------------------------------------------


def test_tikz_backend_manual_drawing():
    buf = io.StringIO()
    g = TikzBackend(buf)
    g.header(Bidegree.x_y(3, 3))
    g.node(Bidegree.x_y(1, 1), 1)
    # Unlike SvgBackend, TikzBackend supports all four orientations.
    g.text(Bidegree.x_y(0, 0), "lbl", Orientation.Above)
    del g
    out = buf.getvalue()
    assert r"\begin{tikzpicture}" in out
    assert r"\draw [fill]" in out
    assert r"\end{tikzpicture}" in out


def test_backend_ext_attrs():
    assert SvgBackend.EXT == "svg"
    assert TikzBackend.EXT == "tex"


# --------------------------------------------------------------------------
# PyFileWriter error propagation
# --------------------------------------------------------------------------


class RaisingFile:
    def write(self, _data):
        raise ValueError("boom from .write")


def test_write_exception_propagates_not_panic():
    g = SvgBackend(RaisingFile())
    with pytest.raises(ValueError, match="boom from .write"):
        g.header(Bidegree.x_y(2, 2))


def test_non_backend_argument_to_write_to_graph():
    s = sseq.Sseq(2)
    s.set_dimension(Bidegree.x_y(0, 0), 1)
    with pytest.raises(TypeError):
        s.write_to_graph(object(), 2, False, [], lambda _: None)


# --------------------------------------------------------------------------
# End-to-end: Sseq.write_to_graph
# --------------------------------------------------------------------------


def make_small_sseq():
    s = sseq.Sseq(2)
    s.set_dimension(Bidegree.x_y(0, 0), 0)
    s.set_dimension(Bidegree.x_y(1, 0), 2)
    s.set_dimension(Bidegree.x_y(0, 1), 0)
    s.set_dimension(Bidegree.x_y(0, 2), 2)
    s.add_differential(2, elem(Bidegree.x_y(1, 0), 2, [1, 0]), vec(2, [1, 0]))
    s.add_differential(2, elem(Bidegree.x_y(1, 0), 2, [0, 1]), vec(2, [1, 1]))
    s.update()
    return s


def test_write_to_graph_svg_no_products():
    s = make_small_sseq()
    buf = io.StringIO()
    s.write_to_graph(SvgBackend(buf), 2, False, [], lambda _: None)
    out = buf.getvalue()
    assert "<svg" in out
    assert "</svg>" in out
    # The E_2 page has classes at (1, 0) and (0, 2): nodes are drawn.
    assert "<circle" in out


def test_write_to_graph_svg_with_differentials():
    s = make_small_sseq()
    buf = io.StringIO()
    s.write_to_graph(SvgBackend(buf), 2, True, [], lambda _: None)
    out = buf.getvalue()
    # The d_2 differential out of (1, 0) is drawn as a structure line.
    assert 'class="structline d2"' in out


def test_write_to_graph_tikz_with_products():
    s = make_small_sseq()
    # A product living in (1, 1) multiplying (0, 0) -> (1, 1) (trivial here,
    # since (0, 0) is 0-dimensional) just exercises the products code path.
    prod = sseq.Product(
        Bidegree.x_y(1, 1), True, [(Bidegree.x_y(0, 0), Matrix.from_vec(2, [[1]]))]
    )
    buf = io.StringIO()
    s.write_to_graph(TikzBackend(buf), 2, False, [("h0", prod)], lambda _: None)
    out = buf.getvalue()
    assert r"\begin{tikzpicture}" in out
    assert r"\end{tikzpicture}" in out


def test_write_to_graph_consumes_backend():
    s = make_small_sseq()
    backend = SvgBackend(io.StringIO())
    s.write_to_graph(backend, 2, False, [], lambda _: None)
    # The backend was consumed; further manual use raises (no panic).
    with pytest.raises(RuntimeError):
        backend.header(Bidegree.x_y(2, 2))


def test_write_to_graph_header_callback_exception_propagates():
    s = make_small_sseq()

    def bad_header(_):
        raise ValueError("boom from header")

    with pytest.raises(ValueError, match="boom from header"):
        s.write_to_graph(SvgBackend(io.StringIO()), 2, False, [], bad_header)


# --------------------------------------------------------------------------
# Regression: a .write that fails ONLY on the closing tag must not be swallowed
# --------------------------------------------------------------------------


class CloseTagRaisingFile:
    """A text file-like that succeeds for every body write but raises on the
    single write containing the closing tag (``</svg>`` / ``\\end{tikzpicture}``).

    The closing tag is emitted by the backend's ``Drop``, which runs inside the
    upstream ``write_to_graph`` call (the backend is moved in by value). Before
    the fix, that error was recorded but never re-raised because the upstream
    result was ``Ok``, so ``write_to_graph`` returned success with truncated
    output. It must now propagate.
    """

    def __init__(self, closing_tag):
        self.closing_tag = closing_tag
        self.parts = []

    def write(self, data):
        text = data if isinstance(data, str) else data.decode()
        if self.closing_tag in text:
            raise ValueError("boom on closing tag")
        self.parts.append(text)
        return len(data)


def test_write_to_graph_svg_closing_tag_write_error_propagates():
    s = make_small_sseq()
    f = CloseTagRaisingFile("</svg>")
    with pytest.raises(ValueError, match="boom on closing tag"):
        s.write_to_graph(SvgBackend(f), 2, False, [], lambda _: None)
    # The body was written but the chart is truncated (no closing tag); the
    # point is that the failure is surfaced rather than silently swallowed.
    assert "".join(f.parts).startswith("<svg") or f.parts


def test_write_to_graph_tikz_closing_tag_write_error_propagates():
    s = make_small_sseq()
    f = CloseTagRaisingFile(r"\end{tikzpicture}")
    with pytest.raises(ValueError, match="boom on closing tag"):
        s.write_to_graph(TikzBackend(f), 2, False, [], lambda _: None)


# --------------------------------------------------------------------------
# SvgBackend.legend
# --------------------------------------------------------------------------


def test_svg_backend_legend_nonempty():
    buf = io.StringIO()
    SvgBackend.legend(buf)
    out = buf.getvalue()
    assert out  # nonempty
    assert "<svg" in out
    # The legend draws one bordered box per node pattern.
    assert '<rect fill="none" stroke="black"' in out
    assert "</svg>" in out
