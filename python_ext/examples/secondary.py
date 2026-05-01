"""Compute d_2 differentials in the Adams spectral sequence.

Translation of `ext/examples/secondary.rs`.

The module must be over the Milnor basis (we add ``@milnor`` if missing),
since the secondary Steenrod algebra computation requires it.
"""

from __future__ import annotations

import argparse
import sys

import sseq_ext as ext


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("module", nargs="?", default="S_2")
    parser.add_argument("max_n", nargs="?", type=int, default=30)
    parser.add_argument("max_s", nargs="?", type=int, default=7)
    parser.add_argument("--save-dir", default=None)
    args = parser.parse_args()

    ext.init_logging()
    # Force milnor basis.
    res = ext.construct(args.module, algebra="milnor", save_dir=args.save_dir)
    res.compute_through_stem(ext.Bidegree.n_s(args.max_n, args.max_s))

    lift = ext.SecondaryResolution(res)

    # Sharded execution: SECONDARY_JOB=s computes only data for that s.
    job = ext.secondary_job()
    if job is not None:
        lift.compute_partial(job)
        return 0

    lift.extend_all()

    d2_shift = ext.Bidegree.n_s(-1, 2)

    # Iterate through targets of the d_2 differential.
    for b in res.iter_nonzero_stem():
        if b.s < 3:
            continue
        if b.t - 1 > res.module(b.s - 2).max_computed_degree():
            continue
        homotopy = lift.homotopy(b.s)
        m = homotopy.hom_k(b.t - 1)
        for i, entry in enumerate(m):
            source_gen = ext.BidegreeGenerator(b - d2_shift, i)
            print(f"d_2 x_{source_gen} = {entry}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
