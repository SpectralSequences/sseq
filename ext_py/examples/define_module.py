#!/usr/bin/env python3
"""
Interactive module definition tool for creating finite dimensional or finitely presented modules.
Python translation of define_module.rs example.
"""

import ext
import json
from typing import Dict, List, Any


def get_generators() -> Dict[int, List[str]]:
    """Interactively get generators from user."""
    print("Input generators. Press return to finish.")

    generators = {}

    while True:
        degree_input = input("Generator degree (or press Enter to finish): ").strip()
        if not degree_input:
            if generators:
                print("Generators and degrees:")
                for deg, gens in generators.items():
                    for gen in gens:
                        print(f"({deg}, {gen})", end=" ")
                print()

                if input("Is it okay? (y/n): ").lower().startswith("y"):
                    break
                else:
                    if input("Start over? (y/n): ").lower().startswith("y"):
                        generators = {}
                    continue
            else:
                break

        try:
            degree = int(degree_input)
        except ValueError:
            print("Invalid degree. Please enter an integer.")
            continue

        if degree not in generators:
            generators[degree] = []

        default_name = f"x{degree}{len(generators[degree])}".replace("-", "_")
        gen_name = (
            input(f"Generator name (default '{default_name}'): ").strip()
            or default_name
        )

        # Validate generator name
        if not gen_name[0].isalpha():
            print("Variable name must start with a letter")
            continue
        if not all(c.isalnum() or c == "_" for c in gen_name):
            print("Variable name must be alphanumeric with underscores only")
            continue

        generators[degree].append(gen_name)

    return generators


def define_finite_dimensional_module(prime: int) -> Dict[str, Any]:
    """Define a finite dimensional module interactively."""
    output = {"p": prime, "type": "finite dimensional module"}

    # Create algebra
    alg = ext.AdemAlgebra(prime=prime, truncated=False)

    # Get generators
    generators = get_generators()
    min_degree = min(generators.keys()) if generators else 0
    max_degree = max(generators.keys()) if generators else 0

    # Compute algebra basis
    alg.compute_basis(max_degree - min_degree)

    # Create module
    graded_dims = [
        len(generators.get(i, [])) for i in range(min_degree, max_degree + 1)
    ]
    module = ext.FDModuleBuilder(alg, "", graded_dims, min_degree)

    # Set generator names
    for degree, gen_list in generators.items():
        for idx, gen_name in enumerate(gen_list):
            module.set_basis_element_name(degree, idx, gen_name)

    # Input actions
    print(
        "Input actions. Write the value of the action in the form 'a x0 + b x1 + ...'"
    )
    print("where a, b are non-negative integers and x0, x1 are generator names.")
    print("The coefficient can be omitted if it is 1.")

    for input_deg in sorted(generators.keys(), reverse=True):
        for output_deg in range(input_deg + 1, max_degree + 1):
            if output_deg not in generators or not generators[output_deg]:
                continue

            op_deg = output_deg - input_deg

            for op_idx in alg.generators(op_deg):
                for input_idx, input_gen in enumerate(generators[input_deg]):
                    op_string = alg.basis_element_to_string(op_deg, op_idx)
                    prompt = f"{op_string} {input_gen}: "

                    while True:
                        expression = input(prompt).strip()
                        try:
                            result = parse_linear_combination(
                                expression, generators[output_deg]
                            )
                            module.set_action(
                                op_deg, op_idx, input_deg, input_idx, result
                            )
                            break
                        except ValueError as e:
                            print(f"Error: {e}")

            module.extend_actions(input_deg, output_deg)
            module.check_validity(input_deg, output_deg)

    # Convert to JSON
    output.update(module.to_json())
    return output


def define_finitely_presented_module(prime: int) -> Dict[str, Any]:
    """Define a finitely presented module interactively."""
    output = {"p": prime, "type": "finitely presented module"}

    # Get generators
    generators = get_generators()

    # Create evaluator
    evaluator = ext.SteenrodEvaluator(prime)

    # Set up generator lookup
    generator_degrees = {}
    for degree, gen_list in generators.items():
        for gen in gen_list:
            generator_degrees[gen] = degree

    # Input relations
    print("Input relations")
    if prime == 2:
        print("Write relations in the form 'Sq6 * Sq2 * x + Sq7 * y'")
    else:
        print("Write relations in the form 'Q5 * P(5) * x + 2 * P(1, 3) * Q2 * y'")
        print("where P(...) and Qi are Milnor basis elements.")

    adem_relations = []
    milnor_relations = []

    while True:
        relation = input("Enter relation (or press Enter to finish): ").strip()
        if not relation:
            break

        try:
            # Parse and validate relation
            result = evaluator.evaluate_module_adem(relation)

            if not result:
                continue

            # Check degrees are consistent
            degrees = []
            for gen, (op_deg, _) in result.items():
                total_deg = op_deg + generator_degrees[gen]
                degrees.append(total_deg)

            if len(set(degrees)) > 1:
                print(f"Error: Relation terms have different degrees: {degrees}")
                continue

            # Convert to string representations
            adem_terms = []
            milnor_terms = []

            for gen, (op_deg, adem_op) in result.items():
                if not adem_op.is_zero():
                    adem_str = evaluator.adem_element_to_string(op_deg, adem_op)
                    milnor_op = evaluator.adem_to_milnor(op_deg, adem_op)
                    milnor_str = evaluator.milnor_element_to_string(op_deg, milnor_op)

                    adem_terms.append(f"{adem_str} {gen}")
                    milnor_terms.append(f"{milnor_str} {gen}")

            if adem_terms:
                adem_relations.append(" + ".join(adem_terms))
                milnor_relations.append(" + ".join(milnor_terms))

        except Exception as e:
            print(f"Error parsing relation: {e}")

    # Build output JSON
    output["gens"] = {
        gen: deg for deg, gen_list in generators.items() for gen in gen_list
    }
    output["adem_relations"] = adem_relations
    output["milnor_relations"] = milnor_relations

    return output


def parse_linear_combination(expression: str, generator_names: List[str]) -> List[int]:
    """Parse a linear combination like 'a x0 + b x1' into coefficients."""
    if expression == "0":
        return [0] * len(generator_names)

    result = [0] * len(generator_names)

    for term in expression.split("+"):
        term = term.strip()
        if " " in term:
            coef_str, gen = term.split(" ", 1)
            coef = int(coef_str)
        else:
            coef = 1
            gen = term

        try:
            gen_idx = generator_names.index(gen)
            result[gen_idx] += coef
        except ValueError:
            raise ValueError(f"Unknown generator: {gen}")

    return result


def main():
    # Get module type
    print("Input module type:")
    print("(fd) - finite dimensional module")
    print("(fp) - finitely presented module")
    module_type = input("Type (default 'fd'): ").strip() or "fd"

    if module_type not in ["fd", "fp"]:
        print(f"Invalid type '{module_type}'. Type must be 'fd' or 'fp'")
        return

    # Get prime
    prime = int(input("p (default 2): ").strip() or "2")

    print(f"module_type: {module_type}")

    if module_type == "fd":
        output = define_finite_dimensional_module(prime)
    else:
        output = define_finitely_presented_module(prime)

    print(json.dumps(output, indent=2))


if __name__ == "__main__":
    main()
