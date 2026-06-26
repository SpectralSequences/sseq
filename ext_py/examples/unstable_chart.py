#!/usr/bin/env python3
"""Compute and chart the suspension maps between unstable Ext groups.

Given an unstable Steenrod module M, compute the unstable Ext groups of the
suspensions of M for all shifts up to the stable range, writing a TikZ figure
for each shift to stdout.

Python port of ext/examples/unstable_chart.rs.
"""

import os
import sys

import _query as query
import ext
from ext import algebra, sseq


def query_unstable_module_only():
    """Inline mirror of ext::utils::query_unstable_module_only.

    Queries a single "Module" spec, parses the optional ``@adem``/``@milnor``
    algebra suffix (default Milnor) and the module name, builds the algebra with
    ``unstable=True`` and returns the corresponding Steenrod module.
    """

    def parse_spec(spec):
        # Mirror Config::try_from(&str): split on '@' for the algebra type.
        module_name, _, algebra_name = spec.partition("@")
        if algebra_name == "":
            algebra_type = algebra.AlgebraType.Milnor
        elif algebra_name == "adem":
            algebra_type = algebra.AlgebraType.Adem
        elif algebra_name == "milnor":
            algebra_type = algebra.AlgebraType.Milnor
        else:
            raise ValueError(f"Invalid algebra type: {algebra_name}")
        # NOTE: depends on ext.parse_module_name (API_PROPOSAL §7.4).
        module = ext.parse_module_name(module_name)
        return (module, algebra_type)

    module_json, algebra_type = query.raw("Module", parse_spec)
    alg = algebra.SteenrodAlgebra.from_json(module_json, algebra_type, True)
    return algebra.SteenrodModule.from_spec(module_json, alg)


def main():
    module = query_unstable_module_only()

    # Mirror the `save_dir` closure: an optional base directory under which each
    # shift gets its own `suspension{shift}` subdirectory.
    base = query.optional("Module save directory", str)

    def save_dir(shift):
        if base is None:
            return None
        return os.path.join(base, f"suspension{shift}")

    max = sseq.Bidegree.n_s(
        query.raw("Max n", int),
        query.raw("Max s", int),
    )

    disp_template = query.raw(
        "LaTeX name template (replace % with min degree)",
        str,
    )

    products = module.algebra().default_filtration_one_products()

    for shift_t in range(0, max.n - module.min_degree() + 3):
        shift = sseq.Bidegree.s_t(0, shift_t)
        # NOTE: depends on ext.SuspensionModule, ext.ChainComplex.ccdz and
        # ext.UnstableResolution.new_with_save (API_PROPOSAL §7.1, §7.3, §7.5).
        res = ext.UnstableResolution(
            ext.ChainComplex.ccdz(
                algebra.SuspensionModule(module, shift.t)
            ),
            save_dir=save_dir(shift.t),
        )

        res.compute_through_stem(max + shift)

        print("\\begin{figure}[p]\\centering")

        ss = res.to_sseq()
        shift_products = [
            (name, res.filtration_one_products(op_deg, op_idx))
            for (name, op_deg, op_idx) in products
        ]

        def header(g, shift_t=shift_t):
            return g.text(
                sseq.Bidegree.x_y(1, max.s - 1),
                disp_template.replace("%", f"{shift_t}"),
                sseq.Orientation.Right,
            )

        ss.write_to_graph(
            sseq.TikzBackend(sys.stdout),
            2,
            False,
            shift_products,
            header,
        )

        print("\\end{figure}")


if __name__ == "__main__":
    main()
