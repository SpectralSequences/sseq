"""Pure-Python I/O-driven resolution helpers, layered on the compiled bindings.

These mirror ``ext::utils::query_module`` / ``query_module_only`` (see
``ext/src/utils.rs``): they prompt (via :mod:`ext._query`) for a module spec
and an optional save directory, then build a :class:`ext.Resolution` via the
compiled :func:`ext.construct` pyfunction. All interactive I/O lives here in
Python; the Rust ``construct`` does no prompting.

Algebra vs. algorithm reconciliation
-------------------------------------
The compiled ``construct(spec, save_dir=None, algorithm=None)`` takes an
``algorithm`` string selecting the resolution ALGORITHM (``"auto"`` / ``"nassau"``
/ ``"standard"``), NOT the Steenrod-algebra basis. The algebra basis (Adem vs
Milnor) is instead selected by an ``@adem`` / ``@milnor`` suffix on the spec
string, which ``Config`` parses (matching the Rust ``query_module_only``). So the
``algebra`` argument of these helpers, when given, is encoded as a spec suffix --
it is never forwarded to ``construct`` as the ``algorithm`` parameter.
"""

import os

from . import ext as _ext
from . import _query


def _algebra_suffix(alg):
    """Normalize an ``algebra`` argument to the ``"adem"``/``"milnor"`` suffix.

    Accepts either a plain string (``"adem"``/``"milnor"``) or an
    ``algebra.AlgebraType`` (whose ``str()`` is e.g. ``"AlgebraType.Milnor"``).
    """
    s = str(alg).rsplit(".", 1)[-1].strip().lower()
    if s not in ("adem", "milnor"):
        raise ValueError(
            f"unrecognized algebra {alg!r}; expected 'adem' or 'milnor' "
            "(or an algebra.AlgebraType)"
        )
    return s


def query_module_only(prompt="Module", alg=None, save_dir=None):
    """Mirror of ``ext::utils::query_module_only``.

    Prompt for a module spec (default ``S_2``); prompt for an optional save
    directory IN PYTHON unless ``save_dir`` is supplied by the caller; then build
    and return a :class:`ext.Resolution` via :func:`ext.construct`.

    ``algebra`` (a string or ``algebra.AlgebraType``), when given and the spec
    does not already carry an ``@`` suffix, is appended as ``@<algebra>`` so the
    chosen basis is honored. See the module docstring for why this is not passed
    as ``construct``'s ``algorithm`` argument.
    """
    spec = _query.with_default(prompt, "S_2", str)

    if alg is not None and "@" not in spec:
        spec = f"{spec}@{_algebra_suffix(alg)}"

    if save_dir is None:
        save_dir = _query.optional(f"{prompt} save directory", str)

    return _ext.construct(spec, save_dir)


def query_module(alg=None, save_dir=None):
    """Mirror of ``ext::utils::query_module``.

    Build a module via :func:`query_module_only`, then prompt for ``Max n``
    (default 30) and ``Max s`` (default 7), honor the ``SECONDARY_JOB``
    environment hook (capping ``max_s``), resolve through that stem, and return
    the resolution.
    """
    resolution = query_module_only("Module", alg, save_dir)
    max_n = _query.with_default("Max n", "30", int)
    max_s = _query.with_default("Max s", "7", int)

    secondary_job = os.environ.get("SECONDARY_JOB")
    if secondary_job is not None:
        s = int(secondary_job)
        if s > max_s:
            raise ValueError("SECONDARY_JOB is larger than max_s")
        max_s = min(s + 1, max_s)

    resolution.compute_through_stem(_ext.sseq.Bidegree.n_s(max_n, max_s))
    return resolution


def query_unstable_module_only(prompt="Module", alg=None, save_dir=None):
    """Mirror of ``ext::utils::query_unstable_module_only``.

    The unstable analogue of :func:`query_module_only`: prompt for a module spec
    (default ``S_2``); prompt for an optional save directory IN PYTHON unless
    ``save_dir`` is supplied; then build and return an
    :class:`ext.UnstableResolution` via :func:`ext.construct_unstable`.

    Unstable resolutions are computed by the general algorithm only (there is no
    Nassau analogue), so there is no ``algorithm`` argument. The algebra basis
    (Adem vs Milnor, default Milnor) is selected by an ``@adem``/``@milnor``
    suffix on the spec, exactly as in :func:`query_module_only`; ``algebra``,
    when given and the spec has no ``@`` suffix, is appended as ``@<algebra>``.
    """
    spec = _query.with_default(prompt, "S_2", str)

    if alg is not None and "@" not in spec:
        spec = f"{spec}@{_algebra_suffix(alg)}"

    if save_dir is None:
        save_dir = _query.optional(f"{prompt} save directory", str)

    return _ext.construct_unstable(spec, save_dir)


def query_unstable_module(alg=None, save_dir=None):
    """Mirror of the PYTHON :func:`query_module` flow, for the unstable family.

    NOTE: this does NOT mirror the Rust ``ext::utils::query_unstable_module``,
    which only builds the resolution (it neither prompts ``Max n``/``Max s`` nor
    resolves through a stem). Like the Python :func:`query_module`, this helper
    builds an unstable module via :func:`query_unstable_module_only`, then prompts
    for ``Max n`` (default 30) and ``Max s`` (default 7), honors the
    ``SECONDARY_JOB`` environment hook (capping ``max_s``), resolves through that
    stem, and returns the :class:`ext.UnstableResolution`.
    """
    resolution = query_unstable_module_only("Module", alg, save_dir)
    max_n = _query.with_default("Max n", "30", int)
    max_s = _query.with_default("Max s", "7", int)

    secondary_job = os.environ.get("SECONDARY_JOB")
    if secondary_job is not None:
        s = int(secondary_job)
        if s > max_s:
            raise ValueError("SECONDARY_JOB is larger than max_s")
        max_s = min(s + 1, max_s)

    resolution.compute_through_stem(_ext.sseq.Bidegree.n_s(max_n, max_s))
    return resolution
