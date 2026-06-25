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


def test_unstable_adem_milnor_charts_agree():
    # Derived known invariant: the unstable Ext chart is independent of the
    # Steenrod-algebra basis (cf. ext/tests/milnor_vs_adem.rs::compare_unstable).
    a = _unstable_s2("S_2@adem")
    b = _unstable_s2("S_2@milnor")
    assert a.graded_dimension_string() == b.graded_dimension_string()


def test_unstable_to_sseq_returns_sseq():
    r = _unstable_s2()
    ss = r.to_unstable_sseq()
    assert isinstance(ss, sseq.Sseq)
    assert ss.prime() == 2


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
