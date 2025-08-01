#!/usr/bin/env python3
"""
Generate SVG chart of a spectral sequence.
Python translation of chart.rs example.
"""

import sys
import ext


def main():
    # Query for module
    resolution = ext.query_module(None, False)

    # Convert resolution to spectral sequence
    ss = resolution.to_sseq()

    # Get filtration one products
    products = []
    for name, op_deg, op_idx in resolution.algebra().default_filtration_one_products():
        product_data = resolution.filtration_one_products(op_deg, op_idx)
        products.append((name, product_data))

    # Write SVG to stdout
    svg_backend = ext.SvgBackend(sys.stdout)
    ss.write_to_graph(
        backend=svg_backend,
        page_number=2,
        show_differentials=False,
        products=products,
        callback=lambda x: None,
    )


if __name__ == "__main__":
    main()
