#!/usr/bin/env python3
"""Print all available filtration-one products for a module (prime 2 only).

Outputs where the target bidegree is zero (or not yet computed) are omitted.

Python port of ext/examples/filtration_one.rs.
"""

import _query as query
from ext import sseq


def main():
    resolution = query.query_module()
    assert resolution.prime() == 2

    for b in resolution.iter_stem():
        i = 0
        while resolution.has_computed_bidegree(b + sseq.Bidegree.s_t(1, 1 << i)):
            # TODO: This doesn't work with the reordered Adams basis
            products = resolution.filtration_one_product(1 << i, 0, b)
            for idx, row in enumerate(products):
                g = sseq.BidegreeGenerator(b, idx)
                if not row:
                    continue
                print(f"h_{i} x_{g} = {list(row)}")
            i += 1


if __name__ == "__main__":
    main()
