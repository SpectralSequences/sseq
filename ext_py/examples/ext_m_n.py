#!/usr/bin/env python3
"""Compute Ext(M, N) and print an ASCII depiction.

Python port of ext/examples/ext_m_n.rs.
"""

import sys

import _query as query
import ext
from ext import algebra, fp, sseq


class HomCochainComplex:
    """Cochain complex Hom(R_*, N) over HomModule + HomPullback.

    Mirrors the inline `hom_cochain_complex` module in ext_m_n.rs: the source is
    a free chain complex (a Resolution) and the target is a module N; term s is
    HomModule(R_s, N) and the codifferential s -> s+1 is the HomPullback of the
    resolution differential d_{s+1}.
    """

    def __init__(self, source, target):
        self.source = source
        self.target = target
        self.modules = []
        self.differentials = []

    def min_degree(self):
        return self.modules[0].min_degree()

    def compute_through_stem(self, max):
        for s in range(len(self.modules), max.s + 1):
            self.modules.append(
                algebra.HomModule(self.source.module(s), self.target)
            )
        for s in range(len(self.differentials), max.s):
            self.differentials.append(
                algebra.HomPullback(
                    self.modules[s],
                    self.modules[s + 1],
                    self.source.differential(s + 1),
                )
            )
        for s, module in enumerate(self.modules):
            module.compute_basis(max.n + s + 1)
        for s, d in enumerate(self.differentials):
            d.compute_auxiliary_data_through_degree(max.n + s + 1)

    def homology_dimension(self, b):
        if b.s == 0:
            return self.differentials[b.s].kernel(b.t).dimension()
        # NOTE: depends on Subquotient.from_parts (not yet in API_PROPOSAL §4.4)
        return fp.Subquotient.from_parts(
            self.differentials[b.s].kernel(b.t),
            self.differentials[b.s - 1].image(b.t),
        ).dimension()


def main():
    print("This script computes Ext(M, N)", file=sys.stderr)
    res = query.query_module_only("Module M")
    module_spec = query.raw("Module N", ext.parse_module_name)
    module = algebra.steenrod_module_from_json(res.algebra(), module_spec)

    max = sseq.Bidegree.n_s(
        query.raw("Max n", int),
        query.raw("Max s", int),
    )

    res.compute_through_stem(max + sseq.Bidegree.n_s(module.max_degree(), 1))
    res.algebra().compute_basis(max.t + module.max_degree() + 2)

    hom_cc = HomCochainComplex(res, module)
    hom_cc.compute_through_stem(max)

    # FreeChainComplex::graded_dimension_string
    result = ""
    for s in range(max.s, -1, -1):
        for n in range(hom_cc.min_degree(), max.n + 1):
            b = sseq.Bidegree.n_s(n, s)
            result += ext.unicode_num(hom_cc.homology_dimension(b))
            result += " "
        result += "\n"
        # If it is empty so far, don't print anything
        if result.lstrip() == "":
            result = ""
    print(result, end="")


if __name__ == "__main__":
    main()
