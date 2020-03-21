
import asyncio
import time
import threading

from .algebra import AdemAlgebra
from .module import FDModule
from . import RustResolution

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

class Resolution:
    def __init__(self, name, chart=None, module=None): 
        if module is None:
            self.M = make_unit_module()
        else:
            self.M = module
        self.A = self.M.algebra
        self.rust_res = RustResolution(self.M)
        self.rust_res.extend_through_degree(0,200,0,200)
        self.loop = asyncio.get_event_loop()
        self.filtration_one_products = self.A.default_filtration_one_products()[:-1] 
        self.class_handlers = []
        self.structline_handlers = []
        if chart is not None:
            self.set_chart(chart)


    def add_class_handler(self, handler):
        self.class_handlers.append(handler)

    def add_structline_handler(self, handler):
        self.structline_handlers.append(handler)

    def resolve(self, n):
        t = threading.Thread(target=self._resolve_thread(n), daemon=True)
        t.start()

    def _resolve_thread(self, n):
        def run(): 
            self.A.compute_basis(n)
            t0 = time.time()    
            for j in range(n):
                for i in range(n):
                    self.rust_res.step_resolution(i,j)
                    f = asyncio.run_coroutine_threadsafe(self.after_step(i, j), self.loop)
                    f.result()
            t1 = time.time()
            time_elapsed = t1 - t0
            print("Time taken to resolve %s stems:" % n,  time_elapsed)
        return run 


    def add_sseq_class_handler(self, chart):
        async def handler(self, x, y, idx): 
            await chart.add_class(x, y) 
        self.add_class_handler(handler)
     
    def add_sseq_structline_handler(self, chart):
        async def handler(self, 
            source_x, source_y, source_idx, 
            target_x, target_y, target_idx
        ):
            source_class = chart.get_class_by_idx(source_x, source_y, source_idx)
            target_class = chart.get_class_by_idx(target_x, target_y, target_idx)
            await chart.add_structline(source_class, target_class)
        self.add_structline_handler(handler)

    def set_chart(self, chart):
        self.chart = chart
        self.add_sseq_class_handler(chart)
        self.add_sseq_structline_handler(chart)

    async def add_class(self, s, t, idx):
        xy = st_to_xy(s, t)
        for handler in self.class_handlers:
            await handler(self, *xy, idx)
    
    async def add_structline(self, 
        source_s, source_t, source_idx,
        target_s, target_t, target_idx
    ):
        source_xy = st_to_xy(source_s, source_t)
        target_xy = st_to_xy(target_s, target_t)
        for handler in self.structline_handlers:
            await handler(self, *source_xy, source_idx, *target_xy, target_idx)   
   
    async def after_step(self, s, t):
        for idx in range(self.rust_res.module(s).number_of_gens_in_degree(t)):
            await self.add_class(s, t, idx)
        await self.compute_filtration_one_products(s, t)

    async def compute_filtration_one_products(self, target_s, target_t):
        if target_s == 0:
            return

        source_s = target_s - 1

        source = self.rust_res.module(source_s)
        target = self.rust_res.module(target_s)

        target_dim = target.number_of_gens_in_degree(target_t)

        for (op_name, op_degree, op_index) in self.filtration_one_products:
            source_t = target_t - op_degree
            if source_t - source_s < self.rust_res.min_degree:
                continue

            source_dim = source.number_of_gens_in_degree(source_t)

            d = self.rust_res.differential(target_s)

            products = [[0 for _ in range(target_dim)] for _ in range(source_dim)];

            for target_idx in range(target_dim):
                dx = d.output(target_t, target_idx)

                for source_idx in range(source_dim):
                    idx = source.operation_generator_to_index(op_degree, op_index, source_t, source_idx)
                    products[source_idx][target_idx] = dx.entry(idx)

            for target_idx in range(target_dim):
                for source_idx in range(source_dim):
                    if products[source_idx][target_idx] != 0:
                        await self.add_structline(
                            source_s, source_t, source_idx,
                            target_s, target_t, target_idx
                        ) 
