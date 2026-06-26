#!/usr/bin/env python3
"""Compute the triple Massey product <a, b, -> (up to a sign).

Optimized to compute <a, b, -> for fixed a, b of small degree and all -.

Python port of ext/examples/massey.rs.
"""

import _query as query
import ext
from ext import fp, sseq


def main():
    # standard backend: this example uses get_unit(), unavailable on Nassau
    resolution = query.query_resolution(algorithm="standard")
    resolution.compute_through_stem(query.query_n_s())
    p = resolution.prime()

    is_unit, unit = ext.get_unit(resolution)

    a = sseq.Bidegree.n_s(
        query.raw("n of Ext class a", int),
        query.raw("s of Ext class a", int),
    )
    unit.compute_through_stem(a)
    a_class = query.vector("Input Ext class a", unit.number_of_gens_in_bidegree(a))

    b = sseq.Bidegree.n_s(
        query.raw("n of Ext class b", int),
        query.raw("s of Ext class b", int),
    )
    unit.compute_through_stem(b)
    b_class = query.vector("Input Ext class b", unit.number_of_gens_in_bidegree(b))

    # The Massey product shifts the bidegree by this amount.
    shift = a + b - sseq.Bidegree.s_t(1, 0)

    if not is_unit:
        unit.compute_through_stem(shift)

    if not resolution.has_computed_bidegree(
        shift + sseq.Bidegree.s_t(0, resolution.min_degree())
    ):
        return

    b_hom = ext.ResolutionHomomorphism.from_class("", unit, unit, b, b_class)
    b_hom.extend_through_stem(shift)

    offset_a = unit.module(a.s).generator_offset(a.t, a.t, 0)
    for c in resolution.iter_nonzero_stem():
        if not resolution.has_computed_bidegree(c + shift):
            continue

        tot = c + shift

        num_gens = resolution.number_of_gens_in_bidegree(c)
        product_num_gens = resolution.number_of_gens_in_bidegree(b + c)
        target_num_gens = resolution.number_of_gens_in_bidegree(tot)
        if target_num_gens == 0:
            continue

        answers = [[0] * target_num_gens for _ in range(num_gens)]
        product = fp.AugmentedMatrix2(p, num_gens, [product_num_gens, num_gens])
        product.segment(1, 1).add_identity()

        matrix = fp.Matrix(p, num_gens, 1)
        for idx in range(num_gens):
            hom = ext.ResolutionHomomorphism("", resolution, unit, c)

            matrix.row_mut(idx).set_entry(0, 1)
            hom.extend_step(c, matrix)
            matrix.row_mut(idx).set_entry(0, 0)

            hom.extend_through_stem(tot)

            homotopy = ext.ChainHomotopy(hom, b_hom)
            homotopy.extend(tot)

            last = homotopy.homotopy(tot.s)
            for i in range(target_num_gens):
                output = last.output(tot.t, i)
                for k, v in enumerate(a_class):
                    if v != 0:
                        answers[idx][i] += v * output.entry(offset_a + k)

            for k, v in enumerate(b_class):
                if v != 0:
                    g = sseq.BidegreeGenerator(b, k)
                    hom.act(product.row_mut(idx).slice_mut(0, product_num_gens), v, g)

        product.row_reduce()
        kernel = product.compute_kernel()

        for row in kernel.iter():
            c_element = sseq.BidegreeElement(c, row.to_owned())
            entries = []
            for i in range(target_num_gens):
                entry = 0
                for j, v in enumerate(row):
                    entry += v * answers[j][i]
                entries.append(str(entry % p))
            print(f"<a, b, {c_element.to_basis_string()}> = [{', '.join(entries)}]")


if __name__ == "__main__":
    main()
