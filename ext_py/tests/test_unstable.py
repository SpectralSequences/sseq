"""Tests for the unstable (``U = true``) family: the ``UnstableResolution``
pyclass, the ``construct_unstable`` pyfunction, and the pure-Python
``query_unstable_module`` / ``query_unstable_module_only`` I/O helpers.

The unstable construct path monomorphises upstream ``construct_standard::<true,
_, _>`` (general algorithm only -- there is no Nassau analogue). The smallest
meaningful unstable resolution here is the (unsuspended) sphere ``S_2``, mirror-
ing ``examples/resolve_unstable.py`` and ``ext/examples/resolve_unstable.rs``.

Unstable Ext differs from stable Ext, so we assert structural invariants plus a
derived known invariant (the Adem/Milnor charts must agree -- the upstream
``ext/tests/milnor_vs_adem.rs::compare_unstable`` invariant), not the stable
``h_i`` pattern.
"""

import itertools

import pytest

import ext
from ext import _query, sseq


def _ns(n, s):
    return sseq.Bidegree.n_s(n, s)


def _st(s, t):
    return sseq.Bidegree.s_t(s, t)


def _unstable_s2(spec="S_2", n=8, s=4):
    r = ext.construct_unstable(spec)
    r.compute_through_stem(_ns(n, s))
    return r


# --- UnstableResolution: construction + structural invariants ------------


def test_unstable_resolution_basic_invariants():
    r = _unstable_s2()
    assert r.prime() == 2
    assert r.min_degree() == 0
    assert r.next_homological_degree() > 0
    # The unit: exactly one generator at (0, 0).
    assert r.number_of_gens_in_bidegree(_st(0, 0)) == 1
    # graded_dimension_string is nonempty/consistent.
    assert r.graded_dimension_string().strip() != ""


def test_unstable_resolution_constructor_matches_pyfunction():
    # The UnstableResolution(spec) constructor and the construct_unstable
    # pyfunction agree on the chart.
    a = ext.UnstableResolution("S_2")
    a.compute_through_stem(_ns(6, 3))
    b = ext.construct_unstable("S_2")
    b.compute_through_stem(_ns(6, 3))
    assert a.graded_dimension_string() == b.graded_dimension_string()


def _nonblank_glyphs(chart):
    # Upstream renders an all-zero bidegree as a space (unicode_num(0) == ' ')
    # and separates entries with spaces/newlines, so the count of non-whitespace
    # characters is the number of populated bidegrees ("dots") on the chart.
    return sum(1 for c in chart if not c.isspace())


def test_unstable_adem_milnor_charts_agree():
    # Derived known invariant: the unstable Ext chart is independent of the
    # Steenrod-algebra basis (cf. ext/tests/milnor_vs_adem.rs::compare_unstable,
    # which uses *suspended* spheres precisely to get non-trivial charts).
    #
    # The unsuspended S_2 unstable chart has only the single (0,0) unit glyph, so
    # its Adem-vs-Milnor agreement is near-vacuous. The suspension S_2[5] has a
    # genuinely non-trivial chart, so we also assert it has more than one glyph.
    a = _unstable_s2("S_2[5]@adem")
    b = _unstable_s2("S_2[5]@milnor")
    chart_a = a.graded_dimension_string()
    chart_b = b.graded_dimension_string()
    assert chart_a == chart_b
    assert _nonblank_glyphs(chart_a) > 1, (
        f"suspended-sphere unstable chart should be non-trivial, got:\n{chart_a}"
    )


def test_unstable_vs_stable_s3_charts_differ():
    # THE core semantic proof that the unstable binding produces genuinely
    # unstable data and is not silently the stable resolution: resolve the
    # 3-sphere S^3 = S_2[3] BOTH stably and unstably over the same range and
    # assert the charts DIFFER.
    #
    # We use the *standard* stable backend (not the default Nassau backend,
    # which renormalises min_degree to 0): standard keeps min_degree == 3, the
    # same coordinate frame as the unstable resolution, so the comparison is a
    # genuine apples-to-apples per-bidegree contrast rather than a coordinate
    # artifact.
    rng = _ns(10, 6)
    stable = ext.construct("S_2[3]", algorithm="standard")
    stable.compute_through_stem(rng)
    unstable = ext.construct_unstable("S_2[3]")
    unstable.compute_through_stem(rng)

    assert stable.min_degree() == 3
    assert unstable.min_degree() == 3

    chart_stable = stable.graded_dimension_string()
    chart_unstable = unstable.graded_dimension_string()
    assert chart_stable != chart_unstable, (
        "unstable S^3 chart must differ from the stable S^3 chart:\n"
        f"STABLE:\n{chart_stable}\nUNSTABLE:\n{chart_unstable}"
    )

    # Document the first divergence: the unstable resolution truncates/kills a
    # class that survives stably. In these (shared) coordinates the first such
    # bidegree is (n=6, s=1): the stable resolution has one generator there, the
    # unstable one has none (the unstable conditions kill it).
    assert stable.number_of_gens_in_bidegree(_ns(6, 1)) == 1
    assert unstable.number_of_gens_in_bidegree(_ns(6, 1)) == 0

    # Sanity: both charts are non-trivial (so "differ" is not a trivial
    # empty-vs-empty difference), and the unstable resolution exposes genuine
    # unstable generator data at more than one bidegree.
    populated_unstable = sum(
        1
        for n in range(0, 11)
        for s in range(0, 7)
        if unstable.number_of_gens_in_bidegree(_ns(n, s)) > 0
    )
    assert populated_unstable > 1, "unstable S^3 chart should be non-trivial"


def test_unstable_to_sseq_returns_sseq():
    r = _unstable_s2()
    ss = r.to_unstable_sseq()
    assert isinstance(ss, sseq.Sseq)
    assert ss.prime() == 2


def test_unstable_filtration_one_products():
    # The unstable analogue of test_filtration_one_products_h0: the degree-1
    # operation Sq^1 induces the filtration-one product living in (n, s) = (0, 1).
    r = _unstable_s2()
    prod = r.filtration_one_products(1, 0)
    assert isinstance(prod, sseq.Product)
    assert prod.b.s == 1
    assert prod.b.n == 0
    # Negative op_deg is a ValueError (mirrors the stable binding).
    with pytest.raises(ValueError):
        r.filtration_one_products(-1, 0)
    # op_deg = 1 has a single operation (Sq^1) at p = 2, so op_idx = 9999 is out
    # of range and must raise IndexError, not panic.
    with pytest.raises(IndexError):
        r.filtration_one_products(1, 9999)


def test_unstable_iter_nonzero_stem_islice():
    r = _unstable_s2()
    # The iterator is bounded; islice a few entries. Every yielded bidegree
    # must be nonzero.
    seen = list(itertools.islice(r.iter_nonzero_stem(), 6))
    assert seen, "expected at least one nonzero unstable bidegree"
    for b in seen:
        assert r.number_of_gens_in_bidegree(b) > 0
    # The unit (n=0, s=0) is among the nonzero entries.
    assert any(b.n == 0 and b.s == 0 for b in seen)


def test_unstable_iter_stem_terminates():
    r = _unstable_s2(n=4, s=2)
    # Bounded by the resolved range, so list() terminates.
    allb = list(r.iter_stem())
    assert len(allb) > 0


# --- Panic guards: no panic across FFI -----------------------------------


def test_unstable_negative_bidegree_raises_value_error():
    r = _unstable_s2()
    with pytest.raises(ValueError):
        r.number_of_gens_in_bidegree(_st(-1, 0))
    with pytest.raises(ValueError):
        r.number_of_gens_in_bidegree(_st(0, -1))
    with pytest.raises(ValueError):
        r.compute_through_stem(_st(-1, 0))
    with pytest.raises(ValueError):
        r.compute_through_bidegree(_st(0, -1))
    with pytest.raises(ValueError):
        r.has_computed_bidegree(_st(-1, 0))


def test_unstable_out_of_range_bidegree_is_zero_no_panic():
    r = _unstable_s2()
    # Far outside the computed range -> 0, never a panic.
    assert r.number_of_gens_in_bidegree(_st(100, 100)) == 0
    assert r.number_of_gens_in_bidegree(_st(1, 10_000)) == 0
    # Uncomputed (but in-axis) bidegree is reported as not computed.
    assert r.has_computed_bidegree(_st(0, 0)) is True


def test_unstable_boundary_string_guards():
    r = _unstable_s2()
    # The unit generator at (0,0,0) is valid.
    assert isinstance(
        r.boundary_string(sseq.BidegreeGenerator.s_t(0, 0, 0)), str
    )
    # Out-of-range generator idx / negative -> ValueError, no panic.
    with pytest.raises(ValueError):
        r.boundary_string(sseq.BidegreeGenerator.s_t(0, 0, 9))
    with pytest.raises(ValueError):
        r.boundary_string(sseq.BidegreeGenerator.s_t(-1, 0, 0))


def test_unstable_construct_bad_spec_raises_value_error():
    with pytest.raises(ValueError):
        ext.construct_unstable("definitely_not_a_module")


@pytest.mark.parametrize("spec", ["C9", "C4"])
def test_unstable_construct_cofiber_spec_raises_value_error(spec):
    # A cofiber-bearing spec is stable-only: upstream construct_standard::<true>
    # builds the algebra/module then trips assert!(!U, "Cofiber not supported for
    # unstable resolution"). The binding must contain that panic and surface a
    # clean ValueError (NOT a PanicException, which is a BaseException) on both
    # construct_unstable and the UnstableResolution(spec) constructor.
    with pytest.raises(ValueError, match="cofiber"):
        ext.construct_unstable(spec)
    with pytest.raises(ValueError, match="cofiber"):
        ext.UnstableResolution(spec)


def test_unstable_construct_save_dir_is_file_raises(tmp_path):
    f = tmp_path / "afile"
    f.write_text("not a directory")
    with pytest.raises(ValueError):
        ext.construct_unstable("S_2", save_dir=str(f))


def test_unstable_construct_save_dir_round_trip(tmp_path):
    save = str(tmp_path)
    r1 = ext.construct_unstable("S_2", save_dir=save)
    r1.compute_through_stem(_ns(6, 3))
    chart1 = r1.graded_dimension_string()
    assert any(p.is_file() for p in tmp_path.rglob("*")), "expected save files"

    r2 = ext.construct_unstable("S_2", save_dir=save)
    r2.compute_through_stem(_ns(6, 3))
    assert r2.graded_dimension_string() == chart1


# --- query_unstable_module* (pure-Python I/O) ----------------------------


@pytest.fixture
def feed(monkeypatch):
    def _feed(answers):
        _query._reset_args(list(answers))

    yield _feed
    _query._reset_args()


def test_query_unstable_module_only_builds_sphere(feed):
    # Answers: module spec, then (empty) save directory.
    feed(["S_2", ""])
    res = ext.query_unstable_module_only("Module")
    res.compute_through_stem(_ns(4, 2))
    assert res.number_of_gens_in_bidegree(_st(0, 0)) == 1


def test_query_unstable_module_only_explicit_save_dir_skips_prompt(feed, tmp_path):
    # Only the module spec is consumed; save_dir supplied -> no save-dir prompt.
    feed(["S_2"])
    res = ext.query_unstable_module_only("Module", save_dir=str(tmp_path))
    res.compute_through_stem(_ns(4, 2))
    assert res.number_of_gens_in_bidegree(_st(0, 0)) == 1


def test_query_unstable_module_resolves_through_stem(feed):
    # module spec, save dir (empty), Max n, Max s.
    feed(["S_2", "", "6", "3"])
    res = ext.query_unstable_module()
    assert res.number_of_gens_in_bidegree(_st(0, 0)) == 1
    # Resolved through the requested stem.
    assert res.next_homological_degree() > 0


def test_query_unstable_module_save_dir_round_trip(feed, tmp_path):
    feed(["S_2", str(tmp_path), "6", "3"])
    res = ext.query_unstable_module()
    chart = res.graded_dimension_string()
    assert any(p.is_file() for p in tmp_path.rglob("*")), "expected save files"

    # Fresh build from the same directory loads the saved data.
    feed(["S_2", str(tmp_path), "6", "3"])
    res2 = ext.query_unstable_module()
    assert res2.graded_dimension_string() == chart
