#!/usr/bin/env python3
"""Interactively define a finite dimensional or finitely presented module.

The module JSON is printed to stdout; all prompts go to stderr.

Python port of ext/examples/define_module.rs.
"""

import json
import sys

import _query as query
from ext import algebra


def get_gens():
    """Query generators and their degrees. Returns a dict ``degree -> [names]``."""
    print("Input generators. Press return to finish.", file=sys.stderr)

    gens = {}
    while True:
        gen_deg = query.optional("Generator degree", int)
        if gen_deg is None:
            print("This is the list of generators and degrees:", file=sys.stderr)
            for deg in sorted(gens):
                for g in gens[deg]:
                    print(f"({deg}, {g}) ", end="", file=sys.stderr)
            print(file=sys.stderr)
            if query.yes_no("Is it okay?"):
                break
            if query.yes_no("Start over?"):
                gens = {}
            continue

        gens.setdefault(gen_deg, [])
        default = f"x{gen_deg}{len(gens[gen_deg])}".replace("-", "_")

        def parse_name(x):
            if not x:
                raise ValueError("Variable name cannot be empty")
            if not x[0].isalpha():
                raise ValueError("variable name must start with a letter")
            for c in x:
                if not (c.isalnum() or c == "_"):
                    raise ValueError(
                        f"Variable name cannot contain {c}. "
                        "Should be alphanumeric and '_'"
                    )
            return x

        gens[gen_deg].append(query.with_default("Generator name", default, parse_name))

    return gens


def define_fdmodule(output_json, p):
    output_json["p"] = p
    alg = algebra.AdemAlgebra(p, False)

    gens = get_gens()
    min_degree = min(gens) if gens else 0
    max_degree = (max(gens) + 1) if gens else 0

    alg.compute_basis(max_degree - min_degree)

    # Create module
    graded_dims = [len(gens.get(i, [])) for i in range(min_degree, max_degree)]
    module = algebra.FDModuleBuilder(alg, "", graded_dims, min_degree)

    for deg in sorted(gens):
        for j, g in enumerate(gens[deg]):
            module.set_basis_element_name(deg, j, g)

    print(
        "Input actions. Write the value of the action in the form 'a x0 + b x1 + "
        "...' where a, b are non-negative integers and x0, x1 are names of the "
        "generators. The coefficient can be omitted if it is 1",
        file=sys.stderr,
    )

    for input_deg in reversed(range(min_degree, max_degree)):
        for output_deg in range(input_deg + 1, max_degree):
            op_deg = output_deg - input_deg
            out_gens = gens.get(output_deg, [])
            if not out_gens:
                continue
            for op_idx in alg.generators(op_deg):
                for input_idx in range(len(gens.get(input_deg, []))):

                    def parse_action(expr):
                        result = [0] * len(out_gens)
                        if expr == "0":
                            return result
                        for term in expr.split("+"):
                            term = term.strip()
                            if " " in term:
                                coef_str, g = term.split(" ", 1)
                                coef = int(coef_str)
                            else:
                                coef, g = 1, term
                            if g in out_gens:
                                result[out_gens.index(g)] += coef
                            else:
                                raise ValueError(
                                    f"No generator {g} in degree {output_deg}"
                                )
                        return result

                    op_string = alg.basis_element_to_string(op_deg, op_idx)
                    output = query.raw(
                        f"{op_string} {gens[input_deg][input_idx]}", parse_action
                    )
                    module.set_action(op_deg, op_idx, input_deg, input_idx, output)
            module.extend_actions(input_deg, output_deg)
            module.check_validity(input_deg, output_deg)

    output_json.update(module.to_json())


def replace(algebra_elt, g):
    """Right-multiply each term of an algebra element string by ``g``."""
    return algebra_elt.replace("+", f"{g} +") + " " + g


def define_fpmodule(output_json, p):
    gens = get_gens()
    ev = algebra.SteenrodEvaluator(p)

    print("Input relations", file=sys.stderr)
    if p == 2:
        print("Write relations in the form 'Sq6 * Sq2 * x + Sq7 * y'", file=sys.stderr)
    else:
        print(
            "Write relations in the form 'Q5 * P(5) * x + 2 * P(1, 3) * Q2 * y', "
            "where P(...) and Qi are Milnor basis elements.",
            file=sys.stderr,
        )

    degree_lookup = {g: deg for deg, gs in gens.items() for g in gs}

    adem_relations = []
    milnor_relations = []
    while True:

        def parse_relation(rel):
            result = ev.evaluate_module_adem(rel)
            if not result:
                return result
            degrees = []
            for g, (op_deg, _) in sorted(result.items()):
                if g not in degree_lookup:
                    raise ValueError(f"Unknown generator: {g}")
                degrees.append(degree_lookup[g] + op_deg)
            for a, b in zip(degrees, degrees[1:]):
                if a != b:
                    raise ValueError(
                        f"Relation terms have different degrees: {a} and {b}"
                    )
            return result

        relation = query.raw("Enter relation", parse_relation)
        if not relation:
            break

        adem_terms = []
        milnor_terms = []
        for g, (op_deg, adem_op) in sorted(relation.items()):
            if adem_op.is_zero():
                continue
            milnor_op = ev.adem_to_milnor(op_deg, adem_op)
            adem_terms.append(replace(ev.adem_element_to_string(op_deg, adem_op), g))
            milnor_terms.append(
                replace(ev.milnor_element_to_string(op_deg, milnor_op), g)
            )
        if adem_terms:
            adem_relations.append(" + ".join(adem_terms))
            milnor_relations.append(" + ".join(milnor_terms))

    output_json["p"] = p
    output_json["type"] = "finitely presented module"
    output_json["gens"] = {g: deg for deg in sorted(gens) for g in gens[deg]}
    output_json["adem_relations"] = adem_relations
    output_json["milnor_relations"] = milnor_relations


def main():
    def parse_type(x):
        if x in ("fd", "fp"):
            return x
        raise ValueError(f"Invalid type '{x}'. Type must be 'fd' or 'fp'")

    module_type = query.with_default(
        "Input module type (default 'finite dimensional module'):\n (fd) - finite "
        "dimensional module \n (fp) - finitely presented module\n",
        "fd",
        parse_type,
    )

    p = query.with_default("p", "2", int)
    output_json = {}

    print(f"module_type: {module_type}", file=sys.stderr)
    if module_type == "fd":
        define_fdmodule(output_json, p)
    else:
        define_fpmodule(output_json, p)

    print(json.dumps(output_json, separators=(",", ":"), ensure_ascii=False))


if __name__ == "__main__":
    main()
