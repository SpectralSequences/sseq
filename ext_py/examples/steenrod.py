#!/usr/bin/env python3
"""Compute Steenrod operations on an Ext class of the sphere.

Python port of ext/examples/steenrod.rs.
"""

import sys

import _query as query
import ext
from ext import fp, sseq
from ext.algebra import FreeModuleHomomorphism

# NOTE: depends on TensorChainComplex / SumModule / TensorChainMap (not in
# API_PROPOSAL / requires Rust-side machinery). steenrod.rs defines these as
# inline modules (`sum_module`, `tensor_product_chain_complex`) implementing the
# `Module`, `ChainComplex` and `ModuleHomomorphism` traits with custom
# block-structure / tensor-product / quasi-inverse logic. They cannot be
# expressed as a thin Python composition of the proposed bindings; the flow
# below is ported faithfully against the proposed API, assuming an
# `ext.TensorChainComplex` providing `.new`, `.module`, `.differential`,
# `.compute_through_bidegree` and the `.swap(result, vec, bidegree)` helper that
# the example's `TensorChainComplex<A, CC, CC>` exposes.


def main():
    # ext::utils::init_logging() -> stderr only; no stdout effect.

    resolution = query.query_resolution("Module", None)
    module = resolution.target().module(0)
    p = resolution.prime

    if resolution.target().max_s() != 1 or not module.is_unit() or p != 2:
        raise AssertionError("Can only run Steenrod on the sphere")

    b = sseq.Bidegree.n_s(
        query.raw("n of Ext class", int),
        query.raw("s of Ext class", int),
    )

    resolution.compute_through_bidegree(b + b)

    class_ = query.vector("Input Ext class", resolution.number_of_gens_in_bidegree(b))

    yoneda = ext.yoneda_representative_element(resolution, b, class_)

    print("Dimensions of Yoneda representative: 1", end="")
    for s in range(b.s + 1):
        print(f" {yoneda.module(s).total_dimension()}", end="")
    print()

    # NOTE: depends on TensorChainComplex (not in API_PROPOSAL / requires
    # Rust-side machinery).
    square = ext.TensorChainComplex.new(yoneda, yoneda)
    doubled_b = b + b

    # tracing::info_span!("Computing quasi-inverses") -> stderr only.
    square.compute_through_bidegree(doubled_b)
    for s in range(doubled_b.s + 1):
        square.differential(s).compute_auxiliary_data_through_degree(doubled_b.t)

    print("Computing Steenrod operations: ", file=sys.stderr)

    delta = []

    for i in range(b.s + 1):
        maps = []
        for s in range(doubled_b.s - i + 1):
            source = resolution.module(s)
            target = square.module(s + i)
            m = FreeModuleHomomorphism(source, target, 0)
            maps.append(m)
        delta.append(maps)

    # tracing::info_span!("Computing Steenrod operations") -> stderr only.

    # We use the formula d Delta_i + Delta_i d = Delta_{i-1} + tau Delta_{i-1}
    for i in range(b.s + 1):
        shift_s = sseq.Bidegree.s_t(i, 0)
        # Delta_i is a map C_s -> C_{s + i}. So to hit C_{2s}, we only need to
        # compute up to 2 * s - i.
        # std::time::Instant::now() -> only used for the stderr timing print.

        for s in range((doubled_b - shift_s).s + 1):
            if i == 0 and s == 0:
                m = delta[0][0]
                m.add_generators_from_matrix_rows(
                    0, fp.Matrix.from_vec(p, [[1]]).slice_mut(0, 1, 0, 1)
                )
                m.extend_by_zero(doubled_b.t)
                continue

            source = resolution.module(s)
            target = square.module(s + i)

            dtarget_module = square.module(s + i - 1)

            d_res = resolution.differential(s)
            d_target = square.differential(s + i)

            m = delta[i][s]
            prev_map = None if s == 0 else delta[i][s - 1]
            prev_delta = None if i == 0 else delta[i - 1][s]

            for t in range(doubled_b.t + 1):
                bb = sseq.Bidegree.s_t(s, t)

                num_gens = source.number_of_gens_in_degree(t)

                fx_dim = target.dimension(t)
                fdx_dim = dtarget_module.dimension(t)

                if fx_dim == 0 or fdx_dim == 0 or num_gens == 0:
                    m.extend_by_zero(t)
                    continue

                output_matrix = fp.Matrix(p, num_gens, fx_dim)
                result = fp.FpVector(p, fdx_dim)
                for j in range(num_gens):
                    if prev_delta is not None:
                        # Delta_{i-1} x
                        prevd = prev_delta.output(t, j)

                        # tau Delta_{i-1} x
                        # NOTE: depends on TensorChainComplex.swap (not in
                        # API_PROPOSAL / requires Rust-side machinery).
                        square.swap(
                            result, prevd, bb + shift_s - sseq.Bidegree.s_t(1, 0)
                        )
                        result.add(prevd.as_slice(), 1)

                    if prev_map is not None:
                        dx = d_res.output(t, j)
                        prev_map.apply(
                            result.slice_mut(0, fdx_dim), 1, t, dx.as_slice()
                        )

                    assert d_target.apply_quasi_inverse(
                        output_matrix.row_mut(j), t, result.as_slice()
                    )

                    result.set_to_zero()
                m.add_generators_from_matrix_rows(
                    t, output_matrix.slice_mut(0, num_gens, 0, fx_dim)
                )

        final_map = delta[i][(doubled_b - shift_s).s]
        num_gens = resolution.number_of_gens_in_bidegree(doubled_b - shift_s)
        basis_string = sseq.BidegreeElement(
            b, fp.FpVector.from_slice(p, class_)
        ).to_basis_string()
        result = ", ".join(
            str(final_map.output(doubled_b.t, k).entry(0)) for k in range(num_gens)
        )
        print(f"Sq^{(b - shift_s).s} {basis_string} = [{result}]", end="")
        sys.stdout.flush()
        # eprint!(" ({:?})", start.elapsed()) -> stderr timing only.
        sys.stderr.flush()
        print()


if __name__ == "__main__":
    main()
