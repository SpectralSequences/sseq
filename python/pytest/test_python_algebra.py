import rust_algebra
fp = rust_algebra.fp
algebra = rust_algebra.algebra

dimension_list = [1,1,1,2,1,1,1]

B = rust_algebra.algebra.MilnorAlgebra(2, truncated=True, profile=[2,1])
B.compute_basis(10)

result = rust_algebra.fp.FpVector(2, 2)
multiplication_table = []
for r_deg in range(8):
    r_deg_table = []
    for out_deg in range(8):
        s_deg = out_deg - r_deg
        s_deg_table = []
        result.set_scratch_vector_size(B.dimension(out_deg))
        for r_idx in range(B.dimension(r_deg)):
            r_idx_table = []
            for s_idx in range(B.dimension(s_deg)):
                print("rdeg: %s, r_idx: %s, s_deg: %s, s_idx: %s" % (r_deg, r_idx, s_deg, s_idx))
                B.multiply_basis_elements(result, 1, r_deg, r_idx, s_deg, s_idx)
                r_idx_table.append(result.to_list())
                print("result: %s", result)
                result.set_to_zero_pure()
            s_deg_table.append(r_idx_table)
        r_deg_table.append(s_deg_table)
    multiplication_table.append(r_deg_table)



def myalgebra_dimension(degree, excess):
    if degree < 0 or degree >= len(dimension_list):
        return 0
    else:
        return dimension_list[degree]

def myalgebra_multiply(result, coeff, r_degree, r_idx, s_degree, s_idx, excess):
    if coeff == 0:
        return
    if r_degree == 0:
        result[s_idx] += 1
    elif s_degree == 0:
        pass

A = rust_algebra.algebra.PythonAlgebra(2, 
    get_dimension=myalgebra_dimension,
    compute_basis=None,
    multiply_basis_elements=None,
    basis_element_to_string=None
)

