#!/usr/bin/env python3
"""Resolve an unstable module up to an (n, s) and print an ASCII depiction of Ext.

Python port of ext/examples/resolve_unstable.rs.
"""

import _query as query
import ext
from ext import algebra, sseq


def query_unstable_module(load_quasi_inverse):
    """Inline mirror of ext::utils::query_unstable_module.

    Queries a single "Module" spec, parses the optional ``@adem``/``@milnor``
    algebra suffix (default Milnor) and the module name, builds the algebra with
    ``unstable=True`` and the corresponding Steenrod module, then wraps it in a
    bounded chain complex and an ``UnstableResolution`` with an optional save
    directory.
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
        # NOTE: depends on ext.parse_module_name (API_PROPOSAL §7.7).
        module = ext.parse_module_name(module_name)
        return (module, algebra_type)

    module_json, algebra_type = query.raw("Module", parse_spec)
    alg = algebra.SteenrodAlgebra.from_json(module_json, algebra_type, True)
    module = algebra.SteenrodModule.from_spec(module_json, alg)

    # NOTE: depends on ext.ChainComplex.ccdz and
    # ext.UnstableResolution.new_with_save (API_PROPOSAL §7.1, §7.2).
    cc = ext.ChainComplex.ccdz(module)

    save_dir = query.optional("Module save directory", str)

    resolution = ext.UnstableResolution(cc, save_dir=save_dir)
    resolution.load_quasi_inverse = load_quasi_inverse and resolution.save_dir() is None

    return resolution


def main():
    res = query_unstable_module(False)

    max = sseq.Bidegree.n_s(
        query.raw("Max n", int),
        query.raw("Max s", int),
    )

    res.compute_through_stem(max)

    print(res.graded_dimension_string())


if __name__ == "__main__":
    main()
