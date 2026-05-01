"""Print every differential in the minimal resolution.

Translation of `ext/examples/differentials.rs`.
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
    res = ext.construct(args.module, save_dir=args.save_dir)
    res.compute_through_stem(ext.Bidegree.n_s(args.max_n, args.max_s))

    for b in res.iter_stem():
        for i in range(res.number_of_gens_in_bidegree(b)):
            g = ext.BidegreeGenerator(b, i)
            print(f"d x_{g:compact} = {res.boundary_string(g)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
