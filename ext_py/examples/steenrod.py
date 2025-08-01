#!/usr/bin/env python3
"""
Compute Steenrod operations on module elements.
Python translation of steenrod.rs example.
"""

import ext


def main():
    # Query for module
    resolution = ext.query_module(None, False)

    # Get the module and algebra
    module = resolution.module(0)  # Get degree 0 part
    alg = module.algebra()

    print("Available generators:")
    for deg in range(module.min_degree(), module.max_computed_degree() + 1):
        if module.dimension(deg) > 0:
            for i in range(module.dimension(deg)):
                gen_name = module.basis_element_name(deg, i)
                print(f"  {gen_name} (degree {deg})")

    # Interactive computation
    while True:
        # Get Steenrod operation
        operation = input(
            "\nEnter Steenrod operation (e.g., 'Sq2', 'P1') or 'quit': "
        ).strip()
        if operation.lower() == "quit":
            break

        try:
            # Parse operation
            if operation.startswith("Sq"):
                # Adem algebra element
                op_deg = int(operation[2:])
                if alg.prime() != 2:
                    print("Sq operations only available at prime 2")
                    continue
            elif operation.startswith("P"):
                # Milnor P operation
                op_deg = int(operation[1:])
            else:
                print("Unknown operation format")
                continue

            # Get target element
            element = input("Enter element name: ").strip()

            # Find element
            element_deg = None
            element_idx = None

            for deg in range(module.min_degree(), module.max_computed_degree() + 1):
                for i in range(module.dimension(deg)):
                    if module.basis_element_name(deg, i) == element:
                        element_deg = deg
                        element_idx = i
                        break
                if element_deg is not None:
                    break

            if element_deg is None:
                print(f"Element '{element}' not found")
                continue

            # Compute operation
            result = module.apply_operation(operation, element_deg, element_idx)

            if result.is_zero():
                print(f"{operation} {element} = 0")
            else:
                result_str = module.element_to_string(element_deg + op_deg, result)
                print(f"{operation} {element} = {result_str}")

        except Exception as e:
            print(f"Error: {e}")


if __name__ == "__main__":
    main()
