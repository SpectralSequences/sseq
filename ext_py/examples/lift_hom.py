#!/usr/bin/env python3
"""Lift a Hom map to the induced map Ext(N, k) -> Ext(M, k) by composition.

Python port of ext/examples/lift_hom.rs.
"""

import sys

import _query as query
import ext
from ext import fp, sseq


def main():
    source = query.query_module_only("Source module")
    b = sseq.Bidegree.n_s(
        query.with_default("Max source n", "30", int),
        query.with_default("Max source s", "7", int),
    )

    source_name = source.name

    def parse_target(s):
        if s == source_name:
            return source
        save_dir = query.optional("Target save directory", str)
        target = ext.Resolution.construct(s, save_dir)
        target.set_name(s)
        return target

    target = query.with_default("Target module", source_name, parse_target)

    assert source.prime() == target.prime()
    p = source.prime()

    name = query.raw("Name of product", str)

    shift = sseq.Bidegree.n_s(
        query.with_default("n of product", "0", int),
        query.with_default("s of product", "0", int),
    )

    source.compute_through_stem(b)
    target.compute_through_stem(b - shift)

    target_module = target.target().module(0)
    hom = ext.ResolutionHomomorphism(name, source, target, shift)

    print("\nInput Ext class to lift:", file=sys.stderr)
    for output_t in range(0, target_module.max_degree() + 1):
        output = sseq.Bidegree.s_t(0, output_t)
        input = output + shift
        matrix = fp.Matrix(
            p,
            hom.source.number_of_gens_in_bidegree(input),
            target_module.dimension(output.t),
        )

        if matrix.rows() == 0 or matrix.columns() == 0:
            hom.extend_step(input, None)
        else:
            for idx in range(matrix.rows()):
                row = matrix.row_mut(idx)
                g = sseq.BidegreeGenerator(input, idx)
                v = query.vector(f"f(x_{g}", len(row.as_slice()))
                for i, x in enumerate(v):
                    row.set_entry(i, x)
            hom.extend_step(input, matrix)

    hom.extend_all()

    for b2 in hom.target.iter_stem():
        shifted_b2 = b2 + shift
        if (
            shifted_b2.s >= hom.source.next_homological_degree()
            or shifted_b2.t > hom.source.module(shifted_b2.s).max_computed_degree()
        ):
            continue
        matrix = hom.get_map(shifted_b2.s).hom_k(b2.t)
        for i, r in enumerate(matrix):
            g = sseq.BidegreeGenerator(b2, i)
            print(f"{name} x_{g} = {list(r)}")


if __name__ == "__main__":
    main()
