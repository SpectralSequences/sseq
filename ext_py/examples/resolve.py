#!/usr/bin/env python3
"""
Resolves a module up to a fixed (s, t) and prints an ASCII depiction of the Ext groups.
Python translation of resolve.rs example.
"""

import ext
from ext import sseq


def main():
    # Query for module interactively
    resolution = ext.query_module_only("Module", None, False)

    # Set computation bounds
    max_n = int(input("Max n (default 30): ") or "30")
    max_s = int(input("Max s (default 15): ") or "15")

    max_bidegree = sseq.Bidegree.n_s(max_n, max_s)

    # Compute resolution through the specified bidegree
    resolution.compute_through_stem(max_bidegree)

    # Print ASCII chart
    print(resolution.graded_dimension_string())


if __name__ == "__main__":
    main()
