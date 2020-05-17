import pathlib
import json
import itertools

from message_passing_tree.prelude import *
from message_passing_tree import SocketChannel
from message_passing_tree.utils import json_stringify
from message_passing_tree import ansi

from ext import fp

from spectralsequence_chart import SseqSocketReceiver, ChartAgent

from ..repl.executor import Executor
from .. import config

from ..process_overlay import process_overlay


from fastapi.templating import Jinja2Templates
templates = Jinja2Templates(directory=str(config.TEMPLATE_DIR))


@subscribe_to("*")
@collect_transforms(inherit=True)
class TableChannel(SocketChannel):
    def __init__(self, name, repl_agent):
        super().__init__(name)
        self.repl_agent = repl_agent
        self.executor = Executor(repl_agent)
        self.chart = ChartAgent(name)
        sseq = self.chart.sseq
        sseq.x_range = [0, 120]
        sseq.y_range = [0, 60]
        sseq.initial_x_range = [0, 60]
        sseq.initial_y_range = [0, 30]
        self.table = ProductTable()
        self.setup_executor_namespace()
        self.populate_chart()

    channels = {}
    async def send_start_msg_a(self):
        pass

    async def setup_a(self):
        await self.repl_agent.add_child_a(self.executor)
        await self.executor.add_child_a(self.chart)
        await self.chart.add_child_a(self)
        self.chart._interact_source = None
        await self.executor.load_repl_init_file_if_it_exists_a()
        self.table.build_dense_products()

        
    @transform_inbound_messages
    async def transform__console__take__a(self, envelope):
        envelope.mark_used()
        self.repl_agent.set_executor(self.executor)

    @transform_inbound_messages
    async def transform__click__a(self, envelope, x, y, chart_class):
        envelope.mark_used()

    @transform_inbound_messages
    async def transform__interact__select_bidegree__a(self, envelope, bidegree):
        envelope.mark_used()
        names = self.get_names_info(bidegree)
        matrix = self.get_matrix(bidegree)
        prod_info = self.get_product_info(bidegree)
        await self.send_message_outward_a("interact.product_info", *arguments(names=names, matrix=matrix, product_info=prod_info))

    @transform_inbound_messages
    async def transform__interact__click_product__a(self, envelope, bidegree, idx):
        envelope.mark_used()
        (in1, in2, out) = self.get_filtered_decompositions(bidegree)[idx]
        print(in1, in2, out)
        # await self.send_message_outward_a("interact.product_info", *arguments(product_info=prod_info))


    async def add_subscriber_a(self, websocket):
        recv = SseqSocketReceiver(websocket)
        await self.add_child_a(recv)
        await recv.start_a()

    def setup_executor_namespace(self):
        globals = self.executor.get_globals()
        globals["REPL"] = self.repl_agent
        globals["chart"] = self.chart
        globals["channel"] = self
        globals["table"] = self.table

    @classmethod
    async def get_channel_a(cls, name, repl):
        if name in cls.channels:
            print("Found")
            return cls.channels[name]
        print("Generating")
        new_channel = TableChannel(name, repl)
        await new_channel.setup_a()
        return new_channel


    @classmethod
    def has_channel(cls, name):
        return True #name in cls.channels or cls.get_file_path(name)

    @classmethod
    def http_response(cls, channel_name, request):
        response_data = { 
            "port" : cls.port, 
            "directory" : cls.directory,
            "channel_name" : channel_name,
            "request" : request, 
        }
        if cls.has_channel(channel_name):
            return templates.TemplateResponse("table.html", response_data)

    def populate_chart(self):
        chart = self.chart.sseq
        for (y, row) in enumerate(self.table.num_gens):
            for (x, gens) in enumerate(row):
                for _ in range(gens):
                    chart.add_class(x,y)

        for [(in1, in2), out_list] in self.table.product_table.items():
            if in2[0] < in1[0]:
                (in1,in2) = (in2, in1)
            if in1 in [(0,1,0), (1,1,0),(3,1,0)]:
                for out in out_list:
                    try:
                        chart.add_structline(chart.class_by_idx(*in2), chart.class_by_idx(*out))
                    except IndexError:
                        continue
        chart.add_structline(chart.class_by_idx(0, 0, 0), chart.class_by_idx(0, 1, 0))
        chart.add_structline(chart.class_by_idx(0, 1, 0), chart.class_by_idx(3, 2, 0))
        chart.add_structline(chart.class_by_idx(0, 2, 0), chart.class_by_idx(3, 3, 0))
        chart.add_structline(chart.class_by_idx(0, 0, 0), chart.class_by_idx(1, 1, 0))
        chart.add_structline(chart.class_by_idx(0, 0, 0), chart.class_by_idx(3, 1, 0))

        for [c,name] in self.table.class_names:
            try:
                chart.class_by_idx(*c).monomial_name = name
                chart.class_by_idx(*c).name = self.table.name_to_str(name)
            except IndexError:
                pass
        
        for c in chart.classes:
            if not c.name:
                c.set_color("blue")

        for c in chart.classes:
            if self.table.indecomposable_q(c.x, c.y, c.idx):
                if c.name:
                    c.set_color("red")
                else:
                    c.set_color("purple")

    def get_name(self, tuple):
        c = self.chart.sseq.class_by_idx(*tuple)
        if c.name:
            return c.name
        else:
            return f"x_{{{tuple[0], tuple[1]}}}^{{{tuple[2]}}}"

    def get_monomial_name(self, tuple):
        c = self.chart.sseq.class_by_idx(*tuple)
        if hasattr(c, "monomial_name"):
            return c.monomial_name
        else:
            return [f"x_{{{tuple[0], tuple[1]}}}^{{{tuple[2]}}}", 1]

    def get_names_info(self, bidegree):
        num_classes = len(self.chart.sseq.classes_in_bidegree(*bidegree))
        return [self.get_name((*bidegree, i)) for i in range(num_classes)]            
    
    def get_matrix(self, bidegree):
        (x, y) = bidegree
        return self.table.bases[y][x].matrix.to_python_matrix()
        

    def get_product_info(self, bidegree):
        result = []
        for (in1, in2, out) in self.get_filtered_decompositions(bidegree):
            n1 = (self.get_name(in1), self.get_monomial_name(in1))
            n2 = (self.get_name(in2), self.get_monomial_name(in2))
            out_indexes = [x[-1] for x in out]
            out_vec = [0] * len(self.chart.sseq.classes_in_bidegree(*bidegree))
            for idx in out_indexes:
                out_vec[idx] = 1
            result.append((n1, n2, out_vec))
        return result

    def get_filtered_decompositions(self, bidegree):
        bidegree = tuple(bidegree)
        result = []
        decompositions = self.get_decompositions(bidegree)
        for (in1, in2, out) in decompositions:
            indec1 = self.table.indecomposable_q(*in1)
            indec2 = self.table.indecomposable_q(*in2)
            if not indec1:
                continue
            if indec1 and indec2 and in1 > in2:
                continue
            if in1 == (0,0,0):
                continue
            # We don't want a bunch of redunant P-juggling 
            # (really Px shouldn't be considered indecomposable)
            if self.get_name(in1).strip().startswith("P"):
                continue
            result.append((in1, in2, out))
        return result

from ..name_tools import monomial_name


class ProductTable:
    def __init__(self):
        self.load_numgens()
        self.load_product_table()
        
        self.initialize_change_of_basis_matrices()
        
        self.generate_decomposition_table()
        self.setup_class_names()
        self.compute_all_indecomposables()
    
    def load_numgens(self):
        num_gens = json.loads(pathlib.Path(config.USER_DIR / "S_2-dims.json").read_text())
        self.num_gens = [[x or 0 for x in row] for row in num_gens]

    def load_product_table(self):
        product_table_json = json.loads(pathlib.Path(config.USER_DIR / "product_table.json").read_text())
        self.product_table = dict([tuple(tuple(l) for l in key), value] for [key, value] in product_table_json)

    def initialize_change_of_basis_matrices(self):
        self.bases = [[fp.Basis(2, n) for n in r] for r in self.num_gens]
    
    def gens_in_bidegree(self, x, y):
        return self.num_gens[y][x]
    
    def basis_in_bidegree(self, x, y):
        self.bases[y][x]

    def generate_decomposition_table(self):
        self.decomposition_table = {}
        self.nontrivial_pairs = {}
        for ((in1, in2), out) in self.product_table.items():
            key = (in1[0] + in2[0], in1[1] + in2[1])
            if key not in self.decomposition_table:
                self.decomposition_table[key] = []
                self.decomposition_table[key] = set()
            self.decomposition_table[key].append((in1, in2, out))
            self.nontrivial_pairs[key].add((in1[:-1], in2[:-1]))

    def compute_all_indecomposables(self):
        self.indecomposables = [[[] for n in r] for r in self.num_gens]
        for (y, row) in enumerate(self.num_gens):
            for (x, e) in enumerate(row):
                if (x,y) not in self.decomposition_table:
                    continue
                self.indecomposables[y][x] = self.compute_indecomposables_in_bidegree(x, y)

    def compute_indecomposables_in_bidegree(self, x, y)
        ng = self.gens_in_bidegree(x, y)
        subspace = fp.Subspace(2, ng+1, ng)
        subspace.set_to_zero()
        image_vecs = [ [ x[-1] for x in out] for (in1, in2, out) in self.decomposition_table[(x,y)] if in1 != (0,0,0)]
        py_v = [0] * ng
        v = fp.FpVector(2, ng)
        w = fp.FpVector(2, ng)
        B = self.basis_in_bidegree(x, y)
        for e in image_vecs:
            for i in range(ng):
                py_v[i] = 1 if i in e else 0
            v.pack(w)
            w.set_to_zero()
            B.apply_inverse(w, 1, v)
            subspace.add_vector(w)
        return [idx for (idx, e) in enumerate(subspace.matrix().pivots()) if e == -1]
    
    def build_dense_products(self):
        self.dense_products = {}
        for [tout, pairs] in self.nontrivial_pairs.items():
            self.dense_products[tout] = {}
            for (t1, t2) in pairs:
                ng1 = self.gens_in_bidegree(*t1)
                ng2 = self.gens_in_bidegree(*t2)
                ngout = self.gens_in_bidegree(*tout)
                products = [[fp.FpVector(2, ngout) for _ in range(ng2)] for _ in range(ng1)]
                self.dense_products[tout][(t1, t2)] = products
                for idx1 in range(ng1):
                    in1 = t1 + (idx1,)
                    for idx2 in range(ng2):
                        in2 = t2 + (idx2,)
                        for [_, _, e] in self.product_table.get((in1,in2), []):
                            products[idx1][idx2][e] = 1
                
    
    def get_decompositions(self, x, y):
        v1 = fp.FpVector(2, 0)
        w1 = fp.FpVector(2, 0)
        v2 = fp.FpVector(2, 0)
        w2 = fp.FpVector(2, 0)
        vout = fp.FpVector(2, ngout)
        wout = fp.FpVector(2, ngout)
        tout = (x, y)
        ngout = self.gens_in_bidegree(*tout)
        bout = self.basis_in_bidegree(*tout)
        for (t1, t2) in self.dense_products[tout]:
            ng1 = self.gens_in_bidegree(*t1)
            ng2 = self.gens_in_bidegree(*t2)            
            b1 = self.basis_in_bidegree(*t1)
            b2 = self.basis_in_bidegree(*t2)
            v1.set_scratch_vector_size(ng1)
            w1.set_scratch_vector_size(ng1)
            v2.set_scratch_vector_size(ng2)
            w2.set_scratch_vector_size(ng2)
            for idx1 in range(ng1):
                v1.set_to_zero()
                w1.set_to_zero()
                v1[idx1] = 1
                b1.apply(w1, 1, v1)
                for idx2 in range(ng2):
                    v2.set_to_zero()
                    w2.set_to_zero()
                    v2[idx1] = 1
                    b2.apply(w2, 1, v2)
                    
            

    def indecomposable_q(self, x, y, idx):
        return idx in self.indecomposables[y][x]

    def setup_class_names(self):
        self.class_names = json.loads(pathlib.Path(config.USER_DIR / "class_names_parsed.json").read_text())
        self.class_name_table = dict([tuple(rest,),name] for [rest, name] in self.class_names)
        self.gen_degs = {}
        for [t, n] in self.class_names:
            if len(n) == 1 and n[0][1] == 1:
                self.gen_degs[n[0][0]] = t
        self.gen_degs["P"] = [0,0,0]
    
    def name_to_str(self, name):
        return monomial_name(*sorted(name, key=lambda x : self.gen_degs[x[0]] if x[0] in self.gen_degs else [10000, 10000]))







# Generate from original file

# def load_numgens(self):
#     num_gens = json.loads(pathlib.Path(config.USER_DIR / "S_2-dims.json").read_text())
#     self.num_gens = [[x or 0 for x in row] for row in num_gens]
#     self.s_totals = [[x - 1 for x in itertools.accumulate(row)] for row in num_gens]
#     self.s_indexes = [[] for _ in range(120)]
#     for (i, row) in enumerate(self.s_totals):
#         for (j, entry) in enumerate(row):
#             if entry > row[j-1]:
#                 self.s_indexes[i].append([entry, j])


# def s_idx_to_x_idx(self s, idx):
#     if idx == 0:
#         return (0, s, 0)
#     prev_gens = 0
#     for [i, [gens, x]] in enumerate(self.s_indexes[s]):
#         if idx <= gens:
#             return (x, s, idx - prev_gens - 1)
#         prev_gens = gens


# def generate_product_table(self):
#     all_JR = pathlib.Path(config.USER_DIR / "all.JR.txt").read_text()
#     ajr_lines = [l for l in all_JR.splitlines() if l]
#     product_table = {}
#     for s in ajr_lines:
#         l = s.replace("(", "").split()
#         try:
#             output = [int(x) for x in l[:2]]
#             in1 = [int(x) for x in l[2:4]]
#             in2 = [int(x) for x in l[-1].split("_")]
#         except ValueError:
#             print(l)
#             break
#         [output, in1, in2] = [s_idx_to_x_idx(*x) for x in (output, in1, in2)]
#         if None in [output, in1, in2]:
#             continue
#         if (in1, in2) not in product_table:
#             product_table[(in1,in2)] = []
#         product_table[(in1,in2)].append(output)
#     return product_table