#!/usr/bin/env python3
"""Write an SVG chart of the spectral sequence of a resolution to stdout.

Python port of ext/examples/chart.rs.
"""

import sys

import _query as query
from ext import sseq


def main():
    resolution = query.query_resolution()
    resolution.compute_through_stem(query.query_n_s())

    ss = resolution.to_sseq()
    products = [
        (name, resolution.filtration_one_products(op_deg, op_idx))
        for (name, op_deg, op_idx) in resolution.algebra().default_filtration_one_products()
    ]

    ss.write_to_graph(
        sseq.SvgBackend(sys.stdout),
        2,
        False,
        products,
        lambda _: None,
    )


if __name__ == "__main__":
    main()
