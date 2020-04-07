chart = SpectralSequenceChart("S3-unst")
chart.set_initial_x_range(0, 40)
chart.set_initial_y_range(0, 20)
chart.set_x_range(0, 80)
chart.set_y_range(0, 40)

A = AdemAlgebra(2, unstable=True)
A.compute_basis(80)
M = FDModule(A, "M", 0)
M.add_generator(3, "x0")
M.freeze()
res_sphere = Resolution("S^3", chart=chart, module=M)
# import rust_ext
