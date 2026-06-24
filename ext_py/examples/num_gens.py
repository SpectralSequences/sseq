#!/usr/bin/env python3
"""Print the number of generators in each Ext^{s, n+s} as `n,s,num_gens`.

Python port of ext/examples/num_gens.rs.
"""

import _query as query


def main():
    resolution = query.query_module()

    for b in resolution.iter_stem():
        print(f"{b.n()},{b.s()},{resolution.number_of_gens_in_bidegree(b)}")


if __name__ == "__main__":
    main()
