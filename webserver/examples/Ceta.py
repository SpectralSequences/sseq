A = AdemAlgebra(2)
A.compute_basis(20)
M = FDModule(A, "M", 0)
M.add_generator(0, "x0")
M.add_generator(2, "x2")
M.parse_action("Sq2 x0 = x2", None)
M.freeze()
r = ext.resolution.Resolver("Ceta", module=M)
Ceta = ResolverChannel(r, REPL)

Ceta.chart.sseq.initial_x_range = [0, 40]
Ceta.chart.sseq.initial_y_range = [0, 20]
Ceta.chart.x_range = [0, 80]
Ceta.chart.y_range = [0, 40]

Ceta.resolver.resolve(50)