#!/usr/bin/env python3
"""
Compute tensor product of two modules.
Python translation of tensor.rs example.
"""

import ext
import json


def main():
    # Get left module
    left_name = input("Left module (default 'S_2'): ").strip() or "S_2"
    left_module_json = ext.parse_module_name(left_name)

    p = left_module_json["p"]

    # Get right module (must have same prime)
    while True:
        right_name = input("Right module (default 'S_2'): ").strip() or "S_2"
        right_module_json = ext.parse_module_name(right_name)

        if right_module_json["p"] == p:
            break
        else:
            print("Error: Two modules must be over the same prime")

    # Create algebra
    alg = ext.SteenrodAlgebra.adem_algebra(prime=p, truncated=False)

    # Load modules from JSON
    left_module = ext.steenrod_module_from_json(alg, left_module_json)
    right_module = ext.steenrod_module_from_json(alg, right_module_json)

    # Create tensor product
    tensor_module = ext.TensorModule(left_module, right_module)

    # Convert to finite dimensional module for output
    fd_tensor = ext.FDModule.from_tensor_module(tensor_module)
    fd_tensor.name = ""

    # Output as JSON
    output = {"p": p}
    output.update(fd_tensor.to_json())

    print(json.dumps(output, indent=2))


if __name__ == "__main__":
    main()
