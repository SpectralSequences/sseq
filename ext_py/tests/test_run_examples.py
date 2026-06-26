"""Execute every ``examples/*.py`` script and record which actually run.

Unlike :mod:`tests.test_examples` (which only statically checks that examples
reference bound symbols), this module *runs* each example in a subprocess and
asserts it exits 0.

Interactive input
-----------------
The examples drive all of their prompts through :mod:`ext._query`, which
consumes answers from ``sys.argv[1:]`` first and only falls back to stdin when
the argument stream is exhausted (see ``ext/python/ext/_query.py``). We therefore
feed each script a curated answer sequence as command-line arguments, in the
exact left-to-right order the prompts fire, and connect stdin to ``/dev/null`` so
that a script asking for more input than we supplied fails fast (EOF) instead of
hanging. Parameters are kept small so the resolutions are cheap.

Each script runs in its own temporary working directory because a few of them
(``d2_charts.py``, ``save_bruner.py``) write output files relative to the cwd.

Expected failures
-----------------
Most ports were written against the aspirational API in ``API_PROPOSAL.md`` and
do not yet run end-to-end against the current bindings -- either because a
binding is still unbound, or because the default resolution backend (Nassau)
does not expose a method the example calls, or because of a genuine
example/binding mismatch. Each such script is marked ``xfail(strict=True)`` with
the observed reason, so that if a future binding change makes one start passing
(or breaks one of the currently-passing scripts) the suite fails loudly and
forces a conscious update to this table.
"""

import pathlib
import subprocess
import sys

import pytest

EXAMPLES_DIR = pathlib.Path(__file__).resolve().parent.parent / "examples"

# Per-example invocation table.
#   name:   the script filename in examples/
#   args:   answers fed as argv (consumed by ext._query in prompt order)
#   xfail:  None if the script is expected to exit 0; otherwise a short reason
#           string describing why it currently fails.
#
# Common answer prologue meanings:
#   "S_2"  -> module spec (the sphere at p=2)
#   ""     -> empty answer (e.g. an optional "save directory" -> None)
#   then small Max n / Max s (or Max t / Max s) bounds.
EXAMPLES = [
    # --- Run to completion against the current bindings ---
    {"name": "algebra_dim.py", "args": [], "xfail": None},
    {"name": "resolve.py", "args": ["S_2", "", "10", "5"], "xfail": None},
    {
        "name": "resolve_through_stem.py",
        "args": ["S_2", "", "8", "4"],
        "xfail": None,
    },
    # --- Genuine example/binding mismatches ---
    {"name": "num_gens.py", "args": ["S_2", "", "8", "4"], "xfail": None},
    {"name": "differentials.py", "args": ["S_2", "", "8", "4"], "xfail": None},
    {"name": "filtration_one.py", "args": ["S_2", "", "8", "4"], "xfail": None},
    {
        "name": "unstable_chart.py",
        "args": ["S_2", "", "6", "3", "M"],
        "xfail": None,
    },
    {
        # fd module, p=2, generators x0 (deg 0) and x1 (deg 1), finish (empty),
        # confirm (y), then the action Sq1 x0 = x1.
        "name": "define_module.py",
        "args": ["fd", "2", "0", "x0", "1", "x1", "", "y", "x1"],
        "xfail": None,
    },
    {
        "name": "lift_hom.py",
        "args": ["S_2", "", "4", "2", "S_2", "prod", "0", "0"],
        "xfail": "Resolution has no set_name()",
    },
    # --- Nassau (default) backend lacks methods the example needs ---
    {
        "name": "resolution_size.py",
        "args": ["S_2", "", "8", "4"],
        "xfail": None,
    },
    {"name": "chart.py", "args": ["S_2", "", "8", "4"], "xfail": None},
    {
        "name": "save_bruner.py",
        "args": ["S_2", "", "8", "4"],
        "xfail": None,
    },
    {
        "name": "ext_m_n.py",
        "args": ["S_2", "", "S_2", "6", "3"],
        "xfail": "HomPullback rejects HomModules built from the same target N "
        "(source.target()/target.target() are distinct Arcs)",
    },
    {
        "name": "sq0.py",
        "args": ["S_2", "", "8", "4"],
        "xfail": "Resolution.target() not bound; also needs DoubleChainComplex",
    },
    {
        "name": "steenrod.py",
        "args": ["S_2", "", "1", "1", "[1]"],
        "xfail": "Resolution.target() not bound; also needs TensorChainComplex",
    },
    {
        "name": "yoneda.py",
        "args": ["S_2", "", "0", "1", "[1]"],
        "xfail": None,
    },
    {
        "name": "secondary.py",
        "args": ["S_2", "", "8", "4"],
        "xfail": None,
    },
    {
        "name": "d2_charts.py",
        "args": ["S_2", "", "8", "4"],
        "xfail": None,
    },
    {
        "name": "secondary_product.py",
        "args": ["S_2", "", "8", "4", "prod", "0", "1", "[1]"],
        "xfail": "ResolutionHomomorphism.source is a method, not the bound resolution (no number_of_gens_in_bidegree)",
    },
    {
        "name": "massey.py",
        "args": ["S_2", "", "8", "4", "0", "1", "[1]", "0", "1", "[1]"],
        "xfail": "ResolutionHomomorphism.extend_step() is not bound",
    },
    {
        "name": "secondary_massey.py",
        "args": ["S_2", "", "8", "4", "0", "1", "a", "0", "1", "[1]",
                 "0", "1", "b", "0", "1", "[1]"],
        "xfail": "ResolutionHomomorphism.source is a method, not the bound resolution (no number_of_gens_in_bidegree)",
    },
    # --- Unbound bindings (aspirational API) ---
    {
        "name": "tensor.py",
        "args": ["S_2", "S_2"],
        "xfail": None,
    },
    {
        "name": "resolve_unstable.py",
        "args": ["S_2", "", "8", "4"],
        "xfail": "UnstableResolution.load_quasi_inverse is not a settable attribute",
    },
    {
        "name": "unstable_suspension.py",
        "args": ["S_2", "", "6", "3"],
        "xfail": None,
    },
    {
        "name": "bruner.py",
        "args": [],
        "xfail": "FiniteChainComplex is not bound (import fails)",
    },
    {
        "name": "mahowald_invariant.py",
        "args": ["", "", "8"],
        "xfail": None,
    },
]


def _runnable_example_files():
    """All examples/*.py except the ``_query`` shim (not a runnable example)."""
    return sorted(p for p in EXAMPLES_DIR.glob("*.py") if p.name != "_query.py")


def test_examples_table_covers_every_script():
    """Every runnable example must appear in EXAMPLES exactly once, so a newly
    added example cannot silently escape this runner."""
    on_disk = {p.name for p in _runnable_example_files()}
    in_table = [e["name"] for e in EXAMPLES]
    duplicates = {n for n in in_table if in_table.count(n) > 1}
    assert not duplicates, f"duplicate EXAMPLES entries: {sorted(duplicates)}"
    in_table_set = set(in_table)
    missing = on_disk - in_table_set
    stale = in_table_set - on_disk
    assert not missing, f"examples not covered by the runner: {sorted(missing)}"
    assert not stale, f"EXAMPLES entries with no script on disk: {sorted(stale)}"


def _params():
    return [
        pytest.param(
            e,
            id=e["name"],
            marks=(
                pytest.mark.xfail(reason=e["xfail"], strict=True)
                if e["xfail"]
                else ()
            ),
        )
        for e in EXAMPLES
    ]


@pytest.mark.parametrize("example", _params())
def test_example_runs(example, tmp_path):
    script = EXAMPLES_DIR / example["name"]
    result = subprocess.run(
        [sys.executable, str(script), *example["args"]],
        cwd=tmp_path,
        stdin=subprocess.DEVNULL,
        capture_output=True,
        text=True,
        timeout=120,
    )
    assert result.returncode == 0, (
        f"{example['name']} exited {result.returncode}\n"
        f"--- stderr ---\n{result.stderr[-2000:]}"
    )
