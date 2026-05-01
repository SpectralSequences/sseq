"""Compute triple Massey products <a, b, -> for fixed Ext classes a, b.

Translation of `ext/examples/massey.rs`.

Inputs `a` and `b` are read interactively (or supplied as CLI args).
"""

from __future__ import annotations

import argparse
import sys

import sseq_ext as ext


def parse_class(s: str) -> list[int]:
    """Parse a class given as ``[1,0,1]`` or ``1 0 1``."""
    s = s.strip().lstrip("[").rstrip("]")
    return [int(x) for x in s.replace(",", " ").split()]


def prompt(prompt_str: str, default: str | None = None) -> str:
    text = prompt_str
    if default is not None:
        text += f" (default: {default})"
    text += ": "
    line = input(text).strip()
    if line == "" and default is not None:
        return default
    return line


def get_bidegree(label: str) -> ext.Bidegree:
    n = int(prompt(f"n of Ext class {label}"))
    s = int(prompt(f"s of Ext class {label}"))
    if s <= 0:
        raise ValueError("s must be a non-zero positive integer")
    return ext.Bidegree.n_s(n, s)


def get_class(label: str, length: int) -> list[int]:
    cls = parse_class(prompt(f"Input Ext class {label} (length {length})"))
    if len(cls) != length:
        raise ValueError(f"Expected vector of length {length}, got {len(cls)}")
    return cls


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("module", nargs="?", default="S_2")
    parser.add_argument("max_n", nargs="?", type=int, default=20)
    parser.add_argument("max_s", nargs="?", type=int, default=10)
    parser.add_argument("--save-dir", default=None)
    parser.add_argument("--unit-save-dir", default=None)
    args = parser.parse_args()

    ext.init_logging()
    res = ext.construct(args.module, save_dir=args.save_dir)
    res.compute_through_stem(ext.Bidegree.n_s(args.max_n, args.max_s))
    p = res.prime()
    p_int = int(p)

    is_unit, unit = ext.get_unit(res, unit_save_dir=args.unit_save_dir)

    print("\nComputing Massey products <a, b, ->", file=sys.stderr)
    print("\nEnter a:", file=sys.stderr)
    a = get_bidegree("a")
    unit.compute_through_stem(a)
    a_class = get_class("a", unit.number_of_gens_in_bidegree(a))

    print("\nEnter b:", file=sys.stderr)
    b = get_bidegree("b")
    unit.compute_through_stem(b)
    b_class = get_class("b", unit.number_of_gens_in_bidegree(b))

    # Massey product shifts the bidegree by (a + b - (s_t=1, t=0)).
    shift = a + b - ext.Bidegree.s_t(1, 0)

    if not is_unit:
        unit.compute_through_stem(shift)

    if not res.has_computed_bidegree(shift + ext.Bidegree.s_t(0, res.min_degree())):
        print("No computable bidegrees", file=sys.stderr)
        return 0

    b_hom = ext.ResolutionHomomorphism.from_class("", unit, unit, b, b_class)
    b_hom.extend_through_stem(shift)

    offset_a = unit.module(a.s).generator_offset(a.t, a.t, 0)

    for c in res.iter_nonzero_stem():
        if not res.has_computed_bidegree(c + shift):
            continue

        tot = c + shift

        num_gens = res.number_of_gens_in_bidegree(c)
        product_num_gens = res.number_of_gens_in_bidegree(b + c)
        target_num_gens = res.number_of_gens_in_bidegree(tot)
        if target_num_gens == 0:
            continue

        answers: list[list[int]] = [[0] * target_num_gens for _ in range(num_gens)]
        product = ext.AugmentedMatrix(p, num_gens, [product_num_gens, num_gens])
        product.segment_add_identity(1)

        # One-hot matrix used to seed `extend_step`.
        seed = ext.Matrix(p, num_gens, 1)

        for idx in range(num_gens):
            hom = ext.ResolutionHomomorphism("", res, unit, c)

            seed.set_entry(idx, 0, 1)
            hom.extend_step(c, extra_images=seed)
            seed.set_entry(idx, 0, 0)

            hom.extend_through_stem(tot)

            homotopy = ext.ChainHomotopy(hom, b_hom)
            homotopy.extend(tot)

            last = homotopy.homotopy(tot.s)
            answer_row = answers[idx]
            for i in range(target_num_gens):
                output = last.output(tot.t, i)
                for k, v in enumerate(a_class):
                    if v != 0:
                        answer_row[i] += v * output.entry(offset_a + k)

            for k, v in enumerate(b_class):
                if v != 0:
                    g = ext.BidegreeGenerator(b, k)
                    # Mutable view onto product[idx, segment 0]; equivalent
                    # to `product.row_mut(idx).slice_mut(0, product_num_gens)`
                    # in Rust.
                    row_view = product.row_segment_view_mut(idx, 0)
                    hom.act(row_view, v, g)

        product.row_reduce()
        kernel = product.compute_kernel()

        for row in kernel.basis():
            c_element = ext.BidegreeElement(c, row)
            entries = []
            for i in range(target_num_gens):
                entry = 0
                for j, v in enumerate(row.to_list()):
                    entry += v * answers[j][i]
                entries.append(entry % p_int)
            print(f"<a, b, {c_element.to_basis_string()}> = {entries}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
