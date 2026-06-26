#!/usr/bin/env python3
"""Tensor two modules together and print the result as module JSON.

Python port of ext/examples/tensor.rs.
"""

import json

import _query as query
import ext
from ext import algebra


def main():
    left = query.with_default("Left module", "S_2", ext.parse_module_name)
    p = left["p"]

    def parse_right(name):
        module = ext.parse_module_name(name)
        if module["p"] != p:
            raise ValueError("Two modules must be over the same prime")
        return module

    right = query.with_default("Right module", "S_2", parse_right)

    alg = algebra.SteenrodAlgebra.adem(p)

    left_module = algebra.SteenrodModule.from_spec(left, alg)
    right_module = algebra.SteenrodModule.from_spec(right, alg)

    tensor_module = algebra.TensorModule(left_module, right_module)

    # Convert to a finite dimensional module for output.
    # NOTE: `from_tensor_module` is NOT yet bound (aspirational API); the class
    # was renamed FDModule -> FDModuleBuilder, but this conversion constructor is
    # still pending in the bindings. This line will not run until it is bound.
    tensor = algebra.FDModuleBuilder.from_tensor_module(tensor_module)
    tensor.name = ""

    output = {"p": p}
    output.update(tensor.to_json())

    # serde_json's Display is compact (no spaces) and preserves insertion order.
    print(json.dumps(output, separators=(",", ":"), ensure_ascii=False))


if __name__ == "__main__":
    main()
