#!/usr/bin/env python3
"""Write E2/E3 page charts (with and without d2) of a resolution to TikZ files.

Python port of ext/examples/d2_charts.rs.
"""

import _query as query
import ext
from ext import algebra, sseq


def main():
    # standard backend: this example uses SecondaryResolution, unavailable on Nassau
    resolution = query.query_resolution(alg=algebra.AlgebraType.Milnor, algorithm="standard")
    resolution.compute_through_stem(query.query_n_s())

    lift = ext.SecondaryResolution(resolution)
    lift.extend_all()

    ss = lift.e3_page
    products = [
        (name, resolution.filtration_one_products(op_deg, op_idx))
        for (name, op_deg, op_idx) in resolution.algebra().default_filtration_one_products()
    ]

    def write(path, page, diff, prod):
        # NOTE: depends on TikzBackend.EXT and Resolution.name() (API_PROPOSAL §6.3, §7.4).
        suffix = sseq.TikzBackend.EXT
        backend = sseq.TikzBackend(
            open(f"{path}_{resolution.name}.{suffix}", "w")
        )
        ss.write_to_graph(backend, page, diff, products[:prod], lambda _: None)

    write("e2", 2, False, 3)
    write("e2_d2", 2, True, 3)
    write("e3", 3, False, 3)

    write("e2_clean", 2, False, 2)
    write("e2_d2_clean", 2, True, 2)
    write("e3_clean", 3, False, 2)


if __name__ == "__main__":
    main()
