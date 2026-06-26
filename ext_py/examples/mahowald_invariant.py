#!/usr/bin/env python3
"""Compute algebraic Mahowald invariants (aka algebraic root invariants).

Python port of ext/examples/mahowald_invariant.rs.
"""

import os

import _query as query
import ext
from ext import algebra, fp, sseq

TWO = 2


def resolve_s_2(s_2_path, k_max):
    # `utils::construct(..., "standard")` (the algorithm string selects the resolution type).
    s_2_resolution = ext.Resolution.construct("S_2", s_2_path, "standard")
    # See the Rust source for the bidegree bounds; resolve S_2 far enough to
    # detect Mahowald invariants of all classes of interest.
    s_2_resolution.compute_through_stem(
        sseq.Bidegree.n_s(2 * k_max - 2, k_max // 2 + 1)
    )
    return s_2_resolution


class MahowaldInvariant:
    def __init__(self, g, output_t, invariant, indeterminacy_basis):
        self.g = g
        self.output_t = output_t
        self.invariant = invariant
        self.indeterminacy_basis = indeterminacy_basis

    def __str__(self):
        output_t = self.output_t

        def f2_vec_to_sum(v):
            elt = sseq.BidegreeElement(
                sseq.Bidegree.s_t(self.g.s, output_t), v
            )
            return elt.to_basis_string()

        if not self.indeterminacy_basis:
            indeterminacy_info = ""
        else:
            inner = ", ".join(f2_vec_to_sum(v) for v in self.indeterminacy_basis)
            indeterminacy_info = f" mod <{inner}>"

        invariant = f2_vec_to_sum(self.invariant)
        return f"M(x_{self.g}) = {invariant}{indeterminacy_info}"


class PKData:
    def __init__(self, k, p_k_prefix, s_2_resolution):
        self.k = k
        self.s_2_resolution = s_2_resolution

        p_k_config = {
            "p": 2,
            "type": "real projective space",
            "min": -k,
        }
        p_k_path = p_k_prefix
        if p_k_path is not None:
            p_k_path = os.path.join(p_k_path, f"RP_{-k}_inf")
        # `utils::construct((p_k_config, AlgebraType::Milnor), ..., "standard")`.
        self.resolution = ext.Resolution.construct(
            (p_k_config, algebra.AlgebraType.Milnor), p_k_path, "standard"
        )
        # RP_-k_inf won't detect Mahowald invariants of classes in the k-stem and
        # beyond or of filtration higher than k/2+1.
        self.resolution.compute_through_stem(sseq.Bidegree.n_s(k - 2, k // 2 + 1))

        self.bottom_cell = ext.ResolutionHomomorphism.from_class(
            "bottom_cell",
            self.resolution,
            s_2_resolution,
            sseq.Bidegree.s_t(0, -k),
            [1],
        )
        self.bottom_cell.extend_all()

        self.minus_one_cell = ext.ResolutionHomomorphism.from_class(
            "minus_one_cell",
            self.resolution,
            s_2_resolution,
            sseq.Bidegree.s_t(0, -1),
            [1],
        )
        self.minus_one_cell.extend_all()

    def mahowald_invariants(self):
        for b in self.s_2_resolution.iter_stem():
            yield from self.mahowald_invariants_for_bidegree(b)

    def mahowald_invariants_for_bidegree(self, b):
        b_p_k = b - sseq.Bidegree.s_t(0, 1)
        if self.resolution.has_computed_bidegree(b_p_k):
            b_bottom = b_p_k + sseq.Bidegree.s_t(0, self.k)
            bottom_s_2_gens = self.s_2_resolution.number_of_gens_in_bidegree(b_bottom)
            minus_one_s_2_gens = self.s_2_resolution.number_of_gens_in_bidegree(b)
            p_k_gens = self.resolution.number_of_gens_in_bidegree(b_p_k)
            if bottom_s_2_gens > 0 and minus_one_s_2_gens > 0 and p_k_gens > 0:
                bottom_cell_map = self.bottom_cell.get_map(b_bottom.s)
                matrix = [[0] * p_k_gens for _ in range(bottom_s_2_gens)]
                for p_k_gen in range(p_k_gens):
                    output = bottom_cell_map.output(b_p_k.t, p_k_gen)
                    for s_2_gen, row in enumerate(matrix):
                        index = bottom_cell_map.target().operation_generator_to_index(
                            0, 0, b_bottom.t, s_2_gen
                        )
                        row[p_k_gen] = output.entry(index)

                padded_columns, matrix = fp.Matrix.augmented_from_vec(TWO, matrix)
                rank = matrix.row_reduce()

                if rank > 0:
                    kernel_subspace = matrix.compute_kernel(padded_columns)
                    indeterminacy_basis = [
                        row.to_owned() for row in kernel_subspace.basis()
                    ]
                    image_subspace = matrix.compute_image(p_k_gens, padded_columns)
                    quasi_inverse = matrix.compute_quasi_inverse(
                        p_k_gens, padded_columns
                    )

                    for i in range(minus_one_s_2_gens):
                        image = fp.FpVector.new(TWO, p_k_gens)
                        g = sseq.BidegreeGenerator(b, i)
                        self.minus_one_cell.act(image.slice_mut(0, p_k_gens), 1, g)
                        if not image.is_zero() and image_subspace.contains(
                            image.slice(0, p_k_gens)
                        ):
                            invariant = fp.FpVector.new(TWO, bottom_s_2_gens)
                            quasi_inverse.apply(
                                invariant.slice_mut(0, bottom_s_2_gens),
                                1,
                                image.slice(0, p_k_gens),
                            )
                            yield MahowaldInvariant(
                                g,
                                b_bottom.t,
                                invariant,
                                list(indeterminacy_basis),
                            )


def main():
    s_2_path = query.optional("Save directory for S_2", str)
    p_k_prefix = query.optional(
        "Directory containing save directories for RP_-k_inf's", str
    )
    # Going up to k=25 is nice because then we see an invariant that is not a
    # basis element and one that has non-trivial indeterminacy.
    k_max = query.with_default("Max k (positive)", "25", int)

    s_2_resolution = resolve_s_2(s_2_path, k_max)

    print("M({basis element}) = {mahowald_invariant}[ mod {indeterminacy}]")
    for k in range(1, k_max + 1):
        p_k = PKData(k, p_k_prefix, s_2_resolution)
        for mi in p_k.mahowald_invariants():
            print(mi)


if __name__ == "__main__":
    main()
