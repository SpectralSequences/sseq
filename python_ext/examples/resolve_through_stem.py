"""Resolve a module up to a fixed (n, s) and print an ASCII Ext chart.

Translation of `ext/examples/resolve_through_stem.rs`.

This is the variant of `resolve.py` that asks for stem `n` and homological
degree `s` instead of internal degree `t` and `s`.
"""

from __future__ import annotations

import argparse
import sys

import sseq_ext as ext


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("module", nargs="?", default="S_2")
    parser.add_argument("max_n", nargs="?", type=int, default=30)
    parser.add_argument("max_s", nargs="?", type=int, default=15)
    parser.add_argument("--save-dir", default=None)
    args = parser.parse_args()

    ext.init_logging()
    res = ext.construct(args.module, save_dir=args.save_dir)
    res.compute_through_stem(ext.Bidegree.n_s(args.max_n, args.max_s))
    print(res.graded_dimension_string(), end="")
    return 0


if __name__ == "__main__":
    sys.exit(main())
