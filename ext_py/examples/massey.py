#!/usr/bin/env python3
"""
Compute Massey products in the Adams spectral sequence.
Python translation of massey.rs example.
"""

import ext

def main():
    ext.init_logging()
    
    # Query for module
    resolution = ext.query_module(None, True)
    
    # Set up computation bounds
    max_t = int(input("Max t (default 30): ").strip() or "30")
    max_s = int(input("Max s (default 15): ").strip() or "15")
    
    max_bidegree = ext.Bidegree.from_t_s(max_t, max_s)
    resolution.compute_through_bidegree(max_bidegree)
    
    # Get elements for Massey product
    print("\nEnter elements for Massey product computation:")
    elements = []
    
    while True:
        element_input = input(f"Element {len(elements) + 1} (or 'done' to finish): ").strip()
        if element_input.lower() == 'done':
            if len(elements) >= 3:
                break
            else:
                print("Need at least 3 elements for Massey product")
                continue
        
        try:
            # Parse element specification (e.g., "h_0", "h_1", "c_0")
            elements.append(element_input)
        except Exception as e:
            print(f"Error parsing element: {e}")
    
    print(f"\nComputing Massey product <{', '.join(elements)}>")
    
    try:
        # Create Massey product computer
        massey_computer = ext.MasseyProductComputer(resolution)
        
        # Compute the Massey product
        result = massey_computer.compute_massey_product(elements)
        
        if result.is_zero():
            print("Massey product is zero")
        elif result.is_indeterminate():
            print("Massey product is indeterminate")
            print(f"Indeterminacy: {result.indeterminacy()}")
        else:
            print(f"Massey product: {result}")
    
    except Exception as e:
        print(f"Error computing Massey product: {e}")

if __name__ == "__main__":
    main()