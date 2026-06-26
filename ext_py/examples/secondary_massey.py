#!/usr/bin/env python3
"""Compute Massey products in Mod_{C lambda^2}.

Python port of ext/examples/secondary_massey.rs.
"""

import os
import sys

import _query as query
import ext
from ext import algebra, fp, sseq


class HomData:
    def __init__(self, name, class_, hom_lift, lambda_part):
        self.name = name
        self.class_ = class_
        self.hom_lift = hom_lift
        self.lambda_part = lambda_part


def get_hom(name, source, target):
    p = source.prime

    shift = sseq.Bidegree.n_s(
        query.raw(f"n of {name}", int),
        query.raw(f"s of {name}", int),
    )

    ext_name = query.raw(f"Name of Ext part of {name}", str)

    source.underlying().compute_through_stem(shift + ext.LAMBDA_BIDEGREE)

    hom = ext.ResolutionHomomorphism(
        ext_name, source.underlying(), target.underlying(), shift
    )

    num_gens = source.underlying().number_of_gens_in_bidegree(shift)
    num_lambda_gens = hom.source().number_of_gens_in_bidegree(shift + ext.LAMBDA_BIDEGREE)

    class_ = fp.FpVector(p, num_gens + num_lambda_gens)

    matrix = fp.Matrix(p, num_gens, 1)

    if hom.name() != "":
        if matrix.rows() == 0:
            print("No classes in this bidegree", file=sys.stderr)
        else:
            v = query.vector(f"Input Ext class {ext_name}", num_gens)
            for i, x in enumerate(v):
                matrix.row_mut(i).set_entry(0, x)
                class_.set_entry(i, x)

    hom.extend_step(shift, matrix)

    hom_lift = ext.SecondaryResolutionHomomorphism(source, target, hom)

    lambda_part = None
    if num_lambda_gens > 0:
        lambda_name = query.raw(f"Name of λ part of {name}", str)
        if lambda_name == "":
            lambda_part = None
        else:
            v = query.vector(f"Input Ext class {lambda_name}", num_lambda_gens)
            for i, x in enumerate(v):
                class_.set_entry(num_gens + i, x)
            lambda_part = ext.ResolutionHomomorphism.from_class(
                lambda_name,
                hom_lift.source(),
                hom_lift.target(),
                shift + ext.LAMBDA_BIDEGREE,
                v,
            )

    lambda_name = lambda_part.name() if lambda_part is not None else ""
    if ext_name == "" and lambda_name == "":
        raise AssertionError("Do not compute zero Massey product")
    elif ext_name == "":
        final_name = f"λ{lambda_name}"
    elif lambda_name == "":
        final_name = f"[{ext_name}]"
    else:
        final_name = f"[{ext_name}] + λ{lambda_name}"

    return HomData(final_name, class_, hom_lift, lambda_part)


def main():
    print(
        "We are going to compute <-, b, a> for all (-), where a is an element in Ext(M, k) and b "
        "and (-) are elements in Ext(k, k).",
        file=sys.stderr,
    )

    # standard backend: this example uses get_unit()/SecondaryResolution, unavailable on Nassau
    resolution = query.query_resolution(alg=algebra.AlgebraType.Milnor, algorithm="standard")
    resolution.compute_through_stem(query.query_n_s())

    is_unit, unit = ext.get_unit(resolution)

    p = resolution.prime

    res_lift = ext.SecondaryResolution(resolution)
    unit_lift = res_lift if is_unit else ext.SecondaryResolution(unit)

    a_data = get_hom("a", res_lift, unit_lift)
    a_name = a_data.name
    a = a_data.hom_lift
    a_lambda = a_data.lambda_part

    b_data = get_hom("b", unit_lift, unit_lift)
    b_name = b_data.name
    b_class = b_data.class_
    b = b_data.hom_lift
    b_lambda = b_data.lambda_part

    shift = sseq.Bidegree.s_t(
        (a.underlying().shift() + b.underlying().shift()).s,
        (a.shift() + b.shift()).t,
    )

    # Extend resolutions
    if not is_unit:
        res_max = sseq.Bidegree.n_s(
            resolution.module(0).max_computed_degree(),
            resolution.next_homological_degree() - 1,
        )
        unit.compute_through_stem(res_max - a.underlying().shift())

    if is_unit:
        res_lift.extend_all()
    else:
        res_lift.extend_all()
        unit_lift.extend_all()

    # Now extend homomorphisms
    a.underlying().extend_all()
    a.extend_all()
    b.underlying().extend_all()
    b.extend_all()
    if a_lambda is not None:
        a_lambda.extend_all()
    if b_lambda is not None:
        b_lambda.extend_all()

    res_sseq = res_lift.e3_page
    unit_sseq = res_sseq if is_unit else res_lift.e3_page

    b_shift = b.underlying().shift()

    chain_homotopy = ext.ChainHomotopy(a.underlying(), b.underlying())
    chain_homotopy.initialize_homotopies((b_shift + a.underlying().shift()).s)

    # Compute first homotopy
    v = a.product_nullhomotopy(a_lambda, res_sseq, b_shift, b_class)
    homotopy = chain_homotopy.homotopy(b_shift.s + a.underlying().shift().s - 1)
    htpy_source = a.shift() + b_shift
    homotopy.extend_by_zero(htpy_source.t - 1)
    homotopy.add_generators_from_rows(
        htpy_source.t,
        [fp.FpVector.from_slice(p, [x]) for x in v],
    )

    chain_homotopy.extend_all()

    ch_lift = ext.SecondaryChainHomotopy(
        a, b, chain_homotopy, a_lambda, b_lambda
    )

    secondary_job = os.environ.get("SECONDARY_JOB")
    if secondary_job is not None:
        ch_lift.compute_partial(int(secondary_job))
        return

    ch_lift.extend_all()

    def get_page_data(ss, b):
        # NOTE: depends on Subquotient accessors subspace_dimension / subspace_gens /
        # quotient_dimension / quotient_pivots / reduce_by_quotient (API_PROPOSAL §4 lists a
        # "full Subquotient pyclass" but does not enumerate these methods)
        #
        # Sseq.page_data(b, r) returns the E_r page subquotient (r indexed by page
        # number, starting at MIN_R = 2). We want the E3 page (r = 3), falling back
        # to the last computed page when d2 was the final differential.
        for r in (3, 2):
            try:
                return ss.page_data(b, r)
            except IndexError:
                continue
        raise IndexError(f"no computed page data at bidegree {b}")

    scratch1 = fp.FpVector(p, 0)

    h_0 = ch_lift.algebra().p_tilde()

    p_int = int(p)

    # Iterate through the multiplicand
    for c in unit.iter_stem():
        if not resolution.has_computed_bidegree(
            c + shift - sseq.Bidegree.s_t(2, 0)
        ) or not resolution.has_computed_bidegree(c + shift + sseq.Bidegree.s_t(0, 1)):
            continue

        # Now read off the products
        source = c + shift - sseq.Bidegree.s_t(1, 0)

        source_num_gens = resolution.number_of_gens_in_bidegree(source)
        source_lambda_num_gens = resolution.number_of_gens_in_bidegree(
            source + ext.LAMBDA_BIDEGREE
        )

        if source_num_gens + source_lambda_num_gens == 0:
            continue

        # We find the kernel of multiplication by b.
        target_num_gens = unit.number_of_gens_in_bidegree(c)
        target_lambda_num_gens = unit.number_of_gens_in_bidegree(c + ext.LAMBDA_BIDEGREE)
        target_all_gens = target_num_gens + target_lambda_num_gens

        prod_num_gens = unit.number_of_gens_in_bidegree(c + b_shift)
        prod_lambda_num_gens = unit.number_of_gens_in_bidegree(
            c + b_shift + ext.LAMBDA_BIDEGREE
        )
        prod_all_gens = prod_num_gens + prod_lambda_num_gens

        target_page_data = get_page_data(unit_sseq, c)
        target_lambda_page_data = get_page_data(unit_sseq, c + ext.LAMBDA_BIDEGREE)
        product_lambda_page_data = get_page_data(
            unit_sseq, c + b_shift + ext.LAMBDA_BIDEGREE
        )

        # We first compute elements whose product vanish mod lambda, and later see what the
        # possible lifts are. We do it this way to avoid Z/p^2 problems

        product_matrix = fp.Matrix(
            p,
            target_page_data.subspace_dimension(),
            target_num_gens + prod_num_gens,
        )

        m0 = fp.Matrix.from_vec(
            p,
            b.underlying().get_map(c.s + b.underlying().shift().s).hom_k(c.t),
        )
        for g, out in zip(target_page_data.subspace_gens(), product_matrix.iter_mut()):
            out.slice_mut(prod_num_gens, prod_num_gens + target_num_gens).add(g, 1)
            for i, val in g.iter_nonzero():
                out.slice_mut(0, prod_num_gens).add(m0.row(i), val)
        product_matrix.row_reduce()
        e2_kernel = product_matrix.compute_kernel(prod_num_gens)

        # Now compute the e3 kernel
        e2_ker_dim = e2_kernel.dimension()
        product_matrix = fp.Matrix(
            p,
            e2_ker_dim + target_lambda_page_data.quotient_dimension(),
            target_all_gens + prod_all_gens,
        )

        b.hom_k_with(
            b_lambda,
            unit_sseq,
            c,
            e2_kernel.basis(),
            list(product_matrix.slice_mut(0, e2_ker_dim, 0, prod_all_gens).iter_mut()),
        )
        for vec, t in zip(e2_kernel.basis(), product_matrix.iter_mut()):
            t.slice_mut(prod_all_gens, prod_all_gens + target_num_gens).assign(vec)

        # Now add the lambda multiples
        m = fp.Matrix.from_vec(
            p,
            b.underlying().get_map(b_shift.s + c.s + 1).hom_k(c.t + 1),
        )

        count = 0
        for i, vv in enumerate(target_lambda_page_data.quotient_pivots()):
            if vv >= 0:
                continue
            row = product_matrix.row_mut(e2_ker_dim + count)
            row.add_basis_element(prod_all_gens + target_num_gens + i, 1)
            row.slice_mut(prod_num_gens, prod_all_gens).add(m.row(i), 1)
            product_lambda_page_data.reduce_by_quotient(
                row.slice_mut(prod_num_gens, prod_all_gens)
            )
            count += 1

        product_matrix.row_reduce()
        e3_kernel = product_matrix.compute_kernel(prod_all_gens)

        if e3_kernel.dimension() == 0:
            continue

        m0 = chain_homotopy.homotopy(source.s).hom_k(c.t)
        mt = fp.Matrix.from_vec(
            p, chain_homotopy.homotopy(source.s + 1).hom_k(c.t + 1)
        )
        m1 = fp.Matrix.from_vec(
            p, ch_lift.homotopies()[source.s + 1].homotopies.hom_k(c.t)
        )
        mp = fp.Matrix.from_vec(
            p,
            resolution.filtration_one_product(
                1, h_0, sseq.Bidegree.s_t(source.s, c.t + shift.t)
            ),
        )
        ma = a.underlying().get_map(source.s).hom_k(c.t + b_shift.t)
        mb = b.underlying().get_map(c.s + b_shift.s).hom_k(c.t)

        for g in e3_kernel.iter():
            # Print name
            print(f"<{a_name}, {b_name}, ", end="")
            ext_part = g.restrict(0, target_num_gens)
            has_ext = sum(1 for _ in ext_part.iter_nonzero()) > 0
            if has_ext:
                basis_string = sseq.BidegreeElement(
                    c, ext_part.to_owned()
                ).to_basis_string()
                print(f"[{basis_string}]", end="")

            lambda_part = g.restrict(target_num_gens, target_all_gens)
            num_entries = sum(1 for _ in lambda_part.iter_nonzero())
            if num_entries > 0:
                if has_ext:
                    print(" + ", end="")
                print("λ", end="")
                basis_string = sseq.BidegreeElement(
                    c + ext.LAMBDA_BIDEGREE,
                    g.restrict(target_num_gens, target_all_gens).to_owned(),
                ).to_basis_string()
                if num_entries == 1:
                    print(f"{basis_string}", end="")
                else:
                    print(f"({basis_string})", end="")
            print("> = ±", end="")

            scratch0 = [0] * source_num_gens
            scratch1.set_scratch_vector_size(source_lambda_num_gens)

            # First deal with the null-homotopy of ab
            for i, val in g.restrict(0, target_num_gens).iter_nonzero():
                for k in range(source_num_gens):
                    scratch0[k] += val * m0[i][k]
                scratch1.as_slice_mut().add(m1.row(i), val)
            for i, val in g.restrict(target_num_gens, target_all_gens).iter_nonzero():
                scratch1.as_slice_mut().add(mt.row(i), val)
            # Now do the -1 part of the null-homotopy of bc.
            sign = p_int * p_int - 1
            out = b.product_nullhomotopy(b_lambda, unit_sseq, c, g)
            for i, val in out.iter_nonzero():
                for k in range(source_num_gens):
                    scratch0[k] += val * ma[i][k] * sign

            for i, val in enumerate(scratch0):
                extra = val // p_int
                scratch1.as_slice_mut().add(mp.row(i), extra % p_int)

            print(f"[{', '.join(str(x % p_int) for x in scratch0)}]", end="")

            # Then deal with the rest of the null-homotopy of bc. This is just the null-homotopy of
            # 2.
            scratch0 = [0] * prod_num_gens

            for i, val in g.restrict(0, target_num_gens).iter_nonzero():
                for k in range(prod_num_gens):
                    scratch0[k] += val * mb[i][k]
            for i, val in enumerate(scratch0):
                extra = (val // p_int) % p_int
                if extra == 0:
                    continue
                for gen_idx in range(source_lambda_num_gens):
                    mm = a.underlying().get_map((source + ext.LAMBDA_BIDEGREE).s)
                    dx = mm.output((source + ext.LAMBDA_BIDEGREE).t, gen_idx)
                    idx = unit.module((c + shift).s).operation_generator_to_index(
                        1, h_0, (c + shift).t, i
                    )
                    scratch1.add_basis_element(gen_idx, dx.entry(idx))
            print(f" + λ{scratch1}")


if __name__ == "__main__":
    main()
