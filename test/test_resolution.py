import rust_algebra
import timeit

def setup_resolution():
    global A
    global M
    global res
    A = rust_algebra.algebra.AdemAlgebra(2)
    A.compute_basis(60)
    M = rust_algebra.algebra.FDModule(A, "M", 0)
    M.add_generator(0, "x0")
    M.freeze()
    res = rust_algebra.Resolution(M)
    res.extend_through_degree(0,50,0,50)

import time
def resolve(res, n):
    t0 = time.time()
    for i in range(n):
        for j in range(n):
            res.step_resolution(i,j)
    t1 = time.time()
    time_elapsed = t1 - t0
    print("Time taken to resolve %s stems:" % n,  time_elapsed)


setup_resolution()
filtration_one_products = A.default_filtration_one_products()

def compute_filtration_one_products(self, target_s, target_t):
    if target_s == 0:
        return

    source_s = target_s - 1

    source = self.module(source_s)
    target = self.module(target_s)

    target_dim = target.number_of_gens_in_degree(target_t)

    for (op_name, op_degree, op_index) in filtration_one_products:
        source_t = target_t - op_degree
        if source_t - source_s < self.min_degree:
            continue

        source_dim = source.number_of_gens_in_degree(source_t)

        d = self.differential(target_s)

        products = [[0 for _ in range(target_dim)] for _ in range(source_dim)];

        for i in range(target_dim):
            dx = d.output(target_t, i)

            for j in range(source_dim):
                idx = source.operation_generator_to_index(op_degree, op_index, source_t, j)
                products[j][i] = dx.entry(idx)

        print(products)
