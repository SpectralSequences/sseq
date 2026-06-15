"""Smoke tests for the example scripts."""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

import sseq_ext as ext

EXAMPLES = Path(__file__).parent.parent / "examples"


def run_example(name: str, *args: str, stdin: str = "") -> str:
    """Run `examples/<name>.py` with the given CLI args and stdin."""
    result = subprocess.run(
        [sys.executable, str(EXAMPLES / f"{name}.py"), *args],
        input=stdin,
        capture_output=True,
        text=True,
        check=True,
    )
    return result.stdout


def test_resolve_runs():
    out = run_example("resolve", "S_2", "8", "4")
    # 4 lines of dots with at least one '·'
    lines = [line for line in out.splitlines() if line.strip()]
    assert any("·" in ln for ln in lines)


def test_num_gens_runs():
    out = run_example("num_gens", "S_2", "8", "4")
    # parse n,s,num_gens
    rows = [ln.split(",") for ln in out.splitlines() if ln]
    # we should see x_(0,0) = 1 (the unit class)
    assert any(int(s) == 0 and int(n) == 0 and int(num) == 1
               for n, s, num in rows)


def test_chart_runs():
    # Should produce a valid SVG.
    out = run_example("chart", "S_2", "8", "4")
    assert out.startswith("<svg")
    assert out.rstrip().endswith("</svg>")


def test_secondary_runs():
    out = run_example("secondary", "S_2", "8", "6")
    # `secondary` produces lines like `d_2 x_(...) = [...]`. Pin the exact
    # set of d_2 values in this range so a regression that returns garbage
    # (or stops computing) fails rather than silently passing.
    d2 = dict(re.findall(r"d_2 (x_\([^)]*\)) = (\[[^\]]*\])", out))
    assert d2 == {
        "x_(1, 1, 0)": "[0]",
        "x_(8, 2, 0)": "[0]",
    }


def test_massey_h0_h0_runs():
    # <h0, h0, ->: h0 in (0, 1), [1]
    stdin = "0\n1\n[1]\n0\n1\n[1]\n"
    out = run_example("massey", "S_2", "10", "5", stdin=stdin)
    # Pin the exact Massey products computed in this range.
    products = dict(re.findall(r"<a, b, (x_\([^)]*\))> = (\[[^\]]*\])", out))
    assert products == {
        "x_(1, 1, 0)": "[0]",
        "x_(2, 2, 0)": "[0]",
        "x_(6, 2, 0)": "[0]",
        "x_(8, 2, 0)": "[0]",
        "x_(8, 3, 0)": "[0]",
    }


def test_in_process_basic_resolve():
    """Same as test_resolve_runs but in-process so we exercise the API."""
    res = ext.construct("S_2")
    res.compute_through_bidegree(ext.Bidegree.s_t(4, 8))
    out = res.graded_dimension_string()
    assert "·" in out


def test_algebra_dim_runs():
    out = run_example("algebra_dim", "2", "10")
    lines = out.splitlines()
    # Mod-2 Steenrod algebra dimensions for n = 0..=10:
    #   1, 1, 1, 2, 2, 2, 3, 4, 4, 5, 6
    expected = [1, 1, 1, 2, 2, 2, 3, 4, 4, 5, 6]
    assert len(lines) == 11
    for n, want in enumerate(expected):
        assert lines[n] == f"dim A_{n} = {want}"


def test_resolve_through_stem_runs():
    out = run_example("resolve_through_stem", "S_2", "8", "4")
    lines = [ln for ln in out.splitlines() if ln.strip()]
    assert any("·" in ln for ln in lines)


def test_filtration_one_runs():
    out = run_example("filtration_one", "S_2", "8", "4")
    lines = [ln for ln in out.splitlines() if ln.strip()]
    # Every line is of the form `h_i x_(...) = [...]`.
    assert all(ln.startswith("h_") for ln in lines)
    # h_0 x_(0,0,0) = [1] is the canonical first entry.
    assert "h_0 x_(0, 0, 0) = [1]" in lines


def test_differentials_runs():
    out = run_example("differentials", "S_2", "8", "4")
    # Every line should be of the form `d x_(...) = ...`
    lines = [ln for ln in out.splitlines() if ln.strip()]
    assert all(ln.startswith("d x_(") for ln in lines)
    # The first generator d x_(0,0,0) is in degree 0 and is killed by the
    # augmentation, so its boundary is 0.
    assert "d x_(0,0,0) = 0" in lines


def test_resolution_size_runs():
    out = run_example("resolution_size", "S_2", "8", "4")
    # Output is one line per homological degree, comma-separated dims.
    lines = [ln for ln in out.splitlines() if ln.strip()]
    assert len(lines) == 5  # s = 4, 3, 2, 1, 0
    # Each line is dimensions of F_s; the last (s=0) is just dim of S_2 in
    # successive degrees. dim S_2(0) = 1.
    last = lines[-1].split(", ")
    assert last[0] == "1"
