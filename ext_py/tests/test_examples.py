"""Smoke test guarding examples/*.py against removed/renamed bindings.

These example scripts are interactive (they read from stdin) and/or run heavy
resolutions, so we do NOT execute them. Instead, we statically extract every
top-level module attribute access of the form ``<module>.<Name>`` (where
``<module>`` is one of the bound modules ``ext``/``algebra``/``fp``/
``sseq``) from each example via the ``ast`` module, and assert that every
referenced name is either actually bound on the built extension module, or is
explicitly listed in ``KNOWN_UNBOUND`` below.

This catches the class of breakage where a binding is renamed or removed (e.g.
``ext.FDModule`` -> ``ext.FDModuleBuilder``) but an example still
references the old name: such a name is neither bound nor allow-listed, so the
test fails loudly. Conversely it does not block on genuinely-aspirational APIs
that the examples were drafted against but which are not yet bound -- those are
documented in ``KNOWN_UNBOUND`` with a reason.

Note on namespaces: the example scripts frequently write ``ext.<Name>`` for
classes/functions that are actually bound on a *submodule* of ``ext``
(``algebra``/``fp``/``sseq``), because ``from .ext import *`` only
re-exports the submodules and the top-level free functions, not the nested
classes. We therefore resolve an ``ext.<Name>`` reference as bound if the
name exists on ``ext`` OR on any of its bound submodules. This keeps the
regression guard meaningful (a removed/renamed class is still caught everywhere)
without flagging the examples' namespace shorthand.
"""

import ast
import pathlib

import ext
from ext import algebra, fp, sseq

EXAMPLES_DIR = pathlib.Path(__file__).resolve().parent.parent / "examples"

# The modules that example scripts access attributes on.
MODULES = {
    "ext": ext,
    "algebra": algebra,
    "fp": fp,
    "sseq": sseq,
}

# Submodules reachable from ext; an ``ext.<Name>`` reference is satisfied
# if <Name> is bound on ext itself or on any of these.
EXT_PY_SUBMODULES = (algebra, fp, sseq)

# Names referenced by examples that are legitimately NOT bound yet: they were
# drafted against the aspirational API in API_PROPOSAL.md. Each entry must have
# a reason. This list is intentionally explicit so that if any of these later
# becomes bound (or a CURRENTLY-bound name is removed/renamed) the test changes
# behaviour and forces a conscious update.
KNOWN_UNBOUND = {
    # tensor.py: parses a module *name* (e.g. "S_2") into JSON; no such free
    # function is bound (only steenrod_module_from_json, which takes JSON).
    "parse_module_name",
    # tensor.py: FDModuleBuilder.from_tensor_module / .from_module conversion
    # constructor is documented as a planned thin wrapper but is not yet bound.
    "from_tensor_module",
    # unstable_chart.py: unstable resolution entry point is not bound.
    "query_unstable_module",
    # massey.py: Massey product computer is not bound.
    "MasseyProductComputer",
    # --- Resolution / construction entry points (not yet bound) ---
    # construct/construct_standard: top-level resolution constructors are not
    # bound (only the lower-level builder APIs are).
    "construct",
    "construct_standard",
    # get_unit: unit resolution accessor is not bound.
    "get_unit",
    # ResolutionHomomorphism and friends: lifting/secondary machinery not bound.
    # (ResolutionHomomorphism itself is now bound.)
    "SecondaryResolutionHomomorphism",
    "UnstableResolutionHomomorphism",
    # SecondaryChainHomotopy: secondary homotopy type not bound.
    # (ChainHomotopy itself is now bound.)
    "SecondaryChainHomotopy",
    # --- Chain complex types (not yet bound) ---
    "DoubleChainComplex",
    "TensorChainComplex",
    # --- Unstable machinery (not yet bound) ---
    "UnstableResolution",
    # --- Yoneda products (not yet bound) ---
    "yoneda_representative_element",
    # --- Misc constants / helpers (not yet bound) ---
    # secondary_*: lambda-algebra bidegree constant is not bound.
    "LAMBDA_BIDEGREE",
    # ext_m_n.py: unicode integer formatter helper is not bound.
    "unicode_num",
}


def _example_files():
    return sorted(EXAMPLES_DIR.glob("*.py"))


def _module_attr_references(source):
    """Yield (module_name, attr_name) for each ``<module>.<attr>`` access where
    ``<module>`` is one of MODULES (referenced by bare name)."""
    tree = ast.parse(source)
    for node in ast.walk(tree):
        if (
            isinstance(node, ast.Attribute)
            and isinstance(node.value, ast.Name)
            and node.value.id in MODULES
        ):
            yield node.value.id, node.attr


def _is_bound(module_name, attr):
    if hasattr(MODULES[module_name], attr):
        return True
    # ext shorthand: classes/functions bound on a re-exported submodule.
    if module_name == "ext":
        return any(hasattr(sub, attr) for sub in EXT_PY_SUBMODULES)
    return False


def test_examples_dir_is_nonempty():
    assert _example_files(), "no example scripts found to check"


def test_examples_only_reference_bound_or_known_unbound_symbols():
    failures = []
    for path in _example_files():
        for module_name, attr in _module_attr_references(path.read_text()):
            if _is_bound(module_name, attr):
                continue
            if attr in KNOWN_UNBOUND:
                continue
            failures.append(f"{path.name}: {module_name}.{attr} is not bound")
    assert not failures, "examples reference removed/renamed bindings:\n" + "\n".join(
        sorted(failures)
    )


def test_known_unbound_entries_are_actually_unbound():
    """If a KNOWN_UNBOUND name becomes bound, force a conscious update so the
    allowlist does not silently mask a future regression."""
    now_bound = [name for name in KNOWN_UNBOUND if any(
        hasattr(m, name) for m in MODULES.values()
    )]
    assert not now_bound, (
        "these KNOWN_UNBOUND names are now bound; remove them from the "
        f"allowlist: {sorted(now_bound)}"
    )


def test_fdmodulebuilder_rename_is_enforced():
    """Direct guard for the specific breakage this test was added for: the old
    ``ext.FDModule`` name must not reappear in examples, and the new
    ``FDModuleBuilder`` must be the bound name."""
    assert hasattr(algebra, "FDModuleBuilder")
    assert not hasattr(algebra, "FDModule")
    for path in _example_files():
        refs = list(_module_attr_references(path.read_text()))
        assert ("ext", "FDModule") not in refs, (
            f"{path.name} still references removed ext.FDModule"
        )
