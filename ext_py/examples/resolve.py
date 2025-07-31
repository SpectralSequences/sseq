#!/usr/bin/env python3
"""
Resolves a module up to a fixed (s, t) and prints an ASCII depiction of the Ext groups.
Python translation of resolve.rs example.
"""

import ext

def main():
    ext.init_logging()
    
    # Query for module interactively
    resolution = ext.query_module_only("Module", None, False)
    
    # Set computation bounds
    max_t = int(input("Max t (default 30): ") or "30")
    max_s = int(input("Max s (default 15): ") or "15")
    
    max_bidegree = ext.Bidegree.from_t_s(max_t, max_s)
    
    # Compute resolution through the specified bidegree
    resolution.compute_through_bidegree(max_bidegree)
    
    # Print ASCII chart
    print(resolution.graded_dimension_string())

if __name__ == "__main__":
    main()