#!/usr/bin/env python3
"""
Generate chart for unstable spectral sequences.
Python translation of unstable_chart.rs example.
"""

import sys
import ext


def main():
    # Query for unstable module
    module_name = input("Module name (e.g., 'S_2', 'RPinf'): ").strip()

    if not module_name:
        print("No module specified")
        return

    # Create unstable resolution
    resolution = ext.query_unstable_module(module_name, save=False)

    # Set computation bounds
    max_degree = int(input("Max degree (default 20): ").strip() or "20")

    # Compute unstable resolution
    resolution.compute_through_degree(max_degree)

    # Convert to spectral sequence
    ss = resolution.to_unstable_sseq()

    # Generate chart
    print("Generating unstable Adams spectral sequence chart...")

    # Create SVG backend
    svg_backend = ext.SvgBackend(sys.stdout)

    # Write chart
    ss.write_unstable_chart(
        backend=svg_backend, max_degree=max_degree, show_differentials=True
    )

    print(
        f"Chart generated for {module_name} through degree {max_degree}",
        file=sys.stderr,
    )


if __name__ == "__main__":
    main()
