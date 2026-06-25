"""The ``ext`` package: a compiled PyO3 extension plus a thin pure-Python I/O layer.

Layout (maturin "mixed" project): the compiled extension is installed as the
submodule ``ext.ext`` (``ext/ext.<abi>.so``); this ``__init__`` is
the package's pure-Python source (from ``python/ext/`` in the repo). It
re-exports everything from the compiled submodule so that the historical import
surface is preserved EXACTLY, then layers the pure-Python I/O helpers on top.

Import-surface contract (all of these must keep working):
  - ``import ext``
  - ``ext.Resolution`` (and every other top-level compiled symbol)
  - ``from ext import sseq`` / ``fp`` / ``algebra`` (as attributes)
  - ``from ext.algebra import <name>`` (etc.)
  - ``import ext.ext`` (the compiled submodule, dotted path)
  - ``ext.construct`` / ``ext.query_module`` / ``ext.query_module_only``

Note the deliberate name overlap: ``ext.query_module`` / ``query_module_only``
are the interactive *pure-Python* helpers (defined in ``ext.utils``), which
shadow the lower-level *Rust* pyfunctions of the same name that remain reachable
as ``ext.ext.query_module`` / ``query_module_only``. They are NOT the same
object; prefer the package-level (Python) ones.
"""

# 1) Re-export every compiled symbol (top-level functions/classes AND the
#    submodules sseq/fp/algebra, which the compiled module exports) so
#    ``ext.<Name>`` and ``from ext import <Name>`` behave as before.
from .ext import *  # noqa: F401,F403

# 2) Keep the compiled submodule importable as ``ext.ext`` (some code and
#    the fresh-build check use that dotted path) and mirror its docstring/__all__.
from . import ext as _ext  # noqa: F401

# 3) Belt-and-suspenders: ``from .ext import *`` only re-exports names listed
#    in the module's ``__all__`` (or, absent that, its non-underscore globals).
#    Explicitly bind the submodules so ``from ext import sseq`` works as an
#    attribute even if a future ``__all__`` change drops them. This is additive
#    and never drops a currently-exported top-level symbol.
from .ext import algebra, fp, sseq  # noqa: F401

# Register the compiled submodules under their dotted package paths in
# ``sys.modules`` so the true-submodule import form ``from ext.algebra
# import <name>`` (used by several examples) resolves. The compiled module
# exports them with bare ``__name__``s (``algebra`` etc.), which Python's
# import machinery does not find as ``ext.algebra`` on its own; this is
# additive and never breaks the attribute form ``from ext import algebra``.
import sys as _sys  # noqa: E402

for _sub in (algebra, fp, sseq):
    _sys.modules.setdefault(f"{__name__}.{_sub.__name__}", _sub)

__doc__ = _ext.__doc__
if hasattr(_ext, "__all__"):
    __all__ = list(_ext.__all__)

# 4) Layer the pure-Python I/O utilities ON TOP of the compiled symbols. This is
#    done AFTER ``from .ext import *`` so the Python ``query_module`` /
#    ``query_module_only`` INTENTIONALLY SHADOW the compiled (Rust) pyfunctions
#    of the same name: all interactive I/O lives in Python, while the Rust
#    pyfunctions remain bound under ``ext.ext.query_module*`` for anyone
#    who needs them. ``construct`` is the Rust pyfunction (no Python override).
from .utils import (  # noqa: E402
    query_module,
    query_module_only,
)

# Re-export the low-level query primitives too, so ``ext._query`` consumers
# and examples can reach them via the package if desired.
from ._query import (  # noqa: E402,F401
    optional,
    raw,
    vector,
    with_default,
    yes_no,
)

# Make sure the Python utils appear in __all__ (so ``from ext import *`` in
# downstream code exposes them, and they take precedence over the shadowed
# compiled names).
for _name in ("query_module", "query_module_only", "construct"):
    if "__all__" in dir() and _name not in __all__:
        __all__.append(_name)
