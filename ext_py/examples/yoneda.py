#!/usr/bin/env python3
"""Compute a Yoneda representative of an Ext class and print module dimensions.

Python port of ext/examples/yoneda.rs.
"""

import _query as query
import ext
from ext import sseq


def main():
    # query.query_module_only mirrors `utils::query_module_only("Module", None,
    # false)`; the shim ignores the `load_quasi_inverse` flag, which is fine.
    resolution = query.query_module_only("Module")

    b = sseq.Bidegree.n_s(
        query.raw("n of Ext class", int),
        query.raw("s of Ext class", int),
    )

    resolution.compute_through_stem(b)

    class_ = query.vector(
        "Input Ext class", resolution.number_of_gens_in_bidegree(b)
    )

    # NOTE: depends on ext.yoneda_representative_element (API_PROPOSAL §7.5).
    yoneda = ext.yoneda_representative_element(resolution, b, class_)

    for s in range(0, b.s() + 1):
        print(
            f"Dimension of {s}th module is {yoneda.module(s).total_dimension()}"
        )


if __name__ == "__main__":
    main()
