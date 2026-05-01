"""Print the dimension of the mod-2 Steenrod algebra in each degree.

Translation of `ext/examples/algebra_dim.rs`.
"""

from __future__ import annotations

import argparse
import sys

import sseq_ext as ext


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("p", nargs="?", type=int, default=2)
    parser.add_argument("max_n", nargs="?", type=int, default=125)
    args = parser.parse_args()

    ext.init_logging()
    a = ext.MilnorAlgebra(args.p)
    a.compute_basis(args.max_n)
    for n in range(args.max_n + 1):
        print(f"dim A_{n} = {a.dimension(n)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
