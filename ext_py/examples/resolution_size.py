#!/usr/bin/env python3
"""Print the dimension of each module in the resolution, by homological degree.

Python port of ext/examples/resolution_size.rs.
"""

import _query as query


def main():
    res = query.query_module()

    for s in reversed(range(res.next_homological_degree())):
        module = res.module(s)
        for t in range(res.min_degree() + s, module.max_computed_degree() + 1):
            print(f"{module.dimension(t)}, ", end="")
        print()


if __name__ == "__main__":
    main()
