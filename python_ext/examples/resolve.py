"""Resolves a module up to fixed (s, t) and prints an ASCII chart of Ext.

Translation of `ext/examples/resolve.rs`.

Usage:
    python resolve.py [module] [max_t] [max_s] [save_dir]

All arguments are optional; defaults match the Rust example.
"""

from __future__ import annotations

import argparse
import sys

import sseq_ext as ext


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("module", nargs="?", default="S_2",
                        help="module spec, e.g. S_2 or Ceta@adem")
    parser.add_argument("max_t", nargs="?", type=int, default=30)
    parser.add_argument("max_s", nargs="?", type=int, default=15)
    parser.add_argument("--save-dir", default=None)
    args = parser.parse_args()

    ext.init_logging()
    res = ext.construct(args.module, save_dir=args.save_dir)
    res.compute_through_bidegree(ext.Bidegree.s_t(args.max_s, args.max_t))
    print(res.graded_dimension_string(), end="")
    return 0


if __name__ == "__main__":
    sys.exit(main())
