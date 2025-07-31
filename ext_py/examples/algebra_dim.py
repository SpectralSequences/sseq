#!/usr/bin/env python3
"""
Compute dimensions of the Milnor algebra A_n for n = 0 to 125.
Python translation of algebra_dim.rs example.
"""

import ext


def main():
    ext.init_logging()

    # Create Milnor algebra over F_2
    alg = ext.MilnorAlgebra(prime=2, truncated=False)

    # Compute basis up to degree 125
    alg.compute_basis(125)

    # Print dimensions
    for n in range(126):
        print(f"dim A_{n} = {alg.dimension(n)}")


if __name__ == "__main__":
    main()
