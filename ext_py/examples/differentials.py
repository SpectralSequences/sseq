#!/usr/bin/env python3
"""Print all the differentials in the resolution.

Python port of ext/examples/differentials.rs.
"""

import _query as query
from ext import sseq


def main():
    resolution = query.query_module()

    for b in resolution.iter_stem():
        for i in range(resolution.number_of_gens_in_bidegree(b)):
            g = sseq.BidegreeGenerator(b, i)
            boundary = resolution.boundary_string(g)
            print(f"d x_{g:#} = {boundary}")


if __name__ == "__main__":
    main()
