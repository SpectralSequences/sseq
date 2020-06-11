import asyncio
from datetime import datetime
import itertools
import json
import pathlib

from concurrent.futures import ThreadPoolExecutor


from message_passing_tree.prelude import *
from message_passing_tree import SocketChannel
from message_passing_tree.utils import json_stringify
from message_passing_tree import ansi

from ext import fp

from spectralsequence_chart import SseqSocketReceiver, ChartAgent, ChartData


from ...repl.executor import Executor
from ... import config

from ... import name_tools

from fastapi.staticfiles import StaticFiles
from fastapi.templating import Jinja2Templates
CHANNEL_DIR = pathlib.Path(__file__).parent
templates = Jinja2Templates(directory=str(CHANNEL_DIR))

HI_MAX = 8


@subscribe_to("*")
@collect_transforms(inherit=True)
class TableChannel(SocketChannel):
    serve_as = "table"
    SAVE_DIR = config.SAVE_DIR / "table"

    @classmethod
    def serve_extra(cls, app, host, port, cls_dir):
        app.mount("/client/table", StaticFiles(directory=CHANNEL_DIR / "dist"), name="client")
        app.mount("/debug/table/chart", StaticFiles(directory=config.CHART_REPOSITORY_ROOT), name="debug")


    def __init__(self, name, repl_agent):
        super().__init__(name)
        self.repl_agent = repl_agent
        self.executor = Executor(repl_agent)
        self.py_executor = ThreadPoolExecutor(max_workers = 1)
        self.chart = ChartAgent(name)
        self.setup_executor_namespace()
        self.ready = asyncio.Event()

    channels = {}
    async def send_start_msg_a(self):
        pass

    @classmethod
    def has_channel(cls, name):
        return True #name in cls.channels or cls.get_file_path(name)

    @classmethod
    async def get_channel_a(cls, name, repl):
        if name in cls.channels:
            result = cls.channels[name]
        else:
            result = await cls.create_channel_a(name, repl)
        return result

    @classmethod
    async def create_channel_a(cls, name, repl):
        channel = cls(name, repl)
        await channel.setup_a()
        cls.channels[name] = channel # Important 
        return channel

    async def setup_a(self):
        await self.repl_agent.add_child_a(self.executor)
        await self.executor.add_child_a(self.chart)
        await self.chart.add_child_a(self)
        self.chart._interact_source = None
        await self.executor.load_repl_init_file_if_it_exists_a()
        self.py_executor.submit(self.finish_setup)

    @transform_inbound_messages
    async def transform__new_user__a(self, envelope):
        envelope.mark_used()
        await self.ready.wait()
        await self.send_message_outward_a("initialize.chart.state", *arguments(
            state=self.sseq, display_state=self.chart.display_state
        ))


    def finish_setup(self):
        self.table = ProductTable()
        self.executor.get_globals()["table"] = self.table
        if not TableChannel.SAVE_DIR.is_dir():
            TableChannel.SAVE_DIR.mkdir()
        self.load()
        if self.undoStack == []:
            self.do_initial_commands()
        self.ready.set()

    async def close_channel_a(self, code):
        del type(self).channels[self.name]
        await self.close_connections_a(code)
        await self.repl_agent.remove_child_a(self.executor)

    async def add_subscriber_a(self, websocket):
        recv = SseqSocketReceiver(websocket)
        # recv.executor = self.executor
        await self.add_child_a(recv)
        await recv.start_a()

    def setup_executor_namespace(self):
        globals = self.executor.get_globals()
        globals["REPL"] = self.repl_agent
        globals["chart"] = self.chart
        globals["channel"] = self


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



    @transform_inbound_messages
    async def transform__console__take__a(self, envelope):
        envelope.mark_used()
        self.repl_agent.set_executor(self.executor)

    @transform_inbound_messages
    async def transform__click__a(self, envelope, *args, **kwargs):
        envelope.mark_used()


    def save(self):
        save_str = json_stringify(self.undoStack)
        iso_time = datetime.now().replace(microsecond=0).isoformat().replace(":", "-")        
        out_files = [TableChannel.SAVE_DIR / f"{self.name}_{i}.json" for i in [5, 10, 50] if self.saves % i == 0]
        out_files.append(f"{self.name}.json")
        if self.saves % 100 == 0:
            out_files.append(f"{self.name}_100_{iso_time}.json")
        for path in out_files:
            (TableChannel.SAVE_DIR / path).write_text(save_str)
        self.saves += 1

    def load(self):
        self.table.initialize_change_of_basis_matrices()
        self.table.compute_all_indecomposables()
        self.populate_chart()
        self.redoStack = []
        self.previews = {}
        self.saves = 1
        save_path = TableChannel.SAVE_DIR / f"{self.name}.json"
        self.undoStack = []
        if save_path.is_file():
            json_str = save_path.read_text()
            self.undoStack = json.loads(json_str)
        for action in self.undoStack:
            self.do_action(action)
            self.update_action_bidegrees(action)




############# start of math logic


    def populate_chart(self):
        self.sseq = ChartData(self.name)
        self.executor.get_globals()["sseq"] = self.sseq
        self.sseq.x_range = [0, 180]
        self.sseq.y_range = [0, 60]
        self.sseq.initial_x_range = [0, 60]
        self.sseq.initial_y_range = [0, 30]
        self.sseq.offset_size = 9
        self.sseq.class_scale = 9     
        for (y, row) in enumerate(self.table.num_gens):
            for (x, gens) in enumerate(row):
                for _ in range(gens):
                    self.sseq.add_class(x,y)

        for x in range(self.table.x_max):
            for y in range(self.table.y_max):
                for (s, t) in self.get_outgoing_edges(x, y):
                    self.sseq.add_structline(self.sseq.class_by_idx(*s), self.sseq.class_by_idx(*t))
                

        # for [c,name] in self.table.class_names:
        #     try:
        #         self.sseq.class_by_idx(*c).monomial_name = name
        #         self.sseq.class_by_idx(*c).name = self.table.name_to_str(name)
        #     except IndexError:
        #         pass
        
        for c in self.sseq.classes:
            self.update_color(c)
        unit = self.sseq.class_by_idx(0,0,0)
        unit.name = "1"
        unit.set_color("black")
        self.chart.set_sseq(self.sseq)


    def do_initial_commands(self):
        self.process_user_action({
            "cmd_list" : [{
                "type" : "set_name", 
                "bidegree" : [(1<<i) - 1, 1], 
                "our_basis_vec" : [1],
                "vec" : [1],
                "name" : f"h_{i}"
            } for i in range(HI_MAX)],
            "description" : "Named <katex-expr>h_i</katex-expr> family"
        })

    @transform_inbound_messages
    async def transform__interact__select_bidegree__a(self, envelope, bidegree):
        envelope.mark_used()
        names = self.get_names_info(bidegree)
        [x, y] = bidegree 
        named_vecs = [[k, self.table.name_to_str(v)] for [k, v] in self.table.named_vecs[y][x].items()]
        matrix = self.get_matrix(bidegree)
        prod_info = self.get_product_info(bidegree)
        await self.send_message_outward_a("interact.product_info", *arguments(names=names, named_vecs=named_vecs, matrix=matrix, product_info=prod_info))


    async def send_action_info(self, action):
        await self.send_message_outward_a("interact.action_info", 
            *arguments(action=action)
        )

# Actions:

    @transform_inbound_messages
    async def transform__interact__action__a(self, envelope, action):
        envelope.mark_used()
        for [bidegree, state] in self.previews.items():
            self.restore_bidegree_state(bidegree, state)
        self.previews = {}
        action = await asyncio.wrap_future(self.py_executor.submit(self.process_user_action, action))
        self.save()
        await self.send_action_info(action)
        await self.chart.sseq.update_a()

    def process_user_action(self, action):
        action["cmd_list"] = list(itertools.chain.from_iterable(self.prepare_command(cmd) for cmd in action["cmd_list"]))
        for cmd in action["cmd_list"]:
            self.apply_command(cmd)
        self.update_action_bidegrees(action)
        self.undoStack.append(action)
        self.redoStack = []
        return action     

    def prepare_command(self, cmd):
        return getattr(self, "prepare_" + cmd["type"])(**cmd)

    def prepare_set_name(self, **kwargs):
        [x, y] = kwargs["bidegree"]
        name_mono = name_tools.parse_name(kwargs["name"])
        kwargs["name"] = name_mono if name_mono else None
        kwargs["state"] = self.table.named_vecs[y][x].get(tuple(kwargs["vec"]), None)
        if name_mono:
            return self.propagate_names([kwargs])
        else:
            return [kwargs]

    def propagate_names(self, names):
        result = dict([(*name["bidegree"], tuple(name["vec"])), name] for name in names)
        new_names = result
        product_list = [
            {
                "bidegree" : [(1 << i) - 1, 1],
                "name" : name_tools.parse_name(f"h_{i}"),
                "vec" : [1]
            }
            for i in range(HI_MAX)
        ]
        while new_names:
            last_names = new_names
            new_names = {}
            for name in last_names.values():
                for prod in product_list:
                    out_x = prod["bidegree"][0] + name["bidegree"][0]
                    out_y = prod["bidegree"][1] + name["bidegree"][1]
                    out_vec = tuple(self.table.multiply_vectors(
                        prod["bidegree"], prod["vec"], 
                        name["bidegree"], name["vec"]
                    ))
                    out_our_basis = fp.FpVector(2, self.table.gens_in_bidegree(out_x, out_y))
                    try:
                        self.table.basis_in_bidegree(out_x, out_y) \
                            .apply(out_our_basis, fp.FpVector.from_list(2, out_vec))
                        if 1 in out_vec and \
                            not self.table.named_vecs[out_y][out_x].get(out_vec, None) \
                            and (out_x, out_y, out_vec) not in result:

                            new_name = name_tools.reduce_monomial(name["name"] + prod["name"])
                            new_names[(out_x, out_y, out_vec)] = {
                                "type" : "set_name", 
                                "bidegree" : [out_x, out_y],
                                "name" : new_name,
                                "vec" : out_vec,
                                "our_basis_vec" : list(out_our_basis),
                                "state" : None
                            }
                    except IndexError:
                        pass
            result.update(new_names)
        return list(result.values())
            

    def prepare_set_matrix(self, **kwargs):
        # Check that matrix is invertible. Raise error if not, otherwise return input unchanged.
        matrix = kwargs["matrix"]
        bidegree = kwargs["bidegree"]
        fp.Basis(2, len(matrix)).set_matrix(matrix)
        kwargs["state"] = self.table.basis_in_bidegree(*bidegree).matrix.to_python_matrix()
        return [kwargs]

    def apply_command(self, cmd):
        getattr(self, "apply_" + cmd["type"])(**cmd)

    def apply_set_name(self, type, bidegree, vec, name, state, **kwargs):
        [x, y] = bidegree
        self.table.named_vecs[y][x][tuple(vec)] = name
        if name and len(name) == 1 and name[0][1] == 1:
            new_gen = name[0][0]
            if not new_gen.startswith("h_{"):
                self.table.gen_degs[new_gen] = [x, y]
    
    def apply_set_matrix(self, type, bidegree, matrix, state, **kwargs):
        self.table.basis_in_bidegree(*bidegree).set_matrix(matrix)

    @transform_inbound_messages
    async def transform__interact__revert_preview__a(self, envelope, bidegree):
        envelope.mark_used()
        if tuple(bidegree) in self.previews:
            self.restore_bidegree_state(bidegree, self.previews.pop(tuple(bidegree)))
            await self.chart.sseq.update_a()

    @transform_inbound_messages
    async def transform__interact__redo__a(self, envelope):
        envelope.mark_used()
        if not self.redoStack:
            await self.send_action_info(None)
            return
        action = await asyncio.wrap_future(self.py_executor.submit(self.redo_action_main)) 
        await self.send_action_info(action)
        await self.chart.sseq.update_a()

    def redo_action_main(self):
        action = self.redoStack.pop()
        self.undoStack.append(action)
        self.do_action(action)
        self.update_action_bidegrees(action)
        self.save()
        return action        

    def do_action(self, action):
        for cmd in action["cmd_list"]:
            self.apply_command(cmd)

    @transform_inbound_messages
    async def transform__interact__undo__a(self, envelope):
        envelope.mark_used()
        if not self.undoStack:
            await self.send_action_info(None)
            return        
        action = await asyncio.wrap_future(self.py_executor.submit(self.undo_action_main))
        await self.send_action_info(action)
        await self.chart.sseq.update_a()

    def undo_action_main(self):
        action = self.undoStack.pop()
        self.redoStack.append(action)
        for cmd in reversed(action["cmd_list"]):
            self.undo_cmd(cmd)
        self.update_action_bidegrees(action)
        self.save()
        return action

    def update_action_bidegrees(self, action):
        for bidegree in set(tuple(cmd["bidegree"]) for cmd in action["cmd_list"]):
            self.update_bidegree(bidegree)

    def undo_cmd(self, cmd):
        getattr(self, "undo_" + cmd["type"])(**cmd)
    
    def undo_set_name(self, bidegree, name, vec, state, **kwargs):
        [x, y] = bidegree
        if state is None:
            state = ""
        self.table.named_vecs[y][x][tuple(vec)] = state

    def undo_set_matrix(self, bidegree, state, **kwargs):
        self.table.basis_in_bidegree(*bidegree).set_matrix(state)


    @transform_inbound_messages
    async def transform__interact__validate__name__a(self, envelope, name):
        envelope.mark_used()
        [validated, error] = name_tools.validate_name(name)
        await self.send_message_outward_a("interact.validate.name", *arguments(name=name, validated=validated, error=error))

    @transform_inbound_messages
    async def transform__interact__validate__matrix__a(self, envelope, bidegree, matrix):
        envelope.mark_used()
        row_labels = self.get_matrix_row_labels(bidegree, matrix)
        singular = False
        if tuple(bidegree) not in self.previews:
            self.previews[tuple(bidegree)] = self.collect_bidegree_state(bidegree)
        try:
            B = fp.Basis(2, len(matrix))
            B.set_matrix(matrix)
            [x,y] = bidegree
            self.table.bases[y][x] = B
            self.replace_edges(*bidegree, "blue", [15, 5])
        except ValueError:
            self.replace_edges(*bidegree, "red", [15, 5])
            singular = True
        await self.chart.sseq.update_a()
        await self.send_message_outward_a("interact.validate.matrix", *arguments(row_labels=row_labels, singular=singular))
    

    def get_matrix_row_labels(self, bidegree, matrix):
        [x, y] = bidegree
        named_vecs = self.table.named_vecs[y][x]
        return [self.table.name_to_str(named_vecs.get(tuple(row), None)) for row in matrix]


    def collect_bidegree_state(self, bidegree):
        [x, y] = bidegree
        sseq = self.chart.sseq
        state = {}
        state["basis"] = self.table.basis_in_bidegree(*bidegree).matrix.to_python_matrix()
        state["named_vecs"] = list(self.table.named_vecs[y][x].items())
        return state

    def restore_bidegree_state(self, bidegree, state):
        sseq = self.chart.sseq
        [x, y] = bidegree
        basis = state["basis"]
        self.table.basis_in_bidegree(*bidegree).set_matrix(basis)
        named_vecs = dict([[tuple(k),v] for [k, v] in state["named_vecs"]])
        self.table.named_vecs[y][x] = named_vecs
        self.update_bidegree(bidegree)

    def update_bidegree(self, bidegree):
        self.replace_edges(*bidegree, "black", [])
        [x, y] = bidegree
        named_vecs = self.table.named_vecs[y][x]
        matrix = self.table.basis_in_bidegree(*bidegree).matrix
        self.table.update_indecomposables_in_bidegree(*bidegree)
        for (idx, c) in enumerate(self.chart.sseq.classes_in_bidegree(*bidegree)):
            mono = named_vecs.get(tuple(matrix[idx]))
            c.monomial_name = mono
            c.name = self.table.name_to_str(mono)
            self.update_color(c)


    def update_color(self, c):
        c.indec = self.table.indecomposable_q(c.x, c.y, c.idx)
        c.hi_indec = self.table.hi_indecomposable_q(c.x, c.y, c.idx)        
        named = hasattr(c, "monomial_name") and c.monomial_name
        if not c.indec and named:
            c.set_color("black")
        elif not c.indec and not named:
            c.set_color("blue")
        elif c.indec and named:
            c.set_color("red")
        elif c.indec and not named:
            c.set_color("purple")


    def get_outgoing_edges(self, x, y):
        prods = self.table.compute_hi_products(x, y)
        for [hi, table] in zip([0, 1, 3], prods):
            for (in_idx, row) in enumerate(table):
                for (out_idx, v) in enumerate(row):
                    if v != 0:
                        yield ((x, y, in_idx), (x+hi, y+1, out_idx))

    
    def get_incoming_edges(self, x, y):
        prods = [self.table.compute_hi_products(x - hi, y - 1)[idx] for (idx, hi) in enumerate([0, 1, 3])]
        for [hi, table] in zip([0, 1, 3], prods):
            for (in_idx, row) in enumerate(table):
                for (out_idx, v) in enumerate(row):
                    if v != 0:
                        yield ((x-hi, y-1, in_idx), (x, y, out_idx))

    def get_edges(self, x, y):
        from itertools import chain
        return chain(self.get_incoming_edges(x, y), self.get_outgoing_edges(x, y))



    def replace_edges(self, x, y, color = None, dash = None, line_width = None):
        sseq = self.chart.sseq
        new_edges = self.get_edges(x, y)
        deleted_edges = 0
        for c in sseq.classes_in_bidegree(x, y):
            for e in list(c._edges):
                e.delete()
                deleted_edges += 1
        # added_edges = 0
        for (s, t) in new_edges:
            e = sseq.add_structline(sseq.class_by_idx(*s), sseq.class_by_idx(*t))
            e.color = color
            e.dash = dash
            e.line_width = line_width




    def get_name(self, tuple):
        c = self.chart.sseq.class_by_idx(*tuple)
        if c.name:
            return c.name
        else:
            return f"x_{{{tuple[0], tuple[1]}}}^{{{tuple[2]}}}"

    def get_monomial_name(self, tuple):
        c = self.chart.sseq.class_by_idx(*tuple)
        if hasattr(c, "monomial_name"):
            return c.monomial_name or None

    def get_names_info(self, bidegree):
        num_classes = len(self.chart.sseq.classes_in_bidegree(*bidegree))
        return [
            (self.get_name(t), self.get_monomial_name(t)) 
            for t in 
                [ (*bidegree, i) for i in range(num_classes) ]
        ]
    
    def get_matrix(self, bidegree):
        (x, y) = bidegree
        try:
            return self.table.bases[y][x].matrix.to_python_matrix()
        except IndexError:
            return []
        

    def get_product_info(self, bidegree):
        result = []
        v = fp.FpVector(2, self.table.gens_in_bidegree(*bidegree))
        w = fp.FpVector(2, self.table.gens_in_bidegree(*bidegree))
        b = self.table.basis_in_bidegree(*bidegree)
        for (in1, in2, out) in self.get_filtered_decompositions(bidegree):
            [n1, n2] = [(x, self.get_name(x), self.get_monomial_name(x)) for x in [in1, in2]]
            v.pack(out)
            w.set_to_zero()
            b.apply_inverse(w, v)
            out_name = self.table.name_to_str(self.table.get_vec_name(*bidegree, out))
            result.append({ "left" : n1, "right" : n2, "out_res_basis" : out, "out_our_basis" : list(w),  "out_name" : out_name })
        return result

    def get_filtered_decompositions(self, bidegree):
        bidegree = tuple(bidegree)
        try:
            decompositions = self.table.get_decompositions(*bidegree)
        except KeyError:
            return []
        result = []
        for (in1, in2, out) in decompositions:
            indec1 = self.table.indecomposable_q(*in1)
            indec2 = self.table.indecomposable_q(*in2)
            if not indec1:
                continue
            if indec1 and indec2 and in1 > in2:
                continue
            if in1 == (0,0,0):
                continue
            # We don't want a bunch of redundant P-juggling 
            # (really Px shouldn't be considered indecomposable)
            if self.get_name(in1).strip().startswith("P") and self.get_name(in2).strip().startswith("P"):
                continue
            result.append((in1, in2, out))
        return result




class ProductTable:
    def __init__(self):
        self.load_json()
        self.generate_decomposition_table()
        self.setup_class_names()
        self.build_dense_products()
    
    def load_json(self):
        json_obj = json.loads(pathlib.Path(CHANNEL_DIR / "product_table.json").read_text())
        self.num_gens = json_obj["dimensions"]
        self.product_table = dict(
            [tuple(tuple(l) for l in key), value] 
            for [key, value] in json_obj["product_table"]
        )
        self.y_max = len(self.num_gens)
        self.x_max = len(self.num_gens[0])        

    def initialize_change_of_basis_matrices(self):
        self.bases = [[fp.Basis(2, n) for n in r] for r in self.num_gens]
    
    def gens_in_bidegree(self, x, y):
        try:
            return self.num_gens[y][x]
        except IndexError:
            # print("gens_in_bidegree index error", x, y)
            return 0
    
    def basis_in_bidegree(self, x, y):
        try:
            return self.bases[y][x]
        except IndexError:
            # print("IndexError in basis_in_bidegree", x, y)
            raise

    def generate_decomposition_table(self):
        self.decomposition_table = {}
        self.nontrivial_pairs = {}
        for ((in1, in2), out) in self.product_table.items():
            key = (in1[0] + in2[0], in1[1] + in2[1])
            if key not in self.decomposition_table:
                self.decomposition_table[key] = []
                self.nontrivial_pairs[key] = set()
            self.decomposition_table[key].append((in1, in2, [ x[-1] for x in out ]))
            self.nontrivial_pairs[key].add((in1[:-1], in2[:-1]))

    def compute_all_indecomposables(self):
        self.indecomposables = [[[] for n in r] for r in self.num_gens]
        self.hi_indecomposables = [[[] for n in r] for r in self.num_gens]
        for (y, row) in enumerate(self.num_gens):
            for (x, e) in enumerate(row):
                if (x,y) in self.decomposition_table:
                    self.update_indecomposables_in_bidegree(x,y)

    def update_indecomposables_in_bidegree(self, x, y):
        self.indecomposables[y][x] = self.compute_indecomposables_in_bidegree(x, y)
        self.hi_indecomposables[y][x] = self.compute_hi_indecomposables_in_bidegree(x, y)

    def compute_indecomposables_in_bidegree(self, x, y):
        ng = self.gens_in_bidegree(x, y)
        subspace = fp.Subspace(2, ng+1, ng)
        subspace.set_to_zero()
        image_vecs = [ out for (in1, in2, out) in self.decomposition_table[(x,y)] if in1 != (0,0,0)]
        py_v = [0] * ng
        v = fp.FpVector(2, ng)
        w = fp.FpVector(2, ng)
        B = self.basis_in_bidegree(x, y)
        for e in image_vecs:
            for i in range(ng):
                py_v[i] = 1 if i in e else 0
            v.pack(py_v)
            w.set_to_zero()
            B.apply_inverse(w, v)
            subspace.add_vector(w)
        return [idx for (idx, e) in enumerate(subspace.matrix().pivots()) if e == -1]

    def compute_hi_indecomposables_in_bidegree(self, x, y):
        ng = self.gens_in_bidegree(x, y)
        subspace = fp.Subspace(2, ng+1, ng)
        subspace.set_to_zero()
        image_vecs = [ out for (in1, in2, out) in self.decomposition_table[(x,y)] if in1[1] == 1]
        py_v = [0] * ng
        v = fp.FpVector(2, ng)
        w = fp.FpVector(2, ng)
        B = self.basis_in_bidegree(x, y)
        for e in image_vecs:
            for i in range(ng):
                py_v[i] = 1 if i in e else 0
            v.pack(py_v)
            w.set_to_zero()
            B.apply_inverse(w, v)
            subspace.add_vector(w)
        return [idx for (idx, e) in enumerate(subspace.matrix().pivots()) if e == -1]


    def compute_hi_products(self, x, y):
        v2 = fp.FpVector(2, 0)
        w2 = fp.FpVector(2, 0)
        vout = fp.FpVector(2, 0)
        wout = fp.FpVector(2, 0)
        result = []   
        for hi in [0, 1, 3]:
            t1 = (hi, 1)
            t2 = (x, y)
            tout = (x + hi, y + 1)
            hi_result = []
            if tout not in self.dense_products:
                result.append(hi_result)
                continue
            ng2 = self.gens_in_bidegree(*t2)
            ngout = self.gens_in_bidegree(*tout)
            b2 = self.basis_in_bidegree(*t2)
            try:
                bout = self.basis_in_bidegree(*tout)
            except IndexError:
                result.append(hi_result)
                continue
            v2.set_scratch_vector_size(ng2)
            w2.set_scratch_vector_size(ng2)
            vout.set_scratch_vector_size(ngout)
            wout.set_scratch_vector_size(ngout)
            for i in range(ng2):
                v2.set_to_zero()
                w2.set_to_zero()
                vout.set_to_zero()
                wout.set_to_zero()
                v2[i] = 1
                b2.apply(w2, v2)
                pair = (t1, t2) if t1 <= t2 else (t2, t1)
                products = None
                if pair in self.dense_products[tout]:
                    products = self.dense_products[tout][pair]
                else:
                    pair = tuple(reversed(pair))
                    if tuple(reversed(pair)) in self.dense_products:
                        products = self.dense_products[tout][pair]
                if products:
                    for j in range(ng2):
                        if w2[j] != 0:
                            vout.add(products[j])
                bout.apply_inverse(wout, vout)
                hi_result.append(list(wout))
            result.append(hi_result)
        return result
    
    def build_dense_products(self):
        self.dense_products = {}
        for [tout, pairs] in self.nontrivial_pairs.items():
            self.dense_products[tout] = {}
            for (t1, t2) in pairs:
                self.dense_products[tout][(t1, t2)] = self.build_dense_products_bidegree(t1, t2)
    
    def build_dense_products_bidegree(self, t1, t2):
        tout = (t1[0] + t2[0], t1[1] + t2[1])
        ng1 = self.gens_in_bidegree(*t1)
        ng2 = self.gens_in_bidegree(*t2)
        ngout = self.gens_in_bidegree(*tout)
        products = [ fp.FpVector(2, ngout) for _ in range(ng1 * ng2) ]
        for idx1 in range(ng1):
            in1 = t1 + (idx1,)
            for idx2 in range(ng2):
                in2 = t2 + (idx2,)
                if (in1,in2) in self.product_table:
                    product_table_entry = self.product_table.get((in1,in2), [])
                else:
                    product_table_entry = self.product_table.get((in2, in1), [])
                for [_, _, e] in product_table_entry:
                    try:
                        products[ idx2 * ng1 + idx1 ][e] = 1
                    except IndexError:
                        print("bdpbi", t1, t2, "product_table_entry", product_table_entry, "ngout", ngout)
                        raise
        return products

    def multiply_vectors(self, in1, vec1, in2, vec2):
        t1 = tuple(in1)
        t2 = tuple(in2)
        ng1 = self.gens_in_bidegree(*t1)
        ng2 = self.gens_in_bidegree(*t2)            
        v1 = fp.FpVector(2, ng1)
        v2 = fp.FpVector(2, ng2)
        v1.pack(vec1)
        v2.pack(vec2)
        w1 = fp.FpVector(2, 0)
        w2 = fp.FpVector(2, 0)
        tensor = fp.FpVector(2, 0)
        vout = fp.FpVector(2, 0)
        wout = fp.FpVector(2, 0)
        return self.multiply_vectors_helper(t1, v1, t2, v2, w1, w2, tensor, vout, wout)

    def multiply_vectors_helper(self, 
        t1, v1, t2, v2,
            w1, w2, tensor, vout, wout
    ):
        [x1, y1] = t1
        [x2, y2] = t2
        tout = (x1 + x2, y1 + y2)
        

        try:
            ng1 = self.gens_in_bidegree(*t1)
            ng2 = self.gens_in_bidegree(*t2)         
            ngout = self.gens_in_bidegree(*tout)

            b1 = self.basis_in_bidegree(*t1)
            b2 = self.basis_in_bidegree(*t2)
            bout = self.basis_in_bidegree(*tout)
        except IndexError:
            return []


        w1.set_scratch_vector_size(ng1)
        w2.set_scratch_vector_size(ng2)
        vout.set_scratch_vector_size(ngout)
        wout.set_scratch_vector_size(ngout)
        tensor.set_scratch_vector_size(ng1 * ng2)

        w1.set_to_zero()
        w2.set_to_zero()
        vout.set_to_zero()
        wout.set_to_zero()
        tensor.set_to_zero()
        if tout not in self.dense_products or (t1, t2) not in self.dense_products[tout]:
            return tuple(vout)

        b1.apply(w1, v1)
        b2.apply(w2, v2)
        tensor.add_tensor(w1, w2)
        for i in range(ng1 * ng2):
            if tensor[i] != 0:
                vout.add(self.dense_products[tout][(t1, t2)][i])
        bout.apply_inverse(wout, vout)
        return tuple(vout)

    def get_decompositions(self, x, y):
        v1 = fp.FpVector(2, 0)
        w1 = fp.FpVector(2, 0)
        v2 = fp.FpVector(2, 0)
        w2 = fp.FpVector(2, 0)
        tensor = fp.FpVector(2, 0)
        tout = (x, y)
        ngout = self.gens_in_bidegree(*tout)
        vout = fp.FpVector(2, ngout)
        wout = fp.FpVector(2, ngout)
        result = []
        for (t1, t2) in self.dense_products[tout]:
            ng1 = self.gens_in_bidegree(*t1)
            ng2 = self.gens_in_bidegree(*t2)            
            v1.set_scratch_vector_size(ng1)
            v2.set_scratch_vector_size(ng2)
            for idx1 in range(ng1):
                v1.set_to_zero()
                v1[idx1] = 1
                for idx2 in range(ng2):
                    v2.set_to_zero()
                    v2[idx2] = 1
                    product = self.multiply_vectors_helper(
                        t1, v1, t2, v2, 
                        w1, w2, tensor, vout, wout
                    )
                    if 1 in product:
                        result.append((t1 + (idx1,), t2 + (idx2,), product))
        result.sort(key=lambda x : [-sum(x[-1]), *x[-1], -x[0][0], -x[0][1]], reverse=True)
        return result

    def indecomposable_q(self, x, y, idx):
        return idx in self.indecomposables[y][x]

    def hi_indecomposable_q(self, x, y, idx):
        return idx in self.hi_indecomposables[y][x]

    def setup_class_names(self):
        # class_names = json.loads(pathlib.Path(config.USER_DIR / "class_names_parsed.json").read_text())
        # self.class_names = [[loc, name] for [loc,name] in class_names if len(name) != 1 or name[0][0] != "h_{0}" or loc[0] == 0]
        self.gen_degs = {}
        # for [t, name] in self.class_names:
        #     if len(name) == 1 and name[0][1] == 1:
        #         self.gen_degs[name[0][0]] = t
        override_gens = ["M", "P", "\\Delta", "\\Delta_{1}", *(f"h_{{{i}}}" for i in range(HI_MAX))]
        for [idx, gen] in enumerate(reversed(override_gens)):
            self.gen_degs[gen] = [-idx, -idx, -idx]
        self.named_vecs = [[{} for _ in range(self.x_max)] for _ in range(self.y_max)]
    
    def get_vec_name(self, x, y, vec):
        return self.named_vecs[y][x].get(tuple(vec), None)

    def set_vec_name(self, x, y, vec, name):
        self.named_vecs[y][x][tuple(vec)] = name

    def name_to_str(self, name):
        if name:
            return name_tools.monomial_name(
                *sorted(name, 
                    key= lambda x : 
                        self.gen_degs[x[0]] if x[0] in self.gen_degs else [10000, 10000]
                )
            )
        else:
            return ""







# Generate from original file

    # def load_numgens(self):
    #     self.num_gens = [
    #         [1, *[0 for _ in range(183)]], 
    #         *(
    #             row[idx:] 
    #             for [idx, row] in
    #             enumerate(json.loads(pathlib.Path(config.USER_DIR / "Rank_Ext.txt").read_text()))
    #         )
    #     ]
    #     self.s_totals = [[x - 1 for x in itertools.accumulate(row)] for row in self.num_gens]
    #     self.s_indexes = [[] for _ in range(129)]
    #     for (i, row) in enumerate(self.s_totals):
    #         for (j, entry) in enumerate(row):
    #             if entry > row[j-1]:
    #                 try:
    #                     self.s_indexes[i].append([entry, j])
    #                 except IndexError:
    #                     print(i)


    # def s_idx_to_x_idx(self, s, idx):
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
    #     self.product_table = {}
    #     for s in ajr_lines:
    #         l = s.replace("(", "").split()
    #         try:
    #             output = [int(x) for x in l[:2]]
    #             in1 = [int(x) for x in l[2:4]]
    #             in2 = [int(x) for x in l[-1].split("_")]
    #         except ValueError:
    #             print(l)
    #             break
    #         [output, in1, in2] = [self.s_idx_to_x_idx(*x) for x in (output, in1, in2)]
    #         if None in [output, in1, in2]:
    #             print(l)
    #             continue
    #         if (in1, in2) not in self.product_table:
    #             self.product_table[(in1,in2)] = []
    #         self.product_table[(in1,in2)].append(output)