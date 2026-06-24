#!/usr/bin/env python3
"""Compute the action of Sq^0 on Ext^{*,*}(F_2) for the sphere at the prime 2.

Python port of ext/examples/sq0.rs.
"""

import _query as query
import ext
from ext import fp, sseq

# NOTE: sq0 requires the inline `double` chain-complex machinery from
# ext/examples/sq0.rs, which is NOT part of API_PROPOSAL.md. This port assumes a
# future binding exposing DoubleChainComplex(resolution); it cannot run until
# that (or a Python-subclassable ChainComplex) is provided.


def main():
    # query.query_module mirrors `utils::query_module(None, true)`; the shim
    # ignores the `load_quasi_inverse` flag, which is fine here.
    res = query.query_module(None)
    assert (
        res.prime() == 2
        and res.target().max_s() == 1
        and res.target().module(0).is_unit()
    ), "Sq^0 can only be computed for the sphere at the prime 2"

    # NOTE: depends on a future `DoubleChainComplex` binding (see top-of-file
    # note). The doubled chain complex halves Steenrod operations degree-wise.
    doubled = ext.DoubleChainComplex(res)
    doubled.compute_through_bidegree(
        sseq.Bidegree.s_t(res.next_homological_degree() - 1, 0)
    )

    hom = ext.ResolutionHomomorphism(
        "Sq^0",
        res,
        doubled,
        sseq.Bidegree.zero(),
    )
    # NOTE: depends on `ResolutionHomomorphism.extend_step_raw`, which is not
    # explicitly listed in API_PROPOSAL.md (§7.3 lists `extend_step`). The Rust
    # call passes an optional list of output FpVectors for the first step.
    hom.extend_step_raw(
        sseq.Bidegree.zero(),
        [fp.FpVector.from_slice(res.prime(), [1])],
    )
    hom.extend_all()

    for b in res.iter_nonzero_stem():
        doubled_b = sseq.Bidegree.s_t(b.s(), 2 * b.t())
        if not res.has_computed_bidegree(doubled_b):
            continue

        source_num_gens = res.number_of_gens_in_bidegree(doubled_b)
        module = res.module(b.s())
        offset = module.generator_offset(b.t(), b.t(), 0)
        map = hom.get_map(b.s())

        for i in range(res.number_of_gens_in_bidegree(b)):
            g = sseq.BidegreeGenerator(b, i)
            entries = [
                str(map.output(doubled_b.t(), j).entry(offset + i))
                for j in range(source_num_gens)
            ]
            print(f"Sq^0 x_{g} = [{', '.join(entries)}]")


if __name__ == "__main__":
    main()
