import asyncio
import time
import threading

from message_passing_tree.prelude import *
from message_passing_tree import MathAgent

from .fp import Matrix
from .algebra import AdemAlgebra
from .module import FDModule
from . import RustResolution
import rust_ext 
RustResolutionHomomorphism = rust_ext.ResolutionHomomorphism

def st_to_xy(s, t):
    return (t-s, s)

def xy_to_st(x, y):
    return (y, x + y)

def make_unit_module():
    A = AdemAlgebra(2)
    M = FDModule(A, "M", 0)
    M.add_generator(0, "x0")
    M.freeze()
    return M

@subscribe_to("*")
@collect_handlers(inherit=False)
class Resolver(MathAgent):
    def __init__(self, name, chart=None, module=None): 
        super().__init__()
        if module is None:
            self.M = make_unit_module()
        else:
            self.M = module
        self.name=name
        self.A = self.M.algebra
        self.rust_res = RustResolution(self.M)
        self.rust_res.freeze()
        self.loop = asyncio.get_event_loop() # Need this so that worker thread can submit events to run on the same event loop as main thread
        self.filtration_one_products = self.A.default_filtration_one_products()[:-1] 
        self.class_handlers = []
        self.structline_handlers = []

        self.max_degree = -1
        self.target_max_degree = -1
        self.finished_degrees = set()
        self.unfinished_degrees = {}
        
        self.unit_resolution = None
        self.chain_maps_to_unit_resolution = [[None] * 200 for _ in range(200)]
        
        self.chart = chart

    def resolve(self, n):
        t = threading.Thread(target=self._resolve_st_rectangle(n), daemon=True)
        t.start()
        # self._resolve_thread(n)()

    def _resolve_st_rectangle(self, n):
        def run(): 
            asyncio.set_event_loop(self.loop)
            self.A.compute_basis(n)
            self.target_max_degree = n
            self.rust_res.extend_through_degree(n, n)
            t0 = time.time()
            for t in range(n):
                for s in range(n):
                    self.step_if_needed(s,t)

            t1 = time.time()
            time_elapsed = t1 - t0
            print(f"Time taken to resolve {self.name} from stem {self.max_degree + 1} to stem {self.target_max_degree}:",  time_elapsed)
            self.max_degree = self.target_max_degree
        return run 


    def _resolve_xy_rectangle(self, n):
        def run(): 
            self.A.compute_basis( x + y + 1)
            self.target_max_degree = n
            self.rust_res.extend_through_degree( x + y + 2)
            t0 = time.time()
            for x in range(n):
                for y in range(n):
                    self.step_if_needed(*xy_to_st(x,y))
            t1 = time.time()
            time_elapsed = t1 - t0
            print(f"Time taken to resolve {self.name} from stem {self.max_degree + 1} to stem {self.target_max_degree}:",  time_elapsed)
            self.max_degree = self.target_max_degree
        return run 

    def step_if_needed(self, i, j):
        if (i, j) not in self.finished_degrees:
            self.rust_res.step_resolution(i,j)
            asyncio.ensure_future(self.step_after(i, j))
            # if self.rust_res.number_of_gens_in_bidegree(i, j) > 0:
                # print(i, j, self.rust_res.number_of_gens_in_bidegree(i, j))
            # f = asyncio.run_coroutine_threadsafe(
            #     self.broadcast(["resolution", "finished_bidegree"], i, j), 
            #     self.loop
            # )
            # f.result()
            self.finished_degrees.add((i, j))

    async def step_after(self, s, t):
        if not self.chart:
            return
        self.add_classes(s, t)
        products = self.compute_filtration_one_products(s, t)
        for product in products:
            source_t = product["source_t"]
            source_s = product["source_s"]
            target_t = t
            target_s = s 
            table = product["table"]
            for (source_idx, row) in enumerate(iter(table)):
                for (target_idx, entry) in enumerate(iter(row)):
                    if entry != 0:
                        await self.add_structline(
                            source_s, source_t, source_idx,
                            target_s, target_t, target_idx
                        )
        await self.chart.update_a()
        


    def add_classes(self, s, t):
        for i in range(self.rust_res.number_of_gens_in_bidegree(s, t)):
            self.chart.sseq.add_class(*st_to_xy(s, t))

    async def add_structline(self,
        source_s, source_t, source_idx,
        target_s, target_t, target_idx
    ):
        try:
            source = self.chart.sseq.class_by_idx(*st_to_xy(source_s, source_t), source_idx)
            target = self.chart.sseq.class_by_idx(*st_to_xy(target_s, target_t), target_idx)
            self.chart.sseq.add_structline(source, target)
        except Exception as e:
            await self.send_error_a("", exception=e)


    def cocycle_string(self, x, y, idx):
        return self.rust_res.cocycle_string(*xy_to_st(x, y), idx)

    def compute_filtration_one_products(self, target_s, target_t):
        if target_s == 0:
            return []

        source_s = target_s - 1
        source = self.rust_res.module(source_s)
        target = self.rust_res.module(target_s)

        target_dim = target.number_of_gens_in_degree(target_t)
        result = []
        for (op_name, op_degree, op_index) in self.filtration_one_products:
            source_t = target_t - op_degree
            if source_t - source_s < self.rust_res.min_degree:
                continue

            source_dim = source.number_of_gens_in_degree(source_t)

            d = self.rust_res.differential(target_s)

            products = [[0 for _ in range(target_dim)] for _ in range(source_dim)]
            for target_idx in range(target_dim):
                dx = d.output(target_t, target_idx)
                for source_idx in range(source_dim):
                    idx = source.operation_generator_to_index(op_degree, op_index, source_t, source_idx)
                    products[source_idx][target_idx] = dx.entry(idx)
            result.append({"source_s" : source_s, "source_t" : source_t, "table" : products})
        return result

    def construct_maps_to_unit_resolution_in_bidegree(self, s, t):
        if self.unit_resolution is None:
            raise ValueError("Need to define self.unit_resolution first.")
        if self.chain_maps_to_unit_resolution[s][t] is not None:
            return

        p = self.rust_res.prime()

        # Populate the arrays if the ResolutionHomomorphisms have not been defined.
        num_gens = self.rust_res.module(s).number_of_gens_in_degree(t)
        self.chain_maps_to_unit_resolution[s][t] = []
        if num_gens == 0:
            return
        unit_vector = Matrix(p, num_gens, 1)
        for idx in range(num_gens):
            f = RustResolutionHomomorphism(
                f"(hom_deg : {s}, int_deg : {t}, idx : {idx})",
                self.rust_res, self.unit_resolution,
                s, t
            )
            unit_vector[idx].set_entry(0, 1)
            f.extend_step(s, t, unit_vector)
            unit_vector[idx].set_to_zero_pure()
            self.chain_maps_to_unit_resolution[s][t].append(f)

    def construct_maps_to_unit_resolution(self):
        for s in range(self.max_degree):
            for t in range(self.max_degree):
                self.construct_maps_to_unit_resolution_in_bidegree(s, t)
