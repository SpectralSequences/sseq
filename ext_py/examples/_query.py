"""Backwards-compatible shim: the query I/O machinery now lives in the package.

Historically this file held the Python mirror of the Rust ``query`` crate plus
the ``query_*`` helpers. That code has moved INTO the installed package
(``ext._query`` and ``ext.utils``) as part of the maturin mixed layout. This
shim re-exports those names so existing examples that do
``import _query as query`` keep working unchanged.
"""

from ext import query_n_s, query_resolution  # noqa: F401
from ext._query import (  # noqa: F401
    optional,
    raw,
    vector,
    with_default,
    yes_no,
)
