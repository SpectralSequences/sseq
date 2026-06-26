"""Smoke test guarding examples/*.py against removed/renamed bindings.

These example scripts are interactive (they read from stdin) and/or run heavy
resolutions, so we do NOT execute them. Instead, we statically extract every
top-level module attribute access of the form ``<module>.<Name>`` (where
``<module>`` is one of the bound modules ``ext``/``algebra``/``fp``/
``sseq``) AND every ``from ext import <Name>`` / ``from
ext.<submodule> import <Name>`` statement from each example via the ``ast``
module, and assert that every referenced name is either actually bound on the
built extension module, or is explicitly listed in ``KNOWN_UNBOUND`` below.

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
    # tensor.py: FDModuleBuilder.from_tensor_module / .from_module conversion
    # constructor is documented as a planned thin wrapper but is not yet bound.
    "from_tensor_module",
    # massey.py: Massey product computer is not bound.
    "MasseyProductComputer",
    # --- Resolution / construction entry points ---
    # construct: now bound as the ext.Resolution.construct staticmethod (no
    # longer a top-level pyfunction); removed from the allowlist.
    # construct_standard remains unbound (the algorithm="standard" argument to
    # Resolution.construct selects the standard algorithm instead).
    "construct_standard",
    # ResolutionHomomorphism and friends: lifting/secondary machinery.
    # (ResolutionHomomorphism, SecondaryResolutionHomomorphism,
    # UnstableResolutionHomomorphism are now bound.)
    # --- Chain complex types (not yet bound) ---
    "DoubleChainComplex",
    "TensorChainComplex",
    # bruner.py imports `from ext import FiniteChainComplex`; that name was
    # never bound. The bound finite-complex types are ChainComplex (CCC) and
    # FiniteAugmentedChainComplex; bruner.py predates them. Caught now that the
    # guard scans `from ext import ...` statements as well as attribute access.
    "FiniteChainComplex",
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


def _module_import_references(source):
    """Yield (module_name, name) for each ``from ext import <name>`` and
    ``from ext.<submodule> import <name>`` statement.

    ``from ext import X`` resolves ``X`` against the top-level ``ext``
    package (and, via ``_is_bound``, its re-exported submodules); ``from
    ext.algebra import X`` resolves against that submodule. Imports of the
    submodules themselves (``from ext import algebra``) and star imports
    are skipped, as are non-binding Python submodules (e.g. ``ext._query``).
    """
    tree = ast.parse(source)
    for node in ast.walk(tree):
        if not isinstance(node, ast.ImportFrom) or node.module is None:
            continue
        mod = node.module
        if mod == "ext" or mod == "ext.ext":
            key = "ext"
        elif mod.startswith("ext.") and mod.split(".", 1)[1] in MODULES:
            key = mod.split(".", 1)[1]
        else:
            # Not a bound module namespace (e.g. ``ext._query``): skip.
            continue
        for alias in node.names:
            if alias.name == "*":
                continue
            # ``from ext import algebra`` imports a submodule, not a binding.
            if key == "ext" and alias.name in MODULES:
                continue
            yield key, alias.name


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
        source = path.read_text()
        refs = list(_module_attr_references(source)) + list(
            _module_import_references(source)
        )
        for module_name, attr in refs:
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
