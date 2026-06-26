#!/usr/bin/env python3
"""Save a resolution to Bruner's format in the current working directory.

Python port of ext/examples/save_bruner.rs.
"""

import _query as query
from ext.algebra import AlgebraType
# NOTE: depends on MilnorAlgebra-typed resolution algebra and a SteenrodAlgebra ->
# MilnorAlgebra conversion (API_PROPOSAL §5.2); also on Resolution methods
# next_homological_degree / module / differential / prime and FreeModule
# {min_degree, max_computed_degree, number_of_gens_in_degree, generator_offset} and
# FreeModuleHomomorphism.output (API_PROPOSAL §7.1/§7.2/§5.3/§5.4).


def main():
    # standard backend: this example uses module(), unavailable on Nassau
    resolution = query.query_resolution(alg=AlgebraType.Milnor, algorithm="standard")
    resolution.compute_through_stem(query.query_n_s())

    assert resolution.prime == 2
    # Rust views the resolution's SteenrodAlgebra as a &MilnorAlgebra via try_into.
    # No such conversion is bound; we use the resolution's algebra directly, which
    # is the Milnor algebra and exposes dimension / basis_element_from_index.
    alg = resolution.algebra()

    for s in range(resolution.next_homological_degree()):
        with open(f"hDiff.{s}", "w") as f:
            module = resolution.module(s)
            # We don't use this when s = 0
            dmodule = resolution.module(max(s - 1, 0))
            min_degree = module.min_degree()
            max_degree = module.max_computed_degree()
            num_gens = sum(
                module.number_of_gens_in_degree(t)
                for t in range(min_degree, max_degree + 1)
            )

            f.write(f"        {num_gens}        {max_degree}\n\n")

            d = resolution.differential(s)
            for t in range(min_degree, max_degree + 1):
                for idx in range(module.number_of_gens_in_degree(t)):
                    f.write(f"{t}\n\n")

                    if s == 0:
                        f.write("1\n0 0 1 i(0).\n\n\n\n")
                        continue
                    row_count = 0
                    buffer = ""
                    dx = d.output(t, idx)

                    gen_count = 0
                    for gen_deg in range(min_degree, t):
                        for gen_idx in range(dmodule.number_of_gens_in_degree(gen_deg)):
                            op_deg = t - gen_deg
                            algebra_dim = alg.dimension(op_deg)
                            start = dmodule.generator_offset(t, gen_deg, gen_idx)
                            slice = dx.slice(start, start + algebra_dim)
                            if slice.is_zero():
                                gen_count += 1
                                continue
                            row_count += 1
                            buffer += f"{gen_count} {op_deg} {algebra_dim} i"
                            for op_idx, _ in slice.iter_nonzero():
                                elt = alg.basis_element_from_index(op_deg, op_idx)
                                buffer += "(" + ",".join(str(x) for x in elt.p_part) + ")"
                            buffer += ".\n"
                            gen_count += 1
                    f.write(f"{row_count}\n")
                    # buffer has one new line, writeln has one new line, add another one.
                    f.write(f"{buffer}\n")


if __name__ == "__main__":
    main()
