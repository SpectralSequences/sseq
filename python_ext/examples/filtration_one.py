"""Print all filtration-one products in the resolution.

Translation of `ext/examples/filtration_one.rs`. Currently only works at
p=2.

Outputs lines of the form ``h_i x_(n,s,idx) = [coefficients]`` whenever the
target bidegree is non-empty.
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
    if int(res.prime()) != 2:
        raise SystemExit("filtration_one currently only works at p = 2")
    res.compute_through_stem(ext.Bidegree.n_s(args.max_n, args.max_s))

    for b in res.iter_stem():
        i = 0
        while res.has_computed_bidegree(b + ext.Bidegree.s_t(1, 1 << i)):
            # h_i = filtration-one product with the operation Sq^{2^i}.
            # The Rust code uses `op_idx = 0`, which only makes sense with
            # the canonical (Adem-style) basis ordering.
            products = res.filtration_one_product(1 << i, 0, b)
            assert products is not None  # guaranteed for stable resolutions
            for idx, row in enumerate(products):
                if row:  # non-empty row
                    g = ext.BidegreeGenerator(b, idx)
                    print(f"h_{i} x_{g} = {row}")
            i += 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
