"""Smoke tests for the example scripts."""

from __future__ import annotations

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
    # `secondary` produces lines like `d_2 x_(...) = [...]`
    assert "d_2 x_" in out


def test_massey_h0_h0_runs():
    # <h0, h0, ->: h0 in (0, 1), [1]
    stdin = "0\n1\n[1]\n0\n1\n[1]\n"
    out = run_example("massey", "S_2", "10", "5", stdin=stdin)
    # Massey product output starts with "<a, b, ..."
    assert "<a, b," in out


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


def test_resolution_size_runs():
    out = run_example("resolution_size", "S_2", "8", "4")
    # Output is one line per homological degree, comma-separated dims.
    lines = [ln for ln in out.splitlines() if ln.strip()]
    assert len(lines) == 5  # s = 4, 3, 2, 1, 0
    # Each line is dimensions of F_s; the last (s=0) is just dim of S_2 in
    # successive degrees. dim S_2(0) = 1.
    last = lines[-1].split(", ")
    assert last[0] == "1"
