#!/usr/bin/env python3
"""Resolve a module up to a fixed (s, t) and print an ASCII depiction of Ext.

Python port of ext/examples/resolve.rs.
"""

import _query as query
from ext import sseq


def main():
    res = query.query_resolution("Module")

    t = query.with_default("Max t", "30", int)
    s = query.with_default("Max s", "15", int)

    res.compute_through_bidegree(sseq.Bidegree.s_t(s, t))

    print(res.graded_dimension_string())


if __name__ == "__main__":
    main()
