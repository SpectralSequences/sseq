"""Generate an SVG chart of Ext groups with filtration-one products.

Translation of `ext/examples/chart.rs`. Writes the SVG to stdout (or a file
if `--out` is supplied).
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
    parser.add_argument("--out", default=None,
                        help="output file (defaults to stdout)")
    parser.add_argument("--save-dir", default=None)
    args = parser.parse_args()

    ext.init_logging()
    res = ext.construct(args.module, save_dir=args.save_dir)
    res.compute_through_stem(ext.Bidegree.n_s(args.max_n, args.max_s))

    if args.out is None:
        # Get bytes back and write to stdout.
        svg = ext.write_sseq_svg(res)
        sys.stdout.buffer.write(svg)
    else:
        ext.write_sseq_svg(res, path=args.out)
    return 0


if __name__ == "__main__":
    sys.exit(main())
