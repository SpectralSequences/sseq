#!/usr/bin/env python3
"""Compute products in Mod_{C lambda^2}.

Python port of ext/examples/secondary_product.rs.
"""

import os

import _query as query
import ext
from ext import algebra, fp, sseq


def main():
    # standard backend: this example uses get_unit()/SecondaryResolution, unavailable on Nassau
    resolution = query.query_resolution(alg=algebra.AlgebraType.Milnor, algorithm="standard")
    resolution.compute_through_stem(query.query_n_s())

    is_unit, unit = ext.get_unit(resolution)

    p = resolution.prime

    name = query.raw("Name of product", str)

    shift = sseq.Bidegree.n_s(
        query.raw(f"n of Ext class {name}", int),
        query.raw(f"s of Ext class {name}", int),
    )

    hom = ext.ResolutionHomomorphism(name, resolution, unit, shift)

    matrix = fp.Matrix(p, hom.source().number_of_gens_in_bidegree(shift), 1)

    if matrix.rows() == 0 or matrix.columns() == 0:
        raise AssertionError("No classes in this bidegree")

    v = query.vector("Input ext class", matrix.rows())
    for i, x in enumerate(v):
        matrix.row_mut(i).set_entry(0, x)

    if not is_unit:
        res_max = sseq.Bidegree.n_s(
            resolution.module(0).max_computed_degree(),
            resolution.next_homological_degree() - 1,
        )
        unit.compute_through_stem(res_max - shift)

    hom.extend_step(shift, matrix)
    hom.extend_all()

    res_lift = ext.SecondaryResolution(resolution)
    res_lift.extend_all()

    # Check that class survives to E3.
    m = res_lift.homotopy(shift.s + 2).homotopies.hom_k(shift.t)
    assert len(m) == len(v)
    total = [0] * len(m[0])
    for x, d2 in zip(v, m):
        for k in range(len(total)):
            total[k] += x * d2[k]
    assert all(a % int(p) == 0 for a in total), "Class supports a non-zero d2"

    unit_lift = res_lift
    if not is_unit:
        unit_lift = ext.SecondaryResolution(unit)
        unit_lift.extend_all()

    hom_lift = ext.SecondaryResolutionHomomorphism(res_lift, unit_lift, hom)

    secondary_job = os.environ.get("SECONDARY_JOB")
    if secondary_job is not None:
        hom_lift.compute_partial(int(secondary_job))
        return

    hom_lift.extend_all()

    # Compute E3 page
    res_sseq = res_lift.e3_page
    unit_sseq = res_sseq if is_unit else unit_lift.e3_page

    def get_page_data(ss, b):
        # NOTE: depends on Subquotient methods complement_pivots / subspace_gens /
        # subspace_dimension on Subquotient (API_PROPOSAL §4, listed as "full Subquotient
        # pyclass … add_gen, …" but these accessors are not enumerated)
        d = ss.page_data(b)
        return d[min(3, len(d) - 1)]

    name = hom_lift.name()
    # Iterate through the multiplicand
    for b in unit.iter_nonzero_stem():
        # The potential target has to be hit, and we need to have computed (the data need for) the
        # d2 that hits the potential target.
        if not resolution.has_computed_bidegree(b + shift + ext.LAMBDA_BIDEGREE):
            continue
        if not resolution.has_computed_bidegree(b + shift - sseq.Bidegree.s_t(1, 0)):
            continue

        page_data = get_page_data(unit_sseq, b)

        target_num_gens = resolution.number_of_gens_in_bidegree(b + shift)
        lambda_num_gens = resolution.number_of_gens_in_bidegree(
            b + shift + ext.LAMBDA_BIDEGREE
        )

        if target_num_gens == 0 and lambda_num_gens == 0:
            continue

        # First print the products with non-surviving classes
        if target_num_gens > 0:
            hom_k = hom.get_map((b + shift).s).hom_k(b.t)
            for i in page_data.complement_pivots():
                g = sseq.BidegreeGenerator(b, i)
                print(f"{name} λ x_{g} = λ {list(hom_k[i])}")

        # Now print the secondary products
        if page_data.subspace_dimension() == 0:
            continue

        outputs = [
            fp.FpVector(p, target_num_gens + lambda_num_gens)
            for _ in range(page_data.subspace_dimension())
        ]

        hom_lift.hom_k(
            res_sseq,
            b,
            page_data.subspace_gens(),
            [out.as_slice_mut() for out in outputs],
        )
        for g, output in zip(page_data.subspace_gens(), outputs):
            basis_string = sseq.BidegreeElement(b, g.to_owned()).to_basis_string()
            print(
                f"{name} [{basis_string}] = "
                f"{output.slice(0, target_num_gens)} + "
                f"λ {output.slice(target_num_gens, target_num_gens + lambda_num_gens)}"
            )


if __name__ == "__main__":
    main()
