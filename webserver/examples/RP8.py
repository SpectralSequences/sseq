A = AdemAlgebra(2)
A.compute_basis(20)
M = FDModule(A, "RP8")
for i in range(1, 9):
    M.add_generator(i, f"x{i}")
for i in range(1, 9, 2):
    M.parse_action(f"Sq1 x{i} = x{i+1}")
M.parse_action(f"Sq2 x2 = x4")
M.parse_action(f"Sq2 x3 = x5")
M.parse_action(f"Sq2 x6 = x8")
M.parse_action(f"Sq4 x4 = x8")
M.extend_actions()
M.freeze()
r = ext.resolution.Resolver("RP8", module=M)
RP8 = ResolverChannel(r, REPL)
await RP8.setup_a()
RP8.chart.sseq.initial_x_range = [0, 40]
RP8.chart.sseq.initial_y_range = [0, 20]
RP8.chart.x_range = [0, 80]
RP8.chart.y_range = [0, 40]

RP8.resolver.resolve(50)