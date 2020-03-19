import rust_algebra
import asyncio
from decorators import monkey_patch
import time
from .spectral_sequence import SpectralSequenceChart

def st_to_xy(s, t):
    return (t-s, s)

def xy_to_st(x, y):
    return (y, x + y)

async def default_add_class(res, x, y):
    await res.ss.add_class(x, y) 

class Resolution:
    def __init__(self, name="resolution", step_handler=None, add_class=None):
        self.A = rust_algebra.algebra.AdemAlgebra(2)
        self.A.compute_basis(80)
        self.M = rust_algebra.algebra.FDModule(self.A, "M", 0)
        self.M.add_generator(0, "x0")
        self.M.freeze()
        self.res = rust_algebra.Resolution(self.M)
        self.res.extend_through_degree(0,100,0,100)
        self.ss = SpectralSequenceChart(name)
        self.loop = asyncio.get_event_loop()
        self.filtration_one_products = self.A.default_filtration_one_products()[:-1] 
        self.step_handler = step_handler or (lambda x,y: 0)
        if add_class:
            bound_method = add_class.__get__(self, self.__class__)
            self.add_class_handler = bound_method
        else:
            self.add_class_handler = default_add_class

    def resolve_thread(self, n):
        def resolve(): 
            t0 = time.time()    
            for j in range(n):
                for i in range(n):
                    self.res.step_resolution(i,j)
                    f = asyncio.run_coroutine_threadsafe(self.after_step(i, j), self.loop)
                    f.result()
                    
            t1 = time.time()
            time_elapsed = t1 - t0
            print("Time taken to resolve %s stems:" % n,  time_elapsed)
        return resolve    
   
    async def after_step(self, i, j):
        for _ in range(self.res.module(i).number_of_gens_in_degree(j)):
            await self.add_class_handler(*st_to_xy(i,j))
        await self.compute_filtration_one_products(i, j)
        # await add

    async def compute_filtration_one_products(self, target_s, target_t):
        if target_s == 0:
            return

        source_s = target_s - 1

        source = self.res.module(source_s)
        target = self.res.module(target_s)

        target_dim = target.number_of_gens_in_degree(target_t)

        for (op_name, op_degree, op_index) in self.filtration_one_products:
            source_t = target_t - op_degree
            if source_t - source_s < self.res.min_degree:
                continue

            source_dim = source.number_of_gens_in_degree(source_t)

            d = self.res.differential(target_s)

            products = [[0 for _ in range(target_dim)] for _ in range(source_dim)];

            for target_idx in range(target_dim):
                dx = d.output(target_t, target_idx)

                for source_idx in range(source_dim):
                    idx = source.operation_generator_to_index(op_degree, op_index, source_t, source_idx)
                    products[source_idx][target_idx] = dx.entry(idx)

            for target_idx in range(target_dim):
                for source_idx in range(source_dim):
                    if products[source_idx][target_idx] != 0:
                        source_class = self.ss.get_class_by_idx(*st_to_xy(source_s, source_t), source_idx)
                        target_class = self.ss.get_class_by_idx(*st_to_xy(target_s, target_t), target_idx)
                        await self.ss.add_structline(source_class, target_class) 
