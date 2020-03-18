import sys
import rust_algebra
fp = rust_algebra.fp
algebra = rust_algebra.algebra

dimension_list = [1,1,1,2,1,1,1]

B = rust_algebra.algebra.MilnorAlgebra(2, truncated=True, profile=[2,1])
B.compute_basis(10)

result = rust_algebra.fp.FpVector(2, 2)
def make_multiplication_table():
    multiplication_table = []
    for r_deg in range(8):
        r_deg_table = []
        for out_deg in range(r_deg, 8):
            s_deg = out_deg - r_deg
            s_deg_table = []
            result.set_scratch_vector_size(B.dimension(out_deg))
            for r_idx in range(B.dimension(r_deg)):
                r_idx_table = []
                for s_idx in range(B.dimension(s_deg)):
                    B.multiply_basis_elements(result, 1, r_deg, r_idx, s_deg, s_idx)
                    r_idx_table.append(result.to_list())
                    result.set_to_zero_pure()
                s_deg_table.append(r_idx_table)
            r_deg_table.append(s_deg_table)
        multiplication_table.append(r_deg_table)
    return multiplication_table

multiplication_table = make_multiplication_table()





def myalgebra_dimension(degree, excess):
    if degree < 0 or degree >= len(dimension_list):
        return 0
    else:
        return dimension_list[degree]


def myalgebra_multiply(result, coeff, r_degree, r_idx, s_degree, s_idx, excess):
    if coeff == 0:
        return
    result.add(fp.FpVector.from_list(2, multiplication_table[r_degree][s_degree][r_idx][s_idx]), coeff)

basis_element_names = [["1"], ["Sq(1)"], ["Sq(2)"], ["Sq(3)", "Sq(0,1)"], ["Sq(1,1)"], ["Sq(2,1)"], ["Sq(3,1)"]]
def myalgebra_basis_element_to_string(degree, idx):
    return basis_element_names[degree][idx]

A = rust_algebra.algebra.PythonAlgebra(2, 
    get_dimension=myalgebra_dimension,
    compute_basis=None,
    multiply_basis_elements=myalgebra_multiply,
    basis_element_to_string=myalgebra_basis_element_to_string
)

x = A.new_element(3)
y = A.new_element(3)
z = A.new_element(6)
x.vec[0] = 1
y.vec[1] = 1
z.multiply_add(x, y)
print("%s * %s = %s" % (x, y, z))