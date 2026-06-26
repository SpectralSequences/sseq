#!/usr/bin/env python3
"""Compute d_2 differentials in the Adams spectral sequence (Milnor basis only).

Differentials are omitted where the target bidegree is zero.

Python port of ext/examples/secondary.rs.
"""

import os

import _query as query
import ext
from ext import algebra, sseq


def main():
    # standard backend: this example uses SecondaryResolution, unavailable on Nassau
    resolution = query.query_resolution(alg=algebra.AlgebraType.Milnor, algorithm="standard")
    resolution.compute_through_stem(query.query_n_s())

    lift = ext.SecondaryResolution(resolution)

    secondary_job = os.environ.get("SECONDARY_JOB")
    if secondary_job is not None:
        lift.compute_partial(int(secondary_job))
        return

    lift.extend_all()

    d2_shift = sseq.Bidegree.n_s(-1, 2)

    # Iterate through the target of the d2.
    for b in lift.underlying().iter_nonzero_stem():
        if b.s < 3:
            continue
        if b.t - 1 > resolution.module(b.s - 2).max_computed_degree():
            continue

        homotopy = lift.homotopy(b.s)
        m = homotopy.homotopies.hom_k(b.t - 1)

        for i, entry in enumerate(m):
            source_gen = sseq.BidegreeGenerator(b - d2_shift, i)
            print(f"d_2 x_{source_gen} = {list(entry)}")


if __name__ == "__main__":
    main()
