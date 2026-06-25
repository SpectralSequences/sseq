"""Tests for the pure-Python I/O util layer (``ext._query`` / ``ext.utils``)
and the ``ext.construct`` pyfunction with ``save_dir`` support.

The interactive ``query_module*`` helpers consume a module-level argument stream
(``ext._query._args``, built from ``sys.argv[1:]`` at import). We drive them
deterministically by monkeypatching that stream via the ``_reset_args`` hook,
which feeds a fixed answer sequence in the same left-to-right order the Rust
``query`` crate consumes ``std::env::args()``.
"""

import pytest

import ext
from ext import _query


@pytest.fixture
def feed(monkeypatch):
    """Return a callable that loads a deterministic answer sequence into the
    ``_query`` argument stream (so no prompt ever reads stdin)."""

    def _feed(answers):
        _query._reset_args(list(answers))

    yield _feed
    # Restore the real argv-derived stream so we don't leak state across tests.
    _query._reset_args()


def _bidegree(n, s):
    return ext.sseq.Bidegree.n_s(n, s)


# --- query_module_only / query_module (Python I/O) -----------------------


def test_query_module_only_builds_sphere(feed):
    # Answers: module spec, then (empty) save directory.
    feed(["S_2", ""])
    res = ext.query_module_only("Module")
    res.compute_through_bidegree(ext.sseq.Bidegree.s_t(0, 0))
    assert res.number_of_gens_in_bidegree(ext.sseq.Bidegree.s_t(0, 0)) == 1


def test_query_module_only_explicit_save_dir_skips_prompt(feed, tmp_path):
    # Only the module spec is consumed; save_dir is supplied, so NO save-dir
    # prompt is read (if it were, the stream would be exhausted -> EOF exit).
    feed(["S_2"])
    res = ext.query_module_only("Module", save_dir=str(tmp_path))
    res.compute_through_bidegree(ext.sseq.Bidegree.s_t(0, 0))
    assert res.number_of_gens_in_bidegree(ext.sseq.Bidegree.s_t(0, 0)) == 1


def test_query_module_resolves_through_stem(feed):
    # module spec, save dir (empty), Max n, Max s.
    feed(["S_2", "", "8", "4"])
    res = ext.query_module()
    # Standard low-dimensional Ext of the sphere.
    assert res.number_of_gens_in_bidegree(_bidegree(0, 0)) == 1
    assert res.number_of_gens_in_bidegree(_bidegree(0, 1)) == 1  # h_0 at (1,1)
    assert res.number_of_gens_in_bidegree(_bidegree(1, 1)) == 1  # h_1 at (1,2)


def test_query_module_secondary_job_caps_max_s(feed, monkeypatch):
    monkeypatch.setenv("SECONDARY_JOB", "2")
    feed(["S_2", "", "8", "7"])
    res = ext.query_module()
    # max_s is capped to min(2+1, 7) = 3, so s=4 must be unresolved -> 0.
    assert res.number_of_gens_in_bidegree(_bidegree(0, 0)) == 1
    assert res.number_of_gens_in_bidegree(_bidegree(0, 5)) == 0


def test_query_module_secondary_job_too_large_raises(feed, monkeypatch):
    monkeypatch.setenv("SECONDARY_JOB", "10")
    feed(["S_2", "", "8", "7"])
    with pytest.raises(ValueError):
        ext.query_module()


# --- construct + save_dir round-trip -------------------------------------


def test_construct_save_dir_round_trip(tmp_path):
    save = str(tmp_path)
    r1 = ext.construct("S_2", save_dir=save)
    r1.compute_through_stem(_bidegree(8, 4))
    chart1 = r1.graded_dimension_string()

    # Save files were written.
    written = list(tmp_path.rglob("*"))
    assert any(p.is_file() for p in written), "expected save files under tmp_path"

    # A fresh construct from the SAME directory loads the saved data.
    r2 = ext.construct("S_2", save_dir=save)
    r2.compute_through_stem(_bidegree(8, 4))
    assert r2.graded_dimension_string() == chart1


# --- construct error taxonomy / algorithm --------------------------------


def test_construct_bad_spec_raises_value_error():
    with pytest.raises(ValueError):
        ext.construct("definitely_not_a_module")


def test_construct_nassau_eligible_module():
    r = ext.construct("S_2", algorithm="nassau")
    r.compute_through_bidegree(ext.sseq.Bidegree.s_t(0, 0))
    assert r.number_of_gens_in_bidegree(ext.sseq.Bidegree.s_t(0, 0)) == 1


def test_construct_bad_algorithm_raises_value_error():
    with pytest.raises(ValueError):
        ext.construct("S_2", algorithm="bogus")


# --- import-surface regression -------------------------------------------


def test_import_surface_intact():
    import ext as _e

    assert _e.Resolution is not None
    from ext import algebra, fp, sseq  # noqa: F401

    import ext.ext as _compiled

    assert _compiled is _e.ext

    # The Python utils shadow the Rust pyfunctions at package level...
    assert _e.query_module.__module__ == "ext.utils"
    assert _e.query_module_only.__module__ == "ext.utils"
    # ...while the Rust pyfunctions remain reachable on the compiled submodule.
    assert callable(_compiled.query_module)
    assert callable(_compiled.query_module_only)
    # construct is the compiled (Rust) pyfunction.
    assert callable(_e.construct)


def test_query_primitives_exposed():
    for name in ("raw", "with_default", "optional", "yes_no", "vector"):
        assert hasattr(ext, name)


# --- non-IO module utils: unicode_num / LAMBDA_BIDEGREE / parse_module_name /
#     load_module_json / get_unit -------------------------------------------


def test_unicode_num_exact_output():
    # Byte-for-byte match with ext::utils::unicode_num (ext/src/utils.rs).
    assert ext.unicode_num(0) == " "
    assert ext.unicode_num(1) == "·"
    assert ext.unicode_num(2) == ":"
    assert ext.unicode_num(3) == "∴"
    assert ext.unicode_num(4) == "⁘"
    assert ext.unicode_num(5) == "⁙"
    assert ext.unicode_num(6) == "⠿"
    assert ext.unicode_num(7) == "⡿"
    assert ext.unicode_num(8) == "⣿"
    assert ext.unicode_num(9) == "9"
    # Anything >= 10 collapses to '*'.
    assert ext.unicode_num(10) == "*"
    assert ext.unicode_num(123) == "*"


def test_lambda_bidegree_value():
    # ext::secondary::LAMBDA_BIDEGREE == Bidegree::n_s(0, 1) -> n=0, s=1, t=1.
    b = ext.LAMBDA_BIDEGREE
    assert isinstance(b, ext.sseq.Bidegree)
    assert b.n == 0
    assert b.s == 1
    assert b.t == 1
    # The compiled getter agrees with the package-level constant value.
    assert ext.lambda_bidegree().coords == b.coords


def test_parse_module_name_valid():
    d = ext.parse_module_name("S_2")
    assert isinstance(d, dict)
    assert d["p"] == 2
    assert "type" in d
    assert "gens" in d


def test_parse_module_name_with_shift():
    d = ext.parse_module_name("S_2[3]")
    assert isinstance(d, dict)
    assert d["shift"] == 3


def test_parse_module_name_bad_name_raises():
    with pytest.raises(ValueError):
        ext.parse_module_name("definitely_not_a_module")


def test_load_module_json_valid():
    d = ext.load_module_json("S_2")
    assert isinstance(d, dict)
    assert d["p"] == 2


def test_load_module_json_unknown_name_raises():
    with pytest.raises(ValueError):
        ext.load_module_json("definitely_not_a_module")


def test_get_unit_round_trip():
    res = ext.construct("S_2", algorithm="standard")
    res.compute_through_stem(_bidegree(8, 4))
    is_unit, unit = ext.get_unit(res)
    # S_2 IS the unit, so upstream returns (True, the same resolution).
    assert is_unit is True
    assert isinstance(unit, ext.Resolution)
    assert unit.prime() == 2
