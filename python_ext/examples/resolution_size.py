"""Print the dimension of each module in the resolution.

Translation of `ext/examples/resolution_size.rs`.

For each homological degree `s` (from largest to smallest), print
`dim F_s(t)` for `t` in `[min_degree + s, max_computed_degree]`,
comma-separated.
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

    for s in range(res.next_homological_degree() - 1, -1, -1):
        module = res.module(s)
        line = "".join(
            f"{module.dimension(t)}, "
            for t in range(res.min_degree() + s, module.max_computed_degree() + 1)
        )
        print(line)
    return 0


if __name__ == "__main__":
    sys.exit(main())
