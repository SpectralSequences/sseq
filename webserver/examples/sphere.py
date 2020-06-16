A = AdemAlgebra(2)
A.compute_basis(2)
M = FDModule(A, "M", 0)
M.add_generator(0, "x0")
M.freeze()
r = ext.resolution.Resolver("S_2", module=M)
S = ResolverChannel(r, REPL)
S.resolver.resolve(50)


S.chart.initial_x_range = [0, 40]
S.chart.initial_y_range = [0, 20]
S.chart.x_range = [0, 80]
S.chart.y_range = [0, 40]
